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

---

## A1.1 — reads answer, writes propose (2026-08-15)

**Built in full; NOT gated.** The item stays `[ ]` — see the halt below. What
exists is the whole path ADR 0047 decided, compiling clean and green on every
test that does not need a database.

**What shipped.**

- `platform/alo-ai/src/agent_tool.rs` — the effect bit. `AgentTool { name,
  effect }` with `Effect::{Read, Write}`, declared **beside the name in the same
  const list the prompt is generated from**, so a tool cannot be added to a
  product without answering the question. The eleven `*_TOOLS` consts changed
  from `&[&str]` to `&[AgentTool]`; `all_tools()` is the one list; `is_read_tool`
  asks the registry and nothing else — not the name, not the description, and
  never the model's own word for what it is doing.
- The prompt's statement of the split is **generated** from those declarations
  (`effect_block()`), replacing the hand-written "It only READS; it changes
  nothing." that sat in eleven tool descriptions. Prose and boundary now read one
  list, so they cannot drift. A test asserts the old sentence never comes back.
- `products/mail/alo-jmap/src/agent_turn.rs` — the turn as a small loop: ask, and
  if the decision is a read, run it, put its result among the numbered sources,
  ask again. **At most three read executions per turn** (ADR 0047 §2); on the
  fourth the turn answers with what it has and says so rather than turning a read
  into a button. A write comes back as a proposal exactly as before.
- The execution boundary: `execute_tool` now takes a `ToolRun` carrying
  `Approval::{InTurn, Asker}`, and **refuses a write under `InTurn`** before
  dispatching. That is what makes "reads only" true of a turn no matter what the
  model returns — it is not asked of the prompt.
- `agent_tool_runs` (migration `0400`) + `AccountStore::record_tool_run` /
  `agent_tool_runs` / `agent_read_counts`, and `AgentRecord.reads`. Both paths
  are audited, including refusals: eleven of the thirty-three tools are reads,
  and without this a third of an agent's work would leave nothing behind. A run
  is readable only by the person whose access it ran through — a tally is a leak
  too, just a slower one. The result is deliberately NOT stored; arguments are,
  exactly as `chat_proposals` already stores them.
- Both surfaces use the loop: `POST /ai/agent` (palette) and the chat turn.
  `Account` gained `Clone` so a room's turn can run off the request and still act
  through the asker's own door. `agent_json` gained `reads`.

**How verified (and what could not be).**

- `cargo fmt` clean. `SQLX_OFFLINE=true cargo clippy -p alo-store -p alo-ai
  -p alo-jmap --all-targets` — zero errors, zero new warnings (two pre-existing
  `type_complexity` warnings in `alo-store/src/meet.rs`, another track's file,
  left alone).
- `cargo nextest run -p alo-ai` — **111/111 green**, including the two new tests
  that pin the eleven reads by name and assert the prompt's split is rendered
  from the declarations.
- `cargo nextest run -p alo-jmap --lib` — **677/677 green**, including
  `agent_turn`'s: every read runs and every write waits, over every tool that
  exists rather than a sample; an unknown name is never treated as a read; a read
  still wanted at the bound becomes an answer and never a proposal.
- Migration `0400` proven to apply: its SQL was executed against the reachable
  Postgres on 5432 inside a transaction that was rolled back, checking that the
  table, indexes and CHECK are creatable, that `effect = 'peek'` is refused, that
  `read` and `write` are accepted, and that `args` defaults to `{}`. The probe
  was a throwaway test file, deleted before committing; the database was left
  untouched. This was done specifically so a new migration could not break every
  other track's suite while the test database is down.
- **NOT run, and the reason A1.1 is not ticked:** `cargo nextest run -p
  alo-store` and `-p alo-jmap` (integration). That includes the new
  `platform/alo-store/tests/agent_tool_runs.rs`, which carries the **mandatory
  wrong-tenant test** — written, never executed. Also missing and owed before
  this item can be ticked: an integration test that a read creates no
  `chat_proposals` row, and one that `execute_tool` refuses a write under
  `Approval::InTurn`. The unit level proves the decision; only the wire proves
  the boundary.

**Cuts / flags.**

- Two `#[allow(clippy::unwrap_used)]` test-module attributes in
  `platform/alo-ai/src/sites.rs` and `site_edits.rs` gained `clippy::expect_used`.
  Those are the sites track's files and this loop does not touch them, but they
  were failing `clippy --all-targets` on `main` before this item started — the
  crate's tests would not compile for anybody. Purely a lint allowance, no logic
  touched; flagged here for the sites track rather than fixed silently.
- Four alo-ai tests asserted the per-tool sentence "It only READS" that this item
  removes. They now assert the fact where it is decided (`is_read_tool`) and keep
  the claim specific to each tool ("files nothing with any tax authority",
  "reserves nothing", "tells nobody"). No coverage was dropped.
- No CHANGELOG-visible cut. No new route prefixes, so nothing is owed to the
  production Caddyfile.

**LOOP HALT: docker daemon unresponsive — the test database cannot be started.**

The test Postgres is on **5433** and its port is refused; `docker ps`, `docker
version` and `docker exec` all hang until killed (over 30 minutes of wall clock
spent confirming it, including `scripts/prune-test-db.sh`, which never printed
its first line because its first statement is a `docker exec`). The daemon is
wedged, so the container cannot be started from here. Postgres on **5432** is
alive over TCP — that is the local backend's own `alo` database, on a different
migration lineage (`Migrate(VersionMismatch(154))`), and is not the test
database; it was used only for the read-only, rolled-back migration probe above.

For whoever restarts: restart Docker Desktop, bring the test Postgres on 5433 up,
then `bash scripts/prune-test-db.sh` followed by `cargo nextest run -p alo-store
-p alo-jmap`. If that is green, add the three integration tests named above,
tick A1.1, and go on to A1.2. The code is committed rather than left in the tree,
because the wrapper discards uncommitted work and this is a complete
implementation — but it is committed **ungated**, which is why the box is
unticked.

**Next:** A1.1's gate, then A1.2 — a product on the agent record.
