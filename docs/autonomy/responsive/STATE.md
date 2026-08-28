# alo responsive layout — loop journal

Track opened 2026-08-28 from a measured audit of both live apps at
360/768/1024/1440 px: the shell is responsive everywhere, Tasks/Projects is
broken on a phone, and four other modules are rough there because no module
except Mail and Chat ever adopted the phone pattern. Queue: R1 fix the broken
screen, R2 make the pattern a design-system contract and adopt it, R3 turn the
audit into a real-browser test.

Standing facts every iteration should know:

- Do not touch `#root`/shell clipping — the page-scroll invariant is tested and
  deliberate.
- Codex is active in `web/src/billing/**` and `web/src/ds`; prefer new files,
  rebase early, keep both sides of additive i18n conflicts.
- This track runs in its own checkout. If any other editor's uncommitted work
  appears in the tree, halt rather than build over it.
- `.github/` and `deploy/` are off-limits; CI wiring for R3 is a human
  handover recorded here.

## Iterations

### 2026-08-28 — iteration 1 — R1 Tasks/Projects on a phone

Shipped: `TasksModule` adopts the MailModule folder-drawer pattern via
`useIsMobile` from `web/src/ds`. Below the mobile breakpoint the project
sidebar (My plate / Suggestions / project list) is an off-canvas drawer:
hidden by default, toggled by a `PanelLeftOpen/Close` IconButton at the head
of the module header, laid over the content with a dimmed backdrop
(`z-overlay`, same width formula as mail's drawer: `min(82vw, 20rem)`), and
closed by picking My plate, Suggestions, a project, or the backdrop. The
desktop/tablet branch keeps the exact original class string
(`w-60 … max-md:w-52 max-sm:hidden`), so ≥1024 px is pixel-for-pixel
unchanged — verified: sidebar renders at 240 px with no toggle button. New
i18n keys `taskShowProjects`/`taskHideProjects` in all four catalogs
(en/fr/nl/de — the parity test requires all four, `untranslated.ts` stays
empty).

Verified: tsc, eslint (changed files), `npm run build`, and
`vitest run src/tasks src/projects src/i18n src/ds` — 238 tests green. The
real-browser check ran in the cached Playwright Chromium against an
uncommitted fixture harness (real `TasksModule` + `AuthProvider` with a
stubbed `fetch`, Vite on 5178, deleted after the run): 15/15 checks pass at
360×740 and 1024×768 — sidebar hidden by default at 360, opens on the
toggle, closes on project selection and on backdrop tap, empty-state CTA on
one line inside the viewport (w=201 px), zero horizontal overflow (doc
scrollWidth = 360), no element wider than the viewport outside intentional
scroll strips. Why a harness and not the wire stack: Codex's live stack
holds 5173/8080 (their vite + alo-jmap from `C:\dev\Ficina`), and LOOP says
hand 5173 over rather than run a second server beside it; R1 changes no
HTTP route, so the layout check needs a real layout engine, not a real
backend.

Cuts/flags:
- Escape-to-close and a focus trap are NOT in this slice — mail's own drawer
  has neither today. R2's `ModuleSidebar` contract adds both in one place and
  MailModule + Tasks adopt it.
- `useIsMobile` treats exactly 768 px as mobile (inclusive) — the established
  mail/chat convention; the queue's "below 768 px" differs by that one pixel.
- Housekeeping: this checkout's `web/node_modules` was missing `qrcode.react`
  (declared in package.json by the billing stream) — `npm install` fixed it;
  tsc is otherwise not clean without it.

Next: R2 (lift the drawer into `web/src/ds/ModuleSidebar`, adopt in mail,
CRM, Inventory, Insights, Chat — one module per commit).

### 2026-08-28 — iteration 2 — R2 the phone layout as a ds contract

Shipped, one module per commit (seven commits):

- `ds/ModuleSidebar` (+ 8 vitest tests): renders its children untouched at
  desktop widths; at ≤768 px it is the off-canvas drawer — mail's geometry
  (left edge, `min(82vw, 20rem)`, laid over the content, backdrop at
  `z-overlay−1`) plus the Modal's focus trap, Escape-to-close and
  focus-give-back, which neither module copy had. Design note: the child is
  the fully styled column and fills the drawer; the drawer owns position,
  width, backdrop and keyboard. Rejected alternative: keep per-module drawer
  classes — that is exactly how two copies shipped without Escape or a trap.
- Mail adopts it (inline drawer + `.drawer`/`.drawerOverlay` CSS deleted;
  `!w-full` on `FolderSidebar` inside the drawer since its own width is
  `--sidebar-width`).
- Tasks adopts it (R1's inline drawer replaced).
- CRM: on a phone the board renders ONE full-width column picked by a
  `ds/Select` labelled with the existing `crmStage` key (counts in the option
  labels); drag still reorders within the column, cross-stage moves go
  through the deal drawer's own stage select (verified present). Desktop
  identical (media query + `visibleStages`).
- Inventory: `.tabs` becomes a horizontal scroll strip at ≤48rem; `.search`
  takes its own toolbar row and `.toolbar > button` share a full-width row.
- Insights: `.boardBar` wraps at ≤48rem, the board name takes its own line —
  "Ask for a chart" no longer clipped.
- Chat: `ConversationHeader` compacts by the `mobile` prop it already
  received (tighter paddings, 44 px controls, `#` glyph and dividers
  dropped) so the room title keeps ~88 px at 360. Chose prop conditionals
  over `max-md:!size-11` after measuring that a media-variant `!important`
  does not reliably beat a base `!important` utility in this Tailwind build.

Verified: `tsc`, eslint on changed files, `npm run build`, full
`vitest run` (231 files / 1286 tests green), and a real-browser pass —
uncommitted fixture harness (`web/e2e-tmp/`, deleted after the run): real
modules + `AuthProvider`/`DialogProvider`/`MemoryRouter`, network stubbed at
`window.fetch` (JMAP session + batch answers, CRM/chat/insights/inventory
canned data), Vite on 5178, driven by the npx-cached Playwright Chromium.
36/36 checks at 360×740 and 1024×768: drawers hidden by default, open on
toggle, Escape/backdrop close, focus trapped and returned; CRM one column at
360 and four at 1024 with picker swap working; Inventory strip scrolls with
the last tab reachable; Insights controls all inside the viewport; chat
title 88 px wide; zero horizontal document overflow everywhere.

Cuts/flags for later items:
- `Modal` and `ModuleSidebar` each carry the same focus-trap ~30 lines;
  unify into a shared `useFocusTrap` once ds churn calms (Codex is active in
  `web/src/ds`).
- Chat's DM header uses plain (non-`!`) `max-md:` compaction — works, but
  is untested by the harness; R3's sweep will cover it.
- R3 must allow-list Inventory's `.tabs` strip as an intentional horizontal
  scroller (the planned data attribute is not added yet).
- Harness learning for R3: the push-stream stub must HANG OPEN —
  `useChatFeed`'s reconnect loop backs off only on failure, so a stream that
  ends successfully spins the page's main thread (an hour of debugging).
- Mail's drawer toggle on mobile still shows the collapse/expand label pair
  (`collapseFolders`/`expandFolders`) — pre-existing, cosmetic, left as is.

Next: R3 (Playwright as a dev dependency, `web/e2e/responsive.spec.ts`
against a local stack, `npm run test:responsive`; CI wiring is a human
handover to be recorded here).

### 2026-08-28 — iteration 3 — R3 the audit as a test that never needs a human

Shipped: `@playwright/test` 1.62.1 as a `web` devDependency (pinned to the
Chromium already in this machine's Playwright cache — `chromium-1234` — so
the gate downloads nothing), `web/e2e/` with the suite and its stack, and
`npm run test:responsive`. The suite brings its OWN stack rather than
touching anyone's: global setup creates a throwaway database `alo_e2e` on
the dockerised postgres (dropped again in teardown, FORCE both ways — the
one-database rule's test-suite clause), runs the debug `identityctl` to
migrate + seed a tenant/admin and register the `web` OAuth client for
`http://localhost:5199/auth/callback` (5199, not 5173: the LOOP's
one-port doctrine protects the shared dev database's registration — this
registration lives only in the throwaway database and dies with it), then
spawns the debug `alo-jmap` on 8199 and Vite on 5199 with `VITE_DEV_API`
pointed at it. The spec signs in through the real login form (form POST →
PKCE code → token) and walks all 16 workplace modules at 360/768/1024/1440,
asserting: no horizontal document scroll, no element wider than the viewport
unless inside `[data-allow-overflow]`, and for the ModuleSidebar modules
(mail, tasks) drawer-closed-by-default + toggle present at phone widths and
no drawer at desktop widths. One test per viewport walking all modules in
one signed-in page — a fresh context per module would replay the rotated
refresh token into the server's replay-chain revocation. Failures are soft
asserts with a per-module screenshot in `web/e2e/.artifacts/`.

What the sweep found and this iteration fixed (all at 360 px unless said):
- Finance and HR tab strips ran off-screen — both now scroll horizontally
  at ≤48rem (Inventory's R2 pattern) and carry `data-allow-overflow`.
- The Projects list header's actions kept a one-line width (`shrink-0`) and
  overflowed; now `min-w-0` so its declared wrap actually happens.
- Intentional scrollers marked: ds `Table`'s scroll region (one attribute
  covers Finance and every other adopter), the Projects list's wide-table
  wrapper (also at 768/1024), Inventory's eight `.tableWrap` uses and its
  tab strip, HR's and Finance's strips.

Verified: `npm run test:responsive` 4/4 green (final confirming run on the
exact committed tree), `npm run typecheck`, `tsc -p e2e/tsconfig.json`,
eslint on all changed files, `npm run build`, full `vitest run` 231 files /
1286 tests green. e2e teardown verified: `alo_e2e` absent from pg, 5199/8199
released.

Cuts/flags:
- **Human handover (CI):** the workflow step R3 cannot wire itself is:
  build the debug binaries (`cargo build -p alo-jmap -p alo-identity
  --bins`), have a postgres service named `alo-pg` with user/db `alo`
  password `alo-dev-only` (or adjust `web/e2e/stack.ts`), install the
  Playwright version pinned in `web/package.json` with its Chromium, then
  `npm run test:responsive` in `web/`. Chromium download IS needed on a
  cold CI runner — the no-download promise is about this machine's cache.
- Boundary note: the sweep's fixes touched `web/src/finance`, `web/src/hr`
  and one attribute in `ds/Table.tsx` — outside the modules R1–R2 name, but
  exactly the "screen it fails on, fixed in the same commit" R3 orders, and
  each is additive-minimal (an attribute, a media query). Codex churns
  `web/src/ds`; the Table diff is one attribute to keep a rebase trivial.
- The lazy-module Suspense skeleton is 384 px wide, so it overflows a
  360 px phone while a chunk loads — transient, shell-clipped, not asserted
  against (the spec waits it out); a cosmetic item for a future wave.
- The spec saw a stray 404 in the console on module walks (resource-level,
  no visible effect) — not chased this iteration.
- `/admin` and `/control` are operator consoles, not modules; the sweep
  skips them by decision, recorded here rather than silently.

Queue complete: R1, R2, R3 all `[x]`.

LOOP COMPLETE
