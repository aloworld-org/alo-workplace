# alo agents — track A: the business modules (Sales, Finance, Projects, Inventory, HR)

**Read ADR 0057, ADR 0058, `docs/design/complete-agents.md`, then
`docs/autonomy/agents/QUEUE.md`'s "Areas and rules for waves A4–A9" — those
rules are this queue's rules too.** This track exists so that several loops
can move modules to intents at once without editing the same lines: it owns
**only** the files named below and lands each module as new files plus one
additive line in each shared list (`A4.1c` in the agents queue made that
possible; **do not start before it is `[x]` there**).

Since 2026-08-28 (A4.1c `[x]`) those lists are, exactly: one row
`(AgentProduct::<Product>, &<MODULE>_INTENTS)` in `alo_ai::agent_product::MOVED`,
one row `crate::<module>_intents::dispatch` in `alo_jmap::agent::MODULES`
(the module's `pub(crate) fn dispatch(state, account, tool, args) ->
Option<crate::agent::Dispatched>`, copied from Billing's), the `pub mod` line
in each `lib.rs`, and the routes in `server.rs`. A test on each side holds the
two lists to the same length, so a module registered in one and not the other
fails the gate. A rebase conflict on those rows is resolved by keeping both.

Every module moves the same way — copy Billing (`alo_ai::billing_intents`,
`alo-jmap`'s `billing_intents.rs`): an `IntentModule` in a new
`platform/alo-ai/src/<module>_intents.rs` (verbs with purpose, typed args,
`answers`, previews for writes, exclusions with reasons, guidance); executors
in a new `products/mail/alo-jmap/src/<module>_intents.rs` that return the
module's own record views (its routes' JSON) with readable amounts beside
integers; a scripted-model wire suite `tests/agent_<module>_intents_http.rs`
(a read answered from the record, wrong tenant, a write proposed and not run);
the hand-written `agent_<module>.rs` tool set in `alo-ai` deleted and its
executors kept or folded; the coverage test reading `server.rs` green. Reads
first: the questions a colleague would ask ("what do we have", "where are we
with X", "what is open/overdue/due"), then the writes the app already has.

Migrations for this track: **`0410`–`0429`**. Check the directory immediately
before rebasing.

## Areas this track owns

`platform/alo-ai/src/{crm,finance,projects,inventory,hr}_intents.rs`, the matching `products/mail/alo-jmap/src/*_intents.rs` executors and `tests/agent_*_intents_http.rs`; the existing `agent_crm.rs`, `agent_finance*.rs`, `agent_projects.rs`, `agent_inventory.rs`, `agent_hr.rs` (executors) in `alo-jmap`, and the same-named tool-set files in `alo-ai` (to delete).

## Never touch

`web/src/**` except `web/src/chat/**` (and only for a `[web]` item);
`agent.rs`, `agent_product.rs`, `lib.rs`, `server.rs` beyond the one additive
line per shared list; any other track's `*_intents.rs`; the store modules of
another product (a store function you need and do not find is added as a
**new** function, additive).

## Queue

- [x] AA.1 ★ **Sales (CRM)**: reads — open deals by stage and owner, deal lookup by name or company, pipeline summary, a contact's/company's history; writes — the existing `create_deal`, `move_deal_stage`, `draft_followup` with previews. `@crm which deals are open, and at what stage?` answers from the record.
- [x] AA.2 ★ **Finance**: reads — invoiced / paid / outstanding for a period (Finance's own ledger view, not Billing's), VAT summary, unmatched bank lines, expenses awaiting approval, an account's balance; writes — the existing `categorise_transactions`, `vat_summary`, `flag_anomalies` and approving an expense, with previews. `@finance how much have we invoiced this year, and how much is unpaid?` answers from the record.
- [x] AA.3 ★ **Projects**: reads — active projects overview, project lookup with status, budget and hours, who is on what, this week's time; writes — the existing `log_time`, `draft_timesheet_from_calendar` with previews. `@projects which projects are active?` answers from the record.
- [x] AA.4 ★ **Inventory**: reads — `stock_answer` kept, plus stock below minimum, open purchase orders, a supplier's prices, recent moves; writes — the existing `reorder_proposals` and receiving a delivery, with previews.
- [x] AA.5 ★ **HR (People)**: reads — `who_is_off` kept, plus who works here (directory by team), leave balance for the asker, open leave requests for a manager, onboarding checklists open; writes — the existing `draft_letter_from_template` and approving a leave request (manager only, as the asker), with previews.
- [x] AA.6 Wave review: every module above answers its `answers` questions in the scripted suites; hand-written tool sets gone; `docs/design/complete-agents.md` lists the five modules as moved; `CHANGELOG.md` says what a user can now ask. Then `LOOP COMPLETE`.
