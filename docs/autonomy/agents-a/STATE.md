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
