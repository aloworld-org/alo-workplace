# alo agents — agents-b build journal

One entry per completed queue item: what was built, what the isolation tests
proved, and the scripted-model transcript quoted, not summarised. Started
2026-08-28 from `docs/autonomy/agents/QUEUE.md`'s split into parallel tracks
(ADR 0057, ADR 0058).

## AB.1 — Drive moves to intents (2026-08-28)

**Shipped.** Drive is the second module on the intent layer (Billing was the
first, A4.1c the same day):

- `platform/alo-ai/src/drive_intents.rs` — the `DRIVE` `IntentModule`: six
  reads (`recent_files`, `list_folder`, `shared_with_me` new; `find_file`,
  `file_read`, `attachment_read` kept) and three writes with previews
  (`create_folder` new; `file_rename`, `file_move` kept). Every `/drive/`
  route is a verb's or excluded with its reason (trash/restore/copy/versions/
  download/office/upload/Base — 16 exclusions). The hand-written
  `agent_drive.rs` tool set in `alo-ai` is deleted; registration is one row in
  `MOVED` and one in `alo-jmap`'s `MODULES`, per A4.1c.
- `products/mail/alo-jmap/src/drive_intents.rs` — shared cores (`node_list`,
  `node_record`, `create_folder`, `rename_node`, `move_node`) that the
  `/drive/` handlers now adapt (A4.1b-style, behaviour unchanged) and the
  executors run; new executors for the three new reads and the folder write;
  dispatch reaching the kept executors in `agent_drive.rs`/
  `agent_attachments.rs`. A coverage test reads `server.rs`; another asserts
  each verb's route handler calls the verb's core.
- `alo-store`: one additive function, `drive_recent(limit)` — personal,
  non-trashed, `kind <> 'folder'`, newest first, same scoping reasoning as
  `drive_find`. No migration needed (0430–0449 untouched).

**Verified.** `cargo fmt`; clippy clean on alo-store/alo-ai/alo-jmap
(all targets); `cargo nextest run`: alo-store+alo-ai 2792 passed, alo-jmap
1390 passed (one pre-existing count test, `agent_turn`'s 40 reads, updated to
43 for the three new reads). Wire suite
`tests/agent_drive_intents_http.rs`, 4/4 green:

- *Read from the record*: scripted model asks `recent_files` for
  "@drive which files do we have?" → answer lands in the room
  ("Two files: the handover note and the price list [1]."), sources contain
  `driveRecentFiles` with `Handover note.md` and `Price list.csv`.
- *Wrong tenant*: a second tenant's "Their secret strategy.md" and a
  colleague's private "Bens private appraisal.md" are seeded and asserted
  ABSENT from the model's sources; `shared_with_me` likewise never names
  another tenant's Space ("Their secret space").
- *Write proposed, not run*: "@drive make a Contracts folder" →
  `proposal.tool == "create_folder"`, and `/drive/list` shows no such folder
  until a tap.
- *Refusals by name*: a duplicate folder is refused ("there is already a
  Contracts…"), an absent destination lists the folders that exist
  ("— you have: Contracts"), and `recent_files` lists files only.

**Flags.** `alo_scratch` (this checkout's test database) did not exist on this
machine; created empty, suites migrate it themselves. The full alo-jmap
test-binary relink after an alo-store change took ~85 min on this Mac — the
background+marker exception in LOOP.md step 5 was used for the build only,
tests ran foreground. Next: AB.2 (Docs).

## Repair — main did not compile (2026-08-29)

Found at the top of the iteration: a keep-both rebase between AB.1's Drive
commit (8c2987cd) and agents-c's Chat commit (af6ace90) had resurrected the
deleted `CHAT_SET`/`DRIVE_SET` lines in `agent_product.rs` (referencing
constants neither crate defines any more), left `chat_intents.rs` calling
`answer_if_asked` with the arity from before channel memory (38d2506e), and
left the new `/chat/channels/{id}/memory` route with no row in Chat's
coverage. Restored each commit's stated intent — a moved module's static set
is empty, the call passes the message id, the memory route is excluded with
its reason — and pushed as `4c6fa402` before starting the queue item, since
every track gates on a compiling main.

## AB.2 — Docs moves to intents (2026-08-29)

**Shipped.** Docs is the third module of this track on the intent layer:

- `platform/alo-ai/src/docs_intents.rs` — the `DOCS` `IntentModule`: three
  reads (`list_documents` new — recent, and by folder; `doc_read`,
  `doc_answer` kept) and three writes with previews (`create_document` new;
  `doc_draft_section`, `doc_rewrite` kept). The hand-written `agent_docs.rs`
  tool set is deleted; registration is one row in `MOVED` and one in
  `alo-jmap`'s `MODULES`, per A4.1c. The old set's rules survive as purpose
  sentences and tests: cited blocks, translation-is-the-rewrite, nothing
  deletes, moving/renaming stays the Drive agent's.
- **No `/docs/` route is adapted, by design**: the agent's documents are
  Drive nodes (kind `doc`, the editor's block-array blob), while `/docs`
  serves the standalone ADR 0015 technical-authoring surface — a different
  record. Both `/docs` routes are excluded with reasons and the coverage
  test holds every route accounted for and every exclusion a real route.
- `products/mail/alo-jmap/src/docs_intents.rs` — executors for the two new
  verbs over the store paths the editor itself uses (`drive_docs`,
  `drive_list` + kind filter, `put_blob` `[]` + `drive_create_file` kind
  `doc`); dispatch reaching the four kept executors in `agent_docs.rs`.
  Duplicate titles refused by name, absent folders refused with the folders
  that exist. No migration (0430–0449 untouched), no new store function.

**Verified.** `cargo fmt`; clippy clean on alo-ai + alo-jmap, all targets;
nextest: alo-ai 277, alo-jmap 1435 — all green. Wire suite
`tests/agent_docs_intents_http.rs`, 3/3:

- *Read from the record*: "@docs which documents exist?" → `list_documents`
  → "Two documents: the handover and the terms of engagement [1]." lands in
  the room; sources contain `docsList` with both names; the prompt offers
  all six verbs and no other product's.
- *Wrong tenant*: a second tenant's "Their secret plan" and colleague Ben's
  private "Bens private notes" are seeded and asserted ABSENT from the
  model's sources.
- *Write proposed, not run*: "@docs start a document called Handover" →
  `proposal.tool == "create_document"`, and `drive_docs` shows no such
  document until a tap. Verb tests over the approval route: duplicate
  refused ("there is already a Handover…"), a folder that is not there
  refused with "you have: Contracts", by-folder listing exact, and the
  created document opens as an empty `docRead`.

**Also repaired in passing:** AB.1's `agent_drive_intents_http.rs` was
orphaned — written before B7.04's suite consolidation, never adopted, so
with `autotests = false` it was silently not building or running. Adopted
into `agents_http_suite` (`use crate::common`, one mod line), as the
2026-08-28 addendum in the business STATE orders; the consolidated suite now
runs 134/134 with both intents suites inside. Counts bumped with the two new
verbs: reads 50→51, all tools 93→95.

Next: AB.3 (Sheets).

## AB.3 — Sheets moves to intents (2026-08-29)

**Shipped.** Sheets is the fourth module of this track on the intent layer:

- `platform/alo-ai/src/sheets_intents.rs` — the `SHEETS` `IntentModule`: four
  reads (`list_spreadsheets` new — recent, and by folder; `sheet_read`,
  `sheet_answer`, `sheet_formula_explain` kept) and two writes with previews
  (`sheet_write_formula`, `sheet_clean_column` kept). The hand-written
  `agent_sheets.rs` tool set is deleted; registration is one row in `MOVED`
  and one in `alo-jmap`'s `MODULES`, per A4.1c. The old set's five rules
  survive as purpose sentences and tests: every figure cited to its cell, no
  arithmetic in the model's head, a formula written and a fact never, tidying
  about typing not meaning, both writes wait for a tap.
- **No route is adapted, because there is none**: alo Sheets has no route
  surface of its own — a workbook is a Drive node of kind `sheet` and the
  editor saves through Drive's routes. The exclusion list is empty and the
  coverage test holds the router to registering no `/sheets` route at all, so
  the empty list stays honest.
- `products/mail/alo-jmap/src/sheets_intents.rs` — the executor for the one
  new verb over the store path the resolver already used (`drive_sheets`,
  `drive_list` + kind filter); dispatch reaching the five kept executors in
  `agent_sheets.rs` (which stays, its doc now pointing at the intent module).
  No migration (0430–0449 untouched), no new store function.

**Verified.** `cargo fmt`; clippy clean on alo-ai + alo-jmap, all targets;
nextest: alo-ai 277, alo-jmap 1464 — all green (counts bumped: registry reads
57→58, all tools 103→104). Wire suite `tests/agent_sheets_intents_http.rs`,
3/3, adopted into `agents_http_suite`:

- *Read from the record*: "@sheets which spreadsheets do we have?" →
  `list_spreadsheets` → "Two spreadsheets: the Q1 figures and the price list
  [1]." lands in the room; sources contain `sheetsList` with both names; the
  prompt offers all six verbs and no other product's.
- *Wrong tenant*: a second tenant's "Their secret figures" and colleague
  Ben's private "Bens private numbers" are seeded and asserted ABSENT from
  the model's sources.
- *Write proposed, not run*: "@sheets add up the amounts column" →
  `proposal.tool == "sheet_write_formula"`, and the stored workbook blob
  still holds no formula until a tap. Verb tests over the approval route:
  the listing holds workbooks only (a `doc` node is asserted absent), a
  folder narrows it by name, a folder nobody has is refused with the folders
  that exist, and a listed workbook opens by name through the kept
  `sheet_read`.

Next: AB.4 (Tasks).

## AB.4 — Tasks moves to intents (2026-08-29)

**Shipped.** Tasks is the fifth module of this track on the intent layer:

- `platform/alo-ai/src/tasks_intents.rs` — the `TASKS` `IntentModule`: five
  reads (`board_tasks` — one board's open work in board order — and
  `task_lookup` — one task in full, finished work included — new; `my_plate`,
  `overdue_by_owner`, `thread_actions` kept) and six writes with previews
  (`complete_task` and `reassign_task` new; `create_task`,
  `set_task_priority`, `chase_task`, `capture_actions` kept). The old set's
  rules survive as purpose sentences and tests: the plate includes the
  undated, the reach is the boards the asker can open, a chase is the asker's
  own comment, a capture is proposed twice (ADR 0023), a task is named never
  identified — and the two new writes carry their own: completing is the
  user's word ("propose it only when the user SAID the work is finished"),
  a handover changes the owner and nothing else. The hand-written
  `agent_tasks.rs` tool set is deleted; registration is one row in `MOVED`
  and one in `alo-jmap`'s `MODULES`, per A4.1c. Every `/tasks` route is a
  verb's or excluded with its reason (18 exclusions: board management,
  labels, subtasks, followers, attachments, dependencies, and the
  accept/reject taps ADR 0023 reserves for the person).
- `products/mail/alo-jmap/src/tasks_intents.rs` — executors for the four new
  verbs over the same store paths the board runs (`tasks_in_project`,
  `move_task` — which sets `completed_at` — and `update_task` carrying every
  other field across unchanged); the reads answer with the routes' own record
  views (`tasks::task_json`; `task_lookup` returns the full `GET /tasks/{id}`
  record through a `task_record` core extracted A4.1b-style, the handler
  unchanged in behaviour). A colleague for a handover resolves by exact email
  or by first name **among the people already on the visible boards** — never
  a directory, so a name that matches nobody says nothing about who exists.
  Dispatch reaches the six kept executors in `agent_tasks.rs`;
  `execute_create_task` moved home there from `agent.rs` (same behaviour,
  `parse_due` now `pub(crate)`). No migration (0430–0449 untouched), no new
  store function, no new route.

**Verified.** `cargo fmt`; clippy clean on alo-ai + alo-jmap, all targets;
nextest: alo-ai 282, alo-jmap 1488, alo-store 2533 — all green (counts
bumped: registry reads 62→64, all tools 109→113). Wire suite
`tests/agent_tasks_intents_http.rs`, 3/3, adopted into `agents_http_suite`:

- *Read from the record*: "@tasks what is open on the Launch board?" →
  `board_tasks` → "Two tasks are open: the venue and the invitations [1]."
  lands in the room; sources contain `boardTasks` with both open tasks and
  not the finished one; the prompt offers all eleven verbs and no other
  product's.
- *Wrong tenant*: a second tenant's "Their secret rollout" and colleague
  Ben's private "Bens private errand" are seeded and asserted ABSENT from
  the model's sources; asking for the stranger's board by name gets the same
  "no board of yours" as a board that does not exist.
- *Write proposed, not run*: "@tasks the venue is booked — mark it done" →
  `proposal.tool == "complete_task"`, and the stored task is still not done
  until a tap. Verb tests over the approval route: the board read is exact
  and refuses an unknown board; the lookup opens finished work and treats
  two matches as a question; completing sets `completed_at` through the
  board's own move and refuses a task already done; a handover by email and
  then by first name lands on Ben with priority untouched, and "Zelda" is
  refused with "no colleague on your boards".

**Rebase at the push.** While AB.4 gated, agents-c landed Mail's move to
intents plus standing instructions, and Inventory's move was already on main
— so the first push was rejected and the rebase conflicted exactly where
A4.1c predicts: the set constants (both sides deleting different rows — kept
neither), and three counts, resolved by adding both sides' deltas rather
than picking one (workspace tools 117+4 = 121, registry reads 69+2 = 71).
Full clippy + nextest re-run on the rebased tree before the push: alo-ai
287, alo-jmap 1509, all green — the suite counts in the entry above describe
the pre-rebase tree.

Next: AB.5 (Agenda).

## AB.5 — Agenda moves to intents (2026-08-29)

**Shipped.** Agenda is the sixth module of this track on the intent layer:

- `platform/alo-ai/src/agenda_intents.rs` — the `AGENDA` `IntentModule`: six
  reads (`event_lookup` — one meeting in full, replies included — and
  `colleague_free` — one shared diary's span — new; `whats_on`, `am_i_free`,
  `find_a_time`, `meeting_prep` kept) and four writes with previews
  (`cancel_event` and `respond_to_invitation` new; `create_event`,
  `reschedule_event` kept). The old set's rules survive as purpose sentences
  and tests: a day is a date never a phrase, an unreadable diary is never
  free, a meeting is named and a day disambiguates it, a move keeps the
  length — and the two new writes carry their own: cancelling is the user's
  word ("only when the user SAID to cancel", guests told, no undo), an
  invitation is answered with the answer the user gave, never chosen. The
  hand-written `agent_agenda.rs` tool set is deleted; registration is one row
  in `MOVED` and one in `alo-jmap`'s `MODULES`, per A4.1c. Every `/calendar`
  route is a verb's or excluded with its reason (7 exclusions: the two
  mail-side iMIP taps, diary management, sharing grants, the share dialog's
  group list, and the tenant-wide freebusy row — which is NOT the agent's way
  around the shared-diaries-only reach; `colleague_free` was deliberately
  built on `find_a_time`'s shared-diary resolution instead, because the queue
  says "where shared").
- `products/mail/alo-jmap/src/agenda_intents.rs` — executors for the four new
  verbs over cores extracted A4.1b-style, handlers unchanged in behaviour:
  `calendar::event_json` (the route's record view) now `pub(crate)` for the
  lookup; `calendar::cancel_core` (the DELETE handler's body — occurrence
  `EXDATE` + one-instance `CANCEL` mails, or whole-series delete +
  cancellations) shared by route and executor; `calendar::rsvp_core` (the
  RSVP handler's body — REQUEST parse, personal-calendar upsert unless
  declined, `METHOD:REPLY` to the organizer) likewise, the handler now
  loading the blob and calling it. `respond_to_invitation` finds the
  invitation in the asker's own mail by title (the same account-scoped
  `workspace_search` sweep `meeting_prep` reads, REQUEST parts only, deduped
  by UID; several distinct matches are a question listing them).
  `colleague_free` resolves the colleague through
  `agent_agenda::shared_diaries` (find_a_time's diary-gathering extracted as
  a helper, behaviour unchanged) and the shared `resolve_person`, so an
  unshared diary and a stranger get the identical sentence. `cancel_event`
  checks `can_edit_calendar` first and refuses a read-only diary by name.
  Dispatch reaches the six kept executors where they live (`agent_reads`,
  `agent_agenda`, `agent_meeting`, and `agent.rs`'s `execute_create_event`,
  which stays put because Meet's `schedule_meeting` runs that same shared
  write); the dead Agenda match arms in `agent.rs` are removed. No migration
  (0430–0449 untouched), no new store function, no new route.

**Verified.** `cargo fmt`; clippy clean on alo-ai + alo-jmap, all targets;
nextest: alo-ai 289 + alo-jmap 1516 in one run, 1805/1805 green (counts
bumped: registry reads 71→73, all tools 121→125). Wire suite
`tests/agent_agenda_intents_http.rs`, 4/4, adopted into `agents_http_suite`:

- *Read from the record*: "@agenda when is the Board review?" →
  `event_lookup` → "The Board review is in Room 2, and Paula has accepted
  [1]." lands in the room; sources contain `eventLookup` with the route's own
  record (location, ACCEPTED reply); the prompt offers all ten verbs and no
  other product's.
- *Wrong tenant*: a second tenant's "Their secret ceremony" and colleague
  Ben's unshared "Bens private appraisal" are seeded and asserted ABSENT
  from the model's sources.
- *Write proposed, not run*: "@agenda cancel the vendor demo" →
  `proposal.tool == "cancel_event"`, and the meeting is still in the diary
  until a tap. Verb tests over the approval route: the lookup is exact,
  treats two sittings as a question and a day settles it; `colleague_free`
  names Ben's clash, says free when free, and refuses Marta's unshared diary
  with the same sentence a non-existent person gets; a one-off cancel removes
  the event (scope "series", guests named), a series cancel skips one sitting
  and next week's survives, a viewer-only shared diary is refused ("read but
  not change"); accepting an invitation lands the organizer's UID in the
  diary, declining does not, and a made-up answer or an invitation nobody
  received is a 422 by name.

**Rebase at the push.** While AB.5 gated, another loop landed HR's and
Sites' moves plus a wave review — so the first push was rejected and the
rebase conflicted exactly where A4.1c predicts: the static-set imports and
constants (both sides deleting different rows — kept neither, and with the
last static set gone the now-unused `set()` helper and the dead per-tool
match in `alo-jmap`'s `dispatch` went too), and three counts, resolved by
adding both sides' deltas (workspace tools 121+9+4 = 134, registry reads
71+8+2 = 81). One upstream test flipped meaning: `agent_plan`'s roster test
used Agenda as its example of "a product still on hand-written tools" — no
such product exists any more, so it now asserts Agenda's line carries its
hints. Full clippy + nextest re-run on the rebased tree before the push:
1825/1825 green — the suite counts in the entry above describe the
pre-rebase tree.

**Flags.** `alo_scratch_b` (this checkout's DATABASE_URL database) did not
exist on this machine; created empty, suites migrate it themselves — same as
AB.1's flag, new name. One test-only correction during the gate:
`create_event` deliberately stores an empty reply map, so the suite records
Paula's ACCEPTED through `set_attendee_status` (the inbound-REPLY path)
rather than seeding it on the event row.

Next: AB.6 (wave review).

## 2026-08-29 — AB.6 Wave review

**Reviewed, four checks, all pass.** (1) *Suites answer the `answers`
questions*: each of the five modules carries its verb-coverage module test
(every verb: route or named exception, sentence purpose, non-empty
`answers`, preview on every write — named per the module's shape:
`every_verb_has_a_route_a_purpose_and_a_question_it_answers` in Drive,
`…and_no_authoring_route` / `…and_no_route` in Docs/Sheets whose records are
Drive nodes, `…a_write_its_preview` in Tasks/Agenda), and the five wire
suites ask the queue's flagship questions against the scripted model —
17 tests total (`agent_drive_intents_http` 4, docs/sheets/tasks 3 each,
agenda 4), each suite covering a read answered from the record, a write
proposed and not run, and a wrong-tenant denial (seeded and asserted absent
inside the read tests), all listed by name in `agents_http_suite`.
(2) *Hand-written tool sets gone*: `alo_ai::agent_{drive,docs,sheets,tasks,
agenda}` no longer exist — only the infrastructure files (`agent_tool`,
`agent_product`, `agent_plan`, `agent_memory`) and intent modules remain in
`alo-ai`; the kept jmap executor files are reached only from the new
dispatches. All five rows sit in `MOVED` and `MODULES` (16 modules moved
across the tracks, the two lists held to one length by test).
(3) `complete-agents.md`'s **Moved modules** section now lists the five with
their verbs, appended additively under AA.6's business five, each line
carrying the rule its move preserved (figures cited to cells, completing is
the user's word, diaries only where shared). (4) `CHANGELOG.md` opens with
the wave line: what a user can now ask the five personal work agents, and
that every change previews first.

**Verified.** Pruned `alo_scratch_b` (1529 tenants, 37 MB — healthy);
nextest green on the reviewed tree: alo-ai + alo-jmap 1825/1825 in one run
(309 s), the 17 wire tests above confirmed present by `cargo nextest list`.
Docs-only diff — no Rust or web code changed by the review itself, no
migration (0430–0449 never used by this track), no new route prefixes, no
UI strings.

**Rebase at the push.** AC.6 (the communication wave review) landed while
this review gated, so the first push was rejected and the rebase conflicted
on the two deliberately-shared docs — the Moved modules list and the
changelog's top — both purely additive, both resolved by keeping both
sides (their five lines and wave line, then ours). This review's diff is
docs-only, so the merged tree needed no re-gate; the code AC.6 carried was
gated by its own track.

Queue complete: AB.1–AB.6 all `[x]`.

LOOP COMPLETE
