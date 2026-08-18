# Handover — 2026-08-18

Where each thread stands, what blocks it, and the operational facts that cost
time to learn. Written at the end of a long session so the next one starts from
the state rather than from the search.

## Running

**`ds`** — started 13:16 on `C:\dev\Ficina-loop`, 12 items, taking
`web/src/ds/redefined.ts` toward zero. Web-only, so it gates on
tsc/eslint/vitest/build and does not touch Rust. Needs nothing.

## In order of what blocks what

### 1. The campaign identity, and warm-up — the only item with a clock

Everything else here can be done later at the same price. This cannot: warm-up
is weeks of gentle sending and it has not started.

- Dual-sign landed (`7e331ced`): a domain may hold one active DKIM key **per
  algorithm**, `sign_outbound` emits one signature per key, ARC still seals once.
- What remains is wiring `news.alomails.com` to sign with the `camp` RSA key
  (private half at `deploy/production/dkim/campaign.key`, public already
  published) beside an Ed25519 one, then publishing the second DKIM record.
- **Then send one message and read `Authentication-Results`.** It must say
  `spf=pass dkim=pass dmarc=pass`. That is how the transactional stack was proved
  and the only way to know the records are right rather than merely present.
- Strict alignment (`adkim=s; aspf=s`) is live on `_dmarc.news`, so three things
  must be exactly `news.alomails.com`: the From domain, the envelope-from, and
  the DKIM `d=`. A bounce domain defaulting elsewhere fails DMARC while SPF and
  DKIM each pass — two green checks and a red verdict.

### 2. `orders` — one session, and smaller than it looks

ADR 0053 is **disputed**; do not build from it. The wave is suspended in
`ROADMAP.md`. Two reads have already shrunk it (`docs/autonomy/orders/STATE.md`):

- **Reservation is a missing refusal, not a missing number.** `inv_reorder`
  already computes `available = on_hand + on_order − committed`, explicitly
  "computed, never stored". Nothing refuses a confirmation that pushes
  `committed` past `on_hand + on_order`. That guard is one transaction.
- **The quote link may be partly built** — `quote` already appears in
  `inv_so.rs` and `inv_so_confirm.rs`. Establish what those do before deciding
  O1.2.

So: successor ADR from the code, re-cut O1 to roughly four items, run it in
`C:\dev\Ficina-orders` (branch `orders-track`, already checked out, outside any
sync folder).

### 3. `campaigns` C4–C6 — send, measure, automate

Needs (1). C1, C2s and C3 are complete: audience, consent, suppression,
unsubscribe, and the whole email build through preview. An ungated draft of the
send path is parked in the session scratchpad at `campaign-send-draft`; it was
built outside the queue's boundary and must not land as it stands.

### 4. The visible half

Every agent built in the agents wave is invisible in the product. Now unblocked
by Codex's chat rebuild:

- agent DMs in the room list, agent avatar and badge, and **answer-vs-proposal
  rendering** — the read/write split of ADR 0047 cannot be seen today;
- the **charts editor UI** on ADR 0051's tested foundation, which also returns
  chart-from-intent as an ordinary agent tool.

### 5. Deploy

`main` has moved a long way since the 2026-08-17 deploy: campaigns, VAT
treatment, dual-sign, and the corrections. Same procedure as the last one, which
is recorded in that session: back up first, ship source with `git archive` (never
rsync — `.localdev` must not travel), build on the server, migrations run on
start, then web and Caddyfile together, and **restart Caddy rather than reload**
(a reload exits 0 while doing nothing).

## Owner-only

- **Rotate the OpenAI key.** It has been in a transcript since 2026-08-17 and is
  in three databases. Nothing is blocked; it is overdue.
- **Codex and the `ds` queue.** He is migrating Tailwind by hand while the `ds`
  loop does the same job. Two people on one problem without a shared queue
  produce two dialects.

## Operational facts that cost time today

- **One agent per working tree, and the signal is the process, not the log.** A
  worker between iterations writes nothing, so an idle log does not mean
  finished. Check for the wrapper process. Two loops in one checkout make every
  gate meaningless in both directions.
- **`[ ]` is an instruction to keep trying.** An item that will never be built
  here is `[~]` with its reason, or the wrapper keeps invoking past
  `LOOP COMPLETE`.
- **The disk fills from stale test binaries**, not only `.pdb` files. Cargo never
  removes a previous build's `<name>-<hash>.exe`. Sweep to the newest per target
  before a gate; it freed 8 GB twice today. `[profile.test] debug = 0` is already
  in the manifest.
- **`git push … | tail` reports success when the push was rejected** — the
  pipeline returns `tail`'s status. Verify with
  `git rev-list --count origin/main..HEAD`.
- **Read the module header before concluding a feature is absent.** This
  repository states its reasoning in the first twenty lines of a file. ADR 0053
  was written on a filename grep that missed `inv_so_*`, and an entire wave was
  scoped on the answer.
