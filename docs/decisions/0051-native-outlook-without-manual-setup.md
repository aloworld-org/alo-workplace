# 0051 — Native Outlook: a client-side connector, not a server-side MAPI stack

**Status:** proposed
**Date:** 2026-08-21
**Sequences** the "Exchange adapters become edge translators to JMAP" clause of
[ADR 0001](0001-jmap-native-core.md) and replaces the ROADMAP item
*"MAPI-over-HTTP adapter"* with a different shape of the same goal.

## Context

The goal: an Outlook user types an address and a password and is working, with
no manual server settings, keeping the client they already know.

Two readings of the market were considered, and the second is correct.

**The wrong reading** (recorded because it was mine first): new Outlook became
the default in April 2026, classic Outlook retires "at least 2029", and new
Outlook is not MAPI-compliant and does not support on-premises, hybrid or
sovereign Exchange at all. Conclusion drawn: do not serve classic Outlook.

That conclusion follows Microsoft off a cliff. The same facts read properly say
the opposite:

**The right reading.** Microsoft is evicting its own on-premises customers and
handing them a client many of them dislike. Those customers are precisely who
alo is for — a sovereign, self-hosted workspace. They want to keep classic
Outlook, and classic Outlook is a desktop application: when Microsoft stops
shipping it, it does not stop working. **Whoever keeps it working against a
non-Microsoft server inherits those customers.** That is a differentiator
Microsoft is handing us, not a legacy burden.

alo is an on-premises product. Copying the retirement schedule of a cloud vendor
whose customers we are trying to take is strategy by imitation.

## The distinction that decides this

"Support Outlook natively" has two implementations that differ by an order of
magnitude, and conflating them is what made the first analysis wrong.

**Server-side MAPI-over-HTTP** — impersonate Exchange on the wire. Documented
(`[MS-OXCMAPIHTTP]`), but the transport is the easy part: ROP (`[MS-OXCROPS]`),
`[MS-OXCSTOR]`, `[MS-OXCFOLD]`, `[MS-OXCMSG]`, `[MS-OXCTABL]`, `[MS-OXCFXICS]`,
and `[MS-OXPROPS]`'s ~2000 properties, plus NTLM/Negotiate. **OpenChange spent
about a decade here and never reached dependable parity; Zentyal shipped it and
dropped it.** This is the grave.

**A client-side connector** — a MAPI service provider installed on the PC that
translates Outlook's MAPI calls into calls *our own server* understands, with a
local cache and a sync engine. This is what **Zimbra's ZCO** does, and it ships:
mail, folders, tags, contacts, personal calendars, appointment reminders and
tasks, with offline use via a local store. Kopano and Open-Xchange took the same
route.

The difference is not effort-in-degree, it is kind. Server-side means
reimplementing Microsoft's protocol exactly, against a spec you do not control,
where every deviation is a bug in *your* product. Client-side means implementing
Outlook's client interfaces and translating to **JMAP, which we own** — both ends
of the translation are ours, and the wire between connector and server is our
API, over 443.

## Decision

**Build a client-side Outlook connector. Do not build server-side MAPI-over-HTTP
or ActiveSync.**

Sequenced so customers are served before the connector lands:

1. **Now — IMAP + SMTP + Autodiscover.** Outlook self-configures from
   Autodiscover XML: address and password, no manual settings. Endpoints are
   built and wire-verified on production; what remains is per-email-domain DNS
   and vhosts, already scoped as operator work. This makes classic *and* new
   Outlook usable for mail today.
2. **Next — CalDAV/CardDAV** for calendar and contacts, which in Outlook needs
   an add-in until (3). State that gap plainly.
3. **The project — the alo Connector for Outlook.** A MAPI service provider
   translating to JMAP, with a local cache for offline use. Written in **Rust**
   as a COM in-process server, so the two-language rule holds
   ([ADR 0001](0001-jmap-native-core.md)); if that proves impossible the
   exception needs its own ADR, not a quiet import of C++.
   Staged: mail and folders → contacts → calendar → tasks/reminders → offline
   cache. Each stage is independently shippable and independently useful.

**ActiveSync stays rejected:** patent-licensed by Microsoft, which sits badly
with [AGPL](0002-agpl-dual-license.md) and with a product whose pitch is not
paying Microsoft; and current Outlook does not use it as a store.

**Ports.** The connector talks HTTPS to alo, so it satisfies a 443-only firewall
by construction — the question that started this. Nothing needs SMTP or IMAP
moved off 587/465, and alo's own clients already run entirely over 443 via JMAP.

## Consequences

- The connector is still a large, multi-stage project. The difference from the
  rejected option is that **this shape has shipped elsewhere and that one has
  not** — we are taking a proven route, not attempting what OpenChange could not
  finish.
- It serves classic Outlook only. New Outlook supports neither MAPI nor
  connectors. That is acceptable and deliberate: classic is where the customers
  we want actually are, and it keeps running after Microsoft stops shipping it.
- Every stage before the connector still pays off — Autodiscover and
  CalDAV/CardDAV serve Thunderbird, Apple Mail, iOS and Android regardless.
- We should say publicly what this is: **keep the Outlook you know, lose the
  Microsoft server.** Microsoft is creating that demand for us.

## Sources

- [Zimbra Connector for Outlook — Administration Guide](https://zimbra.github.io/zm-windows-comp/latest/ZCS_Connector_For_Outlook_Admin_Guide.html)
- [Zimbra Connector for Outlook — User Guide](https://zimbra.github.io/zm-windows-comp/latest/ZCS_Connector_For_Outlook_User_Guide.html)
- [Stages of migration to new Outlook for Windows — Microsoft Learn](https://learn.microsoft.com/en-us/microsoft-365-apps/outlook/get-started/guide-product-availability)
- [New Outlook: MAPI / Exchange — Microsoft Community Hub](https://techcommunity.microsoft.com/discussions/outlookgeneral/new-outlook-mapi--exchange/4157034)
