# alo Billing — the rest of the app its agent cannot reach

> **PARKED 2026-08-30, before this track ran an iteration.** The owner closed
> the agent programme: sixteen product agents and Ask alo are built, verified
> against a real model, and the three defects that run found are fixed. The
> coverage measured there — 136 verbs against 347 excluded routes, 47 of them
> excused as "a later intent set" — is recorded rather than closed, and this
> queue is the record of what closing it would mean. **No loop takes an item
> from here.** If the work is picked up later, nothing needs rewriting: the
> items below still name the exact routes and the rule that decides each one.

The A9.1 evaluation (`docs/autonomy/agents/STATE.md`, 2026-08-30) measured what
the sixteen agents actually cover: **136 verbs against 347 excluded routes**, and
**47 of those exclusions say in their own words "a later intent set"** — they
are not decisions that an agent should never make, they are work nobody has
done yet. **Twenty of the forty-seven are Billing's.** This track is those
twenty.

**Read first, in this order:** ADR 0057, ADR 0058, `docs/design/complete-agents.md`,
then `alo_ai::intent`, `alo_ai::billing_intents` and `alo-jmap`'s
`billing_intents.rs` — Billing is the reference module, and this track extends
the module that every other one was copied from, so its shape is the contract.

## The rule that decides each route

For every route listed below, exactly one of two things happens, and the
journal says which and why:

1. **It becomes a verb** — an `IntentSpec` with its purpose, effect, args,
   `answers` questions and (for a write) a preview and an undo, an executor
   returning the module's own record views, and a wire test. A write is
   proposed and never run without approval (ADR 0023); a read answers in the
   room.
2. **Its exclusion is rewritten to say why it is permanent** — not "a later
   intent set", which is a promise, but the actual reason an agent must never
   do this. "Ticking somebody's checklist is finishing their work" is a
   permanent reason. "A person keeps the price list" is not: a person also
   keeps the customer list, and `customer_lookup` exists.

A route left with "a later intent set" at the end of this track is the item
not being done.

## Areas this track owns

`platform/alo-ai/src/billing_intents.rs`, `products/mail/alo-jmap/src/billing_intents.rs`,
and new `alo-jmap` executor modules if a subject deserves its own file (one
file, one reason to change). The Billing store surface may gain **new**
functions, never changed ones. `web/**` is **not** this track's — Codex owns
`web/src/billing/**`. Migrations `0930`–`0949` if a verb genuinely needs one
(most will not). Nothing in `agent.rs`/`agent_product.rs` beyond Billing's own
rows, which already exist.

## Rules

- **Never widen a route's authority.** A verb calls the same core the route's
  handler calls (A4.1b), so what the agent may do is exactly what the screen
  may do, at the asker's own scope. A verb that needs a new store function
  adds one; it never bypasses `require_finance`/`can_edit`-style gates.
- **A write states its preview and its undo.** `render_preview` says what will
  change in the asker's words and money in `…Display` form; the undo names the
  verb that reverses it, or the spec says plainly that it cannot be undone and
  the preview says so too.
- **The coverage test stays green.** `every_billing_route_is_a_verb_or_an_exclusion`
  and `every_verbs_route_handler_calls_the_executors_core` are the gate: this
  track's work is exactly moving routes from one side of that test to the other.
- **Every new verb brings its `answers` questions**, and they must be askable
  without inventing a name — the A10.3 lesson: "what did we quote X" cannot be
  scored. Write "which offers were declined this month", not "was X declined".
- **Wire evidence per item**: the scripted-model suite proves the shape; the
  journal quotes one real exchange per item from a local run
  (`scratchpad`-style harness, the owner's `a91.py` is the pattern).
- **i18n** for anything user-visible, every language file.

## Wave VA — Billing's twenty

- [~] VA.1 **The offer's own end.** `/billing/quotes/{id}/decline` and
  `/billing/quotes/{id}/expire` — an offer closes as declined or lapsed.
  Reads first: "which offers were declined this month", "which offers have
  lapsed". Then the two writes, previewed, with the record view back.
- [~] VA.2 **Sending an invoice.** `/billing/invoices/{id}/send` composes the
  mail with the PDF. `draft_payment_reminder` already chases; this is the first
  send. It proposes a draft the asker approves — never a silent send — and the
  preview names the recipient and the document.
- [~] VA.3 **Corrections.** `/billing/invoices/{id}/void` and
  `/billing/invoices/{id}/credit-note`. Money moves and the books follow, so
  the preview must be exact and the undo must be honest (a credit note is not
  undone by deleting it). If the conclusion is that an agent must never do
  these, rewrite both exclusions with that reason and say so in the journal —
  that is a valid outcome for this item and only this item.
- [~] VA.4 **The price list.** `/billing/products` (list, create, update) and
  `/billing/products/{id}/archive`. Reads: "what do we charge for X", "which
  products are stocked", "what is our day rate". Writes: raise a product,
  change a price, archive one — each previewed against the current price.
- [~] VA.5 **The customer list's end.** `/billing/customers/{id}/archive`, with
  the read that makes it safe ("which customers have no open documents").
- [~] VA.6 **Supplier bills.** `/billing/bills` (list, one, approve, reject) —
  four routes, the purchase side of the same ledger. Reads: "which bills are
  waiting for approval", "what do we owe suppliers this month". Writes:
  approve, reject, each previewed with the supplier and the amount.
- [~] VA.7 **Recurring billing.** `/billing/schedules` (list, one, pause,
  resume, run). Reads: "what recurring billing runs this month", "which
  schedules are paused". Writes: pause, resume — and `run` only if the
  conclusion is that an agent may raise a scheduled batch at all; if not,
  rewrite that one exclusion permanently.
- [~] VA.8 **Catalogue connections.** `/billing/price-connections` and
  `/billing/price-connections/{id}`. Most likely outcome: a read ("which
  catalogue connections are healthy") plus permanent exclusions for the
  configuration itself. Whichever way, no route keeps "a later intent set".
- [~] VA.9 Wave review: the coverage test lists Billing's routes as verbs and
  permanent exclusions with **no "later intent set" left**; the count of verbs
  and exclusions before and after is in the journal; the standing evaluation
  questions for Billing still answer from the record in a fresh room, quoted;
  `CHANGELOG.md` has a user-readable line per shipped verb group. Then
  `LOOP COMPLETE`.
