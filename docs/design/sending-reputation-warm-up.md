# Warming the campaign sending identity

**Status:** the operating plan for ROADMAP C2.0d, which is Phase 0's long-open
"IP warming begins now". Started **2026-08-18**.

## Why this document exists and why it is dated

Every other item in the campaigns track can be done later at the same price.
This one cannot. A sending identity's reputation is built out of weeks of
consistent, wanted mail, and a cold IP sending its first real campaign is
filtered however correct the DKIM is. The cost of starting late is calendar
time, and calendar time is the one thing working faster afterwards does not
recover.

So the schedule below starts on the day the identity could first sign and send,
not on the day the campaigns wave starts.

## What is being warmed

Two things at once, and they are not the same thing:

- **The IP** `159.195.89.28` — netcup, Nürnberg, verified clean at allocation
  (0 of 60 blocklists), in `159.195.88.0/23`, a different range from the
  transactional `152.53.176.0/22`.
- **The domain** `news.alomails.com` — its own SPF, its own DKIM selector
  (`camp`, RSA, plus an Ed25519 record when it is published), and its own DMARC
  policy at `p=none; adkim=s; aspf=s`.

Receivers score both, and a new domain on a new IP is the hardest start there
is. Nothing can be done about that except to go slowly and to be boring.

## The schedule

Volumes are **per day**, and every figure is a ceiling rather than a target. A
day with nothing worth sending sends nothing; skipping a day costs far less than
sending filler.

| Days | Ceiling | What is sent |
|---|---|---|
| 1–3 | 5 | Seed sends to our own mailboxes and to people who have agreed to receive them. The point is to establish that the identity exists and is not a spammer, not to reach anybody. |
| 4–7 | 20 | The same, widened to colleagues and to any address that has actually asked for something. |
| 8–14 | 100 | The first mail with a real reason to exist — a genuine announcement to people who consented. |
| 15–21 | 500 | Ramp only if the previous week's reports are clean (below). |
| 22–28 | 2 000 | Same condition. |
| 29+ | ×2 weekly | Same condition, indefinitely, until the volume matches the actual audience. |

**Doubling is conditional, not scheduled.** Move to the next row only when the
week just finished shows no hard-bounce rate above 2%, no complaint above 0.1%,
and no DMARC report showing a failure we did not cause deliberately. Otherwise
hold at the current row for another week. Going back a row costs a week; being
throttled by a large receiver costs a month.

## What is watched, and where it comes from

- **DMARC aggregate reports** arrive at `dmarc@alomails.com` from the `rua=` in
  `_dmarc.news.alomails.com`, and the subdomain policy is `p=none` precisely so
  that early mistakes are *reported* rather than quarantined. They are the only
  independent evidence of how our mail authenticates at receivers we cannot see
  into. Read them weekly; a week of clean reports is what licenses the next row.
- **Our own SMTP results** — accepted, deferred, hard bounce — are facts we
  already hold, and C5.1 turns them into per-recipient events. Until then they
  are in the queue's logs.
- **Blocklists.** Re-check `159.195.89.28` at the start of each week. A clean IP
  that suddenly appears on a list is the ramp being too fast, and the answer is
  to stop rather than to appeal.

## Two things that will look like failures and are not

- **A VERP return path will not align under `aspf=s`.** Per-recipient bounce
  attribution (C2.10) lives at a sub-subdomain, which strict SPF alignment
  rejects. DMARC passes if *either* identifier aligns, so a correct
  `d=news.alomails.com` signature carries it — and the `p=none` reports will say
  so before anything is enforced.
- **A second DKIM signature can fail while the first passes.** The identity
  dual-signs; a receiver that cannot read RFC 8463 reports the Ed25519 signature
  as unverifiable and takes the RSA one. That is the arrangement working, not a
  key problem — which is exactly why the Ed25519 record must be published
  *before* its key is installed, never after.

## The honest limit of the first two weeks

There is no send path yet (C4) and no consenting audience assembled, so days
1–14 are seed sends: real messages, to real people, in single and double digits.
That is genuinely what a warm-up looks like at the start, but it should not be
read as the ramp being under way in the sense the later rows mean. The ramp
becomes real when there is something worth sending to people who asked for it,
and the schedule above is written so that the clock starts anyway — because the
alternative is a cold identity on the day the first campaign is ready.

## The log

One line per sending day: date, how many, to whom in general terms, and anything
the reports said. Kept here because a warm-up nobody recorded is a warm-up
nobody can prove.

| Date | Sent | To | Notes |
|---|---|---|---|
| 2026-08-18 | 6 | authentication verifiers | **Day 1** — the identity's first mail. Four were rejected or lost while the egress was wrong (Docker's masquerade rule outranked ours, so they left by the transactional IP and were refused for SPF); the last two authenticated cleanly: `spf=pass` from `159.195.89.28`, `dkim=pass` `d=news.alomails.com s=camp`, `dmarc=pass` under strict alignment. A refused message is not a reputation event — the receiver never accepted it — so day 1 counts as two delivered. |

**Day 1's other findings.** The receiver scored the identity down 3 of 10 for
greeting as `mail.alomails.com` while connecting from an address whose reverse
DNS says `news.alomails.com`; fixed the same day, and the reverse-DNS check now
reads `IP: 159.195.89.28  HELO: news.alomails.com  rDNS: news.alomails.com`.

**What is left, and one of it needs the registrar:**

- **`news.alomails.com MX 10 mail.alomails.com` — published 2026-08-18** at
  Namecheap and resolving at Google, Cloudflare and the authoritative servers;
  the apex `MX` and everything else in the zone were left untouched. It was the
  last authentication deduction a receiver made (−3): *"We didn't find a mail
  server (MX Record) behind your domain name"*, because a sending domain that
  cannot receive looks one-way.

  **What it does and does not do, stated exactly.** The domain now answers for
  mail, and a message to `bounces@news.alomails.com` reaches our MX and gets
  `550 5.7.1 Relaying denied: recipient not local` — the anti-open-relay guard,
  since the domain is not in `ALO_SMTP_LOCAL_DOMAINS`. That is a clean permanent
  refusal the sending server reports back to its own user, not a black hole and
  not an open door. **It is still not a working return path**: nothing of ours
  reads a bounce. Making them arrive and act is C2.10, and it needs the domain
  accepted for delivery, which is a deliberate second step rather than an
  oversight.

  *Not re-measured at the receiver:* the free tier of the verifier used all day
  was exhausted by the time the record propagated, so the score was not read
  again. The deduction was a statement about a DNS record, and that record now
  demonstrably exists; the authentication result itself was already proved twice
  and nothing in this change touches it.
- The other deductions are content rather than identity, and both are queued
  ahead of any real send: no `List-Unsubscribe` (C2.4/C2.5) and no HTML part
  (C3's renderer exists; the day-1 probes were hand-written plain text).

**The transactional identity was re-checked the same day and is untouched:**
10/10, leaving by `152.53.179.142`, greeting as `mail.alomails.com`, signing
`d=alomails.com s=fic`, passing SPF, DKIM and DMARC under `p=quarantine`. That
comparison is the point of the whole arrangement — two identities, neither
borrowing the other's reputation — so it is worth re-running whenever the
egress configuration changes.
