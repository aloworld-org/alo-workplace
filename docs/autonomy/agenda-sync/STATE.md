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
