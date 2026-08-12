# Handing over — read this first

Written at the end of a long session so the next one starts with facts rather
than rediscovery. Paste the "Prompt" section into a new chat, or just read this.

---

## Prompt for the next session

> You are continuing work on **alo / Ficina**, a sovereign EU workplace suite
> (an M365 + Odoo replacement) in `C:\dev\Ficina-loop`.
>
> **Read first:** `docs/guides/next-session.md` (this file), then
> `docs/design/positioning.md` for what the product is against and what to
> claim, then whichever module doc matches the task —
> `docs/design/meet-roadmap.md`, `docs/design/chat.md`,
> `docs/design/site-chatbot-and-commerce.md`.
>
> **You are not alone in this repository.** Two other autonomous streams push
> to the same `main`. Never edit their areas, always rebase before pushing, and
> never put work into a `QUEUE.md` unless you intend a loop to build it next
> iteration.
>
> **Verification is by looking, not by tests.** For any UI change: one change,
> one screenshot, every time. A green suite has repeatedly coexisted with a
> blank page here.
>
> **Ambitious about what to build, unsparing about what is finished.** Never
> argue that a competitor's head start makes something unreachable; do say
> plainly when something has not been verified.

---

## Who else is working here

| Stream | Where | Area — do not touch |
|---|---|---|
| **Codex, on this machine** | `C:\dev\Ficina` (a second checkout of the same repo) | **alo Sites**: `products/sites/**`, `platform/alo-store/src/site_*`, `/sites/*` routes, `web/src/sites/**`, sites generation in `platform/alo-ai` |
| **The Mac's loop** | Another machine | **Business modules**: billing, CRM, finance, inventory, projects, HR |
| **You** | `C:\dev\Ficina-loop` | Everything else — chat, Meet, agents, shell, mail, drive, calendar |

Their queues are `docs/autonomy/QUEUE.md` and `docs/autonomy/sites/QUEUE.md`.
**These are live input.** A loop takes the first unchecked item and builds it in
its next iteration, so an idea placed there gets built rather than discussed.
Record thinking in `docs/design/*.md` or an ADR in `docs/decisions/` instead.

Expect their commits to land under yours constantly. `git pull --rebase` before
every push; conflicts are usually additive (two modules adding to the same
string catalog or module list) and are resolved by **keeping both sides**.

---

## Git

- **Never override the author.** Commits carry the user's name; Claude appears
  only as `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Always push after committing** — `git pull --rebase -q origin main && git push -q origin main`.
- **Commit messages are professional prose.** No "phase N", no "task N", no
  references to a session plan. Say what changed and why; record what is *not*
  verified.

---

## Building and running

The Bash tool's cwd resets to a OneDrive path every call, so **start every
command with `cd /c/dev/Ficina-loop`**.

```bash
export DATABASE_URL="postgres://alo:alo-dev-only@localhost:5432/alo_test"
export SQLX_OFFLINE=true          # queries are cached in .sqlx; no DB needed to build
cargo build -p alo-jmap
cargo test -p alo-store --test <name>
cargo clippy -p alo-store -p alo-jmap --all-targets
cargo fmt -p alo-store -p alo-jmap
```

Web:

```bash
cd /c/dev/Ficina-loop/web
npx tsc --noEmit && npx eslint src/<area> && npx prettier --write src/<area>
npx vitest run src            # READ THE TEST LINE, not just "✓ built"
npm run build
```

### Ports and databases — allocated, not first-come

Two streams run a full stack on this machine, and both default to Vite `5173`
and `alo-jmap` `8080`. Whoever starts first takes the port and the other
silently lands somewhere else, which on 2026-08-12 produced forty minutes of
"my correct password stopped working": the browser was on `5173`, served by the
*other* checkout's Vite, proxying to the *other* backend, against the *other*
database — where that account does not exist.

| Stream | Checkout | Vite | alo-jmap | Database |
|---|---|---|---|---|
| Claude | `C:\dev\Ficina-loop` | **5174** | **8080** | `alo_loop` |
| Codex | `C:\dev\Ficina` | **5173** | **8082** | `alo_ficina` |

Shared on purpose, because they hold no per-stream state worth splitting:
Postgres on `5432` (separate databases inside it) and LiveKit on `7880`.

**Separate stacks are deliberate.** Each stream builds its own Rust binary, so
one shared backend could only ever run one stream's code; every Rust change
needs a restart, which would interrupt whoever else was mid-test; and one
stream's new migration against another's older binary is a `VersionMismatch`
that stops it dead.

**Before believing a bug, check which stack answered.** `netstat -ano | grep
5173` and `Get-CimInstance Win32_Process` name the process and its executable
path, which is the fastest way to find out whose app you are looking at.

### The local stack

```bash
# Postgres (Docker container alo-pg) holds alo_loop (dev) and alo_test (tests)
cd /c/dev/Ficina-loop
export DATABASE_URL="postgres://alo:alo-dev-only@localhost:5432/alo_loop" \
  ALO_BLOB_DIR="C:/dev/Ficina-loop/.localdev/blobs" \
  ALO_IDENTITY_ISSUER="http://localhost:5173" ALO_JMAP_ADDR="127.0.0.1:8080" \
  SQLX_OFFLINE=true
# Meet needs these three or it reports "not configured" (correctly):
export ALO_MEET_URL="ws://localhost:7880" ALO_MEET_API_KEY="devkey" \
  ALO_MEET_API_SECRET="devsecretdevsecretdevsecretdevsecret"
nohup ./target/debug/alo-jmap.exe > /tmp/jmap.log 2>&1 &
# Dropping the three ALO_MEET_* variables does not fail — Meet just reports
# itself unconfigured, which reads as a bug in Meet. Keep them on the line.

cd web && nohup env VITE_DEV_API=http://localhost:8080 npm run dev > /tmp/vite.log 2>&1 &
```

Sign in at <http://localhost:5173> as `disan@alomails.com` / `alo-local-dev-1`
(and `ben@alomails.com`, same password, for two-person tests).

**When the API fails with "database migration failed"** — normal after rebasing
onto newer migrations — recreate the throwaway dev database:

```bash
docker exec alo-pg psql -U alo -d postgres -c "DROP DATABASE IF EXISTS alo_loop;"
docker exec alo-pg psql -U alo -d postgres -c "CREATE DATABASE alo_loop OWNER alo;"
# start alo-jmap (it migrates), then reseed:
ALO_ADMIN_PASSWORD="alo-local-dev-1" ./target/debug/identityctl.exe bootstrap-admin alo disan@alomails.com
./target/debug/identityctl.exe register-client web alo http://localhost:5173/auth/callback
```

LiveKit for Meet:

```bash
MSYS_NO_PATHCONV=1 docker run -d --name alo-livekit \
  -p 7880:7880 -p 7881:7881 -p 50000-50020:50000-50020/udp \
  -v "C:/dev/Ficina-loop/.localdev/livekit.yaml:/livekit.yaml" \
  livekit/livekit-server:latest --config /livekit.yaml
```

### Looking at the screen

Playwright drives the installed Chrome; it is installed in the session
scratchpad, not the repo. Log in, navigate, screenshot, **and open the
screenshot**. This is the only verification that counts for UI.

---

## Traps that have already cost hours

1. **A new top-level route prefix must be added to the proxies** — the array in
   `web/vite.config.ts` *and* all **three** `@backend path` matchers in
   `deploy/production/Caddyfile`. Otherwise the route reaches the SPA and 404s,
   which looks exactly like a broken handler. Cost two afternoons (chat, Meet).
2. **Migration numbers collide across checkouts.** Before adding one, list
   `platform/alo-store/migrations/` in **both** `Ficina-loop` and `Ficina`.
3. **Disk.** Two Rust `target/` directories filled a 474 GB disk twice in one
   day, corrupting build artifacts and wedging Docker's daemon mid-pull.
   `cargo clean` in your own checkout frees tens of GB in seconds. Never delete
   in the other checkout without asking, and never while it is building.
4. **A wedged Docker daemon** (CLI hangs, Desktop looks fine) is cured by
   quitting Docker Desktop, `wsl --shutdown`, then reopening. Nothing needs
   purging.
5. **Git Bash rewrites container paths** — `/livekit.yaml` becomes
   `C:/Program Files/Git/livekit.yaml`. Use `MSYS_NO_PATHCONV=1`.
6. **`echo $?` after a pipeline reports the last command's status**, not the
   one you care about. Redirect to a file and check separately.
7. **`unwrap` and `expect` are denied** in `alo-jmap` and `alo-store`,
   including their tests. Store test files declare
   `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top; `alo-jmap`
   integration tests do not, so write them without either.
8. **`exactOptionalPropertyTypes` is on.** An optional prop that may receive
   `undefined` must be typed `foo?: T | undefined`.
9. **Read the whole gate output.** A commit went out on "✓ built" while the
   line above said `1 failed`.
10. **Every new string needs Dutch and French in the same change.** The
    catalogs are at full parity (3704 keys each) and `locale.test.ts` fails on
    any English key missing a translation. If a change genuinely cannot carry
    them, add the key to `web/src/i18n/untranslated.ts` — a deliberate
    exemption, not a habit; that file is how 588 keys accumulated unnoticed,
    because a missing key silently falls back to English at runtime. Both
    languages address people formally (**u**, **vous**), and the product's own
    type names — Space, Base, Sheet, Doc — are never translated.

---

## Where things stand

**Shipped and pushed:** alo Chat complete (threads, reactions, mentions,
search, Drive attachments, agents as participants, `@`-autocomplete,
edit/withdraw, paging, browse/join, rename/archive, day dividers, unread
separator, drafts, `Ctrl+K`, drag-and-drop, formatting with code and KaTeX
maths, Teams-style bubbles, mobile layout). Agent tool sets for Drive, Agenda,
Chat and Contacts, verified against a live model. Per-user timezone. alo Meet
(records, LiveKit token seam, calendar link, Meet page, pre-flight, screen
share, announces itself in its room). The shell's first mobile layout.
Favourites that maintain themselves.

**Verified in a browser:** the Meet room renders, the token passes LiveKit's own
`/rtc/validate`, chat's formatting and layout, the launcher's ranking.

**NOT verified:** two people seeing each other's video; the meeting
announcement card rendering in a room.

**Nothing is deployed.** Chat, Sites and Meet are all built and reachable by
nobody. That is the largest single gap in the project.

**Next, in order:** the two unverified checks above; `meet-roadmap.md` Stage 1
(participant list, active speaker, reconnection); a Meet link on every calendar
invitation (needs `EventModal`'s `onSave` to return the created event — it
currently returns `void`, which is why this was not done); then recording →
transcript → minutes.

**Waiting on the user:** the deploy, and deleting the temporary OpenAI key in
`.localdev/.env` — which should be rotated at OpenAI regardless, since it was
pasted into a chat.
