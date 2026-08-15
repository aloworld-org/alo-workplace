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

## Two areas this track must NOT touch

- **`web/src/chat/**` is being rebuilt on Tailwind by another agent right
  now.** Every item here is store-and-API first. An item needing chat UI is
  marked `[web]` and stays blocked until that rebuild lands; do not open those
  files to "just add a badge".
- **`alo-ai`'s sites modules** (`sites.rs`, `site_edits.rs`,
  `site_translation.rs`) belong to the sites track. A2.1 adds a **new**
  `agent_sites.rs` beside them and reads them; if it cannot be done without
  editing them, `LOOP HALT` and say so rather than racing.

---

## Wave A1 — what makes an agent a *product* agent

- [x] A1.0 The two ADRs this wave needs, one commit, no code: **reads answer, writes propose** (narrowing ADR 0023 to writes — state plainly that a read behind an approval button was the bug, not the design), and **an agent as a DM counterpart** (a channel kind, since `dm_key` is a pair of user ids and cannot express it). Number them after the highest existing ADR; check that number again immediately before committing.
- [x] A1.1 Reads answer, writes propose. A read-only tool returns its answer into the room immediately; only a tool that changes something waits for approval. The split is a property of **the tool**, declared once in the registry, never a judgement made at the call site. Both paths audited. Test that a read never creates a proposal row and that a write never executes without one.
- [x] A1.2 A product on the agent record (migration `04xx`), and the tool registry scoped by it: an agent is offered its own product's tools in the prompt **and refused the others at the execution boundary**. The boundary test is the one that matters — a prompt that asks nicely is not a permission system.
- [x] A1.3 Product-scoped retrieval: each agent grounds in its own product's records rather than one shared workspace search. `Ask alo` keeps the workspace-wide view; it is the only agent allowed to look everywhere. Prove a Mail agent's grounding contains no Drive rows.
- [x] A1.4 One-to-one with an agent: a DM whose counterpart is an agent. New channel kind, listed beside human DMs, the agent answers there exactly as it does in a channel. API and store only — the room list rendering is `[web]` and comes after the chat rebuild.
- [x] A1.5 The default agent set: a tenant gets its agents without an admin registering handles by hand, and a module the tenant cannot open has no agent. Reuse the existing per-user module access rather than inventing a second gate; a denied module must yield no agent in any surface.
- [x] A1.6 The isolation tests, one per surface — channel, agent DM, in-module. Wrong tenant and wrong user both prove an agent reaches nothing the asker could not, including a private channel the asker is not in and a colleague's diary.
- [ ] A1.7 The two questions end to end, on the wire, against the local backend: `@mail are we in contact with ABC?` answers from correspondence with the messages behind it, and `@inventory is the X100 in stock?` answers from the stock record — **no button in between**. Record the actual request and response in STATE.md, not a claim that it worked.

## Wave A2 — the agents with no tools yet

Each item is that agent's reads **and** its writes, to the implement skill's
definition of done. An agent that can only answer is half a product. Every one
of them ends with a real question asked and answered on the wire.

- [ ] A2.1 ★ Website (Sites) agent — new `agent_sites.rs`: answer from the live site, draft and edit a page, translate the site, review SEO. Publishing is proposed, never silent. Read the sites modules; do not edit them.
- [ ] A2.2 ★ Sheet agent: formula from intent, explain a formula, clean a column, answer from the data **with the cells cited**, chart from intent.
- [ ] A2.3 ★ Docs agent reachable from a room: draft a section, rewrite a selection, translate a document — the editor's agent mode, addressable as an agent.
- [ ] A2.4 ★ Insights agent: answer from the numbers, explain a change, build a report.
- [ ] A2.5 Drive agent beyond `find_file`: summarise a document, extract from an attachment, propose a move or a rename.
- [ ] A2.6 Agenda agent beyond reads: find a time across several diaries, prep a meeting from its thread and attachments, reschedule.
- [ ] A2.7 Tasks agent beyond `create_task`: what is on my plate, prioritise, chase an overdue owner, extract actions from a thread.
- [ ] A2.8 Mail agent's answer half, explicitly: correspondence questions answered from the record — "are we in contact with X", "who last replied", "what did we promise them" — cited to the messages, never to a snippet.

## Wave A3 — orchestration, and the meeting

- [ ] A3.1 ★ Ask alo orchestrates rather than owns: it routes to the product agents and runs multi-step work across them, with one approval surface, a visible plan, and a **Stop** that actually stops mid-run.
- [ ] A3.2 ★ Meet, after the fact: minutes, decisions and actions into the meeting's thread, becoming tasks and events through the ordinary agent path — no second mechanism.
- [ ] A3.3 The agent directory, API side: what each agent is for, what it may touch, and what it has done, per tenant.

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
