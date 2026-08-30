# agenda-sync — journal

The loop's journal for `docs/autonomy/agenda-sync/QUEUE.md`: one entry per
item, newest at the bottom, in the shape `docs/autonomy/LOOP.md` prescribes.

## Opened 2026-08-29

Agenda's remaining cuts (`docs/interop.md` § CalDAV) and its unbuilt
launch-tier rows (`docs/features.md` § Agenda), as one Rust-first track for
the Mac. Migrations `0910`–`0929`.

## AS.1 — phone-originated per-occurrence edits (2026-08-29)

**Shipped.** `ical::from_ics_series` reads every `VEVENT` of a CalDAV PUT
(master = the one without `RECURRENCE-ID`; each `RECURRENCE-ID` instance →
an override at that slot; `STATUS:CANCELLED` instance → `EXDATE` on the
master; `VTIMEZONE` blocks skipped as before). New store method
`AccountStore::replace_overrides` reconciles the stored override set to the
PUT body (upsert + delete-missing in one tx, edit-gated exactly like
`override_occurrence`); the CalDAV PUT calls it after `put_event`. ETags
(`event_etag`) now hash the override set too — without that, an
instance-only edit kept the old tag and a second device never re-fetched —
and `override_occurrences` is slot-sorted so bodies and tags are
deterministic. RFC 5545 §3.8.4.4, RFC 4791 §4.1.

**Verified.** `cargo nextest run -p alo-store` 2551 passed, `-p alo-jmap`
1568 passed, clippy clean both, fmt done. New tests: 4 ical unit tests
(order-independence, cancel→EXDATE, single-VEVENT equivalence,
VTIMEZONE-skip + slot dedupe), corpus fixtures (Apple-style VTIMEZONE +
moved instance; DAVx⁵-style cancelled instance; re-PUT drops the override)
byte-stable through real Postgres, tenant-isolation (viewer and cross-tenant
`replace_overrides` → NotFound; editor reconcile works), and a caldav.rs
wire test (PUT→GET round-trip, ETag moves on instance-only edit,
cancel→EXDATE, dropped override removed). Live wire pass against the local
backend (debug `alo-jmap` on :8080, db `alo`, tenant `as1-wire`):

```
PUT /dav/calendars/<uid>/default/wire-series-1.ics  (master + RECURRENCE-ID:20260914T090000Z, DTSTART:20260914T150000Z)
→ HTTP/1.1 201 Created, etag "2792080982ee6733"
GET → 200, body serves both VEVENTs incl. RECURRENCE-ID:20260914T090000Z / DTSTART:20260914T150000Z
   db: calendar_event_overrides row (wire-series-1, 2026-09-14 09:00+00, "Standup (moved)", starts 15:00+00)
PUT same href (master + STATUS:CANCELLED at 20260921T090000Z, moved override no longer sent)
→ 204, etag "ccbd745d75d12a4d" (tag moved)
GET → single VEVENT with EXDATE:20260921T090000Z; override rows: 0
```

**Cuts/choices** (recorded in interop.md): `RANGE=THISANDFUTURE` read as a
single-instance override; override-only documents keep the first `VEVENT`
as the event; duplicated slots keep the last. Overrides on a non-recurring
master are cleared, not stored. Depth-1 PROPFIND now loads overrides per
recurring event for the ETag (one query each; batching via `overrides_for`
is a follow-up if listings ever feel it). No migration needed (the
overrides table existed).

**Next:** AS.2 (`VTIMEZONE` emitted).

## AS.2 — `VTIMEZONE` emitted (2026-08-29)

**Shipped.** A served iCalendar document whose date-times carry a `TZID` now
includes one `VTIMEZONE` per zone (RFC 5545 §3.6.5, §3.2.19). New
`tz::observances(zone, from, to)` walks jiff's transition iterators
(`preceding`/`following`) and returns the `STANDARD`/`DAYLIGHT` rules in
force across a span: the rule holding at its start (with the prior rule's
offset as `TZOFFSETFROM`) plus each transition inside it — a bounded set,
never the whole history. `ical::vtimezone_lines` renders the blocks
(`DTSTART` as the onset in the pre-change local time, `TZOFFSETFROM/TO`,
`TZNAME` from jiff's abbreviation) and is wired into `to_ics_at`,
`to_ics_series_at` and `to_imip`; the span unions every zoned instant of the
master and its overrides (starts/ends/RDATE/EXDATE/RECURRENCE-ID). Incoming
`VTIMEZONE` blocks stay ignored; an unresolvable `TZID` still falls back to
UTC and emits nothing.

**Verified.** `cargo nextest run -p alo-store` 2558 passed, `-p alo-jmap`
1568 passed, clippy clean both, fmt done. New tests: 2 tz unit tests
(span-bounded observances incl. the from-offset across the 2026-10-25
switch; fixed-offset zone → one since-forever rule), 5 ical unit tests
(single-rule block byte-checked; open-ended series covers the next two
switches while an `UNTIL`-bounded one stops; UTC/all-day/unknown-zone emit
nothing; `Etc/GMT-2` → single epoch-dated STANDARD; offset formatting incl.
seconds), corpus: zoned + DST-series + Apple-series canonicals now pin the
served `VTIMEZONE` byte-for-byte and stay fixed points, plus a new
fixed-offset-zone fixture (`TZID=Etc/GMT-2`). Live wire pass (debug
`alo-jmap` on :8080, db `alo`, tenant `as2-wire`):

```
PUT /dav/calendars/rG3ib…/default/wire-vtz-1.ics  (Apple PRODID, TZID=Europe/Brussels weekly, no VTIMEZONE shipped)
→ HTTP/1.1 201 Created, etag "88dce0a06ad9e3a1"
GET → 200: BEGIN:VTIMEZONE / TZID:Europe/Brussels /
  DAYLIGHT DTSTART:20260329T020000 +0100→+0200 CEST /
  STANDARD DTSTART:20261025T030000 +0200→+0100 CET /
  DAYLIGHT DTSTART:20270328T020000 +0100→+0200 CEST / END:VTIMEZONE
  before the VEVENT (still ;TZID= wall-clock).
python caldav 2.x + icalendar: principal → calendar → event fetch parses the
block (3 observances, offsets as above) and resolves
DTSTART = 2026-10-19 09:00:00+02:00 Europe/Brussels. Server killed after.
```

**Cuts/choices** (recorded in interop.md): observances are explicit dated
onsets, not `RRULE`-recurring rules; an open-ended or `COUNT`-bounded
recurrence extends the covered span one year past the last referenced
instant (beyond that the IANA name remains the definition, as before —
`UNTIL` is honoured exactly); a client's shipped `VTIMEZONE` is never
echoed back. No migration. Canonical corpus bytes now depend on the tz
database's future rules (they already depended on its conversions; noted
here in case a tzdata update ever moves an EU switch).

**Next:** AS.3 (working hours and zone per person).

## AS.3 — working hours and zone per person (2026-08-29)

**Shipped.** A person's working schedule — weekday set, one daily wall-clock
window, IANA zone — in new table `calendar_working_hours` (migration 0910;
no row = the default Mon–Fri 09:00–17:00). Store module
`calendar_working_hours.rs`: `working_hours`/`set_working_hours` on the
account door (validated: day bits, window order, zone known), and the pure
`outside_hours_spans(hours, fallback_zone, from, to)` — the schedule's
complement over a window, computed day by day in the schedule's zone through
the existing `tz` seam, so DST moves the UTC window and never the local one.
Routes: `GET/PUT /calendar/working-hours` (new `calendar_hours.rs`; wire
spelling ISO weekday numbers + `"HH:MM"` + zone-or-null); `POST
/calendar/freebusy` now serves `outsideHours: [{start,end}]` per known
person **additively beside** `busy` — busy is byte-identical to before, the
two kinds never merge. Web: a Working hours dialog off the Agenda sidebar
(day toggles, time fields, zone select with "My time zone" = null);
EventModal's availability check reports two findings apart — busy guests and
outside-hours guests. i18n keys in en/de/fr/nl. CalDAV untouched.

**Design notes.** Alternative rejected: columns on `user_settings` (mail's
row) — Agenda's schedule gets its own table/module so neither file gains a
second reason to change. The queue's "default … in the tenant's zone" names
a fact the schema does not have (there is no tenant zone); the zone fallback
is the person's profile zone (`users.timezone`, 0152), else UTC — recorded
here as the deliberate deviation. The agents' route-coverage test requires
every `/calendar/*` route to be an intent verb or an exclusion; the new
route got the additive exclusion entry in `alo-ai`'s `agenda_intents.rs`
(the shared-list, keep-both mechanism).

**Verified.** `cargo nextest run -p alo-store` 2571 passed (13 new: 9 unit —
default schedule, UTC/zone/DST complements incl. the 2026-10-25 Brussels
switch, 24:00 end, empty windows, validation — plus 3 DB: roundtrip,
refused-write leaves default, tenant isolation with a forged
(tenantB, userA) door reading the default and writing past A's row);
`-p alo-jmap` 1574 passed (5 new HTTP: default served, wire round-trip,
4×422 verbatim refusals, 401s, freebusy outsideHours beside untouched busy
in UTC and Brussels); `-p alo-ai` 297 passed; clippy clean on all three;
fmt done. Web: `tsc`, eslint, `npm run build` clean; vitest 107 passed
incl. 2 new dialog tests (schedule loads into controls and edits return in
wire spelling; backwards window refused in place, no request). Live wire
pass (debug `alo-jmap` on :8080, db `alo` — confirmed via
`pg_stat_activity` — tenant `as3-wire`):

```
GET /calendar/working-hours → 200 {"days":[1,2,3,4,5],"start":"09:00","end":"17:00","zone":null}
PUT {"days":[1,2,3,4],"start":"08:30","end":"16:00","zone":"Europe/Brussels"} → 200 (echoed)
PUT {"start":"17:00","end":"09:00"} → 422 "working hours must end after they start, within one day"
GET without token → 401
POST /calendar/freebusy (self, 2026-09-01 UTC day) → busy [10:00Z–11:00Z] (the event, untouched);
  outsideHours [00:00Z–06:30Z, 14:00Z–24:00Z]  (= 08:30–16:00 CEST, UTC+2)
db: calendar_working_hours row days=15 start=510 end=960 zone=Europe/Brussels
```

**Cuts/flags.** Cut (recorded): per-day distinct windows and split shifts
(one window for all working days); admin editing others' schedules; tenant
default zone (above); shading one's own off-hours in the week grid — the
"scheduling grid" surface that exists today is EventModal's availability
check, which now shows both kinds. Flag: the web screenshot pass could not
run — port 5173 (the only registered redirect URI) is held by the
agents-web track's dev server from its own checkout, and screenshotting
that stack would show code without this change; the two touched screens are
covered by the new component tests instead, and AS.5's wave review walks
the screens in a real browser. Freebusy now does 2 extra queries per person
(schedule + profile zone, ≤50 people) — fine for the scheduling UI.

**Next:** AS.4 (rooms and resources).

## AS.4 — rooms and resources (2026-08-29)

**Shipped.** A bookable resource is a `calendars` row of kind `resource` in
the tenant's name plus its facts (migration 0911: `calendar_resources` —
address, location, capacity — and `calendar_resource_bookings`, the
event↔room link). Store module `calendar_resources.rs`: tenant-wide list /
by-id / by-address reads, admin CRUD, and `book_resources`, which in ONE
transaction locks the named rooms (`FOR UPDATE`), clears the event's previous
holds, expands every occurrence of the event over the booking window and
refuses (`Conflict`) if any collides with what the room already holds. Routes:
`GET/POST /calendar/resources`, `PUT/DELETE /calendar/resources/{id}` (new
`calendar_resources.rs`; writes `require_admin`, the list open to any member
because you cannot pick a room you cannot see). `POST/PUT /calendar/events`
read the guest list for room addresses and book **before** writing the event,
so a 409 leaves nothing behind; `unbook_event` undoes the reservation if the
write then fails. `POST /calendar/freebusy` answers for a room's address like
a person's, additively tagged `"kind": "user" | "resource" | "unknown"`. An
iMIP invitation is never mailed to a room (it has no mailbox). Web: a Room
picker in `EventModal` (name — location, seats N), the chosen room riding out
as an attendee, a room already on the event shown in the picker rather than
the guest box, the availability check reporting a taken room as its own
finding, and a 409 save saying *which* room is taken. i18n en/de/fr/nl.

**Design notes.** The room is a `calendars` row so it inherits the stable id
that is also a CalDAV collection segment and, more importantly, the *refusal*:
`visible_pred` and `editable_pred` now both exclude kind `resource`, so no
owner and no grant can write into a room's calendar and a room's bookings can
never flood a colleague's week grid. The link row carries no times — *when* a
room is taken is always read back from the event, so moving a meeting moves
its booking and the two cannot disagree; a link whose event is gone occupies
nothing (every read joins the event). The expansion was factored out of
`events_in_range` as `calendar::expand_masters` and is what the room's
schedule uses — still one expansion, as the memory note requires. New
`create_event_at` lets the route choose the id before the row exists, which is
what makes "reserve, then write" possible; `create_event` is now one line over
it. The two new routes got their additive exclusion entries in `alo-ai`'s
`agenda_intents.rs` (the route-coverage test's shared list).

**Verified.** `cargo nextest run -p alo-store` 2582 passed (11 new: 4 unit —
validation, back-to-back vs overlap, the booking window for one-offs and
series, refusal formatting — plus 7 DB: round-trip, one-address-one-thing
incl. a person's, a room being neither visible nor writable as a calendar,
the clash rule with release and self-re-save, a series holding every
occurrence incl. EXDATE-freed and moved-instance slots, delete giving the room
back, and tenant isolation across a forged `(tenantB, userA)` door for all
eight verbs); `-p alo-jmap`+`-p alo-ai` 1877 passed (6 new HTTP: admin CRUD,
role gate + 401, verbatim 422/409 refusals, booking + refusal + move +
release, free/busy for a room beside a stranger, a recurring booking);
clippy clean on all three, fmt done. Web: `tsc`, eslint, `npm run build`
clean; vitest 1356 passed incl. 3 new (picker sends the room out, a held room
sits in the picker not the guest box, a 409 names the room). Live wire pass
(debug `alo-jmap` on **:8091** — 8080 was held by the agents-web track's
server, so a second port rather than killing theirs — db `alo`, tenant
`AS4 Wire`):

```
POST /calendar/resources {"name":"Board room","email":"board@as4-wire.test",…}
→ 200 {"id":"AjMXEkaHXoUAeIx6A_GC0w",…}   db: calendars.kind='resource' + calendar_resources row
POST /calendar/events  10:00–11:00, attendees ["board@as4-wire.test"] → 200
   db: calendar_resource_bookings(AjMX…, ixgh…) ; event 10:00–11:00
POST /calendar/events  10:30–11:30, same room
→ 409 {"detail":"Board room is already booked from 2026-09-02T10:00:00Z to 2026-09-02T11:00:00Z"}
POST /calendar/events  11:00–12:00, same room → 200 (back-to-back is not a clash)
POST /calendar/freebusy → {"kind":"resource","busy":[10:00Z–12:00Z],"outsideHours":[]}
GET  /calendar/calendars → ["cal_personal_…"] only — the room is not in the week grid
POST /calendar/events with calendarId = the room → 404 (can_edit refuses)
PUT  the first meeting to 14:00–15:00 → 200; the freed 10:30 slot then saves → 200
DELETE /calendar/resources/{id} → 200; list empty; bookings cascaded to 0 rows,
   and the three meetings are still in the diary (they lost a room, not their time)
```

**Cuts/flags.** Cut to a shippable whole and queued as **AS.4b**: the CalDAV
half — serving each room as a read-only collection and booking from a CalDAV
PUT. It is a real seam (hrefs must be built against the collection being
listed, and `event_propstat`/`fetch_event` have to take one), and shipping it
half-done would have put ghost hrefs in phones. Recorded in `interop.md` so
nobody reads today's behaviour as final. Cut as the item allowed: approval
workflows for rooms, and per-instance conflict reporting (a colliding series
is refused whole, naming the first taken slot). Also cut: a room's own colour
and a rooms admin screen — rooms are created through the API this iteration;
the picker reads them. Flags: a room's collection shows the *titles* of
colleagues' bookings, which is how a shared room calendar works everywhere and
is why free/busy (times only) is what Sites and the grid see. An open-ended
series reserves its room 400 days out (`MAX_BOOKING_DAYS`), re-checked on each
later save. The web screenshot pass again could not run — port 5173 is the
agents-web track's; the three new component tests cover the picker and AS.5's
review walks the screens.

**Next:** AS.4b (a room's calendar over CalDAV).

## AS.4b — a room's calendar over CalDAV (2026-08-29)

**Shipped.** A room is now a CalDAV collection of its own at
`calendars/<uid>/<resourceId>/`, read-only to every member of the tenant.
`PROPFIND` on calendar-home lists it beside the personal and shared calendars
(name, location as `calendar-description` per RFC 4791 §5.2.1, `read` alone in
`current-user-privilege-set`); its members are the meetings that booked it,
whoever owns them. The href seam AS.4 flagged is closed: `event_propstat` takes
the **collection being listed** rather than deriving one from the event's own
calendar, so a room's client is never handed a colleague's collection. `GET`,
`calendar-multiget`, `calendar-query` (`time-range`) and `free-busy-query` all
answer on a room; a booking is readable there and nowhere else the caller
cannot already see. Every write is `403` — `PUT` and `DELETE` refuse before the
store is touched, because a room's schedule is written by booking it. The
served `ATTENDEE` for a room carries `CUTYPE=ROOM;RSVP=FALSE;PARTSTAT=ACCEPTED`
(RFC 5545 §3.2.3); an incoming `CUTYPE` is still ignored — what a room *is* is
the tenant's resource list, not a parameter a client sends. And a resource
attendee arriving on a **CalDAV PUT** now books the room through the same
`book_resources` check the Agenda and the JSON API use, taken before the write,
so a collision is `409` (RFC 4791 §5.3.2) carrying the store's own sentence and
leaves nothing behind; a room dropped from the guest list is released by the
same PUT.

**Design notes.** One new store read, `event_in_calendar`, over the existing
`calendar_scope_pred` — the predicate AS.4 already wrote for
`events_of_calendar`, so the room case and the visible-calendar case stay one
rule rather than two. A room's `sync-token`/`getctag` is a **hash of its
members' ETags** (`urn:alo:room:<hash>`) instead of the account modseq: a
room's members are other people's meetings, and their writes never bump this
caller's modseq, so a modseq token would sit still while the room filled up.
That token cannot answer "what changed", so it answers only what it honestly
can (RFC 6578 §3.2): no token → every member; the current token → nothing
changed; anything else → `403 DAV:valid-sync-token`, which sends the client to
a full listing. `to_ics_series_with_rooms` carries the tenant's room addresses
into the serializer, which has no database and must not grow one; with an empty
set it is byte-identical to `to_ics_series`, which is what keeps the round-trip
corpus pinned. The lazy-override branch in `report_events` was removed: it
loaded the overrides on the next line regardless, so it was two paths spelling
one, and the window test is now the shared `in_window`.

**Verified.** `cargo nextest run -p alo-store` 2584 passed (3 new: the
`CUTYPE=ROOM` unit test incl. the empty-set identity and the parse-back, a room
serving a colleague's booking as its own member while their private event and a
guessed id stay `None`, and the wrong-tenant/forged-door proof for
`event_in_calendar`); `-p alo-jmap` + `-p alo-ai` 1880 passed (3 new wire
tests: the read-only collection with hrefs under the room, `CUTYPE=ROOM`, the
403s and the untouched booking; the PUT booking with its 409, back-to-back and
release; the sync-token's three answers). Clippy clean on all three, fmt done.
No web changes, so no screenshot pass was needed this iteration. Live wire pass
(debug `alo-jmap` on **:8092** — the agents-web track's server is still up, so
a third port rather than killing theirs — db `alo` confirmed via
`pg_stat_activity`, tenant `AS4b Wire`, users `as4b@` and `mate@`):

```
PROPFIND /dav/calendars/<uid>/ Depth:1
→ .../default/            | Personal   | -                    | urn:alo:calendar:1
  .../9K2Ahp…/            | Board room | 2nd floor, east wing | urn:alo:room:df6dd6642f5bedf9 | read-only

PROPFIND .../9K2Ahp…/ Depth:1  → 207, href .../9K2Ahp…/nMM2hs….ics  (the COLLEAGUE's booking)
GET      .../9K2Ahp…/nMM2hs….ics → 200, ETag "20e36f1c69bed596" (= the propstat's)
  ATTENDEE;CUTYPE=ROOM;ROLE=REQ-PARTICIPANT;RSVP=FALSE;PARTSTAT=ACCEPTED:mailto:board@wire.test
GET      .../default/nMM2hs….ics → 404   (readable through the room, not through mine)
PUT      .../9K2Ahp…/squat-1.ics → 403 ; DELETE .../9K2Ahp…/nMM2hs….ics → 403 ; GET again → 200
REPORT   free-busy-query on the room → 200 FREEBUSY;FBTYPE=BUSY:20260902T100000Z/20260902T110000Z
                                        (no SUMMARY — the serializer has no field for one)

PUT .../default/clash-1.ics  10:30–11:30, ATTENDEE:board@wire.test
→ 409 "Board room is already booked from 2026-09-02T10:00:00Z to 2026-09-02T11:00:00Z"
   GET clash-1 → 404 (a refusal leaves nothing behind)
PUT .../default/after-1.ics  11:00–12:00, same room → 201; db: bookings(9K2Ahp… ← after-1)
PUT .../default/after-1.ics  same times, ATTENDEE removed → 204; db: 0 rows for after-1
PUT .../default/clash-2.ics  11:00–12:00, the room again → 201 (the released slot is free)

REPORT sync-collection (no token) → 207, both members + urn:alo:room:ee06f930d70e581f
REPORT sync-collection (that token) → 207, no members, same token
  …a third booking lands…
REPORT sync-collection (that token) → 403 <d:error><d:valid-sync-token/></d:error>
GET /calendar/calendars → ["cal_personal_…"] only — no room booking in the week grid
```

**Cuts/flags.** Cut (recorded in `interop.md`): a room's sync reports deletions
by absence from a full listing rather than as `404` responses — the price of a
state-hash token, and revisitable if a room ever holds enough bookings for the
listing to hurt. Flags: a room's collection shows the **titles** of colleagues'
bookings — how a shared room calendar works everywhere, and the reason
free/busy (times only) stays what the grid and Sites see; it is written down in
`interop.md` as a disclosure rather than left implicit. The room's ctag reads
all of a room's bookings on every PROPFIND (O(members)); fine at a room's scale,
worth an index-backed tag if a room ever collects years of history.
`scripts/prune-test-db.sh` ended with an FK error deleting a tenant whose
`billing_products` are referenced by `billing_invoice_lines` (the billing demo
seeds — another track's area, reported not fixed); it still pruned to 5464
tenants / 93 MB and the gate ran in under a minute.

**Next:** AS.5 (wave review).

## AS.5 — wave review (2026-08-29)

**Shipped.** The three things the item asked for, and one bug the review was
written to find.

*The bug.* `docs/interop.md`'s scripted **CalDAV and CardDAV transcripts had
silently stopped regenerating.** `scripts/wire-transcripts.sh` selects its tests
with `-E 'binary(transcripts)'`; when `alo-jmap`'s `tests/` was consolidated
into suite binaries on 2026-08-28 (the change that removed the 40-minute
relink), `transcripts.rs` became a *module of* `mail_http_suite` and stopped
matching. The script's own missing-transcript check would have caught it — but
only after the earlier splice had already been written, so a run that "passed"
left the old bytes in place. The checked-in CalDAV transcript therefore showed a
`TZID=Europe/Brussels` object served **without a `VTIMEZONE`**: the server
described in the docs was two features out of date. Filter fixed
(`+ test(transcripts::caldav_transcript)`, and the CardDAV one), transcripts
regenerated: the CalDAV section now carries the `VTIMEZONE` with the
2026-10-25 and 2027-03-28 transitions. The only other churn is the
nondeterministic UIDVALIDITY/ETag/DTSTAMP values in the mail transcripts.

*`interop.md` reads true.* Audited the whole § CalDAV against the code. Three
statements were false and are corrected: the opening said **one** collection per
account at `default/` (it is one per calendar the caller can see, rooms
included); it said the sync-token is always the account modseq (a room's is a
state hash — the AS.4b note said so 100 lines further down and the opening
contradicted it); and a scope-cut bullet still claimed `calendar-query`
**time-range** filters are not evaluated, which the "Time-range filtering"
bullet on the same page had already contradicted since M3.1 — only the
non-range parts of the filter (`comp-filter`, `prop-filter`, `text-match`) are
unevaluated. Everything else checked out against the source: `PROPPATCH`/
`MKCALENDAR`/`MOVE` are `405` (the method match has five arms), `VTODO`/
`VJOURNAL`/`TRANSP` are unmodelled, `RDATE;VALUE=PERIOD` is skipped,
`BYMONTH`/`BYSETPOS`/`WKST` are ignored with Monday assumed.

*`ROADMAP.md` Phase 2 § Agenda* now says what is built: the events/invitations/
free-busy/recurrence row and the shared-calendars/rooms row are `[x]` with the
detail behind them; `alo-dav` keeps `[~]` but the remainder is **only the crate
extraction** (CalDAV itself is built); the Mail-integration row is `[~]` —
invite cards built, the Meet half waits on Meet. All four `[L]` rows of
`features.md` § Agenda are now built.

**Verified.** `cargo nextest run -p alo-store -p alo-jmap` → **4167 passed**,
1 skipped, 239 s. Transcript regeneration green (7 tests, incl. the two that
had been skipped for a day). Live wire pass against the local backend (debug
`alo-jmap` on **:8092**, db `alo` confirmed via `pg_stat_activity`, tenant
`AS5 Wave`, users `as5@`/`mate5@wire.test`; the server was killed by pid, never
`pkill`, because another track's binary was up). Raw curl with the literal XML —
the python `caldav` library is not installed on this machine, which the queue
allows.

AS.1 — a phone's per-occurrence edits, all four shapes, one resource:

```
PUT  as5-standup.ics  master (weekly ×4, TZID=Europe/Brussels) + one VEVENT
                      with RECURRENCE-ID:20261026T090000 moved to 11:00
→ 201 ETag "60ca6c1e5a4967a5"
GET → both VEVENTs, the override carrying its RECURRENCE-ID at the ORIGINAL slot
PUT  the same instance moved again (14:00)        → 204; GET → 14:00, new summary
PUT  the instance with STATUS:CANCELLED           → 204; GET → no override VEVENT,
                                                    EXDATE;TZID=Europe/Brussels:20261026T090000
PUT  the master alone (the body is the resource)  → 204; GET → master only, EXDATE gone
```

AS.2 — `VTIMEZONE`, from the same GET and two more objects:

```
GET as5-standup.ics →  BEGIN:VTIMEZONE / TZID:Europe/Brussels
   DAYLIGHT 20260329T020000 +0100→+0200 CEST
   STANDARD 20261025T030000 +0200→+0100 CET     (the switch the series crosses)
   DAYLIGHT 20270328T020000 +0100→+0200 CEST    (one year past the last instant)
PUT TZID=Etc/GMT-5 → one STANDARD block at 19700101T000000, +0500→+0500, TZNAME +05
PUT TZID=Eastern Standard Time (Outlook's display name)
   → served as DTSTART:20261110T090000Z, no VTIMEZONE — the documented fallback
```

The room collection (AS.4/AS.4b), re-driven end to end by a second user:

```
POST /calendar/resources {"Wave room","3rd floor, north",8,wave-room@wire.test} → 200
as5:   PUT default/as5-review.ics with ATTENDEE:mailto:wave-room@wire.test → 201
mate5: PROPFIND calendars/<mate5>/ Depth:1
   → .../default/            Personal    urn:alo:calendar:0
     .../w3eCBR7…/           Wave room   calendar-description "3rd floor, north"
                             urn:alo:room:f9ec8840dc767c6a   privilege: read only
mate5: PROPFIND .../w3eCBR7…/ Depth:1 → the href is .../w3eCBR7…/as5-review.ics
       (under the ROOM, not under as5's collection)   ETag "65eb86127aea00ae"
mate5: GET  that href → 200, ATTENDEE;CUTYPE=ROOM;ROLE=REQ-PARTICIPANT;RSVP=FALSE;
                              PARTSTAT=ACCEPTED:mailto:wave-room@wire.test
mate5: GET  .../default/as5-review.ics → 404   (readable through the room alone)
mate5: PUT into the room → 403 ; DELETE from it → 403 ; as5 (its admin) PUT → 403
       GET again → 200 (a refusal changed nothing)
mate5: PUT default/mate5-clash.ics 09:30–10:30, same room
   → 409 "Wave room is already booked from 2026-09-10T09:00:00Z to …T10:00:00Z"
     GET the refused event → 404
mate5: REPORT free-busy-query → 200, FREEBUSY;FBTYPE=BUSY:20260910T090000Z/…T100000Z
       (no SUMMARY — the serializer has no field for one)
mate5: REPORT calendar-query, time-range 2026-10-01…10-30 → 0 hrefs (range honoured)
mate5: REPORT sync-collection: no token → the member + urn:alo:room:f9ec8840dc767c6a;
       that token → 0 hrefs, same token; a stranger token → 403 <d:valid-sync-token/>
mate5: GET /calendar/calendars → ["cal_personal_…"] only — a room is not in the grid
```

**Cuts/flags.** No CHANGELOG line: nothing user-facing changed this iteration
(docs, a ROADMAP status, and a test-selection filter). Follow-on, not built
because the item is a review: the scripted CalDAV transcript still exercises the
personal collection only — a room leg would keep AS.4b honest the way the
regenerated `VTIMEZONE` now keeps AS.2 honest, and it belongs in
`transcripts.rs` next to the existing legs. Flag for whoever owns the harness:
`scripts/wire-transcripts.sh` checks for a missing transcript only *after*
splicing, so a silent skip leaves stale bytes in the doc looking fresh — the
check should run before the splice. `scripts/prune-test-db.sh` needs
`DATABASE_URL` naming `alo_scratch` here (that is `alo-test-db`'s fallback, and
the biggest database on the box at 158 MB); run so, it reported 4072 tenants and
removed none — all of them are younger than its 2-hour cutoff, i.e. another
track's suites, so nothing was stale rather than nothing was found.

**Next:** AS.6 — the meeting in focus, and its agent, at phone width.

Wave AS closed here, and its `LOOP COMPLETE` was removed by the owner on
2026-08-30 when wave 2 was added: the agents-web review walked sixteen pages at
360px and found Agenda the only module with no record-in-focus surface, in
files that belong to this track. The marker is gone rather than edited around,
because a wrapper that sees it stops before reading the queue.

## AS.6 — the meeting in focus, and its agent, at phone width (2026-08-30)

**Shipped.** `EventModal` mounts `RecordAgentPanel` — the same component the
day panel mounts, imported, not reimplemented — under the form, for a saved
event, and only below the width at which `AgendaModule.module.css` hides
`.dayPanel`. That threshold is now written once, as `DAY_PANEL_HIDDEN`
(`"(max-width: 1100px)"`) exported from `DayPanel.tsx` and read through
`ds/useMediaQuery`, so the stylesheet's rule and the modal's condition cannot
drift into a screen with two agent panels or none. The panel's props are the
record the modal already holds (`event.id`, the title as typed, `origin: null`
— `/calendar/events` still says nothing about where an event came from, the
same reason the day panel gives), and `onBeforeNavigate={onClose}` so a verb
that opens the agent's conversation closes the editor behind it.

**The one structural change, and why it was unavoidable.** The panel carries
its own `<form>` (its one-line ask) and HTML forbids a form inside a form — a
nested one would have bubbled the ask's submit into the editor and *saved the
meeting when somebody asked a question*. So the editor's fields moved into a
named form of their own (`useId`), the modal's outer element became a `<div>`,
and the footer's submit button points at the form by id — exactly the shape
Finance's `DialogFrame` settled on for exactly this reason, and the shape its
`aside` prop documents. No `.module.css` rule was added, changed or removed
(ADR 0046); nothing under `web/src/agents/**` was touched beyond the import;
no new i18n key — every string in the panel is one of the 97 `recordAgent*`
keys that already exist in en/fr/nl/de.

**Verified in a real browser**, not assumed: a throwaway stack per
`web/e2e/stack.ts` (db `alo_e2e`, jmap :8199, vite :5199 — created, then
dropped and killed), signed in through the app's own login, one meeting seeded
through the app itself, then the same meeting opened at two widths. Screenshots
read one by one in `web/e2e/.artifacts/agenda-agent-as6/` (local, gitignored):

```
desktop-agenda-editor.png   1280×900  Edit event — title, times, zone, repeat,
                            reminder, location, meeting, guests, notes, and NO
                            agent panel (asserted: section[data-record] × 0)
desktop-agenda-daypanel.png 1280×900  the same meeting in focus in the day
                            panel: "This record's agent" / "This record doesn't
                            say where it came from." (× 1)
phone-agenda-agent.png      360×740   the editor, scrolled to the panel:
                            "This record's agent  @agenda",
                            "This record doesn't say where it came from.",
                            "What @agenda can do here" — Prepare for it /
                            Move it / Cancel it, and "Ask @agenda about this…"
                            with its Ask button. Delete / Cancel / Save still
                            under it; documentElement.scrollWidth − innerWidth
                            = 0 (no sideways scroll).
```

The walk's spec (`e2e/agendaAgentAs6.spec.ts`) was deleted after its shots were
read, as AW.3/AW.4/AW.6's were and for the same reason: the e2e config's
`testDir` is the whole folder, so a kept file changes what `npm run
test:responsive` runs.

**The `knownAbsent` entry is corrected.** `docs/autonomy/agents-web/STATE.md`'s
AW.6 walk recorded Agenda as the one of sixteen pages with no record-in-focus
surface at 360px, evidenced by `phone-agenda-FAILED.png`. It has one now, and
the replacement evidence is `phone-agenda-agent.png` above — 16/16 at both
widths. That journal is another track's file and is not edited from here; this
entry is the correction, and AS.7 is the wave check that re-walks it.

**Gates.** `npx tsc --noEmit` clean; `npx eslint` clean on the three changed
files; `npx vitest run src` **1379/1379** (249 files) — four of them new in
`src/agenda/eventAgent.test.tsx`: the panel is there at 360px with its verbs
and its origin sentence, absent at 1280px (and nothing is fetched for it),
absent for an unsaved event, and *the fields still save while the panel is
mounted beside them* — the test that guards the form-id change. `npm run build`
clean. No Rust, no migrations, no new routes, no deploy note.

**Cuts/flags.** (1) Nothing cut. (2) Two things the walk saw that are not this
item's to fix, and are not regressions: at 360px the month grid's last row sits
under the app's bottom bar, so an event on the 30th could not be clicked there
(the walk used the Agenda list view, which is how a phone reads a calendar
anyway) — that is the responsive track's area; and the day panel's agent can be
screenshotted before its directory read returns, which is why
`desktop-agenda-daypanel.png` shows the origin sentence without the handle and
verbs. (3) The e2e teardown dropped `alo_e2e` but left an `alo-jmap` from the
run before it holding the port; killed here. Worth knowing for whoever runs
that suite next: check `pgrep -f alo-jmap` before starting, as CLAUDE.md's
rule says.

**Next:** AS.7 — the wave check (a sixteen-page walk at 360px with no
`knownAbsent` for Agenda, `interop.md` and `ROADMAP.md` still true).
