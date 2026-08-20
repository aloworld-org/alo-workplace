# The campaign send job

*alo Campaigns, ADR 0044, wave C4 — item C4.1 and the control half of C4.4.*

Everything a campaign needs in order to become mail exists. `campaign_content`
holds the blocks, `campaign_html` and `campaign_text` compile them,
`campaign_mime` assembles the `multipart/alternative`, `campaign_merge`
personalises with a fallback, `campaign_audience` answers who may be mailed and
`campaign_unsubscribe` mints the token that lets them leave. `campaign_mime`
ends its own module doc with the sentence this design exists to answer:

> Handing this entity to a submission path is one function call on the day
> there is one.

There is no such day yet, because there is no record of a send. This is that
record — not the submission itself.

## Surface

One new store module, `campaign_send.rs`, and two tables.

| Call | Answers |
|---|---|
| `open_campaign_send(campaign_id)` | Starts a send. Returns the send with its state. |
| `enrol_campaign_send_page(send_id, page)` | Materialises the next page of recipients. Returns how many were written and the cursor to continue from. |
| `campaign_send(send_id)` | One send with its tallies. |
| `campaign_sends(campaign_id)` | Every send of a campaign, newest first. |
| `pause_campaign_send` / `resume_campaign_send` / `stop_campaign_send` | The state machine's three operator verbs. |

Enrolment is **paged and driven by the caller**, not a single statement that
walks the whole audience. An audience of two hundred thousand assembled in one
transaction is a lock held for minutes over three tables that CRM and Billing
are still writing to; a page at a time is resumable, and resumability is the
entire point of the item.

## Errors

| Condition | The caller sees |
|---|---|
| No such campaign, or another tenant's | `NotFound` — never a database error, never a hint that the id exists elsewhere |
| Campaign has no content blocks | `Validation`, naming that the campaign has nothing to say |
| A send is already open for this campaign | `Conflict`, naming the open send's id |
| Enrolling a send that is stopped or done | `Conflict` — state outranks the request |
| Pausing a send that is not sending | `Conflict`, naming the state it is actually in |
| Resuming a send that is not paused | `Conflict` |
| Stopping a send already stopped | `Ok`, deliberately — stop is idempotent, because the operator pressing it twice means the same thing both times and the second press must not be an error at the exact moment they are panicking |
| Page size outside `1..=AUDIENCE_PAGE_MAX` | `Validation`, the same rule `campaign_audience` already applies |

## Tenancy

Both tables carry `tenant_id` in their primary key and reach their parent
through a **composite foreign key** — `campaign_send_recipients` references
`(tenant_id, send_id)`, and `campaign_sends` references
`(tenant_id, campaign_id)`. Tenancy is therefore structural: there is no way to
attach a recipient row to another tenant's send, because the reference itself
would not resolve. Every statement is additionally scoped by the store's own
tenant binding, as everywhere else.

The wrong-tenant test is mandatory and covers all six verbs: tenant B naming
tenant A's campaign or send gets `NotFound`, never data and never a 500.

## Idempotency: on who was *mailed*

C4.1 says *idempotency on (campaign, address)*. The state it is keyed on is the
entire design, and two of the three candidates are wrong:

| Key | What it prevents | What it breaks |
|---|---|---|
| `(send, address)` | one send enrolling somebody twice | a *second* send of the same campaign re-mails everybody the first reached |
| `(campaign, address)`, all rows | the double mail | enrolment writes everybody as `pending` seconds after the button is pressed, so **stopping a send that had mailed nobody leaves the campaign permanently unsendable** — the safety button kills the campaign |
| `(campaign, address)` where `state = 'sent'` | the double mail | nothing |

The third is what is built. A campaign reaches a given person at most once,
ever, while somebody enrolled and never mailed stays reachable — so stopping a
send halfway means the next one reaches exactly the people the first did not,
which is what an operator means by "fix it and send it again".

Enrolment therefore skips addresses with a `sent` row for the campaign, and
reports them as `already_mailed` rather than silently. Within a single send the
primary key `(tenant_id, send_id, address)` still makes a repeated page a no-op,
which is what makes enrolment **resumable after a crash rather than restarted**.

The consequence that *is* deliberate: mailing the same people the same campaign
a second time is impossible, and doing it on purpose means writing a second
campaign. That is the honest model for bulk mail — a "resend" that quietly
re-enrols is how people get mailed four times by a system that believes it is
behaving.

Reaching the `sent` state at all needs
`mark_campaign_recipient_sent`, which moves only a `pending` row. That is the
seam the dispatcher (C4.2) will use, and without it the guarantee above would be
unenforceable: a ledger with no way to record a send guarantees nothing.

## Who is enrolled

`campaign_recipients` already applies consent and tenant-wide suppression, so
enrolment inherits both. It does **not** apply per-topic opt-outs, because a
topic is a fact about the campaign rather than about the audience — so this
module applies `campaign_topics_declined_by` itself and records the skip.

A recipient who declined the topic is written as a row in state `skipped` with
the reason, **not** omitted. Omitting them would make the send's tallies lie:
"we mailed 900 of 1000" with no account of the hundred is the kind of number
that gets a sender into trouble with a regulator who asks what happened to
them. The row is the account.

## Out of scope, deliberately

- **The dispatcher itself (C4.2, C4.3).** Rendering per recipient at send time
  and pacing under the warm-up cap are the consumer of this ledger, and they
  are their own change with their own load test. C4.3 additionally blocks on
  C2.2 (a separate queue) and C2.3 (per-tenant caps), neither of which exists —
  building pacing before the queue it paces would be building against a design
  that has not been made.
- **Scheduling (the other half of C4.4).** A send opens now or not at all. A
  `scheduled_at` is one additive column on the day there is a runner to honour
  it, and a column nothing reads is a promise the screen would display.
- **The send safety screen (C4.5).** It needs this API to exist first; it is
  the next change, not this one.
- **`List-Unsubscribe` headers.** They are written by the submission path, per
  `campaign_mime`'s own reasoning — a fact about a send, added where the send
  happens.

## The alternative rejected

**A single `send_campaign` call that enrols and dispatches in one transaction.**
Rejected because it makes the two failure modes inseparable: a submission that
fails halfway leaves no record of who was already mailed, so the only safe
recovery is to mail nobody or to mail everybody again. Separating the ledger
from the dispatch means a crash is answered by reading the table rather than by
guessing — and the ledger is the artefact a regulator, a customer, and an
engineer at 3am all need to read.
