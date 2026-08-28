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
