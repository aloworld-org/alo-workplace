# 0051 — Native Outlook: do not build MAPI

**Status:** proposed
**Date:** 2026-08-21
**Amends** the "Exchange adapters (EAS/MAPI) become edge translators to JMAP,
year two" clause of [ADR 0001](0001-jmap-native-core.md), and closes the ROADMAP
item *"MAPI-over-HTTP adapter: native Outlook — the last wall"* as **will not
build**.

## Context

The goal: an Outlook user types an address and a password and is working, with
no manual server settings — what Exchange 2019 on-premise gives them over 443.
The exit gate says *"An Outlook desktop user works a full week against alo
without knowing Exchange is gone."*

That framing was written when classic Outlook was the client. It no longer is,
and the two facts below change the answer rather than the schedule.

### Fact 1 — the client MAPI serves is being retired

New Outlook for Windows became **the default in April 2026**. Classic Outlook
keeps support "through at least 2029"; full retirement is announced but
undated, and new installations will ship only the new client.

### Fact 2 — the replacement client will never speak MAPI to us

New Outlook **is not MAPI-compliant** and **does not support on-premises,
hybrid, or sovereign Exchange deployments** — with no published timeline. It
supports Microsoft 365, Outlook.com and Gmail accounts.

Taken together: a MAPI-over-HTTP adapter would work **only** with classic
Outlook — a client that is no longer the default, is on a declared path to
retirement, and whose successor cannot use the protocol at all. We would spend
the most expensive engineering years we have to land on a shrinking install base
with a hard ceiling and no forward path.

### What that costs, for completeness

MAPI-over-HTTP is documented (`[MS-OXCMAPIHTTP]`), so this is not
reverse-engineering — but the transport is the easy part. A working store pulls
in ROP (`[MS-OXCROPS]`), `[MS-OXCSTOR]`, `[MS-OXCFOLD]`, `[MS-OXCMSG]`,
`[MS-OXCTABL]`, `[MS-OXCFXICS]` for incremental sync, and `[MS-OXPROPS]` —
roughly two thousand properties — plus NTLM/Negotiate or OAuth. OpenChange, the
only serious open-source attempt, ran about a decade with more people than we
have and never reached dependable parity; Zentyal shipped it and dropped it.

Even if we succeeded, Fact 2 caps the return.

### ActiveSync fails twice

Smaller (WBXML over HTTP, mail + calendar + contacts together), but: it is
patent-licensed by Microsoft, which sits badly with [AGPL](0002-agpl-dual-license.md)
and with a product whose pitch is not paying Microsoft; and current Outlook for
Windows does not use it as a store. It would buy phones, which already work over
IMAP/CalDAV/CardDAV.

## Decision

**Do not build MAPI-over-HTTP or ActiveSync.** Instead:

1. **alo's own clients are the answer.** Web and the Tauri desktop app over JMAP
   on 443. This is where Exchange refugees land, and it is the only surface where
   we control the experience end to end.
2. **IMAP + SMTP + Autodiscover is the bridge for holdouts.** Outlook
   self-configures an IMAP account from Autodiscover XML — address and password,
   no manual settings. The endpoints are built and wire-verified on production;
   what remains is per-email-domain DNS and vhosts, already scoped as operator
   work. **Microsoft's own guidance to on-premises customers on new Outlook is
   to use IMAP** — we are pointing where they point.
3. **Calendar and contacts** via CalDAV/CardDAV. In classic Outlook this needs an
   add-in; in new Outlook nothing native exists. State that plainly.
4. **Revisit only on a funded contract** that names MAPI and pays for it, and
   even then behind a two-week read-only spike gate.

**Ports.** None of this needs SMTP or IMAP moved off 587/465. alo's own clients
already run entirely over 443 via JMAP. A 443-only customer firewall is a
separate and much smaller decision — TLS ALPN/SNI multiplexing ahead of Caddy —
and must not be smuggled into this one.

## Consequences

- We do not spend two years imitating a protocol whose client is being retired.
- **The strategic read inverts.** New Outlook dropping on-premises Exchange means
  Microsoft is pushing its own on-prem customers off the platform. Those
  customers must move somewhere. That is alo's opening — and it is won by being
  a better client than new Outlook, not by impersonating an Exchange server the
  new client refuses to talk to anyway.
- Honest gap, stated rather than implied: calendar and contacts inside classic
  Outlook need an add-in, and inside new Outlook are not available at all. Our
  answer is to make people want alo's own client, not to claim parity we do not
  have.
- If native Outlook calendar parity is ever contractually required, it is a
  funded product decision with a spike gate — never an engineering task absorbed
  into a sprint.

## Sources

- [Stages of migration to new Outlook for Windows — Microsoft Learn](https://learn.microsoft.com/en-us/microsoft-365-apps/outlook/get-started/guide-product-availability)
- [New Outlook: MAPI / Exchange — Microsoft Community Hub](https://techcommunity.microsoft.com/discussions/outlookgeneral/new-outlook-mapi--exchange/4157034)
- [Why the New Outlook for Windows Doesn't Support On-Premises Exchange (Yet)](https://en.ittrip.xyz/ms-office/outlook/new-outlook-onprem-exchange)
- [Microsoft Sets New Deadline for Classic Outlook Retirement — TechRepublic](https://www.techrepublic.com/article/news-microsoft-extends-classic-outlook-retirement-deadline/)
