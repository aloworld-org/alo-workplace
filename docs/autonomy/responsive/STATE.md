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
