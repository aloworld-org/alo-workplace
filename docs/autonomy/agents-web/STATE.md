# agents-web — journal

The loop's journal for `docs/autonomy/agents-web/QUEUE.md`: one entry per
item, newest at the bottom, in the shape `docs/autonomy/LOOP.md` prescribes.

## Opened 2026-08-29

The agents track closed with A8.4 blocked on its web writ; this track carries
that item with the writ widened to the module detail views (see the queue's
"Areas this track owns"). Backend finished; web only; Codex owns
`web/src/billing/**` and `web/src/shell/**`, which this loop never touches.

## AW.1 — `RecordAgentPanel` + the reference mount: Tasks (2026-08-29)

**What shipped.** The record's agent, on the record, in three parts (queue
rules, ADR 0057/0058, ADR 0023).

*The component.* New `web/src/agents/RecordAgentPanel.tsx`: (1) **where this
came from** — the record's origin `{kind, id, label}` said in words, one
sentence per source kind (person / thread / message / event / quote, plus a
citing fallback), a thread origin without a name read once from the room so
it can be cited by name, and a source link where the source has a screen
(the conversation, the email); (2) **what its agent can do here** — the
verbs rendered from `GET /chat/agents/directory` ∩ a per-product catalogue
(new `web/src/agents/recordAgent.ts`; tasks: chase / set priority / mark
done / hand over), each button opening the agent's one-to-one
(`POST /chat/agents/{id}/dm`) with the words pre-filled — the panel runs
nothing and says nothing itself; (3) **ask about this** — one line posted to
the agent's one-to-one with the record named, the agent's reply shown in
place with an "open the conversation" link; a turn that outlasts the
panel's patience (20 × 1.5 s) shows "no answer yet" with the same link, and
the answer still lands in the room. Quiet until asked: the only read on
open is the directory (plus the one room-name read when a thread origin
arrives unnamed). New `web/src/agents/api.ts` is the panel's own client for
the two agent routes, sharing chat's error shape — `ChatApi` was not
widened, since chat's files are not this track's writ.

*The reference mount.* `TaskDetail` mounts the panel between the field box
and Blocked-by, and derives the origin from what its record already
carries (`taskOrigin`, exported): `sourceKind` `chat`/`email`/`event`
(ADR 0021) → thread/message/event origin, else the `created` activity's
actor → "Created by …". The existing source-link chips stay (preserve
existing surfaces). *The door the buttons use:* `ChatModule` now reads
`/chat?channel=<id>&draft=<words>` — opens the room, seeds the composer,
clears the params (Mail's `?open=` pattern); its test mounts inside
`MemoryRouter` for it.

**Verified.** `tsc` clean; `eslint` on the twelve touched files clean;
`vitest` **1345/1345** (five new panel tests: origin-in-words +
offered-verbs-only, unnamed thread origin cited by room name, verb opens
the DM pre-filled and runs nothing, ask posts as the person with the
record named and shows the reply, no-origin/no-agent empty state);
`npm run build` clean. **On the wire, looked at:** throwaway stack per
`web/e2e/stack.ts`'s recipe (db `alo_e2e`, jmap :8199, vite :5199 —
dropped and killed after), signed in, created "Pricing sheet for
Northstar", opened it: the panel shows **"Created by admin@e2e.test."**,
all four verbs, the ask box (screenshot
`web/e2e/.artifacts/record-agent/06-detail.png`, local); clicking **Chase
it** landed in the @tasks one-to-one with the composer holding
`Chase "Pricing sheet for Northstar".` and nothing sent (`07-chat.png`).
No live model call anywhere.

**Decisions and flags.** (1) The panel takes `recordLabel` and `origin`
from its mount beside the queue's `{product, recordKind, recordId}`: no
route serves a stored `record_origins` row for a task — A4.5's read-side
enrichment is adopted only by billing's lookups (`task_lookup` answers
`kind:"taskLookup"`, outside the event vocabulary, so the funnel injects
nothing) and `GET /tasks/{id}` does not join it. The record's own source
fields are the origin the API does carry; the mount derives the shape, and
when a module's record view adopts the A4.5 join its mount passes the
field through unchanged. No backend added, per this queue's own rule.
(2) The ask runs through the agent's real room, not a side surface — audit,
approval and history stay where they live; the panel is where it starts
(queue: "the room is where things run"). (3) i18n keys in all four
catalogs; fr/nl/de drafted by this iteration, flagged for native review at
the wave review. (4) No new route prefixes, no deploy note. (5)
Environment: the developer 5173 was held by the owner's interactive
session (left untouched, verified still serving after teardown); the
copied same-commit `alo-jmap` binary was reused, but the copied
`identityctl` was a stale build that failed migrating a fresh database —
rebuilt here (`cargo build -p alo-identity --bins`, ~21 s warm), which is
the fix if it recurs.

**Next:** AW.2 — the five business modules (Sales `DealDrawer`, Finance,
Projects, Inventory, People/HR), five screenshots; the panel and the chat
door now exist, so each is a mount plus its module's origin derivation and
verb catalogue entry.
