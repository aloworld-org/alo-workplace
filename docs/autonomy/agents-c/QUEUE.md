# alo agents — track C: the communication and insight modules (Chat, Meet, Insights, Mail, Sites)

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

Migrations for this track: **`0450`–`0469`**. Check the directory immediately
before rebasing.

## Areas this track owns

`platform/alo-ai/src/{chat,meet,insights,mail,contacts,sites}_intents.rs`, the matching `products/mail/alo-jmap/src/*_intents.rs` executors and `tests/agent_*_intents_http.rs`; the existing `agent_correspondence.rs`, `agent_meet.rs`, `agent_meeting.rs`, `agent_insights.rs`, `agent_sites.rs`, `agent_directory.rs` executors in `alo-jmap` and the same-named tool-set files in `alo-ai` (to delete). `alo-ai`'s `sites.rs`, `site_edits.rs`, `site_translation.rs` are read, never edited (sites track).

## Never touch

`web/src/**` except `web/src/chat/**` (and only for a `[web]` item);
`agent.rs`, `agent_product.rs`, `lib.rs`, `server.rs` beyond the one additive
line per shared list; any other track's `*_intents.rs`; the store modules of
another product (a store function you need and do not find is added as a
**new** function, additive).

## Queue

- [ ] AC.1 ★ **Chat**: reads — `catch_up_room`, `find_in_chat` kept, plus my rooms, unread by room, who is in a room; writes — post a message to a room the asker is in, create a room, with previews.
- [ ] AC.2 ★ **Meet**: reads — `meetings_recent`, `meeting_record` kept, plus upcoming meetings, a meeting lookup with its notes; writes — `meeting_minutes` kept, schedule a meeting (Agenda's intent called as the asker), with previews.
- [ ] AC.3 ★ **Insights**: reads — `insight_catalog`, `insight_answer`, `insight_change` kept, plus the dashboard's tiles as a list; writes — `insight_report` kept, pin a tile, with previews.
- [ ] AC.4 ★ **Mail** (with Contacts): reads — `correspondence`, `message_read`, `find_contact` kept, plus unread summary, a thread lookup, who I emailed last week; writes — the nine existing kept, with previews (a send is previewed with recipients and subject).
- [ ] AC.5 ★ **Sites**: the existing seven kept as intents with previews, plus reads — the site's pages, its published state, orders and bookings summary (`site_answer` stays the grounding).
- [ ] AC.6 Wave review, as AA.6, for these five. Then `LOOP COMPLETE`.
