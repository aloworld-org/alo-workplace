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

---

## 2026-08-19 — O1.a: the refusal, and the ADR it amended within the hour

**Shipped.** `platform/alo-store/src/inv_so_commit.rs` (the question),
`inv_so_confirm.rs` (which now asks it), `ON_ORDER_SQL`/`COMMITTED_SQL` opened to
the crate so the fold is re-used rather than restated, the `allowBackorder` body
on `POST /inventory/sales-orders/{id}/confirm`, and
`tests/inv_so_commit.rs` — nine tests. **No migration**: `reserved` stays
computed, so `07xx` is still untouched.

**The test was written first and it failed first, which is the only reason it
means anything.** Seven of the eight original tests failed against `main`. The
one that matters failed like this:

```
exactly one of two confirmations for the last fan may be allowed to promise it;
got first=Ok(… number: Some("SO-2026-00001") …) second=Ok(… number: Some("SO-2026-00002") …)
```

Two numbered orders, one fan. That is the failure this wave is named after,
demonstrated rather than described.

**Why the order's own row lock was not enough**, which is what makes this more
than an `if`: `confirm_inv_sales_order` already held `SELECT … FOR UPDATE` on the
order. That serialises two confirmations *of the same order* and nothing else —
two different orders for one product lock different rows and their counts
interleave. A refusal built on it would have passed every single-threaded test
here and failed in production. The count and the decision are made one act by a
transaction-scoped advisory lock per `(tenant, product)`, which is the instrument
`inv_stock_sale.rs` already uses for the same race, so the two paths that can
promise a unit contend one way instead of two. Every stocked product on the order
is locked in **ascending product-id order**: an order has many lines where a shop
hold has one, and two orders sharing two products, each locking in the order the
lines happened to appear, deadlock.

### The ADR was amended by building it, and this is the part worth keeping

ADR 0054 §3, written this morning, said "refuse" without qualification. Three
`inv_reorder` tests then failed, and one of them said this:

```rust
assert_eq!(
    promised.available_qty_milli, -2_000,
    "more promised than exists is legitimately negative"
);
```

**The module that owns the number states that over-commitment is legitimate.** It
is the state its shortage report exists to report. An unconditional refusal makes
that state unreachable and hollows out the report — and the way I would have
found out is by deleting another module's assertion to make my own feature pass,
which is the exact move this repository's history says goes wrong.

So the rule is not *never over-promise*, it is **never over-promise by
accident**. `confirm_inv_sales_order` takes `allow_backorder`, defaulting to
refuse. The shape is not invented: cancelling a part-delivered order already
needs `short_close` because, in `inv_so.rs`'s own words, *"that is a decision
rather than a slip"* — so the caller says it out loud. The HTTP surface mirrors
`shortClose` exactly, down to absent-means-false on a body that may be empty.

Three failing tests in a module this track does not own were the signal. Bending
them would have been a fixture edit that quietly changed a product decision; the
right reading was that two features contradicted, and one of them was mine.

**The guarantee, stated at its true width:** two people cannot each sell the last
fan *without either of them choosing to*. A tenant who backorders on purpose sees
it in the shortage report, where they asked to see it.

**Who could not be over-promised, quoted rather than summarised.**

- `two_confirmations_for_the_last_fan_leave_exactly_one_promise` — two separate
  orders, `tokio::join!`, exactly one `Ok`; the loser's refusal must name the
  product, and the shelf is asserted untouched afterwards because confirming
  promises and never moves goods. Deliberately two orders rather than one
  confirmed twice: the row lock already stops the second, and a test passing on
  that lock would prove nothing about the shelf.
- `an_order_beyond_the_shelf_is_refused_with_what_is_short` — six against four
  refuses and says "short by 2"; then four fits **exactly**, because `available`
  is a quantity that may be promised in full rather than one to stay under; then
  one more is refused, since the shelf is now spoken for.
- `nothing_a_refused_confirmation_asked_for_reaches_the_order` — still a draft,
  no number drawn, no day stamped, still deletable. A refusal that half-wrote
  would leave a hole in a sequence a customer's bookkeeping can see.
- `what_is_already_on_order_from_a_supplier_may_be_promised` — nothing on hand
  and six on order confirms; the seventh does not. A business that could not
  promise what it has already bought could not take an order at all.
- `a_service_promises_nothing_and_never_blocks_an_order` — an empty warehouse
  must never stop a quote of consultancy days.
- `delivering_releases_the_promise_for_the_next_order` and
  `cancelling_releases_the_promise_for_the_next_order` — the lifecycle that comes
  free with the fold, proved rather than asserted: no hook anywhere to forget.
- `a_seller_who_says_so_may_promise_goods_they_will_buy` — refused at ten against
  one, taken when said out loud, and then the shortage is **visible** as
  `available = −9`; and the next order is refused again, because a backorder is a
  decision about one order and never a setting.
- `another_tenants_stock_can_never_back_our_promise` — the mandatory wrong-tenant
  test in the shape this module could break it: a neighbour's fifty fans do not
  make our empty warehouse look stocked, they may still promise all fifty, and
  their promises do not make ours any shorter.

**How verified.** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store
-p alo-jmap --all-targets` — clean for this change (the two pre-existing
`type_complexity` warnings in `meet.rs`, untouched); the four affected inventory
binaries **35/35 green**; the new suite **9/9**; the full `alo-store` suite
**2 361 tests, 2 355 passed, 1 skipped, 314 s**.

**The six that failed are the sites track's load-flaky sweepers, and the
reasoning is recorded rather than asserted** — `site_ticket_orders` ×2,
`site_bookings_public`, `snooze`, `site_publish_schedule_tenancy` ×2. Three
things say so together: they are the *same* tests the campaigns journal has
flagged three times (they claim from a global cross-tenant sweep with a fixed
round budget, so added concurrency starves the watched row); `site_ticket_orders`
passes **14/14 alone** with this change in the tree, exactly as it does on
`main`; and — the decisive one — **none of the four files contains a single
reference to `inv_`, `sales_order` or `confirm`**, so there is no path by which a
change to `inv_so_confirm`, `inv_so_commit`, two `pub(crate)` consts and one
route handler could reach them.

A stash-and-compare was attempted for completeness and abandoned as a poor
instrument: `git stash` leaves untracked files behind, so the first attempt
compiled this item's own test file against a tree with no module behind it, and
matching a full suite's parallel load inside a subset is guesswork either way.
Code-path reachability is the fair test and it is above.

**Cuts:** none. One thing deliberately not built: the shop's own availability
still ignores confirmed sales orders and this refusal still ignores live shop
holds — ADR 0054 §2's named limitation, unchanged here because
`inv_stock_sale.rs` is not this track's file and joining the two doors is its own
decision.

**Next:** O1.b, the `quote_id` column mirroring `billing_invoices.quote_id`
(migration 0106). It takes migration `07xx` — check the directory immediately
before rebasing, not once at the start.

---

## 2026-08-19 — O1.b: the offer an order came from, in one column

**Shipped.** Migration `0700_inv_sales_order_quote_link.sql`, `quote_id` through
`NewSalesOrder`/`SalesOrder`/`SO_COLS`/`normalize_sales_order`, `quoteId` on the
order JSON, and `tests/inv_so_quote_link.rs` — four tests. The `07xx` block was
checked immediately before writing, not once at the start: `0700` was free.

**A column, not a link table**, and the evidence was next door.
`billing_invoices.quote_id` (migration 0106) has answered the identical question
for the other branch of an acceptance since B1.12, with the same composite
foreign key and the same partial unique index. An order comes from at most one
quote, so a second shape for one relationship would be a second thing to read.
**This retires the `0700_order_quote_links.sql` a halted iteration drafted** — it
was solving a problem the schema had already solved, and the draft is now
superseded rather than merely unused.

0106's reasoning transferred whole and is quoted in the migration where the next
reader will be standing: the column lives on the **order** because that is the
newer document and the one that knows its own origin, and a column on the quote
would have to be written into a row that is frozen the moment it is sent — the
very property that makes a sent quote trustworthy. `NO ACTION` rather than
`CASCADE`, because only a draft quote is ever deleted and a draft has never been
accepted, so no linked quote can vanish under a standing order.

**Two decisions that are this item's own.**

- **The tenant check is in Rust as well as in the key.** The composite foreign
  key would refuse a stranger's quote id on its own — but as a database error,
  where every other cross-tenant reference in this module answers with a clean
  `NotFound`. A foreign id must be indistinguishable from one that never existed,
  so the quote is resolved through the account door first and the key is the
  second line rather than the first.
- **`quoteId` is read-only on the wire.** It is absent from `OrderBody`
  entirely, so sending one is an unknown field and ignored, and `editable()`
  carries the stored value through a `PATCH` rather than re-stating it. An order
  that could be told it came from an offer it did not come from would make the
  link worthless in exactly the case it exists for. The unit test
  `no_request_can_state_or_clear_where_an_order_came_from` holds it: a stated id
  is ignored, a `null` does not clear it, the rest of the body still merges, and
  a body naming only `quoteId` states no header at all.

**Who could not claim what, quoted rather than summarised.**

- `another_tenants_quote_can_never_be_the_origin_of_our_order` — the mandatory
  wrong-tenant test. A neighbour's offer is a `NotFound` from our handle;
  **nothing of ours was written for it** (our order list is asserted empty);
  their offer is untouched, still unclaimed, and still becomes *their* order
  afterwards; and an id that never existed answers identically, so ours discloses
  nothing about whether theirs is real.
- `an_offer_can_be_taken_up_as_an_order_only_once` — the second attempt is a
  `Conflict` saying so, from the partial unique index rather than from a check
  that could be raced past; and two orders from no offer at all are still
  perfectly ordinary, because the index is partial.
- `an_order_remembers_the_offer_it_was_taken_from` — written, read back on the
  single read **and in the list**, and surviving an ordinary header edit.

**How verified.** `cargo fmt`; clippy clean for both crates; `inv_so*` +
`inv_reorder` + `billing_quote*` binaries **50/50**; every `inv_*` and
`billing_*` binary **174/174**; the new suite **4/4**; the `alo-jmap` inventory
unit tests **14/14**; and the full `alo-store` suite **2 365 tests, 2 363 passed,
1 skipped, 437 s** — up from 2 361, by this item's four.

The two failures are `site_ticket_orders` again, down from six in O1.a's run,
which is what load variance looks like rather than a defect converging. The
reasoning is not repeated: that file holds no reference to `inv_`, `sales_order`
or `confirm`, and it passes 14/14 alone with this change in the tree. **It has
now failed under load in two consecutive full runs from this track and was
flagged three times from campaigns — it is past being worth a note each time and
is a request the sites queue should take.**

**The disk filled mid-gate and the handover's note was half right.** `rustc`
died with `IO failure on output stream: no space on device` at 100% of 474 GB.
The stale-binary sweep it recommends had nothing to reclaim here — 176 test
binaries against 172 distinct targets, so almost nothing was stale — but **1.9 GB
of `.pdb` files** were sitting in this checkout's `deps` alone despite
`[profile.test] debug = 0`, and removing them plus one `cargo clean -p` freed
6.4 GB. Three checkouts' `target` directories hold 24 GB between them. Recorded
because the existing note sends the next person looking for stale `.exe` files
first, and here the answer was the symbols.

**Next:** O1.c — accepting a quote routes by content. The test that pins today's
services path is written **before** the branch exists: `accept_billing_quote` is
in daily use and raising an invoice from a services quote must stay byte for byte
what it is.

---

## 2026-08-19 — O1.c halted: a quote has no idea what it is selling

**Nothing shipped, and nothing should have been.** O1.c says acceptance routes by
content — *an order when any line names a stocked product, an invoice when none
does*. **A quote line does not name a product, and cannot.** This was read out of
the schema before a line of routing was written, which is the only reason it cost
an hour rather than a wave.

### What a quote line actually is

`billing_quote_lines` (migration 0105) is `description, unit, qty_milli,
unit_price_cents, vat_rate_bp` — and the migration states why there is no more:

> Lines SNAPSHOT the price list, exactly as invoice lines do: **no foreign key
> back to `billing_products`**, so a later price change never rewrites an offer
> already made.

`billing_invoice_lines` is the same. Only the inventory documents —
`inv_sales_order_lines`, `inv_purchase_order_lines` — carry `product_id`, because
they are the ones that move goods. The boundary is deliberate and it is drawn in
the right place: **billing documents are money, inventory documents are goods.**

So the routing signal the item asks for does not exist. There is no field on a
quote, or on any of its lines, that says which catalog item a line is.

### The part that settles it: a manual route would be no better

The obvious fallback is to stop guessing and let the seller say which one they
want — the idiom this repository already uses twice for decisions the system
cannot infer (`short_close`, and `allow_backorder` from O1.a). **That does not
work either, and the reason is worth writing down:**

An order raised from a quote would copy the quote's lines, and every one of them
would carry `product_id: None`. `inv_so_deliver.rs:365` refuses exactly that:

> `line {position} is a charge in words, not goods; nothing leaves against it`

Every line would be a charge in words. Nothing could ever be delivered against
the order, `inv_so_commit` would find no stocked line to check, and the document
would sit at `confirmed` for ever. It would look like the feature and be an
ornament — which is worse than not having it, because somebody would trust it.

**There is therefore no version of "a quote becomes an order" that works today,
automatic or manual.** The blocker is not the routing rule; it is that a quote
cannot say what it is selling.

### What unblocks it, and whose it is

One additive column: `billing_quote_lines.product_id TEXT NULL`, composite
foreign key to `billing_products`, `ON DELETE SET NULL` — **exactly the shape
`inv_sales_order_lines` already uses** (migration 0162, lines 129 and 152). It
does not weaken 0105's reasoning: the line keeps snapshotting description, unit,
price and rate, so a price change still cannot rewrite an offer. The product id
is *provenance*, which is the same distinction O1.b just settled for the
order-to-quote link.

**That is a change to `billing_*`, and this track does not make those.** The
queue's own rule: *"A join that seems to need a change in someone else's module
is a request to put in their queue, not a change to make here."* It is filed
below as a request for the business track, with the shape it should take, so
whoever picks it up does not have to rediscover any of this.

It is a bigger change than one column, and the request says so honestly: the
quote editor has to let somebody pick a catalog item on a line, and the copy into
an invoice draft should carry the product across too, or the same gap simply
moves one document downstream.

### O1.c is `[~]`, not `[ ]`

Per this repository's own rule — *`[ ]` is an instruction to keep trying* — the
item is marked blocked with its reason rather than left open for the next
iteration to walk into. The wave is not stuck: **O1.d, the order book, is
unblocked and is the next item.**

**The lesson, which is now three for three in this wave.** ADR 0053 was scoped on
a grep that missed `inv_so_*`; O1.a's refusal was scoped on "refuse" until
`inv_reorder`'s own test said over-commitment is legitimate; and O1.c was scoped
on a quote line naming a product it has never named. Each time the answer was
written down in the schema or the module header, and each time reading it first
was what stopped a wave being built on it. **Read the migration before designing
against the table.**

---

## 2026-08-19 — O1.d: the order book, folded at the moment of asking

**Shipped.** `platform/alo-store/src/inv_so_book.rs`,
`products/mail/alo-jmap/src/inventory_order_book.rs`,
`GET /inventory/order-book?scope=open|all`, and eleven tests — seven pure ones
on the arithmetic, four over the real router. **No migration**: nothing here is
stored.

**Its own path rather than `/inventory/sales-orders/book`**, which would have sat
under that route's `{id}` and quietly made `book` a reserved order id.

### Three decisions, each of which is a bug avoided

- **Delivered and invoiced value are not a share of the order's total.** Each is
  `billing_totals::line_net_cents` — the same function the order, the invoice and
  the quote all use — applied to the same line at the quantity that actually
  moved. Splitting a rounded total by a ratio produces cents belonging to
  nothing, and the parts stop adding up to the whole. The unit test asserts
  `delivered + outstanding == ordered` exactly, which a proportional split would
  fail.
- **A charge in words is money and never goods.** Assembly has a value and never
  leaves on a pallet — `inv_so_deliver` refuses to move it — so counting it in an
  outstanding *quantity* would hold an order open for ever. It counts in the
  cents, where it is real, and not in the milli-units, where it is not.
- **`reserved` is the undelivered remainder while the order is open**, and
  nothing else. There is no column, per ADR 0054 §3; this is the same fold
  `inv_reorder`'s `committed` performs across all orders, seen one order at a
  time. A draft reserves nothing because nobody was promised anything; a
  delivered or cancelled order reserves nothing because there is nothing left to
  send. `outstanding` stays a fact about the order in every state — only what it
  *holds against the warehouse* depends on being open, and the test asserts both
  halves of that distinction.

### Two things the tests caught that review would not have

- **A discount would have vanished from the book.** `outstanding_qty` clamped at
  zero to stop an over-delivered line printing a negative that reads as a credit
  — and that clamp also silently deleted a **negative quantity**, which is how
  `billing_line` expresses a discount. The book would have overstated what every
  discounted customer owed. The clamp is now sign-aware: it can never overshoot
  *past zero*, in whichever direction the line runs. Written as a test before the
  code, and it failed on the first run for exactly this.
- **`invoiced_qty_milli` is not a column.** It is a correlated sum over
  `inv_so_invoice_lines` that counts only documents still standing, and selecting
  it produced a `500` from the new route. The fix was not to write the subquery
  again: it is now `inv_so_lines::INVOICED_QTY_SQL`, spliced into both readers.
  **A second reading of what has been billed would have let the order book and
  the order document disagree about one line** — which is the two-truths failure
  this wave has now avoided three times (the `committed` fold in O1.a, on-hand in
  `inv_so_commit`, and this).

### Who could not see what

- `one_tenants_book_never_contains_another_tenants_orders` — the mandatory
  wrong-tenant test on the surface where it would be worst, since this is the
  screen somebody reads to decide what their business is owed. Asserted from both
  sides so a leak would have to appear as a named row rather than a number nobody
  checked: our book holds exactly our order and our total is exactly ours, theirs
  likewise, and a caller with no token gets `401`.
- `the_scope_is_strict_and_a_draft_is_not_open_business` — a draft is absent from
  the morning's book and present under `scope=all`, a scope this build cannot
  name is a `422` rather than a silent widening, and the case-insensitivity
  matches the sales-order list's own filter rather than inventing a second rule.
- `a_book_with_nothing_in_it_is_a_shape_and_not_a_null` — zeros and an empty
  list, never a null a screen has to special-case.

**How verified.** `cargo fmt`; clippy clean for both crates; the store's unit
tests **7/7**; the HTTP suite **4/4**; every `inv_*` and `billing_*` binary
**174/174**.

**Not built, and named rather than left implicit:** there is **no screen**. The
route answers, and `web/src/inventory/` has no order-book view — the web surface
has an owner and this track does not take web work without asking. The item's
words are "the screen a manufacturer opens first", so it is honest to say the
half that exists is the half a client can call.

**Next:** wave O1 is now O1.a, O1.b and O1.d done, with O1.c blocked on the
billing schema request. The exit gate's walkthrough — a quote becoming an order,
shipping in two consignments, billing each — cannot be walked end to end until
that request lands, because its first step is the blocked one.
