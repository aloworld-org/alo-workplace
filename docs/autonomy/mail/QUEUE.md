# alo Mail — build queue

Everything left on the mail roadmap that a loop can build. The engine is done
and live: SMTP + trust stack, JMAP on 443, IMAP/POP3, CardDAV, filters,
out-of-office with scheduling, per-identity signatures, campaigns' store side.
MAPI is retired by decision, not deferred
([ADR 0056](../../decisions/0056-our-own-client-on-443-is-the-product.md)) —
nothing here may resurrect it. What remains is the tail that separates "works"
from "a business runs on it", and each item below maps to an open line in
`ROADMAP.md` — this queue adds no scope of its own.

## What this queue does not contain, and why

- **The two-week daily-driver gate** — the owner's habit, not a build.
- **Deliverability warming** — earned by sending history over weeks; no code.
- **GUI-client interop passes** (real Thunderbird/Apple Mail clicked by hand) —
  need a human at a screen. M6 builds the protocol-level transcripts; the
  GUI half stays owner-gated.
- **Deploys.** The loop builds, tests, commits, pushes. Production is the
  human's, always (hard rails in LOOP.md).

## Ground truth before building

- Codex (another agent, its own checkout) is actively reworking `web/src/ds/**`,
  `web/src/billing/**`, `web/src/chat/**` and touching some
  `web/src/mail/components/*`. The queue is therefore ordered backend-first;
  where an item must touch `web/src`, rebase early, keep both sides of
  additive i18n conflicts (LOOP.md rule), and prefer new files to edits of
  files that agent is churning.
- Migrations for this track are **`09xx`**, append-only, expand-only.
- A 2FA account on IMAP/POP3 **fails closed today by design** — the sanctioned
  cut seam recorded in the identity milestone. M1 closes it properly; do not
  weaken the fail-closed behaviour to pass a test.
- `alo-dav` already speaks CardDAV (RFC 6352) end to end: OPTIONS/PROPFIND/
  REPORT(multiget + sync-collection)/GET/PUT/DELETE. It is the pattern for the
  CalDAV half — read it before writing any WebDAV plumbing twice.
- `web/src/mail/components/InvitationCard.tsx` exists; `alo-jmap` has calendar
  routes and the store has calendar tables (Agenda slice 1). M3 builds on
  those, it does not restart them.
- The MX refuses `bounces@news.alomails.com` today with a clean 550 — the
  domain is not in `ALO_SMTP_LOCAL_DOMAINS`. That refusal is correct until
  M4.4 gives bounces somewhere to go.

## Areas this track owns

`platform/alo-identity/**` (app passwords, XOAUTH2), `platform/alo-dav/**`,
mail-side `platform/alo-store` modules (`account_*`, `settings`, calendar,
delivery) with migrations `09xx`, `products/mail/**`, `web/src/mail/**`,
`web/src/i18n/**` (additive keys + the new `de.ts`), and for M5 only:
`web/index.html`, `web/public/**`, a new `web/src/pwa/**`. It reads and never
restructures billing/chat/ds/sites code.

## The queue

### M1 — Legacy-client access for 2FA accounts

- [x] M1.1 App-specific passwords in the store + identity: per-user, named
  ("Thunderbird on the desk machine"), generated server-side (CSPRNG, shown
  once, never retrievable), argon2id-hashed with the same parameter contract
  as account passwords, revocable one at a time, migration `09xx`. Verify is
  constant-time through the existing dummy-hash seam so an unknown user and a
  wrong app password are indistinguishable by timing — prove it with the same
  style of timing test the identity crate already carries. Wrong-tenant AND
  wrong-user tests on every operation.
- [x] M1.2 The legacy auth seam accepts them: IMAP/POP3 `LOGIN` and SMTP `AUTH`
  try app passwords for accounts that have them; a 2FA account's *primary*
  password stays refused on legacy exactly as today — the whole point is that
  a phished primary cannot bypass 2FA over IMAP. Tests: 2FA account + app
  password succeeds on real TLS IMAP; 2FA account + primary password still
  fails; revoked app password fails on the next connection; a non-2FA account
  behaves exactly as before.
- [x] M1.3 Owning them from the product: `/api/settings/app-passwords`
  (create/list/revoke; the secret appears only in the create response) with
  wrong-tenant tests per route, and a settings-screen section — shown once on
  create with copy affordance, list shows name + created + last-used, revoke
  is immediate. Strings in en/fr/nl from day one.
- [x] M1.4 SASL `XOAUTH2` on IMAP and SMTP submission, verifying our own OIDC
  bearer tokens through the existing introspection seam (ADR 0025) — so a
  client that can do OAuth never needs an app password at all. Record the
  exact SASL exchange in `docs/interop.md` (the base64 shape trips every
  implementer). Tests over real TLS: valid token in, revoked token refused.

### M2 — Groups of people as mail destinations

- [x] M2.1 Distribution lists: a tenant-level list address that delivery
  expands to member mailboxes. Store model + migration (`09xx`), expansion in
  the delivery path (one inbound message → one copy per member through the
  normal Sieve/delivery pipeline), loop-safe (a list containing a list
  terminates; a cycle is refused at membership-write time, with a test), and
  admin CRUD over `/api/admin/*` gated exactly as other admin routes. Tests:
  wrong-tenant on every operation, a member leaving stops delivery on the next
  message, the envelope recipient a member sees is the list address.
- [x] M2.2 Shared-mailbox audit, then close what it finds: delegation exists
  (ADR 0017: none/as/on-behalf) — audit the full path as a user would live it
  (grant, read, send-as, where the sent copy lands, unread counts) against a
  real stack, record findings in STATE.md, then fix the gaps found and test
  each. Do not redesign ADR 0017; finish it.

### M3 — Calendar interop (the minefield; budget several iterations)

- [x] M3.1 CalDAV calendar collections (RFC 4791) over the existing calendar
  store, mirroring the CardDAV implementation's shape: OPTIONS/PROPFIND,
  `calendar-query` + `calendar-multiget` + `sync-collection` REPORTs, GET/PUT/
  DELETE with ETags. iCalendar (RFC 5545) serialization gets a **round-trip
  corpus**: every fixture parses → stores → serializes byte-stable. Start the
  corpus with plain events, all-day events, and UTC/zoned/floating times
  (jiff owns tz math). Wrong-tenant and wrong-account tests per method.
- [x] M3.2 Recurring events with exceptions: RRULE/RDATE/EXDATE/RECURRENCE-ID
  expansion, correct across DST boundaries — the corpus grows fixtures for
  weekly-with-exceptions, monthly-by-day, and a Europe/Brussels DST crossing.
  Expansion is one function used by both CalDAV time-range queries and the
  Agenda UI, never two implementations.
- [x] M3.3 Invitations, iTIP over iMIP (RFC 5546/6047): sending a REQUEST when
  an event gains attendees (through the one submission door — the internal
  listener), parsing REQUEST/REPLY/CANCEL on inbound mail into attendee state
  on the stored event, and the Mail reading pane's existing `InvitationCard`
  wired to real parsed data with Accept/Decline/Tentative that send the REPLY.
  Every deviation a real client forces goes in `docs/interop.md`. Tests: full
  request→reply round trip across two accounts on a real local stack;
  a CANCEL removes the instance, not the series, when it names one.
- [x] M3.4 Free/busy: `VFREEBUSY` answered from the store (RFC 5545 §3.6.4)
  for a queried window, exposing busy/free only — never event detail — with a
  cross-account test proving titles cannot leak through it.

### M4 — Platform polish (small, high-value, mostly backend)

- [ ] M4.1 German catalog: `web/src/i18n/de.ts`, the entire key surface at
  native quality (the bar French set), registered in `locale.ts` + the
  switcher. Additive-only against en/fr/nl. If the surface is too large for
  one iteration, ship complete modules per iteration (the fallback mechanism
  makes partial catalogs safe) and record the boundary in STATE.md.
  - Tranche 1 shipped 2026-08-27 (iteration 11): `de` registered end to end;
    the mail daily-driver surface translated (~770 of 5 182 keys) and
    ratcheted per shipped module in `locale.test.ts`.
  - Tranche 2 shipped 2026-08-27 (iteration 12): Docs (block editor,
    technical authoring, formatting toolbar), Drive + Spaces, Sheets, the
    Office embed, the search overlay and the Drive picker (+562 keys →
    1 348 of 5 182), all families joined the ratchet. Remaining: Chat/Meet,
    admin + control plane, and the business modules — boundary in STATE.md.
- [ ] M4.2 Server-synced locale preference: a per-user setting (existing
  `user_settings` row, migration `09xx` if a column is needed), read at login,
  written when the switcher changes, browser detection stays the fallback for
  anonymous pages. Test the round trip through the real API.
- [ ] M4.3 The mail surface's login screen says alomails, not "workspace":
  audit the brand strings on the login/signup/reset pages under
  `ALO_PRODUCT=mail`, fix what still says workspace, and keep the workspace
  surface's own copy intact (the `@product` seam decides, not a hardcode).
- [ ] M4.4 The campaign return path: accept `bounces@news.alomails.com` for
  delivery (config-driven, not hardcoded — the domain list is deployment
  config), parse DSNs (RFC 3464) into hard/soft verdicts, and fire hard
  bounces into the campaign suppression store (C1.3's seam). Tests: a
  fabricated DSN suppresses; a soft bounce does not; a non-DSN message to the
  address is stored, not crashed on. The production `.env` change is the
  human's deploy step — record it in STATE.md as the handover.
- [ ] M4.5 The Ed25519 DKIM second signature: dual-sign (rsa + ed25519,
  RFC 8463) in the signer behind config, `--install-dkim-key` handles the
  second selector, and the DNS record the owner must publish is printed and
  recorded in STATE.md. The key order rule stands: the record is published
  BEFORE the key signs — so the code path ships dark, with a test proving
  dual-signing produces two valid signatures against fixture keys.

### M5 — PWA (the one frontend-heavy block; last on purpose)

- [ ] M5.1 Installable: web manifest per product surface (alomails / alo
  workplace names + icons), served + linked, passing Chromium installability.
  No behaviour change beyond installability.
- [ ] M5.2 Offline shell: a service worker that precaches the app shell and
  shows an honest offline screen — mail data offline is **deliberately out of
  scope** (a sync engine is not a queue item; write the cut into STATE.md).
  The worker must never cache API responses or auth redirects; a stale-shell
  bug is worse than no PWA, so the worker updates on new deploys and a test
  covers the version-bump path.
- [ ] M5.3 Push: Web Push (VAPID) endpoints + subscription store (migration
  `09xx`, per user+device, wrong-tenant tested) fed from the existing JMAP
  PushHub, per-user opt-in in settings, payload carries counts and ids only —
  never subject lines or bodies (the notification fetches on tap). The server
  half is testable end to end locally; the browser-permission half gets a
  structural test plus a STATE.md note for the owner's manual check.

### M6 — Interop evidence, the half that needs no human

- [ ] M6.1 Scripted wire transcripts against a full local stack, recorded in
  `docs/interop.md`: IMAP (LOGIN/SELECT/FETCH/STORE/IDLE), POP3, SMTP
  submission incl. 8BITMIME and SMTPUTF8, CardDAV sync, CalDAV once M3.1
  lands, XOAUTH2 once M1.4 lands. Each transcript is generated by a committed
  script (so it can be re-run after any change), trimmed to the meaningful
  exchange, and dated. The GUI-client passes (real Thunderbird / Apple Mail /
  Gmail-app) remain owner-gated and are listed in STATE.md as the handover.
