# alo — ROADMAP.md

Progress is marked, never dated: a phase advances when its exit gate is fully
checked, however long that takes. Mark items [x] only when they meet the
definition of done in `.claude/skills/implement/` — full depth, gated,
documented. No item may be checked with a stub behind it.

Rules of this file:

- Work top to bottom inside a phase; phases may overlap only where marked ⇄.
- The exit gate is the phase. Unchecked gate = phase not done, regardless of
  how many items above it are checked.
- New items enter only through the scope gate (`features.md` tier + product
  doc Non-goals). Removing an item requires noting where it went (cut, moved,
  merged).

## Phase 0 — Foundations

### Legal & identity

- [ ] Company/IP structure decided and set up
- [ ] License model fixed: AGPL-3.0 core + commercial (per ADR 0002); open-core boundary written down
- [ ] CLA text chosen and signing tool wired (before the repo is public, not after)
- [ ] Name confirmed via EUIPO search (alo vs Atelier — Open Decisions closes)
- [ ] Trademark filed, classes 9 + 42
- [ ] Domains registered (.eu, .com, .io) + GitHub/Forgejo org + social handles

### Repo & infrastructure

- [x] Monorepo initialized: `core/` `web/` `control/` `migrate/` `deploy/` `docs/` + the governance layer (CLAUDE.md, skills, agents, ADRs)
- [x] CI: quality-gate commands run on every PR; releases build from tags only
- [ ] EU hosting partner selected (Open Decisions closes); first server live
- [x] `deploy/` composes the engine set (Synapse, LiveKit, Collabora, Garage, Postgres, Rspamd) at pinned versions
- [ ] Test domain configured: DNS, rDNS, first DKIM/SPF records (BLOCKED until constant-time credential compare lands — no public 587 with a timing oracle)
- [x] IP warming begins now — sending reputation is grown for launch, not at launch. *(Started 2026-08-18 on `159.195.89.28`, once the identity could sign and its first mail authenticated at a receiver: `spf=pass dkim=pass dmarc=pass`. Schedule and log in `docs/design/sending-reputation-warm-up.md`; Campaigns track, C2.0d.)*

### Exit gate — Phase 0 done when:

- [ ] `alo-smtp` (stub) accepts one message on the test domain, delivered through CI-built artifacts
- [ ] A stranger could clone the repo and understand the project from CLAUDE.md + docs alone

## Phase 1 — Mail core

### Receiving & sending

- [x] SMTP server: session state machine, EHLO negotiation, receive (25) and submission (587), per protocol skill non-negotiables
  - [x] Session state machine (RFC 5321 §4.1.4), EHLO/HELO negotiation, full receive path: MAIL/RCPT/DATA, §4.1.2 address parsing, dot-stuffing, Received: stamping, durable spool (M1; production port 25 binding lands with real deployment)
  - [x] Submission (587) with RFC 6409 rewrites (M3: Date/Message-ID fixups; From/Sender canonicalization deliberately out of scope)
- [x] Queueing: durable queue, retry schedule, 4xx/5xx semantics, bounce generation (DSN)
  - Local delivery now exists (see "Local delivery" below), so a mixed remote+local envelope's local recipients are filed directly; the remaining deferral is holding the *remote* recipients' bounce for a mixed envelope and i18n'ing the DSN prose
- [x] Outbound delivery: MX resolution + delivery, per-pass connection reuse
  - Deferred (not launch-blocking): cross-pass connection pooling and per-destination concurrency caps — delivery is currently sequential per pass; hardened when volume warrants
- [x] STARTTLS + AUTH; TLS enforced on submission (M3: STARTTLS/implicit-TLS via rustls, AUTH PLAIN/LOGIN over TLS on submission, truthful EHLO capabilities; credentials are a config-file dev bootstrap until alo-identity in M9)
- [x] Size limits enforced during read; timeouts per RFC 5321
- [x] Local delivery: inbound SMTP files into the account store with Sieve at the boundary — recipients resolved via `Store::account_by_email` at RCPT (unknown local user → `550 5.1.1` at RCPT, not after DATA), each resolved recipient delivered through `AccountStore::deliver_sieve` (parse → spam score → Sieve → file), Sieve `redirect`/`vacation` enqueued through the M2 outbound queue with CR/LF-stripped headers, per-recipient try-then-commit with a conservative whole-message `4xx` on transient store faults (no mail loss), durable on-disk blobs (`BlobStore::local`, `ALO_SMTP_BLOB_DIR`); the inbound spool is retired as the local sink and its all-local backlog migrated into the store once at startup. Design note `docs/design/local-delivery.md`; reviewed + security-audited

### Trust stack

Built as the `alo-auth-mail` crate, wired into `alo-smtp` at DATA
(inbound verdicts → `Received-SPF` + `Authentication-Results`, the
RFC 8601 contract) and at submission (DKIM signing). RSA crypto uses
`ring` (constant-time), not the `rsa` crate (RUSTSEC-2023-0071).

- [x] SPF verification (M4: RFC 7208 full `check_host` — all mechanisms, macro expansion, 10-DNS-lookup + 2-void-lookup hard limits)
- [x] DKIM: verification and signing, key management with rotation support (M4: verify multi-sig + relaxed/simple canon + `l=`/`x=`; sign RSA-2048 + Ed25519; `KeyStore` trait addressed by (domain, selector), file impl with perm checks + zeroize; verified by an independent tool (dkimpy))
- [x] DMARC evaluation + report generation (M4: PSL org-domain, relaxed/strict alignment, disposition→550 on `p=reject`, aggregate-report XML per Appendix C)
  - [x] DMARC report *delivery* (per-domain daily windows from recorded MX evaluations → gzip → §7.2.1.1 report mail via the outbound queue, §7.1 external-destination verification, DKIM-signed; `ALO_SMTP_DMARC_REPORTS=off` kill switch)
  - Deferred: TLS-RPT (`_smtp._tls` JSON) report delivery — needs per-policy TLS session outcome tracking in the outbound client first
- [x] ARC sealing (RFC 8617 first hop `i=1; cv=none` on Sieve-redirect forwards, sealed with the forwarding tenant's DKIM key; chain validator cross-checked against dkimpy both directions; `ALO_SMTP_ARC_SEALING=off` kill switch)
  - Deferred: inbound `arc=` stamping in Authentication-Results + sealing onto an existing chain (`i>1`) — the validator exists; wiring it at ingress is a follow-up
- [x] DANE for outbound (RFC 7672: per-MX `_25._tcp` TLSA over a DNSSEC-validating resolver — hickory validates the chain itself; secure usable set → mandatory DANE-EE-verified TLS, secure unusable set → mandatory unauthenticated TLS, TLSA lookup failure → host skipped, never downgraded; `ALO_SMTP_DANE=off` kill switch)
  - Deferred: DANE-TA(2) chain building (such records count as unusable → TLS still mandatory); secure-MX gating per §2.2.1 (we enforce on any secure TLSA — strictly stronger, recorded in `resolver.rs`)
  - Deferred from M4 (sanctioned cut seam): chain validation + AAR/AMS/AS sealing; needed for mailing-list forwarding, not first-hop receive/submit
- [x] MTA-STS published for our domains (M4b: RFC 8461 policy rendered from config — mode/mx/max_age, content-derived `id` — and served at `GET /.well-known/mta-sts.txt` behind the deploy TLS proxy; DNS records documented)
  - Deferred (sanctioned cut seam): TLS-RPT report JSON (`_smtp._tls` reporting)
- [x] Rspamd integrated at SMTP time; verdict wired to reply codes and headers (M4b: HTTP `/checkv2` consult at DATA → reject=550, soft-reject/greylist=451, else accept with `x-spam` merged into Authentication-Results; **fail-closed 451** when the scanner is unreachable; verified end-to-end with real Rspamd 4.1.2 — GTUBE → 550)
  - [x] Junk training: `Email/set` moves into/out of the Junk role call the Rspamd controller's learnspam/learnham (best-effort spawned, never gates the move; `ALO_JMAP_RSPAMD_URL` env, off when unset). Deploy gains a pinned redis (Bayes token store — previously Bayes was silently dead: no backend) + `secure_ip` controller access on the private network
  - [x] ClamAV malware scanning at DATA (clamd INSTREAM over the private network, pinned `clamav/clamav:1.4.5` with a persistent signature volume; signature match → 550 5.7.1 with the sanitized signature name, scanner outage → **fail-closed 451** exactly like Rspamd; >20 MiB messages pass unscanned, loudly logged; `ALO_SMTP_CLAMAV_ADDR` env, off when unset)
  - [x] Abuse controls: native per-source-IP concurrent-connection cap (accept-loop, IPv6 bucketed by /64, over cap → 421; `ALO_SMTP_MAX_CONNECTIONS_PER_IP`, default 20) + greylisting (Rspamd greylist module, now redis-backed → `soft reject`/451 for unknown triplets) + outbound per-destination-domain send-rate limiting (token bucket in the queue, over-rate → defer-not-bounce, protects sending-IP reputation; `ALO_SMTP_OUTBOUND_RATE_PER_MIN`/`_BURST`, off by default)
  - Deferred: FBL/ARF complaint handling (needs an inbound `abuse@`/complaint mailbox + RFC 5965 ARF parsing → auto-suppress; its own item)

### Store & APIs

- [x] `alo-store`: mailboxes, messages, flags, threads, blobs on Postgres + Garage; full-text index (opaque JMAP ids, hierarchical mailboxes with transactional counters, References threading, content-addressed blobs per-tenant, parsed Authentication-Results stored queryable, Postgres `tsvector` FTS; sqlx compile-checked with an offline cache; ADR 0006)
  - Deferred: the Garage S3 *live-integration* test (backend is behind the `garage` feature and compiles; in-memory backend is tested). The one-way spool-migration seam is now built as the local-delivery startup pass (all-local spool entries migrate into the store before the queue runner starts); a general mixed inbound/outbound backlog migration remains out of scope
  - A durable on-disk blob backend (`BlobStore::local`, object-store's filesystem store) now backs single-node local delivery, so a delivered body survives a restart without requiring Garage/S3
- [x] Every store operation tenant-scoped; wrong-tenant test suite in place and required by CI (tenancy by construction — a `TenantStore` is the only door and bakes the tenant predicate into every query; the isolation suite covers every public read/write path and CI runs it against real Postgres)
- [x] JMAP: session, Mailbox/*, Email/get|query|set|changes, push (RFC 8620/8621) (`alo-jmap`: Session with honest enforced limits, request batching + result references, interim bearer auth (argon2, trait-swappable for OIDC), Mailbox/Email/Thread get/set/query/changes with per-tenant modseq state tokens, blob upload/download, EventSource push; ADR-style design note; wrong-tenant suite extended to every method + blob + push, CI-gated)
  - Deferred (sanctioned cut seam): `EmailSubmission/set` (h) — sending a draft through the M2/M3 queue; recorded for the next pass. Also additive-later: full MIME `bodyStructure`/attachments in `Email/get`, JMAP-over-WebSocket (RFC 8887)
- [x] IMAP shim: LOGIN/SELECT/FETCH/STORE/SEARCH/IDLE against the store (9051/3501 compat) (`alo-imap`: implicit-TLS 993 + STARTTLS 143, LOGIN/AUTHENTICATE over TLS only with a failed-auth cap; full mailbox + message command set incl. UID variants, COPY/MOVE (6851), APPEND through the one ingestion path, special-use LIST (6154), and IDLE (2177) as account-scoped push; stable per-mailbox UIDs + UIDVALIDITY (migration 0006); byte-exact FETCH body sections + bounded-honest BODYSTRUCTURE; cross-tenant AND cross-account isolation suite over real TLS; reviewed + security-audited)
  - Deferred (additive later, named in `docs/design/imap-pop3-shims.md`): CONDSTORE/QRESYNC, SORT/THREAD, ACL/QUOTA/METADATA/COMPRESS/BINARY, sub-second IDLE via LISTEN/NOTIFY; a Thunderbird desktop-GUI interop pass (imaplib + openssl transcript stand in for this milestone)
- [x] POP3 shim (`alo-imap::pop3`: implicit-TLS 995, USER/PASS via the same credential seam, STAT/LIST/RETR/DELE/RSET/NOOP/QUIT/UIDL/TOP, inbox-only, UIDL reusing the stable IMAP UIDs, deletion committed on QUIT)
- [x] Sieve engine: base + vacation + subaddress; rules stored per user (`alo-sieve`: RFC 5228 parser/AST/evaluator with `require` enforcement and hard parse limits + an instruction budget — all security controls; actions keep/fileinto/discard/redirect/stop with implicit keep and mail-never-lost on any failure; vacation (5230) with RFC 3834 guards + per-correspondent `:days` suppression, subaddress (5233), imap4flags (5232) mapped to store keywords. Wired at the store delivery entry `AccountStore::deliver_sieve` (after spam scoring, before filing), with per-account script storage, vacation suppression, and a per-account redirect rate budget — all account-scoped, isolation inherited. Rule management is JMAP for Sieve (RFC 9661, ADR 0007): `SieveScript/{get,set,validate}` compile-checked on set. Reviewed + security-audited.)
  - The SMTP → mailbox **local-delivery bridge** that turns swaks into a filed message is now built (see "Local delivery" under "Receiving & sending"): the seam (`Store::account_by_email` + `deliver_sieve`, returning redirect/vacation `OutboundAction`s) is exercised on the real inbound path, with Sieve outbound actions enqueued through the M2 queue. Also additive-later, per `docs/design/sieve-filtering.md`: ManageSieve, `body`/`regex`/`variables`/`relational`/`date`/`notify`/`include` extensions, a per-account vacation-send budget, and blob-based (vs inline) SieveScript content.
- [x] `alo-identity` v1: users, groups, aliases, argon2 credentials, OIDC provider, 2FA (TOTP) (`alo-identity` is the credential authority behind SMTP AUTH, IMAP/POP3 `LOGIN`, and the JMAP bearer — the interim `StaticAuthenticator`, the store's `auth.rs`, and the SMTP credentials-file loader are **deleted**. **argon2id** password hashing with a documented parameter contract and a **constant-time** verify plus **dummy-hash** for unknown users, closing the pinned M3 timing oracle — proven by a timing test, `ratio≈1.0`. Every secret compared constant-time (`subtle`); tokens/recovery codes stored only as SHA-256 hashes. Identity model: tenants → users → aliases + groups; `account_by_email` is **alias-aware**; admin bootstrap is a CLI (`identityctl`), never a public endpoint. **OIDC/OAuth 2.0 provider**: discovery (RFC 8414), JWKS, `authorization_code` + **PKCE S256** (RFC 6749/7636), token + userinfo + revoke (RFC 7009); **opaque revocable access tokens**, rotated refresh tokens with replay-chain revocation, **EdDSA** ID tokens (ADR 0008). **TOTP 2FA** (RFC 6238) with drift window + single-use recovery codes; enforced everywhere — the OIDC flow prompts for the code, and the legacy protocols (which cannot) **fail closed** for a 2FA account, so a phished password cannot bypass 2FA over IMAP. Per-`(client,)username` backoff on the token endpoints **and** the legacy auth path. Reviewed + security-audited across **two independent passes**; cross-tenant AND cross-account isolation tested on every identity operation, plus OAuth negative-path coverage (wrong PKCE verifier, code/refresh replay → chain revoke, unregistered redirect). Design note `docs/design/identity.md`, ADR 0008)
  - Sanctioned cut seam (named in the design note): app-specific passwords + `XOAUTH2` on submission (how a 2FA user drives a legacy IMAP/SMTP client — the interim is account-password-on-legacy, TOTP enforced on the browser flow). Also deferred: binding submission `MAIL FROM` to the authenticated identity (send-as), which needs the group/alias permission model this milestone ships the data for; confidential clients, dynamic client registration, device/client-credentials grants, WebAuthn/passkeys, and an admin HTTP console

### Exit gate — Phase 1 done when:

- [ ] The founder's real daily mail runs on alo via Thunderbird + a raw JMAP client, for two continuous weeks, zero lost messages
- [~] Interop pass recorded: Thunderbird, Apple Mail, Gmail-app-via-IMAP send/receive/flag/search correctly (transcripts in `docs/interop.md` where quirks surfaced).
  **Protocol-level loop verified on prod** (2026-08-02, IMAPS+SMTPS): LOGIN/SELECT/
  SEARCH ALL/SEARCH SUBJECT (quoted)/STORE flag + AUTH/SEND; send→receive→search in
  ~6s; imaplib multi-word-quoting quirk recorded in `docs/interop.md`. Remaining: the
  GUI-client matrix (the actual Thunderbird/Apple Mail/Gmail apps)
- [~] Deliverability: our mail reaches Gmail/Outlook.com/Proton inboxes (not spam) from the warmed IP.
  **Trust stack verified on prod** (SPF/DKIM/DMARC/MX all pass, strict alignment).
  **Blocker: PTR (reverse DNS) is unset** — must be set at the IP/hosting provider;
  it dominates inbox placement. MTA-STS optional/unpublished. External-inbox receipt
  test pending the PTR fix (see `docs/interop.md`)

## Phase 2 — Product layer ⇄ (may overlap Phase 1 tail)

### Webmail & mail UX

- [x] Web app shell: design system, auth flow, navigation — the one-product frame (React/Vite/TS app: "warm workshop" design tokens + self-hosted Inter/EB Garamond + shared primitives; the left rail + layout frame + a module registry that makes Agenda/Chat/Drive/Docs one entry each; first-party OIDC authorization-code + PKCE login against `alo-identity` with 2FA-on-demand; a typed JMAP client with bearer + transparent refresh. Served at the same origin as the API behind Caddy (SPA + `/oauth`+`/jmap`+`/.well-known` proxied), strict CSP; login verified end-to-end on the live deployment. Design note `docs/design/web-shell.md`)
  - This item is the *frame*; the mail body below it is read-only so far (folders → message list → reading pane), with compose/reply and the rest as their own items
- [ ] Mail: read/compose/reply, conversation view + flat toggle, folders/subfolders, drag-drop
- [ ] Organization primitives: flags with due dates, categories/colors, archive keystroke, unread counts
- [x] Undo send, send later, snooze — *attested 2026-08-08: all three live in production (send-later scheduler, snooze store + sweeper, deployed routes on mail.alomails.com)*
- [ ] Visual Sieve rule builder
- [ ] Signatures (per identity + org footer), out-of-office with scheduling
- [x] Search UI over the store index — fast enough to feel local — *attested 2026-08-08: the Ctrl/Cmd-K workspace search over the store FTS is live and daily-used; it also grounds the Ask-alo agent*
- [x] Responsive / phone layout for Mail: below 768px the three-pane view becomes single-pane list↔detail (a `useIsMobile` matchMedia hook drives it) — folders slide in as an off-canvas drawer (toggled from the list header, closes on selection), the reading pane gets a back-to-list control, resize handles are hidden, and the reading toolbar wraps. Desktop unchanged
- [ ] PWA installable: offline shell, push notifications
- [~] Localization (i18n): runtime locale mechanism built — `strings` is a proxy over the active catalog so all ~50 call sites switch language with zero changes, English fallback per key (a partial catalog never blanks), browser detection + localStorage persistence + `<html lang>`, language switcher in the account menu, full-tree remount on switch. **French catalog complete** (the whole ~600-key surface, native quality). Follow-on: NL + DE catalogs (data only, mechanism ready); server-synced per-user preference (currently client-side)

### Agenda

- [~] Contacts (address book): backend built — a tenant/user-scoped `contacts` store (migration 0034, multi-valued emails/phones as JSONB), full vCard 4.0 round-trip (`alo-store::vcard`: FN/N/EMAIL/TEL/ORG/TITLE/NOTE/UID, folding + escaping, lenient parse), and a JMAP Contacts API (`Contact/get`/`Contact/set`, `urn:ietf:params:jmap:contacts`) with tenant isolation tested at both the store and API layers. Saved contacts now surface first in compose recipient autocomplete. **Web address-book UI built** — a two-pane modal (searchable list + editable detail) opened from the account menu, with create/edit/delete, dynamic multi-value emails/phones with type labels, server-derived display name, fully localized (EN + FR). vCard import/export is built (see "Contacts import/export"). **CardDAV device sync built** (see "alo-dav" below)
- [~] `alo-dav`: CalDAV/CardDAV over the store. **CardDAV built** (contact sync, RFC 6352): OPTIONS/PROPFIND/REPORT(multiget+sync-collection)/GET/PUT/DELETE at `/dav/…`, HTTP Basic via `authenticate_legacy`, RFC 6578 sync-token = account modseq, content-hash ETags, tenant-isolated (tested store + protocol). Currently a module in alo-jmap; the crate extraction + CalDAV (calendar) remain. Cut: `addressbook-query` filters (unfiltered fallback), PROPPATCH/MKCOL. Details in `docs/interop.md`
- [ ] Events, invitations (iTIP/iMIP), free/busy, recurring events with exceptions (the interop minefield — its own test corpus)
- [ ] Shared calendars, room/resource booking
- [ ] Agenda UI integrated with Mail (invite cards) and later Meet

### Chat & Meet

- [ ] Synapse per tenant provisioned by control plane; OIDC delegated to `alo-identity`
- [ ] alo Chat UI: channels, DMs, threads, reactions, mentions, guest access — Matrix invisible.
  Design bar = **Sila (silahq.com)**: sidebar (DMs/channels/agents/shared/search), polished
  message feed (avatars, bubbles, timestamps, media previews, presence). See `features.md` → Chat.
- [ ] ★ Agent-native chat: AI agents as first-class participants (own avatars, @mentionable,
  propose-then-approve replies/reactions) — the AI-native differentiator applied to chat.
- [ ] Application service streaming events to the (future) AI bus
- [ ] LiveKit deployed; token minting from alo identities; Meet UI on the components SDK
- [ ] Meeting links native in Agenda; recording to Drive with consent indicators

### Drive & Docs

- [ ] Drive: spaces, permissions, share links (password/expiry), trash/restore, version history
- [ ] Native editors on embedded open engines (ADR 0033, replaces Collabora/WOPI):
  - [x] alo Sheets — Univer engine + alo's own ribbon UI; real `.xlsx` import + `.xlsx` export
  - [ ] alo Docs — BlockNote; real `.docx` best-effort import
  - [ ] alo Slides — native in-house canvas; real `.pptx` best-effort import
  - [ ] Remove Collabora + WOPI once every format has a native home
- [ ] Technical authoring: browser-local math (KaTeX) + code (Prism) + alo auto-numbering/cross-references — standalone module first, docks into the Docs shell (ADR 0015)
- [ ] Fidelity CI: real-document corpus round-trips to desktop Office without mangling
- [ ] Desktop sync client v1

### Platform

- [x] Admin console: tenants, users, domains, quotas
- [ ] Deliverability autopilot v1: DNS wizard, DKIM rotation, DMARC monitoring, blacklist alerts
- [ ] Multi-tenant control plane: provisioning APIs for every engine, billing hooks
      — tenant lifecycle, domain ownership, quotas, and the operator surface
      (`alo-control`) landed (ADR 0012); per-engine provisioning + billing remain
- [ ] Native distribution lists + shared mailboxes with delegation
- [ ] Backups per DR targets; restore rehearsal scripted and passing
- [ ] Audit log; GDPR export; tenant export (exit as a feature)
      — tenant audit log landed (ADR 0012); GDPR + tenant export remain

### Exit gate — Phase 2 done when:

- [ ] Axon company #1 fully cut over: every employee's mail, calendar, chat, meetings, files on alo
- [ ] One non-technical Axon user, asked how many products they're using, answers "one"
- [ ] A restore rehearsal recovered a full tenant within the RTO target

## Phase 3 — AI layer ⇄ (may overlap Phase 2)

- [ ] Event bus: store/chat/calendar/file events flowing to indexers
- [ ] Per-tenant semantic index (embeddings local; pgvector first per ADR)
- [ ] Model-agnostic inference API; EU-hosted open-weight default; Self-Hosted GPU path; per-tenant AI off-switch
- [ ] Semantic search across mail/chat/files in one query bar
- [ ] Thread summarization; drafted replies in user tone (user-invoked)
- [ ] Attachment understanding: incoming .docx/.xlsx readable and summarizable
- [ ] Daily digest: "what did I miss" across all modules
- [ ] Inbox triage v1, per-user trainable
- [ ] Meeting minutes: transcript → summary/decisions/actions posted to the meeting's chat thread
- [ ] MCP server with per-agent permissions
- [ ] "Where did X go?" onboarding assistant
- [ ] Contractual guarantees implemented and verifiable: no training on customer data, no inference logs crossing tenant boundary

### Docs & Sheets AI (editor-native)

Depends on the native editors (Phase 2, "Drive & Docs") **and** the AI
layer above — the editors are alo's own UI on embedded open engines
(Univer/BlockNote, ADR 0033, superseding the Collabora shell of ADR 0010),
so these ship after the core suite, never in Phase 1. UX source of truth:
Figma pages "10 · Docs" and "11 · Sheets". Trust model throughout: **the AI
proposes and diffs; the user accepts** — never a silent overwrite of a
document or a formula.

- [ ] Docs — clean paste: strip foreign formatting by default on external paste and match destination styles; dismissible "formatting cleaned — keep original" toast
- [ ] Docs — Ask-AI-from-your-docs: in-editor panel answering from the user's real files/workspace (not just the open doc), every answer **source-cited**; cross-suite (Mail/Drive/Calendar); suggested actions (insert / summarize)
- [ ] Docs — agentic AI: inline command bar (rewrite / shorten / fix grammar / custom on a selection); AI changes shown as an **accept/reject inline diff**; agent mode for multi-step tasks as a **visible plan** (per-step done/doing/pending, live progress, workspace grounding, Stop control)
- [ ] Docs — semantic-conflict flag: AI detects co-edits whose *meaning* no longer reconciles (e.g. a unit price vs a total) and surfaces an inline flag with keep-A / keep-B / let-me-fix
- [ ] Docs — draft-from-workspace-context: on an empty doc, list the sources it will use (email thread, meeting recording + AI notes, related sheets) and generate a first draft from them — the cross-suite killer move
- [ ] Sheets — explain-and-fix errors: plain-language card for #REF!/#VALUE!/#NAME? (why it broke + one-click fixes), AI proposes / user accepts
- [ ] Sheets — natural-language formulas: English → the **actual formula, shown and explained** (transparent and auditable — never a black box)
- [ ] Sheets — formula paste-guard: warn before a raw value overwrites a formula cell (paste-as-value vs keep-formula)
- [ ] Sheets — ask-your-data: NL question → answer with the **source cells cited + highlighted + a chart**; cross-suite; optional agent mode for multi-step data tasks with a visible plan + approval

### Exit gate — Phase 3 done when:

- [ ] The demo runs live on real Axon data: "what did I miss this week?" answered correctly across mail, chat, and meetings
- [ ] AI cost per tenant measured and within the pricing model's margin

## Phase 4 — Migration suite

- [ ] Tenant audit tool: Graph API scan → usage report, blocker flags (macros, Power Automate), readiness score, savings figure
- [ ] Identity import from Entra ID; alo as IdP for the customer's other SaaS
- [~] Mailbox import: mail, folders, rules, signatures, OOF, aliases; PST import.
  **IMAP mail import built, all folders + flags** — an account-menu wizard
  (Gmail/Outlook presets + any IMAP server) and `POST /import/imap`
  (`imap_import`): SSRF-guarded connect, verified-TLS (webpki-roots) implicit-TLS
  client, `LIST` + per-folder `SELECT`/`FETCH (FLAGS BODY.PEEK[])` of the newest
  ≤500 messages across all selectable folders. Folder structure preserved
  (special-use → role mailbox, others created by leaf name; Gmail virtual
  `\All`/`\Flagged`/`\Important` skipped to avoid double-import); flags carried
  over (`\Seen`/`\Flagged`/`\Answered`/`\Draft` → JMAP keywords); idempotent
  `Message-ID` dedup (per-run + against the store); honest imported/skipped/failed;
  auth refusal → 401 app-password hint. EN + FR. Wire-verified on prod.
  Remaining: modified-UTF-7 folder-name decoding, rules/signatures/OOF/aliases, PST
- [ ] Shared mailboxes + delegation permissions mapped
- [ ] Calendar import: recurrences with exceptions, rooms/resources; Teams links in future events rewritten to alo Meet
- [x] Contacts import/export: `.vcf` (vCard 4.0) bulk import (`POST /contacts/import`, multi-card split + per-card cap, honest imported/skipped count) and whole-address-book export (`GET /contacts/export`, a `text/vcard` attachment), wired into the address-book UI (Import/Export buttons, EN + FR). Migrates a Gmail/Outlook/Apple contacts export straight in
- [ ] OneDrive/SharePoint import with permission mapping + unmappable-items report
- [~] Autodiscover/autoconfig endpoints: clients self-configure from an email address.
  **Endpoints built** (`autoconfig`): Mozilla `clientConfig` (Thunderbird/Apple Mail)
  at `/.well-known/autoconfig/mail/config-v1.1.xml` + `/mail/config-v1.1.xml`, and
  Microsoft POX Autodiscover (Outlook) at `/autodiscover/autodiscover.xml` (both
  casings) — unauthenticated, advertising IMAPS 993 + SMTPS 465 on the server FQDN,
  caller input XML-escaped + charset-validated. Wire-verified on prod. Remaining
  (operator/deploy): per-email-domain `autoconfig`/`autodiscover` DNS + Caddy vhosts
  (documented in deploy README) so real clients resolve it from the email domain
- [ ] Cutover safety: dual delivery during DNS propagation, read-only archive of old tenant, per-user rollback
- [ ] Subscription retirement screen: dependency check, savings figure, cancellation checklist

### Exit gate — Phase 4 done when:

- [ ] A non-Axon pilot company is migrated in one weekend by their own IT person — alo staff observing, not driving
- [ ] The pilot cancels (or formally schedules cancellation of) their M365 subscription

## Phase 5 — Launch

- [ ] Remaining Axon companies migrated; two written as public case studies
- [ ] External security audit + penetration test; findings fixed; report summarized publicly
- [ ] Pricing published (Cloud below the M365 tier it replaces, AI included; Self-Hosted license)
- [ ] Public source page live: our repos + upstream engines + versions (AGPL compliance + the trust story)
- [ ] Status page, public post-mortem policy, security.txt, disclosure policy
- [ ] Docs site: admin guide, migration playbook (incl. the VBA/desktop-Office answer), API/MCP reference
- [ ] 2–3 Belgian/EU MSP partners signed with reseller margin; white-label mode v1
- [ ] Support channel + SLA definitions per tier

### Exit gate — Phase 5 done when:

- [ ] A customer with no prior relationship to the founder signs, migrates, and pays
- [ ] The company survives the founder taking one full week off (runbooks + monitoring prove it)

## Phase 6 — Year-two battles (post-launch, ordered)

- [ ] EAS adapter: phones sync natively (mail/calendar/contacts) against the JMAP core
- [ ] Fast-follow features from `features.md` tier [2]: booking pages, meeting polls, shared-inbox collaboration, auto-translation, follow-up nudges, smart folders, internal recall, huddles, AI digest hardening
- [ ] Tauri desktop shell: tray, notifications, autostart
- [ ] Offline-first local cache (design review first, per ADR 0005)
- [ ] Mobile apps
- [~] **MAPI-over-HTTP adapter: native Outlook — the last wall.** Decided and
  specified in [ADR 0051](docs/decisions/0051-native-outlook-without-manual-setup.md):
  server-side, in Rust, on 443, translating to the JMAP core rather than forking
  the store. Its own crate (`products/mail/alo-mapi`) so a half-built adapter
  cannot destabilise mail that works. Every stage is stated as observable Outlook
  behaviour and verified on the wire, never as "the spec is implemented":
  - [ ] 1. Autodiscover returns a `mapiHttp` block; Outlook stops asking for manual settings
  - [ ] 2. `Connect`/`Execute`/`Disconnect` envelopes; Outlook completes the handshake and authenticates
  - [ ] 3. `Logon` + folder hierarchy; Outlook draws the folder tree
  - [ ] 4. Contents tables; Outlook lists messages in a folder
  - [ ] 5. `OpenMessage` + streams; Outlook opens and reads a message — **the kill gate**: not reached, we stop and ship a client-side connector instead
  - [ ] 6. NSPI; the address book resolves recipients
  - [ ] 7. Submission; Outlook sends
  - [ ] 8. ICS/FastTransfer; cached mode and offline
  - [ ] 9. Calendar, contacts and tasks as native MAPI classes
- [ ] Remote support / screen control (AnyDesk/TeamViewer-class — the EU IT-management play: one sovereign suite instead of a bolted-on remote tool). **Integrate** a self-hostable engine (RustDesk primary candidate); never build the capture/stream/input-injection engine ourselves — the highest-CVE-density surface in the product (ADR 0009). alo owns the UI/UX, session brokering, auth, consent, and audit logging. Launches from **Chat** (primary: the 1:1 DM header + person-profile quick-actions, beside Meet/Call/Email — where "help me" conversations live) and **Meet** (secondary: an in-call control-bar button for take-over-while-talking); a dedicated Remote/Support **rail tab is deferred** until the feature needs its own session management, history, and audit views. Requirements: native per-device agent (browsers cannot grant OS-level control — a hard boundary), E2E-encrypted session, **explicit per-session consent before any input**, an audit-log entry in the controlled user's security log, instant termination by either party, and a self-hosted relay (no third-party cloud). Screen *share* (read-only) is already in Meet; this is remote *control*, correctly sequenced post-launch. UX source of truth: the Figma design (request access / consent prompt / active-control banner with stop-sharing)
- [ ] Second developer hired and shipping independently (bus factor > 1 proven by a release the founder didn't cut)

### Exit gate — Phase 6 done when:

- [ ] An Outlook desktop user works a full week against alo without knowing Exchange is gone

---

## Business track — the Work OS (ADR 0035) ⇄ runs alongside Phases 3–6

alo widens from "replace M365" to **the one place a business does its work**:
the SAP/Odoo operational backbone, built from scratch on alo Base + the store +
the agent framework. Waves ship one at a time, full depth; **each wave's agent
is part of its definition of done** (ADR 0034 — propose-then-approve, EU
models). Feature detail lives in `features.md` → Business modules; this file
tracks the build order and gates.

### Wave B1 — Billing: Quotes & Invoices (the EU e-invoicing wedge)

Slices, in order — each shippable and wire-verified alone:

- [x] B1.1 Foundations: customer billing records (VAT ID, terms) on Contacts; product/price list; money as integer cents end-to-end
- [x] B1.2 Invoices: record + lines + server-computed VAT/totals; draft → issue with gapless per-tenant legal numbering; immutable once issued; credit notes
- [x] B1.3 Quotes: same line model; draft → sent → accepted/declined/expired; accept → invoice
- [ ] B1.4 PDF: branded invoice/quote PDF, sent via alo Mail — *PDF and the invoice covering-mail draft are built; a **quote** cannot yet be mailed (its `/send` is the lifecycle transition). One additive route short of done.*
- [x] B1.5 ★ E-invoice out: Factur-X + XRechnung/UBL, schematron-validated (EN 16931)
- [x] B1.6 E-invoice in: inbound Factur-X/XRechnung → parsed bill record
- [x] B1.7 Payments: mark paid / partial, overdue view, manual reminders; VAT summary per period
- [x] B1.8 ★ Billing agent: create/convert/chase by plain language, propose-then-approve, wired into Ask alo
- [ ] B1.9 Peppol via a certified access point (integrate first — open decision on own membership) — *human item: needs a contract and credentials with a certified AP. The formats it carries (Factur-X, XRechnung) are done.*

The module is translated end to end in en/fr/nl — interface, printed document,
covering note and reminder letter (B1.27).

### Exit gate — B1 done when:

- [ ] A real business (Axon Group) runs a full month's invoicing on alo: quotes sent, invoices issued with legal numbering, one credit note, an XRechnung accepted by a government/large-buyer portal, and the VAT summary handed to the accountant without a spreadsheet

### Wave B2 — CRM & Sales — deals on real email *(gate: B1 live with ≥1 real tenant)*

Built ahead of its gate: the gate is a **deployment** milestone and B2 is
code, migrations and tests, none of it deployed. A human confirms or moves
the gate before any of this reaches a tenant.

- [x] B2.1 Pipelines & deals: per-tenant boards, ordered stages with win/loss flags, deal record (value in cents, currency, expected close, owner, source), every move kept as history
- [x] B2.2 ★ Deal ↔ mail-thread linking: suggest from the requesting user's own recent mail, exact-address-only for free-mail domains, user-confirmed; the link is a pointer, never a copy
- [x] B2.3 Activities: notes/calls/meetings logged, and a next step that is a **real task** in its owner's own list
- [x] B2.4 Board, list and deal drawer on screen — the Tasks board interaction, a deal link a colleague can be sent
- [x] B2.5 Win/loss: lost reasons, won → draft quote or invoice in Billing, pipeline report by stage and by period, CSV — per currency, never converted
- [x] B2.6 Lead import: CSV preview-then-commit, all-or-nothing, dedupe by address then company domain — *API only; the import screen is a named cut*
- [x] B2.7 ★ CRM agent: `create_deal` (from a thread), `move_deal_stage`, `draft_followup` — propose-then-approve, nothing sent
- [x] B2.8 Billing extensions: recurring invoices (drafts only, never auto-issued) and the SEPA `pain.001` file for paying approved bills
- [x] B2.9 Audit trail: every mutating billing/CRM route writes one entry; a **History** panel on invoices, quotes and deals
- [ ] B2.10 Payment links on invoices via an EU PSP — *human item: needs a contract and credentials with a payment provider. No code written toward it.*
- [ ] B2.11 Role-based access per module (sales vs finance) — *B4.12 shipped the first scoped role (`tenant_user_roles`, the external accountant: finance write, billing and CRM read-only, no mail or files) rather than a Spaces scoping. That is one role, not a per-module matrix: every member of a tenant still sees every deal, which `docs/design/crm.md` says out loud. A sales-vs-finance split is a second role on the same table, and still unshipped.*

CRM is translated end to end in en/fr/nl — interface, agent cards, record
history (B2.14). It renders no server-side document, so unlike B1 there was
nothing outside the browser to translate.

### Wave B3 — Projects & Timesheets — billable hours feed B1

Built on the same terms as B2 and BI-1: code, migrations and tests, none of
it deployed. Nothing here starts with a new noun — a project is the board
Tasks already ships, seen as client work.

- [x] B3.1 Client projects: a customer, a currency, an hourly rate and a budget in hours or money attached to the board a team already uses — and taken off again without touching an hour that was logged
- [x] B3.2 Hours: a timer visible from every screen in the workspace, a manual entry, a week grid you type into; minutes stored, the rate snapshotted onto the entry, and every total the server's
- [x] B3.3 The week: submit → approve or send back, approved hours locked, an approval reopened only while no invoice carries them — with two doors, so a colleague's hours are unreachable rather than merely refused
- [x] B3.4 ★ Billable hours → a **draft** invoice in Billing: approved, unbilled hours grouped per project per rate, the entries stamped with the invoice they went onto — *API only and wire-verified; the select-and-raise screen is a named cut*
- [x] B3.5 Profitability: hours × rates against a budget, per engagement per currency, with CSV — value, never margin
- [x] B3.6 The plan: milestones on a date axis over the existing board, reached when a person says so; and templates — a project marked reusable, copied onto new dates with nobody's assignees, comments, hours or finished cards
- [x] B3.7 ★ Projects agent: `log_time`, `project_status_summary`, `draft_timesheet_from_calendar` — every hour it writes is a **suggestion** in nobody's total until the person whose timesheet it is accepts it
- [ ] B3.8 Per-project access roles (who may see an engagement at all) — *B4.12 shipped the role mechanism (`tenant_user_roles`) and used it for the external accountant only; a per-engagement scope is a further role and remains unshipped*

Projects is translated end to end in en/fr/nl — interface, the assistant's
cards, and the unit label on an invoice line raised from a timesheet
(B3.11).

### Wave B4 — Expenses & Accounting core — receipts, ledger, reconciliation, VAT

Built on the same terms as B2, BI-1 and B3: code, migrations and tests, none
of it deployed. This is the first wave whose output is not a screen a
colleague reads but a statement a stranger audits, so every figure on it is
the server's fold of an append-only journal, in integer cents.

- [x] B4.1 The chart of accounts: a neutral EU-SME chart seeded per tenant in the reader's own language, renamed, renumbered and retired by the tenant — posting rules resolve by an account's **job** and never by its number, so an accountant's numbering breaks nothing
- [x] B4.2 The journal: append-only entries whose debits equal their credits, enforced inside the transaction and proven by property tests over generated documents, with per-event idempotency so a document posts exactly once
- [x] B4.3 The posting rules: invoice issue, payment settlement (including partials) and credit note, each golden-tested against hand-written entries, an original and its credit summing to zero — *written and tested; **not yet called from the billing routes**, so today the books open at reconciliation (B4.6)*
- [x] B4.4 Expenses: a claim with its project, method and VAT, submit → approve → reject or reimburse, an approver's queue and a to-pay-back list; mileage at a per-km rate table — *mileage is API-only, the category picker and the receipt upload are named cuts, and an approved claim does not post to the journal*
- [x] B4.5 ★ Receipts: a deterministic extractor (vendor, date, amounts, VAT) behind a pluggable trait, fixture-proven, with a parsed-fields-for-confirmation door — *the AI backend is a seam awaiting a human's model decision, not a backend*
- [x] B4.6 ★ The bank: CAMT.053, MT940 and a CSV mapping wizard against public goldens; then reconciliation — exact matching, windowed heuristics, per-tenant learned rules, manual pick, set-aside, an undo on each, and a confirm that **books the invoice and the payment**
- [x] B4.7 Fiscal periods with a soft close: postings before the lock date refused typed, an admin unlock audited
- [x] B4.8 The four reports: profit and loss with the year-earlier period beside it, the balance sheet on any day, who owes what by how overdue, and the VAT-return figures — from the books, with a CSV per report; a balance sheet that does not balance says so instead of printing a figure that looks fine
- [x] B4.9 The accountant role: alo's **first scoped role** — finance read plus journal write, no mail and no files, proven by tests that the scope holds
- [x] B4.10 ★ The finance agent: `categorise_transactions` (a suggestion kept apart from the category you chose), `vat_summary` and `flag_anomalies` — answers with the entries behind them, no score, nothing about any person, nothing filed
- [ ] B4.11 The ledger posting from the documents themselves (issue, settle, credit — and an expense rule, which does not exist) — *the wave's largest gap, named in `docs/design/finance.md` § "What B4 promised"; a follow-up item, not a cut*
- [ ] B4.12 The manual journal entry with description and attachment (the accountant's escape hatch) — *the store function exists and is tested; it has no route and no screen*

Finance is translated end to end in en/fr/nl — the claim form, the bank and
reconciliation screens, the chart, the four reports and the agent's cards
(B4.15). The CSV column headings stay English on purpose: they are a contract
read by an accountant's own tooling, not a sentence a person reads.

### Wave B5 — Purchasing & Inventory — products, stock, PO/SO chains

Built on the same terms as B2, BI-1, B3 and B4: code, migrations and tests,
none of it deployed. This is the first wave whose records are **not the
truth** — a stock level is a claim about a shelf in a room, and the shelf does
not read our database — so the module's job is not to hold the quantity but to
explain it. Hence its one rule, from which everything else follows: *a quantity
is never written, only movements are.*

- [x] B5.1 The catalog as things rather than prices: the product Billing invoices from gains an SKU unique per tenant, a GTIN whose check digit is verified on write, a purchase price beside the sale price, a usual supplier, and a `stocked` flag that becomes a one-way door the moment anything has moved — one product record, never two that drift
- [x] B5.2 Suppliers: the people you buy from kept the way customers are, each with their own price list, lead time and minimum order quantity, which is what a reorder proposal is priced from — *API-only: there is no supplier screen, and this is the wave's largest UI gap*
- [x] B5.3 The move ledger: locations real and virtual, every movement naming where the goods came from and where they went, on-hand as the fold of them, and a cached per-location balance **proven** a fold by a test rather than trusted; stock never goes below zero, and manual adjustments and transfers carry a closed vocabulary of reasons — *the adjustment flow is API-only*
- [x] B5.4 Purchase orders, whole: the draft, then one act that draws the number from a gapless series, freezes the order, and writes the covering letter with the printed order attached to **your Drafts** — never sending it for you; then booking an arrival, which opens on what is outstanding, writes the stock moves and raises a draft bill for what came
- [x] B5.5 Sales orders, whole: draft, confirm (the number drawn, no message written, because the customer already has your answer), a consignment at a time out of a named place, then a **draft** invoice for what has actually gone and never for what is still on order — *the delivery note is a record, not a printed document*
- [x] B5.6 Reorder rules and the shortage report: a minimum and a target per product and place, counting on hand, on order and promised out **separately** so ordering this morning takes the item off the list this afternoon; buy quantities rounded up to what the supplier will sell, priced and lead-timed from their own quote, with a CSV — *API-only, reachable through the agent card or the API but not from a screen*
- [x] B5.7 Stocktake: a count sheet snapshotted per place, variance worked out per row, an uncounted row that stays uncounted rather than counting as zero, and an apply that corrects against **what is on the shelf at the moment of applying** — so a shipment that went out mid-count is never written over — writing ordinary movements, once, with no re-apply — *API-only*
- [x] B5.8 The screens: the catalog and what is on the shelves with the movement history behind every row; both order documents with the sentence that says what placing or confirming will do before it happens; and barcode scanning — the phone camera where the browser has one, and the keyboard-wedge path a handheld scanner uses, which needs no permission and works on the machine bolted to the packing bench
- [x] B5.9 ★ The inventory agent: `reorder_proposals`, which writes one draft purchase order per supplier and contacts nobody, and `stock_answer`, which reads one product back — on hand, on order, promised, and which shelves are under their minimum — and never guesses when you will run out
- [ ] B5.10 ★ Stock valuation and the ledger: the inventory asset account, cost of goods sold, and a costing method — *the wave's largest cut, and a deliberate one: a method (FIFO, weighted average, standard cost) is a per-tenant accounting policy with tax consequences, and B4.11 (documents posting to the journal at all) is its unmet prerequisite. Until then the stock screen shows a reference value at today's purchase price and refuses to call it a balance*
- [ ] B5.11 The screens the wave did not reach: suppliers and their price lists, adjustments and transfers, the stocktake, and the reorder rules with their shortage report — *all four are shipped, routed and tested behind the API; they are the natural first items of any B5 follow-up*

Inventory is translated end to end in en/fr/nl — the catalog, the stock list
and its history, both order documents with the sentences that precede an
irreversible act, the scanner and the agent's cards (B5.11). The printed
purchase order and its covering letter were already in three languages from
B5.05a2, and a tenant's starting locations are seeded in the reader's own
language. The shortage CSV's column headings stay English on purpose, for the
same reason B4's do.

### Wave B6 — HR — records, leave, recruitment-lite (payroll calc = permanent non-goal)

Built on the same terms as B2–B5: code, migrations and tests, none of it
deployed. The last Work OS module, and the first whose records are **about
people who have rights over them** — a person can demand a copy of their record,
demand it corrected, in cases demand it erased, and the law separately says keep
parts of it for years after they leave. Hence its one rule: *a person's record is
readable by them, by the people who must act on it, and by nobody else, and every
act on it is recorded.* Three doors, never a tenant-wide read.

- [x] B6.1 The record and the doors: an employee, the employments under them, and three doors — your own record, your reports', and HR's — deciding field by field who sees a home address, a date of birth or a pay figure; an org chart folded from manager links that refuses a cycle; archived and never deleted, because retention and erasure pull opposite ways on the same row; and contract letters filed in an **HR-only area of Drive** where nobody without the role can open them or learn they exist
- [x] B6.2 Leave, the arithmetic: entitlement in **minutes** over a person's own working pattern rather than in days that mean different things to different contracts, accrual a twelfth at a time, carry-over with its expiry, and a first annual policy seeded from the tenant's country so leave works before anybody configures anything — *a default, not advice* — *editing a policy, or having a sick or unpaid one at all, is API-only*
- [x] B6.3 Leave, the flow: request → the manager's or HR's decision with a note, nobody approving their own, a request withdrawable while undecided and cancellable until the day it starts, and an approved day that appears in the team absence layer as **a name and a date and nothing else** — never a reason
- [x] B6.4 Public holidays: fifteen European calendars seeded, one chosen per tenant, and a holiday inside a leave request that costs nobody a day — *choosing the calendar is API-only*
- [x] B6.5 Onboarding and offboarding: a template a company writes once, an instance per person, each step naming who does it — and the mailbox is a **task for an administrator**, never a write path from HR into identity — *store and routes only, no screen*
- [x] B6.6 Recruitment-lite: openings, applicants with their CV in the HR-only Drive area, interview notes and a stage board on the shared board pattern, ending in a hire that opens the directory form — and **no machine ever reads a CV**
- [x] B6.7 One approvals inbox: leave, expense claims and timesheet weeks in a single manager view with counts, each row opening the screen that decides it
- [x] B6.8 The screens: the directory and the org chart, asking for leave and answering it, the month that says who is away, the hiring board and the day a candidate becomes a colleague
- [x] B6.9 ★ The HR agent, two tools and a refusal: `who_is_off`, which reads names and days and is told in its own description never to guess why; and `draft_letter_from_template`, which fills in a letter **the company itself wrote** and refuses rather than improvise one about a person
- [ ] B6.10 ★ CV screening — **refused, not scheduled.** Not suggest-only, not ranked, not scored, in any form: EU AI Act Annex III 4(a) high-risk territory, and a scored candidate list is a decision dressed as a suggestion. `docs/features.md` still promises it and needs a product owner's amendment
- [ ] B6.11 The screens the wave did not reach: leave policies, holiday-calendar selection, onboarding checklists, letter templates and the payroll export — *all five are shipped, routed and tested behind the API. The letter templates one is the first item of any B6 follow-up: the agent tool refuses a template the tenant has not written, and there is no way in the product to write one*
- [ ] B6.12 The absence layer drawn in **Agenda** — features.md names it and what shipped is a month in People; also a decision about whether everybody's calendar should show colleagues' absences by default

Payroll **calculation** stays a permanent non-goal. The **export** — a `[B+]`
line brought forward into this wave — ships: a per-period CSV in four column
mappings with a receipt for every draw, and no figure alo computed.

HR is translated end to end in en/fr/nl — and, unlike B1–B5, without a
translation pass at the wave review, because the i18n ratchet has been green
since B2 and each screen landed its French and Dutch in its own commit. What
B6.11 added is a test that no HR string may ever be exempted from that, and that
**no string in any language ever says why somebody is away**.

### Wave BI-1 — alo Insights first slice ⇄ inserted after B2 (ADR 0037: zero-setup overview dashboard, tile gallery, ask-to-chart)

Inserted ahead of B3 by owner decision. Built on the same terms as B2: code,
migrations and tests, none of it deployed.

- [x] BI-1.1 The ChartSpec and the semantic layer: a typed envelope over a closed catalog of four datasets (invoices, receivables, payments, deals) — the AI and the user both choose from it, and neither writes SQL. Money is folded in the same Rust the invoice and the VAT return use, so a chart and a tax return cannot disagree about a cent
- [x] BI-1.2 Boards and tiles: tenant-scoped dashboards, specs validated on write, fractional order and a 1–4 column span, `/insights/*` on the wire — a spec from another tenant is not a capability
- [x] BI-1.3 The Insights tab: number, bar, line, pie and table tiles under alo chrome, drawn by one embedded Apache-2.0 library, every chart also a table for a screen reader
- [x] BI-1.4 ★ The zero-setup **Business overview**: seven live figures on a real board the first time a tenant opens Insights, with a gallery of ten ready-made questions beside it
- [x] BI-1.5 ★ Ask-to-chart: plain language → a proposed chart you look at before it is pinned; strict parse, one repair, a refusal believed rather than repaired
- [ ] BI-1.6 Spaces-scoped board sharing (finance sees finance, sales sees pipeline) — *B4.12 shipped a role table rather than a Spaces scoping, and used it for the external accountant only; insight boards are still tenant-wide*

The module is translated end to end in en/fr/nl — interface, chart labels,
the seeded overview's own captions, down to the quarter and week
abbreviations on an axis (BI1.08).

### Wave BI-2 — alo Insights full — after B4 (finance depth, module-embedded strips, digest mail)

Later waves (post-traction, unordered): manufacturing-lite, POS, subscriptions,
e-signature (eIDAS), marketing sends, storefront, DATEV/PSD2 integrations.

## Order track — **re-cut 2026-08-18 to four items** (ADR 0054) ⇄ after Campaigns C1

> **Read [ADR 0054](docs/decisions/0054-what-the-order-book-still-needs.md), not
> 0053, which is superseded.** The old wave's premise was false: the sales order,
> delivery notes and invoice-from-delivery were all built in wave B5.06
> (2026-08-10) under `inv_so_*` names, and the absence was concluded from a
> `sales_order` grep that missed the convention. **O1.1, O1.4 and O1.5 are
> built**; building them again would create the duplicate object 0053 rejects in
> its own *Rejected* section.
>
> What is genuinely missing is four things — a **refusal** at confirmation, the
> **quote → order link**, **routing on acceptance**, and the **order book read** —
> and they are the wave below. The queue is
> `docs/autonomy/orders/QUEUE.md`; the track has its own checkout now
> (`C:\dev\Ficina-orders`), which is what unblocked it.

**What is missing, stated plainly — corrected 2026-08-18.** alo has more of the
goods half than this section claimed. A quote still goes straight to an invoice
draft, and that is real: nothing routes an acceptance to an order. But an
accepted order, once raised, already reserves nothing *by design*, delivers in
parts through `record_move`, and invoices what shipped.

```
Odoo / SAP:  quote -> SALES ORDER -> production -> DELIVERY -> invoice
alo today:   quote  ----------------------------------------> invoice
             (and, raised by hand:  SALES ORDER -> DELIVERY -> invoice)
```

So the consequence is narrower and sharper than "there is no record of ordered
but not delivered" — that record exists. **Nothing refuses an over-commitment**:
`confirm_inv_sales_order` draws the number without asking whether the goods can
exist, so two orders for the last fan both confirm. And **nothing connects the
offer to the order**, so the flow a customer actually walks still ends at an
invoice for goods that may never have shipped.

**The ADR is written and it is 0054.** The three questions the prerequisite named
turned out to be already answered *in the build* rather than open: the sales
order is its own object, reservation is a promise and not moved stock, and
invoicing follows the delivery. 0054's job was different — to say what is left
once you read that, and to settle the one thing the code genuinely leaves open
(how a confirmation refuses an over-commitment without inventing a second ledger).

**A quote for services must still become an invoice directly.** Products already
carry `stocked`; a quote of consultancy days has nothing to reserve or deliver,
and routing it through an order would add a step that serves nobody. The order
exists for the lines that move.

Migrations take the **`07xx`** block (campaigns `05xx`, mail/platform `06xx`).

### Wave O1 - the refusal, the link, the routing and the read

- [x] O1.0 **ADR 0054** — written from the code, superseding 0053. The sales order already exists, so extend `inv_so_*` rather than building a second one; `reserved` stays **computed** (`inv_reorder`'s `committed` fold) and what is missing is the **refusal** at confirmation; the quote link is one additive column mirroring `billing_invoices.quote_id`; and the three availability answers already in the build deliberately do not consult one another, which is now a named limitation rather than an inherited surprise.
- [~] ~~O1.1 The sales order~~ — **built** in wave B5.06 (migration 0162, `inv_so.rs`, `inv_so_lines.rs`). Ordered, delivered and invoiced per line all exist.
- [~] ~~O1.4 Delivery notes~~ — **built** (`inv_so_deliver.rs`): movements through `record_move`, partial delivery normal, over-delivery refused, notes numbered `SO-2026-00001/D1`.
- [~] ~~O1.5 Invoice from what was delivered~~ — **built** (`inv_so_invoice.rs`, migration 0164).
- [ ] O1.a The **refusal at confirmation**: an order whose stocked lines would push `committed` past `on_hand + on_order` is refused inside the transaction that draws the number, naming the product and the shortfall. Settled with a per-`(tenant, product)` advisory lock the way `inv_stock_sale.rs` settles the same race, locking every stocked product on the order in ascending id order so two orders sharing two products cannot deadlock. **A fan promised twice is the failure this exists to prevent**, and its test must fail against today's code before it passes.
- [x] O1.b The **quote → order link**: an additive `quote_id` on `inv_sales_orders` with a composite foreign key, exactly as migration 0106 gave the invoice its own. Not a link table. **Migration 0700**, with the partial unique index that makes one offer yield at most one order, and the field read-only on the API — where an order came from is provenance, and a request that could restate it would make the link worthless in the case it exists for.
- [x] O1.c **Accepting a quote routes by content** — an order when any line names a stocked product, a draft invoice when none does; the services path unchanged, pinned by tests written before the branch existed. It was blocked on a **billing** schema change (`billing_quote_lines` carried no `product_id`, so a quote could not say what it was selling and an order copied from one could never deliver anything). **The owner authorised it 2026-08-20**; migration `0701` adds the column in `inv_sales_order_lines`' own shape, without weakening 0105's rule that a line snapshots its own price. The order raised is a draft, so acceptance never commits stock. **Still owed, and web work:** a product picker on the quote editor, and an accept handler that follows `salesOrder` — until then the goods branch is reachable over the API and not from a screen.
- [x] O1.d The **order book**: ordered, reserved, delivered, invoiced and outstanding, per order and in total, wrong-tenant tested per route. Smaller than it was, because the four numbers per line already exist. **`GET /inventory/order-book`**, nothing stored: delivered and invoiced value are `line_net_cents` at the quantity that actually moved, not a share of a rounded total, so the parts reconstitute the whole to the cent.

**Cut to O2:** the Orders agent (was O1.7). It reads what O1.d produces and cannot
be specified before that screen exists.

### Exit gate - O1 done when:

- [x] The fan quote from the walkthrough becomes an order, ships four on one note and two on another, and bills each delivery - with the order book showing the remainder at every step. **Walked end to end 2026-08-20** over the real HTTP surface; figures in `docs/autonomy/orders/STATE.md`.
- [x] Two concurrent confirmations cannot both promise the last unit, proven by a test that failed before the refusal existed
- [x] A services quote still becomes an invoice directly, unchanged

### Wave O2 - making the thing *(not started; needs O1)*

Bill of materials, works orders and capacity. A manufacturer that can take an
order it cannot build has moved the problem rather than solved it - but an order
book with no reservation is the more urgent absence, and O1 is worth shipping
before this begins.

---

## Campaigns track — bulk email that cannot poison the mailbox (ADR 0044) ⇄ after the Agent track

Ordered so the half blocked on a purchase does not block the half that is not.
**C1 needs no second IP and sends nothing** — the audience, the consent record
and the suppression rule are the differentiator and the part nobody else can
copy. C2 is the sending identity and cannot start until there is a second IP.

Migrations take the **`05xx`** block (agents hold `04xx`, sites `03xx`).

**On measurement, stated once here so no item has to re-argue it.** Four
different qualities of number get called "analytics" in this business, and the
reporting screen must never mix them:

| | trust | how it is known |
|---|---|---|
| delivered, bounced, complained | **fact** | our own SMTP result and the feedback loop |
| clicked | **reliable** | a redirect the recipient actually followed |
| opened | **weak** | a remote image; Apple pre-fetches it, so an unknown share are machines |
| how long it was read | **very weak** | the same image held open; most clients block it, many cache it |

Delivery and clicks are earned. Opens are an opt-in per campaign, off unless
chosen and disclosed (ADR 0044 §5). **Read duration is not decided by any ADR**,
and C5.4 writes one before anything is built.

### Wave C1 — the audience, and the two rules that make it safe

- [x] C1.1 The reachable audience: one tenant-scoped view over **billing customers, CRM deal contacts and site form submissions**, deduplicated by address. Explicitly **not** the `contacts` table — it is a per-user address book, and a company campaign drawn from it would mail somebody's private contacts. A test proves that table is never a source.
- [x] C1.2 Consent as a record: when, from which source, from which address, provenance stored rather than a boolean. A person with no consent record cannot be a recipient, proven by a test rather than by a filter in the caller.
- [x] C1.3 Suppression, absolute and global to the tenant: unsubscribe, hard bounce and complaint each suppress, and **the audience query itself excludes them in SQL**. A test proves an import cannot resurrect a suppressed address — if the sender applies the rule, it is not absolute. **Built:** the exclusion is in the audience SQL itself (`campaign_audience.rs`), and `campaign_suppression_tenancy.rs::an_import_cannot_resurrect_a_suppressed_address` proves the import path cannot undo it.
- [~] C1.4 Segments: a saved query with the conditions ADR 0044 names — bought/not bought within a period, country, has/has not received a given campaign. The count and the exclusions are both readable, because a number without its exclusions is not auditable. **Three of four conditions built** (`campaign_segments.rs`): country, and bought/not-bought in a period against issued invoices. *Has/has not received a given campaign* was deliberately deferred — there was no send record to point at. **C4.1's `campaign_send_recipients` now unblocks it**: an additive column and one extra CTE.
- [~] C1.5 The audience screen: the segment reading as a question with the count moving as it is refined, and excluded people named with the reason. Wrong-tenant and wrong-user tests per surface. **Screen built** (`AudienceTable.tsx`, `useAudience.ts`, `QuestionBar.tsx`, `TallyLine.tsx`); the store surfaces beneath it are tenancy-tested. **Not done: there is not one test file under `web/src/campaigns/`**, and this item asks for wrong-tenant and wrong-user tests *per surface*.

### Exit gate — C1 done when:

- [ ] A segment answers "bought in the last 18 months but not the last 90 days, in Belgium" from real CRM and Billing rows, with its exclusions listed
- [x] A suppressed address cannot be returned by any segment, and re-importing it does not bring it back **Proven:** `campaign_segments_tenancy.rs::a_segment_cannot_reach_somebody_the_audience_would_not` ("an import resurrected somebody a segment had excluded") and `campaign_suppression_tenancy.rs::an_import_cannot_resurrect_a_suppressed_address`.
- [x] No query in the module reads the per-user `contacts` table **Proven twice:** `the_per_user_address_book_is_never_a_source_of_a_segment`, plus a unit `all_sql()` audit in all seven campaign modules that walks every statement each can issue and asserts none names the table.

### Wave C2 — the sending identity *(the IP is bought; three steps remain)*

**`159.195.89.28`** — netcup, Nürnberg, ordered 2026-08-17, €2.03/mo on a
12-month term. Verified clean at allocation: 0 of 60 blocklists at MXToolbox
(Spamhaus ZEN, SpamCop, Barracuda, UCEPROTECT L1–L3, Abusix, ivmSIP, Mailspike,
PSBL, LASHBACK among them) and "no issues" at Spamhaus. It sits in
`159.195.88.0/23` (`DE-NETCUP-SERVER`) rather than the `152.53.176.0/22` the
transactional IP is in — a legacy ERX block, same registrant and same admin
contact, confirmed at RIPE. That is exactly the kind of range where recycled
reputation hides, which is why the check mattered rather than being a formality.

Before C2.1 can start:

- [x] C2.0a Attached 2026-08-17 — netcup routes it as `159.195.89.28/32` via the primary address, which is their normal arrangement for an extra IPv4. No reboot or network restart was needed, and the primary address, its gateway and its PTR were untouched.
- [x] C2.0b Reverse DNS set 2026-08-17 and confirmed from outside: forward `news.alomails.com → 159.195.89.28` and reverse `159.195.89.28 → news.alomails.com` both resolve at Google. That is the forward-confirmed reverse DNS Gmail and Outlook check.
- [x] C2.0e Bound inside the VM — netcup routing the address is not the same as the operating system holding it, and nothing can send from an address the kernel does not have. Added to `eth0` and persisted in `/etc/network/interfaces.d/50-cloud-init.cfg` as `up`/`down` commands appended to the existing stanza rather than a second `iface` block, so the primary address and gateway are never re-parsed to reach it; `|| true` on both so a failure there can never abort `ifup` at boot and cost the server its network. cloud-init's network management is disabled on this host (`99_nc_network_disable.cfg`), so the file is not regenerated. Verified: outbound traffic sourced from the new address works, the gateway is still reachable, and SMTP still answers.
- [x] C2.0c Forward record `news.alomails.com → 159.195.89.28` at **Namecheap** — added 2026-08-17, propagated and resolving at both Google and Cloudflare. Nothing else in the zone touched.
- [x] C2.0d **Begin warm-up as soon as it can send**, not when C2 starts. This is
      Phase 0's long-unchecked "IP warming begins now", and it is the only item
      in this track whose cost is calendar time that cannot be recovered later —
      a cold IP sending its first real campaign is filtered however correct the
      DKIM is.

      **Started 2026-08-18**, the day the identity could first sign and send.
      The schedule, what is watched each week, and the day-by-day log live in
      `docs/design/sending-reputation-warm-up.md`. The ramp is conditional on a
      clean week rather than on the calendar, and the first fortnight is
      honestly seed sends — there is no send path (C4) and no consenting
      audience yet, which is precisely why the clock had to start anyway.

- [x] C2.1 A dedicated sending subdomain per tenant with its own SPF, DKIM selector and DMARC alignment, provisioned and **verified on the wire together**, as the transactional trust stack was — never one record at a time, because a record nobody has tested is one everybody assumes is right.

  **Verified 2026-08-18 for `news.alomails.com`, at an independent receiver, on
  the real wire** — the three records and the egress judged together, which is
  the only reading that means anything:

  ```
  Received-SPF: Pass (mailfrom) identity=mailfrom; client-ip=159.195.89.28
  dkim=pass (2048-bit key) header.d=news.alomails.com header.s=camp header.a=rsa-sha256
  dmarc=pass (p=none dis=none) header.from=news.alomails.com
  ```

  Two failures were found by sending rather than by reading, and neither would
  have shown up in a test: Docker's per-bridge masquerade rule outranked the
  SNAT rule that selects the campaign address, so the mail left by the
  transactional IP while every log line said it was pinned; and the greeting
  still named the transactional host, which the receiver scored down by 3 of 10
  because it did not match the campaign address's reverse DNS. Both fixed.

  **Per-tenant provisioning is still manual.** This proves the shape end to end
  for our own identity; the self-service half needs the DNS automation in
  `docs/design/dns-onboarding.md` and is not claimed here.

  **The last record landed the same day:** `news.alomails.com MX 10
  mail.alomails.com`, published at Namecheap and resolving at Google, Cloudflare
  and the authoritative servers, with the apex MX and the rest of the zone
  untouched. It was the only authentication deduction left (−3), because a
  sending domain that cannot receive looks one-way. Mail to
  `bounces@news.alomails.com` now reaches our MX and is refused with
  `550 5.7.1 Relaying denied: recipient not local` — a clean permanent refusal
  rather than a black hole or an open door. **A working return path is still
  C2.10**, which needs the domain accepted for delivery and something that reads
  what arrives.

  **Why the parent SPF must not simply be widened.** `alomails.com` publishes
  `v=spf1 mx -all` and `p=quarantine; adkim=s; aspf=s`. Strict alignment means an
  envelope sender at `alomails.com` sent from the new IP **fails SPF and is
  quarantined** — and the fix is emphatically not to add the campaign IP to the
  parent's SPF. That would hand the marketing stream the transactional domain's
  reputation, which is what buying a second IP was meant to prevent. The whole
  point is a separate identity, so it gets separate records.

  Worked example for our own domain, the shape every tenant's gets:

  | record | value | why |
  |---|---|---|
  | `news.alomails.com` TXT | `v=spf1 ip4:159.195.89.28 -all` | authorises the campaign IP and nothing else |
  | `<selector>._domainkey.news.alomails.com` TXT | the DKIM public key | generated **on the sending host**; the private half never leaves it |
  | `_dmarc.news.alomails.com` TXT | `v=DMARC1; p=none; adkim=s; aspf=s; rua=…` | report-only while the parent stays at `quarantine`, and **strict alignment from the start** |

  **Published 2026-08-17** and verified at Google and the registrar's own
  resolvers: SPF, the 410-character DKIM key (byte-identical to the generated
  one, decoding to a well-formed RSA-2048 DER key, so not silently truncated by
  the 255-byte TXT string limit), and the subdomain DMARC. The parent's
  `v=spf1 mx -all` and `p=quarantine` were not touched.

  **Alignment is strict from the first day, deliberately.** A subdomain policy
  record carries no alignment tags by default, which means relaxed — and under
  relaxed a message `From: @news.alomails.com` can pass DMARC on a DKIM
  signature of `d=alomails.com`, because the two share an organizational domain.
  That would let the campaign identity authenticate with the transactional key,
  which is the separation the second IP exists to create. Strict costs nothing
  while `p=none` only reports, and tightening `p=` later then changes one
  variable instead of two.

  **Strict alignment makes three things exact, and the sender must be built for
  it** — this is a constraint on C2.1's code half, not a DNS detail. All three
  must be exactly `news.alomails.com`, with no parent domain and no deeper
  label: the **From: header domain**, the **envelope-from** (`aspf=s`), and the
  DKIM **`d=`** (`adkim=s`). A sender that defaults its bounce domain elsewhere
  fails alignment even while SPF and DKIM each pass on their own, which is the
  confusing failure to expect — two green checks and a DMARC fail.

  Watch during warm-up rather than before it: a VERP return path for
  per-recipient bounce attribution (C2.10) lives at a sub-subdomain and will not
  align under `aspf=s`. DMARC passes if **either** identifier aligns, so a
  correct `d=news.alomails.com` signature carries it — and the `p=none` reports
  will say so before anything is enforced.

  That last row is load-bearing and easy to miss: `_dmarc.alomails.com` carries
  **no `sp=` tag**, so subdomains inherit `p=quarantine` today. Warming a new
  identity under an inherited enforcing policy means early misconfigurations are
  quarantined instead of reported, which is the opposite of what a warm-up is
  for. Publishing a subdomain policy is the only way to differ from the parent,
  and it is tightened to `quarantine` once the reports come back clean.
- [x] C2.1a **Decide how the campaign identity signs — a fork found by reading the code, not by design.** Per-domain DKIM already exists (ADR 0014): `authmail` resolves a signing key by the `From` domain from `dkim_keys` and falls back to the one configured file key. But the store path is **ed25519 only** — `authmail`'s own comment says the stored seed is consumed as Ed25519 — while the file-key fallback does RSA and signs for a single domain, the transactional one. The RSA key published by hand at `camp._domainkey.news` therefore fits neither path, and would be ignored rather than used. Three ways out, and the choice is a deliverability judgement rather than a coding one:
  1. **Use the product's own path.** Register `news.alomails.com` as a domain, let `ensure_dkim_key` generate and store its key, publish that record instead. Cleanest — rotation already has a route (`/admin/domains/dkim/rotate`) and no private key sits in a file. But it signs **ed25519 only**, and RFC 8463 support among receivers is still patchy — which matters more for bulk mail than anywhere else in the product.
  2. **Teach the store path RSA.** The `dkim_keys` table already carries an `algorithm` column and `DkimSigningMaterial` already returns it; what is missing is the signer honouring it. Keeps the published RSA record and the automatic per-domain resolution.
  3. **Dual-sign, RSA and ed25519.** What large senders do, and the most work. Worth it only if 2 proves insufficient in the aggregate reports.
  Recommendation: **2**, because bulk mail is exactly where a verifier that cannot check ed25519 costs real delivery, and the column that makes it possible is already there. Whatever is chosen, the DNS published on 2026-08-17 must be reconciled with it — the record is currently for a key nothing will sign with.

  **Decided 2026-08-18: 3, dual-sign** — which is 2 plus one more row, since
  teaching the store path RSA was the bulk of the work either way. `7e331ced`
  moved the store's constraint to one active key per domain **per algorithm**
  and made `sign_outbound` emit one signature per active key, RSA first.
  Following the decision through the code found three things it needed and did
  not have, all built here (`docs/design/campaign-sending-identity.md`): nothing
  could put an RSA key into `dkim_keys` at all, so the published `camp` record
  was still for a key nothing would sign with (now `alo-smtp
  --install-dkim-key`); the admin API rendered every stored key as `k=ed25519`
  and returned only the first of them; and outbound never chose a source
  address, so a `news.` message would have left by the transactional IP and
  failed SPF against `-all` while its DKIM passed.
- [x] C2.1b **Egress: a message leaves by the address its own identity authorises.** `ALO_SMTP_EGRESS_IPS` maps a sending domain to a source address, matched on the **envelope-from** because that is the identity SPF is evaluated for; the outbound client binds it, and a pinned address of the wrong family fails the attempt rather than falling back to the kernel's choice — mail deferred with a reason is recoverable, mail delivered under a failing identity is not. On Docker this needs a lane of its own: a container cannot bind one of the host's public addresses, so alo-smtp binds a private address on a fixed compose network and the host source-NATs it (`ops/systemd/alo-campaign-egress.service`). Proved on the server before anything was built on it — the pinned path reports `159.195.89.28` to an outside observer while the default path reports `152.53.179.142`.
- [ ] C2.2 A separate queue and egress path, proven by a test that queues a campaign and sends a password reset behind it — the reset must not wait
- [ ] C2.3 Per-tenant warm-up and rate limits, with the cap and its reason shown in the send flow
- [x] C2.4 `List-Unsubscribe` and `List-Unsubscribe-Post` on every campaign message — the **writer** half of what `unsubscribe.rs` already reads. This is the header the mail client turns into its own Unsubscribe button, and it must work with a single POST and no login. **Built** (`campaign_unsubscribe_link.rs`): the RFC 2369 §3.2 header and RFC 8058 §3.1's literal `List-Unsubscribe-Post: List-Unsubscribe=One-Click`, carried out of `render_campaign_message` with the body so a sender cannot forget them. A non-HTTPS URL is refused rather than emitted beside a header every client would ignore, and a URL carrying CR or LF cannot smuggle a second header. The endpoint that answers the POST is C2.6/C2.7's, which already works with no account and no login.
- [x] C2.5 **The visible link in the mail**, because the header is not enough: only some clients render a button from it, and everyone else scrolls to the footer looking for the word. Every campaign carries one, it is never disguised as anything else, and it goes to a page that works with no account and no login. **Built** in both parts, from one invitation, so the two alternatives can never offer different ways out: under the card in HTML, below a `--` separator with the URL written in full in the text part. `unsubscribe` is a **required** field of `CampaignLetter` — a campaign that cannot be left does not compile — and the words are the caller's, in the recipient's language, because the store holds no translations.
- [x] C2.6 The unsubscribe link is a **per-recipient unguessable token**, not an address in a query string. Two failures it prevents: somebody iterating identifiers to unsubscribe other people, and a scraper confirming an address is live by watching what the page does. The token identifies the send and the recipient, and reveals neither to whoever holds it. **Built:** `campaign_unsubscribe.rs` mints a 256-bit URL-safe secret, stores only its digest and never reads it back; the raw token exists once, at mint time. Placing it in a message is C2.4/C2.5.
- [x] C2.7 The landing page offers **fewer rather than only none** — this campaign type, or everything. A recipient who only wanted the newsletter to stop has no way to say so if the single button is "unsubscribe from all", and the alternative they reach for is the spam button, which is the signal that ends a sending reputation. Confirmed in one click either way, with no "are you sure" maze. **Built:** `UnsubscribeView.tsx` offers this topic or everything, confirmed in one click, with no account and no login (`campaign_topic_optout.rs` records the narrower choice).
- [ ] C2.8 **Transactional mail never carries an unsubscribe** — an invoice, a password reset and a meeting invitation are not marketing, and offering to stop them is both wrong and a support ticket. The separate sending identity (C2.1) is what makes this distinction structural rather than a flag somebody sets.
- [ ] C2.9 An unsubscribe suppresses **immediately and tenant-wide** through C1.3's rule, before the next batch of the same send goes out — a recipient who unsubscribes at 10:00 and receives batch four at 10:05 has been told the button does not work.
- [ ] C2.10 Bounce and complaint feedback acted on: hard suppresses, soft retries then suppresses, complaints count against the tenant's rate

### Wave C3 — building the email

- [x] C3.1 The content model is the Docs block model — one editor, not a second one. A campaign is blocks, plus a subject and a preheader. **Built:** `campaign_content.rs` — the alo Docs block model, validated on every write, with a `schema_version` envelope.
- [x] C3.2 The email renderer: blocks → **email-safe HTML**, table layout and inline CSS, because Outlook still renders through Word. This is the hard part of the wave, and it is a compiler rather than a stylesheet. **Built:** `campaign_html.rs`, held to fixtures by `campaign_html_golden.rs`.
- [x] C3.3 A plain-text alternative generated from the same blocks, sent as `multipart/alternative`. Not optional: a campaign with no text part is scored as spam by filters older than every design decision here. **Built:** `campaign_text.rs` (golden-tested) with `campaign_mime.rs` assembling the `multipart/alternative`.
- [x] C3.4 Personalisation from the record — first name, company, last order — with a **visible fallback for every field**. "Hi ," is the classic bulk-mail failure and it comes from a merge field nobody defaulted. **Built:** `campaign_merge.rs` — a merge field with no fallback is a validation error at save, not a blank at send.
- [x] C3.5 Dark mode, blocked images and accessibility: the mail must read with images off, every image carries alt text, and colour is never the only carrier of meaning. Half of recipients see the degraded version, and they are not a degraded audience. **Built:** in `campaign_html.rs`, covered by the golden fixtures.
- [~] C3.6 Preview and test send: the rendered mail, its text part, and a send to a seed address before any audience may be chosen. **Honest limit written into the screen** — we own no rendering farm, so a preview is our renderer's opinion, not proof of how Outlook 2016 will draw it. **Preview built** (`campaign_preview.rs`, `LetterPreview.tsx`), including the honest-limit note. **Test send is not built and cannot be** until there is a send path — it is the one half of this item that C4 gates.

### Wave C4 — sending it

- [x] C4.1 A send is a durable job with per-recipient rows, so a crash resumes rather than restarts and **nobody is mailed twice** — idempotency on (campaign, address), enforced in the store. **Written, not yet proven.** `campaign_send.rs` + migration 0800: the job, the per-recipient ledger, and idempotency on (campaign, address) — enforced per *campaign* rather than per send, so a stop-fix-resend cannot re-mail anybody. **Nine tenancy tests pass** against real Postgres, including the one this item exists for — a second send of a campaign enrols nobody the first already reached — and the mandatory wrong-tenant test across all six verbs. The dispatcher that consumes this ledger is C4.2.
- [ ] C4.2 Rendering happens per recipient at send time, and a render failure suppresses that one recipient rather than failing the campaign.
- [ ] C4.3 Pacing: batches sized by the tenant's warm-up cap, spread rather than burst, and a campaign backlog that never delays transactional mail — C2.2's guarantee, tested again under load.
- [ ] C4.4 Schedule, and **pause/stop mid-send**. The control that matters most: the first thing anybody notices after pressing send is the typo. Stopping is immediate, and what has already gone is reported honestly.
- [ ] C4.5 The send screen as a safety screen: consent count, exclusions named, sending identity shown, warm-up cap and its reason, and a test send required before the button is live.

### Wave C5 — what actually happened

- [ ] C5.1 Delivery events, which are **facts**: accepted, deferred, hard bounce, soft bounce, complaint — from our own SMTP result and the feedback loop, per recipient, with C2.5's suppression firing off them.
- [ ] C5.2 Click tracking by redirect — a first-party link the recipient followed, so it is **reliable**. Per link and per recipient, fast, and it must never lose the destination when tracking is off.
- [ ] C5.3 Open tracking, **opt-in per campaign and disclosed** (ADR 0044 §5), reported as an estimate and never as a headline: the screen itself states that Apple pre-fetches images, so the figure includes machines. A number shown without that caveat is a lie told by omission.
- [~] C5.4 **Read duration — decided against, ADR 0052.** Not built, and not a gap to be filled later: the measurement needs a pixel held open and reporting, which most clients block and many cache, while Apple pre-fetches on delivery — so the error runs in a direction and magnitude nobody can size. A metric that cannot be caveated honestly cannot be shipped honestly. Open tracking remains available opt-in per campaign (ADR 0044 §5); what is refused is timing a person's attention on top of a signal that is already weak.
- [ ] C5.5 Attribution: delivered → clicked → visited → converted → **invoiced**, joined to the invoice rather than estimated, in euros. The campaign list's headline column, and the reason the wave exists.
- [ ] C5.6 The report screen, ordered by trustworthiness: money and delivery first, clicks next, opens last and labelled an estimate. The campaign's numbers export, because a customer's results are theirs.

### Wave C6 — the campaign that sends itself

- [ ] C6.1 Automations: a send triggered by a CRM stage change, an invoice going unpaid, or a form submission — the same consent, suppression and identity rules, and no new sending path.
- [ ] C6.2 A/B on subject and content, decided on **clicks and revenue** rather than opens: the metric a test optimises had better be one that means something.
- [ ] C6.3 ★ The Campaigns agent: a sentence becomes a segment you can see and edit, and a draft campaign proposed for approval. The query is the artefact, never the copy.

### Exit gate — Campaigns done when:

- [ ] A real campaign goes to a real segment from a dedicated subdomain, and the transactional domain's reputation is measurably untouched
- [ ] The campaign's row shows money invoiced, traced to the invoices behind it
- [ ] An unsubscribe is honoured everywhere immediately and survives a re-import — tested from **both** doors: the client's own header button, and the link in the footer
- [ ] Somebody who unsubscribes mid-send does not receive the batches that follow
- [ ] A send was paused mid-flight, and the report says truthfully how many had already gone
- [ ] The report screen states, in the interface, which of its numbers are facts and which are estimates

---

## Sites track — alo Sites, the AI-native website builder (ADR 0036) ⇄ parallel to the Business track

Second autonomous loop (office PC): a no-code, **AI-first** website builder —
"describe your business" → full draft site → conversational preview-then-
approve editing. V1 = marketing site + blog (written in alo Docs) + forms,
published to a subdomain instantly **and** connectable to custom domains.
Section-JSON model, static Rust rendering, a separate public `alo-sites`
service, privacy-first analytics. Queue: `docs/autonomy/sites/QUEUE.md`
(items S1.01–S1.32).

### Wave S2 — after the S1 gate (queue authored at S1 wave review)

Collections (CMS from alo Base tables, ADR 0037-adjacent), the site-editor
role, whole-site AI translation, version history/rollback, richer images —
and ★ **Site Insights** (features.md [S2]): the EU answer to Google
Analytics — consent-free referrers/campaigns/geo/devices, aggregated
click+scroll heatmaps, conversions, and full-funnel attribution
(visit → form → CRM lead → invoice) surfaced in the Insights tab.
Session replay / individual tracking: permanent non-goal.

### Exit gate — Sites v1 done when:

- [ ] A real business site (Axon Group or aloworld itself) is generated from a description, edited, published on a custom domain, receives a form submission that lands in the owner's inbox, and publishes one blog post written in alo Docs — with analytics counting the visit and storing no personal data

---

## Agent track — an agent in every product (ADR 0034) ⇄ runs alongside Phases 3–6

The framework is built and six product agents already have tools, but they all
share one brain: an agent's tool set and its retrieval are not scoped to its
product, and every tool — including the read-only ones — arrives as a proposal
to approve. So "the Inventory agent" is a name on the same generic assistant,
and asking it whether something is in stock offers a button instead of an
answer.

A1 fixes that for every agent at once and gates the rest: nothing below is
worth building on an agent that cannot answer a question or tell its own
product from anyone else's.

### Wave A1 — what makes an agent a *product* agent

- [ ] A1.1 Reads answer, writes propose: a read-only tool returns its answer in the room immediately; only a tool that changes something waits for approval. Both paths audited; the split is a property of the tool, not of the caller
- [ ] A1.2 A product on the agent record, and the tool registry scoped by it — an agent is offered its own product's tools and no others, refused at the execution boundary as well as in the prompt
- [ ] A1.3 Product-scoped retrieval: each agent grounds its answer in its own product's records, not one shared workspace search. Cross-product questions belong to Ask alo, the only agent allowed to look everywhere
- [ ] A1.4 One-to-one with an agent: a DM whose counterpart is an agent rather than a colleague — its own channel kind, since a DM key of two user ids cannot express it
- [ ] A1.5 The default set: every tenant gets its agents without an admin registering handles by hand; a module a tenant cannot open has no agent (per-user module access already decides this)
- [ ] A1.6 A wrong-tenant and wrong-user test per agent surface — channel, DM and in-module — proving an agent reaches nothing the asker could not

### Exit gate — A1 done when:

- [ ] In a channel, `@mail are we in contact with ABC?` answers from correspondence with the messages behind it, and `@inventory is the X100 in stock?` answers from the stock record — both with no button in between
- [ ] The same two questions in a DM with each agent answer identically
- [ ] An agent asked for something outside its product declines and names the agent that owns it, rather than answering from a search snippet
- [ ] A colleague who cannot open Billing gets no answer from the Billing agent, in any surface

### Wave A2 — the agents with no tools yet

Each item is that agent's read tools **and** its writes, to the depth of the
implement skill — an agent that can only answer is half a product.

- [ ] A2.1 ★ Website (Sites) agent: answer from the live site, draft and edit a page, translate the site, review SEO — publishing proposed, never silent (`alo-ai/site_edits.rs` and `site_translation.rs` are the foundation)
- [ ] A2.2 ★ Sheet agent: formula from intent, explain a formula, clean a column, answer from the data with the cells cited, chart from intent
- [ ] A2.3 ★ Docs agent in chat and DM: the editor's agent mode reachable from a room — draft a section, rewrite a selection, translate a document
- [ ] A2.4 ★ Insights agent: answer from the numbers, explain a change, build a report
- [ ] A2.5 Drive agent beyond `find_file`: summarise a document, extract from an attachment, propose a move or a rename
- [ ] A2.6 Agenda agent beyond reads: find a time across several diaries, prep a meeting from its thread and attachments, reschedule
- [ ] A2.7 Tasks agent beyond `create_task`: what is on my plate, prioritise, chase an overdue owner, extract actions from a thread

### Exit gate — A2 done when:

- [ ] Every module in the rail has an agent that answers a real question about that module from the record, in all three surfaces
- [ ] Each agent's refusals are as tested as its answers: it declines to guess rather than answering from a snippet

### Wave A3 — orchestration, and the meeting

- [ ] A3.1 ★ Ask alo orchestrates rather than owns: it routes to the product agents and runs multi-step work across them — one approval surface, a visible plan, a working **Stop**
- [ ] A3.2 ★ Meet, after the fact: minutes, decisions and actions into the meeting's thread, where they become tasks and events through the ordinary agent path
- [ ] A3.3 Meet, live — **ADR first**: an agent as a LiveKit participant that hears the room and answers in-call. A media path, not a tool set, and not to be started before it is decided
- [ ] A3.4 The agent directory: what each agent is for, what it may touch, and what it has done — browseable, per tenant

### Exit gate — Agent track done when:

- [ ] "Summarise the Acme thread, draft a reply, and block an hour to review it" runs as one request across three agents, with one approval and a working Stop
- [ ] A meeting ends and its actions are in the right people's task lists, each traceable to the minute it came from
- [ ] Every agent's audit trail names the asker, the tool, the record it read, and the approval that let it write

---

When every box in a phase is checked, mark the phase header — DONE and record
the date of the gate in git history, not in this file. The file stays about
what; git stays about when.
