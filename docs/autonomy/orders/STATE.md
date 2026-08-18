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

---

## 2026-08-18 — read before the successor ADR, so it is not written from memory either

Three claims checked against the code rather than against the disputed ADR. Two
shrink the wave further; one corrects something I asserted without looking.

**Reservation is not a missing number — it is a missing refusal.**
`inv_reorder.rs` states in its own header that `available = on_hand + on_order −
committed`, and that **`on_order` and `committed` are computed, never stored**,
`committed` being the undelivered remainder of every confirmed sales-order line.
So the quantity ADR 0053 §2 wanted to add already exists, is already correct, and
already has the lifecycle for free. `inv_so.rs` says equally plainly that
**"confirming reserves nothing… therefore no reserved quantity anywhere in this
module to drift out of step"** — which is a deliberate choice, not an oversight.

What is actually absent is the **check at confirmation**: nothing refuses a
confirmation that would push `committed` past `on_hand + on_order`. That is one
guarded transaction, not a schema change, and it is the whole of the
over-commitment protection the wave was built around.

**The quote link may be partly there.** `quote` already appears in `inv_so.rs`
and `inv_so_confirm.rs`. I claimed it absent without reading them; the successor
ADR must establish what those references do before deciding anything about
O1.2.

**The lesson this file should keep.** ADR 0053 was written from a filename
search, and its correction was nearly written from memory of that search. Both
times the code said something different and said it in the first twenty lines of
a header. **Read the module headers first — this repository writes its reasoning
where the code is, and the answer to "does it do X" is usually stated outright
rather than inferred from a grep.**

---

## 2026-08-18 — the successor ADR, and O1 re-cut to four items

**Shipped:** `docs/decisions/0054-what-the-order-book-still-needs.md`, ADR 0053
marked superseded, `QUEUE.md`'s wave O1 re-cut from seven items to four, and the
ROADMAP's order track corrected. No code — this is the re-cut the halt asked a
human for, and it is now unblocked in a checkout of its own.

**Every claim in 0054 was read out of the module it describes.** The entry above
records that the last two attempts both nearly wrote from memory of a search;
this one names the file and line behind each statement so the next reader can
check rather than trust.

**Three questions the previous reads left open, now answered:**

- **The quote link is absent, not "partly built".** Every occurrence of `quote`
  in `inv_so.rs`, `inv_so_lines.rs`, `inv_so_confirm.rs` and `inv_so_invoice.rs`
  is **prose in a comment** — one about a conversation, one listing states, one
  about what a line was quoted at, one about not restating a price. There is no
  column, no table and no code. The previous entry's "may be partly there" was
  the right instinct and the wrong conclusion.
- **The link should be a column, not a link table.** `billing_invoices.quote_id`
  already exists (migration 0106) with a composite foreign key. An order comes
  from at most one quote and the invoice side already answers the same question
  the same way; a second shape for one relationship is a second thing to read.
  This retires the halted iteration's drafted `0700_order_quote_links.sql`
  outright — it was solving a problem the schema had already solved next door.
- **The row lock at confirmation is not enough for the refusal.**
  `confirm_inv_sales_order` takes `SELECT … FOR UPDATE` on the *order* row. Two
  confirmations of two different orders for the same product lock different rows
  and their folds interleave freely, so a refusal built on that lock alone would
  pass every single-threaded test and fail exactly when it mattered — the failure
  this journal's opening rule names.

**The finding neither 0053 nor the two corrections mention: there are already
three answers to "what can we promise", and none consults the others.**

| answer | counts | for |
|---|---|---|
| `inv_reorder::available_qty_milli` | `on_hand + on_order − committed` | *do I need to buy?* |
| `inv_stock_sale::stock_for_sale` | `on_hand` at `stock` locations `−` live shop holds | *can this buyer check out?* |
| `record_move` | the ledger, which refuses to go negative | the physical floor |

The first ignores shop holds; the second ignores confirmed sales orders — and
`inv_stock_sale.rs` says so on purpose: *"a warehouse door that suddenly
consulted a shop table would repeal [Inventory's decision] from the outside."*
0054 §2 keeps that and writes down what follows: a hold and an order can both
count on the same unit, whoever picks second finds it gone, and `record_move`
keeps the ledger honest so it surfaces as scarcity rather than as an oversold
shelf. Joining the two doors is a separate ADR, not a side effect of this wave —
and `inv_stock_sale.rs` is not this track's file to change either way.

**How the refusal is settled, decided rather than left to the item.** A
transaction-scoped advisory lock per `(tenant, product)` — the pattern
`inv_stock_sale.rs` already uses at line 164 for the same race, so the two paths
that can promise a unit settle contention one way rather than two. Every stocked
product on the order is locked **in ascending product-id order**, because an
order has many lines where a hold has one, and two orders sharing two products
locking them in opposite orders is a deadlock nobody would find in testing.

**What the re-cut actually removed.** Three items were struck as built (O1.1,
O1.4, O1.5) with the file that builds each; one (the Orders agent) was moved to
O2 because it reads a screen that does not exist yet, and an item nobody can
start is the `[ ]` this track already learned to write as `[~]` with a reason.
Four remain: the refusal, the link, the routing, the read.

**Next:** O1.a. Its test is the one that has to be written first and has to fail
first — two concurrent confirmations for the last unit, against today's code,
where today both succeed. A race test that has never failed proves nothing, and
this wave's whole argument is that a fan cannot be promised twice.
