# Interop log

Client quirks and RFC deviations. Format per entry: date · client+version · quirk observed · our response · RFC section affected.

(no client-forced entries yet — and every entry added here is a debugging session nobody repeats)

## Standing policies (deliberate strictness/tolerance choices, not client-forced)

- 2026-07-25 · **Bare LF / stray CR rejected everywhere** · RFC 5321 §2.3.8 requires
  CRLF; bare line endings are the SMTP-smuggling vector. Commands: 500, session
  continues. Inside DATA: 500 and the connection **closes** (the stream cannot be
  trusted to re-sync). Matches Postfix `smtpd_forbid_bare_newline=yes` posture.
- 2026-07-25 · **One space tolerated after `MAIL FROM:`/`RCPT TO:`** · RFC 5321
  §4.1.1.2 admits no space; many clients send one; no security ambiguity → accepted.
- 2026-07-25 · **DATA line length** · §4.5.3.1.6 sets 1000 octets as the sender
  limit; long HTML lines are routine in real mail. We accept content lines up to
  8192 octets and reject the message (500, drained, session survives) beyond —
  tolerance with a defensive ceiling.
- 2026-07-25 · **8-bit bytes accepted in DATA without 8BITMIME advertised** ·
  Strictly, unadvertised 8BITMIME means 7-bit only (RFC 6152); rejecting 8-bit
  bodies would bounce half of real-world mail. Accepted, like every mainstream MTA.
  8BITMIME advertisement lands with the capability milestone (M3).
- 2026-07-25 · **General address literals rejected 501** · §4.1.3 tagged literals
  (`[tag:content]`) are syntactically legal; nothing routable can be done with
  them; IPv4 and `IPv6:` literals are accepted.
- 2026-07-25 · **Source routes accepted and ignored** · §4.1.2 / Appendix C:
  `<@relay:user@dom>` parses, the route is validated then discarded.
- 2026-07-26 · **DATA has a 10-minute total budget** · §4.5.3.2 specifies
  per-wait timeouts; we additionally bound the whole message receive at 600 s as
  anti-flood policy. A legitimate sender slower than ~350 kbps on a 25 MiB
  message would be cut off (421); accepted trade-off, revisit with real traffic.
- 2026-07-26 · **EHLO/HELO argument restricted to printable ASCII** · §4.1.1.1
  expects a Domain/address-literal; we reject control octets with 501 so the
  attacker-controlled greeting can never inject binary into the Received: stamp
  or the spool sidecar. SMTPUTF8 (M3) will widen this to U-labels.
- 2026-07-27 · **Outbound delivery off by default** · M1 accepts any recipient;
  turning on relaying before the AUTH gate (M3) would make an exposed instance
  an open relay. Delivery requires `ALO_SMTP_OUTBOUND_ENABLED=true`, and a
  smarthost route is the supported self-hosted mode until MX+AUTH are complete.
- 2026-07-27 · **Empty MAIL FROM on outbound = null path** · we send
  `MAIL FROM:<>` for DSNs and never generate a DSN for a message that itself
  arrived with a null reverse-path (RFC 5321 §4.5.5 loop prevention).
- 2026-07-27 · **Domainless recipients parked, not delivered** · a bare
  `<postmaster>` (§4.1.1.3) has no domain to route to; M2 holds such messages
  in the spool (logged) pending local delivery (M5) rather than dropping or
  bouncing them.
- 2026-07-27 · **STARTTLS discards all pre-TLS state** · RFC 3207 §4.2: after a
  successful STARTTLS the HELO/EHLO identity, any transaction, and any prior
  auth are cleared; the client must EHLO again. Buffered plaintext arriving
  after our 220 and before the handshake is treated as a command-injection
  attempt (CVE-2011-0411 class) and the connection is dropped, nothing executed.
- 2026-07-27 · **AUTH offered only on submission over TLS** · `AUTH PLAIN`/`LOGIN`
  are advertised and accepted only on a submission listener with TLS active.
  On the MX (port 25) role AUTH is refused 503; before TLS it is refused 538.
  Wrong password and unknown user return the same 535 (anti-enumeration, §7.3).
- 2026-07-27 · **EHLO capabilities are state-exact** · we advertise STARTTLS only
  while TLS is inactive, AUTH only on submission-over-TLS, and always SIZE and
  8BITMIME. Advertising a capability implies accepting its MAIL parameters, so
  `SIZE=`, `BODY=7BIT|8BITMIME`, and `AUTH=` (RFC 4954 §5, accepted and ignored)
  are honored; every other MAIL parameter is still 555.
- 2026-07-27 · **Submission adds Date/Message-ID only** · RFC 6409 §8 permits the
  MSA to rewrite more, but we make the minimal non-destructive fix (add `Date:`
  and `Message-ID:` when absent) and never touch `From`/`Sender` or the body.
- 2026-07-27 · **Submission requires STARTTLS then AUTH** · on submission
  ports (587/465) MAIL before TLS gets 530 (must STARTTLS) and MAIL before a
  successful AUTH gets 530 (auth required) — the open-relay gate. MX (25)
  authenticates no one and never advertises AUTH.
- 2026-07-27 · **Authentication-Results is the verdict contract** · every
  SPF/DKIM/DMARC (and later ARC/spam) result is recorded in one
  `Authentication-Results` header (RFC 8601) under one authserv-id (our
  hostname). Downstream (store/JMAP/UI) parses THIS, not internal types; the
  rendered format changes additively only. `Received-SPF` is also stamped for
  operators/legacy tooling but is not the authoritative record.
- 2026-07-27 · **Malformed auth input fails, never crashes** · a malformed
  SPF/DKIM/DMARC record, DKIM signature, or DNS key (all internet-sourced)
  yields a fail/permerror verdict, never a panic — enforced by the workspace
  unwrap/panic deny-lints plus fuzz-style tests and a bounded hand-rolled DER
  parser for DKIM public keys.
- 2026-07-27 · **DMARC disposition** · `p=reject` + authenticated-fail → 550 at
  DATA. `p=quarantine` is accepted (the verdict is recorded in
  Authentication-Results; actual foldering is a store concern, M5). SPF `ptr`
  is implemented but discouraged (RFC 7208 §5.5).
- 2026-07-27 · **RSA crypto via ring, not the rsa crate** · the `rsa` crate
  carries the unfixed Marvin timing sidechannel (RUSTSEC-2023-0071); DKIM RSA
  sign/verify use ring (constant-time). DKIM public keys (SPKI) are unwrapped
  to PKCS#1 by a small bounded DER parser before ring verification.
- 2026-07-27 · **Rspamd fail-closed at DATA (M4b)** · when a scanner is
  configured (`ALO_SMTP_RSPAMD_URL`) and is unreachable / times out / answers
  unparseably, the message is deferred **451**, not accepted — a scanner outage
  must never silently disable filtering. `reject` → 550, `soft reject`/`greylist`
  → 451, else accept with an `x-spam` method in Authentication-Results. DMARC
  `p=reject` is evaluated *before* the spam verdict, so an authenticated-fail is
  a 550 DMARC rejection regardless of spam score. Verified end-to-end against
  real Rspamd 4.1.2 (GTUBE → 550).
- 2026-07-27 · **Rspamd request metadata is CR/LF-stripped** · the envelope
  fields we pass to `/checkv2` (`IP`/`Helo`/`From`/`Rcpt`/`MTA-Name`) are
  attacker-controlled; control characters are stripped so a crafted MAIL FROM
  cannot inject extra HTTP headers into the scanner request.
- 2026-07-27 · **MTA-STS served in plaintext behind the TLS proxy (M4b)** ·
  RFC 8461 §3.2 mandates HTTPS with a WebPKI-valid cert on `mta-sts.<domain>`;
  `alo-smtp` serves the policy over plaintext HTTP on
  `ALO_SMTP_MTA_STS_ADDR` and the deploy reverse proxy terminates TLS. The
  policy `id` is derived from the policy content, so it rotates automatically on
  any change. **DNS records to publish:** `_mta-sts.<domain> TXT "v=STSv1;
  id=<the id we render>"` and `mta-sts.<domain>` A/AAAA (or CNAME) pointing at
  the proxy that fronts the policy endpoint. TLS-RPT (`_smtp._tls`) reporting is
  deferred.
- 2026-07-27 · **Inbound trust headers stripped before stamping** · RFC 8601
  §5: on the MX boundary we delete any pre-existing `Authentication-Results`
  bearing our own authserv-id, and any `Received-SPF`, before adding ours — a
  remote sender must not be able to plant the verdict header downstream trusts.
  A different authserv-id's `Authentication-Results` (a legitimate upstream) is
  preserved.
- 2026-07-27 · **DKIM `From` must be signed** · RFC 6376 §6.1.1: a signature
  whose `h=` omits `From` is a permerror, not a pass — otherwise the visible
  sender could be altered while DKIM still reported pass.
- 2026-07-27 · **DKIM `l=` counts canonicalized octets** · §3.7: inbound `l=`
  is applied after body canonicalization, not before, so `simple`-body
  signatures with trailing-whitespace differences score correctly. Our signer
  omits `l=` by default (it permits post-signing appends) but can emit it.
- 2026-07-27 · **DMARC `pct` sampled with a non-crypto draw** · §6.6.4: for the
  `100 - pct` fraction "sampled out", the next-lower policy applies
  (reject→quarantine→none). The per-message draw is a sub-nanosecond timestamp
  sample — sufficient for policy sampling, not a security decision.
- 2026-07-27 · **SPF `redirect=` to a recordless domain is permerror** · §6.1:
  a redirect whose target publishes no (or a malformed) SPF record is a
  permerror, distinct from a bare `none`. A no-record lookup also charges the
  §4.6.4 void-lookup budget.
- 2026-07-27 · **Non-UTF-8 header octet drops only its own field** · a stray
  8-bit byte in one header no longer erases the whole header block (which would
  silently void DKIM/DMARC for the message); each header's UTF-8 is validated
  in isolation. A multi-address `From` with differing domains yields no DMARC
  From-domain (RFC 7489 §6.6.1).
- 2026-07-27 · **JMAP mailbox thread counts approximate email counts** ·
  RFC 8621 §2 requires `totalThreads`/`unreadThreads`; the interim store does
  not compute distinct thread counts per mailbox, so `alo-jmap` reports them
  equal to `totalEmails`/`unreadEmails`. Clients rely mainly on
  `unreadEmails`; exact thread counts are an additive refinement.
- 2026-07-27 · **JMAP state tokens are one per-account modseq shared by types** ·
  Mailbox/Email/Thread states are the same opaque per-account counter (keyed
  by `(tenant, user)`, migration 0005), so a type's state may skip numbers
  when another type changed. State tokens are opaque (clients must not parse
  them), so this is spec-conformant and lets `/changes` for any type resume
  correctly. The counter is per-account, not per-tenant, so a co-tenant user's
  activity never advances another's cursor — closing the activity-volume side
  channel that a shared tenant modseq left open (the invariant IMAP IDLE
  relies on for account-scoped push).
- 2026-07-27 · **JMAP isolation is per-account (accountId = user)** · every
  by-id read/mutate, `/changes`, `Thread/get`, and blob download is scoped to
  the token's `(tenant, user)` — a user cannot reach another user's mail even
  within the same tenant. Threads are resolved per-account (no cross-user merge
  on a shared Message-ID). Enforced by `owns_*` guards + a `user_id`-scoped
  change log; covered by a two-users-in-one-tenant isolation test.
- 2026-07-27 · **JMAP `Email/set` per-object update is not yet atomic** ·
  RFC 8620 §5.3 wants a single object's changes applied all-or-nothing; the
  interim maps each keyword/mailbox change to its own store transaction, so a
  mid-update failure can leave a partial (but individually consistent) result
  reported under `notUpdated`. A single-transaction multi-change store method is
  the follow-up. Mixed full-`keywords` + `keywords/X` patches are applied
  sequentially rather than rejected as `invalidPatch` (documented, not fatal).
- 2026-07-27 · **JMAP rate-limiting / concurrent-upload caps deferred to the
  gateway** · `/auth/token` throttling and `maxConcurrentUpload` enforcement
  belong at the gateway/identity layer (alo-identity); `maxObjectsInGet`,
  `maxSizeRequestObject` (per-route body limit), and pagination caps ARE
  enforced in-process now.
- 2026-07-27 · **JMAP auth is interim bearer tokens (not OIDC yet)** ·
  `/auth/token` issues opaque bearer tokens against argon2 credentials in the
  store; the token → `(tenant, account)` resolution is the seam alo-identity
  (OIDC) replaces later. No cookies, no OAuth flow yet (deliberate — see
  `docs/design/jmap-api.md`).
- 2026-07-27 · **Store threading is references-only and forward-only** ·
  `alo-store` threads a message onto an *earlier* message it references
  (`In-Reply-To`/`References`); it does not merge on base subject alone
  (RFC 8621 §3 permits subject as a tiebreaker, but subject-only merging bleeds
  unrelated conversations, so we omit it). Consequence: a reply delivered
  before its parent, or a `Re:` with no `References`, starts its own thread and
  is not retro-merged. Accepted trade-off; revisit if real mail shows
  meaningful out-of-order fragmentation.

## IMAP / POP3 shims (alo-imap)

- 2026-07-27 · **Real-client interop: Python imaplib 3.14 + raw openssl s_client** ·
  the IMAP shim was driven end-to-end by Python's `imaplib` (a real, widely-
  deployed client library) over implicit TLS: LOGIN, LIST, SELECT, FETCH
  (ENVELOPE + `BODY[HEADER.FIELDS]`), STORE ±FLAGS, SEARCH, UID SEARCH, and
  APPEND (returning `[APPENDUID]`) all succeeded with **no accommodation
  required**, and a raw `openssl s_client` transcript confirmed the full loop
  including IDLE receiving an untagged `* n EXISTS` as a message was delivered
  into the selected mailbox. **Thunderbird's GUI could not be driven in this
  headless environment**; `imaplib` + the openssl transcript stand in as the
  real-client evidence for this milestone, and a Thunderbird desktop pass is a
  recorded follow-up for when a GUI environment is available. No client-forced
  quirk surfaced; the entries below are our deliberate strictness/model choices.
- 2026-07-27 · **INBOX is auto-provisioned at LOGIN** · RFC 9051 §5.1 makes
  INBOX always present; the store creates a user's inbox lazily, so the IMAP
  session calls `AccountStore::inbox()` right after authentication so LIST/
  SELECT/APPEND never see a missing INBOX on a brand-new account.
- 2026-07-27 · **HEADER.FIELDS returns message order, byte-exact** · RFC 9051
  §7.5.2: `BODY[HEADER.FIELDS (…)]` returns the named fields **in the order they
  appear in the message**, not the requested order, as exact byte slices of the
  stored header (clients may hash them). `BODY[]`/`[HEADER]`/`[TEXT]` are exact
  slices too.
- 2026-07-27 · **Auth only over TLS; POP3 TLS-only** · IMAP `LOGIN`/
  `AUTHENTICATE` are refused before STARTTLS with `NO [PRIVACYREQUIRED]` and
  `LOGINDISABLED` is advertised pre-TLS (RFC 9051 §7.1.1); POP3 is served only
  on implicit TLS (995), never cleartext 110, since USER/PASS are in the clear.
  Both cap failed authentications per connection and drop the socket.
- 2026-07-27 · **Flags are shared across mailboxes (JMAP model)** · a message
  COPYed into two mailboxes is one store object with one keyword set, so setting
  `\Seen` in one mailbox is visible in the other — a deliberate divergence from
  classic IMAP copy-independence, inherited from the JMAP-native store. Most
  clients do not rely on per-copy flag independence.
- 2026-07-27 · **`\Recent` not tracked; `\Deleted` is the `$deleted` keyword** ·
  RFC 9051 retires `\Recent`, so we always report `0 RECENT` and omit it from
  PERMANENTFLAGS. `\Deleted` has no JMAP standard keyword and is stored as the
  internal `$deleted` keyword; EXPUNGE removes `$deleted` messages from the
  mailbox (destroying a message only when it is left in no mailbox).
- 2026-07-27 · **Hierarchy separator is `/`** · IMAP mailbox paths join name
  segments with `/`; a stored mailbox name that itself contains `/` is
  ambiguous over IMAP and is a documented shim limit (JMAP-created names
  normally do not). Mailbox names with control characters are rejected at
  CREATE/RENAME (a CR/LF in a name would otherwise splice response lines).
- 2026-07-27 · **BODYSTRUCTURE fidelity is bounded and honest** · single-part
  and `multipart/*` trees decompose correctly; MIME malformed past depth 16 /
  256 parts degrades to a single `text/plain` part rather than emitting a
  fabricated structure. Extension fields we do not compute (MD5, disposition,
  language) are `NIL`. CONDSTORE/QRESYNC are **not** advertised (no per-message
  mod-sequence). See `docs/design/imap-pop3-shims.md`.
- 2026-07-27 · **IDLE is poll-driven off the per-account change cursor** · RFC
  2177 push is delivered by watching this account's own modseq (migration 0005)
  at a 1 s cadence and diffing the selected-mailbox view; sub-second LISTEN/
  NOTIFY is a follow-up. The cursor is per-account, so an IDLE stream is
  provably silent about another account's activity.
- 2026-07-27 · **MOVE into the source mailbox is a no-op** · RFC 6851 MOVE of a
  message into the mailbox it already occupies must not lose it; we detect
  same-mailbox MOVE and leave the message untouched (an earlier draft would have
  expunged the sole membership — caught in review, now regression-tested).

## SASL XOAUTH2 (alo-imap IMAP + alo-smtp submission)

- 2026-08-26 · **XOAUTH2, not OAUTHBEARER** · we implement the de-facto
  `XOAUTH2` mechanism (published by Google, shipped by Thunderbird and the
  major mobile clients), not RFC 7628 `OAUTHBEARER` — real MUAs implement
  XOAUTH2 first and some implement nothing else. Tokens are our own OIDC
  access tokens, verified through the same `resolve_access_token` seam the
  RFC 7662 introspection endpoint wraps (ADR 0025), so revocation/expiry
  bite on the next connection. The asserted `user=` must resolve to
  exactly the token's `(tenant, user)`; any mismatch fails like a bad
  token (no oracle). POP3 deliberately has no XOAUTH2 (no client demand);
  app passwords cover it.
- 2026-08-26 · **The base64 shape that trips every implementer** · the
  client response is ONE base64 blob over the whole string, not
  per-field: `base64("user=" user "^Aauth=Bearer " token "^A^A")` where
  `^A` is byte 0x01 (not the two characters `^` `A`, not `\n`, not NUL).
  The trailing `^A^A` is required by the published spec; we accept its
  absence, ignore extra `key=value` fields (`host=`/`port=` appear in the
  wild), and match `Bearer` case-insensitively. Example, token `ya29.x`
  for `user@example.com`:
  `dXNlcj11c2VyQGV4YW1wbGUuY29tAWF1dGg9QmVhcmVyIHlhMjkueAEB`.
- 2026-08-26 · **The failure dialog is part of the mechanism** · on a bad
  token the server does NOT reply `NO`/`535` immediately: it first sends a
  continuation carrying a base64 JSON error status, the client answers
  with one empty line, and only then comes the protocol-level rejection.
  Clients hang or mis-report if the extra round-trip is skipped. Ours is
  `{"status":"401","schemes":"bearer","scope":""}` (clients act on
  `status` only). Malformed blobs (bad base64, missing fields, control
  chars in `user=`) are a protocol error instead: IMAP `BAD` / SMTP `501`,
  with no error-status dialog.
- 2026-08-26 · **IMAP exchange** (implicit TLS; SASL-IR form, RFC 4959 —
  `SASL-IR` and `AUTH=XOAUTH2` are advertised post-TLS):

  ```
  C: a1 AUTHENTICATE XOAUTH2 dXNlcj11c2VyQGV4YW1wbGUuY29tAWF1dGg9QmVhcmVyIHlhMjkueAEB
  S: a1 OK [CAPABILITY IMAP4rev2 ...] LOGIN completed        (live token)

  C: a2 AUTHENTICATE XOAUTH2 dXNlcj1...                      (revoked token)
  S: + eyJzdGF0dXMiOiI0MDEiLCJzY2hlbWVzIjoiYmVhcmVyIiwic2NvcGUiOiIifQ==
  C:                                                          (empty line)
  S: a2 NO [AUTHENTICATIONFAILED] invalid credentials
  ```
- 2026-08-26 · **SMTP submission exchange** (after STARTTLS; EHLO
  advertises `AUTH PLAIN LOGIN XOAUTH2`):

  ```
  C: AUTH XOAUTH2 dXNlcj11c2VyQGV4YW1wbGUuY29tAWF1dGg9QmVhcmVyIHlhMjkueAEB
  S: 235 2.7.0 Authentication successful                     (live token)

  C: AUTH XOAUTH2 dXNlcj1...                                 (revoked token)
  S: 334 eyJzdGF0dXMiOiI0MDEiLCJzY2hlbWVzIjoiYmVhcmVyIiwic2NvcGUiOiIifQ==
  C:                                                          (empty line)
  S: 535 5.7.8 Authentication credentials invalid
  ```
  Without an initial response (`AUTH XOAUTH2` alone / IMAP without IR),
  the server prompts with an empty challenge (`334 ` / `+ `) first.

## Inbound local delivery (SMTP → store)

- 2026-07-28 · **Unknown local user is refused `550 5.1.1` at RCPT** · when
  local delivery is configured (MX + `DATABASE_URL` + hosted domains), each
  hosted-domain `RCPT TO:` is resolved against the store; an unknown mailbox
  gets `550 5.1.1 No such user here` at RCPT, not a post-DATA drop or bounce.
  This is a deliberate recipient-enumeration oracle (a prober learns which
  local addresses exist) — the conscious, mainstream choice: silent-accept-
  then-drop loses mail the sender was told 250 for, and post-DATA bounces are
  backscatter. Enumeration is mitigated at the edge (rate limits), not by
  lying to senders. A non-local recipient is still `550 5.7.1 Relaying denied`
  first (the two policies compose). See `docs/design/local-delivery.md`.
- 2026-07-28 · **Every accepted recipient — including `<postmaster>` — is
  resolved at RCPT** · so no recipient accepted at RCPT can turn out
  unresolvable at DATA and defer the whole message (which would let a hostile
  sender append a never-resolvable recipient to force repeated duplicate
  delivery to the valid ones). `<postmaster>` (RFC 5321 §4.1.1.3) resolves as
  `postmaster@` the first hosted domain and therefore needs a backing mailbox
  to receive.
- 2026-07-28 · **Multi-recipient partial failure → one 4xx (dup over loss)** ·
  DATA carries a single reply (RFC 5321 §4.1.1.4). A transient store fault for
  any recipient returns `451 4.3.0` for the whole message so the sender
  retries — a redelivery to an already-committed recipient (a duplicate,
  deduped to one blob by content-addressing) is strictly safer than losing
  mail (RFC 5321 §6.1). Per-recipient DSN is additive later.
- 2026-07-28 · **Local delivery uses a durable filesystem blob backend** ·
  message bytes are written to `ALO_SMTP_BLOB_DIR` (default `./blobs`) via
  the store's on-disk backend, so a delivered body survives a restart (the DB
  row is only the commit point after the blob is durable). Multi-node
  production swaps in Garage/S3 behind the store's `garage` feature.
- 2026-07-28 · **Inbound local mail bypasses the spool; the spool stays the
  outbound queue** · received mail for a hosted domain lands in the store, not
  the relay spool. Any pre-existing all-local spool entries are migrated into
  the store once at startup (before the outbound queue runner starts); a
  crash between deliver and spool-removal re-delivers a deduped duplicate on
  the next start, never a loss.

## Sieve filtering (alo-sieve)

- 2026-07-27 · **Sieve runs at the store delivery entry; SMTP local delivery
  is M5** · the engine filters at `AccountStore::deliver_sieve` — the ingestion
  boundary, after spam scoring, before filing. SMTP still *spools* inbound mail
  rather than delivering it into the store (local delivery is M5, a separate
  ROADMAP item), so the swaks → SMTP → mailbox wire is not yet closed; the seam
  is ready (`Store::account_by_email` + `deliver_sieve`). Until M5, the Sieve
  path is exercised through that entry (migration tool, tests), and the
  redirect/vacation `OutboundAction`s are returned to the caller for the future
  bridge to enqueue.
- 2026-07-27 · **Rule management is JMAP for Sieve, not ManageSieve** · ADR 0007:
  `SieveScript/{get,set,validate}` over the same bearer-auth account door as
  Mail, compile-checked on `set` (`invalidScript`). ManageSieve (RFC 5804) is a
  deliberate rejection (additive later). One documented deviation from RFC 9661:
  script `content` is carried **inline** rather than via a `blobId` round-trip
  (blob-based content is additive).
- 2026-07-27 · **fileinto auto-create is OFF; a missing folder keeps to Inbox** ·
  RFC 5228: `fileinto "Nope"` to a non-existent mailbox is not created — it
  degrades to implicit keep into the Inbox with a logged warning, so a typo
  never spawns folders and mail is never lost.
- 2026-07-27 · **No script failure loses mail** · RFC 5228 §2.10.6: a compile
  error on the active script, an evaluation budget overrun, or an unperformable
  action all fall back to implicit keep into the Inbox — never a bounce, never a
  drop.
- 2026-07-27 · **imap4flags map to JMAP keywords** · Sieve/IMAP flags are mapped
  to the store's canonical keywords (`\Seen` → `$seen`, RFC 8621 §4.1.1) before
  filing; `\Recent` and unknown system flags are unsettable and dropped rather
  than persisted as a bogus `\`-keyword.
- 2026-07-27 · **Redirect and vacation are bounded at the source** · a runaway
  or hostile script cannot storm: per-script redirect cap (3), per-account
  redirect rate budget, loop guards (Auto-Submitted / null return-path /
  Received-count ceiling), self-redirect refusal; vacation carries the full
  RFC 3834 guard set and per-correspondent `:days` suppression. A per-account
  vacation-send budget and alias-aware self checks are recorded follow-ups (the
  vacation path is 1:1 with no amplification and its outbound is M5-deferred).

## Identity / OIDC (alo-identity)

- 2026-07-28 · **`/oauth/authorize` is POST-only (credential submission), not
  a browser `GET`** · the first-party web app renders its own login page and
  POSTs `username`/`password`(/`otp`) plus the OAuth parameters; the endpoint
  validates the client + exact `redirect_uri` before any redirect, then 303s
  back with the code. A third-party RP that does the classic browser `GET`
  redirect to `authorization_endpoint` gets `405` until a GET consent/login
  render lands (Phase-2 webmail). Recorded because discovery advertises the
  endpoint and the "trust alo as your IdP" goal implies external RPs.
- 2026-07-28 · **ID tokens are signed EdDSA (Ed25519), never RS256** · the
  JWKS publishes `{"kty":"OKP","crv":"Ed25519","alg":"EdDSA"}` and discovery
  advertises `id_token_signing_alg_values_supported:["EdDSA"]`. An RP whose
  JWT library is RS256-only cannot verify our ID tokens — a deliberate cost
  to stay on the audited pure-Rust `ring`/`dalek` stack and avoid the `rsa`
  crate's Marvin vulnerability (RUSTSEC-2023-0071). RS256 is additive to the
  JWKS later if a customer's RP needs it (ADR 0008).
- 2026-07-28 · **Access tokens are opaque and revocable, not self-validating
  JWTs** · a resource server cannot verify an access token offline; it calls
  `/oauth/userinfo` (or resolves via the store). This is what makes logout /
  `/oauth/revoke` actually invalidate a token immediately (verified on the
  wire: revoke → `200`, next userinfo → `401`). The **ID token** is the JWT an
  RP verifies against the JWKS.
- 2026-07-28 · **2FA is enforced on the OIDC/browser flow, not on legacy
  IMAP/SMTP/POP3** · those single-exchange AUTH protocols cannot prompt for a
  TOTP code, so a 2FA-enabled user authenticates them with the account
  password (the documented interim); app-specific passwords + `XOAUTH2` are
  the follow-up. Wrong credentials are refused constant-time with no
  user-existence oracle (IMAP `[AUTHENTICATIONFAILED]`, SMTP `535`, OAuth
  `access_denied` — never distinguishing unknown-user from wrong-password).

## RFC 2047 encoded-word subjects are stored/displayed raw

Observed while verifying JMAP send: a message with a non-ASCII subject (e.g.
"café déjà prêt") is stored in `messages.subject` as its raw RFC 2047
encoded-word form (`=?UTF-8?B?…?=`), so the JMAP `Email/get` `subject` and the
web UI show the encoded text rather than the decoded string, and full-text
search indexes the encoded form. Recipients are unaffected (Gmail/Outlook
decode encoded-words on display), so this is a *local display/index* gap, not a
send bug — the outgoing header is correct (RFC 2047 is exactly how non-ASCII
subjects must be encoded on the wire).

Fixed: `alo-store` now RFC 2047-decodes the `subject` and the `From`/`To`
display names at ingestion (`rfc2047::decode`, applied in `message::parse`) —
any charset via `encoding_rs`, `B`/`Q` encodings, with adjacent-encoded-word
whitespace dropped (§6.2). The addr-spec inside `<…>` is never an encoded-word,
so addresses are untouched; the raw message bytes are unchanged (fidelity
preserved). Applies to newly-ingested mail (received + our own filed copies); a
backfill of rows stored before this landed is a separate, optional migration
(none in production but the test data).

## Contacts / vCard 4.0 (alo-store::vcard)

Our vCard support (RFC 6350) is scoped to the fields the address book
models: `FN` (required), `N` (Family/Given), `EMAIL`, `TEL` (both with
`TYPE=` labels), `ORG`, `TITLE`, `NOTE`, and `UID`. On **read** we unfold
per §3.2, unescape per §3.4, honor the first `TYPE=` label, take the first
component of structured `N`/`ORG`, and silently skip every other property
(`ADR`, `GEO`, `KEY`, embedded `PHOTO`, `X-*`, …) — a foreign card imports
cleanly, it just loses the fields we don't model. On **write** we emit only
that profile, VERSION:4.0, CRLF line endings, 75-octet folding, and drop any
`TYPE=` label that isn't `[A-Za-z0-9-]{1,32}` so a stored label can never
inject structure into a line. A card with no `FN` falls back to `N` or the
first `EMAIL` for the display name; a card with neither a name nor an email
is not a contact (`from_vcard` returns `None`). Deliberate deviation: we do
not preserve unknown properties across an import→export round-trip (a strict
CardDAV server would); acceptable until the CardDAV sync layer, which will
need to store the raw card to be fully lossless.

## CardDAV (alo-jmap::carddav, RFC 6352)

Contacts sync natively to phones and desktops over CardDAV, one addressbook
collection per account at `/dav/addressbooks/<user>/default/` (objects are
`<contactId>.vcf`). Auth is HTTP Basic via the same `authenticate_legacy`
path as IMAP/SMTP (a 2FA account is refused there). The RFC 6578 sync-token
IS the account modseq (`urn:alo:contacts:<modseq>`), so `sync-collection`
maps onto `AccountStore::changes`; per-object ETags are a content hash of
the serialized vCard (a no-op PUT does not churn the client). The href is
authoritative: a `PUT` stores under the path id whatever `UID` the card
carries.

Implemented methods: `OPTIONS`, `PROPFIND` (principal / home / addressbook
/ object), `REPORT` (`addressbook-multiget`, `sync-collection`), `GET`,
`PUT` (with `If-Match`/`If-None-Match`), `DELETE`. Deliberate scope cuts:
- `addressbook-query` filters are **not** evaluated — such a REPORT returns
  the whole collection (a valid, unfiltered result the client narrows).
  Clients sync fine via multiget + sync-collection, so this is cosmetic.
- No `PROPPATCH`, `MKCOL`, or `MOVE`: the single addressbook is fixed, not
  client-created.
- `contacts.id` is a global column, so a client-chosen href that collides
  with another account's id is refused (`409`) rather than silently
  cross-writing — astronomically rare with UUID hrefs, and safe by design.
The eventual clean home is a dedicated `alo-dav` crate (per ROADMAP);
today it is a module in alo-jmap, reusing its auth + store wiring.

## CalDAV (alo-jmap::carddav, RFC 4791)

The calendar syncs natively to Apple Calendar / iOS, Android (via a CalDAV
app), and Thunderbird, riding the **same handler, auth, and discovery** as
CardDAV — a collection per calendar the caller can see under
`/dav/calendars/<user>/` (the personal one keeps the `default` segment; a
shared calendar and a room are their own, see below — objects are
`<eventId>.ics`), and the
principal advertises `calendar-home-set` alongside `addressbook-home-set` so a
client discovers whichever it asks for. `.well-known/caldav` → `/dav/`. The
sync-token is the account modseq (`urn:alo:calendar:<modseq>`) filtered to
`Event` changes, so `sync-collection` maps onto `AccountStore::changes` — the
one exception is a room, whose token is a hash of its members' ETags because a
room fills up without the caller's modseq ever moving (AS.4b, below).
Per-object ETags hash the event **fields** plus its per-occurrence override
set (not the serialized iCalendar, whose `DTSTAMP` changes each render —
hashing that would churn every sync; leaving the overrides out would let an
instance-only edit keep the old tag and strand every other device on its
cached copy).

Implemented methods mirror CardDAV: `OPTIONS` (advertises `calendar-access`),
`PROPFIND` (principal / calendar-home / calendar / object), `REPORT`
(`calendar-multiget`, `calendar-query`, `sync-collection`), `GET`, `PUT`
(`If-Match`/`If-None-Match`), `DELETE`. Deliberate scope cuts:
- **iCalendar (RFC 5545) is minimal**: `UID`, `SUMMARY`, `DESCRIPTION`,
  `LOCATION`, `DTSTART`/`DTEND` as UTC (`…Z`), zone-qualified (`;TZID=` — see
  the time-zones note below), or all-day (`VALUE=DATE`). A floating time is
  read as **UTC** (documented cut). Recurrence (`RRULE`/`RDATE`/`EXDATE`),
  attendees, and display alarms are modelled; `VTODO`/`VJOURNAL` are not.
- A `calendar-query`'s `<C:time-range>` **is** evaluated (see "Time-range
  filtering" below); the rest of its filter — nested `comp-filter`,
  `prop-filter`, `text-match` — is not, so a query narrowed by anything else
  gets the whole (range-filtered) collection back, a valid result the client
  narrows further, exactly as CardDAV does for `addressbook-query`.
- No `PROPPATCH`/`MKCALENDAR`/`MOVE`: the set of collections is what the
  account can see, not what a client creates (`405 Method Not Allowed`).
- **Per-occurrence overrides (`RECURRENCE-ID`) sync both ways.** A series is
  served as the master `VEVENT` plus one `VEVENT` per edited instance (its own
  `RECURRENCE-ID` at the original slot), so phones render "this event" edits
  (GET and the `calendar-data` in multiget/REPORT). A phone-*originated* edit
  is captured too: `from_ics_series` reads every `VEVENT` of a PUT (RFC 5545
  §3.8.4.4, RFC 4791 §4.1) — the one without a `RECURRENCE-ID` is the master,
  each one with a `RECURRENCE-ID` upserts an override at that slot, an
  instance marked `STATUS:CANCELLED` becomes an `EXDATE`, and an override the
  client no longer sends is removed (the PUT body is the whole resource).
  Tolerances, chosen over guessing: a document holding only override
  instances keeps the first as the event; a duplicated slot keeps the last
  `VEVENT`; a `RANGE=THISANDFUTURE` parameter is read as a single-instance
  override (splitting a series is what mainstream clients do instead —
  revisit if a real client sends it). Reminders sync as a `VALARM` (display,
  negative `TRIGGER`). `EXDATE` (skip-one) round-trips both ways.
- **Recurrence rules:** `FREQ`+`INTERVAL`+`COUNT`/`UNTIL`+`BYDAY`+`BYMONTHDAY`
  expand (weekly Mon/Wed/Fri, monthly n-th/last weekday, month-day incl. `-1`);
  `BYMONTH`/`BYSETPOS`/`WKST` are ignored (Monday week-start assumed).
  `RDATE` adds extra occurrence instants (deduplicated against the rule; an
  RDATE-only series works too); `RDATE;VALUE=PERIOD` values are **skipped** —
  start/duration pairs are not modelled. A series whose `DTSTART` carries a
  resolvable `TZID` expands in that zone's **wall-clock**: a 09:00
  Europe/Brussels weekly stays 09:00 local across a DST switch (the UTC
  instant shifts), matching what the client's own expansion shows. `UNTIL` is
  compared as UTC per RFC 5545 §3.3.10. Expansion is one store function shared
  by the Agenda range listing, availability, and CalDAV time-range narrowing —
  never two implementations.
- **Multiple collections:** every calendar the user can see is its own CalDAV
  collection — the personal calendar keeps the backward-compatible `default`
  segment, and each shared/team calendar is served at
  `calendars/<uid>/<calendarId>/`. `PROPFIND` on calendar-home (Depth 1)
  enumerates them (name + colour from the calendar; a view-only shared calendar
  advertises only `read` in `current-user-privilege-set`). A PUT or DELETE
  against a calendar the caller can't edit is refused by the store (`can_edit`),
  not misfiled — on the wire that is **`403`** when the calendar is visible (a
  read-only grant: the denial is a permission) and **`404`** when it is not
  (an unshared calendar id stays unprobeable; it was a raw 500 before the
  M3.1 pass).
  Sync-token is still the account-wide modseq, filtered per collection by the
  event's calendar — so each collection syncs independently, at the cost of a
  no-op sync round when another calendar changed.
- **Time-range filtering:** a `calendar-query` with `<C:time-range>` is now
  honoured (events are filtered to the window; a recurring master is kept only
  if the store's expansion — the same function the Agenda uses — actually
  yields an occurrence in the window, or a per-occurrence override moved one
  into it). No range → whole collection, as before.
- **Free/busy (`free-busy-query`, RFC 4791 §7.10):** a REPORT on a calendar
  collection answers `200` with a `text/calendar` body — one `VFREEBUSY`
  (RFC 5545 §3.6.4) carrying the queried window as `DTSTART`/`DTEND` and one
  `FREEBUSY;FBTYPE=BUSY` UTC period per busy span (clamped to the window,
  overlaps merged; the same store expansion and the same merge the Agenda's
  scheduling grid uses). The serializer has no field for event detail, so
  titles cannot leak by construction — proven by a cross-account test: a
  viewer-role share gets periods, never `SUMMARY`. A REPORT without a
  parseable `<C:time-range>` is `400` (the range is the query — RFC 4791
  §9.11 requires exactly one). Deliberate cut: `TRANSP` is not modelled, so
  every event counts as busy — a "free"-marked (transparent) event, e.g. an
  all-day birthday, still blocks; revisit if a real-client pass objects.
- **Time zones:** a `TZID=`-qualified `DTSTART`/`DTEND`/`EXDATE`/`RDATE` is
  converted from that IANA zone to UTC at rest, and (since M3.2) the zone is
  **kept** on the event so recurrence expands in its wall-clock. Serving goes
  the other way: a zoned event's date-times are written back as
  `;TZID=<zone>:<local>` so the client's own expansion is DST-correct too.
  Since AS.2 a served document whose date-times carry a `TZID` also includes
  one `VTIMEZONE` per zone (RFC 5545 §3.6.5, §3.2.19), built from jiff's zone
  data: the `STANDARD`/`DAYLIGHT` observances in force across the object's
  span — the rule holding at its start plus each transition inside it, never
  the zone's whole history. An open-ended (or `COUNT`-bounded) recurrence
  extends that span one year past the last instant the object references, so
  a client's near-future expansion finds every switch defined; beyond it the
  IANA name remains the definition, as it always was. A fixed-offset zone is
  one `STANDARD` block dated at the epoch. **Incoming `VTIMEZONE` blocks stay
  ignored** — the IANA name in the `TZID` parameter is authoritative, and a
  shipped block is never echoed back. A client that writes a Windows display
  name (`Eastern Standard Time`) or a floating time still falls back to
  UTC-fixed behaviour (no `VTIMEZONE` — nothing zoned is served), and the
  fallback drops the unknown name so expansion and serving agree.
- **Rooms and resources (AS.4):** a bookable resource is a `calendars` row of
  kind `resource` with an address of its own; a meeting books it by carrying
  that address as an `ATTENDEE`, and the store refuses the save when any
  occurrence collides with a booking the room already holds. A refusal is `409`
  with the room's name and the taken slot as RFC 3339 UTC.
  `POST /calendar/freebusy` answers for a room's address exactly as for a
  person's, tagged `"kind":"resource"` with an empty `outsideHours` — a room
  keeps no working hours. A resource calendar stays **outside** every
  visibility predicate, so a room's bookings never land in a colleague's week
  grid; the room's own collection (below) is the one door onto them.
- **A room's collection (AS.4b):** each resource is served as a **read-only**
  calendar collection to every member of the tenant, at
  `calendars/<uid>/<resourceId>/`. `PROPFIND` on calendar-home lists it beside
  the personal and shared calendars, with the room's name as `displayname`, its
  location as `calendar-description` (RFC 4791 §5.2.1) and only `read` in
  `current-user-privilege-set`. Its members are the meetings that booked it,
  whoever owns them, with hrefs under the **room's** segment — an href is built
  from the collection being listed, not from the calendar the event sits on, so
  a room's client is never pointed at a colleague's collection. `GET`,
  `calendar-multiget`, `calendar-query` (with `time-range`) and
  `free-busy-query` all answer there; a booking is readable through the room
  and stays unreadable through anything else. Every write is refused **`403`**:
  a room's schedule is written by booking it, not by PUTing into it, and that
  holds for the admin whose row created it. Deliberate consequence, recorded
  because it is a disclosure: a room's collection shows the **titles** of
  colleagues' bookings, which is how a shared room calendar works everywhere —
  free/busy (times only) remains what the week grid and Sites see.
  - The served `ATTENDEE` for a room carries `CUTYPE=ROOM` (RFC 5545 §3.2.3),
    with `RSVP=FALSE;PARTSTAT=ACCEPTED` — a room was booked, and the booking is
    the answer; every other attendee is unchanged. The match is on the address,
    case-insensitively. Incoming `CUTYPE` is ignored: what a room *is* is the
    tenant's resource list, never a parameter a client sends.
  - A resource attendee arriving on a **CalDAV PUT** now books the room through
    the same `book_resources` check the Agenda and the JSON API use, taken
    *before* the write, so a refusal leaves nothing behind. A collision answers
    **`409`** (RFC 4791 §5.3.2 leaves the code to the server) with the store's
    own sentence — the room's name and the taken slot — as a `text/plain` body.
    The PUT body is the whole resource, so a room dropped from the guest list is
    released by the same PUT.
  - **Sync:** a room's `sync-token` and `getctag` are a **hash of its members'
    ETags** (`urn:alo:room:<hash>`), not the account modseq every other
    collection uses — a room's members are other people's meetings, whose writes
    never touch this caller's modseq, so a modseq token would sit still while
    the room filled up. Consequences, per RFC 6578 §3.2: an initial
    `sync-collection` (no token) returns every member; the current token returns
    no changes; any other token is answered `403 DAV:valid-sync-token`, sending
    the client to a full listing — the same round a changed ctag would cause.
    Deletions are therefore reported by absence from that listing, not as `404`
    responses. Revisit if a room ever holds enough bookings for a full listing
    to hurt.
- **Round-trip corpus** (`alo-store/tests/ical_corpus.rs`): client fixtures —
  plain UTC, all-day, `TZID=Europe/Brussels` zoned, floating, §3.3.11-escaped
  text, folded long lines, (M3.2) weekly-with-exceptions, monthly-by-day
  with an `RDATE`, a Europe/Brussels DST-crossing recurring series, and
  (AS.1) an Apple-style two-`VEVENT` series with a shipped `VTIMEZONE` and a
  moved instance plus a DAVx⁵-style cancelled instance, and (AS.2) a
  fixed-offset-zone event, with every zoned canonical form carrying the
  served `VTIMEZONE` — each
  parse → store (real Postgres, the CalDAV PUT path) → serialize to checked-in
  canonical bytes, and the canonical form is a fixed point of another full
  cycle. `DTSTAMP` is the one property that derives from nothing in the event
  (RFC 5545 §3.8.7.2: the serialization instant), so the corpus pins it
  through `ical::to_ics_at`; live responses stamp the current time.

Wire-verified on the live server (principal discovery, PUT/GET/REPORT/
sync-collection/DELETE, sync-token advancing on writes), and CI-gated
end-to-end by `alo-jmap/tests/caldav.rs` — the full client sequence plus
per-method wrong-tenant, wrong-user-same-tenant, and read-only-share proofs.

## Invitations: iTIP over iMIP (RFC 5546 / RFC 6047)

Scheduling messages are ordinary mail through the one submission door (the
internal listener): a `multipart/alternative` of a short plain-text note and a
base64 `text/calendar; method=REQUEST|REPLY|CANCEL` part. Deliberate shapes
and deviations, so the next implementer inherits knowledge not debugging:

- **Outbound.** Saving an event with attendees mails each a `METHOD:REQUEST`
  from the organizer's address; re-saving re-issues it (same `UID` — clients
  treat it as an update). Editing or deleting **one instance** of a series
  sends a REQUEST/CANCEL carrying the same `UID` plus a `RECURRENCE-ID` at the
  instance's original slot and no `RRULE`, so clients change only that one.
  Sends are best-effort after the calendar write (a dead listener never fails
  the save; it is logged).
- **Inbound application is read-time, not delivery-time.** RFC 6047 imagines
  the receiving agent processing iMIP on arrival; alo parses the
  `text/calendar` part when the message is *read* (`Email/get` surfaces
  `alo:invitation`) and the reading-pane card acts: Accept/Maybe/Decline posts
  `/calendar/rsvp` (stores the event on the personal calendar unless declining
  — keyed on the organizer's `UID`, so a changed mind re-RSVPs in place — and
  mails the `METHOD:REPLY` back), a reply card applies the guest's `PARTSTAT`
  onto the organizer's event on mount, a cancellation card removes on mount.
  All three re-read the message server-side by its account-scoped blob — a
  client cannot name an arbitrary event id, only what a message it owns says.
- **A CANCEL naming a `RECURRENCE-ID` removes the instance, not the series**
  (RFC 5546 §3.2.5): the recipient's stored series gains an `EXDATE` at that
  slot and everything else stays; a CANCEL without one removes the whole
  event by `UID`. Accepted value shapes: UTC (`…Z`), `VALUE=DATE`, and
  `TZID=<zone>` wall-clock. Cancelling what is not on the calendar (declined,
  already removed) is honoured success (`removed:false`), never an error.
- **A REPLY speaks for one attendee**: the first `ATTENDEE` line is read
  (email + `PARTSTAT`, defaulting `NEEDS-ACTION`); it applies only to an
  event the reader can edit (their organized copy, matched by `UID`) —
  otherwise a clean `applied:false`, never another account's data.
- CI-gated end-to-end by `alo-jmap/tests/invitations_http.rs`: the full
  REQUEST→REPLY round trip across two accounts over a real local stack (real
  routes, real Postgres, a real SMTP dialog to an in-process sink), the
  instance-vs-series CANCEL distinction, and foreign-blob 404s on every door.

## IMAP import (client role, RFC 3501)

The **Import mail** wizard makes alo an IMAP *client* (distinct from the
alo-imap *server*): it logs into a remote mailbox and copies recent mail
into the user's alo mailboxes. `POST /import/imap` → `imap_import`.
Deliberate, recorded scope and quirks:

- **All selectable folders, newest ≤500 messages total.** We `LIST "" "*"`,
  then for each folder (INBOX first, then Sent/Drafts/Junk/Trash/Archive,
  then user folders) `SELECT` + `FETCH (FLAGS BODY.PEEK[])` the tail, until
  the shared 500-message budget is spent. `BODY.PEEK[]` does not set `\Seen`
  on the source. Folder mapping: **special-use** (RFC 6154 `\Sent`/`\Drafts`/
  `\Junk`/`\Trash`/`\Archive`) → the alo mailbox of that role (get-or-
  created); otherwise a top-level mailbox created by the folder's **leaf**
  name (`[Gmail]/Work` → `Work`). **Flags carried over:** `\Seen`→`$seen`,
  `\Flagged`→`$flagged`, `\Answered`→`$answered`, `\Draft`→`$draft`.
- **Gmail's virtual folders are skipped.** `\All` (All Mail), `\Flagged`
  (Starred), and `\Important` overlap the real folders; importing them would
  store every message a second time, so they are excluded. Non-selectable
  (`\Noselect`/`\NonExistent`) folders are skipped too. A message that still
  appears in two imported folders is stored **once** (dedup is per run, not
  just against the store).
- **Folder names are taken verbatim.** Modified-UTF-7 (RFC 3501 §5.1.3)
  mailbox names are not decoded, so a non-ASCII remote folder keeps its
  wire-encoded name; ASCII names (the overwhelming majority) are unaffected.
  Decoding is a follow-up.
- **Implicit-TLS only (port 993), certificate verified.** The user's
  password crosses the wire, so the TLS server certificate is verified
  against the Mozilla root set (webpki-roots) — an unverified/accept-any
  connection is refused, not downgraded. Cleartext-then-STARTTLS (143) is
  not offered.
- **Dedup is by `Message-ID`, stored with angle brackets.** A re-import
  skips any message whose `Message-ID` already exists for that user, so the
  wizard is idempotent for well-formed mail. **A message with no
  `Message-ID` header cannot be deduped** and is imported on every run —
  importing it is the safe choice (dropping it would silently lose mail);
  such messages are rare in practice (most MTAs stamp one).
- **Gmail/Outlook require an app password.** Both refuse a normal account
  password over IMAP `LOGIN`; the wizard's provider hint says so, and an
  auth refusal maps to `401` with that guidance rather than a raw error.
- **SSRF-guarded.** The target host is resolved and refused if it maps to a
  loopback/private/link-local address (shared `alo_ai::egress` guard), and
  the connection is pinned to the vetted IP — a hostname cannot be used to
  reach an internal service.

## Mail-client autoconfig (client self-configuration)

Two unauthenticated, read-only endpoints let a mail app configure itself from
the user's email address (`autoconfig`), serving the same public facts —
IMAPS `993` and SMTPS `465` on the server FQDN, password auth inside TLS — in
the two formats clients ask for:

- **Mozilla autoconfig** (Thunderbird; Apple Mail as a fallback):
  `GET /.well-known/autoconfig/mail/config-v1.1.xml` and
  `GET /mail/config-v1.1.xml`, a `clientConfig` document. The `?emailaddress=`
  query names the provider domain (multi-domain deployments answer per
  domain); the username is the literal `%EMAILADDRESS%` placeholder
  Thunderbird substitutes, so caller input is never echoed.
- **Microsoft POX Autodiscover** (Outlook): `GET`/`POST
  /autodiscover/autodiscover.xml` (both path casings registered — axum is
  case-sensitive, Outlook varies it), an `Autodiscover` document with IMAP +
  SMTP `<Protocol>` blocks. The POSTed `<EMailAddress>` is echoed as
  `<LoginName>` only if it is a sane `local@domain` with no markup; otherwise
  the element is omitted and Outlook falls back to the typed address.

Both reveal only public connection settings, so they are unauthenticated by
design (the specs require it — the client has no credentials yet). Any
caller-supplied value is XML-escaped and charset-validated before it reaches
the document, so a hostile `emailaddress`/`EMailAddress` cannot inject markup.

**Operator DNS (per email domain, not the server FQDN).** Clients look under
the *email* domain, so discovery needs records pointing that domain's
`autoconfig`/`autodiscover` names (and, for the well-known path, the bare
domain) at this server, plus Caddy vhosts for them. This is deployment wiring,
documented in `deploy/production/README.md`; the endpoints themselves are
verifiable directly on the server origin.

## Scripted wire transcripts (generated evidence)

The canonical exchanges below are generated by `bash
scripts/wire-transcripts.sh`, which drives the in-repo servers over real
local sockets (the same transcript tests gate in each crate's suite, so a
transcript cannot go green while drifting from behaviour). Re-run the script
after any protocol change; it regenerates everything between the markers.
The GUI-client passes (real Thunderbird / Apple Mail / Gmail app clicked by
hand) are owner-gated and tracked in the mail track's STATE.md.

<!-- wire-transcripts:begin -->

Generated 2026-08-29 by `bash scripts/wire-transcripts.sh`.
TLS transcripts show the decrypted stream of a real rustls session; the
DAV exchanges are the literal HTTP/1.1 bytes (the production proxy
terminates TLS in front of them). Credentials and bearer blobs are
redacted, ids and addresses normalised; `(…)` lines are annotations,
not wire bytes.

### IMAP over implicit TLS: LOGIN / SELECT / FETCH / STORE / IDLE

```text
S: * OK [CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=XOAUTH2 SASL-IR IDLE MOVE UIDPLUS LITERAL+ SPECIAL-USE ENABLE NAMESPACE] alo IMAP ready
C: a1 LOGIN "alice@example.test" "<password>"
S: a1 OK [CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=XOAUTH2 SASL-IR IDLE MOVE UIDPLUS LITERAL+ SPECIAL-USE ENABLE NAMESPACE] LOGIN completed
C: a2 SELECT INBOX
S: * 1 EXISTS
S: * 0 RECENT
S: * FLAGS (\Answered \Flagged \Deleted \Seen \Draft)
S: * OK [PERMANENTFLAGS (\Answered \Flagged \Deleted \Seen \Draft \*)] Limited
S: * OK [UIDVALIDITY 1142] UIDs valid
S: * OK [UIDNEXT 2] Predicted next UID
S: * OK [UNSEEN 1] First unseen
S: a2 OK [READ-WRITE] SELECT completed
C: a3 FETCH 1 (UID FLAGS RFC822.SIZE ENVELOPE)
S: * 1 FETCH (UID 1 FLAGS () RFC822.SIZE 153 ENVELOPE (NIL "Quarterly figures" ((NIL NIL "sender" "example.test")) ((NIL NIL "sender" "example.test")) ((NIL NIL "sender" "example.test")) ((NIL NIL "rcpt" "example.test")) NIL NIL NIL "<Quarterly figures@example.test>"))
S: a3 OK FETCH completed
C: a4 STORE 1 +FLAGS (\Seen)
S: * 1 FETCH (FLAGS (\Seen))
S: a4 OK STORE completed
C: a5 IDLE
S: + idling
  (a second message is delivered while the connection idles)
S: * 2 EXISTS
C: DONE
S: a5 OK IDLE terminated
C: a6 LOGOUT
S: * BYE alo IMAP logging out
S: a6 OK LOGOUT completed
```

### IMAP SASL XOAUTH2: capability, SASL-IR login, revoked-token error dialog

```text
S: * OK [CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=XOAUTH2 SASL-IR IDLE MOVE UIDPLUS LITERAL+ SPECIAL-USE ENABLE NAMESPACE] alo IMAP ready
C: a1 CAPABILITY
S: * CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=XOAUTH2 SASL-IR IDLE MOVE UIDPLUS LITERAL+ SPECIAL-USE ENABLE NAMESPACE
S: a1 OK CAPABILITY completed
C: a2 AUTHENTICATE XOAUTH2 <base64 of "user=alice@example.test^Aauth=Bearer <token>^A^A">
S: a2 OK [CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=XOAUTH2 SASL-IR IDLE MOVE UIDPLUS LITERAL+ SPECIAL-USE ENABLE NAMESPACE] LOGIN completed
C: a3 SELECT INBOX
S: * 0 EXISTS
S: * 0 RECENT
S: * FLAGS (\Answered \Flagged \Deleted \Seen \Draft)
S: * OK [PERMANENTFLAGS (\Answered \Flagged \Deleted \Seen \Draft \*)] Limited
S: * OK [UIDVALIDITY 1143] UIDs valid
S: * OK [UIDNEXT 1] Predicted next UID
S: a3 OK [READ-WRITE] SELECT completed
C: a4 LOGOUT
S: * BYE alo IMAP logging out
S: a4 OK LOGOUT completed
S: * OK [CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=XOAUTH2 SASL-IR IDLE MOVE UIDPLUS LITERAL+ SPECIAL-USE ENABLE NAMESPACE] alo IMAP ready
  (the same token, after revocation)
C: a1 AUTHENTICATE XOAUTH2 <base64 of "user=alice@example.test^Aauth=Bearer <token>^A^A">
S: + eyJzdGF0dXMiOiI0MDEiLCJzY2hlbWVzIjoiYmVhcmVyIiwic2NvcGUiOiIifQ==
  (decoded continuation: {"status":"401","schemes":"bearer","scope":""})
C: 
S: a1 NO [AUTHENTICATIONFAILED] invalid credentials
```

### POP3 over implicit TLS: USER / PASS / STAT / LIST / RETR / DELE / QUIT

```text
S: +OK alo POP3 ready
C: USER alice@example.test
S: +OK
C: PASS <password>
S: +OK mailbox ready
C: STAT
S: +OK 1 153
C: LIST
S: +OK scan listing follows
S: 1 153
S: .
C: RETR 1
S: +OK 153 octets
S: From: sender@example.test
S: To: rcpt@example.test
S: Subject: Quarterly figures
S: Message-ID: <Quarterly figures@example.test>
S: 
S: The numbers are attached.
S: .
C: DELE 1
S: +OK marked for deletion
C: QUIT
S: +OK alo POP3 signing off
```

### SMTP submission: STARTTLS, AUTH PLAIN, 8BITMIME transaction, SMTPUTF8 refusal

```text
S: 220 mx.alo.test ESMTP alo
C: EHLO client.example
S: 250-mx.alo.test greets client.example
S: 250-STARTTLS
S: 250-SIZE 26214400
S: 250 8BITMIME
C: STARTTLS
S: 220 2.0.0 Ready to start TLS
  (TLS handshake; the session state resets)
C: EHLO client.example
S: 250-mx.alo.test greets client.example
S: 250-AUTH PLAIN LOGIN XOAUTH2
S: 250-SIZE 26214400
S: 250 8BITMIME
C: AUTH PLAIN <base64 of "\0alice@alo.test\0<password>">
S: 235 2.7.0 Authentication successful
C: MAIL FROM:<alice@alo.test> BODY=8BITMIME
S: 250 OK
C: RCPT TO:<bob@example.org>
S: 250 OK
C: DATA
S: 354 Start mail input; end with <CRLF>.<CRLF>
C: Subject: Zahlen fürs Quartal
C: 
C: Zwölf Boxkämpfer jagen Viktor quer über den großen Sylter Deich.
C: .
S: 250 OK: queued as <spool-id>
  (SMTPUTF8 (RFC 6531) is not offered; the parameter is refused)
C: MAIL FROM:<alice@alo.test> SMTPUTF8
S: 555 MAIL FROM/RCPT TO parameters not recognized or not implemented
C: QUIT
S: 221 mx.alo.test closing transmission channel
```

### SMTP submission SASL XOAUTH2: bearer login, revoked-token error dialog

```text
S: 220 mx.alo.test ESMTP alo
C: EHLO client.example
S: 250-mx.alo.test greets client.example
S: 250-STARTTLS
S: 250-SIZE 26214400
S: 250 8BITMIME
C: STARTTLS
S: 220 2.0.0 Ready to start TLS
  (TLS handshake)
C: EHLO client.example
S: 250-mx.alo.test greets client.example
S: 250-AUTH PLAIN LOGIN XOAUTH2
S: 250-SIZE 26214400
S: 250 8BITMIME
C: AUTH XOAUTH2 <base64 of "user=alice@alo.test^Aauth=Bearer <token>^A^A">
S: 235 2.7.0 Authentication successful
C: QUIT
S: 221 mx.alo.test closing transmission channel
S: 220 mx.alo.test ESMTP alo
C: EHLO client.example
S: 250-mx.alo.test greets client.example
S: 250-STARTTLS
S: 250-SIZE 26214400
S: 250 8BITMIME
C: STARTTLS
S: 220 2.0.0 Ready to start TLS
  (TLS handshake)
C: EHLO client.example
S: 250-mx.alo.test greets client.example
S: 250-AUTH PLAIN LOGIN XOAUTH2
S: 250-SIZE 26214400
S: 250 8BITMIME
  (the same token, after revocation)
C: AUTH XOAUTH2 <base64 of "user=alice@alo.test^Aauth=Bearer <token>^A^A">
S: 334 eyJzdGF0dXMiOiI0MDEiLCJzY2hlbWVzIjoiYmVhcmVyIiwic2NvcGUiOiIifQ==
  (decoded challenge: {"status":"401","schemes":"bearer","scope":""})
C: 
S: 535 5.7.8 Authentication credentials invalid
```

### CardDAV: discovery, PUT, initial and incremental sync-collection

```text
C: OPTIONS /dav/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Connection: close
C:
S: HTTP/1.1 200 OK
S: dav: 1, 3, addressbook, calendar-access
S: allow: OPTIONS, GET, HEAD, PUT, DELETE, PROPFIND, REPORT
S: connection: close
S: content-length: 0
S: date: <date>
S:
C: PROPFIND /dav/principals/ACCOUNT/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Depth: 0
C: Connection: close
C:
S: HTTP/1.1 207 Multi-Status
S: content-type: application/xml; charset=utf-8
S: content-length: 810
S: connection: close
S: date: <date>
S:
S: <?xml version="1.0" encoding="utf-8"?>
S: <d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/">
S: <d:response>
S: <d:href>/dav/principals/ACCOUNT/</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:resourcetype>
S: <d:principal/>
S: <d:collection/>
S: </d:resourcetype>
S: <d:displayname>ACCOUNT</d:displayname>
S: <d:current-user-principal>
S: <d:href>/dav/principals/ACCOUNT/</d:href>
S: </d:current-user-principal>
S: <card:addressbook-home-set>
S: <d:href>/dav/addressbooks/ACCOUNT/</d:href>
S: </card:addressbook-home-set>
S: <cal:calendar-home-set>
S: <d:href>/dav/calendars/ACCOUNT/</d:href>
S: </cal:calendar-home-set>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: </d:multistatus>
C: PUT /dav/addressbooks/ACCOUNT/default/ada-ACCOUNT.vcf HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Type: text/vcard
C: Content-Length: 91
C: Connection: close
C:
C: BEGIN:VCARD
C: VERSION:4.0
C: FN:Ada Lovelace
C: N:Lovelace;Ada;;;
C: EMAIL:ada@eng.uk
C: END:VCARD
S: HTTP/1.1 201 Created
S: etag: "3e93621538f12ad4"
S: connection: close
S: content-length: 0
S: date: <date>
S:
C: REPORT /dav/addressbooks/ACCOUNT/default/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Length: 73
C: Connection: close
C:
C: <d:sync-collection xmlns:d="DAV:">
C: <d:sync-token/>
C: </d:sync-collection>
S: HTTP/1.1 207 Multi-Status
S: content-type: application/xml; charset=utf-8
S: content-length: 561
S: connection: close
S: date: <date>
S:
S: <?xml version="1.0" encoding="utf-8"?>
S: <d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/">
S: <d:response>
S: <d:href>/dav/addressbooks/ACCOUNT/default/ada-ACCOUNT.vcf</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:getetag>"3e93621538f12ad4"</d:getetag>
S: <d:getcontenttype>text/vcard; charset=utf-8</d:getcontenttype>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: <d:sync-token>urn:alo:contacts:1</d:sync-token>
S: </d:multistatus>
  (another client adds bob.vcf and deletes ada.vcf)
C: PUT /dav/addressbooks/ACCOUNT/default/bob-ACCOUNT.vcf HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Type: text/vcard
C: Content-Length: 87
C: Connection: close
C:
C: BEGIN:VCARD
C: VERSION:4.0
C: FN:Bob Sander
C: N:Sander;Bob;;;
C: EMAIL:ada@eng.uk
C: END:VCARD
S: HTTP/1.1 201 Created
S: etag: "8931bdbf37cbc7dd"
S: connection: close
S: content-length: 0
S: date: <date>
S:
C: DELETE /dav/addressbooks/ACCOUNT/default/ada-ACCOUNT.vcf HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Connection: close
C:
S: HTTP/1.1 204 No Content
S: connection: close
S: date: <date>
S:
C: REPORT /dav/addressbooks/ACCOUNT/default/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Length: 105
C: Connection: close
C:
C: <d:sync-collection xmlns:d="DAV:">
C: <d:sync-token>urn:alo:contacts:1</d:sync-token>
C: </d:sync-collection>
S: HTTP/1.1 207 Multi-Status
S: content-type: application/xml; charset=utf-8
S: content-length: 725
S: connection: close
S: date: <date>
S:
S: <?xml version="1.0" encoding="utf-8"?>
S: <d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/">
S: <d:response>
S: <d:href>/dav/addressbooks/ACCOUNT/default/bob-ACCOUNT.vcf</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:getetag>"8931bdbf37cbc7dd"</d:getetag>
S: <d:getcontenttype>text/vcard; charset=utf-8</d:getcontenttype>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: <d:response>
S: <d:href>/dav/addressbooks/ACCOUNT/default/ada-ACCOUNT.vcf</d:href>
S: <d:status>HTTP/1.1 404 Not Found</d:status>
S: </d:response>
S: <d:sync-token>urn:alo:contacts:3</d:sync-token>
S: </d:multistatus>
```

### CalDAV: PUT of a zoned recurring series, time-range query, VFREEBUSY, sync-collection

```text
C: PROPFIND /dav/calendars/ACCOUNT/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Depth: 1
C: Connection: close
C:
S: HTTP/1.1 207 Multi-Status
S: content-type: application/xml; charset=utf-8
S: content-length: 1350
S: connection: close
S: date: <date>
S:
S: <?xml version="1.0" encoding="utf-8"?>
S: <d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/">
S: <d:response>
S: <d:href>/dav/calendars/ACCOUNT/</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:resourcetype>
S: <d:collection/>
S: </d:resourcetype>
S: <d:displayname>Calendars</d:displayname>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: <d:response>
S: <d:href>/dav/calendars/ACCOUNT/default/</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:resourcetype>
S: <d:collection/>
S: <cal:calendar/>
S: </d:resourcetype>
S: <d:displayname>Personal</d:displayname>
S: <cs:getctag>urn:alo:calendar:0</cs:getctag>
S: <d:sync-token>urn:alo:calendar:0</d:sync-token>
S: <cal:supported-calendar-component-set>
S: <cal:comp name="VEVENT"/>
S: </cal:supported-calendar-component-set>
S: <cs:calendar-color>#e76f51ff</cs:calendar-color>
S: <d:supported-report-set>
S: <d:supported-report>
S: <d:report>
S: <d:sync-collection/>
S: </d:report>
S: </d:supported-report>
S: <d:supported-report>
S: <d:report>
S: <cal:calendar-multiget/>
S: </d:report>
S: </d:supported-report>
S: <d:supported-report>
S: <d:report>
S: <cal:calendar-query/>
S: </d:report>
S: </d:supported-report>
S: <d:supported-report>
S: <d:report>
S: <cal:free-busy-query/>
S: </d:report>
S: </d:supported-report>
S: </d:supported-report-set>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: </d:multistatus>
C: PUT /dav/calendars/ACCOUNT/default/standup-wire.ics HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Type: text/calendar
C: Content-Length: 280
C: Connection: close
C:
C: BEGIN:VCALENDAR
C: VERSION:2.0
C: PRODID:-//alo//transcript//EN
C: BEGIN:VEVENT
C: UID:standup-wire
C: DTSTAMP:20260101T000000Z
C: DTSTART;TZID=Europe/Brussels:20261019T090000
C: DTEND;TZID=Europe/Brussels:20261019T093000
C: RRULE:FREQ=WEEKLY;COUNT=3
C: SUMMARY:Standup
C: END:VEVENT
C: END:VCALENDAR
S: HTTP/1.1 201 Created
S: etag: "08a63714c3538ca2"
S: connection: close
S: content-length: 0
S: date: <date>
S:
C: REPORT /dav/calendars/ACCOUNT/default/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Depth: 1
C: Content-Length: 326
C: Connection: close
C:
C: <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
C: <d:prop><d:getetag/><c:calendar-data/></d:prop>
C: <c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT">
C: <c:time-range start="20261001T000000Z" end="20261130T000000Z"/>
C: </c:comp-filter></c:comp-filter></c:filter>
C: </c:calendar-query>
S: HTTP/1.1 207 Multi-Status
S: content-type: application/xml; charset=utf-8
S: content-length: 1206
S: connection: close
S: date: <date>
S:
S: <?xml version="1.0" encoding="utf-8"?>
S: <d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/">
S: <d:response>
S: <d:href>/dav/calendars/ACCOUNT/default/standup-wire.ics</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:getetag>"08a63714c3538ca2"</d:getetag>
S: <d:getcontenttype>text/calendar; charset=utf-8; component=VEVENT</d:getcontenttype>
S: <cal:calendar-data>BEGIN:VCALENDAR
S: VERSION:2.0
S: PRODID:-//alo//calendar//EN
S: BEGIN:VTIMEZONE
S: TZID:Europe/Brussels
S: BEGIN:DAYLIGHT
S: DTSTART:20260329T020000
S: TZOFFSETFROM:+0100
S: TZOFFSETTO:+0200
S: TZNAME:CEST
S: END:DAYLIGHT
S: BEGIN:STANDARD
S: DTSTART:20261025T030000
S: TZOFFSETFROM:+0200
S: TZOFFSETTO:+0100
S: TZNAME:CET
S: END:STANDARD
S: BEGIN:DAYLIGHT
S: DTSTART:20270328T020000
S: TZOFFSETFROM:+0100
S: TZOFFSETTO:+0200
S: TZNAME:CEST
S: END:DAYLIGHT
S: END:VTIMEZONE
S: BEGIN:VEVENT
S: UID:standup-wire
S: DTSTAMP:20260829T210522Z
S: DTSTART;TZID=Europe/Brussels:20261019T090000
S: DTEND;TZID=Europe/Brussels:20261019T093000
S: RRULE:FREQ=WEEKLY;COUNT=3
S: SUMMARY:Standup
S: END:VEVENT
S: END:VCALENDAR
S: </cal:calendar-data>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: </d:multistatus>
  (free/busy for the same window: the 09:00 Brussels series stays 09:00 local across the 2026-10-25 switch (07:00Z, then 08:00Z))
C: REPORT /dav/calendars/ACCOUNT/default/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Length: 146
C: Connection: close
C:
C: <c:free-busy-query xmlns:c="urn:ietf:params:xml:ns:caldav">
C: <c:time-range start="20261001T000000Z" end="20261130T000000Z"/>
C: </c:free-busy-query>
S: HTTP/1.1 200 OK
S: content-type: text/calendar; charset=utf-8
S: content-length: 400
S: connection: close
S: date: <date>
S:
S: BEGIN:VCALENDAR
S: VERSION:2.0
S: PRODID:-//alo//calendar//EN
S: BEGIN:VFREEBUSY
S: UID:freebusy-cal_personal_ACCOUNT
S: DTSTAMP:20260829T210522Z
S: DTSTART:20261001T000000Z
S: DTEND:20261130T000000Z
S: FREEBUSY;FBTYPE=BUSY:20261019T070000Z/20261019T073000Z
S: FREEBUSY;FBTYPE=BUSY:20261026T080000Z/20261026T083000Z
S: FREEBUSY;FBTYPE=BUSY:20261102T080000Z/20261102T083000Z
S: END:VFREEBUSY
S: END:VCALENDAR
C: REPORT /dav/calendars/ACCOUNT/default/ HTTP/1.1
C: Host: alo.example
C: Authorization: Basic <base64 of "alice@example.test:<password>">
C: Content-Length: 73
C: Connection: close
C:
C: <d:sync-collection xmlns:d="DAV:">
C: <d:sync-token/>
C: </d:sync-collection>
S: HTTP/1.1 207 Multi-Status
S: content-type: application/xml; charset=utf-8
S: content-length: 565
S: connection: close
S: date: <date>
S:
S: <?xml version="1.0" encoding="utf-8"?>
S: <d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/">
S: <d:response>
S: <d:href>/dav/calendars/ACCOUNT/default/standup-wire.ics</d:href>
S: <d:propstat>
S: <d:prop>
S: <d:getetag>"08a63714c3538ca2"</d:getetag>
S: <d:getcontenttype>text/calendar; charset=utf-8; component=VEVENT</d:getcontenttype>
S: </d:prop>
S: <d:status>HTTP/1.1 200 OK</d:status>
S: </d:propstat>
S: </d:response>
S: <d:sync-token>urn:alo:calendar:1</d:sync-token>
S: </d:multistatus>
```

<!-- wire-transcripts:end -->

## Deliverability + real-client interop (prod verification, 2026-08-02)

Verified against the live server (`mail.namel3ss.com`) driving IMAPS/SMTPS
with Python's stdlib `imaplib`/`smtplib` — the same protocols Thunderbird and
Apple Mail speak underneath.

**Trust stack** (`GET /admin/security/checks` against the email domain):

| Check | Result |
|---|---|
| SPF | pass — `v=spf1 mx -all` (strict) |
| DKIM | pass — key published at `fic._domainkey` |
| DMARC | pass — `p=quarantine; adkim=s; aspf=s` (strict alignment) |
| MX | pass — → `mail.namel3ss.com` |
| **PTR (reverse DNS)** | **FAIL** — the sending IP has no PTR. **Operator action, set at the hosting/IP provider (not DNS zone):** Gmail/Outlook spam-file or reject senders whose PTR doesn't match. This is the one open inbox-placement blocker. |
| MTA-STS | warn — not published (optional; improves inbound TLS enforcement). |

**Real-client loop** (all over TLS, one round): IMAPS `LOGIN`, `SELECT INBOX`,
`SEARCH ALL`, `SEARCH SUBJECT "<multi word>"`, `STORE ±FLAGS (\Flagged)`
round-trip; SMTPS `EHLO`/`AUTH`/submission. A self-addressed probe was
delivered and found by subject search in **~6 s**, then expunged. Send →
receive → search → flag all pass.

**Client quirk — multi-word SEARCH must be quoted (imaplib, our own tooling).**
`imaplib`'s `M.uid("search", None, "SUBJECT", "two words")` sends the argument
**unquoted** (`SUBJECT two words`), which the server *correctly* reads as
`SUBJECT "two"` AND a sequence-set `words` → zero hits. Quoting the argument
(`SUBJECT "two words"`) returns the expected matches. The alo-imap SEARCH
parser and substring evaluator are RFC-correct (verified on the wire and in
`core/alo-imap/src/search.rs`); real GUI clients quote their SEARCH strings, so
this bites only hand-rolled `imaplib` scripts — noted so our own test tooling
always quotes.

**Observation — self-send routes outbound then loops back via MX** (`delivering
outbound` → `delivered to store` in the alo-smtp log, message growing by the
added Received/DKIM headers), rather than a direct local hand-off. Harmless
for delivery; it just means a message to your own domain takes the full
send+receive path (and so ~seconds, not instant).

**Still needs external accounts (cannot self-verify here):** confirming our
mail lands in the **inbox** (not spam) at Gmail / Outlook.com / Proton from the
warmed IP — do this once the PTR record is set, since PTR is the dominant
factor. The GUI-client matrix (Thunderbird, Apple Mail, Gmail-app IMAP) beyond
the protocol-level loop above is also outstanding.

## MAPI-over-HTTP (classic Outlook)

**Specification contradiction — `MetaTagIdsetGiven`'s declared type cannot be
encoded.** [MS-OXCFXICS] §2.2.1.3 gives `MetaTagIdsetGiven` as property id
`0x4017` with data type **`PtypInteger32` (`0x0003`)**, while the same section
says its value "contains a serialization of REPLGUID-based IDSET structures" —
a variable-length structure of arbitrary size.

Both cannot be true inside a FastTransfer stream. The lexical grammar in
§2.2.4.1 is:

```abnf
propValue  = fixedPropType propInfo fixedSizeValue
propValue =/ varPropType  propInfo length varSizeValue
```

A fixed-size type carries **no length field**. A reader that meets
`0x40170003` therefore consumes exactly four bytes and resumes parsing in the
middle of the IDSET, so every element after it is garbage — silently, since
the bytes it lands on are still structurally plausible.

Every sibling state property is declared `PtypBinary`: `MetaTagCnsetSeen`
(`0x6796`), `MetaTagCnsetSeenFAI` (`0x67DA`), `MetaTagCnsetRead` (`0x67D2`),
`MetaTagIdsetDeleted` (`0x67E5`). The `0x0003` on `MetaTagIdsetGiven` is
therefore read as a documentation error.

**Our response:** we *write* `0x40170102` (`PtypBinary`), the only encoding the
grammar admits. We *read* both `0x40170102` and the declared `0x40170003`, so a
client that follows the letter of §2.2.1.3 still works. Constants and reasoning
in `products/mail/alo-mapi/src/ics.rs` (`meta::IDSET_GIVEN`,
`meta::IDSET_GIVEN_AS_DECLARED`).

Derived from the specification's own grammar, **not yet confirmed against a
real Outlook** — this is the first thing to re-check if cached mode fails to
establish. Date: 2026-08-23.

**Length widths differ between ROP buffers and FastTransfer streams.**
[MS-OXCDATA] §2.11.1: `PtypBinary` byte counts are **16 bits** "in the context
of ROP buffers"; [MS-OXCFXICS] §2.2.4.1 defines the stream's `length` as
`PtypInteger32` — **32 bits** — for every variable-size value. The two writers
in `alo-mapi` are deliberately separate for this reason
(`fasttransfer::Writer` against the ROP writers); sharing them would corrupt
every stream from its first binary property onward, without an error anywhere.
Date: 2026-08-23.

**The `GID` global counter's byte order is not stated, and is settled by
derivation.** `PidTagSourceKey` carries an `XID` ([MS-OXCFXICS] §2.2.2.2):
a 16-byte namespace GUID plus a `LocalId` of one to eight bytes. For a message
that `XID` is a `GID`, but [MS-OXCDATA] §2.2.1.3 describes `GlobalCounter` only
as "6 bytes; an unsigned integer identifying the folder or message" and never
says which end comes first.

It is nonetheless determined. [MS-OXCFXICS] §2.2.2.4.2 says a `REPLGUID`
combined with the `GLOBCNT` values in a `GLOBSET` "produces a set of `GID`
structures" — an `IDSET` *is* a compressed set of these same identifiers. So
the counter bytes inside a `GID` and the counter bytes a `GLOBSET` encodes must
be the same bytes, and a `GLOBSET` is unambiguously **most significant byte
first**, because its stack holds the values' shared *high-order* bytes
(§2.2.2.6.1).

**Our response:** one function, `ics::globcnt_to_bytes`, makes that choice, and
both the `GLOBSET` encoder and `contents_sync::Xid::for_counter` call it. A test
(`a_source_keys_counter_matches_what_a_globset_encodes`) pins the agreement,
because if the two ever diverge, a client's own id set and the `SourceKey`
values we send it describe different messages — with no error anywhere.

Derived from the specification, **not yet confirmed against a real Outlook**.
If cached mode establishes but the client re-downloads everything each time, or
messages duplicate, this is the first thing to check. Date: 2026-08-24.

## MAPI-over-HTTP: a closed chapter, kept for what it cost to learn

**The adapter is retired and deleted
([ADR 0056](decisions/0056-our-own-client-on-443-is-the-product.md)).** alo does
not serve `/mapi/*`, Autodiscover offers IMAP and SMTP only, and native Outlook
is not offered. Nothing below is live behaviour.

It stays here because specification reading is the expensive part and it does
not expire. If this is ever picked up again — and reversal is deliberately
expensive — these are the findings that would otherwise be rediscovered at the
same price, and the answer to "did a real client ever exercise this" is
recorded rather than remembered.

ADR 0051 stated every MAPI stage as *observable Outlook behaviour verified on
the wire*, and named stage 5 as the criterion for continuing at all.

### Verified against the live deployment, on the wire — 2026-08-25 (before retirement)

Driven by hand against `https://mail.alomails.com` as `disan@alomails.com`, not
in a harness.

**Autodiscover advertises MAPI/HTTP, and only when asked.** A POX request
*without* `X-MapiHttpCapability` returns IMAP and SMTP blocks and no `mapiHttp`
— which is correct ([MS-OXDSCLI] §3.2.5.1: the header is optional, and its
absence means a client that cannot speak MAPI/HTTP). The same request *with*
`X-MapiHttpCapability: 1` returns, alongside those two:

```xml
<Protocol Type="mapiHttp" Version="1">
  <MailStore>
    <InternalUrl>https://mail.alomails.com/mapi/emsmdb/</InternalUrl>
    <ExternalUrl>https://mail.alomails.com/mapi/emsmdb/</ExternalUrl>
  </MailStore>
  <AddressBook>
    <InternalUrl>https://mail.alomails.com/mapi/nspi/</InternalUrl>
    <ExternalUrl>https://mail.alomails.com/mapi/nspi/</ExternalUrl>
  </AddressBook>
</Protocol>
```

Worth recording because the first probe looked like a failure and was not: a
missing `mapiHttp` block is the *documented* answer to a request that never
asked for one. Anyone testing Autodiscover with `curl` must send the header or
they will conclude the adapter is off when it is running.

**`Connect` and `Disconnect` work with real credentials.** `POST
/mapi/emsmdb/` with `X-RequestType: Connect`, Basic authentication, and a
[MS-OXCMAPIHTTP] §2.2.4.1.1 body (empty `UserDn`, `Flags` 0, code page 1252,
LCIDs 1033, no auxiliary buffer):

```
HTTP 200 · Set-Cookie: MapiContext=…; Path=/mapi; HttpOnly; Secure; SameSite=None
trailer  X-ResponseCode: 0
payload  StatusCode=0  ErrorCode=0x00000000
         PollsMax=60000  RetryCount=3  RetryDelay=1000
```

`Disconnect` on that context returns `X-ResponseCode: 0`. So authentication,
session-context issue, the chunked `PROCESSING`/`DONE` response envelope
(§2.2.7) and clean teardown are all real, not inferred.

Note the envelope when reading a response by hand: the payload does **not**
start at byte zero. `PROCESSING\r\nDONE\r\n`, then the trailer headers, then a
blank line, and only then the response body. Parsing from byte zero yields
plausible-looking nonsense — `StatusCode=1129271888` is the ASCII of `PROC`.

### Not verified — no real Outlook has ever connected

Everything above is transport and discovery. The stages that carry the mail —
logon, the folder hierarchy, the contents table, opening a message — have been
built and tested against our own tests only. **No classic Outlook profile has
ever completed against alo.**

This matters more than an ordinary gap, because ADR 0051 makes stage 5 the kill
gate: reach "Outlook opens and reads a message" or stop and ship a connector
instead. `ROADMAP.md` records stage 5 as passed. It was not passed against a
client. Whatever the explanation, the gate has not been run, and the two
derivations in the sections above — the `MetaTagIdsetGiven` type and the `GID`
counter byte order — are still marked "not yet confirmed against a real
Outlook" for the same reason.

### How to run it, and what counts as passing

Half a day, one Windows machine with classic Outlook (not the new Outlook,
which does not speak MAPI/HTTP):

1. **New profile**, Control Panel → Mail → Show Profiles → Add. Enter
   `disan@alomails.com` and the password. Nothing else — the point of the test
   is that Autodiscover does the rest. If Outlook asks for a server name, stage
   1 has failed for this client and the answer is in its Autodiscover log.
2. **Capture the wire.** Fiddler or mitmproxy on 443, or the server side:
   `RUST_LOG=alo_mapi=debug` on `alo-jmap` gives every `X-RequestType`, the ROP
   ids inside each `Execute`, and the failure point.
3. **Record verbatim in this file** — what worked, what did not, and the exact
   request that failed. A summary is worth nothing here; the byte layouts are
   the whole difficulty.

Passing is stage by stage, in the order Outlook does them: the profile
completes; the folder tree appears; a folder lists its messages; **a message
opens and its body is readable** (this is the gate); an attachment opens; a
recipient resolves when typed; a sent message arrives and appears in Sent.

Expect it to stop somewhere. That is the point — where it stops is the finding,
and it belongs here rather than in someone's memory.

**That procedure was never run, and now will not be.** The adapter was retired
the same day it was written, so the kill gate ADR 0051 set — reach "Outlook
opens and reads a message" or stop — is settled the only honest way left: it was
never passed against a client, and we are not continuing. The procedure is kept
because it is the correct way to test a MAPI implementation against a real
Outlook, and because writing it down was how the gap became undeniable.
