# 0051 — Native Outlook without manual setup

**Status:** proposed
**Date:** 2026-08-21
**Supersedes nothing.** Sequences the "edge translators to JMAP, year two"
clause of [ADR 0001](0001-jmap-native-core.md) and the ROADMAP item
*"MAPI-over-HTTP adapter: native Outlook — the last wall"*.

## Context

The goal stated: an Outlook user types their address and password and is
working, with no manual server settings — what Exchange 2019 on-premise gives
them over TCP 443. The exit gate already says it: *"An Outlook desktop user
works a full week against alo without knowing Exchange is gone."*

Three protocols could deliver it, and they are not close in cost.

### MAPI-over-HTTP — what Exchange 2019 actually uses

Documented, not reverse-engineered: `[MS-OXCMAPIHTTP]` defines the transport.
That is the easy part. The payload is ROP (`[MS-OXCROPS]`), and a working store
needs `[MS-OXCSTOR]`, `[MS-OXCFOLD]`, `[MS-OXCMSG]`, `[MS-OXCTABL]`,
`[MS-OXCFXICS]` for incremental sync, and `[MS-OXPROPS]` — roughly two thousand
properties. Authentication is NTLM/Negotiate or OAuth2; modern Outlook refuses
Basic.

**The evidence that matters:** OpenChange, the only serious open-source
implementation, ran for about a decade with more people than we have and never
reached dependable parity. Zentyal shipped it, then dropped it. Treating this as
"months" is the mistake that kills the quarter — it is the hardest single thing
in the product, and the one where failure is least visible until late.

### Exchange ActiveSync — smaller, but two traps

Far simpler: WBXML over HTTP, one spec family, and it carries mail, calendar and
contacts together. Two problems, either of them disqualifying on its own:

1. **Licensing.** EAS is patent-licensed by Microsoft. A per-implementation
   licence sits badly with AGPL ([ADR 0002](0002-agpl-dual-license.md)) and with
   a sovereignty product whose pitch is not paying Microsoft.
2. **Outlook for Windows does not use it.** EAS is a phone protocol. Outlook
   2013/2016 tolerated EAS accounts; current Outlook does not offer it as a
   store. It would buy us phones — which already work over IMAP/CalDAV/CardDAV.

### IMAP + SMTP + Autodiscover — already built

Outlook self-configures an IMAP account from Autodiscover XML. Type address and
password, done — no manual settings. That is **the literal goal for mail**, and
the endpoints exist and are wire-verified on production today. What is missing
is operator work already named in the ROADMAP: per-email-domain `autodiscover`
DNS records and Caddy vhosts, so a real client resolves it from the *email*
domain rather than the server FQDN.

What it does not give: calendar and contacts inside Outlook natively, and
server-side search//rules parity.

## Decision

**Sequence, do not leap.**

1. **Finish Autodiscover per email-domain** (operator/deploy, already scoped).
   Delivers "no manual setup" for mail in Outlook now.
2. **Calendar and contacts** via CalDAV/CardDAV. Outlook needs an add-in for
   this; that is a real gap, and honest to state rather than hide.
3. **Re-evaluate MAPI-over-HTTP as its own funded project** with a spike before
   any commitment: a read-only mailbox that Outlook can open, nothing more. If
   the spike does not land in two weeks, the ten-year OpenChange lesson applies
   and we stop.

**Ports.** Nothing here needs SMTP or IMAP moved to 443. alo's own web and
desktop clients already do everything over 443 via JMAP; only third-party
clients use 587/465/993. If a customer's firewall is 443-only, that is a
separate, smaller decision — TLS ALPN/SNI multiplexing in front of Caddy — and
it should not be smuggled into this one.

## Consequences

- Outlook users get working mail with no manual setup **without** a year-two
  project starting now.
- We state plainly that calendar/contacts in Outlook need an add-in until an
  adapter exists. A sovereignty product does not win by implying parity it does
  not have.
- MAPI stays on the roadmap as the last wall, with a spike gate in front of it
  so it cannot quietly consume a quarter.
- If native Outlook calendar parity is required *sooner* than the sequence
  allows, that is a product decision to fund the MAPI project explicitly — not
  an engineering task to absorb.
