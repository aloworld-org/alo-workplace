# alo agents — agents-c build journal

One entry per completed queue item: what was built, what the isolation tests
proved, and the scripted-model transcript quoted, not summarised. Started
2026-08-28 from `docs/autonomy/agents/QUEUE.md`'s split into parallel tracks
(ADR 0057, ADR 0058).

## AC.1 — Chat moves to intents (2026-08-28)

**Shipped.** `alo_ai::chat_intents` (`CHAT`, seven verbs) and its executors in
`alo-jmap`'s `chat_intents.rs`, registered as one row in `MOVED`, one row in
`MODULES`, one `pub mod` line in each `lib.rs`; the hand-written
`agent_chat.rs` tool set deleted. Reads: `my_rooms`, `unread_rooms` (both over
`channel_summaries` + `unread_mentions`, the sidebar's own view),
`room_members`, and `catch_up_room`/`find_in_chat` kept — their executors stay
in `agent_reads.rs` (shared `room_named` with Tasks) and are dispatched from
the module row; their match arms in `agent.rs` removed. Writes, each with a
preview: `post_message` (the asker's words exactly, posted in the asker's own
name via `post_message`, room notified and a mentioned agent answering exactly
as if typed) and `create_room` (public/private, asker as owner). Every `/chat/`
route in `server.rs` is a verb's route or an `Excluded` with a reason —
21 exclusions (membership, reactions, read-marking, proposals, the agent
runtime's own surface, …) — held by the coverage test.

**The old "no write at all" doctrine was deliberately retired**, as the queue
orders: `agent_chat.rs` refused a posting tool because it would have spoken
for the asker silently; under ADR 0047/0058 the write is previewed word for
word, approved by the asker, and lands in their own name with the room
watching the proposal — the objection is answered, not dodged.

**Verified.** `cargo fmt`; clippy `-p alo-ai -p alo-jmap --all-targets` clean;
nextest `alo-ai` 274/274, `alo-jmap` 1390/1390 — including the new
`agent_chat_intents_http` (5 tests) and `chat_intents::tests` coverage (3).
Wire transcript, as the suite pins it (scripted model, real router and store):

- `@chat what conversations am I in?` → model calls `my_rooms {}` → the tool
  result shown back to the model contained `release`, `ops` and the room's
  last word `the launch is Friday` → answer in the room: `You are in
  #release, #ops and #ask [1].`, `proposal: null` — a read, answered, no
  button.
- `@chat who is in #launch?` → `room_members {"room":"launch"}` → sources
  contained `"found":true`, the member's address and `owner` → `Only you are
  in #launch [1].`
- `@chat tell #general the deploy is done` → `post_message {"room":"general",
  "message":"The deploy is done."}` → the agent's message carries
  `proposal.tool = post_message` and #general has NO such message; after
  `POST /chat/proposals/{id} {"approve":true}` the message is in #general
  with `authorKind:"user"` and `author` = the asker — their words, their
  name.
- `@chat make a private room called audit` → `create_room {"name":"audit",
  "visibility":"private"}` → proposed, no `audit` room exists; on approval
  the room exists and is `private`.
- **Wrong tenant:** tenant B's `#warroom` holds `the secret plan`; tenant A's
  agent calls `catch_up_room {"room":"warroom"}` → `"found":false`, and the
  string `the secret plan` appears nowhere in what tenant A's model was
  shown. Absent, not forbidden.

**Cuts and notes.** No A4.1b-style route-adapter conversion for chat's
handlers (that item was Billing's): drift is prevented instead by the
executors returning the routes' own serializers (`summary_json`,
`member_json`, `message_json`, made `pub(crate)` in `chat.rs`, behaviour
unchanged). No migration needed (0450–0469 untouched). No web, no i18n, no
new route prefixes. Shared-list counts updated with the move: `all_tools` 80
→ 85, declared reads 40 → 43. Next: AC.2 (Meet).
