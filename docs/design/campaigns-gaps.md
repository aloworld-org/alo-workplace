# Campaigns — the complaints we can actually answer

Companion to ADR 0044 (alo Campaigns). Written to the rule in
`positioning.md`: **state a weakness and a move, never a size.**

The point of this file is to separate two things that get muddled whenever a
competitor is discussed. Some of Mailchimp's problems are **structural** — they
follow from the fact that it is a separate company holding a copy of your
customers — and it cannot fix them without ceasing to be what it is. Others are
merely places it is weak and we could be better, which is a much cheaper claim
and should never be sold as a moat.

There is also a third list, at the bottom, of the things it does better than we
will for a long time. That list is the honest one and it is the reason this is
a wave rather than a weekend.

---

## Structural — it cannot fix these without changing what it is

### 1. You pay per contact, including the ones you never mail

The complaint everyone has: billing is by audience size, so an address that
unsubscribed two years ago, or a duplicate, or somebody who will never be
mailed again, still costs money every month. Tier boundaries make it worse —
crossing a threshold by a handful of contacts steps the bill.

**Why it cannot be fixed there:** contact count *is* the meter. The business is
storing a copy of your list and charging rent on it.

**Our move:** there is no meter, because the contacts are already in the
workspace you pay for. A campaign is a query over people alo holds for CRM and
Billing anyway. Growing the address book costs nothing, so nobody is incentivised
to keep a dirty list — which is also, quietly, better for deliverability.

### 2. The same person in two audiences is two contacts

Audiences are separate databases. A customer who is on the newsletter and in
the webinar list is billed twice and unsubscribes once, from one of them. The
workaround everyone lands on — one audience plus tags — is folklore, not
design.

**Our move:** impossible by construction. There are no lists to be in. A person
is one contact; a segment is a question asked about them. Two segments
overlapping is not two records, and unsubscribing is a property of the person.

### 3. Keeping the list and the CRM agreeing

The list lives there, the customers live in your CRM, the invoices live in your
accounts. Every business of any size ends up with an integration, a Zapier
bill, or somebody exporting CSVs on a Friday — and the export is wrong by
Tuesday.

**Our move:** nothing to sync. This is ADR 0044's central claim and it is only
credible because CRM (wave B2) and Billing (B1) are already built and already
in the same database. *"Bought in the last 18 months but not the last 90 days,
in Belgium"* is a join, not an integration.

### 4. A dedicated sending identity is an enterprise upsell

Shared IP pools mean your deliverability is partly decided by strangers, and a
dedicated IP is priced for large accounts.

**Our move:** ADR 0044 makes the separate identity — own subdomain, own DKIM
selector, own IP or pool — **the architecture, not a tier**. It exists because
bulk mail must never share a reputation with the domain carrying invoices and
password resets, which is a correctness requirement rather than a feature. That
it also answers a pricing complaint is a side effect.

### 5. Segmentation and automation gated by plan

The useful segment conditions and the multi-step journeys sit behind higher
tiers. This is the standard SaaS shape: give away the part that is cheap to
run, charge for the part that is valuable.

**Our move:** a segment is a query over data the customer already owns, running
on infrastructure they already pay for. There is no cost basis for gating it,
and gating it would be charging twice for their own records.

### 6. Open rates stopped meaning anything, and it is still the headline number

Apple's Mail Privacy Protection pre-fetches images, so opens are inflated for a
large and unknowable share of any audience. The industry's headline metric has
been broken for years and the dashboards still lead with it.

**Our move:** we were never going to lead with opens — ADR 0044 turns the
tracking pixel off by default because a sovereignty product that silently
pixels every recipient has sold the thing it complains about. So the metric is
**delivered → clicked → visited → converted → invoiced**, in euros. Because the
invoice is in the same database, that number is a fact rather than an estimate.
*"€4,210 from 1,204 emails"* is what the owner wanted to know anyway.

### 7. Your data is in the United States

For a European business this is a compliance conversation every time — transfer
mechanisms, sub-processors, and a customer list under a foreign jurisdiction.

**Our move:** the whole product's reason to exist. EU-only, and the escape
hatch is real: AGPL and self-hostable, so the answer to *"what if you raise the
price"* is that you can run it yourself with your data already in place. That
is not a promise a hosted competitor can copy.

---

## Weak, not structural — worth doing, not worth claiming as a moat

### 8. Suspension without warning or a route back

A widely reported experience: an account frozen over a policy interpretation,
support slow, and the list — the business's own asset — inaccessible while it
is argued about.

**Honest position:** we will also have to police abuse. A self-service bulk
sender that never suspends anybody becomes a spam host, and then everyone's
mail bounces. What we can promise is narrower and should be written down rather
than implied: **suspension never separates a customer from their data.** Export
keeps working, self-hosting is available, and the reason is stated in writing
with an appeal path. That is a policy commitment, not an architecture, and it
belongs in the terms rather than in a feature list.

### 9. The editor, and template lock-in

Clunky, and the template cannot be changed after a send.

**Our move:** reuse the Docs block model rather than build a second editor, and
compile blocks to email-safe HTML — the same shape as Sites compiling section
JSON to static HTML. Better *integrated*, certainly. Whether it is a better
email builder than a company that has iterated on one for twenty years is not a
claim to make in a pitch.

---

## What they do better, and will for a long time

Written down so nobody plans around a fantasy.

- **Reputation.** Their sending IPs have years of history with every major
  provider. Ours start at zero, and a new subdomain and IP mailing five
  thousand people on day one gets filtered no matter how correct the DKIM is.
  **Warm-up is a product feature, visible in the send flow — not a footnote.**
- **Scale.** Millions of messages an hour, with the queueing and feedback-loop
  tooling that took years to build.
- **Deliverability operations.** Relationships, monitoring, and people whose
  whole job is the inbox-placement graph.
- **The commodity half.** Template gallery, stock imagery, the polish of a
  mature editor. Cheap to admire and expensive to match, and it is the half
  that does not decide whether anyone buys.

---

## What this implies for the build

1. The differentiator is **the segment and the attribution**. Everything else
   is table stakes, including the editor, and should be built to "good enough"
   rather than to "better than theirs".
2. **Suppression must be enforced in the store at send time**, not applied by
   the sender. Absolute and global to the tenant (ADR 0044 §2) is only true if
   an import cannot route around it.
3. **The second IP is a purchase, not a decision**, and it blocks the wave.
   Sharing the transactional IP collapses ADR 0044 §1 on day one.
4. Nothing here is queued. `features.md` and `ROADMAP.md` carry no campaigns
   entry yet, and until they do this is direction rather than scope.
