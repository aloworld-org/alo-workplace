# 0051 — Exchange-compatible client access: MAPI-over-HTTP on 443

**Status:** accepted, but the **parity goal is superseded by
[ADR 0055](0055-outlook-is-a-bridge-not-a-destination.md)** — Outlook support
stops at what stage 7 delivers (open, read, send) and is a migration bridge
rather than Exchange parity; stages 8 and 9 are not built. Everything below
about the transport, the crate, translating to JMAP rather than forking the
store, and the stages already shipped still stands. The goal they were serving
does not.
**Date:** 2026-08-21
**Decided by:** the owner, explicitly and repeatedly, with the cost below stated
and accepted. Revisited by the owner on 2026-08-25, on an argument this ADR
never addressed: that perfect Outlook compatibility is itself a reason for the
customer never to open alo's own client.
**Realises** the "Exchange adapters become edge translators to JMAP" clause of
[ADR 0001](0001-jmap-native-core.md) and the ROADMAP item *"MAPI-over-HTTP
adapter: native Outlook — the last wall"*.

## Context

alo replaces Exchange Server on premises. For a business running Exchange 2019,
the mail server is the thing Outlook connects to on **443** — MAPI-over-HTTP,
Autodiscover, EWS, OAB — with no manual configuration and no per-desktop
install. Anything short of that is a migration the customer has to think about,
and thinking about it is where migrations die.

Two cheaper shapes were considered and rejected **by the owner** as not the
goal:

- **IMAP + SMTP + Autodiscover** — works today, self-configures, but it is a mail
  protocol, not Exchange. No native calendar, contacts, tasks, or server-side
  rules in Outlook.
- **A client-side connector** (the Zimbra ZCO shape) — proven, smaller, but it is
  a per-desktop MSI. An Exchange replacement that requires installing software
  on every workstation is not a drop-in replacement.

The decision is to build the server, not to work around it.

## Decision

**Implement MAPI-over-HTTP server-side, in Rust, terminated on 443 alongside the
existing surfaces**, translating to the JMAP-native core rather than to a second
store. Outlook connects to alo the way it connects to Exchange 2019: address and
password, Autodiscover, done.

### The specification surface, stated honestly

This is the largest single piece of work in the product. It is fully documented
under Microsoft's Open Specifications — this is **not** reverse engineering —
but the documentation is the volume, not the difficulty:

| Spec | What it covers |
|---|---|
| `[MS-OXDSCLI]` | Autodiscover — the response that points Outlook at MAPI/HTTP |
| `[MS-OXCMAPIHTTP]` | The transport: `Connect`, `Execute`, `Disconnect`, `NotificationWait`, session contexts, chunked responses |
| `[MS-OXCROPS]` | ROP layer — the operations themselves |
| `[MS-OXCSTOR]` | Logon, store properties, per-mailbox semantics |
| `[MS-OXCFOLD]` | Folders and hierarchy |
| `[MS-OXCMSG]` | Messages, attachments, recipients |
| `[MS-OXCTABL]` | Tables, restrictions, sorting — how Outlook reads a view |
| `[MS-OXCFXICS]` | Incremental sync + FastTransfer — what makes cached mode work |
| `[MS-OXPROPS]` | The property canon, ~2000 entries |
| `[MS-OXNSPI]` | Address book (NSPI) over MAPI/HTTP |

**Known risk, accepted.** OpenChange attempted this for roughly a decade with a
larger team and never reached dependable parity; Zentyal shipped it and dropped
it. We proceed with that on the record, and with the mitigations below — not by
assuming we are cleverer.

### Mitigations that make it survivable

1. **Vertical slices against a real Outlook, always.** Every stage ends with
   "classic Outlook does X against alo", verified on the wire — never "the spec
   is implemented". The `interop-tester` agent exists for exactly this.
2. **Translate to JMAP, never fork the store.** MAPI is an edge protocol over
   the one store ([ADR 0001](0001-jmap-native-core.md)). A second source of
   truth is how this becomes unmaintainable.
3. **Its own crate, `products/mail/alo-mapi`**, behind its own port/route, so a
   half-built adapter can never destabilise mail that works today.
4. **Staged, each stage independently useful**, so the project has value before
   it has parity.
5. **Read-only before read-write.** A mailbox Outlook can open and read is the
   first honest milestone; sending and mutating come after.

### Stages

| # | Milestone — stated as observable client behaviour |
|---|---|
| 1 | Autodiscover returns a `mapiHttp` protocol block; Outlook stops asking for manual settings |
| 2 | `Connect`/`Execute`/`Disconnect` envelopes; Outlook completes the handshake and authenticates |
| 3 | `Logon` + folder hierarchy; Outlook draws the folder tree |
| 4 | Contents tables; Outlook lists messages in a folder |
| 5 | `OpenMessage` + streams; Outlook opens and reads a message, attachments included |
| 6 | NSPI; the address book resolves recipients |
| 7 | Submission; Outlook sends |
| 8 | ICS/FastTransfer; cached mode and offline work |
| 9 | Calendar, contacts, tasks as native MAPI classes |

**Authentication:** Basic over TLS first (Outlook accepts it against a
non-Microsoft endpoint), NTLM/Negotiate assessed at stage 2 — never Basic
without TLS.

**Ports:** terminated on 443 next to the existing routes. Nothing moves off
587/465, and alo's own clients keep running over JMAP on 443.

## Consequences

- The largest project alo has undertaken. It is funded as a project, not
  absorbed into a sprint, and its stages appear in `ROADMAP.md` where progress
  is visible.
- Every earlier stage still pays off if the later ones stall: Autodiscover and
  the folder/contents work serve real Outlook users before parity exists.
- IMAP/SMTP/Autodiscover and CalDAV/CardDAV remain — they serve Thunderbird,
  Apple Mail, iOS and Android, and they are the fallback if a stage proves
  impassable.
- **The kill criterion is written down in advance:** if stage 5 — Outlook opening
  and reading a message from alo — is not reached, we stop and ship the
  connector instead. That is the point where OpenChange's difficulty becomes
  measurable rather than theoretical.

## Sources

- [MAPI — protocol overview](https://en.wikipedia.org/wiki/MAPI)
- [Enable or disable MAPI access to mailboxes in Exchange Server](https://learn.microsoft.com/en-us/exchange/clients/mapi-mailbox-access)
- [MAPI over HTTP in Exchange — Microsoft Learn](https://learn.microsoft.com/en-us/exchange/mapi-over-http-exchange-2013-help)
