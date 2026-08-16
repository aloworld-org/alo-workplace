# ADR 0050 — The first mail alo sends a stranger: the ticket email

**Status:** accepted (2026-08-16)
**Context:** ADR 0034 (agents draft, humans send), ADR 0041 (alo Commerce),
queue item S3.04h, `platform/alo-store/src/site_ticket_fulfil.rs`,
`products/mail/alo-jmap/src/submission.rs`

## The question

A buyer pays for a ticket on a tenant's website and expects it in their
inbox. Until now alo has never mailed a tenant's stranger automatically:
ADR 0034 routes even purchase orders through a human's Drafts, and the
site notification sweeps deliver *inward* to the owner's own inbox only.
The exceptions that do exist are platform mail to the platform's own
counterparties — signup and password-reset codes (`noreply@` the personal
domain, through the trusted submission listener), calendar iMIP on a
user's own behalf, DSNs and DMARC reports from the MTA itself. None of
them lets a *tenant's data* choose a recipient.

The ticket email is exactly that, so before the code exists this ADR
decides the four things an automated outbound path must have decided:
sender identity, DKIM domain, rate caps, bounce handling.

## Decision

**Automated mail to a stranger is allowed only as the transactional
consequence of an action that stranger took and paid for themselves.**
One recipient per event, an address the recipient entered at purchase,
content that is the receipt of their own act. Nothing here opens a
broadcast path; campaigns (ADR 0044) remain human-sent and would need
their own ADR with suppression lists and VERP before touching this seam.

### Sender identity: the platform's, never a tenant's

Every ticket email is sent from **one deployment-owned transactional
address**, configured as `ALO_SITES_MAIL_FROM` (production:
`tickets@alomails.com`), used as both the envelope `MAIL FROM` and the
header `From`. The site's name appears only in the From *display name*
(RFC 2047-encoded, so it cannot inject structure), and the site owner's
own primary address appears only as `Reply-To`, resolved inside the
sale's own tenant — so a buyer's reply reaches the seller, but no
tenant-controlled value ever becomes a sending address.

**Rejected: sending as the site owner's own identity.** Automated mail
bearing a human's From without their per-message intent breaks the
promise ADR 0034 makes (a human sends what leaves in their name), and a
bug in a sweep would spend a real person's deliverability reputation.
**Rejected: a per-site `no-reply@{subdomain}` From** (the display
identity the internal notifications use): sites subdomains have no mail
authority — no SPF, no DKIM key, guaranteed DMARC misalignment — so the
mail would be born spam.

### DKIM: inherited, not invented

Ticket mail leaves through the **same trusted internal submission
listener as every other platform mail** (`submission::submit`, the
signup-code path), which adds Date/Message-ID and DKIM-signs by the
From domain — the deployment's key material that already exists. The
configured From must therefore be on a domain the deployment signs for;
no new keys, no second signing path.

### Rate caps: money first, then a ceiling

- **A mail requires a paid order.** The recipient address comes only
  from an order in state `paid`; payment is the natural throttle, and
  each message costs the sender real money movement.
- **At-most-once per sale.** The claim marks the fulfilment row mailed
  in the same statement that selects it; a crash between claim and send
  loses a mail but can never duplicate one — the ticket stays reachable
  on the checkout return page and the order-status page, so a lost mail
  loses nothing irreplaceable.
- **A per-tenant daily ceiling** (200/day), enforced in the claim SQL:
  a tenant at the ceiling has its remaining mails *deferred* to the
  next 24-hour window, never dropped. Precision is per-round, so a
  burst can overshoot by at most one batch (25) — an ops guardrail,
  not accounting.

### Bounce handling: accept, don't automate

The envelope sender is the same transactional address, so DSNs land in
that mailbox for the operator to read (the deployment should keep it
deliverable). There is **no automated bounce parsing in v1**: with
at-most-once sending there is no retry loop to feed, and the volumes a
suppression list exists for are exactly the volumes this ADR refuses.
VERP and suppression are prerequisites for any future higher-volume
path, recorded here as that path's homework.

### The seam

An expand-only `mailed_at` column on `site_ticket_fulfilments`
(migration 0333); `Store::claim_ticket_mails` claims at-most-once,
after fulfilment has written the sale's description (so the mail quotes
the record of the sale, never a second copy of the price list); a sweep
in alo-jmap composes the message in the site's own language and submits
it. Unset `ALO_SITES_MAIL_FROM` and no mail leaves — the feature ships
**default-off**, and the off-switch is the same config.

Isolation is structural and tested: the claim joins each sale to its
own tenant's site and order; the Reply-To is resolved through
`for_tenant(sale.tenant)`, which answers `None` for any other tenant's
owner; the From never contains a tenant value at all. A foreign
tenant's sale cannot mail through another tenant's identity because no
tenant identity is ever on the wire.

## Verification discipline

No test, gate, or loop iteration ever performs live sending; the
compose is a pure function under test and the claim is proven against
the local database. The wire is only exercised by a human-present
deploy, like every other outbound surface.
