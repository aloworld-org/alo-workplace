# alo agents — track B: the personal work modules (Drive, Docs, Sheets, Tasks, Agenda)

**Read ADR 0057, ADR 0058, `docs/design/complete-agents.md`, then
`docs/autonomy/agents/QUEUE.md`'s "Areas and rules for waves A4–A9" — those
rules are this queue's rules too.** This track exists so that several loops
can move modules to intents at once without editing the same lines: it owns
**only** the files named below and lands each module as new files plus one
additive line in each shared list (`A4.1c` in the agents queue made that
possible; **do not start before it is `[x]` there**).

Since 2026-08-28 (A4.1c `[x]`) those lists are, exactly: one row
`(AgentProduct::<Product>, &<MODULE>_INTENTS)` in `alo_ai::agent_product::MOVED`,
one row `crate::<module>_intents::dispatch` in `alo_jmap::agent::MODULES`
(the module's `pub(crate) fn dispatch(state, account, tool, args) ->
Option<crate::agent::Dispatched>`, copied from Billing's), the `pub mod` line
in each `lib.rs`, and the routes in `server.rs`. A test on each side holds the
two lists to the same length, so a module registered in one and not the other
fails the gate. A rebase conflict on those rows is resolved by keeping both.

Every module moves the same way — copy Billing (`alo_ai::billing_intents`,
`alo-jmap`'s `billing_intents.rs`): an `IntentModule` in a new
`platform/alo-ai/src/<module>_intents.rs` (verbs with purpose, typed args,
`answers`, previews for writes, exclusions with reasons, guidance); executors
in a new `products/mail/alo-jmap/src/<module>_intents.rs` that return the
module's own record views (its routes' JSON) with readable amounts beside
integers; a scripted-model wire suite `tests/agent_<module>_intents_http.rs`
(a read answered from the record, wrong tenant, a write proposed and not run);
the hand-written `agent_<module>.rs` tool set in `alo-ai` deleted and its
executors kept or folded; the coverage test reading `server.rs` green. Reads
first: the questions a colleague would ask ("what do we have", "where are we
with X", "what is open/overdue/due"), then the writes the app already has.

Migrations for this track: **`0430`–`0449`**. Check the directory immediately
before rebasing.

## Areas this track owns

`platform/alo-ai/src/{drive,docs,sheets,tasks,agenda}_intents.rs`, the matching `products/mail/alo-jmap/src/*_intents.rs` executors and `tests/agent_*_intents_http.rs`; the existing `agent_drive.rs`, `agent_docs.rs`, `agent_sheets.rs`, `agent_tasks.rs`, `agent_agenda.rs`, `agent_attachments.rs` executors in `alo-jmap` and the same-named tool-set files in `alo-ai` (to delete).

## Never touch

`web/src/**` except `web/src/chat/**` (and only for a `[web]` item);
`agent.rs`, `agent_product.rs`, `lib.rs`, `server.rs` beyond the one additive
line per shared list; any other track's `*_intents.rs`; the store modules of
another product (a store function you need and do not find is added as a
**new** function, additive).

## Queue

- [x] AB.1 ★ **Drive**: reads — recent files, list a folder, `find_file`/`file_read`/`attachment_read` kept, what is shared with me; writes — `file_rename`, `file_move` kept, create a folder, with previews. `@drive which files do we have?` answers from the record.
- [x] AB.2 ★ **Docs**: reads — list documents (recent, by folder), `doc_read`/`doc_answer` kept; writes — `doc_draft_section`, `doc_rewrite` kept, create a document, with previews. `@docs which documents exist?` answers from the record.
- [ ] AB.3 ★ **Sheets**: reads — list spreadsheets, `sheet_read`/`sheet_answer`/`sheet_formula_explain` kept; writes — `sheet_write_formula`, `sheet_clean_column` kept, with previews.
- [ ] AB.4 ★ **Tasks**: reads — `my_plate`, `overdue_by_owner`, `thread_actions` kept, plus a board's open tasks, a task lookup; writes — the existing four kept, complete a task, reassign, with previews.
- [ ] AB.5 ★ **Agenda**: reads — `whats_on`, `am_i_free`, `find_a_time`, `meeting_prep` kept, plus an event lookup, a colleague's availability where shared; writes — `create_event`, `reschedule_event` kept, cancel, respond to an invitation, with previews.
- [ ] AB.6 Wave review, as AA.6, for these five. Then `LOOP COMPLETE`.
