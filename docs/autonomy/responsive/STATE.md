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

(none yet)
