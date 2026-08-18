# alo Orders — build journal

One entry per completed item: what was built, what the tenancy tests proved,
and — for anything touching stock — **the test that proves the same unit cannot
be promised twice**, quoted rather than summarised.

Started 2026-08-18 against a build where a quote becomes an invoice in one step:
nothing reserved, nothing picked, no stock moved, and no record anywhere of
ordered-but-not-delivered.

**The rule this journal exists to hold.** Every other queue here records what was
built. This one also records **what could not be over-promised**, because a
billing bug produces a wrong number and a stock bug produces a fan that was sold
to two customers. Only one of those is discovered by reading a screen.

Three things are worth more than a green suite:

- **`reserved <= on_hand + on_order` lives in the database.** A read-then-write in
  the application layer passes every single-threaded test and fails the first
  time two people confirm an order in the same second.
- **The services path must not regress.** Quote to invoice already works and is
  in daily use. This wave adds a step beside it, never through it, and O1.2's
  test is what keeps that true while everything after it is built.
- **Inventory is consumed, not reimplemented.** `record_move`, warehouses and
  the stock read already exist and are correct. A second movement path would be
  two truths about the same shelf.

**Not in this queue:** bill of materials, works orders and capacity. An item that
finds itself needing one has found the edge of the wave rather than a problem to
solve — say so and stop.

---

## 2026-08-18 — O1.1 attempted, halted before any of it could be gated

Nothing shipped. Two things were found, and the second one stops the loop; the
first is worth more than the item was.

### 1. The sales order already exists, and it is not a stub

ADR 0053 opens "alo bills well and ships nothing", and the queue's *What is
actually true today* says there is **no record of ordered-but-not-delivered**.
Both are wrong, and the whole wave is planned on them. What is actually in the
build, from wave B5.06 (2026-08-10):

| ADR 0053 asks for | Already there |
|---|---|
| a sales order that is its own object, own numbering | `inv_sales_orders` (migration 0162), `SO-YYYY-NNNNN` drawn from its own series at confirmation (`inv_so_confirm.rs`), never the invoice series |
| ordered / delivered / invoiced per line | `inv_sales_order_lines.qty_milli`, `.delivered_qty_milli`, and `invoiced` as a fold over the invoices raised (`inv_so_lines.rs`) |
| the customer link | `inv_sales_orders.customer_id`, composite-FK'd to the tenant |
| delivery notes, partial delivery, stock moving through `record_move` | `inv_so_deliver.rs` (834 lines): movements + accumulators + status in one transaction, over-delivery refused, `SO-2026-00001/D1` note numbering |
| invoices that follow deliveries | `inv_so_invoice.rs` (707 lines), migration 0164, "at delivery, not at order" for the ADR's own reason |
| reachable over the wire | eight `/inventory/sales-orders*` routes in `server.rs`, and `web/src/inventory/SalesOrdersView.tsx` + `SalesOrderEditor.tsx` |

So O1.1 is ~90% built, and **O1.4 and O1.5 look built outright**. What is
genuinely missing is smaller and sharper than the queue thinks:

- **reservation** — `inv_so.rs` says in its own words that confirming reserves
  nothing;
- **the quote → order link and the routing** — a quote can only become an
  invoice today, and nothing records that an order came from an offer (O1.2);
- **the order book across orders, and the agent** (O1.6, O1.7).

**A second sales-order object must not be built.** Doing what O1.1 literally
says would create exactly the duplicate ADR 0053 rejects in its own *Rejected*
section, and the two truths about one shelf this journal opens by forbidding.
The wave is still real; the queue needs re-cutting around the gaps above, and
that is a human's call, not a loop's.

### 2. `reserved` should be computed, and the reason is a fact ADR 0053 does not cite

ADR 0053 §2 makes `reserved` a stored quantity incremented on the stock record
at confirmation. **That fold already exists.** `inv_reorder.rs`'s shortage query
(wave B5.07) has computed `committed` — "the undelivered remainder of every
confirmed sales-order line", `status IN ('confirmed','partially_delivered')`,
`GREATEST(qty,0) - delivered` — since B5.07, and states there why it is not a
table: *"a second thing that must be kept in step with the orders"*. A stored
`reserved` beside it is a third number meaning what the second means, and the
first day they disagree the order book and the shortage report can each cite the
database.

Computed also gets the lifecycle free: confirming reserves the remainder,
delivering releases it because what left is no longer outstanding, cancelling
releases the rest because a cancelled order is not one of the two reserving
states — no hook anywhere to forget. ADR 0053's hard rule is untouched by the
choice: `reserved <= on_hand + on_order` is refused inside the confirming
transaction either way, which stays O1.3's work.

**Flagged for the human:** this contradicts ADR 0053 §2 as written, on evidence
the ADR does not mention. Either the ADR gets an amendment recording the fold,
or O1.3 is built the ADR's way and `inv_reorder`'s `committed` is retired in the
same item. It must not be left implicit.

### 3. LOOP HALT: a second agent is editing this working tree

CLAUDE.md: *"One agent per working tree. Concurrent editors on one checkout are
forbidden."* This checkout is C:\dev\Ficina-loop, and the **campaigns** loop is
writing it right now — `git status` during this iteration showed uncommitted
`platform/alo-store/src/campaign_send.rs` (53 KB), `migrations/0506_campaign_sends.sql`,
and modified `campaign_record.rs`, `campaign_suppression.rs`, `id.rs`, `lib.rs`,
with mtimes moving under observation (09:49:28 → 09:50:09 while this iteration
was reading them).

It is not only a rule, it is unworkable: `cargo clippy -p alo-store` and `cargo
nextest run -p alo-store` compile the whole crate, so this track's gate would
have been run against another track's half-written module and its verdict would
have meant nothing in either direction — and an untracked `07xx` migration
sitting in the tree would have been applied by *their* test run too.
`docs/autonomy/LOOP.md`'s deliberately-shared files (`lib.rs` `mod` lines) are
designed for two loops on **two checkouts** meeting through `main`, not for two
editors in one.

**The tree was left exactly as it was found.** The drafted O1.1 work was removed
from it rather than committed — an untested migration and two modules that could
not be gated do not belong on `main`, and `lib.rs` could not be committed at all
without dragging the other agent's uncommitted `campaign_send` line onto a `main`
whose `campaign_send.rs` does not exist. The draft is kept outside the checkout
at `%LOCALAPPDATA%\Temp\claude\C--dev-Ficina-loop\7ce12b8a-efa1-4528-95da-0ce06e233e63\scratchpad\orders-o1.1-draft`
(`migrations/0700_order_quote_links.sql`, `src/order_quote_link.rs`,
`src/order_book.rs` — the link table with both references named so the store can
map them to a clean not-found, and the four numbers per line with `reserved`
computed as above and unit-tested over every status). It is a draft: it has never
been compiled, and a scratchpad does not survive forever. Treat it as notes,
not as work in hand.

**A human unblocks this**: give the orders track its own checkout (outside any
file-sync folder), then re-cut wave O1 around the three real gaps in §1 and
settle §2.

LOOP HALT: a second agent (the campaigns loop) is editing this working tree; no
gate run here can be trusted and nothing may be committed from it.
