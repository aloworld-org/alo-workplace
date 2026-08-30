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

## AW.2 — the five business modules (2026-08-29)

**What shipped.** The panel in every business module's record in focus, ten
mounts across the five modules, each with its origin derivation and its
product's verbs.

*The catalogue grows a kind.* `RecordVerb` gains optional `kinds` and the
panel filters through new `verbsFor(product, recordKind)` — a product whose
detail views show more than one record kind (an expense is approved, a bank
line is categorised) offers each verb only where it takes the record. New
entries: crm (move stage / draft follow-up), finance (`approve_expense` on
kind `approval` — the approver's queue, named by merchant exactly as the
verb's arg asks; `categorise_transactions` on kind `expense`, whose draft
names no record because the verb reads the asker's own uncategorised
claims), projects (status summary + log time on `project`, calendar draft
on `timesheet`), inventory (`receive_delivery` on `purchaseOrder`), hr
(`approve_leave_request` on `leave` named by the employee as the verb's arg
asks; `draft_letter_from_template` on `applicant` and `person`).

*The mounts.* **Sales:** `DealDrawer`, between LinkedThreads and history;
origin = the deal's own `source` words ("From Referral."). **Finance,
three:** `ExpenseDialog` (editing an existing claim) via a new `aside` slot
on finance's `DialogFrame` — rendered as the form's sibling because the
panel carries its own ask `<form>` and HTML forbids nesting; the approvals
queue, where the person cell now toggles the waiting claim in focus
(aria-current row, panel under the table, origin = the claimant by email);
the Bank tab, where the period cell toggles the statement in focus (origin
= "Imported from a CAMT file.", the reader from the record's own `source`).
**Projects, two:** `ProjectOverviewView`'s sidebar (top), and the
timesheet's `WeekView` above the week foot, the week's Monday standing in
as id for a never-submitted week. **Inventory, two:** both order editors,
above `RecordHistory`, only once the record exists. **People, three:**
`ApplicantDrawer` (origin = the application's `source` words — "From
LinkedIn."), `LeaveView`'s marked row (`?request=`, which the approvals
inbox already sets; the kind cell now toggles the same mark), and the
directory's person-in-focus (`?person=`, the org-chart highlight).

**Verified.** `tsc` clean; `eslint` on the twenty-two touched files clean;
`vitest` **1349/1349** (two new panel tests: a kind-bound verb is offered
on its kind and no other; an imported record cites its file — plus mount
assertions in the CRM and HiringBoard suites); `npm run build` clean. **On
the wire, looked at:** throwaway stack per `web/e2e/stack.ts` (db
`alo_e2e`, jmap :8199, vite :5199 — dropped and killed after); seeded one
record per module through the real API as the signed-in admin and read all
five screenshots (`web/e2e/.artifacts/record-agent-aw2/01…05`, local): the
deal drawer says "From Referral." and offers Move its stage / Draft a
follow-up; the approvals queue's claim in focus says "Created by
admin@e2e.test." and offers Approve it; the project overview's sidebar
panel offers Sum up its status / Log time on it; the purchase order offers
Receive its delivery; the candidate drawer says "From LinkedIn." and
offers Draft a letter. No live model call anywhere.

**Decisions and flags.** (1) **Cut: the product editor's mount joins AW.5.**
"One product, one editor" made the product's editor Billing's shared
`ProductDialog` in `web/src/billing/**` — Codex's directory, which this
loop never touches (same standing as Billing itself). The inventory verbs
for a product (`stock_answer`, `supplier_prices`) ship with that mount, not
before it. (2) **An opaque subject id is not an origin.** Orders'
`createdBy` and statements' `importedBy` hold the OIDC sub, so those mounts
pass origin `null` (or cite the file format) rather than print
"`_8IcWFFzsx…`" — first drafted the other way, caught on the first
screenshot pass and re-verified; the readable creator arrives when the
reads adopt A4.5's `record_origins` join, and each mount then passes it
through. `dealOrigin` likewise cites `source` only. (3) The approvals and
bank rows had no record-in-focus surface, so the identifying cell became
the house row-button toggling one (additive; every existing control kept).
(4) Approvals' `approve_expense` verb also shows on a claim the viewer
cannot decide (their own, in theory) — the agent refuses politely
(ADR 0023 gate is server-side); noted rather than special-cased. (5) i18n
keys in all four catalogs; fr/nl/de drafted here, flagged for native
review at the wave review. (6) No new route prefixes, no deploy note.

**Next:** AW.3 — the personal work modules (Drive, Docs, Sheets, Agenda),
four screenshots.

## AW.3 — the personal work modules (2026-08-29)

**What shipped.** The panel in Drive, Docs, Sheets and Agenda — four record
surfaces, three of which had to be built before a panel could stand on them.

*The catalogue grows four products.* `drive` (rename / move on a file, doc or
sheet; **list_folder** on a folder — a folder is something to look inside, not
to rename through an agent), `docs` (draft a section / rewrite a passage),
`sheets` (write a formula / tidy a column), `agenda` (prepare for it / move it
/ cancel it). Every verb is a name from the intent registry, so the directory
∩ catalogue rule still decides what appears.

*The mounts.* **Drive** had no record view at all — a file manager is a list —
so the item's work was building one: selecting exactly one item (outside the
Trash) opens a **details pane**, a third grid column with the node's name,
size and last change, then its agent. The row menu gained a "Details" entry
that selects the node, so the pane is reachable without hunting for the
checkbox. **Docs**: `DocEditor` gained a `workArea` — the writing surface and
a real 340px column beside it — and an "Its agent" toggle in the header;
docked rather than floating, so no line of text is ever hidden behind it.
**Sheets**: `SheetEditor` already floats a right-hand rail for charts, so the
panel joins that rail (one absolutely-positioned column holding the agent then
the charts — two panels at the same corner would have sat on top of each
other) behind the same "Its agent" toggle. **Agenda**: `DayPanel` only — the
tracks table reserves it for this loop and `EventModal` belongs to
agenda-sync — where every entry gained a focus button beside it; one meeting
at a time goes in focus and its agent appears under the list it was picked
from. Opening the entry still opens the event editor, unchanged.

*Origin.* Drive nodes carry `sourceKind`/`sourceId` (ADR 0021: the email an
attachment was filed from, the room a shared file was kept from), read by new
`driveNodeOrigin` in `drive/parts.tsx` and passed into the editors on the
`openDoc`/`openSheet`/`openOffice` state, so a document's panel costs no
second read. `createdBy` is again refused as a fallback (AW.2 decision 2): it
holds the account id, an opaque string nobody can follow.

**Verified.** `tsc` clean; `eslint` on `src/drive`, `src/agenda`, `src/agents`
and `src/i18n` clean; `vitest` **1360/1360** (7 new: three DayPanel tests —
quiet until a meeting is put in focus, focusing does not open the editor and
the entry still does, the focus is one at a time and lets go; two panel tests
— a folder is offered folder verbs and a file file verbs, a meeting's verbs
propose in the room and execute nothing; two `driveNodeOrigin` tests);
`npm run build` clean. **On the wire, looked at:** throwaway stack per
`web/e2e/stack.ts` (db `alo_e2e`, jmap :8199, vite :5199 — dropped and killed
after; `cargo build -p alo-jmap -p alo-identity --bins` first, 2m26s warm),
signed in, seeded a file with `sourceKind: "email"`, a document and a meeting
through the real API and made the sheet in the app. Read all seven screenshots
(`web/e2e/.artifacts/record-agent-aw3/`, local): the Drive details pane says
**"Raised from an email."** with the source link and offers Rename it / Move
it; the document's side column offers Draft a section / Rewrite a passage; the
sheet's rail offers Write a formula / Tidy a column; the day panel's meeting
offers Prepare for it / Move it / Cancel it — and **Cancel it** landed in the
@agenda one-to-one with the composer holding `Cancel "Delaunay review".` and
nothing sent. Phone width (360px) walked for both new panes: they stack under
the content and the document does not scroll sideways. No live model call
anywhere.

**Decisions and flags.** (1) **A calendar event carries no origin the API
serves** — nothing in `/calendar/events` says which mail, person or room a
meeting grew out of — so Agenda's panel shows the honest empty state. Not a
`[!]`: the panel's other two parts work, and the sentence arrives when the
read adopts A4.5's `record_origins` join. (2) **A doc's panel is @docs, not
@drive.** The same node has two record views: in the file manager it is a
file (@drive: rename, move), in the editor it is a document (@docs: draft,
rewrite). Each surface talks to the agent of the product that surface is.
(3) **The editors' toggles default closed** and make their two reads only when
opened — the queue's "quiet until asked" applied to a writing surface, where a
permanent 340px column would cost the page more than it gives. The button is
in the editor's own header beside the view switch, not behind a menu.
(4) **Two bugs caught by looking, both fixed here:** Drive's header actions
did not wrap, so with a third column they were drawn *over* the record's own
facts (`.actions` now wraps and right-aligns); and the editors' toggles took
their name from a `<span>` the phone stylesheet hides, leaving an unnamed
button at 360px (both now carry `aria-label`). The panel's ask row also wraps
now, so a 300px day panel no longer clips the placeholder mid-word.
(5) `DocEditor`/`SheetEditor` flush a pending save before the panel navigates
to a room, so starting a verb never loses the last edit. (6) i18n keys in all
four catalogs; fr/nl/de drafted here, flagged for native review at the wave
review. (7) No new route prefixes, no deploy note. (8) The browser walk ran
from a temporary `web/e2e/recordAgentAw3.spec.ts`, deleted after — the e2e
config's `testDir` is the whole folder, and a kept file would change what
`npm run test:responsive` runs.

**Next:** AW.4 — the communication and insight modules (Chat, Meet, Insights,
Mail, Contacts, Sites), six screenshots.

## AW.4 — the communication and insight modules (2026-08-29)

**What shipped.** The panel on the record in focus in all six, seven mounts
across six modules, each with its origin derivation and its product's verbs.

*The catalogue grows five products.* `chat` (catch me up / find something in
it — the two verbs that name one room), `meet` (what happened in it /
write the minutes), `insights` (kind `board`: pin a chart to it; kind `tile`:
how has it changed — a board and a chart are different records with different
verbs), `mail` (kind `message`: draft a reply / catch me up on it; kind
`contact`: what we've said to them / write to them — one agent works in the
mail *and* the address book, ADR 0034), `sites` (how it stands / review it for
search engines / publish it). Every verb is a name from the intent registry,
so the directory ∩ catalogue rule still decides what appears.

*The mounts.* **Chat:** `RoomPeople` — the "Who's here" dialog, beside the
agents that are in it and what they remember (A6.4) — with the room as the
record, under the people, since who is here is what the dialog was opened for.
**Meet** had no record surface: the recent list was six lines with nothing to
open, so the item's work was building one — new `meet/RecentMeetings.tsx`
(extracted from `MeetModule`, which keeps its dashboard job) where each ended
meeting has a focus button and one at a time shows its agent under the list.
**Insights:** one panel under the grid, the board's by default and the chart's
when a reader picks one from the tile's own menu (new `TileActions.focus`) —
two records on one screen, never two panels. **Mail:** `ReadingPane`, under
the thread. **Contacts:** the detail pane, as `ContactForm`'s sibling — the
panel carries its own ask `<form>` and HTML forbids nesting — and only for a
saved card. **Sites:** `SiteView`, directly under how the site stands.

*Origin.* Two modules carry one and four honestly do not. A **meeting** knows
the conversation it was started from or the calendar entry it was scheduled as
(`channel`/`event` on the record), so the panel cites the room by name with a
link to it. A **room** knows who opened it (`createdBy` resolved through the
room's own members to a readable address; unresolvable → no origin, never the
id). A **message**'s provenance is its sender, which is the new `sender` origin
kind — "Sent by Ilse Vermeer." A contact, a board, a chart and a site keep no
provenance the API serves, so those say so.

**Verified.** `tsc` clean; `eslint` on the touched trees clean; `vitest`
**1377/1377** (7 new: three panel tests — mail's two kinds and the sender
origin, board-verb vs chart-verb, a room's verb proposes and searches nothing;
three `RecentMeetings` tests — quiet until focus, one at a time and lets go,
the origin is what the meeting carries; one `ChatModule` test — the Who's-here
room panel says who opened it and offers @chat's verb); `npm run build` clean.
**On the wire, looked at:** throwaway stack per `web/e2e/stack.ts` (db
`alo_e2e`, jmap :8199, vite :5199 — dropped and killed after), signed in,
seeded a room, an ended meeting attached to it, a board with a chart, a
contact, an inbox message and a site through the real API, and read all ten
screenshots (`web/e2e/.artifacts/record-agent-aw4/`, local): mail says **"Sent
by Ilse Vermeer."** and offers Draft a reply / Catch me up on it; the contact
says it does not know where it came from and offers What we've said to them /
Write to them; the room says **"Created by admin@e2e.test."** and offers Catch
me up / Find something in it; the meeting says **"Captured from the "release"
conversation."** with its link and offers What happened in it / Write the
minutes; the board offers Pin a chart to it and the chart in focus How has it
changed (and the board's panel gives way to it); the site offers How it stands
/ Review it for search engines / Publish it — and **Review it for search
engines** landed in the @sites one-to-one with the composer holding `Review
"Delaunay Studio" for search engines.` and nothing sent. Phone width (360px)
walked for Meet and Mail: both panels stack under the content and neither page
scrolls sideways. No live model call anywhere.

**Decisions and flags.** (1) **A DM gets no panel.** Chat's verbs name a room
(`catch_up_room` takes a room name) and a direct message has no name to name,
so the panel is a named room's; the dialog is unchanged for DMs. (2) **A chart
and a board are two records, not one screen.** Offering `pin_chart` on a chart
or `insight_change` on a board would be a verb the boundary would refuse, so
`kinds` splits them and the panel below the grid follows the focus. (3) **The
sender is an origin.** New origin kind `sender` rather than reusing `person`
("Created by …" is wrong for mail you received) — and it links nowhere,
because the record *is* the message. (4) `createdBy` refused again as a
fallback on rooms, boards and meetings (AW.2 decision 2): a readable address
comes from the room's members, and an unresolvable id shows no origin at all.
(5) Two suites needed the app's own context now that a panel stands in them —
`contacts/adoption.test.tsx` gained a router and the workspace's authorized
fetch. (6) i18n keys in all four catalogs; fr/nl/de drafted here, flagged for
native review at the wave review. (7) No new route prefixes, no deploy note.
(8) The browser walk ran from a temporary `web/e2e/recordAgentAw4.spec.ts`,
deleted after — as AW.3's was, and for the same reason: the e2e config's
`testDir` is the whole folder.

**Next:** AW.6 — the wave review (AW.5 is the owner's: Billing's editor and
customer view live in Codex's directory). Every moved module's record walked
at desktop and phone width with screenshots, the strings checked in every
language file, `A8.4` flipped to `[x]` in the agents queue with a pointer
here, and `A9.1` noted as the owner's to run.

## AW.6 — the wave review (2026-08-29)

**What shipped.** No product code: this item is the review itself. The whole
wave was walked in a real browser at both widths, the strings were checked in
every language file, and the two queues were closed out.

*The walk.* A throwaway stack per `web/e2e/stack.ts` (db `alo_e2e`, jmap
:8199, vite :5199 — created, then dropped and killed), seeded through the real
HTTP surface with a real access token taken through the app's own PKCE flow:
a task, a deal (its pipeline and stage read from the tenant, not invented), an
expense claim, a project, a supplier and its purchase order, an opening and a
candidate, a Drive file, a document, a sheet, a calendar event, a room, an
ended meeting attached to it, a board, a contact and an inbox message (both
over JMAP), and a site. Then sixteen modules opened the way a person opens
them — a query parameter where the module has one, a click where it does not
— and each was held to the same assertion: `section[data-record]` visible
(the panel is one component, so one selector), scrolled into view,
screenshotted, and the document must not scroll sideways.

**Verified. 16/16 at 1440px, 15/16 at 360px, 32 screenshots read**
(`web/e2e/.artifacts/record-agent-aw6/`, local; `desktop-*.png`,
`phone-*.png`, plus a `*-report.json` per width). Read one by one: the task
detail says **"Created by admin@e2e.test."** and offers Chase it / Set its
priority / Mark it done / Hand it over; the deal drawer **"From Referral."**
with Move its stage / Draft a follow-up; the Edit-claim dialog offers Suggest
categories; the project overview Sum up its status / Log time on it; the
purchase-order editor Receive its delivery; the candidate drawer **"From
LinkedIn."** with Draft a letter; the Drive details pane Rename it / Move it;
the document's side column Draft a section / Rewrite a passage; the sheet's
rail Write a formula / Tidy a column; the day panel's meeting in focus
Prepare for it / Move it / Cancel it; the Who's-here dialog **"Created by
admin@e2e.test."** with Catch me up / Find something in it; the ended meeting
**"Captured from the “release-8e72c4” conversation. Open it"** with What
happened in it / Write the minutes; the board Pin a chart to it; the message
**"Sent by Ilse Vermeer."** with Draft a reply / Catch me up on it; the
contact What we've said to them / Write to them; the site How it stands /
Review it for search engines / Publish it. Every panel names its own agent
(@tasks, @crm, @finance, @projects, @inventory, @hr, @drive, @docs, @sheets,
@agenda, @chat, @meet, @insights, @mail, @sites) and shows the ask box; no
verb was pressed and **no model was called anywhere in this iteration**.

*Phone (360px).* Fifteen modules stack their panel under the content and none
of the sixteen pages scrolls sideways (asserted, not eyeballed:
`documentElement.scrollWidth - innerWidth ≤ 1` at every stop). The dialogs
(Edit claim, Who's here, Contacts) carry the panel inside them and stay
inside the viewport.

**The one gap, and why it is not this track's to close.** **Agenda has no
record-in-focus surface at phone width.** `AgendaModule.module.css` hides
`.dayPanel` outright under 1100px (`@media (max-width: 1100px) { display:
none }`), and the day panel is where the meeting in focus and its agent live;
at 360px the entry opens `EventModal` instead, which carries no panel. Both
files belong to the **agenda-sync** track (this track's writ is `DayPanel`
itself, nothing else under `web/src/agenda/**`), so this is reported, not
patched — the walk records it as `knownAbsent` with that reason rather than
skipping it silently (`phone-agenda-FAILED.png` is the evidence). For the
owner or the agenda-sync queue: either the day panel earns a phone form, or
`EventModal` mounts the panel.

**Strings.** All **97** `recordAgent*` keys exist in `en`, `fr`, `nl` and
`de`, and the four catalogs are at parity (5935 keys each) — checked directly
and held by the ratchet in `src/i18n/locale.test.ts`, which fails the build
for any English key missing from the other three. The fr/nl/de wordings were
drafted by this loop across AW.1–AW.4 and are **still flagged for native
review** — that flag is a human's to clear, and it is the one thing this wave
hands on.

**Gates.** `tsc --noEmit` clean; `eslint` clean on the one file this iteration
touched; `vitest` **1377/1377** (249 files); `npm run build` clean. No Rust,
no migrations, no new route prefixes, no deploy note, no CHANGELOG line —
nothing user-facing changed in this item.

**Bookkeeping.** `AW.6` is `[x]` above. In `docs/autonomy/agents/QUEUE.md`,
`A8.4` is now `[x]` with a pointer to this track and its journal, and `A9.1`
carries a note that its prerequisite is met and it is **the owner's session to
run, never a loop's** (a real model against a live tenant's provider). `AW.5`
stays `[~]`: Billing's editor and customer view are in Codex's directory, and
the mount there is the owner's to make.

**Decisions and flags.** (1) The walk's spec, `web/e2e/recordAgentAw6.spec.ts`,
was deleted after it was read — as AW.3's and AW.4's were, and for the same
reason: the e2e config's `testDir` is the whole folder, so a kept file changes
what `npm run test:responsive` runs. (2) Three seeding facts worth keeping:
a deal needs `pipelineId` **and** `stageId` from the tenant's own pipeline; a
purchase order needs a supplier, so the review created one (`POST
/inventory/suppliers`); and a **document must be a Drive node** (`POST
/drive/files` with `kind: "doc"`) — `POST /docs` writes a record the file
manager does not list, so a doc seeded that way is invisible in Drive.
(3) Two Playwright facts: a failed test recycles the worker and re-runs
`beforeAll`, so seeds carry a per-run suffix or collide on the second pass;
and a panel below the fold of a scrolling dialog is "visible" to the
assertion but absent from the screenshot — the walk scrolls it into view, so
the evidence shows what the assertion checked.

Wave A8.4 closed here — every item was `[x]` or `[~]`, and the record in focus
showed its agent in all sixteen moved modules. Two notes on the marker that
stood at this line:

It was written **`**LOOP COMPLETE**`**, in bold, and the wrapper's pattern
matched only a plain or heading-prefixed marker — so it restarted this finished
track sixteen times, each restart spending a model call to be told there was no
work. Fixed in `scripts/run-loop.sh` (`6371bd90`); the pattern now reads the
emphasis these markers are naturally written with.

The marker itself was then removed by the owner on 2026-08-30, when wave 2
(AW.7, AW.8) released Billing to this track — `web/src/billing/**` for the
panel's mounts only, under the Codex check the queue spells out. A wrapper that
sees a completion marker stops before reading the queue, so the line is gone
rather than edited around.

**Next:** AW.7 — Billing's document editor and customer view.
