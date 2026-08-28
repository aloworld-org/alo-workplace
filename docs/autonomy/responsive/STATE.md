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
