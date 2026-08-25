# ADR 0055 — Outlook support is a migration bridge, not a destination

**Status:** accepted
**Date:** 2026-08-25
**Decided by:** the owner, revisiting their own earlier decision on the argument
recorded below.
**Supersedes** the parity goal of
[ADR 0051](0051-native-outlook-without-manual-setup.md). The transport, the
crate, the translate-to-JMAP rule and the stages already shipped all stand; what
changes is where the work stops and what we promise.

## Context

ADR 0051 committed to Exchange-class client access: MAPI-over-HTTP on 443, so a
business running Exchange 2019 could replace the server and every existing
Outlook desktop would carry on as though nothing had happened. It priced the
work honestly — "the largest single piece of work in the product" — and recorded
the precedent against it: OpenChange spent roughly a decade and never reached
dependable parity; Zentyal shipped it and dropped it.

That decision answered a question about **migration cost**. It never answered a
question about **what we are selling**, and that is the question that has now
been asked:

> We want to be a competitor of Outlook. We need an application independent of
> Outlook, not an application that works with Outlook.

Half of that needs correcting and half of it is right, and the ADR being
superseded addresses neither.

**The correction.** alo competes with **Exchange Server**, not with Outlook.
MAPI-over-HTTP is a server protocol. Implementing it does not make alo depend on
Microsoft's client; it lets a customer delete Exchange without touching two
hundred desktops on the same day. Outlook is what the customer already owns, not
a partner we would be plugging into.

**The part that is right, and that ADR 0051 does not address at all.** If
Outlook works flawlessly against alo, the customer keeps using Outlook. alo's
own client — the AI-native surface that is the whole reason the product exists —
never gets opened. We would have built a cheaper Exchange and would be competing
with Microsoft on price, with our actual advantage invisible to the person
paying for it. Perfect compatibility is not a neutral convenience: it is a
reason for the customer never to arrive.

### What has changed since ADR 0051 was accepted

Four days of building, and the facts are no longer theoretical.

- Stage 6 (address book) is partial: name resolution works, directory browsing
  is refused rather than half-answered.
- Stage 7 (submission) shipped without attachments on outgoing mail and without
  reply threading.
- Stages 8 (ICS/FastTransfer, cached mode) and 9 (calendar, contacts and tasks
  as native MAPI classes) are unbuilt. Stage 8's producer is blocked on byte
  layouts that the specification does not settle.
- `docs/interop.md` states in two places that the work is **"not yet confirmed
  against a real Outlook"**.

That last point deserves naming plainly, because it bears on the kill gate.
ADR 0051 defines every stage as *observable Outlook behaviour verified on the
wire, never "the spec is implemented"*, and names stage 5 as the criterion:
reach it or stop and ship the connector. Stage 5 is recorded as passed. No real
Outlook profile has ever completed against alo. Whatever the explanation —
verified and not written down, or marked passed on conformance rather than a
client — a kill gate that was never actually run cannot be what authorises
spending more.

## Decision

**Freeze MAPI-over-HTTP at what stage 7 delivers, and declare it a migration
bridge with an end, not a parity target.**

Concretely:

1. **What stays and keeps working.** Autodiscover, the transport, logon, the
   folder tree, the contents table, opening and reading a message with its
   attachments, name resolution, and submission. Outlook opens, reads and sends
   against alo. This is real and it is enough to move a business.
2. **What we stop building.** Stage 8 (cached mode and offline) and stage 9
   (calendar, contacts and tasks as native MAPI classes). Stage 6's directory
   browsing and stage 7's outgoing attachments and reply threading are
   reclassified from "not yet" to "not planned", unless a paying customer's
   migration is actually blocked on one.
3. **What we promise, in exactly these words.** *Your Outlook still opens your
   mail while you move.* Not "Outlook works like it did with Exchange". The
   qualification is deliberate and goes in the sales material, not in a footnote
   discovered later.
4. **Where the effort goes instead.** alo's own client. That is where the
   product is differentiated and it is the only surface on which we can win
   rather than tie.
5. **The bridge has an end.** It is supported for migration and kept working; it
   is not extended. If it starts costing maintenance out of proportion to the
   customers on it, it is removed rather than nursed.

### Why not the two alternatives

- **Continue to parity.** The cost was priced honestly in ADR 0051 and has not
  fallen. Two teams with more people have failed at it. And on the strategic
  argument above, succeeding is its own problem: the better it works, the less
  reason anyone has to open alo.
- **Delete it entirely.** Sharpest strategically, and the cheapest — IMAP, POP3,
  SMTP, CalDAV and CardDAV are shipped and live today, and they serve
  Thunderbird, Apple Mail, iOS and Android. But it throws away work that already
  functions, and it hands the customer a migration to think about on day one.
  ADR 0051 is right that thinking about it is where migrations die.

Freezing keeps the part that removes the objection and stops paying for the part
that removes the reason to switch.

## Consequences

- **The honest promise is narrower, and must be stated narrowly.** Without
  stage 8 there is no offline or cached mode: Outlook works online against alo.
  Without stage 9, calendar and contacts are not native MAPI classes in Outlook
  — they are served by CalDAV and CardDAV, which every other client uses. A
  customer who hears "Exchange parity" and finds this has been mis-sold, and
  that is a worse outcome than not having built it.
- **The kill gate is settled before anything else is spent.** One real classic
  Outlook profile against `mail.alomails.com`, on the wire, recorded in
  `docs/interop.md`: what works, what does not, verbatim. This is a
  half-day of work and it converts an unverified claim into a fact. It happens
  regardless of this ADR, because we are already telling people the mail opens.
- **`ROADMAP.md` changes shape.** Stages 8 and 9 move out of the checklist and
  are recorded as deliberately not built, with this ADR as the reason. A phase
  is not left permanently un-exitable by items we have decided against.
- **The fallback in ADR 0051 becomes the main road.** IMAP/SMTP/CalDAV/CardDAV
  are no longer the contingency if a MAPI stage proves impassable; they are how
  every non-alo client is expected to connect, and they are finished.
- **Reversible, at a price.** The crate, the transport and the stage work all
  remain. If a customer with real money is blocked on cached mode, stage 8
  resumes from where it stopped — with the byte-layout questions still open and
  a real Outlook to answer them against.
- **What this ADR does not change.** MAPI stays an edge translator over the one
  JMAP-native store (ADR 0001), in its own crate, on 443. No second source of
  truth, and no fork of the store, ever.
