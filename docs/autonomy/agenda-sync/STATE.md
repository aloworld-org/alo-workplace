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
