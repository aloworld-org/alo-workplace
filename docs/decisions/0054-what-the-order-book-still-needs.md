# ADR 0054 — What the order book still needs: a refusal, a link, and a routing rule

**Status:** accepted. **Supersedes ADR 0053**, which stays in the tree as a
record of how it went wrong.
**Date:** 2026-08-18
**Context:** ADR 0035 (business modules), ADR 0041 (commerce over one catalog),
`docs/design/inventory.md`, and — the point of this one — the code in
`platform/alo-store/src/inv_so*.rs`, `inv_reorder.rs`, `inv_stock_sale.rs` and
`inv_moves.rs`.

## Why this file exists

ADR 0053 opened *"alo bills well and ships nothing"* and scoped a seven-item wave
on it. The sentence is false. Sales orders, delivery notes with partial
delivery, invoicing-what-shipped, and eight HTTP routes with two React views were
all built in wave B5.06 on 2026-08-10. The ADR was written from a filename
search for `sales_order`, which missed the `inv_so_*` family, and the wave was
then planned around the absence of things that were in daily reach of a screen.

**So this ADR is written the other way round: every claim below was read out of
the module it describes, and the modules say most of it themselves in their own
first twenty lines.** That is not a style note. This repository states its
reasoning where the code is, and both times somebody guessed instead of reading,
the guess became a wave.

## 1. What is already there

Read from the code on 2026-08-18, not recalled:

| | where |
|---|---|
| A sales order as its own object, own numbering `SO-YYYY-NNNNN` from its own series, drawn at confirmation | `inv_sales_orders` (migration 0162), `inv_so_confirm.rs` |
| The lifecycle `draft → confirmed → partially_delivered → delivered`, `cancelled` from the three non-terminal states, enforced by one pure transition table tested over all twenty-five pairs | `inv_so.rs` |
| Ordered / delivered per line, and invoiced as a fold over the invoices raised | `inv_sales_order_lines`, `inv_so_lines.rs` |
| Delivery notes, partial delivery as the normal case, stock moving through `record_move`, over-delivery refused, `SO-2026-00001/D1` note numbering | `inv_so_deliver.rs` |
| Invoices that follow deliveries, at delivery rather than at order | `inv_so_invoice.rs`, migration 0164 |
| Reachable over the wire, with a screen | eight `/inventory/sales-orders*` routes, `web/src/inventory/SalesOrdersView.tsx` |

**ADR 0053's O1.1, O1.4 and O1.5 are therefore already built.** What remains is
three things, and they are the whole of this ADR.

## 2. The finding that matters most: three answers to "what can we promise"

This is not in ADR 0053 and was not in the two corrections to it. The build
already contains **three different availability answers, computed by three
modules, and none of them consults the others**:

| answer | what it counts | what it is for |
|---|---|---|
| `inv_reorder::available_qty_milli` | `on_hand` (all locations) `+ on_order − committed`, where `committed` is the undelivered remainder of every confirmed sales-order line | *Do I need to buy more?* |
| `inv_stock_sale::stock_for_sale` | `on_hand` at `stock` locations `−` live shop holds | *Can this web buyer check out right now?* |
| `record_move` | the ledger itself, which **refuses to go negative** at `stock` and `transit` locations | the physical floor |

The first ignores shop holds. The second ignores confirmed sales orders.
`inv_stock_sale.rs` says so deliberately and explains why: *"a warehouse door
that suddenly consulted a shop table would repeal [Inventory's decision] from the
outside."*

**Decision: that stays true, and it is now written down rather than inherited.**
The two doors are independent promises over one shelf; the ledger is the only
hard floor. The consequence is honest and must be stated: a shop hold and a
confirmed order can both count on the same unit, and whichever picks second finds
the goods gone. `record_move` keeps the ledger truthful, so this surfaces as
honest scarcity at pick time, never as an oversold ledger.

*Rejected: making the shop subtract `committed`, or making confirmation subtract
holds.* Each is a change to another module's documented seam — Sites' door in one
direction, Inventory's decision in the other — and doing it as a side effect of
the orders wave is how a decision gets repealed by a caller. If cross-channel
over-promising becomes a real complaint, it is one ADR joining the two doors, not
two modules quietly reaching into each other.

## 3. Reservation is a missing **refusal**, not a missing number

**Supersedes ADR 0053 §2**, which made `reserved` a stored quantity incremented
on the stock record at confirmation.

That quantity already exists as a fold. `inv_reorder.rs`'s `COMMITTED_SQL` has
computed the undelivered remainder of every confirmed sales-order line since wave
B5.07, and states in its header why it is not a table: a reservation table is *"a
second thing that must be kept in step with the orders"*. `inv_so.rs` and
`inv_so_confirm.rs` both say, in their own words, that confirming reserves
nothing and that there is therefore *"no reserved quantity anywhere in this
module to drift out of step"*.

**Decision: `reserved` stays computed.** A stored column beside the fold is a
third number meaning what the second means, and on the first day they disagree
the order book and the shortage report can each cite the database. Computed also
gets the lifecycle free — confirming reserves the remainder, delivering releases
it because what left is no longer outstanding, cancelling releases it because a
cancelled order is not one of the two reserving states — with no hook anywhere to
forget.

**What is actually absent is the check at confirmation.** Nothing refuses a
confirmation that pushes `committed` past `on_hand + on_order`. ADR 0053's hard
rule survives intact; only its mechanism changes.

### Where the refusal goes, and how the race is settled

`confirm_inv_sales_order` already runs in one transaction and already takes
`SELECT … FOR UPDATE` on the order row. That row lock is **not** enough: two
confirmations of two *different* orders for the same product lock different rows
and their folds interleave freely.

**Decision: take a transaction-scoped advisory lock per `(tenant, product)`
before the fold, inside the confirming transaction** — the pattern
`inv_stock_sale.rs` already uses for exactly this problem
(`pg_advisory_xact_lock(hashtext($1), hashtext($2))`), so the two paths that can
promise a unit settle their races the same way rather than two ways.

Three details that follow from it, each of which is a bug if missed:

- **Lock every stocked product on the order, in ascending product-id order.** An
  order has many lines where a hold has one. Two orders sharing two products and
  locking them in different orders deadlock; a deterministic order makes that
  unrepresentable.
- **The fold is read under the lock and the status write happens under it**, so
  the count and the decision are one act.
- **The comparison is per product, not per location.** `committed` is per product
  because an order line names no shelf, and `on_hand` is summed across the
  tenant's stock locations to match. This is the reorder view's own reading, and
  using a second one here would recreate the problem §2 just named.

The refusal is `StoreError::Conflict` naming the product and the shortfall,
because a confirmation that fails must tell a salesperson what to say to the
customer.

## 4. The quote → order link does not exist, and is one column

I checked, because the previous correction said it "may be partly built": every
occurrence of `quote` in `inv_so.rs`, `inv_so_lines.rs`, `inv_so_confirm.rs` and
`inv_so_invoice.rs` is **prose in a comment**. There is no column, no table and
no code.

**Decision: an additive `quote_id` column on `inv_sales_orders` with a composite
foreign key, mirroring `billing_invoices.quote_id` exactly** (migration 0106,
which added the invoice's link the same way). Not a link table.

*Rejected: a two-sided link table*, which an interrupted iteration had drafted as
`0700_order_quote_links.sql`. An order comes from at most one quote, the invoice
side already answers the same question with a column, and a second shape for one
relationship is a second thing to read. The many-to-many ADR 0053 worried about
is order ↔ *invoice*, which `inv_so_invoice.rs` already handles.

## 5. Accepting a quote routes by content

Today `accept_billing_quote` closes the offer and raises a draft invoice in one
transaction, and that is in daily use.

**Decision: acceptance routes on the lines — an order when any line names a
stocked product, a draft invoice when none does — and the services path is
unchanged, byte for byte.** A quote of consultancy days must still become an
invoice directly. This is the item most able to break something that already
works, so the test that pins the old path is written before the new branch
exists, not after.

## 6. What this wave is not

Bill of materials, works orders and capacity stay out (wave O2). An order book
with no refusal is the more urgent absence, and taking an order you cannot build
moves the problem rather than solving it.

## Consequences

- **Wave O1 re-cuts from seven items to four**, because three are built and one
  (the order book read) is smaller than it looked now that the four numbers per
  line already exist.
- ADR 0053 is superseded rather than deleted: it is the record of a wave scoped
  on a grep, and `docs/autonomy/orders/STATE.md` keeps the lesson — read the
  module header before concluding a feature is absent.
- The cross-channel over-promise in §2 is now a **known, named limitation** with
  the place it would be fixed, instead of a surprise waiting for the first tenant
  who sells the same fan in a shop and on an order.
