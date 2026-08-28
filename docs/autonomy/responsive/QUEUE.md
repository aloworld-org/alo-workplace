# alo responsive layout — build queue

Every screen, every size. The shell already does its half: at ≤768 px the rail
becomes a bottom bar and the document never scrolls (`#root` clips; the shell is
a containing block). What it cannot do is collapse a *module's own* sidebar or
board, and only two modules — Mail and Chat — ever adopted the phone pattern
(`useIsMobile` from `web/src/ds`). The rest lean on the shell and break on a
phone. This queue closes that gap once, as a contract, and keeps it closed with
a test that runs in a real browser.

## Ground truth (measured 2026-08-28 on the live apps, 360/768/1024/1440 px)

- **Zero horizontal document overflow anywhere** — the shell-level invariant
  holds. Do not touch `#root`/shell clipping; it is tested in
  `web/src/shell/AppShell.layout.test.ts`.
- **Broken at 360 px: Tasks/Projects.** The projects sidebar never collapses;
  the content column is ~130 px, the empty-state copy wraps word by word, and
  the *Create your first task* button overflows its own label. Fine at 768 px.
- **Rough at 360 px:** CRM board (one pipeline column visible, the rest cut),
  Inventory (tab strip runs off-screen, Scan + New product stack badly),
  Insights (header actions wrap; "Ask for a chart" clipped), Chat (room header
  loses its title). Fine at ≥768 px.
- **Modules with no phone breakpoint of their own:** tasks, projects, billing,
  drive, meet, insights, crm, finance, hr, inventory. Billing/Drive/Finance/HR/
  Meet happen to be single-pane and survive; the ones with a sidebar or a board
  do not.
- **The pattern to generalise exists:** `web/src/mail/MailModule.tsx` — below
  768 px the three panes become list↔detail, the folder sidebar becomes an
  off-canvas drawer toggled from the list header and closed on selection. That
  is the behaviour, already used daily on the live mail surface.
- **No browser-level test tooling in the repo** — `web/` is vitest + jsdom
  only, and jsdom computes no layout. Playwright drives Chromium locally (the
  browsers are cached on this machine); adding it is R3's job.
- **Another agent (Codex) is active in `web/src/billing/**` (270 file-touches
  in 24 h), `web/src/i18n`, `web/src/ds` (18) and lightly in `web/src/tasks`
  and `web/src/projects`.** Prefer NEW files; rebase early; keep both sides of
  additive i18n conflicts. Anything that must edit a file Codex churns is done
  in the smallest possible diff.
- Three earlier tracks halted because a second agent edited their working
  tree. This track runs in its own checkout (`C:\dev\Ficina-mail`, idle since
  the mail queue completed) — one agent per tree, no exceptions.

## Areas this track owns

`web/src/ds/ModuleSidebar*` and `web/src/ds/useIsMobile*` (new/extended
primitives only — no rework of existing ds components), the sidebar/board
layout files of the modules named in R1–R2, `web/e2e/**` (new), a `web`
npm script for R3, and `web/src/i18n/**` for any additive key. It never edits
`.github/`, `deploy/`, `web/src/billing/**`, or the shell's clipping rules.

## The queue

- [x] R1 Tasks/Projects on a phone: below 768 px the projects sidebar becomes a
  drawer (toggle in the header, opens over the content, closes on selection —
  the exact behaviour `MailModule` has for folders), and the content column
  gets the full width. The empty state and its call-to-action fit at 360 px
  without wrapping the button label. Desktop and tablet unchanged, pixel for
  pixel. Test: a real-browser check at 360 px that the sidebar is hidden by
  default, opens on the toggle, closes on selecting a project, and that no
  element in the content column is wider than the viewport; and the existing
  tasks/projects vitest suites still green.
- [x] R2 The phone layout as a design-system contract: `ModuleSidebar` in
  `web/src/ds` — a sidebar that is a column at ≥768 px and a drawer below,
  with its toggle, backdrop, focus trap and Escape-to-close, built by lifting
  the mail module's implementation rather than writing a second one (then
  `MailModule` adopts it too, so there is one implementation). Adopt it in
  CRM (the board additionally collapses to one column with a stage picker on
  a phone), Inventory (tab strip scrolls horizontally instead of running off;
  actions wrap into one row of full-width buttons), Insights (header actions
  stack cleanly; nothing clipped), and Chat (the room header keeps its title
  at 360 px). One module per commit. Every adoption is verified at 360 px in a
  real browser, and every module's existing vitest suite stays green. Record
  in STATE.md any module whose phone layout needs a product decision rather
  than a layout fix — do not guess at a redesign.
- [ ] R3 The audit that found R1, as a test that never needs a human: add
  Playwright as a `web` dev dependency (browsers from the local cache — no
  network download in the gate), a `web/e2e/responsive.spec.ts` that logs in
  against a local stack, visits every module at 360/768/1024/1440 px and
  asserts: no horizontal document overflow, no element wider than the
  viewport except intentionally scrolling strips (mark them with a data
  attribute, allow-listed in the test), and every module's sidebar hidden by
  default at 360 px. An `npm run test:responsive` script runs it. It must pass
  on main at the end of the item, with any screen it fails on either fixed in
  the same commit or recorded as `[!]` with the screenshot path. **CI wiring
  (`.github/`) is off-limits to the loop — record the workflow step needed
  in STATE.md as the human handover.**
