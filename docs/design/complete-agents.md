# Complete agents — intents, records, events; delegation, channel memory, standing instructions, goals

*Design note for [ADR 0057](../decisions/0057-one-agent-per-app-complete-over-its-api.md)
(what an agent is) and [ADR 0058](../decisions/0058-intents-records-events-one-command-layer.md)
(how the app and its agent share one command layer). Extends `chat-agents.md`,
which stays authoritative for identity, authority, approval and the room
mechanics. The agents queue (`docs/autonomy/agents/QUEUE.md`, waves A4–A9)
executes it.*

## The thesis

Work is a graph of **records**, **verbs** and **events**. People and agents
operate the same graph through the same verbs. Language is one interface to
it; screens are another. Nothing an agent does is outside the app, and nothing
the app does is invisible to the agent. That — not a chat bolted on — is what
AI-native means here, and it is possible because we own every app on one
store.

## Surface

| Who | Sees |
|---|---|
| A person in a channel or DM | `@billing which quotes are open?` answered from the record with links into it; `@billing invoice the Northstar quote` previewed and proposed with one tap; a visible line when Billing hands part of the work to Sales; a **standing instruction** card the author can cancel; a **What I remember** page per agent per channel. |
| A person inside an app | The same agent on the **record in focus** — in the quote editor, "chase it", "make this an invoice", "why is the VAT 9 %" without naming anything. |
| A module owner | One intent registry to keep true; a coverage test that fails when a route is neither an intent's adapter nor an exclusion with a reason. |
| An admin | The agent directory: every agent, every intent it may run, its run record — complete because it is rendered from the registry. |

## 1. Intents — the one command layer

Each module defines its verbs once, in `platform/alo-ai/src/<module>_intents.rs`
(the words the model and the directory read) with executors in
`products/mail/alo-jmap/src/<module>_intents.rs` (the code that runs them):

```rust
IntentSpec {
    name: "open_quotes",
    purpose: "The offers still open — sent and not yet answered — newest first, each with what it is worth.",
    effect: Effect::Read,
    args: &[Arg::optional("customer", "text", "limit to one customer, by name")],
    answers: &["which quotes are open", "what have we offered lately", "what is X worth"],
    preview: None,
}
IntentSpec {
    name: "send_quote",
    purpose: "Send an offer: it gets its number, the validity clock starts, and the document is frozen.",
    effect: Effect::Write,
    args: &[Arg::required("quote", "text", "the quote's number, or the customer's name for their open draft")],
    answers: &["send the quote", "send Northstar their offer"],
    preview: Some("Quote {quote} for {customer} will be numbered and sent; its lines and design will be frozen."),
}
```

Rules:

- **One registry.** The prompt's `- name: …` lines, the tool set, the
  execution boundary's allow-list (`offers`) and the directory's `tools` are
  all rendered from `IntentSpec`s. The hand-written `*_TOOLS` / `*_TOOL_DOC`
  constants are deleted as each module moves over.
- **Routes are adapters.** A route handler does argument parsing and calls the
  same function the intent executor calls. Coverage: every `/<module>/…`
  route in `server.rs` is named by exactly one intent (`route:`) or by an
  `Excluded { route, why }`; the test reads the router source and fails on a
  route that is neither.
- **Writes preview.** A write's `preview` is rendered with the resolved
  arguments and shown on the proposal card before anyone taps; it is what the
  action record keeps as "what this would do". An intent with an inverse
  declares `undo: Some("discard_invoice_draft")`; one without says so in its
  purpose ("an issued invoice cannot be un-issued").
- **Argument resolution is the executor's**, not the model's: a customer may
  be named; a quote may be named by number *or* by customer; the executor
  resolves, refuses ambiguity with a sentence ("two customers match 'North'"),
  and never guesses.
- **Effect is declared once** (ADR 0047) and read at the boundary.
- **Multi-step reads**: up to **six** read executions and two model calls per
  turn; a write ends the turn with one proposal. Constants in code.

### Billing — the reference module

Reads: `open_quotes`, `quote_lookup` (by number or customer), `customer_lookup`,
`unpaid_invoices` (overdue flagged), `invoice_lookup`, `billing_totals` (a
period: invoiced, paid, outstanding, VAT). Writes: `create_invoice_draft`,
`quote_to_invoice`, `draft_payment_reminder` (existing), `send_quote`,
`issue_invoice`, `record_payment`. Excluded with reasons: print/PDF/e-invoice
files (served, not decided), the quote design (the studio's), settings and FX
rates (a person's configuration), bills import and SEPA export (take or
produce files), schedules (a later intent set).

### Moved modules

`alo_ai::agent_product::MOVED` is authoritative for what is moved at any
moment (a test holds it to the same length as the executor list). This list
records what a **wave review** has confirmed — the module's scripted suite
answering its `answers` questions on the wire, the hand-written tool set
deleted — one line per module, added by the track that reviewed it:

- **Sales (CRM)** — 2026-08-28 (AA.1): reads `open_deals`, `deal_lookup`,
  `pipeline_summary`, `company_history`; writes `create_deal`,
  `move_deal_stage`, `draft_followup`, each with a preview.
- **Finance** — 2026-08-29 (AA.2): reads `ledger_summary`, `vat_summary`,
  `flag_anomalies`, `unmatched_bank_lines`, `expenses_awaiting`,
  `account_balance`; writes `categorise_transactions`, `approve_expense`,
  behind the same gate as the Finance screens.
- **Projects** — 2026-08-29 (AA.3): reads `active_projects`,
  `project_status_summary`, `who_is_on_what`, `time_this_week`; writes
  `log_time`, `draft_timesheet_from_calendar` — both still land proposals.
- **Inventory** — 2026-08-29 (AA.4): reads `stock_answer`,
  `stock_below_minimum`, `open_purchase_orders`, `supplier_prices`,
  `recent_moves`; writes `reorder_proposals`, `receive_delivery`.
- **HR (People)** — 2026-08-29 (AA.5): reads `who_is_off`, `who_works_here`,
  `my_leave_balance`, `open_leave_requests`, `open_checklists`; writes
  `draft_letter_from_template`, `approve_leave_request` — the personal note
  on a leave request stays in the app, stripped from what the model reads.

## 2. Record views and provenance

- A module's **record view** is the JSON its own detail route returns
  (`document_json` for a quote or an invoice, `customer_json` for a customer).
  An intent's read returns that shape, trimmed to what the turn needs, so an
  agent grounds in exactly what a person sees. No separate summary.
- **Provenance** is a field on the record: `origin` = `{kind, id, label}` —
  the thread a task came from, the quote an invoice came from, the meeting a
  decision came from. Modules set it where they create records; intents
  return it; agents cite it ("from the Northstar thread, 12 Aug").

## 3. Events as perception

Every intent execution emits `Event { tenant, kind: "quote.sent", record,
actor, on_behalf_of, at }` on the tenant's event stream (`alo-store`,
`events` table, append-only). Consumers: notifications, audit, standing
instructions (module-event triggers) and memory extraction. Nothing polls the
record tables for change.

## 4. Actions are objects

A proposal is already a row (`chat_proposals`). It grows into the **action
record** every intent execution leaves, whoever asked: intent, arguments,
preview, actor, on_behalf_of, result, undo (the inverse intent and its
arguments, when there is one). A person's click through the UI leaves the same
row. Consequences that follow from this and nothing else:

- **Hand an action to an agent**: an open proposal can be reassigned
  ("@billing, you finish this").
- **Assign a task to an agent**: a task whose assignee is an agent is a
  standing instruction with a due date; the board is the agents' work queue.
- **Undo an agent** with the button that undoes a person.

## 5. Delegation

Inside one run, an agent may call another module's intent as the asker:

```
{ "delegate": { "to": "crm", "ask": "which deal is behind quote Q-2026-00031?" } }
```

Depth ≤ 2, at most four per run, one budget; the room sees *Billing asked
Sales: …*; the delegate's answer is folded in and cited; a write proposed by a
delegate lands on the asker's one approval surface; only agents the asker can
see; never across a shared channel. Ask alo's planner is this mechanism, not a
second one.

## 6. Channel memory

**The channel is the consent boundary.** What was shared in a channel may be
remembered by the agents in it, whoever wrote it; it is usable in that channel
and nowhere else. A DM with an agent feeds only that person's memory.

- Facts and decisions with the message they came from, never transcripts;
  learned at the end of a turn from what the turn read, or on an explicit
  *"remember that …"* (which works even where learning is off).
- Per-channel switch (room settings, on by default); workspace default in the
  admin console; off hides that channel's memories (deleted after 30 days).
- *What I remember* — per agent per channel; every member reads; the owner or
  the source author forgets.
- Deletion follows the source: message deleted, channel archived, agent
  removed → its memories go.
- Retrieval only inside scope: the channel the turn runs in, or the asker's
  DM memory. No cross-channel pooling in v1. Records are the memory of facts;
  channel memory is for what is not in a record; **policy memory** ("we
  invoice net 30", "Anna approves discounts over 10 %") is explicit, per
  agent, tenant-wide, editable — a later item.

```sql
CREATE TABLE agent_memories (
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    id TEXT NOT NULL, agent_id TEXT NOT NULL,
    scope TEXT NOT NULL,             -- 'channel' | 'person'
    channel_id TEXT, user_id TEXT,
    fact TEXT NOT NULL, source_msg TEXT, learned_from TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, id)
);
```

## 7. Standing instructions

A person asks once, in advance. A card in the channel with the instruction in
the author's words, the trigger (a schedule, or a module event the intent
registry names — `invoice.overdue`, `quote.expiring`), the agent, and
*Cancel* for the author and the room owner. Each firing is a turn with the
author as asker: reads post, writes propose to the author. Bounds: one firing
per instruction per hour, twenty per channel, paused when the author leaves.
`agent_instructions` (tenant, id, agent, channel, author, text, trigger,
next_run, paused), run on the scheduled-mail sweeper.

## 8. Goals

"Close the Northstar deal by Friday" is a goal record: the plan Ask alo made
(steps, each an agent and an intent), progress, one approval surface, Stop.
Agents work its steps; humans approve its writes; the room sees the card.
Coordination happens through the object, never through agents talking.

## 9. Evaluation is the acceptance test

The seventeen-question run of 2026-08-28 (`STATE.md`) plus every intent's
`answers`, scripted, against a real model, answers quoted verbatim with the
tool runs behind them — the exit gate of every wave. A green suite proves a
tool runs; the run proves an agent answered.

## Errors

| Case | Answer |
|---|---|
| Intent the agent was not offered | `403` at the boundary, run recorded |
| Ambiguous argument (two customers match) | the read answers with the candidates; a write is not proposed |
| Delegation beyond depth or count | the step is dropped; the agent says which part it could not do |
| Budget exhausted | the turn ends with what it has, marked partial in the room |
| Memory learning off | nothing remembered; "remember that …" still works |
| Instruction's author gone | paused, the card says so |

## Tenancy

Every intent runs through the asker's account door via the module's own store
functions; the action record carries tenant and asker; memories and
instructions carry `tenant_id` and a scope the retrieval joins on. Wrong-tenant
and wrong-channel tests are mandatory for every new table; a delegate turn is
the asker's turn.

## Rollout

Per tenant, default off: `agents.intents` (per module as each moves over),
`agents.delegation`, `agents.memory`, `agents.instructions`, `agents.goals`.
Watch: tool runs per agent per day and their `ok` ratio, turns that hit a
budget, delegations per run, memories per channel, instruction firings,
undo per action kind.

## Out of scope for this note

Cross-channel memory; proactive offers; agent-to-agent conversation outside a
run; delegation across shared channels; voice; bringing outside agents into a
room; intents for the admin console.
