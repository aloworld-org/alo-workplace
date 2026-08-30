# alo agents — the agent on the record in focus (web)

The agents track (`docs/autonomy/agents/QUEUE.md`) closed on 2026-08-29 with
one item it could not reach: **A8.4**, the agent on the record in focus in
every moved module. Its web writ was `web/src/chat/**` and `web/src/agents/**`,
and the record-in-focus surfaces are the module detail views. This track is
that item, with the writ widened to exactly those surfaces, and it runs on the
Mac with nobody else in these directories.

**Read first, in this order:** ADR 0057, ADR 0058, `docs/design/complete-agents.md`,
then the agents journal (`docs/autonomy/agents/STATE.md`) entries on the
event stream and **record origins** (`record_origins`, migration `0472`; every
intent result carries `result.origin`; the record views join it in — "a record
remembers where it came from, and its agent says so"), `A6.4` (the
`AgentMemoryPanel` — the house style for an agent panel), and `A8.1`–`A8.3`
(actions, undo, goals).
The backend is finished: origin on records, intents returning it, the action
record, undo, goals. **This track adds no backend** — if a screen needs a
field the API does not carry, that is a `[!]` with the reason, not a new route.

## Who is where right now (2026-08-29, read before any item)

- Codex (a separate editor, own checkout) owns **`web/src/billing/**` and
  `web/src/shell/**`**. Never touch them — Billing's record panel is the owner's
  item (AW.5), not this loop's.
- The ds track is finished; the design system is `web/src/ds/**` and is
  read-only here — use its components, never add a `.module.css` (ADR 0046:
  Tailwind utilities from the tokens; `primitives.test.ts` enforces it).
- The Windows agents loop is finished. Nothing else edits `web/src/agents/**`.
- `web/src/sites/**` is normally ADR 0045's sites-loop territory; that loop is
  finished, and AW.4 may touch only the Sites detail views it names.

## Areas this track owns

`web/src/agents/**` (new components), and **only the detail/drawer/editor
views** of: `web/src/tasks`, `crm`, `finance`, `projects`, `inventory`, `hr`,
`drive`, `docs` (inside `drive/` — `DocEditor`, `BaseEditor`, `OfficeEditor`),
`sheets` (`SheetRibbon`/`SheetEditor` in `drive/`), `agenda`, `chat`, `meet`,
`insights`, `mail`, `contacts`, `sites`. The module's list/board views are out
of scope — the record in focus is what the panel is about. `web/src/i18n/**`
for strings only (additive keys, every language file the repo has). No Rust.
No migrations.

## Rules

- **What the panel is.** The record's agent, on the record. One component,
  `RecordAgentPanel` in `web/src/agents/`, that takes `{product, recordKind,
  recordId}` and shows: (1) **where this came from** — the record's origin as
  A4.7 returns it (a person, an agent's run, an import), said in words; (2)
  **what its agent can do here** — the product's verbs that take this record
  (from the registry the API exposes: `GET /chat/agents/directory` carries each
  agent's tools), each a button that opens the agent's room with the verb and
  the record pre-filled as a proposal, never running anything itself (ADR 0023:
  propose, then the one approval button); (3) **ask about this** — a one-line
  ask that goes to the product's agent with the record as context, answered in
  place, with the source link back to the record. Nothing in it is a second
  implementation of a room: the room is where things run; the panel is where
  they start.
- **Every module, the same panel.** A module adds one mount point in its detail
  view and passes the record's identity. If a module's detail view needs
  restructuring to have a place for it, that restructuring is the item's work —
  a panel hidden behind a tab nobody opens is a stub.
- **Only when asked (ADR 0057).** The panel is quiet until the person acts:
  no auto-generated summaries, no unsolicited suggestions, no calls made on
  open except the two reads that render it (origin, verbs).
- **Strings** are i18n keys from day one; nothing hardcoded in English.
- **Verify by looking.** Every item ends with one screenshot per module of a
  real record's detail view with the panel open, read and reported — run
  `cd web && npm run dev` against a local backend (LOOP-MAC.md §1) and
  Playwright (`npx playwright install chromium` once). `tsc`, `eslint` and
  `vitest` green; a component test for the panel with the three states
  (origin known / verbs offered / answer shown) and the empty state (a record
  with no origin says so, and offers nothing it cannot do).
- **Do not touch** the agents track's queue except in AW.6, where A8.4 is
  flipped to `[x]` with a pointer here.

## Wave AW — the panel, then every module

- [x] AW.1 `[web]` `RecordAgentPanel` + the reference mount: **Tasks**. The
  component with its three parts and its tests; mounted in the task detail
  (the task's origin — "captured from the Friday room by @tasks", "created by
  Disan" — its verbs — chase, set priority — and ask). Screenshot of a real
  task with the panel.
- [x] AW.2 `[web]` The five business modules: **Sales** (`DealDrawer`),
  **Finance** (the expense/approval/bank detail), **Projects** (the project
  and the timesheet), **Inventory** (product, purchase order, sales order
  editors), **People/HR** (applicant drawer, leave, directory record). Five
  screenshots.
- [x] AW.3 `[web]` The personal work modules: **Drive** (the node's detail —
  origin of a file an agent created), **Docs**, **Sheets** (the editor's
  panel/side area), **Agenda** (`DayPanel`/the event view). Four screenshots.
- [x] AW.4 `[web]` The communication and insight modules: **Chat** (a room's
  panel — the room's agents and what they remember is A6.4; this is the
  record-of-the-room, so mount beside `AgentMemoryPanel`), **Meet** (a
  meeting's record), **Insights** (a board/chart), **Mail** (a message's view
  — origin of a draft an agent wrote), **Contacts**, **Sites** (a site's
  record). Six screenshots.
- [~] AW.5 **Billing** — *(was the owner's item while `web/src/billing/**` was
  Codex's; released to this track as AW.7 on 2026-08-30 — see the wave below)*.
- [x] AW.6 Wave review: every moved module's record shows its agent, one
  browser walk at desktop and phone width with screenshots in STATE.md; the
  strings are in every language file; `A8.4` in `docs/autonomy/agents/QUEUE.md`
  becomes `[x]` with a pointer to this journal, and a note that `A9.1` (the
  full real-model evaluation) is the owner's to run. Then `LOOP COMPLETE`.

## Wave 2 — Billing, released by the owner (2026-08-30)

AW.5 was deferred because `web/src/billing/**` belongs to **Codex, a separate
editor with its own checkout** — the one directory this track was told never to
enter. The owner has released it for this item only. That release is a loan,
not a transfer, and it comes with the only rule that matters when two editors
share a directory:

**Before the first edit, and again before the commit, check that Codex has not
moved.** `git log origin/main --since='6 hours ago' -- web/src/billing/` — if a
commit appears there that this iteration did not make, **stop**: mark AW.7
`[!]` with the commit named in STATE.md and write `LOOP HALT`. Do not rebase
over it and carry on, do not resolve a conflict in a billing file by choosing a
side. A collision here costs a person's uncommitted work, which no amount of
this track's progress is worth. The same applies if the rebase before the push
brings in any billing change: halt, do not merge.

**Writ for this item only:** `web/src/billing/**` for the panel's mount points
and whatever restructuring the mount honestly needs — nothing else in there.
`web/src/shell/**` and `web/src/ds/**` stay forbidden, as always.

- [ ] AW.7 **Billing** — the same panel, the same way, in the two surfaces the
  original item named: the **document editor** (an invoice or quotation in
  focus: where it came from — drafted by a person, raised by a schedule, an
  import — its verbs, and an ask) and the **customer view** (the customer as
  the record). Mount `RecordAgentPanel`; do not reimplement it, do not fork it
  for Billing's layout — if it genuinely cannot express what these two screens
  need, widen the component in `web/src/agents/` and say so in its file, which
  is the rule every other module in this queue followed. Billing's money is
  read, never computed here: no total, no VAT, no due date is recalculated in
  the panel — it shows what the record already says (ADR 0011: money is the
  ledger's, not a view's).
  **Done when:** one screenshot of each surface with the panel open, read and
  reported; the origin line says a true thing about a real document (checked
  against the record, not the fixture); every string an i18n key in every
  language file; `npx vitest run src` green; `npx tsc --noEmit` clean; no new
  `.module.css` (ADR 0046); the Codex check above run and its result written in
  STATE.md **even when nothing had moved** — a check whose result is never
  recorded is a check nobody can trust.
- [ ] AW.8 Wave check: `web/src/billing/**` shows the panel in both surfaces;
  `AW.5` is marked `[x]` with a pointer to AW.7; the sixteen-page walk still
  passes; then `LOOP COMPLETE`.
