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
