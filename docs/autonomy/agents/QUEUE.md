# alo agents — build queue

An agent in every product (ADR 0034), which the roadmap's Agent track orders as
waves A1–A3. This queue executes it.

**Read ADR 0034 first, then ADR 0023.** 0034 decided the shape — every product
has an agent, agents are first-class chat participants, "Ask alo" orchestrates
above them. None of that is up for discussion here. 0023 fixed
propose-then-approve, and A1.1 below narrows it rather than overturns it.

## What is actually wrong today

The framework is built and six agents have tools, but they share one brain:

- `ChatAgent` is `{id, handle, name, description, disabled}` — **no product**.
- `alo_ai::run_agent(config, question, ground, today, folders)` takes no agent
  and no scope, so every agent is offered all 33 tools.
- The turn grounds itself with `workspace_search_terms(question, 8)` — one
  generic full-text search, identical for every agent.
- Every tool arrives as a proposal, **including the read-only ones**. Asking
  "is the X100 in stock?" offers a button instead of an answer.

So "the Inventory agent" is a name on a generic assistant. A1 fixes that for
all of them at once and gates the rest of the queue.

## Areas this track owns

`platform/alo-ai/**` (except the sites modules — see below),
`platform/alo-store/src/chat_agents.rs` and the agent-facing store surface,
`products/mail/alo-jmap/src/agent*.rs` and `chat_agent*.rs`, and its own
migrations.

**Migrations are `04xx`.** The sites loop is at `0322` and climbing; 03xx is
theirs. Check the directory again immediately before rebasing, not once at the
start of the item — that is how the last collision happened.

## Areas and rules for waves A4–A9 (read before any item)

- **Read ADR 0057, ADR 0058 and `docs/design/complete-agents.md` first**, then
  `alo_ai::intent` and `alo_ai::billing_intents` (the reference module) and
  `alo-jmap`'s `billing_intents.rs` (its executors). Every module moves the
  same way: an `IntentModule` in a new `platform/alo-ai/src/<module>_intents.rs`
  (verbs, exclusions with reasons, guidance), executors in a new
  `products/mail/alo-jmap/src/<module>_intents.rs` returning the module's own
  record views, dispatch lines in `agent.rs`, the hand-written `agent_<module>.rs`
  tool set in `alo-ai` deleted, `agent_product.rs` pointing the product at the
  module, and a coverage test that reads `server.rs` — copy Billing's shape.
- **Web:** `[web]` items may touch `web/src/chat/**` and new `web/src/agents/**`
  only. `web/src/billing/**`, `web/src/shell/**` and `web/src/ds/**` are being
  edited by another agent right now — never open them. Store-and-API first;
  a `[web]` item whose UI cannot be built inside those bounds is marked `[!]`
  with the reason.
- **Other modules' route files** (`billing_quotes.rs`, `crm.rs`, …) are edited
  only to make a route call an intent executor, behaviour unchanged. Their
  store modules are read, never restructured; a store function an intent needs
  and does not exist is added as a **new** function in the module's store file,
  additive.
- **`alo-ai`'s sites modules** (`sites.rs`, `site_edits.rs`,
  `site_translation.rs`) belong to the sites track; `sites_intents.rs` reads
  them and the existing `agent_sites.rs` executors.
- **The gate of every item is the wire.** The scripted-model suites
  (`tests/agent_*_http.rs`, the `common::model` harness) prove each verb on
  the real router and store, wrong tenant included. The **real-model
  evaluation** (`docs/autonomy/agents/STATE.md`, the 2026-08-28 run) is run by
  the owner after each wave with the tenant's own provider; do not copy an API
  key anywhere to run it yourself. Quote the scripted transcript in STATE.md.
- **A migration is `04xx`.** Check the directory immediately before rebasing.
- **`[~]` for what is not this queue's to build**, with the reason; never leave
  a `[ ]` you will not build.

---

## Wave A1 — what makes an agent a *product* agent

- [x] A1.0 The two ADRs this wave needs, one commit, no code: **reads answer, writes propose** (narrowing ADR 0023 to writes — state plainly that a read behind an approval button was the bug, not the design), and **an agent as a DM counterpart** (a channel kind, since `dm_key` is a pair of user ids and cannot express it). Number them after the highest existing ADR; check that number again immediately before committing.
- [x] A1.1 Reads answer, writes propose. A read-only tool returns its answer into the room immediately; only a tool that changes something waits for approval. The split is a property of **the tool**, declared once in the registry, never a judgement made at the call site. Both paths audited. Test that a read never creates a proposal row and that a write never executes without one.
- [x] A1.2 A product on the agent record (migration `04xx`), and the tool registry scoped by it: an agent is offered its own product's tools in the prompt **and refused the others at the execution boundary**. The boundary test is the one that matters — a prompt that asks nicely is not a permission system.
- [x] A1.3 Product-scoped retrieval: each agent grounds in its own product's records rather than one shared workspace search. `Ask alo` keeps the workspace-wide view; it is the only agent allowed to look everywhere. Prove a Mail agent's grounding contains no Drive rows.
- [x] A1.4 One-to-one with an agent: a DM whose counterpart is an agent. New channel kind, listed beside human DMs, the agent answers there exactly as it does in a channel. API and store only — the room list rendering is `[web]` and comes after the chat rebuild.
- [x] A1.5 The default agent set: a tenant gets its agents without an admin registering handles by hand, and a module the tenant cannot open has no agent. Reuse the existing per-user module access rather than inventing a second gate; a denied module must yield no agent in any surface.
- [x] A1.6 The isolation tests, one per surface — channel, agent DM, in-module. Wrong tenant and wrong user both prove an agent reaches nothing the asker could not, including a private channel the asker is not in and a colleague's diary.
- [x] A1.7 The two questions end to end, on the wire, against the local backend: `@mail are we in contact with ABC?` answers from correspondence with the messages behind it, and `@inventory is the X100 in stock?` answers from the stock record — **no button in between**. Record the actual request and response in STATE.md, not a claim that it worked.

## Wave A2 — the agents with no tools yet

Each item is that agent's reads **and** its writes, to the implement skill's
definition of done. An agent that can only answer is half a product. Every one
of them ends with a real question asked and answered on the wire.

- [x] A2.1 ★ Website (Sites) agent — new `agent_sites.rs`: answer from the live site, draft and edit a page, translate the site, review SEO. Publishing is proposed, never silent. Read the sites modules; do not edit them.
- [x] A2.1b Translating the site through the agent — the half of A2.1 that was cut rather than half-built. The write needs the source the sites track assembles in `alo-jmap`'s `sites.rs` (`translation_source`, private, and that file is theirs), so doing it meant either editing their file or copying sixty lines of their assembly that would then drift. Shape it as a **read** the agent owns plus the existing route: `site_translation_status` over `site_translation_readiness` (which language is short how many pages), and the translating itself stays on `POST /sites/:id/translation-proposals`. If a write is wanted here, the prerequisite is the sites track exporting the source assembly — ask for it in their queue first, and do not race them for it.
- [x] A2.2 ★ Sheet agent: formula from intent, explain a formula, clean a column, answer from the data **with the cells cited**, chart from intent. *(Chart from intent cut — see STATE; a Univer chart is a plugin-owned drawing structure this build has no reader for, and writing one blind puts a broken object in somebody's workbook. Queued as A2.2b.)*
- [~] A2.2b **dropped, not deferred — chart-from-intent leaves this queue.** The loop chased it to the end and the finding stands: alo Sheets cannot hold a chart at all. `SheetEditor.tsx` registers eleven Univer presets and none is a chart; the only implementation is `@univerjs-pro/sheets-chart`, a Univer **Pro** package with no `license` field, present only as a transitive dependency and imported nowhere; and `importOffice.ts` drops charts by construction. Create, import and export are all chartless, so no fixture can exist, and writing a drawing structure inferred from documentation with nothing to check it against is the speculative write this queue forbids. **An agent cannot propose a chart into a product that has no charts** — so this was never an agent item. It is now a Sheets decision, recorded in `features.md` under alo Sheets with the three ways forward. Nothing is lost by dropping it here; leaving it `[!]` would have kept a closed queue permanently open on somebody else's decision.
- [x] A2.3 ★ Docs agent reachable from a room: draft a section, rewrite a selection, translate a document — the editor's agent mode, addressable as an agent.
- [x] A2.4 ★ Insights agent: answer from the numbers, explain a change, build a report.
- [x] A2.5 Drive agent beyond `find_file`: summarise a document, extract from an attachment, propose a move or a rename.
- [x] A2.6 Agenda agent beyond reads: find a time across several diaries, prep a meeting from its thread and attachments, reschedule.
- [x] A2.7 Tasks agent beyond `create_task`: what is on my plate, prioritise, chase an overdue owner, extract actions from a thread.
- [x] A2.8 Mail agent's answer half, explicitly: correspondence questions answered from the record — "are we in contact with X", "who last replied", "what did we promise them" — cited to the messages, never to a snippet.

## Wave A3 — orchestration, and the meeting

- [x] A3.1 ★ Ask alo orchestrates rather than owns: it routes to the product agents and runs multi-step work across them, with one approval surface, a visible plan, and a **Stop** that actually stops mid-run. *(One cut, recorded in STATE: no final synthesis turn — each step's own agent speaks its result in the room and Ask alo does not summarise them afterwards. Everything else shipped.)*
- [x] A3.2 ★ Meet, after the fact: minutes, decisions and actions into the meeting's thread, becoming tasks and events through the ordinary agent path — no second mechanism.
- [x] A3.3 The agent directory, API side: what each agent is for, what it may touch, and what it has done, per tenant.

**Not in this queue: Meet as a live in-call participant** (roadmap A3.3). It is
a media path rather than a tool set — a LiveKit participant, audio in,
transcription, a voice or a message out — and it is not decided. It waits for
its own ADR and an owner's decision, and an autonomous loop does not get to
make that call.

**Also not in this queue: every `[web]` surface.** The chat rebuild owns those
files this week. When it lands, a follow-up queue picks up the agent DM in the
room list, the agent avatar and badge, the answer-versus-proposal rendering,
and the directory screen.

---

## Done means

The implement skill's definition, with one addition that is specific to this
queue: **an agent item is not done until its question has been asked and
answered on the wire**, against the local backend, with the exchange recorded
in STATE.md. A green test proves the tool runs. It does not prove the agent
answered.

---

## Wave A4 — one command layer: intents (ADR 0057, ADR 0058)

**Read ADR 0057, ADR 0058 and `docs/design/complete-agents.md` first.** The
2026-08-28 evaluation (STATE.md) is the acceptance test of every item: run it
against a real model after each item and quote the answers.

- [x] A4.0 `IntentSpec` in `alo-ai` (`intent.rs`): name, purpose, effect,
  typed args, `answers`, `preview`, `undo`, `route`; `ToolSet` becomes a
  rendering of intents (prompt lines, tool list, `offers`, directory `tools`);
  the read budget rises to six per turn. Coverage test: every `/<module>/`
  route in `server.rs` is an intent's adapter or an `Excluded` with a reason.
- [x] A4.1 ★ Billing intents, the reference (agent half — see STATE; the route-adapter half is A4.1b): reads `open_quotes`,
  `quote_lookup`, `customer_lookup`, `unpaid_invoices`, `invoice_lookup`,
  `billing_totals`; writes `send_quote`, `issue_invoice`, `record_payment`
  beside the three existing ones, each with a preview; executors return the
  module's own record views (`document_json`, `customer_json`); routes call
  the same functions. `@billing which quotes are open?` and `@alo what did we
  quote Northstar Foods?` answer from the record, on the wire.
- [x] A4.1b Billing's routes become adapters: each `/billing/` handler calls the same executor its verb calls, so a route and an intent cannot drift; the coverage test then asserts the call, not just the name.
- [x] A4.1c Additive registration, so several loops can land modules at once:
  each `<module>_intents.rs` in `alo-jmap` exposes `dispatch(tool, account,
  args, state) -> Option<Reply>` and `agent.rs`'s central match becomes a loop
  over a list of module dispatchers; `agent_product.rs` builds each product's
  set from a list of `IntentModule`s; `lib.rs` and `server.rs` stay one line
  per module. A module then lands as new files plus one line in three lists,
  and a rebase conflict on those lines is resolved by keeping both. Tests:
  the registry and coverage tests unchanged and green; Billing still answers.
  **This item gates the agents-a/b/c tracks (LOOP-MAC.md).**
- [~] A4.2 *(moved to track agents-a, 2026-08-28 — runs on the Mac in parallel)* Sales (CRM) and Finance intents: open deals by stage, deal lookup;
  invoiced / paid / outstanding for a period, VAT summary.
- [~] A4.3 *(moved: Projects → agents-a; Drive, Docs → agents-b)* Projects, Drive and Docs intents: active projects, project lookup;
  recent files, list a folder; list documents, document lookup.
- [~] A4.4 *(moved: Inventory, HR → agents-a; Sheets, Tasks, Agenda → agents-b; Chat, Meet, Insights, Mail, Sites → agents-c)* Every remaining module's intents (Inventory, HR, Sites, Tasks,
  Agenda, Chat, Meet, Insights, Sheets, Mail), hand-written tool constants
  deleted, coverage tests green.
- [x] *(unblocked 2026-08-29: all three journals show `LOOP COMPLETE`)* A4.5 *(prerequisite: agents-a, agents-b and agents-c journals all show `LOOP COMPLETE`; if not, mark this item `[!]` with that reason and take the next item)* Provenance (`origin`) on the records the moved modules create;
  intents return it; agents cite it.
- [x] A4.6 The event stream: `events` table, every intent execution emits;
  audit reads from it.
- [x] *(unblocked 2026-08-29: all three journals show `LOOP COMPLETE`)* A4.7 *(prerequisite: agents-a, agents-b and agents-c journals all show `LOOP COMPLETE`; if not, mark this item `[!]` with that reason and take the next item)* The evaluation set grows from the intents' `answers`; the scripted
  run records answers verbatim and is the wave's exit gate.

## Wave A5 — delegation

- [x] A5.1 The `delegate` envelope: an agent hands a sub-question to another
  agent inside its run, as the asker, depth ≤ 2, ≤ 4 per run, one budget;
  the room sees the handoff line; the delegate's answer is folded in, cited.
- [x] A5.2 Ask alo's planner becomes the delegation path (one mechanism, not
  two); writes proposed by a delegate land on the asker's one approval surface.
- [x] A5.3 Isolation: a delegate reaches nothing the asker could not; a
  handle the asker cannot see is dropped; never across a shared channel.

## Wave A6 — channel memory

- [x] A6.1 `agent_memories` (migration `04xx`), the per-channel switch, the
  workspace default; learning at the end of a turn from what the turn read;
  explicit "remember that …".
- [x] A6.2 Retrieval inside scope only: a turn reads its channel's memories or
  the asker's own DM memories; the wrong-channel test is the one that matters.
- [x] A6.3 Deletion follows the source: message, channel archive, agent
  removed from the channel, switch off (30-day hide then delete).
- [x] A6.4 `[web]` **What I remember** — per agent per channel; read by every
  member, forgotten by the owner or the source author.

## Wave A7 — standing instructions

- [x] A7.1 `agent_instructions`: schedule and module-event triggers, run as
  the author on the scheduled-mail sweeper, reads post, writes propose to the
  author; bounds (one firing per hour, twenty per channel); paused when the
  author leaves.
- [x] A7.2 `[web]` The instruction card in the channel with Cancel for the
  author and the owner.

## Wave A8 — actions and goals

- [x] A8.1 The action record: every intent execution, by a person or an agent,
  leaves one row with preview, actor, on_behalf_of, result, undo; a person's
  click and an agent's proposal are the same object.
- [x] A8.2 Undo an agent with the button that undoes a person; hand an open
  proposal to an agent; assign a task to an agent (a standing instruction with
  a due date).
- [ ] A8.3 Goals: a goal record with Ask alo's plan, steps, progress, one
  approval surface and Stop; the Northstar demo across Sales, Billing, Mail
  and Agenda.
- [ ] A8.4 `[web]` The agent on the record in focus in every moved module.

## Wave A9 — exit

- [ ] A9.1 *(prerequisite: every other item here and in agents-a/b/c is `[x]` or `[~]`; otherwise `[!]` with the reason)* The full evaluation on the wire, every agent, against a real model,
  quoted in STATE.md; the six that said "I could not find it" answer from
  the record; a standing instruction fires and posts; a delegation is visible
  in a room; a channel's memory is read back and forgotten.

