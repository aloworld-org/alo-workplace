# alo Orders — build queue

The goods half of the business: sales order, reservation, delivery, and
invoicing what actually shipped. `ROADMAP.md` orders this as wave O1.

**Read ADR 0054 first, and do not build from ADR 0053** — it is superseded, and
its central premise is false. 0054 was written from the code and settles what
0053 got wrong:

- the sales order **already exists** and is its own object — extend `inv_so_*`,
  never build a second one;
- reservation stays **computed**, not stored: `inv_reorder`'s `committed` fold is
  already the number, and what is missing is the **refusal** at confirmation;
- the quote → order link is one additive column mirroring
  `billing_invoices.quote_id`, not a link table;
- three availability answers already exist and deliberately do not consult each
  other; §2 of the ADR names the limitation that follows.

## What is actually true today

**Most of the original wave is built.** Wave B5.06 (2026-08-10) shipped the sales
order, its lines with ordered/delivered/invoiced, delivery notes with partial
delivery through `record_move`, invoicing what shipped, eight
`/inventory/sales-orders*` routes and two React views. Read ADR 0054 §1 for the
table of what is where.

What is genuinely missing is four things, and they are the four items below.

- **Nothing refuses an over-commitment.** `confirm_inv_sales_order` takes the
  order's row lock and draws the number; it never asks whether the goods can
  exist. Two orders for the last fan both confirm today.
- **Nothing records that an order came from a quote.** Every `quote` in the
  `inv_so_*` family is prose in a comment — checked, not assumed.
- **Accepting a quote can only raise an invoice.** There is no routing on
  content, and the services path that does work must not break.
- Inventory is real and correct: `record_move` refuses negative stock, and it is
  the only hard floor under all of this. **This wave consumes inventory; it does
  not reimplement it.**

## Areas this track owns

`platform/alo-store/src/order*.rs`, `products/mail/alo-jmap/src/order*.rs` and
its routes, `web/src/orders/**`, migrations **`07xx`**.

**Reads `billing_*` and `inv_*`; does not restructure them.** A join that seems
to need a change in someone else's module is a request to make in their queue,
not a change to make here — the same rule that kept the website agent out of the
sites track's files.

Campaigns hold `05xx`, mail and platform `06xx`. Check the migrations directory
again immediately before rebasing, not once at the start of an item.

---

## Wave O1 — re-cut 2026-08-18 from seven items to four

Three of the original seven (O1.1, O1.4, O1.5) were already built and are struck
below with where they live. What is left is the refusal, the link, the routing
and the read.

- [x] O1.0 ADR 0053 — superseded by **ADR 0054**, written from the code.
- [~] ~~O1.1 The sales order and its lines~~ — **built** (migration 0162, `inv_so.rs`, `inv_so_lines.rs`). Ordered, delivered and invoiced per line all exist; `reserved` is the computed fold in `inv_reorder.rs`, and ADR 0054 §3 keeps it computed. The quote link is the only part missing and it is O1.b.
- [~] ~~O1.4 Delivery notes~~ — **built** (`inv_so_deliver.rs`, 834 lines): movements through `record_move`, partial delivery as the normal case, over-delivery refused, `SO-2026-00001/D1` numbering.
- [~] ~~O1.5 Invoice from a delivery~~ — **built** (`inv_so_invoice.rs`, migration 0164), at delivery rather than at order, for the ADR's own reason.

- [x] O1.a **The refusal at confirmation.** Confirming an order whose stocked lines would push `committed` past `on_hand + on_order` is refused, inside the transaction that draws the number, with a `Conflict` naming the product and the shortfall — a salesperson has to know what to tell the customer. Settled with a transaction-scoped advisory lock per `(tenant, product)` the way `inv_stock_sale.rs` already settles the same race, **locking every stocked product on the order in ascending product-id order** so two orders sharing two products cannot deadlock. `reserved` stays computed (ADR 0054 §3); no new column and no new table. **This wave's mandatory test is two concurrent confirmations for the last unit where exactly one wins** — written to fail first against today's code, because a race test that never failed proves nothing. **Done 2026-08-19**: the eight tests were written first and seven failed against `main`, the race one because both orders confirmed (`SO-2026-00001` and `SO-2026-00002` for one fan). Building it amended the ADR — see the STATE entry: an unconditional refusal contradicted `inv_reorder`'s own stated position, so the rule became "never over-promise **by accident**", with `allow_backorder` in the same shape as `short_close`.
- [x] O1.b **The quote → order link.** An additive `quote_id` on `inv_sales_orders` with a composite foreign key, mirroring `billing_invoices.quote_id` (migration 0106) exactly. Not a link table. Migration `07xx`. **Done 2026-08-19** as migration `0700`, with the partial unique index that makes one offer yield at most one order, and `quoteId` on the API read-only — provenance is written by the acceptance that produced it and no request may restate it.
- [ ] O1.c **Accepting a quote routes by content** — an order when any line names a stocked product, a draft invoice when none does. **The test that pins today's services path unchanged is written before the branch exists**: a quote of consultancy days must still become an invoice directly. This is the item most able to break something in daily use.
- [ ] O1.d **The order book across orders**: ordered, reserved, delivered, invoiced and outstanding, per order and in total, wrong-tenant tested per route. Smaller than it was — the four numbers per line already exist, so this is a read and a screen rather than a model.

**Cut from this wave, deliberately:** the Orders agent (was O1.7). ADR 0047's
read-only agent over the order book is worth building, but it reads what O1.d
produces and cannot be specified before that screen's shape exists. It moves to
O2 rather than being carried as an item nobody can start.

## Exit gate

- [ ] The fan quote becomes an order, ships four on one note and two on another, bills each delivery, and the order book shows the correct remainder at every step — recorded in STATE.md as the actual requests and responses
- [ ] Two concurrent confirmations cannot both promise the last unit, proven by a test that failed before the refusal existed
- [ ] A services quote still becomes an invoice directly, unchanged

**Not in this queue: bill of materials, works orders, capacity** (wave O2). An
order book with no reservation is the more urgent absence, and taking an order
you cannot build moves the problem rather than solving it. If an item here seems
to need a BoM, it has found the edge of this queue rather than a problem to
solve.

---

## Done means

The implement skill's definition, plus one thing specific to this wave: **an item
that changes what can be promised is not done without a test proving what cannot
be promised twice.** Stock is the one number in this product that two people can
try to spend at once.
