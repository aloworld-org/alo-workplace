# ADR 0044 — alo Campaigns: bulk email that cannot poison the mailbox

**Status:** accepted
**Date:** 2026-08-12
**Context:** ADR 0041 (commerce as a surface over one catalog), ADR 0040 (the
site chatbot), `docs/design/positioning.md`

## The decision in one line

Campaigns send from a **separate identity** — their own subdomain, DKIM
selector and IP — so a marketing reputation can never reach the domain that
carries invoices, password resets and meeting invitations; and the audience is
a **CRM query**, never a list that has to be kept in sync.

## Why this belongs to alo

**Mailchimp's weakness:** it is a silo, and it charges you for data you already
own. The list lives there, the customers live in your CRM, the invoices live in
your accounts, and you pay monthly — per contact — to keep three copies of the
same people agreeing with each other. Every merchant of any size ends up with
an integration, a Zapier bill, or somebody exporting CSVs on a Friday.

**Our move:** there is nothing to sync, because there is no list. A segment is
a query over contacts alo already holds:

> everyone who opened the last campaign, has not bought in ninety days, and is
> in Belgium

That sentence is a join across CRM, Billing and campaign events. In Mailchimp
it is impossible; in Mailchimp-plus-your-CRM it is a nightly export that is
wrong by Tuesday. And the loop closes on its own: who opened becomes activity
on the contact, who bought becomes an invoice, and the campaign's worth is
measured in revenue rather than in click-through.

The signup end already exists — `site_forms.rs` captures leads from a published
site — so the audience arrives without anybody typing it in.

## 1. The sending identity is separate, and this is the whole architecture

**Decision: bulk mail never shares a reputation with transactional mail.**

This is the one decision that cannot be deferred, because it is not a setting —
it is a set of DNS records and a routing path.

- Campaigns send from a **dedicated subdomain** (`news.customer.com`), with its
  own SPF, its own **DKIM selector**, and its own DMARC alignment.
- They leave by a **separate IP or pool** from transactional mail, and from a
  separate queue, so a campaign backlog cannot delay a password reset.
- A tenant's campaign reputation is **theirs**: per-tenant limits, per-tenant
  warm-up, and a tenant whose complaint rate rises is throttled before the
  shared infrastructure notices.

**Why this is not negotiable.** alo sends from its own mail stack, and the
whole trust stack — SPF, DKIM, DMARC, PTR — was proved on the wire before any
customer was let near it. One tenant mailing a bought list from the same IP
burns the deliverability of every other tenant's invoices. Reputation is the
product Mailchimp actually sells; the campaign editor is a commodity. We are
taking on the hard half knowingly, and the separation is how it stays
survivable.

*Rejected: "one domain, we will watch it".* Watching is not a control. By the
time a domain is on a blocklist, the mail that mattered has already bounced.

## 2. Consent is a record, not a checkbox

**Decision: every recipient carries the provenance of their consent — when,
from which form, from which address — and a campaign cannot be sent to somebody
without one.**

alo is sold on European sovereignty and on getting compliance right. A
self-service bulk sender with a casual consent model is self-refuting: it would
be the one feature that makes the pitch a lie. So:

- consent is captured with its source and timestamp, and shown on the contact;
- imported lists are the dangerous path and are treated as such — an import
  states where the addresses came from, and that statement is stored;
- **suppression is absolute and global to the tenant.** An unsubscribe, a hard
  bounce or a complaint removes somebody from every future send, and no
  segment, import or re-upload can bring them back.

## 3. Unsubscribing is one click, in the mail, every time

**Decision: every campaign carries `List-Unsubscribe` with one-click support
(RFC 8058), and the link works without a login.**

alo already implements RFC 8058 as a *reader* — `unsubscribe.rs` performs the
one-click POST server-side for mail a user receives. This is the same standard
from the other end, and the reason to do it properly is selfish as well as
legal: a recipient who cannot find the unsubscribe presses "spam" instead, and
that is the signal that ends a sending reputation.

## 4. Bounces and complaints come back, and are acted on

**Decision: feedback loops are day one, not hardening.** Hard bounces suppress
immediately. Soft bounces retry, then suppress. Complaints suppress and count
against the tenant's rate. A list that decays is a list that keeps arriving.

## 5. What is deliberately not built

- **No open-tracking pixel by default.** It is a per-campaign choice, off unless
  chosen, and disclosed. A sovereignty product that silently pixels every
  recipient has sold the same thing it complains about.
- **No third-party sending service.** That would put the customer list in the
  jurisdiction alo exists to leave.
- **No "AI writes your campaign" as the headline.** Everyone ships that. The
  differentiator is the segment and the attribution, which nobody else can
  build.

## Consequences

- Campaigns cannot ship before the sending-identity work: subdomain
  provisioning, a second DKIM selector, a separate queue and egress. That is
  the first slice, and it is infrastructure rather than screens.
- The editor itself is small, because sections and typed content already exist
  for Sites (ADR 0036) and can render an email as well as a page.
- The claim, which neither Mailchimp nor Odoo can make: **the person who
  received it, the invoice they paid, the deal it opened and the site page they
  landed on are the same product** — and the campaign's success is measured in
  money rather than in opens.
