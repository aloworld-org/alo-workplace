# ADR 0041 — alo Commerce: the shop is a surface, not a system

**Status:** accepted
**Date:** 2026-08-11
**Context:** `docs/design/site-chatbot-and-commerce.md`, ADR 0035 (business
modules), ADR 0036 (typed sections), ADR 0040 (the site chatbot)

## The decision in one line

The shop renders the **same** catalog Billing sells from, reserves the **same**
stock Inventory counts, and posts to the **same** books Finance closes — so
there is nothing to reconcile, ever.

## Why this is the whole argument

Shopify's weakness is not its storefront, which is excellent. It is that every
merchant of any size pays somebody — a bookkeeper, an integration, a monthly
SaaS bill — to reconcile Shopify against their accounting, their stock and
their CRM. That reconciliation is the tax on a good storefront and it never
ends.

alo is already in Odoo's position rather than Shopify's: the catalog landed in
Billing, Inventory tracks locations and moves, Finance posts, CRM holds the
customer. **The only way to lose that advantage is to let the shop keep its own
copy of anything.**

So the constraint comes first, before any feature:

> **Stock is one number. Price is one number. The customer is one record.** The
> shop reads and reserves; it never stores a second copy of any of them.

The moment two systems disagree about how many are left, the entire reason for
building rather than integrating has evaporated, and we are Shopify with worse
themes.

## Where "better than Odoo" actually is

Not the storefront, and not the feature list. It is the setup screen.

**Their weakness:** every setting is a form somebody must fill in — product
templates versus variants, fiscal positions, delivery carriers, pricelists —
when the software already knows most of the answers. A bookshop in Belgium
selling paperbacks has one plausible VAT treatment, not a blank field. An
entire consulting industry exists to type these in, which is why a shop is a
project with a partner rather than a decision somebody makes on a Tuesday.

**Our move:** propose the whole configuration from a sentence about the
business, and let somebody approve it.

> "I run workshops in Antwerp and sell two books."
>
> A draft catalog with the workshops as dated products and the books as stock
> items. VAT proposed for each — the Belgian standard rate on the workshops,
> the reduced rate on books, both flagged as needing confirmation. A shop page
> built from sections that already exist. Shipping for the books, none for the
> workshops. **Reviewed and approved, not configured.**

Not fewer settings — the *same* settings, already answered, shown for
confirmation. Every wrong guess costs one correction instead of one lesson in
accounting. This is the propose-then-approve pattern used everywhere else in
alo, and it is worth more here than anywhere, because this is the screen where
Odoo loses the customers who cannot afford a consultant.

It is a harder engineering problem than a settings form, which is the point.
Nobody has done it because it requires the software to understand the business,
and alo is the only one holding the invoices, the stock and the customers
needed to try.

## Build order, narrowest first

1. **Tickets and dated products.** One product type, no shipping, no variants,
   no stock — the calendar is the inventory. Sells through the chatbot on day
   one (ADR 0040), which makes the demo real.
2. **Simple stock items.** One price, one tax, a shipping rate. Enough for the
   books, the merchandise, the spare parts.
3. **Variants.** Size, colour, and the combinatorial explosion behind them.
   This is where every existing commerce system becomes complicated, which
   makes it the most interesting problem here rather than the one to avoid:
   the goal is variants somebody sets up in a minute without ever learning
   what a product template is.
4. **The rest** — subscriptions, bundles, B2B pricelists, multi-warehouse —
   sequenced by what customers ask for, never by what a competitor's feature
   list contains.

## What must be right on day one

- **Capacity is a hold, not a check.** Check-then-create is a race two buyers
  will win simultaneously, and overselling is the classic ticketing failure. A
  hold with an expiry is taken *before* payment and released if the buyer does
  not finish. This is not hardening; it is the first commit.
- **Payments are never ours.** Hosted page, card details never touching alo.
  **Mollie or Adyen ahead of Stripe** — both Dutch, which matters more than
  usual in a product sold on sovereignty.
- **Tax is a rule, not a field.** Event tickets follow place-of-supply rules
  that differ from goods; cross-border selling means OSS and IOSS thresholds. A
  shop that quietly gets this wrong hands its owner a compliance problem years
  later. Worth a tax professional's afternoon before the code, not after.
- **Never a price or a stock figure the model invented.** The catalog answers,
  or nobody answers (ADR 0040).

## Consequences

- The shop cannot ship before the catalog seam it reads exists; that is a
  dependency to sequence, not a reason to keep a second copy.
- Wave 1 is genuinely small — a dated product, a hold, a hosted payment, a
  ticket by email and in the buyer's calendar — and it is a complete story.
- The claim we can make, and neither Odoo nor Shopify can: **the customer who
  bought, the invoice they paid, the email thread about their delivery, the
  meeting where you agreed the discount and the project you delivered are the
  same product.** It is only true while the shop stays a surface.
