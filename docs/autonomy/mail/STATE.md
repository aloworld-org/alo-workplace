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
