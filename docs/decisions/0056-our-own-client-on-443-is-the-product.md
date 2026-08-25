# ADR 0056 — Our own client on 443 is the product; MAPI is retired

**Status:** accepted
**Date:** 2026-08-25
**Decided by:** the owner.
**Supersedes** [ADR 0051](0051-native-outlook-without-manual-setup.md) entirely,
and [ADR 0055](0055-outlook-is-a-bridge-not-a-destination.md), which held for a
day. The record of all three is kept deliberately: the reasoning moved twice in
one day, and the direction it moved in is the useful part.

## Context

ADR 0051 committed to Exchange-class client access — MAPI-over-HTTP on 443, so
an existing Outlook would carry on as though Exchange were still there. ADR 0055
narrowed that to a migration bridge: Outlook opens, reads and sends, and nothing
further gets built. Both answered *how much Outlook compatibility to build*.

The owner has now rejected the question rather than the answer:

> We need an app that works on TCP 443 entirely, not one that works with
> Outlook.

That is a different and better statement of what alo is. The compatibility
question assumed the customer's client stays and our server changes underneath
it. But the client is where alo is actually different — the AI-native workspace
is the product, and it is a thing you open, not a protocol you speak. Every hour
spent making somebody else's client work perfectly is an hour spent making our
own client unnecessary.

ADR 0055 tried to have both by making the bridge deliberately narrow. That was
half a position. A bridge still has to be maintained, still has to be explained
in every sales conversation, and still gives a customer a way to never arrive.

## Decision

**alo's own client, speaking JMAP over 443, is the product. MAPI-over-HTTP is
retired.**

1. **MAPI is switched off in production.** `ALO_MAPI_HTTP_ENABLED` is unset, so
   `/mapi/emsmdb` and `/mapi/nspi` stop being served and Autodiscover stops
   advertising a `mapiHttp` block. The gate is already built the right way round
   for this: both must open before anything is advertised, so switching it off
   is a configuration change and not a deploy of new code.
2. **`products/mail/alo-mapi` is deleted**, with its dependency from `alo-jmap`
   and its workspace entry. Twenty-one thousand lines of adapter for a protocol
   we have decided not to speak is not an asset; it is compile time, review
   surface, and a standing invitation to resume. Git holds it, ADR 0051 holds
   the design, and `docs/interop.md` holds what was learned on the wire — which
   is the part that would actually be expensive to rediscover.
3. **Open standards stay exactly as they are.** IMAP, POP3, CalDAV, CardDAV and
   SMTP submission are finished, live, and cost nothing to keep. They serve
   Thunderbird, Apple Mail, iOS and Android, and they are the difference between
   "our own client is the best way in" and "our own client is the only way in".
   A sovereignty product that locks its customers to one client has argued
   itself out of its own pitch.
4. **Port 25 is not a client concern and does not move.** Mail from the rest of
   the internet arrives over SMTP on 25, and a server that does not answer there
   stops receiving mail. "Entirely on 443" is a statement about how a *person*
   reaches their mail, never about how a *server* does.
5. **What "entirely on 443" obliges us to.** Everything a person needs — mail,
   calendar, contacts, chat, meet, drive, docs, the business modules, login,
   push — reachable from alo's own clients over 443 and nothing else. No
   sidecar port, no per-desktop install, no "you also need to configure X". That
   is already substantially true and it is now the standard a feature is held
   to, not an aspiration.

## Consequences

- **Outlook stops working against alo, and we say so plainly.** Not "not yet",
  not "on the roadmap" — we do not offer it. A customer running Outlook
  connects it over IMAP/SMTP with CalDAV and CardDAV alongside, or moves to
  alo's client. That is a real migration cost and it is now an owned position
  rather than a gap. ADR 0051 was right that thinking about a migration is where
  migrations die; the answer is to make arriving worth it, not to make leaving
  invisible.
- **The Phase 6 exit gate changes again**, and this is the last time. It asked
  for an Outlook user who never notices Exchange is gone; then for a business
  that migrates without a desktop visit. It now asks that a business runs on
  alo's own clients over 443. A gate should describe the product we intend, and
  ours has been describing Microsoft's.
- **Work is freed, and that is the point.** The largest project in the product,
  against a precedent of OpenChange failing for a decade and Zentyal shipping
  and dropping it, is no longer in front of us.
- **The kill gate is moot.** ADR 0051 made stage 5 the criterion to continue,
  and it was never actually run against a client. Retiring the adapter settles
  it in the only honest way now available: we did not pass it, and we are not
  continuing.
- **What was learned is kept.** `docs/interop.md` keeps the MAPI section — the
  Autodiscover header behaviour, the chunked-envelope layout, the two byte-order
  derivations. It is a record of specification reading that cost real time, and
  it is worth more than the code.
- **Reversal is expensive and that is intended.** Restoring the adapter means
  recovering the crate from git and re-establishing the flag. If a customer with
  real money is blocked on native Outlook, that is a decision to take
  deliberately, with a price attached, not a switch someone flips quietly.
