# ADR 0053 — The sales order is its own object, reservation is soft, and invoices follow deliveries

**Status:** **DISPUTED — do not build from this file.** Its central premise is
false and two of its three decisions are affected. Accepted 2026-08-18, disputed
the same day by the orders loop's first iteration, which read the code instead of
trusting the ADR.

> **What is wrong, so nobody rediscovers it.**
>
> This ADR opens "alo bills well and ships nothing". **It ships.** Wave B5.06
> (2026-08-10) already built the sales order as its own object with its own
> `SO-YYYY-NNNNN` series (`inv_sales_orders`, migration 0162), ordered/delivered
> /invoiced per line, delivery notes with partial delivery and over-delivery
> refused (`inv_so_deliver.rs`, 834 lines), invoicing that follows deliveries
> (`inv_so_invoice.rs`, 707 lines, migration 0164), eight `/inventory/sales-orders*`
> routes and two web views.
>
> The error was a search, not a judgement: `sales_order` was grepped and the
> convention is `inv_so_*`. An absence was concluded from a filename pattern and
> an ADR, a roadmap wave and a queue were built on it. **Decision 1 therefore
> describes something that already exists**, and O1.1, O1.4 and O1.5 are largely
> or wholly built.
>
> **Decision 2 is also contradicted by the code.** It makes `reserved` a stored
> quantity; `inv_reorder.rs` (B5.07) already *computes* `committed` — the
> undelivered remainder of confirmed order lines — and says in its own words why
> it is not a table. A stored figure beside it is a third number meaning what the
> second means, and the first day they disagree the order book and the shortage
> report can each cite the database. Computed also gets the lifecycle free:
> delivering releases, cancelling releases, with no hook to forget.
>
> **What survives:** decision 3 (invoices follow deliveries) — which the build
> already implements for the reason given here — and the hard rule that
> `reserved <= on_hand + on_order` is refused inside the confirming transaction
> rather than by the caller. That rule holds whether the figure is stored or
> computed.
>
> **The real gaps** are smaller and sharper than this ADR imagines: reservation
> (`inv_so.rs` says confirming reserves nothing), the quote-to-order link and
> routing (a quote can only become an invoice today), and the order book across
> orders plus its agent. A successor ADR should be written from the code, and
> wave O1 re-cut around those three.
**Context:** roadmap Order track O1, `billing_quotes`, `billing_invoices`,
`inv_*` inventory modules, ADR 0035 (the Work OS), ADR 0047 (reads answer)
**Decides** the three questions O1.0 named, before any of O1 is built.

## What is missing

alo bills well and ships nothing. Walking the live flow with a fan
manufacturer's data, a €29,736.96 order went from quote to invoice in one step:
nothing was reserved, nothing was picked, no stock moved.

The consequence is not cosmetic. There is no record of **ordered but not yet
delivered** — which for anyone selling physical goods is most of the business at
any moment. The order book cannot be answered, the same fan can be sold twice,
and an invoice can be raised for goods that never shipped.

## 1. The sales order is a new object, not an invoice in another state

**Decision: a sales order is its own record, with its own numbering sequence.**

The tempting shortcut is a `status = "ordered"` on the invoice, reusing the
lines and the customer link. It fails on three counts:

- **An invoice is a legal document with gapless numbering.** An order is a
  commercial commitment that may be cancelled, amended or never delivered.
  Making an order occupy an invoice number means either burning numbers on
  things that never become invoices, or numbering at a different time than
  issuing — and gapless numbering is the one property that must not acquire
  exceptions.
- **The relationship is many-to-many.** One order becomes several invoices when
  it part-ships; one invoice covers several orders when a customer consolidates
  a month. A state on a single row cannot express that, and every workaround
  ends in a join table that is the order object arriving late and badly.
- **They answer different questions.** An invoice answers *what is owed*. An
  order answers *what did we promise, and where is it*. Those diverge the moment
  a delivery is partial.

An order line therefore carries **ordered, reserved, delivered and invoiced**
quantities. Those four numbers are the order book.

## 2. Reservation is soft

**Decision: confirming an order increments a `reserved` quantity on the stock
record. It does not move stock into a holding location.**

Hard reservation — physically moving goods to a "committed" location — is what
large warehouses do and it is the wrong trade here:

- it doubles the stock movements, so every cancellation is a compensating move
  and the movement history stops reading like what actually happened;
- an amended order becomes a sequence of moves rather than a changed number;
- and it buys accuracy that matters when pickers walk a floor, which is not the
  problem an SME manufacturer has.

Soft reservation keeps one truth per product per location — `on_hand`,
`reserved`, and `available = on_hand − reserved` — and cancellation is a
decrement.

**The rule that makes it real: `reserved` may never exceed `on_hand + on_order`,
and that is enforced in SQL, not by the caller.** Selling the same fan twice is
the failure this whole wave exists to prevent, and a rule the application layer
remembers is not a rule. O1.3 carries an over-commitment test as the sibling of
the wrong-tenant test.

## 3. Invoices follow deliveries, with order-based invoicing available

**Decision: the default is to invoice what has been delivered. Invoicing against
the order — a deposit or full prepayment — is explicit and allowed.**

Billing what left the building is the honest default and the one that makes a
part-delivered order bill correctly: four fans shipped of six ordered produces
an invoice for four, with two still visible as outstanding.

But deposits are real, and the walkthrough proved it — that fan order took a
€10,000 deposit before anything shipped. A system that could only invoice
deliveries would have refused it. So both paths exist, and the order line's
`invoiced` quantity is what keeps them from double-billing: whichever route
raised the invoice, the quantity is spoken for.

## What does not change

**A services quote still becomes an invoice directly.** Products already carry
`stocked`; a quote of consultancy days has nothing to reserve and nothing to
deliver, and routing it through an order would add a step that serves nobody.
Acceptance routes by content: an order where any line is stocked, an invoice
draft where none is. O1.2 pins today's behaviour with a test so the flow that
already works cannot regress while the new one is built.

## Consequences

- Four objects in the chain — quote, order, delivery, invoice — each linked to
  the one before, so a payment traces back to the promise that earned it.
- The order book (O1.6) becomes answerable, and with it the Orders agent (O1.7):
  *where is this order, what is short, what can ship today* are reads, answered
  in the room under ADR 0047.
- Migrations take the **`07xx`** block.
- **This does not make alo a manufacturing system.** Bill of materials, works
  orders and capacity are O2 and deliberately unstarted. An order book with no
  reservation is the more urgent absence; taking an order you cannot build moves
  the problem rather than solving it.

## Rejected

- **An invoice in another state** — burns gapless numbers on commitments that may
  never become invoices, and cannot express the many-to-many.
- **Hard reservation** — accurate for a warehouse with pickers, and a worse
  history for everyone else.
- **Invoice only from the order** — simpler, and it bills for goods that have not
  shipped, which is the complaint customers make about ERP systems rather than a
  feature of them.
- **Skipping the order and adding delivery notes to invoices** — puts the goods
  half inside the money document and leaves the order book still unanswerable.
