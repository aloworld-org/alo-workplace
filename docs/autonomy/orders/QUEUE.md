# alo Orders — build queue

The goods half of the business: sales order, reservation, delivery, and
invoicing what actually shipped. `ROADMAP.md` orders this as wave O1.

**Read ADR 0053 first.** It already decided the three things that would
otherwise be re-argued in every item, and none of them is open here:

- the sales order is **its own object** with its own numbering — not a status on
  an invoice, because an invoice is a legal document with gapless numbering and
  the order-to-invoice relationship is many-to-many in both directions;
- reservation is **soft** — a number on the stock record, never stock moved to a
  holding location;
- invoices follow **deliveries** by default, with order-based invoicing kept for
  deposits.

## What is actually true today

- A quote goes straight to an invoice draft. Nothing is reserved, nothing is
  picked, no stock moves. There is **no record of ordered-but-not-delivered**.
- Inventory exists and is real: `inv_*` modules, warehouses, `record_move`,
  reorder rules, `stock_answer`. **Read it before adding to it** — this wave
  consumes inventory, it does not reimplement it.
- Quotes, invoices, payments, credit notes and the VAT rule
  (`billing_vat`, ADR-less but tested) all work. This wave adds a step between
  two things that already function, which is the risk: **the flow that works
  today must keep working**.

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

## Wave O1

- [x] O1.0 ADR 0053 — decided 2026-08-18, no code.
- [ ] O1.1 The sales order and its lines: ordered, reserved, delivered and invoiced quantities per line, linked to the quote it came from and the customer it is for. Those four numbers **are** the order book, so get them right before anything reads them.
- [ ] O1.2 Accepting a quote routes by content — an order where any line is stocked, an invoice draft where none is. **A test pins today's services path unchanged**: a quote of consultancy days must still become an invoice directly, and this item is the one most able to break the flow that already works.
- [ ] O1.3 Reservation: confirming an order commits its stocked lines, and `reserved` is visible beside on-hand and on-order. **`reserved <= on_hand + on_order` is enforced in the database, not by the caller** — a constraint or a checked update, never a read-then-write. The over-commitment test is this wave's wrong-tenant test: two orders racing for the last fan, and exactly one wins.
- [ ] O1.4 Delivery notes: goods leave against an order and stock moves through the existing `record_move` rather than a second movement path. Partial delivery is the normal case, not an edge one. Cancelling a delivery returns the reservation.
- [ ] O1.5 Invoice from a delivery: bills what shipped, increments the order line's `invoiced`, and leaves the remainder visible. Order-based invoicing (a deposit) uses the same counter so the two routes cannot double-bill — a test that deposits then delivers then invoices, and proves the customer is charged once.
- [ ] O1.6 The order book: ordered, reserved, delivered, invoiced and outstanding, per order and in total, with wrong-tenant tests per route. The screen a manufacturer opens first.
- [ ] O1.7 The Orders agent, reads only (ADR 0047): where is this order, what is short, what can ship today — answered in the room, from the record, no button in between.

## Exit gate

- [ ] The fan quote becomes an order, reserves six AF-630s, ships four on one note and two on another, bills each delivery, and the order book shows the correct remainder at every step — recorded in STATE.md as the actual requests and responses
- [ ] Two concurrent confirmations cannot both reserve the last unit, proven by a test rather than by care
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
