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

---

## A1.1 (continued) — the gate, and why the last one could not run (2026-08-15)

**A1.1 is now `[x]`.** The code was already committed (`7d935c1c`); this entry is
the gate it was missing, the two tests it owed, and the environment fault that
stopped it.

**The halt above was wrong about its own cause. The disk was full.** `docker
version`, `docker ps` and `docker exec` do not hang because the daemon is
"wedged" for its own reasons — C: was at **100%, 3.2 MB free**, and Docker
Desktop stops answering when it cannot write. The first thing this iteration ran
after `git pull` was a build, and it failed with `rustc-LLVM ERROR: IO failure on
output stream: no space on device` and `LNK1318 Unexpected PDB error`, which is
what sent the search to `df` instead of to docker. **Check `df -h` before
concluding anything about docker.**

Where the space went, and what freed it:

- 17 GB of it was `.pdb` debug-symbol files in **this checkout's own**
  `target/debug/deps` — one per test binary, ~150 MB each, and this workspace has
  ~185 of them. Deleting them frees the space without invalidating a single
  compiled artifact (cargo does not fingerprint them; only symbolised backtraces
  are lost). That got the gate moving.
- They come straight back on the next build, and did: `alo-store`'s suite refilled
  the disk to 380 MB free, and `alo-jmap` then could not link at all. The fix that
  holds is **`CARGO_PROFILE_TEST_DEBUG=0`** — an env var, so no shared file is
  edited and no other track's build is disturbed. It applies to test targets only
  (dependencies keep `profile.dev`), so it does not force a dependency rebuild,
  and test binaries then carry no PDB at all. Every gate below ran with it.
- Two other checkouts hold build caches on the same disk (`/c/dev/Ficina/target`
  is 24 GB). **Not touched** — a loop mid-build there would break, and it is not
  this track's to clean.

**The test database.** The test Postgres on **5433** is a container and could not
be started (see above), so the gate ran against a **fresh database on the live
5432 server**: `sqlx database create` + `sqlx migrate run --source
platform/alo-store/migrations` created `alo_agents_test` and applied all 310
migrations, `0400` included, from empty. Every command below carries
`DATABASE_URL=postgres://alo:alo-dev-only@127.0.0.1:5432/alo_agents_test`. It is
its own database on its own lineage — the backend's `alo` database was not
touched. `scripts/prune-test-db.sh` cannot prune it (its first statement is a
`docker exec`); while docker is down, drop and recreate it with `sqlx` instead.

**The gate, all in the foreground.**

- `cargo fmt` clean. `SQLX_OFFLINE=true cargo clippy -p alo-jmap -p alo-store -p
  alo-ai --all-targets` — zero errors, zero new warnings (the same two
  pre-existing `type_complexity` warnings in `alo-store/src/meet.rs`, another
  track's file).
- **`cargo nextest run -p alo-store` — 1918/1918 green** (119 s), including
  `platform/alo-store/tests/agent_tool_runs.rs`, the **mandatory wrong-tenant
  test**, run on its own as well to be sure it was not filtered:
  `a_run_is_never_another_tenants_and_never_a_colleagues` and
  `a_read_leaves_a_record_even_though_nobody_approved_it`, both PASS.
- **`cargo nextest run -p alo-jmap -p alo-ai --no-fail-fast` — 1166/1167**, the
  one failure being another track's, below.

**The two tests A1.1 owed, now written and green.**

- `products/mail/alo-jmap/tests/agent_reads_answer_http.rs` — the properties on
  the wire, in a real room, with **no live model**: the tenant's AI backend is a
  scripted local socket (the shape `tests/insights_ask_http.rs` established), so
  the two-turn read loop is exercised without a single external call.
  - `a_read_answers_in_the_room_and_leaves_no_proposal`: the model asks for
    `catch_up_room`, it runs inside the turn, and the **second** call to the model
    is asserted to carry the tool's own result (`chatCatchUp`, containing the
    asker's own question) among its sources — so the answer is grounded in the
    record rather than in search hits. The agent's message arrives with
    `proposal: null`, and **every** message in the room is asserted to have none:
    a read files no `chat_proposals` row. One audit row, `catch_up_room`/`read`/ok,
    against that agent and that channel, and `agent_records().reads == 1`.
  - `a_write_executes_nothing_until_the_asker_approves_it`: the model asks for
    `create_task`; the room gets the sentence with a **pending** proposal hanging
    off it, `askedBy` the asker. At that moment the personal project holds **no
    task** and `agent_tool_runs` is **empty** — nothing ran, so nothing is logged
    as having run. `POST /chat/proposals/{id} {approve:true}` then creates exactly
    one task with the proposed title and leaves one `create_task`/`write`/ok row.
- The execution boundary itself: `execute_tool`'s condition is now the named
  `must_wait_for_approval(entry, approval)` in `agent.rs`, and
  `a_write_is_refused_from_inside_a_turn_and_a_read_never_is` checks it over
  **every tool in the registry** — refused under `Approval::InTurn` iff it is a
  write, never refused under `Approval::Asker`. This replaces the integration test
  named in the previous entry, and is a better test than that one could have been:
  `execute_tool` is `pub(crate)`, and no wire path can hand it a write under
  `InTurn` because the turn consults the registry first, so an integration test
  would have had to fake the very thing under test. The rule is now checked
  directly, over the whole list, with no database and no model.

**Flag for the sites track — a pre-existing failing test, not touched.**
`products/mail/alo-jmap/tests/site_schedule_http.rs:193`
(`a_publish_is_scheduled_moved_and_called_off`, from `78781768`) asserts
`instant(&scheduled["publishAt"]) == chosen`, where `chosen =
OffsetDateTime::now_utc() + 2 days`. Windows' clock has 100 ns resolution (7
fractional digits); Postgres `timestamptz` stores microseconds (6). The value
round-trips truncated, so the assertion fails whenever the seventh digit is
non-zero — i.e. about nine runs in ten, on this platform, regardless of database.
Observed three times with three different timestamps. It is the sites track's
file and their area, so it is reported rather than fixed.

**Cuts / flags.**

- No CHANGELOG line added: the user-visible line for this behaviour went in with
  `7d935c1c` and still describes what ships. No new routes, so nothing is owed to
  the production Caddyfile.
- The `Bash` tool's ceiling on this harness is **10 minutes**, not the 600 s the
  LOOP text assumes is generous: a cold `cargo nextest run -p alo-jmap` build
  (4 m 18 s) plus its run does not fit one call. It was not backgrounded — the
  same foreground command was re-run, and cargo's cache carried the build
  forward. That is the right move when a build is cut off at the ceiling.

**Next:** A1.2 — a product on the agent record (migration `04xx`), and the tool
registry scoped by it, refused at the execution boundary rather than in the
prompt. Check the migrations directory again immediately before committing: the
sites loop is climbing through `03xx`.

---

## A1.2 — a product on the agent record, and the boundary that means it (2026-08-15)

**Shipped and gated.** An agent is now the agent *of* something, and that is a
permission rather than a label.

**What shipped.**

- **The fact.** `chat_agents.product` (migration `0401`), `NOT NULL DEFAULT
  'workspace'`, CHECKed against the rail's own module ids plus `mail` and
  `workspace`. `AgentProduct` in `platform/alo-store/src/agent_product.rs` reads
  and writes it, and `AgentProduct::module()` maps each product to the
  `AppModule` migration 0208 already gates per user — which is what A1.5 will
  ask, rather than inventing a second permission system that can disagree with
  the first. `Mail` and `Workspace` map to none: mail cannot be denied, and
  `workspace` is not a module.
  - `workspace` is a **word**, not a NULL. A nullable column would make "nobody
    said" and "deliberately every tool" the same state, and that failure would be
    silent and wide.
  - The migration **backfills by handle**: an agent somebody named `@inventory`
    was created to be the Inventory agent, and the handle is the only evidence of
    intent the table holds. An unrecognised handle is left as `workspace` —
    exactly the reach it has today — rather than guessed at.
  - `create_agent` takes the product as a **required argument**, and
    `POST /chat/agents` requires the field: the only sensible default is
    `workspace`, and the widest agent must not be the one you get by forgetting.
- **The registry.** `platform/alo-ai/src/agent_product.rs` is one table mapping a
  product to its tool sets. Making it truthful meant splitting the old
  undifferentiated "core" list, which had mail, tasks and calendar tools in one
  const because there was one agent: `agent_mail.rs` and `agent_tasks.rs` are new,
  `create_event` moved to `agent_agenda.rs` beside the two diary reads, and the
  address book (`agent_contacts.rs`) is Mail's. Each of the eleven products owns
  its tools exactly once; `insights`, `meet` and `sites` own none yet (A2.1, A2.4,
  A3.2) and say so in their prompt rather than borrowing another product's.
  33 tools, 11 products, no tool in two.
- **The prompt.** `system_prompt_for(product)` replaces `system_prompt()`. It
  names the agent ("You are the alo Inventory agent…"), describes that product's
  tools and no others, renders the ADR 0047 read/write split from that product's
  declarations, and — for every agent but Ask alo — tells it to say which agent
  owns a question rather than answering it from a search snippet. The general
  rules lost the sentence about email `source` numbers, which moved to Mail's
  guidance: an agent with no email tool was being told how to fill in an argument
  it would never have. The folder list is likewise only rendered for a product
  that actually has `move_to_folder`.
- **The boundary — the part that matters.** `execute_tool` asks
  `alo_ai::offers(product, tool)` before anything is dispatched, and the product
  comes from **the agent's own row**, read through the caller's store, never from
  whoever built the `ToolRun`. A caller that could state its own scope is one
  refactor away from stating a wider one, and every test would still pass because
  the tools would still run. It fails closed: an agent that cannot be read runs
  nothing.
- **A refused lookup is not a button.** Product scope is deliberately *not*
  checked in `agent_turn::step`. A read belonging to another product takes the
  read path, is refused at the boundary, and the refusal is handed back to the
  model as that tool's result — so the turn ends with the agent saying which agent
  owns the question. Checking it in the turn as well would be a second copy of a
  permission rule, and would put a button on a lookup, which is the bug ADR 0047
  exists to remove.

**How verified.**

- `cargo fmt` clean. `SQLX_OFFLINE=true cargo clippy -p alo-store -p alo-ai -p
  alo-jmap --all-targets` — zero errors, zero new warnings (the same two
  pre-existing `type_complexity` warnings at `alo-store/src/meet.rs:185` and
  `:319`, another track's file, untouched).
- **`cargo nextest run -p alo-store` — 1923/1923 green** (57 s), including the new
  `platform/alo-store/tests/chat_agent_product.rs` and its **mandatory
  wrong-tenant test**, `an_agent_and_its_product_are_never_another_tenants`:
  tenant B holding tenant A's agent id gets `NotFound` — the same answer an id
  that was never issued gets, so the refusal is not an oracle — and the same
  handle in the other tenant is a different agent with its own product.
- The boundary rule itself, over the whole registry, with no database and no
  model: `an_agent_may_use_only_its_own_products_tools` in `alo-jmap/src/agent.rs`
  checks `offers(product, tool)` for every (product, tool) pair that exists —
  15 × 33 — and that an approval never widens *which* product's tools an agent
  has, only *who* may run them.
- **On the wire**, in a real room, against the local backend, with a scripted
  local socket as the model (no external call — the shape `insights_ask_http.rs`
  established), in `products/mail/alo-jmap/tests/agent_reads_answer_http.rs`:
  - `a_lookup_from_another_product_is_refused_and_the_agent_says_who_owns_it`:
    the Inventory agent's model asks for `whats_on`; the room gets the sentence
    "That's the Agenda agent's — ask @agenda what's on." with `proposal: null`;
    the model's **second** call is asserted to carry `this lookup did not run:
    whats_on is not a tool the inventory agent has`; its **first** call is
    asserted to contain `- stock_answer:` and **not** `- whats_on:`, so the
    prompt and the boundary are provably reading one registry. One audit row,
    `whats_on`, `ok = false`, and `reads == 0`.
  - `approving_another_products_change_still_runs_nothing`: the same agent's
    model asks for `create_task`; it arrives as a pending proposal as any write
    does; the asker's own tap returns **403** naming the tool and the product,
    the personal project holds **no task**, and the audit row is
    `create_task`/`write`/`ok = false`.
- **`cargo nextest run -p alo-jmap -p alo-ai --no-fail-fast` — 1178/1179** (158 s
  after a cold build), the one failure being the sites track's known
  `site_schedule_http::a_publish_is_scheduled_moved_and_called_off`, flagged in
  the previous entry: Windows' 100 ns clock against Postgres' microsecond
  `timestamptz`, so it fails about nine runs in ten regardless of the change
  under test. Re-run on its own here, it **passed** — which is the same
  behaviour, not a fix. Still their file and still reported rather than touched.
  The build did not fit one `Bash` call and the run was finished from the
  background marker form the LOOP sanctions; every assertion this item rests on
  was then re-run in the **foreground** (155 agent tests, then the four wire
  tests plus the two boundary tests, all green).

**Cuts / flags.**

- No new HTTP route prefixes, so nothing is owed to the production Caddyfile.
  `POST /chat/agents` gained a **required** field, which is a break in the
  narrowest sense — but its only caller is this repo's tests (the web app reads
  `GET /chat/agents` and never creates one), and defaulting it would hand every
  tool in the workspace to a client that forgot. Recorded here rather than
  versioned for that reason.
- `agent_json` gained `product` (additive) and `GET /chat/agents` carries it, so
  the chat rebuild can put an agent beside its module without a second mapping.
  No `web/src/chat/**` file was opened.
- The existing wire tests were re-pointed at agents whose product actually owns
  the tool they exercise (`catch_up_room` → a Chat agent, `create_task` → a Tasks
  agent) instead of an "Inventory" agent doing everything. That is the feature
  working, not a weakening: under A1.2 those two tests would otherwise have been
  asserting that scope is not enforced.
- `agent_agenda`'s `nothing_here_can_change_a_diary` became
  `nothing_but_create_event_can_change_a_diary`, naming the one write rather than
  counting to zero, so a second write slipped into that list still fails it.
- **The test database is still the one on 5432, not the container on 5433.**
  Docker remains unresponsive (`docker ps` prints nothing and returns), and C: is
  at **100%, ~4 GB free** — the same disk pressure the last entry diagnosed. The
  gate ran against `alo_agents_test` on the live 5432 server, which the previous
  iteration created; migration `0401` applied to it cleanly from `sqlx migrate
  run`. Every command carried `CARGO_PROFILE_TEST_DEBUG=0`, without which
  `alo-jmap` cannot link at all on this box.

**Next:** A1.3 — product-scoped retrieval, so each agent grounds in its own
product's records rather than one shared `workspace_search_terms`, with Ask alo
keeping the workspace-wide view. The seam it needs is already in place: `Turn`
and `AgentAsk` both carry the product, and `agent_product.rs` is the one table a
per-product retrieval belongs in.
