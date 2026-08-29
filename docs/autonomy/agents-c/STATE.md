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
→ 85, declared reads 40 → 43; after rebasing over agents-a's Sales landing
the merged counts are 89 and 47 (kept-both on `MOVED`, `MODULES` and the
`lib.rs` lines, as the queue prescribes).

**Rebase note, and one repair beyond this item's own files.** B7.04
(`f54867ca`, one test binary per area, `autotests = false`) landed mid-item;
under it a `tests/*.rs` file not named in a suite root is silently never
built. `agent_crm_intents_http.rs` (AA.1) and `agent_delegation_http.rs`
(A9-side) had landed the old way and were orphaned — their wire gates were
green-looking but not running. Wired all three (this item's chat suite and
those two) into `agents_http_suite.rs` (`use crate::common`, one `mod` line
each) and ran them: 117/117 in the suite, alo-ai 276/276, alo-jmap lib
803/803, clippy clean. Next: AC.2 (Meet).

## AC.2 — Meet moves to intents, and learns the diary ahead (2026-08-29)

**Shipped.** `alo_ai::meet_intents` (`MEET`, six verbs) and its executors in
`alo-jmap`'s `meet_intents.rs`, registered as one row in `MOVED`, one row in
`MODULES`, one `pub mod` line in each `lib.rs`; the hand-written
`agent_meet.rs` tool set in `alo-ai` deleted (the same-named executor file in
`alo-jmap` is kept — the record of a sitting is its subject matter — and is
dispatched from the module row; its match arms in `agent.rs` removed). Reads:
`meetings_recent` and `meeting_record` kept, plus `upcoming_meetings` (the
asker's own diary ahead, `events_in_range`, default 14 days, max 60) and
`meeting_lookup` (one diary meeting by title with the invitation's notes,
place and guests, plus whether a sitting already left a record —
`meeting_for_event`). The lookup resolves a title through
`agent_meeting::resolve_meeting` made `pub(crate)` — the Agenda agent's own
resolution rule, reused rather than restated, so a lookup and a reschedule can
never disagree about which sitting a name means. Writes, each with a preview:
`meeting_minutes` kept, and `schedule_meeting` — the queue's "Agenda's intent
called as the asker" — whose executor is one line: `crate::agent::
execute_create_event` (made `pub(crate)`), so a diary entry is made in exactly
one place whichever agent proposed it, held by a structural test. Every
`/meet` route is a verb's route or an `Excluded` with a reason — 14
exclusions, most of them one reason worn many ways: nothing touches a call
while it is running (join, end, moderate, record, vote, workspace). A3.2's
"no calendar entry" doctrine is retired as the queue orders, the same way
AC.1 retired "no write at all": the objection (a second mechanism beside
Agenda's) is answered by sharing the one mechanism, previewed and approved.

**Verified.** `cargo fmt`; clippy `-p alo-ai -p alo-jmap --all-targets`
clean; nextest **1714/1714** (alo-ai 277, alo-jmap 1437), including the new
`agent_meet_intents_http` (4 tests) and `meet_intents` coverage on both
sides. Counts: `all_tools` 93 → 96, declared reads 50 → 52. Wire transcript,
as the suite pins it (scripted model, real router and store):

- `@meet what meetings do I have coming up?` → `upcoming_meetings {}` → the
  sources contained `Q3 budget review` and `Design kickoff` from the asker's
  own diary → answer in the room, `proposal: null` — a read, no button.
- `@meet when is the design kickoff?` → `meeting_lookup {"meeting":"design
  kickoff"}` → sources contained the invitation's notes `Bring the Q3
  figures`, `Room 2`, and `"record":null` (no sitting has run).
- `@meet schedule a budget review for tomorrow` → `schedule_meeting` proposed
  and the diary has NO such entry; after `POST /chat/proposals/{id}
  {"approve":true}` the event exists in the asker's personal calendar at the
  exact proposed start — made by Agenda's shared `execute_create_event`.
- **Wrong tenant:** tenant B's diary holds `warroom sync` with notes `the
  secret plan`; tenant A's `meeting_lookup` earns exactly the words an
  invented title earns (`no meeting of yours in the diary is called warroom
  sync`) and `the secret plan` appears nowhere in what A's model was shown.

**Main was broken, and this iteration repaired it before it could build.**
Four faults, all artifacts of the 08-28/08-29 cross-track rebases, none
caught because they prevented the very builds that would have caught them:
(1) `agent_product.rs` still defined `CHAT_SET`/`DRIVE_SET` from
`CHAT_TOOLS`/`DRIVE_TOOLS` and mapped `static_sets` to them, though
`agent_chat.rs`/`agent_drive.rs` were deleted when those modules moved —
alo-ai did not compile at `ce8df4ac`, so neither did alo-jmap; fixed by
finishing both moves' intent (moved modules carry no static set). (2)
`chat_intents.rs` called `answer_if_asked` with the pre-memory 4-argument
shape; the memory item added a fifth. (3) The memory item's
`/chat/channels/{id}/memory` route had neither verb nor exclusion, failing
Chat's coverage test the moment the crate could build — excluded with its
reason. (4) `agent_drive_intents_http.rs` (AB.1) was orphaned outside the
consolidated binary, the same fault ce8df4ac fixed for three other suites —
converted to a suite module (`use crate::common`) and wired in. The lesson
already in this journal stands: a gate that "passed" on a tree that cannot
compile passed nothing.

**Cuts and notes.** No migration (0450–0469 untouched), no web, no i18n, no
new route prefixes. `meeting_lookup` validates `day` itself so the refusal
names this module's argument before translating it to the shared resolver's
`on`. Mid-push, agents-a landed Finance (AA.2) plus its own fix for the same
broken main (`4c6fa402`) — kept-both on `MOVED`, `MODULES` and the CHANGELOG,
merged the counts (`all_tools` 101, declared reads 56, three copies of the
`/chat/channels/{id}/memory` exclusion deduplicated to the memory item's
own), and re-ran the gates on the merge: alo-ai 276/276, alo-jmap 1449/1449,
clippy clean. Next: AC.3 (Insights).

## AC.3 — Insights moves to intents, and learns its boards (2026-08-29)

**Shipped.** `alo_ai::insights_intents` (`INSIGHTS`, six verbs) and its
executors in `alo-jmap`'s `insights_intents.rs`, registered as one row in
`MOVED`, one row in `MODULES`, one `pub mod` line in each `lib.rs`; the
hand-written `agent_insights.rs` tool set in `alo-ai` deleted (the same-named
executor file in `alo-jmap` is kept — the figures are its subject matter —
and is dispatched from the module row; its match arms in `agent.rs`
removed). Reads: `insight_catalog`, `insight_answer`, `insight_change` kept
word for word (vocabulary looked up never remembered, figures repeated never
recomputed, a change is what moved not why), plus `dashboard_tiles` — the
boards by name with each tile's caption and the question it asks, rendered
through the route's own `dashboard_json` (made `pub(crate)`) and the shared
`asked()` rendering, via the store rather than the listing route on purpose:
`GET /insights/dashboards` seeds a first-time tenant with the Business
overview, and an agent *reading* must not leave a board behind. Writes, each
with a preview: `insight_report` kept, and `pin_chart` — one more chart on a
board that exists, its spec read through the report's own `spec_arg` gate
(made `pub(crate)`) and **evaluated before the tile is written**, the same
answered-before-saved rule the report enforces, held by a structural test on
the source order. Five exclusions with reasons (tile PATCH/DELETE/move —
changing a board colleagues read is done on the board; `tiles/{id}/data` —
the screen's rendering; the gallery — the screen's picker; `/insights/ask` —
the screen's own model turn), held by the coverage test over `/insights`.

**Verified.** `cargo fmt`; clippy `-p alo-ai -p alo-jmap --all-targets`
clean; nextest **1737/1737** (alo-ai 278, alo-jmap 1459), including the new
`agent_insights_intents_http` (3 tests, wired into `agents_http_suite`) and
`insights_intents` coverage on both sides. Counts: `all_tools` 101 → 103,
declared reads 56 → 57. Wire transcript, as the suite pins it (scripted
model, real router and store):

- `@insights what is on the sales board?` → `dashboard_tiles
  {"board":"Sales"}` → the sources contained the tile's caption `Billed
  lately`, its question's own words `billing.documents`, and
  `"tileCount":1` → answer in the room, `proposal: null` — a read, no
  button; the prompt offered all six verbs from the intent registry.
- `@insights pin billed lately to the overview` → `pin_chart` proposed and
  the board has NO tiles; after `POST /chat/proposals/{id}
  {"approve":true}` the tile is on the board over the same route the screen
  reads, caption `Billed lately`.
- **Wrong tenant:** tenant B's board `warroom figures` carries a tile
  captioned `the secret plan`; tenant A's `dashboard_tiles` earns `no board
  of yours is called warroom figures` — the words an invented name earns —
  and `the secret plan` appears nowhere in what A's model was shown.

**Cuts and notes.** No migration (0450–0469 untouched), no web, no i18n, no
new route prefixes. `insight_change` and `insight_catalog` are the module's
two deliberately routeless verbs (the catalog renders from the product's
enums, a change is two evaluations of `insight_answer`'s route), named in the
ROUTELESS list so a new verb cannot join them silently. The old
`agent_insights_http` suite (A2.4) still passes unchanged — the four kept
verbs' executors and names did not move. Mid-push, agents-b landed Docs
(AB.2, `e8658e89`) — kept-both on `MOVED`, `MODULES` and the two deleted
static sets (the conflict was two tracks each deleting their own module's
`*_SET` const; the resolution deletes both), merged the counts (`all_tools`
105, declared reads 58), and re-ran the gates on the merge: alo-ai 279/279,
alo-jmap 1465/1465, clippy clean. Mid-push again, agents-a landed Projects
(AA.3, `ee13092c`) and the room's memory panel (`eb41a261`) — kept-both on
the CHANGELOG, merged the counts once more (`all_tools` 108, declared reads
61), gates re-run green on that merge too: alo-ai 281/281, alo-jmap
1476/1476, clippy clean. Next: AC.4 (Mail with Contacts).

## AC.4 — Mail (with Contacts) moves to intents, and learns its mailbox (2026-08-29)

**Shipped.** `alo_ai::mail_intents` (`MAIL`, fifteen verbs) and its executors
in `alo-jmap`'s `mail_intents.rs`, registered as one row in `MOVED`, one row
in `MODULES`, one `pub mod` line in each `lib.rs`; the hand-written
`agent_mail.rs` and `agent_contacts.rs` tool sets in `alo-ai` deleted — one
module for one product, the address book Mail's as before. Reads:
`correspondence`, `message_read`, `find_contact` kept word for word (the
exchange over the snippet, `"opened": false` never quoted, several matches
named never resolved), plus `unread_summary` — the folders' own unread
counters, Drafts/Sent/internal roles left out, the Inbox always listed —
`thread_lookup` — one message's whole conversation off the store's own
`thread_id`, oldest first, each line saying whose side it came from — and
`who_i_emailed` — the Sent folder over `query_emails`, grouped by address,
newest first. Writes: the nine kept, each executor untouched in `agent.rs`
(made `pub(crate)`; the draft/submission/folder plumbing they reuse lives
there) and each now carrying a preview; the send preview names the subject
(filled from the resolved source) and says in so many words that it goes to
everyone the draft is addressed to and cannot be undone. Exclusions with
reasons: `/contacts/import`, `/contacts/export`, `/mail/config-v1.1.xml` —
Mail's app surface is JMAP, so fourteen verbs are ROUTELESS by declaration
and the coverage test walks the `/contacts` and `/mail` prefixes. A
structural test holds the module to one send path (`execute_send` via the
audited JMAP submission; nothing here composes-and-sends).

**Verified.** `cargo fmt`; clippy `-p alo-ai -p alo-jmap --all-targets`
clean; nextest **1768/1768** (alo-ai 285, alo-jmap 1483), including the new
`agent_mail_intents_http` (3 tests, wired into `agents_http_suite`) and
`mail_intents` coverage on both sides. Counts: `all_tools` 108 → 111,
declared reads 61 → 64. Wire transcript, as the suite pins it (scripted
model, real router and store):

- `@mail how much unread mail do I have?` → `unread_summary {}` → the
  sources contained `"totalUnread":2`, the Inbox by name, `"unread":2` —
  the same counters the mail screen's folder list shows → answer in the
  room, `proposal: null`; the prompt offered all fifteen verbs from the
  intent registry.
- `@mail draft an email to ilse@… saying the March price holds` →
  `draft_email` proposed and NO draft exists anywhere; after
  `POST /chat/proposals/{id} {"approve":true}` the draft is in the asker's
  own Drafts with the proposed subject, and Sent is still empty — a draft
  never sends itself.
- **Wrong tenant:** tenant B's inbox carries `the secret merger`; tenant
  A's `thread_lookup` on B's message id earns `no message of yours has
  that id` — the words an invented id earns — and B's subject appears
  nowhere in what A's model was shown.

**Cuts and notes.** No migration (0450–0469 untouched), no web, no i18n, no
new route prefixes. One flag for the wave review (AC.6): a mail write
proposed in a *room* stores `{"source": n}` verbatim — the palette resolves
a source number to a message id (`resolve_email_source`), the room approval
path does not, so a room-proposed `mark_read`/`send_email` still refuses at
`message_id_arg`. Pre-existing behaviour, unchanged by this item (the room
suite's write goes through `draft_email`, whose args are complete in
themselves); fixing it belongs with the wave review rather than inside a
module move. Mid-push, agents-b landed Sheets (AB.3, `6869fe30`) and
agents-a landed Inventory (AA.4, `34981ef7`) — kept-both on `MOVED`,
`MODULES`, the intent imports and the CHANGELOG; on the static sets and
match arms each side had deleted its own module's, so the resolution
deletes all of them; merged the counts (`all_tools` 117, declared reads
69) and re-ran the gates on the merge: clippy clean, alo-ai 286/286,
alo-jmap 1496/1496. Next: AC.5 (Sites).
