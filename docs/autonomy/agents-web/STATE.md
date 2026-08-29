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
