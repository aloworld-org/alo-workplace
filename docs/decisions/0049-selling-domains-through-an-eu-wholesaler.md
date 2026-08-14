# ADR 0049 — Selling domains through an EU wholesaler

**Status:** accepted — **written after the build, which is the first thing this
ADR has to say about itself.** (2026-08-14)
**Context:** `features.md` → alo Sites [S+], `docs/guides/openprovider-domains.md`,
`platform/alo-store/src/site_registrar.rs`, ADR 0013 (authoritative DNS),
ADR 0036 (alo Sites)

## The process failure, first

`features.md` and the Openprovider guide both said, in bold, **"ADR to write
before build."** Five queue items then shipped the feature — the registrar
adapter, the purchase state machine, the buy API, the payment handoff and
registration worker, and the purchase UI — and this file did not exist.

The loop was not at fault and it is worth being precise about why: **no S2.15
queue item mentioned an ADR.** The prerequisite lived in the feature inventory
and in a guide, and was never carried into the document that drives execution.
A gate recorded where the work is described but not where the work is ordered
is not a gate. That is the lesson to take, and it is the same failure shape as
a migration-block rule that lives in prose while the loop numbers from the
directory.

It was not concealed. The wave review wrote the gap into `features.md` itself —
*"production ships an unconfigured registrar, so nothing can be bought until
the ADR below is written"* — and the journal anticipated the fix. That honesty
is why the cost here is a review rather than a rollback.

**So this ADR is a review, not a rubber stamp.** Every decision below is stated
as a decision, and the section "What was already built" says plainly which ones
the code had already made and whether they stand.

## Decision

**alo sells domains as a reseller over Openprovider**, an EU wholesale
registrar established in the Netherlands, whose membership we already hold.

**We never seek our own ICANN accreditation.** At our volume it buys nothing
and costs accreditation fees, compliance surface, escrow obligations and a
registry relationship per TLD. The wholesaler is the correct layer.

**The customer is the registrant. We are never the registrant.** The buyer's
own name and address go to the registry through a contact handle of their own;
alo is the reseller of record and nothing more. A customer who leaves takes
their domain with them, and transfer-out is never obstructed, delayed, or
priced as a penalty. A sovereignty product that made itself the owner of its
customers' names would be selling the opposite of what it advertises — this is
the single most important line in this ADR.

**Registrar sovereignty is checked, not assumed.** A registrar whose
established country is outside the EU/EEA is refused at construction, not
warned about. (Noted without comment elsewhere: alo's *own* two domains are
registered at a US registrar. That is a separate decision and this ADR does not
launder it.)

**DNS ships in two phases, and phase 1 is the shortcut that unblocks this.**

1. **Phase 1** — register the name *and* create its zone at Openprovider over
   the same API, and alo writes the mail and site records the moment the
   purchase completes. "Live in minutes" without running our own nameservers.
2. **Phase 2** — alo-run authoritative DNS (ADR 0013), with phase-1 zones
   migrating over and bought names pointing at our nameservers.

This collapses "alo-run authoritative DNS" from a blocking prerequisite into an
eventual one. Without it the feature waits on a whole DNS build; with it the
feature ships and the DNS build improves it later.

**Retail posture** (already in `features.md`, restated here because pricing is a
promise): honest flat pricing, **no first-year-bait renewals**, thin margin by
design. This is the onboarding and retention closer, not a profit line. A
renewal price that is not the price shown at purchase is the industry's
standard deception and we do not adopt it.

**Environments:** every development and loop test runs against the sandbox
(CTE). The live endpoint is reachable only from production configuration, and
the seam reports whether its calls spend money so a test can assert it is not
about to.

## On naming their API in here

The question of whether to write the endpoint mapping into this ADR was asked
directly, and the answer is a rule worth keeping:

- **This ADR names the dependency surface** — the operations we bind to:
  identity, catalog, availability search, quote, register, renew, lookup, and
  zone management. That surface is the coupling, and coupling is what a
  decision record is for.
- **The endpoint mapping stays in `docs/guides/openprovider-domains.md`**,
  because it tracks a vendor's swagger and will drift. A decision record that
  goes stale teaches people to stop trusting decision records.
- **Credentials appear in neither.** Server environment only. This repository
  is public, and the guide already says so; it is repeated here because it is
  the kind of rule that is only ever broken once.

The binding is expressed in our own vocabulary (`DomainRegistrar`), not theirs,
so a second wholesaler — or a replacement — is a new implementation of a trait
rather than a rewrite of the product.

## What was already built, and whether it stands

Read off the tree rather than recalled:

| Already decided in code | Verdict |
|---|---|
| `DomainRegistrar` trait: identity, catalog, search, quote, register, renew, lookup | **Stands.** The right seam, and vendor-neutral |
| `register` and `renew` idempotent under an explicit key | **Stands.** Registration is money and a name; at-most-once is not optional |
| `RegistrarIdentity::new` refuses a non-EEA country | **Stands**, and is promoted here from an implementation detail to a decision |
| An environment that reports whether it `spends_money()` | **Stands.** This is what keeps a test from buying a domain |
| Purchase lifecycle: quoted → approved → awaiting_payment → paid → registering → registered → configured, plus cancelled and failed | **Stands.** Approval is a distinct state from payment, which is what makes "approve the price you saw" enforceable |
| A retail price derived from the wholesale price | **Stands**, subject to the no-bait-renewal rule above |
| Billing holds an opaque payment reference; the registrar knows nothing about money | **Stands.** The right direction of ignorance |
| Registrar shipped `unconfigured` in production, fixtures only in tests | **Stands**, and is the reason this ADR is a review rather than an incident |

**Nothing found that this ADR would have decided differently.** That is a good
outcome and not an argument that the order did not matter: it was luck that the
build made defensible choices unsupervised, and the next feature built without
its gate may not.

## Still open — these are prerequisites, not details

- **The EU PSP checkout** (Billing extension). Nothing can be bought until money
  can be taken.
- **Which TLDs we offer at launch**, and the registry-specific requirements each
  one imposes on registrant data.
- **Non-payment and expiry**: what the customer sees, how long the grace period
  is, and the fact that a lapsed domain must warn loudly and early rather than
  quietly.
- **The `SITE_REGISTRAR` configuration value** that finally wires this, which is
  the queue item that comes *after* this ADR — in that order this time.

## Rejected

- **Our own ICANN accreditation** — cost and compliance surface out of all
  proportion to the volume, and it would make us the party the registry holds
  responsible for every customer's name.
- **A US-based reseller API** — cheaper integration, and it would put the
  administration of our customers' domains under a non-EU jurisdiction in a
  product sold on sovereignty.
- **Not selling domains at all** — the alternative is telling a new customer to
  go to a registrar, buy a name, and come back to paste DNS records. That is
  the exact onboarding cliff this product exists to remove.
