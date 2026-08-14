# ADR 0047 — Reads answer, writes propose

**Status:** accepted — **narrows [ADR 0023](0023-propose-then-approve-ai-tasks.md)
to writes**, extends [ADR 0034](0034-per-product-agents.md)
**Date:** 2026-08-14
**Context:** `platform/alo-ai/src/agent.rs`,
`products/mail/alo-jmap/src/chat_agent.rs`,
`products/mail/alo-jmap/src/agent_reads.rs`,
`platform/alo-store/src/chat_proposals.rs`, migration `0141_chat_agents.sql`

## The decision in one line

A tool that only reads runs **immediately** and its answer lands in the room; a
tool that **changes something** still waits for a tap — and which of the two a
tool is, is declared once in the registry, never judged at the call site.

## What was actually wrong

An agent has 33 tools. Eleven of them tell the model, in the system prompt, *"It
only READS; it changes nothing."*

| Product | Read-only tools |
|---|---|
| Agenda | `whats_on`, `am_i_free` |
| Chat | `catch_up_room`, `find_in_chat` |
| Contacts | `find_contact` |
| Drive | `find_file` |
| Finance | `vat_summary`, `flag_anomalies` |
| HR | `who_is_off` |
| Inventory | `stock_answer` |
| Projects | `project_status_summary` |

All eleven come back as `AgentDecision::Action`, become a row in
`chat_proposals` with `state = 'pending'`, and wait for a button.

So `@inventory is the X100 in stock?` returns a sentence describing what the
agent is *willing to look up*, and the stock figure arrives only after a tap.
That is not a safety property. It is the product declining to answer a question
it can answer, and it is the single most visible reason the agents read as a
demo rather than as colleagues.

The distinction exists eleven times **in English** and nowhere in a type.
`AgentDecision` has two variants — `Answer` and `Action` — and
`is_agent_tool()` is one flat allowlist over all 33 names. Nothing in the code
can tell `stock_answer` from `send_email`.

**ADR 0023 never asked for this.** Its decision is about *creating things*: "AI
never creates an active task", `POST /tasks/propose`, accept, reject. A read was
put behind the same button because there was one envelope and one path, not
because anyone decided a read needed approving. The button on a read is an
implementation convenience that grew into a rule.

## What we build instead

1. **The registry declares the effect.** Every tool carries `Read` or `Write`
   beside its name in the same const list the prompt is built from, so a tool
   cannot be added without answering the question. The prompt sentence "It only
   READS" stops being hand-written prose and becomes a rendering of the declared
   bit — the two can no longer disagree.
2. **A read executes inside the turn.** Its result is fed back as a source and
   the model answers from it, so the answer is cited to the record rather than
   to a search snippet. A read turn is therefore up to two model calls; a write
   turn stays one. **At most three read executions per turn** — a bound, so a
   confused or injected turn cannot spend a workspace's inference budget in a
   loop; on the fourth the agent answers with what it has and says so.
3. **A write cannot execute without an approved proposal.** The bit is enforced
   at the execution boundary, not in the prompt: the execute path refuses any
   `Write` tool that does not arrive with a pending `chat_proposals` row
   approved by the asker themselves. A prompt that asks nicely is not a
   permission system.
4. **Both paths are audited.** A read leaves a record of its own — agent, tool,
   args, asker, channel, time. Today an agent's record is counted from approved
   proposals (`AccountStore::agent_records`); if reads simply stopped creating
   proposals and left nothing behind, eleven tools' worth of activity would
   become invisible in the one surface that exists to show what an agent has
   done.
5. **Access does not change at all.** A read runs through the asker's account
   door, exactly as the retrieval that grounds the turn already does. Its blast
   radius under prompt injection is the blast radius workspace search already
   had: a hostile message can make an agent look something up that the person
   who triggered the turn could already look up, and nothing further.

## Consequences

- The read questions get answers. `@mail are we in contact with ABC?` and
  `@inventory is the X100 in stock?` come back with the record behind them and
  no button in between — the two questions Wave A1 ends on.
- **Latency moves.** A read turn is two model calls instead of one. That is the
  price of answering from the record, and it is paid where a user is already
  waiting for an agent rather than in a request path.
- **The approval surface gets quieter, and therefore starts meaning something.**
  A tap that only ever appears in front of a real change is a tap people read.
- Nothing needs migrating: proposals already created for read tools stay
  approvable and still execute; they simply stop being created.
- ADR 0023 is untouched where it was actually about creation. AI still never
  writes an active task.

## Rejected

- **Leave every tool behind a button** (the status quo) — it makes the answer
  the one thing the agent will not give you, and it teaches people to tap
  without reading, which is exactly what destroys the value of the tap on a
  write.
- **Decide read-versus-write at the call site**, or by name ("anything starting
  with `find_`") — a naming convention is not a permission boundary, and the
  first tool that breaks the pattern executes a write with no approval at all.
  The failure is silent and it is the worst one available.
- **Let the model declare the effect in its envelope** — the model is the
  untrusted party in this design. An injected turn would declare a write a read.
- **A per-tenant setting, "run read-only tools automatically"** — a setting is a
  way of not deciding, and whichever way it defaults, one of the two populations
  keeps the bug.
