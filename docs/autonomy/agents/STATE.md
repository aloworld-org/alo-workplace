# alo agents — build journal

One entry per completed queue item: what was built, what the isolation tests
proved, and — for every agent item — **the question that was asked and the
answer that came back on the wire**, quoted rather than summarised.

Started 2026-08-13 against a build where the agent framework exists, six
products have tools, and none of it is per-product: no product on the agent
record, one shared retrieval, and every tool routed through an approval button
including the reads.

**An agent that answers plausibly from a search snippet is a failure, not a
partial success.** The whole claim of a product agent is that it answers from
the record. If a tool cannot reach the record, the agent must decline and say
which agent owns the question — record that here as the intended behaviour
rather than softening the prompt until something comes out.

**The wire test is the deliverable.** A green suite proves a tool runs. It does
not prove an agent answered, and this queue has already been written on the
assumption that those are different things.

---

## A1.0 — the two ADRs the wave rests on (2026-08-14)

**Shipped.** Docs only, no code, as the item required.

- `docs/decisions/0047-reads-answer-writes-propose.md` — narrows ADR 0023 to
  writes. States plainly that a read behind an approval button was the bug and
  not the design: ADR 0023's decision is about *creating* things, and a read was
  put behind the same tap because there was one envelope and one code path, not
  because anyone decided reads needed approving. Decides the five things A1.1
  has to build: the effect (`Read`/`Write`) is declared once in the same const
  list the prompt is generated from; a read executes inside the turn and its
  result grounds the answer (up to two model calls, **at most three read
  executions per turn**); a write is refused at the **execution boundary**
  without a pending proposal approved by the asker; **both** paths are audited,
  because reads that leave nothing behind would erase eleven tools' worth of an
  agent's record; access is unchanged — a read runs on the asker's account door,
  so injection buys no more than workspace search already did.
- `docs/decisions/0048-an-agent-as-a-dm-counterpart.md` — a third channel kind
  `agent_dm`, identified by a nullable `agent_id` column plus a partial unique
  index over `(tenant_id, agent_id, created_by)`, with `dm_key` left NULL and
  keeping its single meaning of a sorted pair of *user* ids. One human member,
  one agent member, member-add refused as it already is for a DM; every human
  message is the trigger (no `@handle`); an agent's own message never triggers a
  turn, so no loops. Store and API only — the room list, avatar and badge are
  `[web]` and wait for the chat rebuild.

**How verified.** No code changed, so no gate applies beyond the facts being
true. Each claim in both ADRs was read off the tree at the commit that pulled in
`53b53f8a`, not recalled: the 33 tools counted from the eleven `*_TOOLS` consts
in `platform/alo-ai/src/`; the **eleven** read-only tools identified by the
literal "It only READS; it changes nothing." in their own `*_TOOL_DOC`
(`whats_on`, `am_i_free`, `catch_up_room`, `find_in_chat`, `find_contact`,
`find_file`, `vat_summary`, `flag_anomalies`, `who_is_off`, `stock_answer`,
`project_status_summary`); the proposal-for-everything path read in
`chat_agent.rs::take_turn` (`AgentDecision::Action` → `post_as_agent` →
`propose_action`); the DM constraints (`chat_channels_shape`, the
`(tenant_id, dm_key)` unique index, `dm_key(a,b) = "alice:bob"`) and the
"deliberately NOT a row in `users`" refusal read in the migrations themselves;
`joinable_channels`' existing `kind = 'channel'` filter confirmed as the reason
an `agent_dm` cannot leak into discovery.

**Numbering.** 0046 was the highest ADR at the time of writing; the directory
was re-checked immediately before committing. 0047 and 0048 were free.

**Cuts / flags.** None. No CHANGELOG line: an ADR changes no user-visible
behaviour — the line belongs to A1.1, which ships the behaviour.

**Next:** A1.1 — reads answer, writes propose, implementing ADR 0047.
