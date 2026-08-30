# alo Agenda — the sync tail and the launch-tier gaps

Agenda is built: personal and shared calendars, invitations (iTIP/iMIP),
free/busy, recurrence with exceptions and per-occurrence edits, CalDAV with one
collection per visible calendar, time-range queries, zoned recurrence. What is
left is written down in `docs/interop.md` ("CalDAV" section) as deliberate
cuts, and in `docs/features.md` § Agenda as launch-tier (`[L]`) rows not yet
built. This queue finishes both. Rust first; protocol work under the
`protocol` skill; every deviation from an RFC recorded in `docs/interop.md`.

**Read first:** `docs/interop.md` § CalDAV (the whole section, cuts included),
`platform/alo-store/src/ical.rs`, `calendar.rs`, `calendar_availability.rs`,
`products/mail/alo-jmap/src/carddav.rs` (CardDAV and CalDAV share it),
`calendar.rs` (the `/calendar/*` routes), and the tests
`alo-store/tests/ical_corpus.rs`, `alo-jmap/tests/caldav.rs`,
`calendar_http.rs`. The memory note that matters: *one store expansion for
recurrence, shared by the Agenda range listing, availability and CalDAV —
never two implementations.*

## Who is where right now (2026-08-29, read before any item)

- Codex (a separate editor) owns `web/src/billing/**` and `web/src/shell/**`.
  Not this track's area anyway.
- `web/src/sites/**` binds calendars for bookings (`site_agenda`, ADR 0040);
  read it, never edit it. The availability seam (`calendar_availability.rs`)
  is the only vocabulary Sites may hear — keep it that way.
- The agents-web track (Mac, other clone) edits `web/src/agenda/DayPanel.tsx`
  and the event view for its panel mount. Coordinate by file: this track's
  web work is `EventModal`, the settings surface it adds, and the sidebar —
  not `DayPanel`. A rebase conflict inside `web/src/agenda/**` is resolved by
  keeping both sides.

## Areas this track owns

`platform/alo-store/src/calendar*.rs`, `ical.rs`, its new store modules;
`products/mail/alo-jmap/src/carddav.rs`, `calendar.rs`, the calendar
half of `api.rs` (invitations); `web/src/agenda/**` (see the coordination
note); `web/src/jmap/client.ts` + `types.ts` additively; `web/src/i18n/**`
(additive keys, every language file). **Migrations `0910`–`0929`.** Check the
directory immediately before rebasing.

## Rules

- **Strict in what we send, tolerant in what we accept** (`protocol` skill).
  Cite the RFC section in the commit body for every wire behaviour.
- **Real-client evidence.** Every item touching CalDAV ends with the python
  `caldav` library (or `curl` with the raw XML) driving the local backend and
  the exchange quoted in STATE.md — the interop-tester pattern this repo has
  used since slice 2. The wire bytes are the proof; a green suite is not.
- **The wrong-tenant test is mandatory** for every new store surface
  (`tests/tenant_isolation.rs` has the calendar-sharing example).
- **Verify by looking** for the web half: one screenshot per screen touched.
- **`docs/interop.md`** loses the cut an item removes and gains the choice an
  item makes, in the same commit.

## Wave AS — the cuts, then the launch-tier rows

- [x] AS.1 **Phone-originated per-occurrence edits.** `from_ics` reads every
  `VEVENT` of a PUT: the one without `RECURRENCE-ID` is the master; each one
  with a `RECURRENCE-ID` becomes an override at that slot
  (`override_occurrence`), a `STATUS:CANCELLED` instance becomes an `EXDATE`,
  and an override the client no longer sends is removed. RFC 5545 §3.8.4.4,
  RFC 4791 §4.1. Corpus fixtures from what Apple Calendar and DAVx⁵ actually
  write; a `caldav.rs` wire test PUTs a two-`VEVENT` series and reads it back
  moved; the interop cut is deleted.
- [x] AS.2 **`VTIMEZONE` emitted.** A served object whose date-times carry a
  `TZID` includes one `VTIMEZONE` per zone (RFC 5545 §3.6.5, §3.2.19), built
  from `jiff`'s zone data — the `STANDARD`/`DAYLIGHT` rules in force across
  the object's span (a bounded set of transitions, not the whole history).
  Incoming `VTIMEZONE` blocks stay ignored (the IANA name is the definition);
  a `TZID` that is not an IANA name still falls back as today. Corpus tests
  round-trip a Europe/Brussels series across a DST switch and a fixed-offset
  zone; the interop note moves from "follow-up if a client balks" to "done".
- [x] AS.3 **Working hours and zone per person** (`features.md` [L] "working
  hours, time-zone sanity for cross-border teams"). A person's working days,
  hours and zone (migration; default Mon–Fri 09:00–17:00 in the tenant's
  zone); `GET/PUT /calendar/working-hours`; free/busy and the scheduling grid
  distinguish *busy* from *outside hours* (a second span kind on the wire,
  additively — existing clients see the same busy periods as before); the
  Agenda settings surface to edit them; `EventModal`'s availability check
  shows both. CalDAV is untouched. Tenant-isolation test.
- [x] AS.4 **Rooms and resources** (`features.md` [L]). A resource is a
  calendar of kind `resource` owned by the tenant, with a name, location and
  capacity, managed by an admin (`/calendar/resources` CRUD, role-gated as
  the admin routes are); an event books it by naming it as a resource
  attendee; a double booking is **refused at write** (the store checks the
  resource's expansion — the one expansion — over the event's span,
  overrides included); free/busy answers for resources like for people. The
  Agenda `EventModal` picks a room and shows the conflict. Served over CalDAV
  as a read-only collection to tenant members (the resource's calendar is
  visible, never editable, through the existing `can_edit` refusal). Cut and
  recorded if too wide: approval workflows for resources, recurring bookings'
  partial conflicts (refuse the whole series, say which instances collide).
- [x] AS.4b **A room's calendar over CalDAV** (the half AS.4 cut, so its
  slice shipped whole). Each resource is served as a read-only collection to
  every tenant member: `PROPFIND` on calendar-home lists it beside the
  personal and shared calendars, its objects are the events that booked it
  (hrefs under the room's own segment, not the booker's — `event_propstat`
  takes the collection it is being listed in), `GET` of one object works for a
  booking somebody else owns, and every write is the existing `can_edit`
  refusal. The served `ATTENDEE` for the room carries `CUTYPE=ROOM`
  (RFC 5545 §3.2.3), and a resource attendee arriving on a CalDAV **PUT**
  books the room through the same `book_resources` check — a collision is
  `409`, which RFC 4791 §5.3.2 allows a PUT to answer. Real-client evidence as
  the rules above require; `docs/interop.md`'s AS.4 note loses both cuts.
- [x] AS.5 Wave review: `docs/interop.md` § CalDAV reads true against the
  code; `python -m caldav` (or the raw exchanges) against the local backend
  for AS.1, AS.2 and the resource collection, quoted in STATE.md; `ROADMAP.md`
  Phase 2 § Agenda rows updated to what is actually built; then
  `LOOP COMPLETE`.

## Wave 2 — the gap the agents-web review found (2026-08-30, owner)

The agents-web track finished A8.4 and walked all sixteen pages at phone
width. Fifteen show the record in focus and its agent; Agenda does not, and
that track reported it rather than patching it, because the files are this
track's. Its evidence is `phone-agenda-FAILED.png` and the `knownAbsent` entry
in its walk. **agents-web is finished, so `DayPanel` is no longer reserved and
this track may edit it.**

- [x] AS.6 **The meeting in focus, and its agent, at phone width.**
  `AgendaModule.module.css` hides `.dayPanel` outright below 1100px
  (`@media (max-width: 1100px) { display: none }`), and the day panel is where
  the meeting in focus and its `RecordAgentPanel` live; at 360px an entry opens
  `EventModal`, which carries no panel. Close it the way that suits the screen
  rather than the way that is quickest: either the day panel earns a phone
  form, or `EventModal` mounts the same panel. Read
  `web/src/agents/RecordAgentPanel.tsx` first and **mount it — do not
  reimplement it**; a second copy of that panel is the defect this item exists
  to avoid, and its props come from the record the modal already holds.
  **Done when:** a 360px screenshot of a meeting shows its origin, its verbs
  and its ask — opened and read, not assumed; the 1280px view is unchanged from
  today; `npx vitest run src` green; `npx tsc --noEmit` clean; no new
  `.module.css` rule (ADR 0046 — Tailwind utilities from the tokens); nothing
  under `web/src/agents/**` edited beyond an import; and the `knownAbsent` walk
  entry is corrected in this journal with the new screenshot named. If the
  honest answer is that the panel does not belong at phone width, that is a
  finding: say so with the reasoning and mark the item `[!]` rather than
  shipping a cramped surface.
- [ ] AS.7 Wave check: the sixteen-page walk at 360px carries no `knownAbsent`
  for Agenda; `docs/interop.md` and `ROADMAP.md` still read true; then
  `LOOP COMPLETE`.
