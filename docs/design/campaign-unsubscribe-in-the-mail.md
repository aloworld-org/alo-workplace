# The unsubscribe, in the message itself

*alo Campaigns, ADR 0044 §3 — queue items C2.4 and C2.5.*

Everything about leaving already exists except the half a recipient can reach.
`campaign_unsubscribe` mints a 256-bit per-recipient token and keeps only its
digest; `campaign_topic_optout` records the narrower choice; `UnsubscribeView`
is the page that offers this kind of mail or all of it, with no account and no
login. What is missing is the two ways a recipient gets to that page: the header
their mail client turns into a button, and the link in the footer for everyone
whose client draws no button.

## Why this gates everything else in C4

A bulk message with no working opt-out is not merely incomplete. Under GDPR
Art. 21(3) and ePrivacy Art. 13, the right to withdraw must be available *at the
time of each message*, and since February 2024 both Gmail and Outlook require
RFC 8058 one-click from bulk senders as a condition of delivery. So this is not
a feature that improves a campaign — it is the condition on sending one at all,
and it is why it comes before the dispatcher (C4.2) rather than after.

## Surface

A new module, `campaign_unsubscribe_link.rs`, owning one type and the two
renderings of it.

```rust
pub struct UnsubscribeInvitation {
    /// The recipient's own one-click URL. HTTPS, unguessable, single-use-ish.
    pub url: String,
    /// The kind of mail this is, when there is one — what the page offers to
    /// stop instead of everything.
    pub topic: Option<String>,
}
```

- `header_pair(&self) -> [(&'static str, String); 2]` — `List-Unsubscribe` and
  `List-Unsubscribe-Post`, ready to write.
- The HTML footer and the text footer, rendered by `campaign_html` and
  `campaign_text` from the same invitation, so the two alternatives can never
  offer different ways out.

**`CampaignLetter` gains `unsubscribe: &UnsubscribeInvitation`, and it is not
optional.** A campaign that cannot be left cannot be rendered — the type system
is where that belongs, not a check somebody remembers to write in the sender.
`CampaignMessage` carries the header pair out with the body for the same
reason: the thing that assembles a campaign hands the sender the headers it must
write, rather than trusting it to know.

## The RFCs, cited

- **RFC 2369 §3.2** — `List-Unsubscribe` holds one or more URLs, each in angle
  brackets, comma-separated, most-preferred first. We emit exactly one HTTPS
  URL: a `mailto:` alternative would need a mailbox that parses unsubscribes,
  which is a second mechanism to keep correct for the clients that already
  prefer the HTTPS one.
- **RFC 8058 §3.1** — one-click is signalled by the literal header
  `List-Unsubscribe-Post: List-Unsubscribe=One-Click`, and applies only when the
  `List-Unsubscribe` URL is HTTPS. A non-HTTPS URL is therefore refused rather
  than emitted with a header that would be ignored.
- **RFC 8058 §3.2** — the client POSTs `List-Unsubscribe=One-Click` as
  `application/x-www-form-urlencoded`. The endpoint must act on that POST alone:
  no login, no confirmation page, no second click.
- **RFC 8058 §7** — the URI must be hard to guess, or anyone can unsubscribe
  anyone. `campaign_unsubscribe`'s 256-bit token already satisfies this; this
  module refuses a URL that does not carry one.
- **Header injection** (house non-negotiable) — the URL is validated against CR
  and LF before it is written into a header. A URL is caller-supplied data, and
  a caller-supplied header value is where header injection lives.

## Errors

| Condition | The caller sees |
|---|---|
| URL is not HTTPS | `Validation` — one-click does not apply to any other scheme, and a header the client ignores is worse than none |
| URL contains CR or LF | `Validation`, naming header injection |
| URL is blank or not a URL | `Validation` |
| Topic present but blank | `Validation` — a blank label makes the page offer "stop receiving ''" |

## Tenancy

This module holds no data and issues no queries; it renders what it is given.
The token behind the URL is minted by `campaign_unsubscribe`, which is already
tenant-scoped and already tested for it. What is added here is that a letter
cannot be built without one, which makes the tenancy of the *link* structural
rather than incidental.

## Out of scope, deliberately

- **The `mailto:` alternative** of RFC 2369. It needs an address that parses
  unsubscribe mail, which is a second implementation of the same promise; every
  client that matters prefers the HTTPS form, and RFC 8058 one-click requires
  it.
- **Minting the token at send time.** That belongs to the dispatcher (C4.2),
  which is the only thing that knows which recipient a given render is for.
  This module takes the URL as given.
- **Transactional mail carrying no unsubscribe (C2.8).** Already structural:
  the transactional composer is `alo-jmap`'s `mime.rs`, a different path that
  never sees an `UnsubscribeInvitation`. Nothing to build; something to test.

## The alternative rejected

**Make `unsubscribe` an `Option` and let the sender decide.** Rejected because
the failure it permits is exactly the one that is unlawful and unrecoverable: a
campaign that went out to ten thousand people with no way to leave cannot be
un-sent, and "the sender forgot" is not a defence anybody can offer a
regulator. A required field turns that into a compile error.
