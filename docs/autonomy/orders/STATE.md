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
