# alo Mail — loop journal

Track opened 2026-08-26. Queue derived from the open mail lines of ROADMAP.md
after the 2026-08-25 audit trued them up; the engine itself (SMTP + trust
stack, JMAP on 443, IMAP/POP3, CardDAV, filters, out-of-office scheduling,
per-identity signatures) is done, live, and wire-verified — this track builds
the tail between "works" and "a business runs on it".

Standing facts every iteration should know:

- MAPI is retired by decision (ADR 0056). Nothing here touches it.
- Another agent (Codex) is actively reworking web/src/ds, web/src/billing,
  web/src/chat and some web/src/mail components in its own checkout. Backend
  items are ordered first for this reason; rebase early, keep both sides of
  additive i18n conflicts.
- Migrations for this track: 09xx, append-only, expand-only.
- Deploys are the human's. Build, test, commit, push — nothing else.

## Iterations

### 2026-08-26 — iteration 1 — M1.1 app-specific passwords (store + identity)

Shipped: migration `0900_app_passwords.sql` (per-user named credentials,
argon2id PHC hash at rest, `last_used_at`, revoke = delete); store module
`alo-store/src/app_passwords.rs` (tenant-door create/list/revoke with a
20-per-user cap bounding the argon2 work one login can cost, pre-tenant
`app_password_credentials_by_username` + `touch_app_password` for the legacy
auth path) with `AppPasswordId`; identity module
`alo-identity/src/app_password.rs` (CSPRNG generation — 16 lowercase letters
shown once as `xxxx-xxxx-xxxx-xxxx`, ~75 bits; hashing under the account
password parameter contract; `verify_app_password` through the existing
dummy-hash seam, verifying the canonical de-dashed form so clipboard grouping
never matters; list/revoke pass-throughs).

Verified: clippy clean on both crates; full suites green (2 540 tests, incl.
12 new: wrong-tenant AND wrong-user on create/list/revoke, username lookup
resolves one user only, cap + name validation, revoked-hash gone, roundtrip
verify with/without dashes, primary password never verifies as app password,
cross-tenant/account denial, and a `constant_time.rs`-style timing test
proving unknown-user vs wrong-app-password minima are comparable).

Cuts/flags: none. Mechanism only by design — policy (which account may use an
app password on which protocol, 2FA fail-closed) is M1.2's seam; no routes
yet (M1.3), so no wire verification was due. A user with several app
passwords pays one argon2 verify per stored hash; timing can reveal at most
the enrolled count of a user already known to exist, never existence — noted
in the rustdoc.

Next: M1.2 — the legacy auth seam accepts app passwords.

### 2026-08-26 — iteration 2 — M1.2 legacy auth seam accepts app passwords

Shipped: `Identity::authenticate_legacy` (the one seam IMAP `LOGIN`, POP3
`USER`/`PASS`, SMTP `AUTH`, and CardDAV HTTP Basic all call) now tries the
presented secret against the user's app passwords when it is not an accepted
primary — so every legacy protocol gained app-password login in one place.
Order preserves the common path's cost: primary first (unchanged for the
non-2FA majority), then `verify_app_password`. A 2FA account's primary stays
refused exactly as before, and that refusal path *also* runs the app-password
check so "correct primary, policy-refused" pays the same argon2 work as
"wrong password" — the seam does not become a timing oracle for the primary.
2FA policy refusals still record no backoff strike; app-password failures do.
Call-site comments (imap session, pop3, smtp server) and
`docs/design/identity.md`'s app-password section updated to match.

Verified: fmt + clippy clean on alo-identity/alo-imap/alo-smtp; 272 tests
green across the three crates, incl. 4 new: identity seam test (non-2FA
primary + app pw both work, 6 primary refusals past the free-attempt budget
arm no backoff, revoked app pw fails next connection, principal is
scope-less and correctly tenant/user-bound), real-TLS IMAP (2FA primary NO,
app pw OK + SELECT INBOX, revoked NO), POP3 over TLS (primary -ERR, app pw
+OK on the same connection), SMTP submission STARTTLS AUTH PLAIN (primary
535, app pw 235). Wire evidence is those suites: real rustls sockets, real
Postgres rows.

Cuts/flags: none. No new routes (M1.3 owns `/api/settings/app-passwords`),
no XOAUTH2 (M1.4). Non-2FA accounts keep primary login byte-identical —
every pre-existing suite login is the regression test for that.

Next: M1.3 — owning app passwords from the product (routes + settings UI).

### 2026-08-26 — iteration 3 — M1.3 owning app passwords from the product

Shipped: route module `alo-jmap/src/app_passwords.rs` — `GET/POST
/settings/app-passwords` and `DELETE /settings/app-passwords/{id}` (mounted
under `/api` like every product route), all scoped to the token's
`(tenant, user)` and nothing else; the secret rides only in the create
response. Store refusals keep their meaning on the wire (422 bad name, 409
at the 20-per-user cap, 404 unknown-or-foreign id — same denial for both).
Web: `JmapClient.listAppPasswords/createAppPassword/revokeAppPassword`,
new settings tab "App passwords" (`web/src/shell/AppPasswordsSection.tsx`,
a new file on purpose — Codex is churning other shell/mail components) with
the one-time secret card (copy affordance + select-all fallback, dismissed
for good on Done), list showing name/created/last-used, immediate revoke
with per-row labels. Strings in en/fr/nl.

Verified: fmt + clippy clean; `cargo nextest run -p alo-jmap` 1313 green
incl. 5 new (secret shape + verifies at the identity seam, list never
carries it, 422s, revoke kills the credential + second revoke 404s,
cross-tenant list/revoke on one shared store denied with the credential
proven still alive, 401s without a token). Web: typecheck, eslint, vitest
(4 new section tests), `npm run build` — clean. Wire-verified against the
local debug backend on the `alo` database with real curl through the full
PKCE login: create 200 `{id,name,secret}`, empty name 422, list 200 with
the secret absent from the body, anonymous 401, revoke 200 then 404, list
empty after. Scratch tenant `wire-m13` created for it; the prune sweep
removes it on age.

Cuts/flags: none in scope. `cargo fmt` on the crate also normalized an
unformatted hunk someone landed in `ai.rs` — formatting only, kept so the
crate passes `fmt --check`. Pre-existing on main and NOT this track's to
fix: `web/src/shell/AppShell.layout.test.ts` fails because
`AppShell.module.css` is not in the tree (shell rework in flight by the
interactive agent this morning) — flagged here for whoever owns the shell.

Next: M1.4 — SASL XOAUTH2 on IMAP and SMTP submission.

### 2026-08-26 — iteration 4 — M1.4 SASL XOAUTH2 on IMAP and SMTP submission

Shipped: `alo-identity/src/xoauth2.rs` — the one XOAUTH2 blob parser both
protocols share (tolerant: extra `host=`/`port=` fields ignored, `Bearer`
case-insensitive, trailing `^A^A` optional; token redacted from Debug) and
`Identity::authenticate_xoauth2`, which verifies the bearer through
`resolve_access_token` — the same seam the RFC 7662 introspection endpoint
wraps (ADR 0025; rejected alternative: an HTTP hop to `/oauth/introspect`
from a process that already links the authority) — and requires the asserted
`user=` to resolve to exactly the token's `(tenant, user)`. No backoff/dummy
hash on this path by design (indexed lookups of a 256-bit token's hash,
nothing guessable; expired-token retries are legitimate) — rationale in the
rustdoc + `docs/design/identity.md`. IMAP: `AUTH=XOAUTH2` + `SASL-IR`
advertised post-TLS, `AUTHENTICATE XOAUTH2` with the mechanism's error
dialog on failure (`+ <b64 status>` → empty client ack → tagged NO); the
old try_login success/cap logic factored into shared `auth_success`/
`auth_failure` (behaviour unchanged). SMTP submission: `AUTH PLAIN LOGIN
XOAUTH2` after TLS, `do_auth_xoauth2` (334 error-status dialog → 535;
malformed blob 501; store fault 454); `Mechanism::XOAuth2` routed before
`collect_credentials`, whose unreachable arm fails safe as 504. The exact
SASL exchange (the base64 shape, the failure dialog, full IMAP + SMTP
transcripts) recorded in `docs/interop.md`; 2FA note: a token is only
issued after the full login, so accepting it does not weaken fail-closed.

Verified: fmt + clippy clean on alo-identity/alo-imap/alo-smtp; 281 tests
green across the three crates incl. 9 new — parser unit tests (canonical
shape, tolerance, rejections, Debug redaction, stable error-status JSON),
identity seam test (own principal OK + scope-less; cross-tenant/cross-user
and unknown-user refused; garbage token; revoked fails next connection),
real-TLS IMAP wire test (capability advertised, SASL-IR login reaches
SELECT INBOX, revoked token runs the error dialog with a decoded 401
status, malformed blob is BAD), real-TLS SMTP submission wire test (EHLO
advertises, live token 235, revoked 334-dialog→535, malformed 501). One
unrelated flake observed and cleared: `alo-smtp::dmarc_report sweep_…`
failed under full parallel load, passed alone and on the full rerun.

Cuts/flags: POP3 gets no XOAUTH2 (queue scopes M1.4 to IMAP + submission;
no client demand — app passwords cover it; noted in interop.md). No
feature flag: the mechanism is additive — a client only uses it by
selecting it — and the off-switch is revert; misbehaviour shows in the
existing auth success/failure tracing. OAUTHBEARER (RFC 7628) not
implemented, recorded in interop.md with rationale. M6.1's transcript
script should cover XOAUTH2 now that it exists.

Next: M2.1 — distribution lists.

### 2026-08-26 — iteration 5 — M2.1 distribution lists

Found built, kept: the mechanism largely predated this track — `groups` +
`group_members` (migration 0008), the list address (0012, globally unique,
lowercased), delivery fan-out in `alo-smtp`'s `local_delivery.rs`
(user/alias precedence, one copy per member through each member's own
Sieve, envelope recipient = the list address, memberless list = 550 at
RCPT), and admin CRUD on `/api/admin/groups*` (admin-gated,
`require_domain_owned` on the address). No new migration needed — the 09xx
slot stays for schema this track actually adds.

Shipped (what the ROADMAP line still owed): explicit wrong-tenant denials
and the proof. Store: `delete_group`/`set_group_address` now NotFound when
the group is not this tenant's (was a silent no-op reported as success),
`group_members`/`group_members_detailed`/`remove_group_member` assert the
group first (was an empty-vec answer indistinguishable from an empty
group); admin `DELETE /groups/{id}` + `POST /groups/members/remove` map
that to 404 (was a blanket 500 swallowing it). Loop-safety documented at
the seam it is enforced: members are users only (`assert_user`), so a list
can never contain a list and expansion is single-level by construction.
Design note `docs/design/local-delivery.md` gained the distribution-lists
section; CHANGELOG line added.

Verified: fmt + clippy clean on alo-store/alo-identity/alo-jmap/alo-smtp.
New tests — store `group_lists.rs` (wrong-tenant on EVERY operation with
the group proven untouched after each; A cannot enroll B's user; a group
id as member refused incl. self-containment; cross-tenant address
Conflict; expansion returns the owning tenant only; case-insensitive
match; clearing the address turns the list off; absent id NotFound), SMTP
`local_delivery.rs` (real-wire: one message → one copy per member with a
member's `envelope :is "to" <list>` Sieve filing it — proving the envelope
recipient members see is the list address; removed member stops receiving
on the very next message; memberless list 550 at RCPT). Full suites: 4 055
tests across the four crates, all green except two known parallel-load
flakes cleared alone — `site_ticket_orders` "…never offered to the
fulfilment sweep" (sites track's sweep timing) and `alo-smtp::dmarc_report
sweep_…` (the same flake iteration 4 recorded). alo-identity's
`groups_are_tenant_scoped` updated to the strengthened NotFound contract.

Cuts/flags: nested lists deliberately unsupported (the loop-safety design,
not a gap); external (non-user) list members out of scope — members are
tenant users; per-member DSN semantics unchanged (conservative 4xx,
documented). No new routes, so no Caddyfile note.

Next: M2.2 — shared-mailbox audit, then close what it finds.

### 2026-08-26 — iteration 6 — M2.2 shared-mailbox audit

Audited the whole delegate lifecycle as a user lives it, against a real
local stack (debug `alo-jmap` on the docker `alo` DB, two provisioned
users, everything driven over HTTP with curl) plus a code walk of every
enforcement seam.

Found working, verified live: no grant → `accountNotFound` (no oracle);
self-service grant/list/revoke on `/jmap/delegates`; the session
advertises the shared account (`isPersonal:false`, owner email as name,
`alo:canSend`); delegate reads the owner's folders and messages with live
unread counts; a canWrite delegate marking seen drops the owner's unread
count (read-state is shared — deliberate: keywords are per message, not
per viewer; Exchange-style per-user read state would be a redesign, noted,
not built); revocation bites on the very next request. Read-only,
folder-restricted, and no-send enforcement re-verified by the existing
`delegation.rs` suite.

Gaps found, then fixed:
1. **Send-later was delegation-blind** (proven live: a delegate scheduling
   a shared-mailbox draft got 404 while the UI offers "Send later" there).
   `/api/send-later` + `/send-later/cancel` now resolve an optional
   `accountId` through the same `resolve_target` door as JMAP — ungranted/
   cross-tenant ids stay 404 (no oracle), a read-only delegate cannot
   cancel (403), the send grant is enforced by the shared validation.
2. **The on-behalf `Sender:` could not survive scheduling** — the schedule
   row never recorded the acting delegate and the sweep re-reads the raw
   draft. Migration `0901` (expand-only) adds `on_behalf_sender`;
   `validate_and_prepare` now returns the acting delegate instead of
   rewriting bytes, both send paths prepend `Sender:` to the wire copy at
   send time, and the stored draft/Sent copy is never rewritten.
3. **First send from a fresh mailbox filed nowhere**: `post_send` gave up
   when no `sent`-role mailbox existed (fresh shared mailboxes typically).
   It now creates Sent on first use, as Drafts always was.
4. **Undo-send account race**: the web flush resolved the active account at
   flush time, so switching mailboxes inside the 5 s undo window re-
   targeted the submission. The account is now captured when the send is
   queued; `scheduleSend`/`cancelScheduledSend` pass the active account.

Sent-copy semantics recorded: a delegated send files into the OWNER's
Sent (the shared mailbox's, modern-Exchange style); the delegate's own
Sent stays empty; the disclosure `Sender:` is a wire matter only.

Verified: fmt + clippy clean (alo-store/alo-submit/alo-jmap); full suites
3 822 tests across the three crates green (two known parallel-load flakes
cleared alone: `site_schedule_http` publish sweep, `site_ticket_orders` —
the same class iterations 4–5 recorded). New `delegated_send.rs`: real-
wire sink proof that on-behalf discloses the delegate and send-as does
not, sent copy lands in owner's Sent (created on first use) and never in
the delegate's account, the scheduled path round-trips the disclosure
through the sweep, and the route door (ungranted 404, wrong-tenant 404,
no-send 403, read-only cancel 403, manage cancel returns the draft to the
owner's Drafts). Wire re-verified post-fix on the live stack: schedule
200 with `on_behalf_sender` in the row, bogus id 404, cancel 200. Web:
tsc, eslint, build clean.

Cuts/flags: per-viewer read state in shared mailboxes not built (design
note above); GUI-client passes stay owner-gated (M6 note). No new route
prefixes — `/api/send-later` already deployed. `alo` DB carries audit
tenant `m22-audit` (two users) from the live walk.

Next: M3.1 — CalDAV calendar collections.

### 2026-08-26 — iteration 7 — M3.1 CalDAV calendar collections

Found built, kept: the protocol surface predated this track —
`carddav.rs` already serves calendar collections (OPTIONS advertising
`calendar-access`, PROPFIND principal/home/collection/object, REPORTs
`calendar-multiget` + `calendar-query` with time-range + `sync-collection`
on the account modseq, GET/PUT/DELETE with field-hash ETags and RFC 7232
preconditions), one collection per visible calendar with `default` for the
personal one, all recorded in `docs/interop.md`.

Shipped (what the queue item still owed): the **round-trip corpus** and
the **per-method isolation proofs** — and the two real bugs writing them
surfaced. (1) `put_event` keyed its upsert by (tenant, user, id), so an
editor's CalDAV PUT replacing a colleague's event on a *shared* calendar
forked a per-user duplicate row of the same href instead of replacing it
(the Agenda path via `update_event` was fine); it now replaces in place,
keeping the row's owner. (2) A PUT/DELETE against a calendar the caller
cannot edit surfaced as a raw 500; it is now 403 when the calendar is
visible (read-only grant) and 404 when it is not — no oracle. New
`ical::to_ics_at(event, dtstamp)` seam pins DTSTAMP (the one property that
derives from nothing in the event) so byte-stability is provable; live
responses still stamp now.

Verified: fmt + clippy clean (alo-store, alo-jmap). New
`alo-store/tests/ical_corpus.rs`: five fixtures (plain UTC, all-day,
TZID=Europe/Brussels zoned, floating-read-as-UTC, §3.3.11 escapes) parse →
store through real Postgres → serialize to checked-in canonical bytes,
and the canonical form is a fixed point of a second full cycle; a folded
long-line fixture proves fold/unfold stability. New
`alo-jmap/tests/caldav.rs`: the full client sequence (discovery → PUT →
GET → PROPFIND → multiget → time-range query → incremental sync with
token advance → delete reported as 404 member → preconditions), plus
wrong-tenant AND wrong-user-same-tenant probes per method (via the
victim's paths and via own paths carrying the victim's ids — never data,
never a 500), and the read-only share: viewer reads, viewer PUT/DELETE
403, editor PUT replaces without forking. Full suites 3 816 tests:
3 815 green + the known `site_ticket_orders` parallel-load flake cleared
alone (the same class iterations 4–6 recorded). One environmental rerun:
a first suite run overlapped a killed predecessor on the same scratch DB
and produced 9 spurious billing failures with ~25-minute test times; all
passed on the clean rerun. Disk hit 100 %/2.9 GB free at gate start —
PDB + stale-binary sweep freed 20 GB (the LOOP.md playbook, again).

Cuts/flags: change-log visibility on shared calendars — an edit by a
delegate bumps only the editor's own account modseq, so another viewer's
sync token does not advance until their own account changes (the
account-wide-modseq cut interop.md already records; flagged for the M3
tail, not redesigned here). Floating/TZID-read-as-UTC stands (documented);
corpus grows recurrence + Europe/Brussels DST fixtures in M3.2. No new
routes, no migration, no web change — no Caddyfile note.

Next: M3.2 — recurring events with exceptions.

### 2026-08-26 — iteration 8 — M3.2 recurring events with exceptions

Adopted the killed predecessor's two uncommitted files (migration
`0902_calendar_event_recurrence_zone.sql`, `alo-jmap/tests/calendar_http.rs`)
and built the item around them.

Shipped: **DST-correct recurrence.** `CalendarEvent` gains `timezone` (IANA
zone the series' wall-clock follows) and `rdates` (RDATE extras); migration
0902 adds `tzid`/`rdates` (additive, existing rows keep UTC-fixed expansion).
New `alo-store/src/tz.rs` (zone lookup + wall↔UTC conversion, jiff-backed,
compatible disambiguation; `ical`'s TZID conversion now goes through it too).
`expand_occurrences` — still the one expansion function — runs the RRULE
period math on the zone's wall-clock and converts each occurrence back to the
UTC instant it lands on, so a 09:00 Brussels weekly stays 09:00 local across
the 2026-10-25 switch everywhere at once (Agenda range listing, availability,
CalDAV) — all-day events and unknown zones stay UTC-fixed. RDATEs expand
(deduped against the rule, EXDATE-cancellable, RDATE-only series work),
parse/serialize in `ical` (`VALUE=PERIOD` skipped, documented), and ride
every store write path. **iCal serving changed shape:** a zoned event now
serializes `;TZID=<zone>:<local>` (DTSTART/DTEND/EXDATE/RDATE/RECURRENCE-ID)
with **no VTIMEZONE** — the IANA name is the definition (interop.md records
the deviation and the owner-gated GUI check). CalDAV `time-range` now narrows
recurring masters through `series_occurs_in_range` (exported store seam over
the same expansion; overrides that move an instance into the window still
count) instead of the keep-if-started-before-end superset. Agenda API:
`timezone` on create/update/read, unknown zone refused 400 with the name
verbatim; a whole-series update now **preserves** stored EXDATEs/RDATEs/zone
the JSON body can't express — before this, editing a series from the UI
silently resurrected every cancelled instance (found-and-fixed under this
item's "exceptions" remit).

Verified: fmt + clippy clean (alo-store, alo-jmap). alo-store 2 505/2 506
green (the known `site_ticket_orders` parallel-load flake, cleared alone —
same class as iterations 4–7); alo-jmap 1 323/1 323. New coverage: tz unit
tests (IANA-only lookup, round-trip across both DST edges, gap-time
disambiguation), expansion tests (Brussels weekly + monthly-BYDAY across the
switch, unknown-zone degrade, RDATE dedupe/EXDATE/RDATE-only,
`series_occurs_in_range` on a spent series), ical tests (TZID capture +
refusal-to-capture on unknown zones, zoned fixed-point serialization, RDATE
round-trip + PERIOD skip), corpus grown per the queue item — weekly-with-
exceptions, monthly-by-day + RDATE, Europe/Brussels DST-crossing series —
and the zoned fixture's canonical form intentionally changed from flattened
UTC to the TZID wall-clock form (it was DST-wrong for recurring events).
HTTP: adopted `calendar_http.rs` proves create→list expands 07:00Z/08:00Z/
08:00Z across the switch and the verbatim 400; new caldav test proves the
zoned series round-trips on the wire and time-range narrowing keeps the
DST-crossing series while dropping a spent COUNT-bounded one.

Cuts/flags: no VTIMEZONE emit/parse (above); `RDATE;VALUE=PERIOD` skipped;
the JSON API exposes but does not accept `rdates` (they arrive via CalDAV);
no Agenda UI timezone picker yet (API + CalDAV carry the zone; UI is a
later slice — Codex is churning web/src/mail). Environment notes for later
iterations: Docker Desktop was down at gate start (started it, ~15 s to
docker, then Postgres up; not a HALT); the profile's `DATABASE_URL` has an
**empty password** and every DB test fails 28P01 with it — export
`postgres://alo:alo-dev-only@127.0.0.1:5432/alo_scratch` for the gate.

Next: M3.3 — invitations, iTIP over iMIP.

### 2026-08-26 — iteration 9 — M3.3 invitations, iTIP over iMIP

Found built, kept (the bulk of the item predated this track): outbound
`METHOD:REQUEST` on save (and a one-instance update carries `UID` +
`RECURRENCE-ID`), `METHOD:CANCEL` on delete (whole-series and one-instance
shapes), all through the one submission door (`crate::submission::submit` →
the internal listener), best-effort after the calendar write; the reading
pane's `Email/get` surfacing `alo:invitation` from the message's
`text/calendar` part; `InvitationCard` already wired to that parsed data —
Accept/Maybe/Decline → `/calendar/rsvp` (stores the event on the personal
calendar keyed on the organizer's UID and mails the `METHOD:REPLY`), the
reply card → `/calendar/apply-reply` (`set_attendee_status` merges the
guest's PARTSTAT onto the organizer's event), the cancellation card →
`/calendar/cancel`.

Shipped (what the item still owed): **a CANCEL naming one instance now
removes the instance, not the series** — `ical::recurrence_id_of` (a
standalone reader for the first VEVENT's `RECURRENCE-ID`; UTC, `VALUE=DATE`,
and `TZID` wall-clock shapes; `from_ics` deliberately keeps parsed masters'
`recurrence_id: None`, the CalDAV override-sync contract) and
`/calendar/cancel` routes through `exclude_occurrence` when the CANCEL names
a slot (an EXDATE on the recipient's stored series) and `delete_event` only
when it does not; the response gains an additive `scope:
"occurrence"|"series"`. And **the mandated round-trip proof**: new
`alo-jmap/tests/invitations_http.rs` drives the full arc across two accounts
on a real local stack — real routes, real Postgres, a real SMTP dialog into
an in-process multi-connection sink; the captured wire bytes are delivered
into the counterpart's mailbox and acted on through the same endpoints the
card calls. iMIP section added to `docs/interop.md` (read-time application
vs RFC 6047's on-arrival model, instance-CANCEL semantics, one-attendee
REPLY reading).

Found-and-fixed under this item's remit (the round-trip test caught it):
`Email/get`'s `alo:invitation` JSON never carried `attendee`/`partstat` —
the struct had them, the hand-built JSON dropped them — so the reply card
could never say WHO replied or how (the web type already declared both
fields; it always got undefined). Additive JSON fix in `jtypes.rs`.

Verified: fmt + clippy clean, zero warnings (alo-store, alo-jmap). New
tests — ical unit (`recurrence_id_of` UTC/`VALUE=DATE`/`TZID` shapes +
absent), REQUEST→REPLY round trip (invite → sink → deliver →
`alo:invitation` REQUEST → decline first (added:false, REPLY sent, event
absent) → accept (added:true) → REPLY wire → deliver → reply card data
(attendee + partstat) → apply → PARTSTAT=ACCEPTED on the organizer's
event), CANCEL (weekly COUNT=4 accepted by the guest; organizer cancels the
2nd Monday → guest applies → `scope:occurrence`, series survives, range
listing shows exactly the other three Mondays; whole-series delete →
`scope:series`, event gone; re-applying the same CANCEL is `removed:false`,
not an error), and tenancy (wrong tenant AND wrong user-same-tenant get 404
on rsvp/cancel/apply-reply against a foreign blob, with the owner's own
RSVP proven alive after). Full suites: 3 833 tests across both crates —
3 832 green + the known `site_ticket_orders` parallel-load flake, cleared
alone (same class iterations 4–8 recorded). Wire evidence is the suite
itself: a real SMTP dialog into the sink, real Postgres rows, the captured
bytes re-delivered and acted on. Ops note: one healthy nextest run was
killed mid-flight after a misread process check (no test binaries visible ≠
stuck — the runner's own CPU stays near zero while binaries run); the
tell that it IS alive is the growing log, not the process table.

Cuts/flags: inbound application stays read-time (the card acting on an
account-scoped blob), not delivery-time — automatic on-arrival processing
would put iMIP logic in alo-smtp as a second implementation; recorded in
interop.md, revisit only if a real-client pass demands it. REPLY reads the
first ATTENDEE only (a REPLY speaks for one attendee per RFC 5546). No web
change was needed — the card was already wired end to end, so no i18n
additions. No new routes, no migration — no Caddyfile note.

Next: M3.4 — free/busy (VFREEBUSY).

### 2026-08-27 — iteration 10 — M3.4 free/busy (VFREEBUSY)

Shipped: the CalDAV `free-busy-query` REPORT (RFC 4791 §7.10) on every
calendar collection, answered as an RFC 5545 §3.6.4 `VFREEBUSY` —
`text/calendar`, the queried window as `DTSTART`/`DTEND`, one
`FREEBUSY;FBTYPE=BUSY` UTC period per busy span. Three seams, each in the
file that owns its subject: `alo_store::merged_busy_spans`
(calendar_availability.rs — clamp to the window, drop empties, merge
overlapping/touching spans; the existing JSON `/calendar/freebusy` endpoint
now calls it too, so "busy" is computed exactly once),
`ical::to_vfreebusy` (a serializer whose type has no field for
titles/locations/descriptions — busy/free only, by construction), and the
REPORT dispatch in carddav.rs (visibility gate identical to PROPFIND: an
unshared or foreign calendar id is 404, unprobeable; missing/invalid
`<C:time-range>` is 400 per RFC 4791 §9.11's exactly-one rule). Instances
come from the store's ONE expansion function (`events_in_range`), so
recurrence, moved occurrences, and EXDATEs are honoured with no second
implementation. `free-busy-query` advertised in supported-report-set.

Verified: fmt + clippy clean (alo-store, alo-jmap); full suites 3 838 run,
3 838 passed, 1 skipped (the standing serial skip), including the new
tests — merge unit tests (clamp/merge/sort, empty/disjoint), VFREEBUSY
byte-exact serialization, and two wire tests over the real router +
Postgres: merged-overlap + weekly-series expansion + outside-window
exclusion + no-SUMMARY-anywhere, and the MANDATED cross-account proof — a
viewer-role share yields busy periods but never the title, A's unshared
personal calendar 404s to a colleague, and a foreign tenant gets 404
through both path shapes with no `FREEBUSY` in any denial body.

Cuts/flags: `TRANSP` is not modelled — every event counts as busy
(transparent all-day events still block); recorded in interop.md, revisit
if a real-client pass objects. iTIP `VFREEBUSY` REQUEST/REPLY over iMIP
(RFC 5546 §3.3) not built — the queue item asks for the queried-window
answer, and no client in our interop set schedules via iMIP free/busy. No
web change (the Agenda's scheduling grid already rides the JSON endpoint,
which now shares the merge). No migration, no new route prefix (rides
`/dav`), no Caddyfile note. Ops note: the background+marker build form
lost its marker subshell when the launching Bash call ended (Windows: the
`( … ) &` parent dies with the call; cargo itself survived) — the build's
"Finished" line in the log plus a fast foreground `--no-run` re-check is
the honest completion signal.

Next: M4.1 — German catalog (`web/src/i18n/de.ts`).

### 2026-08-27 — iteration 11 — M4.1 German catalog, tranche 1 (mail surface)

Shipped: `web/src/i18n/de.ts` — German at native quality (Siezen, „…“
quotes, the type-name rule kept), registered end to end: `Locale` gains
`"de"`, the switcher row (Deutsch), the catalog overlay, storage-key
acceptance, and browser detection (`de` prefix → German) in `locale.ts`.
The queue item's own scaling rule applied: the full surface is 5 182 keys,
so this iteration ships the first **complete-modules tranche** (~770 keys)
— the mail daily-driver surface: brand + all module rail labels, Home,
shell/app launcher, Contacts, IMAP import wizard, auth (sign-in,
two-factor, errors, signup, password reset), Agenda incl. sharing/
reminders/availability, Tasks, Mail entire (list, reading pane, compose,
folders, delegation + app passwords + per-folder access, categories,
Transfer, filters, spam banner, one-click unsubscribe, send/attach error
details), and Mail settings. Wording notes: DMARC/SPF spam-banner
sentences are both phrased with dative "von" so the shared
`spamSenderFallback` declines correctly in each; "Re:"/"Fwd:" prefixes
kept (Gmail-de convention, and AW:/WG: breaks cross-client threading).

Verified: `locale.test.ts` grows a German block — every de key is a real
en key, every interpolation keeps arity, no empty strings, spot-checks
prove real German incl. plural branches, the fallback shows English for a
not-yet-shipped surface, and a **per-module ratchet**: every en key in the
shipped prefix families must exist in de, so a new mail-surface English
key now fails the build without German (the nl/fr UNTRANSLATED ratchet is
untouched — de joins it only when the full surface is done). 71/71 i18n
tests green; `npx tsc --noEmit`, eslint on changed files, `npm run build`
all clean. No Rust, no routes, no migration — web-only, additive.

Cuts/flags: remaining surfaces for later tranches (in queue order of
value): Docs/Drive/Spaces, Chat/Meet, Search, admin console + control
plane, and the business modules (billing/CRM/insights/projects/finance/
inventory/HR/campaigns — each has its own "fully translated" test to join
when its German lands). Meet's caption-language picker hardcodes its own
en/fr/nl union (live-translation feature, not catalog) — left alone.
M4.1 stays `[ ]` on purpose: the tranche note under the item records
progress, and the next iteration continues the surface.

Next: M4.1 tranche 2 — the item stays first in the queue until the
surface is complete.
