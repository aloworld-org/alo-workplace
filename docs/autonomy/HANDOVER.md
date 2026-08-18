# Handover — 2026-08-18

Where each thread stands, what blocks it, and the operational facts that cost
time to learn. Written at the end of a long session so the next one starts from
the state rather than from the search.

**Updated later on 2026-08-18:** the campaign sending identity is done and
warm-up has started — the item that had a clock on it. Section 1 records what it
took; the backend is deployed. `orders` is now the top of the list.

## Running

**`ds`** — started 13:16 on `C:\dev\Ficina-loop`, 12 items, taking
`web/src/ds/redefined.ts` toward zero. Web-only, so it gates on
tsc/eslint/vitest/build and does not touch Rust. Needs nothing.

## In order of what blocks what

### 1. ~~The campaign identity, and warm-up~~ — **done 2026-08-18**

Proved on the wire at an independent receiver, and warm-up started the same day.

```
Received-SPF: Pass (mailfrom) identity=mailfrom; client-ip=159.195.89.28
dkim=pass (2048-bit key) header.d=news.alomails.com header.s=camp header.a=rsa-sha256
dmarc=pass (p=none dis=none) header.from=news.alomails.com
```

The transactional identity was re-checked the same day and is untouched: 10/10
from `152.53.179.142`, `d=alomails.com s=fic`. Two identities, neither borrowing
the other's reputation.

- `alo-smtp --install-dkim-key` is the operator door for a sending identity's
  key (the `camp` RSA key is installed; the record in DNS was compared
  byte-for-byte against what the stored key derives).
- `ALO_SMTP_EGRESS_IPS` chooses the source address **and the greeting** by
  envelope-from domain. On Docker it needs the compose `egress` network and
  `ops/systemd/alo-campaign-egress.service` — a container cannot bind one of the
  host's public addresses.
- **Warm-up: `docs/design/sending-reputation-warm-up.md`** has the schedule, the
  weekly checks and the log. The ramp is conditional on a clean week, not on the
  calendar. Read the DMARC `rua` reports weekly; they are the only independent
  evidence of how our mail authenticates where we cannot see.

**The DNS is complete.** `news.alomails.com MX 10 mail.alomails.com` was
published the same day, closing the last authentication deduction; SPF, the RSA
DKIM key, DMARC and now MX all resolve, and the apex zone was untouched. Mail to
`bounces@news.alomails.com` reaches our MX and is refused with `550 5.7.1
Relaying denied: recipient not local` — clean, and **not yet a return path**:
C2.10 needs the domain accepted for delivery and something that reads a bounce.

**One thing deliberately left undone:** the Ed25519 half of dual-signing is not
installed. Its record must be published *before* its key, or every receiver that
reads it reports an unverifiable signature. RSA alone is signing today, which is
the safe order rather than an omission.

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

**Backend deployed 2026-08-18** — `alo-smtp`, `alo-jmap` and `alo-imap` rebuilt
from `main` and running; database backed up first (`/root/pre-egress-*.dump`,
verified with `pg_restore --list` inside the container). All eleven services
healthy.

**The web bundle and the Caddyfile were not shipped**, so `/campaigns` still
needs its prefix in the production Caddyfile at the next web deploy — new
top-level route prefixes 404 through the SPA otherwise, which has cost two
afternoons before. Same procedure as the last one: build locally, upload, **copy
in place** (`cp -a web-new/. web/`) rather than swapping the directory, and
**restart Caddy rather than reload** (a reload exits 0 while doing nothing).

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
- **Docker outranks your iptables rule.** Creating a bridge network inserts a
  MASQUERADE rule at the *top* of nat `POSTROUTING`. Campaign mail left by the
  transactional IP for an hour while every log line correctly said the source was
  pinned, and the only thing that named it was the receiver's own refusal. The
  answer is to remove the competing rule
  (`enable_ip_masquerade: "false"` on that bridge), never to try to outrank it.
- **A failure that only reaches a DSN is a failure you cannot read.** A permanent
  5xx logged nothing at all; when the DSN itself then failed — a null-sender
  bounce to a return path with no mailbox — the reason was gone, leaving
  `bounced=1` and no explanation. Fixed, but the general lesson stands: whatever
  a diagnosis depends on must be in the log, not in an artefact that can be
  discarded.
- **A green suite says nothing about an identity.** Both of the real defects here
  — the masquerade ordering and the greeting name — were invisible to every test
  and visible in the first message actually sent. For anything a *receiver*
  judges, send one and read what came back.
