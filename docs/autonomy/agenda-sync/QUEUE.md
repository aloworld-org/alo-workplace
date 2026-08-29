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
- [ ] AS.2 **`VTIMEZONE` emitted.** A served object whose date-times carry a
  `TZID` includes one `VTIMEZONE` per zone (RFC 5545 §3.6.5, §3.2.19), built
  from `jiff`'s zone data — the `STANDARD`/`DAYLIGHT` rules in force across
  the object's span (a bounded set of transitions, not the whole history).
  Incoming `VTIMEZONE` blocks stay ignored (the IANA name is the definition);
  a `TZID` that is not an IANA name still falls back as today. Corpus tests
  round-trip a Europe/Brussels series across a DST switch and a fixed-offset
  zone; the interop note moves from "follow-up if a client balks" to "done".
- [ ] AS.3 **Working hours and zone per person** (`features.md` [L] "working
  hours, time-zone sanity for cross-border teams"). A person's working days,
  hours and zone (migration; default Mon–Fri 09:00–17:00 in the tenant's
  zone); `GET/PUT /calendar/working-hours`; free/busy and the scheduling grid
  distinguish *busy* from *outside hours* (a second span kind on the wire,
  additively — existing clients see the same busy periods as before); the
  Agenda settings surface to edit them; `EventModal`'s availability check
  shows both. CalDAV is untouched. Tenant-isolation test.
- [ ] AS.4 **Rooms and resources** (`features.md` [L]). A resource is a
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
- [ ] AS.5 Wave review: `docs/interop.md` § CalDAV reads true against the
  code; `python -m caldav` (or the raw exchanges) against the local backend
  for AS.1, AS.2 and the resource collection, quoted in STATE.md; `ROADMAP.md`
  Phase 2 § Agenda rows updated to what is actually built; then
  `LOOP COMPLETE`.
