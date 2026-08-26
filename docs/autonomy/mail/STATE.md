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
