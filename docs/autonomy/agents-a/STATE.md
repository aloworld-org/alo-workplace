# alo agents — agents-a build journal

One entry per completed queue item: what was built, what the isolation tests
proved, and the scripted-model transcript quoted, not summarised. Started
2026-08-28 from `docs/autonomy/agents/QUEUE.md`'s split into parallel tracks
(ADR 0057, ADR 0058).

## AA.1 — Sales (CRM) intents (2026-08-28)

**Shipped.** CRM moved to the intent pattern, copying Billing: a new
`alo_ai::crm_intents` (`IntentModule` with 4 reads — `open_deals`,
`deal_lookup`, `pipeline_summary`, `company_history` — and the 3 existing
writes `create_deal`, `move_deal_stage`, `draft_followup`, each write now
carrying a preview; 17 excluded routes with reasons; guidance); new executors
in `alo_jmap::crm_intents` answering with the routes' own record views
(`deal_json`, `event_json`, `activity_json`, `report_json` — three widened to
`pub(crate)`), money made readable beside its integers through the shared
`billing_intents::ok` rendering; the hand-written `alo_ai::agent_crm` tool set
deleted; `alo_jmap::agent_crm` kept as the three write executors, reached only
from the new dispatch. Registration is one row per shared list (A4.1c):
`(AgentProduct::Crm, &CRM_INTENTS)` in `MOVED`, `crate::crm_intents::dispatch`
in `agent::MODULES`, one `pub mod` line per `lib.rs`. The three CRM match arms
in `agent.rs`'s legacy match were removed with the move (they were unreachable
behind the MODULES row — same shape as Billing at A4.1c).

**Verified.** `cargo fmt`; clippy `-D warnings` clean on alo-ai and alo-jmap;
nextest green: alo-ai 273/273, alo-jmap 1390/1390 (the full suite; the
test-binary rebuild took 102 min wall — this Mac runs three loops). The new
`tests/agent_crm_intents_http.rs` covers the queue's three demands on the
wire with a scripted model. The `@crm which deals are open, and at what
stage?` transcript, quoted: user posts "@crm which deals are open, and at
what stage?"; model call 1 returns `{"tool":"open_deals","args":{}}` saying
"Let me look at the board."; the tool result shown to the model on call 2
carries `"Kestrel Windfarm"`, `"Falcon Rollout"`, `"dealCount":2`, each
card's `stageName`, `byStage`/`byOwner` tallies and
`"valueCents":500000` beside `"valueDisplay":"5000.00 EUR"`; the agent
answers "Two deals are open: [1]." with no proposal. `deal_lookup` by
company returns the moved deal with its history (`toStageName`) and the
"Called Ada — she wants a revised offer" note. `move_deal_stage` comes back
as a proposal with the card unmoved (stage re-read from the record).

**Isolation.** `another_tenants_deal_is_unreachable_by_its_exact_title`: two
tenants on one store; tenant B holds "Kestrel Windfarm"; tenant A's agent
runs `deal_lookup` on that exact title and the model is shown only "this
lookup did not run: no deal is titled …" — none of B's company, contact
address or value appears in the sources. All reads/writes go through
`account.acc` (`for_account`); resolution by title only ever sees the
tenant's own deals.

**Cuts (scope, not depth).** (1) `open_deals`' `owner` arg accepts "me"
only — there is no user-name lookup on the agent side; every listed deal and
the `byOwner` tally name `ownerUserId` and a `mine` flag instead. A
colleague-by-name filter needs a directory read and is a later intent. (2)
`draft_followup` has no `/crm` route to stand behind (the draft lands in
Mail's Drafts), so its `routes` is empty and the module test states that one
exception. (3) No A4.1b-style route-adapter restructuring of the five CRM
route files: executors reuse the same store functions and the routes' own
JSON views, but the "handler calls the shared core" drift test remains
Billing-only. (4) `/crm/deals/{id}/next-steps` excluded (a next step is a
Tasks record; the Tasks agent's to read).

**Registry ripples (expected, same as A4.1):** counts and examples updated in
alo-ai tests — 80→84 tools, the four CRM reads added to the declared-reads
lists (alo-ai `agent.rs`, alo-jmap `agent_turn.rs` 40→44), CRM is no longer
the write-only-product example (none remains), the planner-roster test now
asserts CRM *has* "Ask it for:" hints and uses Projects as the not-yet-moved
example, guidance marker "For a CRM tool" → "For a CRM verb".

No migration used (0410–0429 untouched), no new route prefixes, no UI
strings. Next: AA.2 Finance.

## AA.2 — Finance intents (2026-08-29)

**Shipped.** Finance moved to the intent pattern, copying Billing/CRM: a new
`alo_ai::finance_intents` (`IntentModule` with 6 reads — `ledger_summary`,
`vat_summary`, `flag_anomalies`, `unmatched_bank_lines`, `expenses_awaiting`,
`account_balance` — and 2 writes `categorise_transactions` (kept) and the new
`approve_expense`, each write with a preview; 30 excluded routes with
reasons; guidance, marker "For a Finance verb"); new executors in
`alo_jmap::finance_intents` behind the same `require_finance` gate as the
finance screens, answering with the store's own figures — `ledger_summary`
reads the AR-role account's `fin_account_ledger` (invoiced/credit-noted/paid
split by entry kind, closing balance as outstanding, the queue's "Finance's
own ledger view, not Billing's"), `account_balance` the same
`fin_trial_balance` fold as `GET /finance/accounts?from&to`,
`expenses_awaiting` the approvals inbox's `pending_json` rows,
`unmatched_bank_lines` the bank page's `line_json`. The hand-written
`alo_ai::agent_finance` tool set deleted; `alo_jmap::agent_finance` (categorise
+ new approve executor) and `agent_finance_answers` (VAT, anomalies) kept as
executors reached only from the new dispatch, their answers now flowing
through `billing_intents::ok` so money displays sit beside integers.
Registration is one row per shared list (A4.1c):
`(AgentProduct::Finance, &FINANCE_INTENTS)` in `MOVED`,
`crate::finance_intents::dispatch` in `agent::MODULES`, one `pub mod` line per
`lib.rs`. The three Finance arms in `agent.rs`'s legacy match removed with the
move (unreachable behind the MODULES row).

**Verified.** `cargo fmt`; clippy `-D warnings` clean on alo-ai and alo-jmap;
nextest green: alo-ai 274/274, alo-jmap 1411/1411 (full suite; test-binary
rebuild 91 min wall — three loops on this Mac). The new
`tests/agent_finance_intents_http.rs` covers the queue's demands on the wire
with a scripted model. The `@finance how much have we invoiced this year, and
how much is unpaid?` transcript, quoted: user posts the question; model call 1
returns `{"tool":"ledger_summary","args":{}}` saying "Let me read the
ledger."; the tool result shown on call 2 carries
`"invoicedCents":121000` beside `"invoicedDisplay":"1210.00 EUR"`,
`"paidCents":60500`, `"outstandingCents":60500` beside
`"outstandingDisplay":"605.00 EUR"` and the `INV-2026-00001` entry trail; the
agent answers "Invoiced 1210.00 EUR this year; 605.00 EUR is unpaid [1]."
with no proposal, and the prompt offered all 8 verbs. `approve_expense` comes
back as a proposal with the claim still `submitted` (re-read from the
record).

**Isolation.** `another_tenants_waiting_claims_are_unreachable`: two tenants
on one store; tenant B holds a submitted claim from "Glasshouse Pharma";
tenant A's agent runs `expenses_awaiting` and is shown `"expenseCount":0` —
neither B's merchant, amount nor claimant email appears in the sources. The
queue reads go through `state.store.for_tenant(account.tenant)` exactly as
the approvals inbox does, behind `require_finance`.

**Cuts (scope, not depth).** (1) `ledger_summary`, `flag_anomalies` and
`categorise_transactions` stand behind no route (named exceptions in the
module test): the AR drill-down and the anomaly scan have no `/finance` route
to adapt, and categorise writes suggestions no route writes. (2) The P&L,
balance-sheet and aged report screens stay excluded ("a later intent set") —
`ledger_summary` and `account_balance` answer the questions the queue names.
(3) `approve_expense` resolves within the pending queue by merchant (+
optional claimant email / spentOn); rejection and reimbursement stay
person-only in the app, stated as exclusions. (4) No A4.1b-style
route-adapter restructuring of the finance route files: executors reuse the
same store functions and the routes' own JSON views (`pending_json`,
`line_json`, `account_json` widened to `pub(crate)`), but the "handler calls
the shared core" drift test remains Billing-only.

**Registry ripples (expected, same shape as AA.1):** 84→89 tools, reads
44→48 (`agent.rs` reads list + `agent_turn.rs` count), Finance's product
tool list rewritten in `agent_product.rs`, guidance marker "For a finance
tool" → "For a Finance verb".

**Rebase note, and three repairs beyond this item's own files.** The Chat
(AC.1) and Drive (AB.1) moves and the agent-memory item landed on main during
this item's 91-minute test build, and `origin/main` itself did not compile or
gate: (1) `agent_product.rs` still held the dead `CHAT_SET`/`DRIVE_SET`
consts referencing `CHAT_TOOLS`/`DRIVE_TOOLS` that both moves had deleted —
removed, `Chat`/`Drive => &[]` in `static_sets` (the 93/50 counts upstream
asserted only hold that way); (2) `chat_intents.rs`'s `post_message` executor
called `answer_if_asked` with four args while the memory item had made it
five — the message id now passes through, as `chat.rs` does; (3) the memory
item's `/chat/channels/{id}/memory` route had no row in Chat's coverage —
excluded ("the owner's setting in the app"). And the same orphaned-suite trap
ce8df4ac repaired had reopened: `agent_drive_intents_http.rs` (AB.1) landed
standalone after B7.04 turned autotests off, silently never built — wired
into `agents_http_suite` together with this item's
`agent_finance_intents_http.rs` (which the pre-B7.04 base did run). Kept-both
on `MOVED`, `MODULES` and the `lib.rs`/CHANGELOG lines; merged counts
`all_tools` 98, declared reads 54. Full merged-tree gate after the repairs:
alo-ai 275/275, alo-jmap 1713/1713 (agents_http_suite 134/134 with the
finance and drive wire tests listed by name), clippy clean on both.

No migration used (0410–0429 untouched), no new route prefixes, no UI
strings. Next: AA.3 Projects.

## 2026-08-29 — AA.3 Projects moved to intents

**Shipped.** `alo_ai::projects_intents` (six verbs) and `alo-jmap`'s
`projects_intents.rs` executors + dispatch. Reads: `active_projects` the
portfolio exactly as `GET /projects` serves it (`project_json` widened to
`pub(crate)`), unfinished by default with an exact-status filter;
`project_status_summary` kept; `who_is_on_what` NEW — open tasks grouped per
colleague across the asker's visible boards (counts + boards, labels via
`emails_of`, unassigned last, no titles, no hours); `time_this_week` the
asker's own `GET /projects/time` week (default Monday–Sunday of today's week,
a stated `from` runs to its own week's Sunday), totals via `week_totals` with
suggestions counted apart. Writes kept with previews: `log_time`,
`draft_timesheet_from_calendar` — both still land `proposed` entries; their
kept executors (`agent_projects.rs`, `agent_timesheet.rs`) now answer through
`billing_intents::ok` so money displays sit beside integers, reached only from
the new dispatch. `alo_ai::agent_projects` (tool set) deleted; registration is
one row per shared list; the three legacy arms in jmap `agent.rs` removed.
Every `/projects` route is a verb's or excluded with a reason (coverage prefix
`/projects`, so the bare list route is covered too); `who_is_on_what` is the
one named routeless verb.

**Verified.** fmt; clippy `-D warnings`-clean both crates; nextest green:
alo-ai 278/278, alo-jmap full suite 1461/1461 with
`agent_projects_intents_http` (3) listed by name. The `@projects which
projects are active?` transcript: model call 1 returns
`{"tool":"active_projects","args":{}}`; the sources on call 2 carry the
running board with `"status":"active"` and its `openTasks`, and the
`completed` board is absent; the agent answers with no proposal, and the
prompt offered all 6 verbs. `log_time` comes back as a proposal with
`/projects/time/proposals` still empty (nothing ran without a tap).

**Isolation.** `another_tenants_boards_are_unreachable`: tenant B's board
"Project Nightingale" never appears in what tenant A's agent is shown.

**Cuts (scope, not depth).** (1) `who_is_on_what` is counts per person per
board, not task listings — allocation, not somebody's worklist; overdue_by_owner
(Tasks) already lists late tasks. (2) No route-adapter restructuring of the
projects route files: executors reuse the routes' own store functions and JSON
views; the drift test stays Billing-only. (3) The unbilled/profitability/plan
screens stay excluded ("a later intent set" where applicable).

**Two mid-item rebases (AC.2, then AB.2, landed during the gates).** agents-c
pushed the Meet move + the 0408 memory-deletion migration while this item
built; the shared scratch DB was already at 408, which surfaced as
`Migrate(VersionMissing(408))` in `readiness` — the fix was the rebase itself.
Then the Docs move (AB.2) landed between my gate and my push. Keep-both on
`MOVED`, `MODULES`, both `lib.rs`; the registry counts are the one
non-additive merge point, resolved by summing the deltas: final `all_tools`
106 (98 + Meet 3 + Docs 2 + Projects 3), declared reads 60 (54 + 2 + 1 + 3).
`agent_plan` roster test updated: Projects now carries "Ask it for:" hints,
Inventory is the still-static example. Full gate re-run after each rebase.

No migration used (0410–0429 untouched), no new route prefixes, no UI strings.
Registry ripples as expected: prompt ordering test now pins `create_deal <
active_projects < log_time`; guidance marker "For a projects tool" → "For a
Projects verb". Next: AA.4 Inventory.
