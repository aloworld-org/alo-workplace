# Running build loops on a Mac

The loop protocol is `LOOP.md`; this is the machine setup for macOS, so a
second computer can carry tracks the build PC cannot. One checkout per track —
**one agent per working tree** is the constitution's rule, and it is what the
loop runner's lock enforces (`~/.alo-loop-<track>.lock`).

## 1. Prerequisites (once)

```sh
xcode-select --install                       # compilers, git
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install git node@22 python3 gh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # rustup; the repo pins its toolchain in rust-toolchain.toml
npm install -g @anthropic-ai/claude-code
claude                                        # sign in once, interactively, then exit
gh auth login                                 # so `git push` over HTTPS works unattended
```

Docker for the test database: Docker Desktop, or `brew install colima docker && colima start --memory 4`.

```sh
docker run -d --name alo-pg --restart unless-stopped \
  -e POSTGRES_USER=alo -e POSTGRES_PASSWORD=alo-dev-only -e POSTGRES_DB=alo \
  -p 5432:5432 postgres:16.14
```

The suites never touch `alo`; they use `alo_scratch` (created and dropped by
the harnesses — `platform/alo-test-db` refuses anything named `alo`). The
supervisor sets `DATABASE_URL` to the scratch database by default.

Keep the Mac awake while loops run: `caffeinate -dims &` (or Energy Saver →
prevent sleep on power).

## 2. One clone per track

```sh
mkdir -p ~/alo && cd ~/alo
for t in agents-a agents-b agents-c; do
  git clone https://github.com/aloworld-org/alo-workplace.git $t
  git -C $t config user.name  "<the owner's name, exactly as on the build PC>"
  git -C $t config user.email "<the owner's email, exactly as on the build PC>"
done
```

Commits from a loop carry the owner's identity, like every other commit in
this repo (`git -C <any laptop checkout> config user.name` shows it). The
track's `[web]` items need `cd web && npm ci` once per clone; the Rust build
happens on the first iteration (expect 20–40 min the first time).

## 3. Start a loop

```sh
cd ~/alo/agents-a
nohup bash scripts/loop-supervisor.sh "$PWD" agents-a > ~/loop-agents-a.log 2>&1 &
```

One supervisor per clone; a supervisor may run several tracks **one after the
other** in the same clone (`… "$PWD" business ds`), never two at once. The
supervisor restarts the runner until the journal writes `LOOP COMPLETE` or
`LOOP HALT`, cleaning the checkout's own build output between runners.

Tunables (environment): `IDLE_KILL_MIN` (default 45), `ITERATION_CEILING_MIN`
(default 300), `DATABASE_URL`.

## 4. Watch it

```sh
tail -f ~/loop-agents-a.log                                   # iterations, kills, restarts
git -C ~/alo/agents-a log --oneline -5 origin/main -- docs/autonomy/agents-a/STATE.md   # finished items
```

Stop: `pkill -f "loop-supervisor.sh .* agents-a"` then `pkill -f "run-loop.sh"`
for that clone. Nothing committed is lost; a killed iteration is redone.

## 5. Which tracks, and the one gate

`LOOP.md`'s table names every track. The agent tracks are split so that they
edit disjoint files: **agents-a** (Sales, Finance, Projects, Inventory, HR),
**agents-b** (Drive, Docs, Sheets, Tasks, Agenda), **agents-c** (Chat, Meet,
Insights, Mail, Sites), and **agents** (the shared machinery: routes as
adapters, additive registration, delegation, memory, standing instructions,
actions, goals, the exit).

**Start `agents-a/b/c` only after `A4.1c` is `[x]` in
`docs/autonomy/agents/QUEUE.md`** — that item makes a module's registration
one additive line in each shared file, which is what lets three loops land
modules without rebasing over each other's edits of the same match arms. Until
then, two of them would spend their iterations resolving conflicts in
`agent.rs` and `agent_product.rs`.

Migration numbers per track are in the table; check the directory immediately
before rebasing, because another loop may have taken a number since the
iteration began.

**2026-08-30 — agents-web and agenda-sync are complete too.** The Mac's next
tracks are **verbs-a** (Billing's twenty deferred exclusions) and **verbs-b**
(the other twenty-seven across eight modules). Both are Rust-first, need the
postgres container, and touch disjoint files by design — `verbs-a` is only in
`billing_intents.rs`, `verbs-b` is in every other module's. The Windows PC is
running track `agents` (wave A10, the three defects the evaluation found);
A10.1 fixes Ask alo's delegation, which `verbs-b`'s VB.3 and VB.8 depend on —
if a delegation still fails there, say so in the journal and carry on rather
than fixing the orchestrator from this track.

**2026-08-29 — a/b/c are complete.** The Mac's next tracks are **agents-web**
(A8.4 with the widened writ; web only, needs `cd web && npm ci` and
`npx playwright install chromium` once) and **agenda-sync** (Rust-first,
CalDAV; needs the postgres container and `pip3 install caldav` for the
real-client pass). One clone each, both may run at once — they share only
`web/src/agenda/**`, and the queues say which files each keeps to.
