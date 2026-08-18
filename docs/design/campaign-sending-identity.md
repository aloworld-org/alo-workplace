# The campaign sending identity: signing for it, and leaving by its own IP

**Status:** design for the code half of ROADMAP C2.1 / C2.1a, written before the
change. ADR 0044 §1 is the decision this implements; ADR 0014 is the per-domain
DKIM mechanism it extends.

## What is already true

Everything outside this repository is done, and none of it is code:

- `159.195.89.28` is bought, routed by netcup, held by the kernel on `eth0`, and
  has forward-confirmed reverse DNS as `news.alomails.com`.
- `news.alomails.com` publishes `v=spf1 ip4:159.195.89.28 -all`,
  `camp._domainkey.news.alomails.com` (RSA-2048, 410 characters), and
  `_dmarc.news.alomails.com` `v=DMARC1; p=none; adkim=s; aspf=s; rua=…`.
- The private half of the campaign key is on the server at
  `deploy/production/dkim/campaign.key` (0600, uid 10001) and never leaves it.
- `7e331ced` taught the store path to dual-sign: one active key per domain **per
  algorithm**, `sign_outbound` emitting one `DKIM-Signature` per active key,
  RSA first, ARC still sealing once.

**So the fork of C2.1a is answered by code that already landed: dual-sign.** What
this note designs is the three things still missing between that and a message
that passes.

## The three gaps, found by reading rather than by design

1. **Nothing can put an RSA key into `dkim_keys`.** `ensure_dkim_key` and
   `/admin/domains/dkim/rotate` both call `generate_ed25519_key`; ADR 0014
   deliberately refuses to generate RSA in-process (the pure-Rust `rsa` crate is
   forbidden, ADR 0008) and says an operator supplies RSA out of band. There is
   no "out of band". The published `camp` record is therefore for a key nothing
   will sign with, exactly as ROADMAP C2.1a says.
2. **`dkim_record_json` renders every stored key as `k=ed25519`.** It calls
   `ed25519_txt_record` unconditionally and returns only the *first* active key.
   After dual-sign a domain has two records to publish, and one of them is RSA —
   so the Domains screen would hand an operator a record that is wrong in its
   `k=` tag and silently omit the other one.
3. **Outbound never chooses a source address.** `OutboundSession::connect` calls
   `TcpStream::connect` and takes whatever the kernel picks, which is the primary
   address. Mail from `@news.alomails.com` would leave from `152.53.179.142`,
   which its SPF record authorises for nothing — `-all`. ADR 0044 §1 says bulk
   mail leaves "by a separate IP", and that sentence has no implementation.

Gap 3 is the one that produces the confusing failure C2.1 warns about: DKIM
passes, DMARC fails on alignment, and two of the three checks look green.

## Surface

### `alo-smtp --install-dkim-key` — the operator door (new CLI contract)

```
alo-smtp --install-dkim-key --tenant <id> --domain <d> [--selector <s>] [--key <pem>]
```

- **With `--key`:** imports an existing PKCS#8 PEM private key. `--selector` is
  then required, because the DNS record was published under a name the operator
  chose. The algorithm is **read from the key**, never taken from a flag — a flag
  the operator gets wrong produces a valid-looking signature that every verifier
  rejects, and that surfaces as lost delivery weeks later rather than as an error
  here.
- **Without `--key`:** generates a fresh Ed25519 key, selector derived from the
  public key (the existing `generate_ed25519_key`, the same one domain
  verification uses).
- **Prints the DNS record to publish** and nothing else. No key material reaches
  stdout, a log, or an error message.
- Runs where the key file and the database already are: inside the `alo-smtp`
  container, which mounts `/dkim` and holds `DATABASE_URL`. The private half
  still never leaves the host.

It reuses `load_pkcs8_pem`, so it refuses a group- or world-readable key exactly
as the signer does. Errors: a missing/oversized/unparseable key, an unknown
tenant, a selector that is not a DNS label, a domain owned by another tenant —
each a one-line message on stderr and a non-zero exit.

### `ALO_SMTP_EGRESS_IPS` — the egress map (new config contract)

```
ALO_SMTP_EGRESS_IPS=news.alomails.com=159.195.89.28
```

Comma-separated `sending-domain=source-ip` pairs. The lookup key is the
**envelope-from domain**, because that is the identity SPF is evaluated for.
Unset (the default) means the kernel chooses, which is today's behaviour
byte-for-byte. A malformed value is a **startup failure**, not a warning: a
sender that silently falls back to the wrong source address is the failure this
whole item exists to prevent.

### `GET /admin/domains` — `dkim` gains a sibling (additive)

`dkim` keeps its meaning (the first active record) so anything scripted against
it still works. A new `dkimRecords` array carries **every** active key's record,
each rendered by its own algorithm — which is what a dual-signing domain must
publish.

## Errors

| condition | what the caller sees |
|---|---|
| key file missing / not PKCS#8 PEM / group-readable | `alo-smtp: …` on stderr, exit 1, no row written |
| key is neither RSA nor Ed25519 | refused by name, exit 1 |
| `--tenant` unknown | refused, exit 1 (no partial write) |
| domain registered to another tenant | refused, exit 1 — see Tenancy |
| `ALO_SMTP_EGRESS_IPS` unparseable | `alo-smtp` refuses to start |
| egress IP configured, MX only reachable over the other IP family | that address is skipped with a logged reason; the message defers and retries, never leaves from an unauthorised address |

## Tenancy

`dkim_keys` rows are tenant-scoped and `install_active_dkim_key` retires the
previous active key **of the same algorithm** for that domain. That is the attack
surface of an install command: a tenant who could install a key for a neighbour's
domain would not read anything, but would knock the neighbour's outbound mail
unverifiable until they noticed. So the command refuses when `domains` holds the
domain for a different tenant, and the wrong-tenant test is that refusal — plus
the proof that the neighbour's existing active key is untouched afterwards.

A sending subdomain that has no `domains` row at all is allowed: `news.…` is an
egress identity, not a hosted domain, and requiring registration would mean
publishing a verification record for a domain nobody receives mail at.

## Out of scope, deliberately

- **A separate queue** (C2.2). This is the egress half only. One queue with a
  per-domain source address is what the wire proof needs; a campaign backlog that
  cannot delay a password reset is a different change with its own test.
- **Per-tenant warm-up caps** (C2.3) — the rate limiter is per destination
  domain today and stays that way here.
- **Provisioning a subdomain per tenant.** This makes *our* identity work end to
  end; turning that into self-service onboarding needs the DNS automation of
  `docs/design/dns-onboarding.md`, and the manual path has to be proven first.
- **The Domains screen showing both records.** The API now returns them
  (`dkimRecords`); the screen still renders `dkim`, so a dual-signing domain
  shows the first of two. Left for the web surface's owner rather than taken
  mid-flight — it is one list where there is one line today, and it is only
  reachable once a tenant dual-signs, which self-service provisioning is what
  makes possible.
- **VERP return paths** (C2.10). Under `aspf=s` a sub-subdomain bounce address
  will not align, which C2.1 already records as something to watch during
  warm-up rather than before it.

## The alternative rejected

**Register `news.alomails.com` as an ordinary domain and let `ensure_dkim_key`
generate an Ed25519 key for it** — ROADMAP C2.1a's option 1, which needs no new
code at all. Rejected because it signs Ed25519 only: RFC 8463 support is still
patchy among receivers, and bulk mail is precisely where a signature a verifier
cannot check is charged to deliverability rather than merely noticed. The
published RSA record would also have to be withdrawn, having been made for
nothing. Dual-sign keeps both audiences and costs one import path.
