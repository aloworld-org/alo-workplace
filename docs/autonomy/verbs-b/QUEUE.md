# The rest of the apps their agents cannot reach — everything but Billing

The A9.1 evaluation (`docs/autonomy/agents/STATE.md`, 2026-08-30) measured what
the sixteen agents actually cover: **136 verbs against 347 excluded routes**, and
**47 of those exclusions say in their own words "a later intent set"** — not
decisions an agent should never make, just work nobody has done yet. Billing's
twenty are track `verbs-a`. This track is the other **twenty-seven**, across
eight modules.

**Read first, in this order:** ADR 0057, ADR 0058, `docs/design/complete-agents.md`,
then `alo_ai::intent`, `alo_ai::billing_intents` and `alo-jmap`'s
`billing_intents.rs` — Billing is the reference module every other one copies.

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
   do this. Several below almost certainly belong here: ticking somebody's
   checklist is finishing their work in small steps, and that is a reason, not
   a delay. Say it permanently and move on.

A route left with "a later intent set" at the end of this track is the item not
being done.

## Areas this track owns

`platform/alo-ai/src/{drive,crm,finance,tasks,chat,hr,projects,sites}_intents.rs`
and their `alo-jmap` executor twins, plus new executor modules where a subject
deserves its own file. Store surfaces may gain **new** functions, never changed
ones. **Not** `billing_intents.rs` (track `verbs-a` is in it), not `web/**`,
not `agent.rs`/`agent_product.rs` beyond rows that already exist. Migrations
`0950`–`0969` if genuinely needed. `web/src/sites/**` stays read-only (ADR 0045).

## Rules

Identical to `verbs-a`: a verb calls the same core the route's handler calls
(A4.1b), so the agent's authority is exactly the screen's at the asker's own
scope; a write states its preview and its undo; the module's coverage test is
the gate; every new verb brings `answers` questions that are askable **without
inventing a name** (the A10.3 lesson — "which files were copied this week", not
"was X copied"); each item quotes one real-model exchange in the journal; i18n
for anything user-visible.

## Wave VB — the twenty-seven

- [ ] VB.1 **Drive: alo Base.** Seven routes (`/drive/base`, `/drive/base/{node}`,
  `/drive/base/{node}/tables`, `/drive/base-tables/{table}/{fields,records,views}`,
  `/drive/base-records/{record}`). ADR 0032 says structured tables are their own
  surface — so the honest first question is whether Base gets **reads** ("which
  tables are in the customer base", "what is in the orders table") and permanent
  exclusions for the schema editing, or a fuller set. Decide it in the journal
  against the ADR, then do exactly that.
- [ ] VB.2 **Drive: the file's own history.** `/drive/nodes/{id}/copy` and
  `/drive/nodes/{id}/versions`. Reads: "what versions does this file have",
  "who changed it last". Write: copy, previewed with the destination.
- [ ] VB.3 **Sales: the won-deal handoff.** `/crm/deals/{id}/quote` and
  `/crm/deals/{id}/invoice` raise a Billing draft from a won deal. The
  exclusion says offers are Billing's to propose — which is exactly what
  delegation is for (A5), so the likely shape is a CRM verb that hands off to
  @billing rather than a second invoice-raiser. Prove it end to end in a room.
- [ ] VB.4 **Sales: the conversation on the deal.** `/crm/deals/{id}/threads`,
  `/crm/deals/{id}/threads/{threadId}`, `/crm/deals/{id}/thread-suggestions`.
  A read for the suggestions ("which conversations look like they belong to the
  Northstar deal") and a proposed link; unlinking may well be permanent.
- [ ] VB.5 **Finance: the bank line.** `/finance/bank/suggestions`,
  `/finance/bank/lines/{id}/match`, `/finance/bank/lines/{id}/unmatch`.
  Reconciliation is decided line by line — so the read is the value here
  ("which bank lines look like they match an invoice"), and a match may be
  proposed with the document named in the preview. Unmatch is likely permanent.
- [ ] VB.6 **Finance: the statements.** `/finance/reports/pl` and
  `/finance/reports/balance`. Reads only, answered as figures with the period
  stated, never as a screenshot of a screen.
- [ ] VB.7 **Tasks: the checklist and the labels.** `/tasks/{id}/subtasks`,
  `/tasks/{id}/subtasks/{sid}`, `/tasks/{id}/labels`, `/tasks/{id}/labels/{lid}`.
  Reading a task's checklist is plainly a verb; **ticking somebody's item is
  plainly not** — write both conclusions permanently rather than deferring them
  a second time.
- [ ] VB.8 **The four singles.** `/chat/channels/{id}/join` (the asker's own
  step — probably permanent), `/hr/openings` (no verb reads an opening at all:
  "which roles are we hiring for" is an obvious read), `/projects/unbilled`
  ("which hours are ready to invoice" — a read that feeds @billing, and a
  delegation worth proving), `/sites/{id}/schedule` (scheduling a publish
  beside the clock it obeys).
- [ ] VB.9 Wave review: no module in this track's areas keeps a "later intent
  set" exclusion; the before/after verb and exclusion counts per module are in
  the journal; the standing evaluation question for each touched agent still
  answers from the record in a fresh room, quoted; `CHANGELOG.md` has a
  user-readable line per shipped verb group. Then `LOOP COMPLETE`.
