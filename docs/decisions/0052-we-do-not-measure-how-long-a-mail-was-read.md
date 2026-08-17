# ADR 0052 — We do not measure how long a mail was read

**Status:** accepted (2026-08-17)
**Context:** roadmap `C5.4`, ADR 0044 (alo Campaigns) §5, ADR 0037 (Insights),
`docs/design/campaigns-gaps.md`
**Decides** the item the campaigns queue was forbidden to build.

## The decision in one line

alo does **not** measure how long a recipient spent reading an email, and will
not ship a number that claims to — because the measurement does not work, and a
figure whose error we cannot even size is worse than no figure.

## Why this is not primarily a privacy decision

It is tempting to decide this on principle, and the principle is real: a product
sold on not tracking people should be slow to track them. But that argument
invites the obvious rebuttal — *our customers want it, it is opt-in, everyone
does it* — and a decision that survives only until a big enough prospect asks is
not a decision.

The stronger reason is that **the number is wrong**.

- Read duration requires a remote image held open and reporting back. Most mail
  clients block remote images by default; many that load them cache the result,
  so the "second open" never reaches us.
- Apple's Mail Privacy Protection pre-fetches images on delivery, so an unknown
  share of every open is a machine that never read anything, and a "duration"
  derived from it is the pre-fetcher's behaviour rather than a person's.
- The error therefore runs in a direction and a magnitude **we cannot
  quantify**. It is not a noisy signal that averages out; it is a mixture of
  people and machines in unknown proportion.

A metric that cannot be caveated honestly cannot be shipped honestly. We could
write "estimated" beside it, but we could not say estimated *within what*, and a
reader would take the number at face value because numbers on dashboards are
taken at face value.

**If it worked and we refused on principle, there would be a real trade-off to
weigh. It does not work, so there is not one.**

## The privacy argument, which is also true

Per-recipient reading behaviour is profiling. In the EU it needs consent, and a
Dutch or German supervisory authority would read it that way. That is a
compliance exposure in the product whose entire premise is compliance.

And it would contradict us in another room. Site Insights is sold as the EU
answer to Google Analytics — consent-free, aggregate, no individuals tracked,
with the rulings that outlawed GA in Austria, France and Italy as the pitch.
Shipping per-recipient reading-behaviour tracking in Campaigns argues the
opposite case in the same product, and a prospect who reads both pages notices.

## What we measure instead, which is better

The reporting screen is ordered by how much each number can be trusted
(roadmap C5.6):

| | trust | how it is known |
|---|---|---|
| delivered, bounced, complained | **fact** | our own SMTP result and the feedback loop |
| clicked | **reliable** | a redirect the recipient actually followed |
| opened | **weak** | opt-in per campaign, disclosed, labelled an estimate |
| **invoiced** | **fact** | joined to the invoice, in the same database |

That last row is the one nobody else can compute. Mailchimp cannot tell you what
a campaign earned, because the invoice lives in a system it does not have.
**"€4,210 invoiced from 1,204 emails" beats "24% opened"**, and it has the
additional merit of being true.

The sales answer to *"can you tell me how long they read it?"* is therefore not
an apology: *"No — and neither can anyone else. Apple broke that years ago and
the industry still prints it. We tell you what it earned."*

## What this does not forbid

**Open tracking stays available** exactly as ADR 0044 §5 decided: off by
default, a per-campaign choice, and disclosed. A customer who wants the familiar
vanity metric is not locked out. This ADR only refuses the further step of
timing a person's attention, which adds no reliable information on top of a
signal that is already weak.

Aggregate read-duration is refused for the same reason as the per-recipient
kind: averaging an unreliable signal produces a confident-looking number built
on the same broken measurement.

## Reconsider if

A mail client population emerges where read time is measurable without a
tracking pixel and without profiling an individual — or a standard appears that
reports engagement in aggregate from the client side. Neither exists today. A
prospect asking for it is not new evidence; a way to measure it honestly would
be.

## Rejected

- **Ship it opt-in and caveat it.** We cannot state the size of the error, so the
  caveat would be decoration. Dashboards launder estimates into facts.
- **Ship it for parity because competitors have it.** Parity with a number that
  stopped working is not parity.
- **Leave the roadmap item open pending demand.** An undecided item gets
  relitigated every quarter and eventually built by whoever is asked last. This
  file exists so the answer can be handed over instead.
