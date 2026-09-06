# Current repository execution roadmap

Updated 2026-09-06 from a clean clone of `origin/main` at `5ad3702e`. This is
the short, evidence-backed execution queue for repository health. The product
programme remains in `ROADMAP.md`; completed specialist queues and their proof
remain under `docs/autonomy/`.

## Repository assessment

- Rust workspace: 14 members across the shared platform, mail, sites and
  control-plane products. The full backend inventory is 5,245 tests across 78
  binaries.
- Web application: React 19, TypeScript, Vite and Vitest; 313 test files and
  1,531 tests. Docs and Sheets embed BlockNote and UniverJS.
- Data boundary: Postgres is accessed through tenant-scoped store doors. The
  test corpus includes cross-tenant and cross-account HTTP and store suites.
- Delivery: Docker Compose and Kubernetes assets live under `deploy/`; CI
  gates formatting, clippy, Rust tests, web lint/typecheck/tests/build and
  dependency audits.
- Governance: `ROADMAP.md` mixes product work, legal decisions, live-operation
  attestations and historic programme notes. It is a portfolio document, not a
  safe autonomous code queue. The specialist queues are the source of truth
  for already completed waves.

## Execution queue

- [x] CR-0 Work outside the stale OneDrive checkout. Preserve its uncommitted
  work and use a clean clone of current `origin/main`.
- [x] CR-1 Establish a baseline with the repository's Rust, web and dependency
  gates.
- [x] CR-2 Repair baseline regressions: Finance translations, deterministic UI
  dates, clippy failures, missing Sites editor controls, and Finance/Sites
  intent coverage.
- [x] CR-3 Reconcile Sites tests with the server's current 19-section palette
  and explicit `text-image-split` rendering class.
- [x] CR-4 Make the two concurrency-sensitive backend tests reliable under the
  full parallel suite: event-instruction coalescing and DMARC report sweeping.
  Event/schedule/once claims now lock selected rows with `SKIP LOCKED`; the
  DMARC test observes the store's reporting boundary instead of sleeping for a
  guessed interval and its host-global sweeps are serialized across nextest's
  process-per-test workers.
- [x] CR-5 Reduce production dependency exposure. `npm audit --omit=dev`
  currently reports 105 findings (94 high, 11 moderate). The high findings are
  the UniverJS dependency tree rooted at the direct `@univerjs/presets` package
  and its `nanoid` version; npm reports no compatible automatic fix for that
  root. The moderate findings are BlockNote's Tiptap tree and do have updates.
  BlockNote is now 0.54.0 and Tiptap is pinned to patched 3.31.3: all 11
  moderate findings are gone and 1,531 web tests pass. The remaining 94 high
  findings have no compatible npm fix and move to the owner/upstream gate.
- [x] CR-6 Remove the Vite circular-chunk warnings around the JMAP/agent barrel
  imports, then introduce deliberate lazy boundaries for the multi-megabyte
  Docs/Sheets editors. Preserve current routes and editor behavior. The JMAP
  and record-agent foundations now share one explicit chunk; circular-chunk
  warnings fell to zero. Docs and Sheets already load behind route-level lazy
  boundaries; their remaining large chunks are their editor engines, not the
  application entry point.
- [x] CR-7 Re-run the complete quality gate, dependency audits and explicit
  tenant-isolation suites; review the final diff and commit in coherent units.
  Evidence: web lint and typecheck clean; 313 files / 1,531 tests green;
  production build green with zero circular-chunk warnings; changed-package
  clippy clean with warnings denied; the full backend run reached 5,240/5,245
  before identifying the five repaired failures; all five now pass in focused
  suites, including all nine instruction HTTP tests, five repeated concurrent
  event runs, and the three serialized DMARC cases. The passing full run also
  covered the cross-tenant store and HTTP suites. Rust audit reports only the
  repository's three allowed warnings; npm audit is 94 high / 0 moderate / 0
  critical, with all remaining findings in the owner-gated UniverJS tree.
- [x] CR-8 Reconcile `ROADMAP.md` status against the closed specialist queues.
  Change a checkbox only where code, tests or a recorded live attestation prove
  completion; do not silently convert legal, commercial or production trials
  into engineering claims. The stale Phase-2 PWA item is now closed from the
  completed M5.1–M5.3 queue and its structural suites. Other unchecked
  portfolio items were left intact where they require product decisions,
  production evidence, or a newly authorised specialist queue.

## Owner- and environment-gated programme work

These are not autonomous repository edits and remain open in `ROADMAP.md`:

- company/IP, CLA, trademark, domains and licence-boundary decisions;
- hosting/vendor selection, production DNS and real-user cutovers;
- multi-week live-mail trials, restore rehearsals and real-campaign evidence;
- product forks requiring commercial packages or licensing decisions, such as
  UniverJS Pro chart support.
- replacement, patch or formally accepted exposure for the current UniverJS
  release, whose dependency tree accounts for all 94 remaining production npm
  audit findings and has no compatible automatic fix.

## Loop

Take the first unchecked `CR-*` item whose prerequisites are satisfied. Read
the owning code and tests, reproduce the failure, implement the smallest full
vertical slice, add or strengthen the regression test, run focused gates, then
run the affected package gates. Record evidence here and continue to the next
unchecked item. Stop only for an owner/environment gate or after CR-7 and CR-8
are complete.
