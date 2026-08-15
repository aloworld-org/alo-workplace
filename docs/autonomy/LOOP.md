# LOOP.md — the autonomous build-loop protocol (multi-track)

You are Claude Code running **unattended, continuously — day and night — until
every item in your track's QUEUE is done**. Nobody will answer questions.
Execute exactly ONE queue item per invocation, completely, then exit; the
wrapper script immediately starts the next iteration.

## Tracks (the invocation prompt names yours)

| Track | Queue / journal | Code areas it owns |
|---|---|---|
| **business** (default) | `docs/autonomy/QUEUE.md` + `STATE.md` | billing/crm/projects/finance/inventory/hr: store modules, `/billing`- etc. routes, `web/src/billing…` (ADR 0035, waves B1–B6) |
| **sites** | `docs/autonomy/sites/QUEUE.md` + `sites/STATE.md` | `site_*` store modules, `products/sites/**`, `/sites/*` routes, `web/src/sites/**`, alo-ai sites module (ADR 0036) |
| **ds** | `docs/autonomy/ds/QUEUE.md` + `ds/STATE.md` | the design system: `web/src/ds/**`, and the `.module.css` of any module it migrates. Touches many modules by design, so it must not run beside another track editing the same web areas — and never `web/src/sites/**` (ADR 0045) |
| **agents** | `docs/autonomy/agents/QUEUE.md` + `agents/STATE.md` | an agent in every product (ADR 0034): `platform/alo-ai/**` except its sites modules, `chat_agents.rs`, `alo-jmap`'s `agent*.rs` and `chat_agent*.rs`, migrations **`04xx`**. Store-and-API only — `web/src/chat/**` is being rebuilt by another agent and is off limits |

**Several streams push to `main` at once, and how many is not a constant — so
find out rather than assume.** Counting them in this file was wrong within a
day of being written. Two ways to see who is live, both cheap enough to do at
the top of an iteration:

```
git log --format='%h %ad %s' --date=format:'%m-%d %H:%M' -1 origin/main -- docs/autonomy/<track>/STATE.md
git log --format='%h %ad %s' --date=format:'%m-%d %H:%M' -12 origin/main
```

A journal touched in the last hour is a loop mid-wave. **A commit in an area no
journal claims is a human or an interactive agent working outside the loops** —
they do not read this file, will not honour your track's boundaries, and are
the likeliest source of a conflict. Treat their area as owned by them for as
long as they are in it, and keep your own commits small enough that a rebase
over their work stays trivial.

**A prerequisite that is not in the queue item does not exist.** `features.md`
and a guide both said in bold that an ADR had to be written before the
domain-selling build; no S2.15 item mentioned it, so five items shipped the
feature without it (ADR 0049 is the retrospective review). A loop cannot honour
a gate it was never given. When authoring a queue item, copy its prerequisites
into the item itself — a gate recorded where the work is *described* but not
where the work is *ordered* is not a gate.

**Never touch the other track's areas.** The deliberately-shared files
(`i18n/en.ts`, `CHANGELOG.md`, route registration in `server.rs`, `mod`
lines in `lib.rs`) only ever receive ADDITIVE lines from either track — on a
rebase conflict there, resolve by keeping BOTH sides; any non-additive
conflict you cannot resolve cleanly → `LOOP HALT`.

## The iteration

1. `git pull --rebase origin main` first — always. If the only conflicts are
   additive i18n/QUEUE/STATE lines, resolve by keeping BOTH sides; any other
   conflict you cannot resolve cleanly → `LOOP HALT` (below).
2. Read your track's STATE (the journal so far) and QUEUE (the ordered
   work) — paths from the Tracks table above.
3. Pick the FIRST item that is neither `[x]` done nor `[!]` blocked.
   - All items done → append `LOOP COMPLETE` to STATE.md, commit, push, exit.
   - Only blocked items remain → re-attempt the OLDEST `[!]` item once with
     fresh eyes; if it fails again, append `LOOP COMPLETE (with blockers)` and
     exit — a human unblocks.
4. Build that ONE item at **full depth** (CLAUDE.md laws + the `implement`
   skill): input → validation → logic → persistence → output → error paths.
   Cut scope, never depth — a narrower slice that fully works beats the listed
   slice half-done (record any cut in STATE.md).
5. Gate it (all must pass):
   - **Before the Rust gate, prune the test database: `bash
     scripts/prune-test-db.sh`.** It takes seconds when there is little to do.
     Tests create a tenant each and mostly never delete it, so the shared local
     database grows by ~12 000 tenants a day and every tenant-scoped query in
     every test slows with it. Left alone for six days it reached 74 671
     tenants and 583 MB, and the identical `cargo nextest run -p alo-store`
     went from **18 seconds to over 90 minutes** — which is what pushed the
     gate past the one-command ceiling and sent earlier iterations into the
     polling workarounds below. If a gate is mysteriously slow, this is the
     first thing to check, not the last: `select count(*) from tenants;`.
   - **A wedged docker, a failed link and a mysteriously dead daemon are all one
     symptom: `df -h` first.** On 2026-08-15 an iteration halted with "docker
     daemon unresponsive" after thirty minutes of `docker ps` hanging; the actual
     fault was C: at **100%, 3.2 MB free**, and Docker Desktop stops answering
     when it cannot write. The tell in a build is `rustc-LLVM ERROR: IO failure on
     output stream: no space on device` or `LNK1318 Unexpected PDB error`. On
     Windows the space is usually **`.pdb` debug symbols in your own checkout's
     `target/debug/deps`** — one per test binary, ~150 MB × ~185 binaries ≈ 17 GB.
     Deleting them frees it and invalidates nothing (cargo does not fingerprint
     them; you lose symbolised backtraces only), but they return on the next
     build. The fix that holds is **`CARGO_PROFILE_TEST_DEBUG=0`** in the gate's
     environment: it applies to test targets only, so no dependency rebuilds, and
     no PDB is written at all. Never clean another checkout's `target/` to make
     room — a loop mid-build there breaks.
     **Once `CARGO_PROFILE_TEST_DEBUG=0` is in force, the next thing to fill the
     disk is stale test binaries, and it is bigger.** Cargo never removes a
     previous build's `<name>-<hash>.exe`, so every iteration that changes a
     crate leaves a full set behind: on 2026-08-15 that was **538 binaries
     totalling 12 GB for 215 distinct targets**, and a full `nextest --no-run`
     died with `LNK1180: insufficient disk space` on a disk the `.pdb` sweep had
     just cleared 7 GB on. Keeping only the newest `.exe` per target name frees
     it and invalidates nothing cargo will not relink anyway. Do it **before**
     the gate, never during one: deleting a binary out from under `nextest
     --list` costs a relink and reads like a build failure.
   - **The `Bash` ceiling on this harness is 10 minutes, whatever timeout you
     pass.** A cold build plus a suite can exceed it. When a foreground gate is
     cut off mid-build, **re-run the same foreground command** — cargo's cache
     carries the build forward and the second call gets to the tests. Do not
     background it, and do not conclude the gate is too big before you have let
     the build finish across two calls.
   - Rust touched: `cargo fmt` on changed crates; `SQLX_OFFLINE=true cargo
     clippy -p <crates> --all-targets` clean; then **`cargo nextest run -p
     <crates>`** green — *not* `cargo test`. `cargo test` runs each test
     binary one after another, and this workspace has ~185 of them, so a full
     `alo-store` run takes over ten minutes and no longer fits one command.
     nextest runs them across all cores: the same 1 769 tests finish in ~18 s
     (measured 2026-08-12). Config, including which binaries must stay serial,
     is in `.config/nextest.toml`. If nextest is missing on this machine,
     install it — `curl -sSfL https://get.nexte.st/latest/mac | tar zxf - -C
     ~/.cargo/bin` — rather than falling back to `cargo test`, which will
     blow the ceiling and send you back to polling.
   - Web touched: `npx tsc --noEmit`; `npx eslint <changed files>`;
     `npm run build` — all clean.
   - Storage touched: the **wrong-tenant test is mandatory** (tenant A
     reaching tenant B's record gets a clean denial, proven by a test).
   - New HTTP routes: wire-verify against the LOCAL backend — docker postgres
     (`alo-pg`, user/db `alo`, password `alo-dev-only`) + the debug
     `alo-jmap` binary (`DATABASE_URL=postgres://alo:alo-dev-only@localhost:5432/alo`,
     `ALO_BLOB_DIR=<repo>/.localdev/blobs`, `ALO_JMAP_ADDR=127.0.0.1:8080`,
     `ALO_IDENTITY_ISSUER=http://localhost:5173`; bootstrap once with
     `identityctl bootstrap-admin` + `register-client web`).

     **The web app has one local port and it is 5173.** Not a default to drift
     from — the only one. `register-client web "alo web"
     http://localhost:5173/auth/callback`, and nothing else: the redirect URI
     is checked before anything else on `POST /oauth/authorize`, so a dev
     server on any other port fails at the login form with `redirect_uri
     mismatch` rather than quietly working. That is the point. Two dev servers
     that both work are two stacks identical on screen, which is what the
     `dev :5173` badge exists to undo; registering a second port would put the
     confusion back and make the badge the only thing standing between you and
     screenshotting the wrong stack. One agent wants the browser at a time —
     hand 5173 over rather than starting a second server beside it.

     **Check which database your server is on before you conclude anything
     about it.** Each checkout has its own: `alo_loop`, `alo_ficina`, `alo`,
     plus every scratch database on the box. An hour went into "proving" a
     redirect URI change on the wire that had been applied to a database
     nothing was reading — the request under test never touched the row that
     was edited, and both the change and the control looked like they passed.
     `select datname from pg_stat_activity where backend_type='client backend'`
     names the one your running binary actually holds open.
     ALWAYS kill any running `alo-jmap` BEFORE
     starting your test server — not only before rebuilding: a stale server
     from a killed iteration squats the port for days and fails every bind
     (found: one survived 19 hours and poisoned a whole night). Kill it before rebuilding too
     (macOS/Linux: `pkill -f alo-jmap`; Windows locks the exe:
     `taskkill //F //IM alo-jmap.exe`). Real curl calls, real DB rows checked.
   - **RUN GATES IN THE FOREGROUND — except the ONE command that cannot fit.**
     One `Bash` call, `timeout: 600000`, read the exit code. Running the tests
     is fast (~18 s for all of `alo-store` under nextest) and clippy is
     seconds; those are always foreground.
     **The exception is the test-binary BUILD after `alo-store` source
     changed.** Every file under `tests/` links its own binary against the
     crate, so a change to the crate invalidates ~115 links, and relinking
     them takes ~40 minutes — it cannot fit one call, and retrying it in the
     foreground kills it at 600 s each time. On 2026-08-14 an iteration spent
     130 of 138 minutes in THIRTEEN such kill-retry cycles (cargo's
     incremental state is what let it converge at all). For that one step,
     use the sanctioned background+marker form — this is exactly the "single
     call cannot fit" case the paragraph below describes:
       `(SQLX_OFFLINE=true cargo nextest run -p alo-store --no-run \
          > /tmp/nextest-build.log 2>&1; echo "BUILD_EXIT=$?" \
          >> /tmp/nextest-build.log) &`
       then `until grep -q "BUILD_EXIT=" /tmp/nextest-build.log; do sleep 15;
       done` — the poll exits on the condition, never on a count. Once built,
       run the tests themselves in the FOREGROUND; they fit with room to
       spare.
     **You run in one-shot mode: there are no notifications, and ending your
     turn ends the PROCESS.** Never background a command and then finish your
     message with "I'll wait for the completion notification" — no
     notification will ever arrive, the wrapper will see you exit and start a
     fresh iteration, and your uncommitted work sits in the tree waiting to be
     re-adopted. That is exactly what iteration 114 did on 2026-08-15: it
     backgrounded clippy, announced it would wait, and died 25 minutes in.
     The ONLY way to wait is inside a tool call that returns when the wait is
     over — the `until grep` poll above, run as a normal foreground command.
     If a poll call itself is killed at the 600 s ceiling, issue the same
     poll again as your next command; the marker file survives between
     calls.
     History, kept because each sentence was paid for: the first version of
     this rule said "everything fits, 5 min 34 s" (2026-08-11) and was false
     a day later after 61 new test binaries landed; the second said
     "foreground, no exceptions" and produced the thirteen kill-retry cycles
     above. A rule that rests on a measurement rots when the measurement
     does. The durable fix is fewer test binaries (queued as a consolidation
     item); until it lands, the exception above is the honest shape of the
     gate.
   - **Never size a wait by iteration count.** `for i in $(seq 1 58); do …
     sleep 10; done` runs 580 s whatever happens — it spends its entire budget
     even when the work finished in the first ten seconds. This is what turned
     a ~40-minute item into 151 minutes on 2026-08-12: **eight** such loops,
     ~9.7 minutes each, 111 of 151 minutes spent waiting on commands that had
     already exited.
   - **Never wait with `until ! pgrep -f "<cmd>"`.** `pgrep -f` matches whole
     command lines, so the waiting shell matches *itself*: the condition never
     goes false and the wait spins to the ceiling. A longer `sleep` does not
     help — the interval was never the problem. This cost ~80 of one
     iteration's 142 minutes on 2026-08-09.
   - If a background run is genuinely unavoidable (only when a single call
     cannot fit), have it write a marker and poll with a loop that **exits on
     the condition, not on a count** — `until grep -q "EXIT=" out.log; do
     sleep 5; done` — and break the self-match with a character class:
     `pgrep -f "[c]argo test -p alo-store"`. Two incidents, in two different
     shapes, both from the same instinct to background a gate that fits in
     the foreground. Resist it.
6. Update what changed behaviour: a CHANGELOG.md line (user voice), rustdoc/
   TSDoc on public items, all UI strings through `i18n/en.ts` (fr/nl at wave
   reviews). New top-level route prefixes: note in STATE.md that the
   production Caddyfile needs the prefix added at next deploy (do NOT touch
   deploy/ yourself).
7. Mark the item `[x]` in QUEUE.md. Append one STATE.md entry: item id, what
   shipped, how verified, cuts/flags, next item id.
8. Commit (conventional message + the Co-Authored-By line), `git push origin
   main`. **Never leave uncommitted work, never skip the push.** If the push
   is rejected (the other track pushed first), `git pull --rebase` (keep-both
   on the additive shared files) and push again — retry up to three times,
   then `LOOP HALT` rather than leave the item unpushed.
9. Exit. The wrapper starts the next iteration.

## Headless discipline (learned the hard way)

You run as a single non-interactive invocation: **when your turn ends, you
cease to exist** — there is no "later". Therefore: NEVER background a build,
test, or any gate command and end your turn waiting for it; run gates in the
foreground and wait for their real exit codes inside the same turn. An ended
turn with uncommitted work means the work is DISCARDED by the wrapper. If the
remaining gates cannot finish in this iteration's budget, cut the item's
scope (LOOP rule: cut scope, never depth), gate the narrower slice fully,
commit and push it, and journal the remainder as the next item's starting
point.

## If stuck

- Two honest failed attempts at a gate → mark the item `[!] blocked: <one
  line>` in QUEUE.md, details in STATE.md, commit, push, exit. The loop moves
  on. Never thrash for hours; never ship a stub to get past a gate.
- Environment broken (docker down, disk full, unresolvable conflict) →
  append `LOOP HALT: <reason>` to STATE.md, commit if possible, exit non-zero.
  The wrapper stops on HALT; a human restarts after fixing.

## Hard safety rails (absolute — no exceptions, ever, unattended)

- **Never touch production**: no ssh, no deploys, nothing at 152.53.179.142 or
  any *.alomails.com / *.aloworkplace.com host. Build + local-verify + push ONLY.
  Deploys happen only when the human is present.
- **Never send real email; never call paid/external AI APIs.** Agent-tool
  slices are verified structurally (routes exist, 401/422 guards, execute
  against the local DB) — never by live model calls.
- **Never commit secrets**, keys, `.env`, `.localdev/` contents, or memory
  files — the repo is PUBLIC. The pre-commit secret hook stays green.
- **Never** force-push, rewrite history, delete branches, edit `deploy/`,
  `.github/`, or others' ADRs. Your write scope is: the code for the current
  item + its tests + migrations + QUEUE/STATE/CHANGELOG/docs for that item.
- Migrations are append-only new files, expand-only — no destructive DDL.
- Legal/compliance items (gapless numbering, VAT, EN 16931 e-invoices):
  implement the strict reading of the cited spec; flag any ambiguity in
  STATE.md for human review — never guess loosely on compliance.

## Standing context

- Vision/scope: ADR 0035; `docs/features.md` → Business modules; ROADMAP →
  Business track. Nothing outside the queue gets built.
- Architecture: `platform/alo-store` = tenant-scoped store (`for_account`,
  newtype ids, `thiserror`); `products/mail/alo-jmap` = axum routes
  (`Problem` errors, `authenticate`, register in `server.rs`); `web/src` =
  React, i18n catalogs, ds tokens, module pattern like Tasks/Calendar.
- Money is ALWAYS integer cents (i64); VAT rates in basis points; totals
  computed server-side; never floats for money anywhere.
- **Every UI slice obeys `docs/design/ux-principles.md`** (the interface
  laws: zero-manual, recognition over recall, empty-states-as-onboarding,
  undo over confirm, verbatim helpful errors, tokens-only styling). Wave
  reviews test the laws on each new screen and file violations as items.
- Tasks, Calendar, and Spaces are the reference implementations for "a new
  module on the store" — read them before inventing a pattern.
