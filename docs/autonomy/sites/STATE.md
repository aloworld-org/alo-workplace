# sites/STATE.md — Sites-track loop journal (append-only; newest at the bottom)

One entry per iteration: item id, what shipped, how verified, cuts/flags,
next item. The end-of-queue / emergency-stop control markers the wrapper
watches for are defined in LOOP.md — never write those exact phrases here
except to actually fire them.

Human-action inbox (things the loop must not do itself):

- **Domain/DNS done (2026-08-07):** `alosites.com` is purchased and its
  apex/wildcard/www DNS is live. Use `SITES_DOMAIN=alosites.com`.
- **Next production deploy:** add the `alo-sites` service to compose with its
  database/blob settings and a strong `ALO_SITES_ANALYTICS_SECRET` (at least
  32 bytes). Route workspace `/sites` to `alo-jmap`, wildcard and custom
  public Hosts to `alo-sites`, and Caddy on-demand TLS decisions to
  `/internal/tls/ask`. The loop never edits `deploy/`.
- **Enable real generation:** in the live tenant's Settings, configure and
  select an OpenAI-compatible AI provider with its base URL, model, and key
  when required. Loop tests use fixtures and never call an external model.
- **Customer custom-domain help:** retain the shown TXT proof through
  verification, then CNAME a subdomain to the deployment ingress; apex
  domains need ALIAS/ANAME or CNAME flattening. Explain that HTTPS may take a
  few minutes after DNS propagates.
- **Post-launch hardening:** submit `alosites.com` to the Public Suffix List
  so browsers isolate customer subdomains from each other.

---

## S1.01 — design note docs/design/sites.md (2026-08-06)

- **Shipped:** `docs/design/sites.md` — full v1 design: data model (sites,
  site_pages with versioned typed-section JSON envelope, immutable
  site_page_snapshots, themes, site_posts, site_forms/submissions,
  site_domains, site_analytics_daily), render pipeline (pure render lib
  shared by alo-sites public serving and the alo-jmap draft preview),
  two-service boundary, form flow (honeypot/rate-limit/internal-delivery),
  privacy analytics model (no-PII schema asserted by test), tenancy model
  incl. the two deliberate global surfaces (subdomain unique index,
  host→site resolver), error maps for both services, and out-of-scope list.
  Both required rejected alternatives recorded: free-form canvas vs typed
  sections, and public serving inside alo-jmap vs a separate binary.
- **Verified:** docs-only item — no code gates apply; note answers all four
  implement-skill blocks (Surface / Errors / Tenancy / Out of scope) plus
  the rejected alternative required by the queue's done-criterion.
- **Cuts/flags:** none. CHANGELOG untouched (no behaviour change).
- **Next:** S1.02 (sites migration + store + wrong-tenant tests).

## S1.02 — sites migration + store module (2026-08-06)

- **Shipped:** migration `0055_sites.sql` (tenant-scoped `sites` table,
  tenant-cascade FK, and the one deliberate global surface: a cross-tenant
  unique index on `subdomain`); new `SiteId`; `platform/alo-store/src/sites.rs`
  on the account door — `create_site` / `sites` / `site` / `rename_site` /
  `set_site_subdomain` / `delete_site` / `subdomain_available`, with subdomain
  validation (DNS-safe `[a-z0-9-]{3,40}`, no edge hyphens, ~80-word reserved
  list covering infrastructure/mail/identity/brand labels) and a
  `SiteStatus` draft/live enum. Unique-violation on the subdomain index maps
  to `Conflict("subdomain is already taken")` — taken/free only, never owner.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store
  --all-targets` zero warnings; full `cargo test -p alo-store` green against
  the local docker Postgres (migration really applied — `\d sites` shows the
  table, PK, global unique index, cascade FK, and rows from the test run).
  New tests: 5 unit tests on validation/status tokens (incl. one asserting
  every reserved-list entry passes the syntax rules — caught dead-weight
  `mx`), plus `sites_scope_by_tenant_and_subdomains_are_globally_unique` in
  the isolation suite: outsider tenant gets clean `NotFound` on every path,
  co-tenant user shares the sites, cross-tenant claim collides with a
  taken-only message, delete releases the subdomain.
- **Cuts/flags:**
  - No theme setter yet — the column ships with `'{}'` default; a raw
    unvalidated write surface would predate the typed theme model, so the
    setter lands with S1.05.
  - No status setter — `live` is the publish flow's to flip (S1.08).
  - Drive-by fix (out of item scope but blocking the mandatory gate): the
    pre-existing isolation test `deleting_a_tenant_purges_its_tasks` was
    red on main — `task_projects()`'s lazy `ensure_personal_project()`
    INSERT hits the tenant FK after tenant deletion and surfaced as a `Db`
    error. `tasks.rs` now treats that FK violation (SQLSTATE 23503) as
    "nothing to ensure", so a deleted tenant reads as absent, never a 500.
  - `cargo fmt -p alo-store` also normalized previously unformatted code in
    `base.rs`, `tasks.rs`, and older `tenant_isolation.rs` tests —
    mechanical churn, kept so the crate is fmt-clean.
  - CHANGELOG untouched: storage foundation only, no user-visible behaviour.
- **Next:** S1.03 (typed section schema v1 in a `site_model` module).

## S1.03 — typed section schema v1, `site_model` (2026-08-06)

- **Shipped:** `platform/alo-store/src/site_model.rs` — the closed v1 section
  vocabulary as an internally-tagged serde enum (`type` tag, snake_case):
  nav, hero, features, text_image, gallery, testimonials, pricing, team,
  faq, cta, contact_form, footer, each with typed props and
  `deny_unknown_fields`; the `SectionsEnvelope { schema_version, sections }`
  write gate (`from_value` = version check before shape check → strict serde
  parse → content rules); content validation covering text bounds
  (300/5 000 chars), list bounds (≤50, non-empty where meaningless empty),
  href allowlist (`/path`, `#fragment`, http(s)/mailto/tel; rejects
  `javascript:`, `data:`, protocol-relative — stored hrefs are always safe
  in an `href` attribute), blob/form id token shape, and icon token shape.
  Pricing `price` is a display string by design (no money computation —
  integer-cents law not in play, per the design note). Golden fixtures: 12
  per-section envelopes + a full-page fixture with all 12 in order
  (`tests/fixtures/site_sections/`), pinned by `tests/site_sections.rs`
  round-trip-to-identical-Value tests. Enabling change: the `opaque_id!`
  macro now also derives serde Serialize/Deserialize (newtype-transparent,
  purely additive) so `BlobId` can live typed inside section JSON.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store
  --all-targets` zero warnings; full `cargo test -p alo-store` green against
  the local docker Postgres (77 unit incl. 10 new schema tests — exhaustive
  fully-populated and minimal round-trips, wire-tag pinning, unknown
  type/prop rejection, future-version error precedence, href/token/content
  rules — plus 3 new golden-fixture tests and the whole isolation suite).
  No storage/routes touched, so wrong-tenant and wire-verify gates don't
  apply; pure model only.
- **Cuts/flags:**
  - Read-side tolerance (skip-with-log on unknown sections so an old
    renderer survives a newer snapshot) deliberately NOT here — it is the
    renderer's job and lands with S1.06, as the design note specifies.
  - `contact_form.form_id` is a plain validated token (`Option<String>`)
    until the forms table + id newtype land in S1.16; wire shape is final.
  - Environment note: parallel rustc runs OOM-killed the first full test
    build on this machine; `cargo test -j 2` builds fine. The DB tests need
    `DATABASE_URL=postgres://alo:alo-dev-only@localhost:5432/alo` (harness
    default points at 5433).
  - CHANGELOG untouched: schema foundation only, no user-visible behaviour.
- **Next:** S1.04 (site_pages migration + store, sections validated through
  this module on every write).

## S1.04 — site_pages migration + store module (2026-08-06)

- **Shipped:** migration `0056_site_pages.sql` (tenant-scoped `site_pages`,
  composite FK cascading tenants → sites → pages, per-site slug unique index,
  partial unique index enforcing one home page per site, and a CHECK that
  only the home page may hold the empty slug); new `SitePageId`;
  `platform/alo-store/src/site_pages.rs` on the account door —
  `create_site_page` (appends at end of nav order, empty sections envelope,
  200-pages-per-site cap) / `site_pages` / `site_page` / `set_page_title` /
  `set_page_slug` / `set_page_seo` (trim, blank-clears, 200/500 char caps) /
  `set_page_sections` (the schema write gate: `SectionsEnvelope::from_value`
  from S1.03, canonical serialization stored) / `set_home_page` (transactional
  demote+promote; demoting a home at the empty slug is a named Conflict) /
  `reorder_site_pages` (full-permutation rewrite in a transaction) /
  `delete_site_page`. Slug rules: `[a-z0-9-]{1,80}`, no edge hyphens,
  reserved public paths (`blog`, `f`, `feed`, `rss`, `atom`, `sitemap`,
  `robots`, `healthz`, `assets`, `static`) rejected; empty slug is the home
  page's spelling, DB-enforced so the rule holds under concurrency. All
  constraint violations map to named `Conflict`s, never a raw 23xxx error.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store
  --all-targets` zero warnings; full `cargo test -p alo-store` green on the
  local docker Postgres (82 unit incl. 5 new slug/SEO rule tests; isolation
  suite 22 incl. the new
  `site_pages_scope_by_tenant_and_site_with_slug_and_home_rules` — outsider
  tenant cleanly denied on all ten paths, same-tenant cross-SITE addressing
  denied, per-site slug reuse allowed, home-flag flip + empty-slug demote
  conflict, sections gate accept/reject, reorder permutation checks, delete
  frees slug, site delete cascades pages). Manual pass: `\d site_pages` in
  psql shows the PK, both unique indexes, the CHECK, and the cascade FK
  exactly as designed.
- **Cuts/flags:**
  - `sections` is returned as stored (opaque `Value`) on read — typed
    read-side handling with skip-with-log tolerance is the renderer's job
    (S1.06), per the design note; the write gate guarantees whatever is on
    disk passed the schema.
  - No route/UI surface yet (S1.10/S1.11) — store + tests only, so the
    wire-verify gate doesn't apply; CHANGELOG untouched (no user-visible
    behaviour).
  - Page cap decision: `MAX_PAGES_PER_SITE = 200` (not in the queue text;
    bounded input everywhere — revisit with quotas if it ever binds).
- **Next:** S1.05 (theme model: palette+typography presets + logo/favicon
  blob refs).

## S1.05 — typed theme model + theme setter (2026-08-07)

- **Shipped:** `platform/alo-store/src/site_theme.rs` — the theme envelope
  `SiteTheme { schema_version, preset, logo?, favicon? }` with the same
  gate pattern as S1.03 (`from_value` = version-before-shape strict parse →
  content rules; `deny_unknown_fields`; absent options stored as absent
  keys) plus `from_stored`, the never-fail read spelling that maps the
  pristine `{}` column default to the default theme. Seven shipped presets
  (`north` default, `ink`, `terra`, `fern`, `plum`, `carbon`, `midnight`),
  each palette (7 hex tokens) + typography (system-font stacks ONLY — a
  published site loads no third-party font, that's the privacy promise —
  plus heading weight), as static tokens the S1.07 stylesheet generator
  will read. Deferred-from-S1.02 setter landed: `set_site_theme` on the
  account door, storing the canonical serialization; schema violations map
  to named `Conflict`s like the sections gate. `site_model`'s id-token rule
  is now shared (`pub(crate) valid_id_token`) so "a valid id" means one
  thing across the sites schema family.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store
  --all-targets` zero warnings; full `cargo test -p alo-store` green on the
  local docker Postgres (90 unit incl. 8 new theme tests — ≥6-presets +
  unique-wellformed-ids, hex format + **WCAG AA contrast ≥4.5:1 enforced on
  every text pairing of every shipped palette**, full/minimal round-trips,
  version-error precedence, unknown prop/preset/blob-ref rejection,
  from_stored pristine + defensive paths; isolation suite 22 with the sites
  test extended: outsider `set_site_theme` cleanly denied, co-tenant write
  lands canonically and reads back, four off-schema payloads rejected
  without touching the stored value). Manual pass: psql shows real rows
  with the pristine `{}` default; the terra write was read back from the
  real DB through the store inside the isolation test.
- **Cuts/flags:**
  - v1 is presets-only, no free-form colors — rejected alternative recorded
    in the module doc: arbitrary user hex would break the build-time
    contrast guarantee the preset test enforces.
  - Logo/favicon blob refs are shape-checked only (same posture as S1.03's
    `SiteImage.blob_id`); ownership resolves through the tenant-scoped blob
    door at render/serve time.
  - Preset display names ("North", "Terra", …) are product proper nouns,
    deliberately not i18n'd — documented on `ThemePreset`.
  - `Site.theme` stays an opaque `Value` on read; renderers use
    `SiteTheme::from_stored`. CHANGELOG untouched (no user-visible surface
    until S1.10/S1.14).
- **Next:** S1.06 (renderer crate `products/sites/alo-sites`, page JSON +
  theme → full HTML document, golden tests).

## S1.06 — renderer crate `products/sites/alo-sites` (2026-08-07)

- **Shipped:** new workspace crate `alo-sites` (library-first; the axum
  service is S1.09) with the pure `render` module: page JSON + theme → one
  complete HTML document. `render_page(SiteRenderContext, PageRenderContext)`
  emits head (charset/viewport, title `seo_title` or `<page> — <site>`, meta
  description, canonical, OG type/site_name/title/description/url, og:image
  from the first hero image, favicon from theme, one stylesheet link) and a
  landmarked body: skip link → `nav` sections as `<header>` → one `<main>` →
  `footer` sections as `<footer>` (rule documented: a mid-page nav still
  lands in the header region — valid landmarks outrank literal order). One
  fragment builder per section type in `render/sections.rs` (h1 only in hero,
  h2 per section, h3 per item, `<details>` FAQ, `alt` on every `<img>`,
  stable `s-<kind>` class hooks for the S1.07 stylesheet). Read-side
  tolerance per the design note: `sections_lenient` parses per-entry and
  skips unknown/newer sections with a `tracing` warning — never a 500.
  Defense in depth independent of the write gate: every string through
  `esc()`, every link target re-checked against the href allowlist (unsafe →
  inert `#`, in `render/html.rs`). Visitor-facing chrome strings (skip link,
  menu, form labels) externalized in `render/strings.rs` (`UiStrings`, `EN`).
  Contact form: posts `/f/<form_id>`, fixed v1 field contract name/email/
  message + visually-hidden `website` honeypot (aria-hidden, tabindex −1),
  `data-success` attribute; without a `form_id` the section renders text
  only. Public-path contract documented in `lib.rs`: `/assets/site.css`,
  `/assets/img/<blob_id>`, `/f/<form_id>` — changing these means
  re-rendering every snapshot.
- **Verified:** `cargo fmt --check`; `SQLX_OFFLINE=true cargo clippy -p
  alo-sites --all-targets` zero warnings; `cargo test -p alo-sites` green —
  13 golden files (one per section type + a themed full-page golden with
  logo/favicon/SEO, blessed via `UPDATE_GOLDENS=1` then re-run clean) and 11
  behavior tests (head/OG/canonical exact strings, landmark order, lenient
  skip incl. newer schema_version, script + attribute-injection escaping,
  javascript: href rendered inert, honeypot + fixed fields, alt on every img
  across the full corpus, theme logo/favicon paths). Manual pass: read the
  full-page golden byte-for-byte — structure, escaping (`&#39;`, `&amp;`),
  and all head tags check out. No storage/routes touched → wrong-tenant and
  wire-verify gates don't apply (pure library).
- **Cuts/flags:**
  - Feature icons: the schema's icon token renders nothing yet (we ship no
    icon set); the fallback path is the only path until an icon set arrives
    with the stylesheet slice, or the prop is retired at wave review.
  - Byte-budget tests (CSS < 50KB, HTML < 100KB) are S1.07's, with the
    stylesheet; the nav toggle button is inert markup until S1.07's JS.
  - Site-level locale selection doesn't exist yet — `EN` chrome strings
    only; fr/nl land at the wave review (S1.31).
  - CHANGELOG untouched: rendering library only, no served surface yet.
- **Next:** S1.07 (stylesheet generation from theme tokens + byte budgets).

## S1.07 — stylesheet from theme tokens + the behavior script (2026-08-07)

- **Shipped:** new pure module `alo_sites::stylesheet` — `stylesheet(&SiteTheme)
  -> String`, the complete CSS document served at `/assets/site.css`: a
  generated `:root` custom-property block from the resolved preset's
  palette/typography tokens over one static mobile-first sheet (single 48rem
  breakpoint, contained images, section layouts for all twelve `s-<kind>`
  hooks, honeypot visually hidden, skip-link focus reveal). Color use sticks
  to store-proven WCAG pairings; the two pairings the sheet adds (links/
  secondary buttons: `primary` on `background`/`surface`) are now pinned in
  the store's contrast test — all presets clear ≥ 4.5:1. Plus the page's
  **entire JS budget**: a static `render/script.rs` block (menu toggle via
  `aria-expanded` + fetch form-submit that swaps in the `data-success`
  message, native-submit fallback on any failure), **inlined** — rejected
  alternative recorded in the module doc: a fourth `/assets/site.js` path
  would widen the public-path contract for a script with zero user data.
  Appended only when the page has a nav or a live form; both behaviors are
  progressive enhancement (no-JS menu renders expanded — collapse only
  exists under the script's `js` class; forms post natively). Forms now
  always carry `data-success` (custom or the new externalized
  `form_success` default string).
- **Verified:** `cargo fmt --check`; `SQLX_OFFLINE=true cargo clippy -p
  alo-sites -p alo-store --all-targets` zero warnings; `cargo test -p
  alo-sites` green (21 tests: new stylesheet_rules suite — site.css golden
  for the default preset, per-preset token wiring + brace balance +
  **CSS < 50KB budget**, self-containment (no `@import`/`@font-face`/`url(`/
  absolute URL — the zero-external-requests promise, mechanically), contract
  selectors incl. the toggle's `[aria-expanded="true"] + ul` pair, pristine-
  theme fallback; render_rules gains script-inclusion exactness + default
  data-success; full-page golden now asserts **HTML < 100KB** — actual:
  CSS 7.6KB, page 5.9KB). Full `cargo test -p alo-store` green on local
  docker Postgres (200 unit + isolation suites) with the extended contrast
  test. Manual pass: read the re-blessed golden diffs byte-for-byte — the
  only change is the script block between `</footer>` and `</body>`, and
  site.css's `:root` carries the north tokens exactly. No storage/routes
  touched → wrong-tenant and wire-verify gates don't apply (pure library).
- **Cuts/flags:**
  - Feature-icon rendering still absent (S1.06 flag stands) — the sheet
    styles no icon slot; retire-or-ship decision at wave review (S1.31).
  - The nav collapse honors only screen width, not menu length; fancy
    behaviors (sticky nav, scroll effects) are out — the JS budget is the
    two behaviors, by design.
  - CHANGELOG untouched: still a rendering library, nothing served yet
    (first user-visible surface lands with S1.09/S1.10).
- **Next:** S1.08 (publish flow: immutable per-page published snapshots +
  site publish state).

## S1.08 — publish flow: immutable snapshots + publish state (2026-08-07)

- **Shipped:** migration `0057_site_publishes.sql` — `site_publishes` (theme
  frozen at publish time, published_by/at, cascading tenants → sites →
  publishes) + `site_page_snapshots` (slug/title/sections/SEO/nav/home frozen
  per page; **deliberately no FK to `site_pages`** — a snapshot must survive
  the draft page being edited or deleted, that's the immutability property) +
  the published-set pointer `sites.published_publish_id` (composite FK to
  site_publishes so it can only name a same-tenant publish; no referential
  action — publishes die only by the site cascade). New `SitePublishId`;
  `platform/alo-store/src/site_publish.rs` on the account door:
  `publish_site` (one transaction: site row locked FOR UPDATE so concurrent
  publishes serialize → named Conflicts for zero pages / no home page →
  publish + snapshot rows copied INSERT…SELECT inside SQL so the snapshot is
  byte-what the write gates admitted → pointer flip + status `live`),
  `unpublish_site` (pointer NULL + status `draft`; history retained;
  idempotent), `current_site_publish`, `site_publish_snapshots` (scoped
  through the site; wrong tenant or wrong site reads as empty/None,
  indistinguishable from absent).
- **Verified:** `cargo fmt --check`; `SQLX_OFFLINE=true cargo clippy -p
  alo-store --all-targets` zero warnings; full `cargo test -p alo-store`
  green on the local docker Postgres (200 unit + all suites; isolation 23
  with the new `site_publishes_freeze_immutable_snapshots_and_scope_by_tenant`:
  empty/homeless site refused, publish freezes pages+theme, then edit +
  retitle + add + **delete** + retheme and the published set doesn't move a
  byte — republish makes a NEW set while the old survives; outsider tenant
  cleanly denied on publish/unpublish/reads; same-tenant cross-site
  addressing reads empty; unpublish keeps history; site delete cascades
  publishes+snapshots through the pointer FK without error). Manual pass:
  `\d site_publishes` / `\d site_page_snapshots` in psql show PKs, indexes,
  cascade FKs and the pointer FK exactly as designed; real snapshot rows
  from the test run read back with frozen slug/home/nav values.
- **Cuts/flags:**
  - No publish-history list API — nothing consumes it yet; the index
    (`site_publishes_by_site`, newest first) and immutable rows are the S2
    rollback substrate, the accessor lands with a consumer.
  - Snapshot retention is unbounded by design (immutable history); revisit
    with quotas if it ever binds.
  - `unpublish_site` shipped (small, completes the state machine) though the
    queue text named only publish; publish UI copy is S1.15's.
  - CHANGELOG untouched: store flow only — the first user-visible surface
    lands with S1.09/S1.10.
- **Next:** S1.09 (`alo-sites` public service: Host resolution → published
  snapshots, cache, /healthz, Host-isolation tests).

## S1.09 — alo-sites public service: Host → published snapshots (2026-08-07)

- **Shipped:** the anonymous serving half of the two-service boundary.
  Store side: `platform/alo-store/src/site_public.rs` — `SitePublicStore`,
  a **separate read-only door** on a plain pool (deliberately not `Store`:
  no system ops, no blob backend, no way to open a tenant/account door);
  `resolve_published(subdomain)` is the one indexed read (sites ⋈
  site_publishes on the published-set pointer, backed by
  `sites_subdomain_unique`) → `PublishedSite` whose **tenant field is
  private**, and `published_pages(&PublishedSite)` scopes by that resolved
  pair — serving rows the Host lookup didn't lead to is unrepresentable.
  Service side: `products/sites/alo-sites/src/serve.rs` (+ `serve/{config,
  host,cache,rendered}.rs`) and the `alo-sites` binary (`src/main.rs`, runs
  **no migrations** — alo-jmap owns the schema). Host parsing reuses
  `validate_subdomain` (ports/FQDN-dot/case tolerated; apex, nested labels,
  IP literals, lookalike suffixes all fall through). Cache is publish-keyed:
  the resolver read runs per request (republish/unpublish visible on the
  next request, ever-stale impossible by construction), rendering happens
  once per publish (bounded map, 512 sites, arbitrary eviction). Response
  contract: strong `ETag "<publish>:<path>"` + `If-None-Match` → 304,
  `Cache-Control: public, max-age=60`, nosniff, trailing-slash tolerance;
  unknown/unpublished host → one byte-identical generic 404 (no existence
  leak); unknown path on a live site → a **themed** 404 (`render_not_found`
  in the render lib + 3 new `UiStrings` entries, en; fr/nl at S1.31);
  DB trouble → terse 503 + Retry-After, internals never on the wire;
  non-GET/HEAD → 405 + Allow. Env contract: `DATABASE_URL`, `SITES_DOMAIN`,
  `ALO_SITES_ADDR` (default 0.0.0.0:8081).
- **Verified:** `cargo fmt --check`; `SQLX_OFFLINE=true cargo clippy -p
  alo-sites -p alo-store --all-targets` zero warnings; `cargo test -p
  alo-sites` green (host-parsing unit tests + 4 in-process integration
  tests via tower::oneshot against the real router + compose Postgres:
  response contract incl. 304/405/css, **Host isolation** — A's host never
  serves B's markers/theme/pages, unknown ≡ unpublished byte-identical,
  republish flips on the next request while draft edits never leak);
  full `cargo test -p alo-store` green (207 unit + all suites; isolation
  24 with the new `public_resolver_scopes_by_subdomain_and_never_leaks_drafts`).
  Manual wire pass with the real binary on 127.0.0.1:8081 against docker
  `alo-pg`, real curl: healthz 200 `ok`; live host → 200 text/html,
  `cache-control: public, max-age=60`, `etag: "i48oI0wvzfP2L4JR0kLkaQ:/"`,
  nosniff, canonical `https://<sub>.sites.test/`; If-None-Match → 304
  size=0; `/assets/site.css` → text/css with the north preset tokens;
  iso-a host serves ALPHA-ONLY only, iso-b host BETA-ONLY only; unknown
  host → 404 `<title>Page not found</title>` (generic); live host unknown
  path → 404 `<title>Page not found — Alpha Site</title>` (themed);
  POST → 405; apex host → 404.
- **Cuts/flags:**
  - `/assets/img/<blob_id>` (the third public path) is NOT served yet —
    no published fixture carries images until logo/gallery upload lands;
    wiring it lands with S1.14 (Drive/blob refs). The renderer already
    emits those URLs; until S1.14 an image-bearing page would 404 its
    images. `/f/:form_id` is S1.16 by design.
  - No CSP header on served pages (inline behavior script + inline 404
    style make it low-value churn now); revisit at wave review.
  - Eviction is arbitrary-at-bound, not LRU — deliberate until real
    traffic exists; noted in `serve/cache.rs`.
  - 503 path exercised by code review only (would need killing Postgres
    mid-test; the mapping is 6 lines, all errors → one static body).
- **Human inbox (deploy, when the wave ships):** production needs the
  `alo-sites` container in compose (env above), the `SITES_DOMAIN`
  purchase, wildcard DNS + wildcard/on-demand TLS at Caddy routing
  `*.<SITES_DOMAIN>` → alo-sites. Deliberately not touched by the loop.
- **Next:** S1.10 (edit API in alo-jmap: `/sites/*` CRUD + section ops +
  publish, Problem errors, wire transcript).

## S1.10 — edit API in alo-jmap: `/sites/*` (2026-08-07)

- **Shipped:** `products/mail/alo-jmap/src/sites.rs` + registration in
  `server.rs`/`lib.rs` (additive lines) — the authenticated edit half of the
  two-service boundary. Sites: `GET/POST /sites`, `GET /sites/subdomain-check`
  (live taken/free for the create form), `GET/PUT/DELETE /sites/{id}` (PUT
  takes `{name?, subdomain?}`, empty PUT is a named 422), `PUT
  /sites/{id}/theme` (body = the theme envelope, through the store's theme
  gate), `POST /sites/{id}/publish` → `{publishId, status:"live"}`, `POST
  /sites/{id}/unpublish` (idempotent). Pages: `GET/POST /sites/{id}/pages`
  (list stays lean — no sections), `PUT /sites/{id}/pages/order` (full
  permutation), `GET/PUT/DELETE /sites/{id}/pages/{pid}` (PUT does partial
  title/slug/seoTitle/seoDescription; SEO merges over the two-field store
  setter — absent keeps, blank clears), `POST .../home`. Sections, addressed
  **by index** into the ordered envelope (no ids by design — the S1.27 AI ops
  speak the same vocabulary): `PUT .../sections` (atomic full set), `POST
  .../sections` `{section, index?}`, `PUT/DELETE .../sections/{index}`,
  `POST .../sections/{index}/move` `{to}` — read-modify-write through the
  schema write gate; every op answers the canonical stored envelope. Error
  contract per the design note: 401 unauthenticated (WWW-Authenticate:
  Bearer); anything not resolving in the caller's tenant → 404; **every**
  rule violation → 422 with the store's rule-naming message (the sites store
  spells them all as `Conflict`, so this module's map sends `Conflict` →
  422, not 409 — documented in the module doc); malformed JSON → 400 notJSON.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-jmap
  --all-targets` zero warnings; **full `cargo test -p alo-jmap` green** on
  the local docker Postgres, including the new `tests/sites_http.rs` (7
  tests through the real router + real DB: 401 across the route families,
  site lifecycle incl. delete-releases-subdomain, page lifecycle incl. SEO
  partial-merge/blank-clears and home promotion, section add/update/move/
  remove + full-set + out-of-range and malformed index, theme gate + publish
  preconditions + live/unpublish flow, and the mandatory wrong-tenant
  barrage — 18 verbs against tenant B's ids all answer A with 404 and leak
  nothing, B's data untouched after). Manual wire pass against the real
  debug binary on 127.0.0.1:8080 + docker `alo-pg`, real curl: no token →
  401 `{"detail":"missing or invalid bearer token"}` + `www-authenticate:
  Bearer`; create happy → full site JSON (draft, `{}` theme); `UPPER` → 422
  "subdomain may only contain lowercase letters, digits, and hyphens";
  `mail` → 422 "subdomain is reserved"; re-claim → 422 "subdomain is already
  taken"; publish-no-pages → 422 "site has no pages to publish"; slug
  `blog` → 422 "slug is reserved"; add hero/cta → canonical envelopes;
  `carousel` → 422 naming the 12 known types; `javascript:` href → 422
  naming the href rule; move/remove reshuffle correctly; index 5 → 422 "no
  section at index 5 (the page has 2)"; bad preset → 422; terra lands;
  publish → `{"publishId":"S9iFT9_n69lckiOdzNTrFw","status":"live"}`; GET
  site shows `publish{id, publishedAt}` + status live; unpublish → draft;
  `{not json` → 400 notJSON. psql after: the sites row (terra, pointer
  cleared after unpublish), 2 page rows (home flag, nav order, 1 section
  stored canonically), publish row with 2 frozen snapshots.
- **Cuts/flags:**
  - No optimistic concurrency on section read-modify-write (single-editor
    assumption; an If-Match seam is S2) — documented in the module doc.
  - No draft-preview endpoint (S1.13 by queue design), no theme-preset
    listing route (lands with the S1.14 theme UI), no publish-history list
    (S2 rollback substrate stays store-only).
  - `GET /sites/{id}` additionally returns the current publish
    (`publish: null | {id, publishedAt}`) — small addition beyond the queue
    text, the S1.15 status chip needs it.
  - **Caddyfile note:** `/sites` is a NEW top-level route prefix — production
    Caddy needs it routed to alo-jmap at the next deploy (same as `/billing`).
  - `cargo fmt` wanted to reflow 7 pre-existing alo-jmap modules this item
    never touched (agent/base/drive/spaces/tasks/wopi/workspace_search —
    import-order + wrapping, likely a rustfmt style-edition delta with the
    other machine). Reverted deliberately: formatting churn on the business
    track's active files invites rebase conflicts; my own files are
    fmt-clean. Flagged for a human to align rustfmt versions at wave review.
  - CHANGELOG: user-voice entry added (first user-visible sites surface).
- **Next:** S1.11 (web module skeleton: rail entry, site list + create,
  page list; i18n en).

## S1.11 — web module skeleton `web/src/sites` (2026-08-07)

- **Shipped:** the Sites web module, cut from the Tasks/Billing module cloth.
  Rail entry **"Websites"** (Globe) registered in the workplace product
  surface only (ADR 0036 — the suite sells it, the standalone mail app never
  sees it). `web/src/sites/`: `api.ts` — its own thin REST client over the
  wire-verified `/sites/*` surface (S1.11 slice only: list/create site,
  get site, subdomain check, list/create page; methods land with their
  consumers), `SitesError` carrying the server's rule-naming detail,
  `sitesMessage` fallback helper; `types.ts` — wire types trimmed to what the
  screens render; `SitesListView` — site table (name → opens the site,
  address mono, live/draft chip), empty state, load-failure banner;
  `NewSiteDialog` — name + address with a **live taken/free check**
  (350 ms debounce, sequence-guarded against out-of-order answers,
  advisory only: submit always allowed, the server's 422 shown verbatim);
  `SiteView` — site header (back link, name, address, status chip) + page
  list (title, home badge, /path) at `/sites/:siteId`, stale/foreign id
  reads as not-found with the way back; `NewPageDialog` — title/path/home,
  the home flag **defaulting on for the first page**; `parts.tsx` — the
  module's own dialog chrome/empty/error/field pieces (deliberately NOT
  imported from billing: cross-track coupling; promoting this chrome into
  `ds` once three modules carry it is a wave-review candidate); module CSS;
  ~40 new `i18n/en.ts` strings (additive block at the catalog's end).
- **Verified:** `npx tsc --noEmit` clean; `npx eslint` on all changed files
  zero warnings; **full `npm test` green — 178 tests / 25 files**, incl. 10
  new module tests driving the REAL client + views over a recording fake
  fetch (list renders the API's answer with status chips; empty state;
  load-failure banner; live check hits `/sites/subdomain-check` and shows
  free/taken/the server's 422 rule sentence; create POSTs exactly
  `{name, subdomain}` and navigates into the new site; a 422 refusal shows
  in the dialog which stays open; page list with home badge; 404 → not-found
  + back link; create-page POSTs `{title, slug, home}` with home defaulted
  on the first page and the list reloads); `npm run build` clean. No new
  HTTP routes and no storage touched → the wire/wrong-tenant gates are the
  server's (proven in S1.10); the component tests pin the exact request
  shapes this client sends to that verified surface.
- **Cuts/flags:**
  - The page-create dialog is a small deliberate addition beyond the queue's
    "page list": no other item owns page-create UI (S1.12 owns sections),
    and a page list with no way to create a page is dead UI. Recorded here.
  - No site rename/delete/publish UI (S1.15), no section editor (S1.12), no
    theme UI (S1.14), no list search (lists are short), fr/nl at S1.31.
  - The create form shows the bare label only; the "goes live at
    `<sub>.<domain>`" copy is S1.15's, when the web learns the sites domain.
  - Rejected alternative (recorded in `parts.tsx`): importing billing's
    dialog chrome — same look, but couples the two tracks' modules; own
    copies now, promotion to `ds` at wave review.
  - `npm ci` had to be run on this machine (fresh checkout, no
    node_modules) — environment note only.
- **Next:** S1.12 (web editor core: section stack + per-type prop forms +
  save).

## S1.12 — web editor core: section stack + per-type prop forms (2026-08-07)

- **Shipped:** the visual page editor at `/sites/:siteId/pages/:pageId`
  (page titles in the site view now link into it). `PageEditorView` — the
  section stack as cards (type name + a summary line: the section's own
  heading, or a count of its entries), native drag-reorder with arrow-button
  fallback, edit, and two-click-confirm delete; `SectionPicker` — the add
  dialog: twelve tiles, each with an inline-SVG schematic thumbnail, the
  type's name and a one-liner; `SectionForm` — the per-type prop forms over
  shared primitives (text/long-text/link/image fields and a generic
  repeating-entries editor for list props; pricing bullets edit as
  one-per-line text). **Design:** every gesture is one call to the
  wire-verified S1.10 section ops (add is form-first → POST, edit → PUT
  index, reorder → move, delete → DELETE) and the stack always renders the
  canonical envelope the server answers — no local dirty buffer to lose, and
  a 422 points at the exact gesture that broke the rule (rejected
  alternative, recorded in the view doc: a dirty buffer with one atomic
  save — fewer requests, but lost-work risk and ambiguous refusals). The
  client re-states NO validation: refusal sentences are the store's,
  verbatim, and the forms drop untouched-blank optionals/rows to absent keys
  on save. Supporting files: `sections.ts` (the TS mirror of schema v1),
  `sectionDrafts.ts` (the editable spelling + toDraft/toSection — props the
  forms don't offer, a feature's `icon` and a contact form's `form_id`, ride
  through edits untouched), `sectionInfo.ts` (labels/descriptions/
  summaries); ~90 new `i18n/en.ts` strings (additive block).
- **Verified:** `npx tsc --noEmit` clean; `npx eslint` on all changed files
  zero warnings; **full `npm test` green — 187 tests / 26 files**, incl. 9
  new editor tests over the recording fake fetch driving the REAL client and
  views: stack renders stored sections in order; picker offers all twelve;
  add POSTs exactly the typed section (trimmed, `toEqual`-pinned so blank
  optionals are ABSENT); a list section sends every added entry; edit opens
  prefilled and PUTs to the index with the untouched subheading riding
  along; `form_id` survives an edit; move-down POSTs `{to:1}`; delete needs
  the second click and one click alone writes nothing; a 422 shows the
  server's sentence in the open dialog; a stale page id reads as the error
  with the way back. `npm run build` clean. No new routes and no storage
  touched → the wire/wrong-tenant gates are the server's (proven in S1.10);
  the tests pin the exact request shapes sent to that verified surface.
- **Cuts/flags:**
  - Image props are blob-id + alt-text inputs until the S1.14 Drive picker
    (the field's hint says so); a pasted Drive file id works today.
  - `contact_form.form_id` (S1.16) and the feature `icon` token (renderer
    ships no icons — S1.06 flag) are preserved-not-offered by the forms.
  - Repeating entries have add/remove but no inner reorder — the queue's
    reorder ask is the section stack; revisit if users hit it.
  - Drag-reorder is native HTML5 DnD, which jsdom cannot drive — the tests
    exercise the arrow buttons, which share the same `move()` path.
  - No optimistic concurrency on the read-modify-write ops (S1.10's
    single-editor flag stands; If-Match is the recorded S2 seam).
  - The repo's `exactOptionalPropertyTypes` means optional wire props are
    spelled `?: X | undefined` so computed blanks can be built and then
    dropped by `JSON.stringify` — noted in `sections.ts`.
- **Next:** S1.13 (live preview: authenticated draft-render endpoint in
  alo-jmap + iframe preview pane).

## S1.13 — live preview: draft-render endpoint + preview pane (2026-08-07)

- **Shipped:** the editor now shows the page while it is built. Render lib:
  one private `render_document` behind both spellings — `render_page`
  (public serving, `<link>` to `/assets/site.css`) and the new
  `render_page_preview` (the same document with the generated stylesheet
  inlined in a `<style>` block, because the public asset paths do not
  resolve on the edit origin); the stylesheet self-containment test now also
  forbids `</` so embedding the sheet verbatim can never close the block.
  Edit API: `GET /sites/{id}/pages/{pid}/preview` — authenticated like every
  edit route, renders the DRAFT with `SiteTheme::from_stored` + the site's
  future public origin (`https://<sub>.<SITES_DOMAIN>`, env with the
  alosites.com default) in canonical/OG, answers `text/html` with
  `Cache-Control: no-store` (a draft has no cache life). Web: the editor is
  now a two-pane layout — section stack left, preview right (sticky,
  stacking on narrow screens); the pane holds a sandboxed iframe
  (`sandbox="allow-scripts"`, document via `srcdoc` — it may run its own
  menu script but never touches the app origin), refetches whenever a save
  lands (keyed on the envelope the last op answered, so a refused gesture
  does not refresh), and a desktop/phone toggle lays the document out at
  375px. `SitesApi.pagePreview` answers text; the non-2xx→`SitesError` map
  is now shared (`#rejectFailed`) between the JSON and text paths. 5 new
  i18n en strings.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-sites
  -p alo-jmap --all-targets` zero warnings; `cargo test -p alo-sites` green
  (28 — incl. the new no-drift pin: for EVERY shipped preset, preview ==
  published document with exactly the stylesheet reference swapped,
  byte-for-byte) and full `cargo test -p alo-jmap` green (30 suites; sites_http
  now 8 — the new preview test: self-contained document, no-store, follows an
  edit on the next fetch; the 401 barrage and the wrong-tenant barrage both
  extended with the preview route). Web: `npx tsc --noEmit`, eslint clean,
  **full `npm test` green — 202 tests / 27 files** incl. 4 new preview-pane
  tests (server document reaches the sandboxed frame's srcdoc; a successful
  save refetches while a refused op provably does not; width toggle flips
  aria-pressed; a failed preview shows its own error and never blocks
  editing); `npm run build` clean. Manual wire pass against the rebuilt
  debug binary + docker `alo-pg`, real curl: hero with `cta` → 422 naming
  the real props (gate live on the wire), `primary_cta` → 200; preview →
  200 `text/html; charset=utf-8`, `cache-control: no-store`, `<style>`
  inlined, zero `/assets/site.css`, `Bread &amp; butter` escaped, canonical
  `https://wire-preview-s113.alosites.com/`; no token → 401; bogus page id →
  404; PUT the hero then re-fetch → the new heading, the old one gone;
  fixture site deleted after.
- **Cuts/flags:**
  - This iteration ADOPTED uncommitted S1.13 work found in the tree (the
    previous invocation died before finishing): the Rust endpoint, render
    split, and preview pane were in place but unfinished — the 5
    `sitesPreview*` i18n strings, all web tests, CHANGELOG, and
    QUEUE/STATE were missing. Everything was read line-by-line, finished,
    and re-verified through the full gates before committing as one item.
  - The preview refetches the whole document per save (no diffing/morphing)
    — right-sized until pages get heavy; revisit only on real complaints.
  - `sites_domain()` reads env once per process (OnceLock); a changed
    `SITES_DOMAIN` needs a restart — same posture as alo-sites' own config.
  - The rustfmt-version delta from S1.10 struck again (fmt wanted to reflow
    7 untouched business-track modules incl. `agent.rs`, active on the other
    machine); reverted the churn again — the S1.10 flag for aligning rustfmt
    versions at wave review stands.
  - Environment note: the first `cargo test -p alo-jmap` build OOM-killed
    parallel rustc (missing-rlib errors); `-j 2` builds clean — same as the
    S1.03 note.
- **Next:** S1.14 (theme UI: preset picker + logo/favicon upload via Drive;
  preview updates).

## S1.14 — theme UI + the whole image path (2026-08-07)

- **Shipped:** the theme becomes a thing users touch, and images become real
  end-to-end. **Web:** `ThemeDialog` (preset cards rendered from the server's
  tokens — swatches + the preset's own heading font — plus logo/favicon
  upload/replace/remove rows), opened from BOTH the site view and the page
  editor; applying PUTs the full envelope through the wire-verified theme
  gate and the editor preview refetches (new `previewEpoch` — the preview
  depends on the theme, not only the sections). Uploads go through Drive:
  new additive `JmapClient.driveUploadBlob` (one upload, registered as a
  drive file, returns the blob id) — rejected alternative: the bare
  `uploadFile`, whose contract says unreferenced blobs may be GC'd; a
  logo must outlive any future GC and stay user-visible. Section image
  fields got the picker S1.12's hint promised: an Upload button beside the
  id input (same Drive path). **Edit API:** `GET /sites/theme-presets`
  (authed; ids/names/palette/typography, camelCase) for the picker;
  the draft preview now inlines theme + section images as `data:` URIs
  (public paths don't resolve on the edit origin; > 4 MiB or non-image →
  public-path fallback). **Render lib:** `SiteRenderContext.images:
  ImageSources` (`PublicPaths` | `Inline(map)` with per-id fallback;
  `og:image` deliberately always the absolute public URL) — rejected
  alternative: post-processing the rendered HTML string. **Public service
  (closes the S1.09 flag):** `/assets/img/<blob_id>` now serves — membership
  gate first (`RenderedSite.serves_image`: the set collected from the
  publish's frozen theme + lenient sections, so servable ≡ shown), then the
  tenant-scoped read `SitePublicStore::published_image` (blob row by the
  resolved site's private tenant, bytes from the new blob backend handle);
  image content types only (allowlist in the new
  `alo-store/src/site_assets.rs`, shared with the preview), `ETag
  "img:<id>"` + 304, `public, max-age=3600`, nosniff, and
  `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'`
  so an SVG opened as a document can't run script on the site origin.
  `alo-sites` env gains **required `ALO_BLOB_DIR`**. Store:
  `AccountStore::site_image` (tenant-scoped, image-typed blob read for the
  preview) and `Section::image_blob_ids()` (exhaustive match — a new
  variant fails to compile until it declares its images).
- **Verified:** fmt; `SQLX_OFFLINE=true cargo clippy -p alo-store -p
  alo-sites -p alo-jmap --all-targets` zero warnings; **full test suites
  green on docker Postgres**: alo-store 318 unit + all suites (isolation
  gains `site_images_scope_by_tenant_and_refuse_non_images`: own image
  serves, own non-image reads as absent, **outsider tenant reading the blob
  id gets clean None**, and `published_image` through B's resolved site
  never reaches A's bytes), alo-sites 30 (serve_http gains
  `serves_exactly_the_published_images`: unreferenced same-tenant blob /
  foreign blob / referenced-but-HTML blob / garbage id all themed-404,
  referenced logo serves with the full header contract + 304; render_rules
  gains the Inline-sources pin incl. og:image exemption; the preset no-drift
  pin still holds byte-for-byte), alo-jmap 284 unit + all suites (sites_http
  now 10: presets shape + 401, preview inlines the logo as base64 while the
  non-image falls back to the public path). Web: tsc, eslint, `npm run
  build` clean; **208 tests / 28 files** incl. 6 new (presets render +
  exact envelope PUT with absent keys, upload feeds blob id in, stored logo
  prefills + remove drops the key, 422 sentence stays in the open dialog,
  theme apply refetches the preview, hero image upload fills the id the
  section then saves). Manual wire pass, real curl against both debug
  binaries + docker `alo-pg`: presets 401 then 200 (7 presets, north
  first); real 70-byte PNG through `/jmap/upload` + `/drive/files`;
  `vaporwave` → 422 naming the preset rule, `not/a/token` logo → 422,
  terra+logo → ok; preview `no-store` with `data:image/png;base64,iVBOR…`
  inline, zero `/assets/img/` for the logo, terra tokens in the inlined
  style; publish → the served home references `/assets/img/<id>`, the image
  answers 200 `image/png` + etag + CSP with **bytes identical to the
  uploaded PNG** (cmp), If-None-Match → 304 size 0, garbage id → themed 404
  (`Page not found — Wire Theme Co`), apex host → 404; fixture site deleted,
  host off the air.
- **Cuts/flags:**
  - Responsive image derivatives/resizing stay S2 (design-note out-of-scope);
    the service serves original bytes, one blob read per request (no image
    byte-cache — revisit with real traffic, noted in `serve.rs`).
  - Theme v1 stays presets-only; free-form colors remain rejected (contrast
    guarantee).
  - The dialog shows uploaded-state text, not a thumbnail — the preview pane
    IS the thumbnail (it renders the real logo); revisit only on complaints.
  - `SitePublicStore` constructors now take the blob backend (breaking only
    in-repo callers; no public wire contract touched). The public door doc
    was updated: it now holds a blob handle, reachable only through
    `published_image`.
  - **Deploy note (human inbox):** the production `alo-sites` container will
    need `ALO_BLOB_DIR` mounted to the same blob directory `alo-jmap` writes
    (read-only is fine), alongside the S1.09 compose/Caddy/DNS items.
  - Blob GC does not exist today; when one lands it MUST treat site
    references (theme logo/favicon, section images, later post covers) as
    live references — flagged here so the future implementer inherits the
    knowledge.
  - The rustfmt-version delta struck again (fmt reflowed 7 untouched
    business-track modules); reverted the churn again — the S1.10 flag about
    aligning rustfmt versions at wave review stands.
- **Next:** S1.15 (publish UI: publish button + "goes live at" copy +
  status chips; STATE human-inbox note for production serving).

## S1.15 — publish UI + the web learns the sites domain (2026-08-07)

- **Shipped:** publishing becomes a button. **Edit API:** one new route,
  `GET /sites/config` → `{"domain": <SITES_DOMAIN>}` — the deployment-wide
  apex the web composes "goes live at" copy and live links from, instead of
  hardcoding a domain (the missing piece S1.11 recorded; publish/unpublish
  routes existed since S1.10). Authenticated like every `/sites/*` route.
  Rejected alternative (recorded in the module doc + this entry): a per-site
  `url` field in site JSON — duplicates a derivable composition and leaves
  the create form (no site yet) without a domain source; one config route
  serves both consumers. **Web:** the site view gains a publish bar — draft:
  **Publish** + "Publishing puts this site live at `<sub>.<domain>`."; live:
  "Your site is live at" + the address as a real `https://` link
  (new-tab, noreferrer) + **Publish changes** + **Take offline** with the
  module's two-click-confirm pattern (first click arms and turns the button
  red, second acts). A refused publish shows the store's 422 sentence
  verbatim inline (`role="alert"`), never swallowed; a failed config fetch
  degrades — the address copy stays off, publishing still works. The
  new-site dialog now previews "Your site will live at `<sub>.<domain>`."
  under the live check (the copy S1.11 deferred here). Live/draft chips were
  already shipped in S1.11 and are unchanged; they now flip through the
  publish flow on this screen. `SitesApi` gains `config` / `publishSite` /
  `unpublishSite`; 9 new `i18n/en.ts` strings (additive block).
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-jmap
  --all-targets` zero warnings; **full `cargo test -p alo-jmap` green** on
  docker Postgres (298 unit + all suites; sites_http now 11 — the 401
  barrage extended with `/sites/config`, plus `config_names_the_sites_domain`).
  Web: `npx tsc --noEmit` clean, eslint zero warnings on all changed files,
  **full `npm test` green — 231 tests / 29 files** incl. 4 new (draft shows
  the composed goes-live copy and Publish POSTs `/publish` then renders the
  live chip + the address as a link with the exact href; a 422 refusal shows
  the server's sentence and stays draft; Take offline writes nothing on the
  first click and POSTs `/unpublish` on the confirm click; the create form
  previews the lowercased typed label against the fetched domain);
  `npm run build` clean. Manual wire pass, real curl against the rebuilt
  debug binary + docker `alo-pg`: `/sites/config` no token → 401 +
  `www-authenticate: Bearer`; with token → 200 `{"domain":"alosites.com"}`;
  fresh site → publish-no-pages → 422 "site has no pages to publish" (the
  sentence the UI shows verbatim); home page + publish →
  `{"publishId":…,"status":"live"}`; GET site → status live +
  `publish{id, publishedAt}`; unpublish → draft + `publish: null`; fixture
  site deleted after.
- **Cuts/flags:**
  - No publish-history UI and no published-at display — the chip + link are
    the S1.15 ask; history stays the S2 rollback substrate.
  - The sites list keeps showing the bare subdomain (no config fetch there);
    the full address lives where publishing happens. Revisit on complaints.
  - `/sites/config` currently carries one key; later deployment facts the
    web needs (e.g. AI availability for S1.28) can ride the same route
    additively.
- **Human inbox (unchanged, re-confirmed for this item):** production
  serving still needs the alo-sites container in compose (+`ALO_BLOB_DIR`
  mount), Caddy wildcard/on-demand-TLS routing `*.alosites.com` → alo-sites,
  and `SITES_DOMAIN=alosites.com` in both services' env — the domain itself
  is purchased and its DNS is live (see inbox top). Until that deploy, a
  "live" site is only reachable in local dev; the UI truthfully says where
  it WILL serve.
- **Next:** S1.16 (forms backend: public POST `/f/:form_id` on alo-sites,
  submissions store, internal-delivery notification, rate limit + tests).

## RESOLVED (was a halt): concurrent editor detected on this checkout (2026-08-07)

- **HALTED (resolved 13:45): a second agent/loop iteration was editing this working tree concurrently; work could not be trusted or committed (CLAUDE.md: one agent per working tree).**
- **Resolution (human, 2026-08-07 13:45):** root cause was a supervisor session restart that orphaned the running worker; the relaunched wrapper started a rival in the same tree. The halting worker's uncommitted S1.16 state was discarded per its own verdict; both workers confirmed exited; single-wrapper supervision relaunched. S1.16 will be redone fresh.
- What happened, in order, while this iteration was mid-way through S1.16
  (forms backend — migration, `site_forms` store module, public POST
  `/f/:form_id`, notification sweep, all three crates compiling and
  `cargo test -p alo-store` green):
  1. The reflog shows a `git pull --rebase -q origin main` this iteration
     did not run (`HEAD@{0}`, fast-forward a1d4666 → 586165e) — the `-q`
     pull is the loop protocol's own step 1, i.e. a second iteration
     started while this one was still running.
  2. Every tracked-file modification of this iteration was then discarded
     (a `git checkout -- .`-style wipe; leaves no reflog trace). Untracked
     files survived.
  3. Minutes later the same S1.16 surfaces reappeared in the tree —
     `site_forms` module registration, an `rfc2047::encode`, a
     `PublishedSite::tenant()` accessor — with the same shapes but
     *reworded documentation*: an independent implementation by another
     model run of the same queue item, written file-by-file (mtimes
     seconds-to-minutes old, one file 57 s before probing).
- No sync-folder markers found on `C:\dev\Ficina-loop` (no OneDrive/
  Dropbox attributes); the double-fired wrapper is the likelier cause, but
  a cross-machine sync of this folder would look identical and should be
  ruled out by the human.
- Actions taken: **no code committed** — the working tree holds
  ambiguously-authored, uncommitted work (four modified alo-store files +
  four untracked new files) and is left untouched for human review. Only
  this HALT note is committed, so both wrappers stop at their next
  iteration.
- To resume after fixing the environment: discard or salvage the working
  tree explicitly, ensure exactly ONE loop wrapper runs for the sites
  track on exactly one machine, then remove/date this HALT and restart.
  S1.16 remains the next queue item; this iteration's design decisions are
  reproducible (public write-set = submission insert only; notification
  delivered by an alo-jmap background sweep through the account door;
  forms auto-provisioned on section save; POST gated to the published
  reference set like images).

## HALT(resolved): concurrent editor detected AGAIN on this checkout (2026-08-07 ~15:56)

- **The same failure mode as the 13:45 incident recurred one iteration
  later.** This iteration started with a clean working tree (per the
  invocation's git-status snapshot), pulled (already up to date), and spent
  ~15 minutes READING code for S1.16 — it wrote nothing. At 15:55 a probe
  found 8 modified + 2 untracked files in the tree: the S1.16 surfaces
  (`0058_site_forms.sql`, `site_forms.rs`, `SiteFormId`/lib registration,
  `site_public.rs`, alo-sites render/strings/rendered, both Cargo.tomls),
  written file-by-file with mtimes 15:50:59 → 15:55:50 — the newest EIGHT
  SECONDS before the probe, i.e. a second agent was actively implementing
  S1.16 in this same checkout while this iteration was reading it.
- Note for the human: this iteration's earlier reads of `id.rs` already
  showed the rival's uncommitted `SiteFormId`/`SiteFormSubmissionId`
  additions — mid-flight rival edits are indistinguishable from committed
  code to a reader, which is exactly why the one-agent-per-tree rule is
  absolute and why no code from this tree can be trusted or committed now.
- Actions taken: **no code written, none committed** — only this HALT note
  is committed so both wrappers stop at their next check. The rival's
  uncommitted, ambiguously-authored work is left untouched in the tree for
  human review (same posture as the 13:45 incident).
- The 13:45 resolution ("both workers confirmed exited; single-wrapper
  supervision relaunched") evidently did not hold: either the supervisor
  relaunch double-fired again, or a second machine/session is running the
  sites track. Before restarting, please (1) enumerate ALL running
  wrappers/agent sessions for this track on every machine, (2) kill all
  but one, and (3) only then discard-or-salvage the tree and remove/date
  this HALT. S1.16 remains the next queue item.

### Resolution of the second concurrent-editor halt (human, 2026-08-07 16:05)

Root cause finally proven: this afternoon's "silently failed" PowerShell
wrapper launches had actually SPAWNED DETACHED WORKERS whose transcripts were
not where the supervisor looked — they survived every wrapper kill as ghosts
and kept editing this tree, which is what every honest worker since 13:19 was
colliding with. Both ghosts were found by command-line process hunt
(sh.exe 36320 + claude.exe 1736), killed with their trees, and the checkout
then proved quiet for 60 seconds. Ghost edits discarded. Single bash wrapper
relaunched. S1.16 (attempt #6) is next — with, at last, an actually empty
stage.

## HALT(resolved): concurrent editor detected a THIRD time — TWO live wrappers found (2026-08-07 18:20)

- **The cause is identified this time, with PIDs.** A process hunt run the
  moment the collision was detected shows **two `run-loop.sh
  /c/dev/Ficina-loop sites` wrappers alive simultaneously**:
  - wrapper PID **17324**, started **16:01:34** (presumably the "single bash
    wrapper relaunched" of the 16:05 resolution),
  - wrapper PID **17836**, started **18:16:25** (a second launch ~2h15m
    later).
  Each fired an iteration within the same ten seconds: headless worker
  claude.exe **49380** (via sh.exe 6344) started **18:16:27**, and headless
  worker claude.exe **18744** (via sh.exe 39964) started **18:16:37**. This
  iteration is worker 18744 (its shell chain executed this note's probes);
  the rival is worker 49380. Both are honest LOOP workers on the same
  S1.16a item — the bug is the double-fired wrapper, not either worker.
- Collision timeline of this iteration (started 18:16:37, clean tree, pull
  up-to-date): it read patterns for S1.16a and wrote exactly ONE file, the
  untracked migration `0120_site_forms.sql` (18:1x). The rival's writes then
  appeared mid-flight: `id.rs` modified 18:19:05, `site_forms.rs` (15,578
  bytes, untracked) 18:20:03, `lib.rs` 18:20:20 — detected at 18:20:36,
  sixteen seconds after the last rival write, when `lib.rs` changed under
  this worker declaring a `site_forms` module it had not written (exporting
  a `normalize_submission` API not of this worker's design).
- Authorship map for the human, explicit this time:
  - **This worker's file (discardable or salvageable):**
    `platform/alo-store/migrations/0120_site_forms.sql` — numbered after the
    current tail (0119_insight_dashboards.sql).
  - **The rival's uncommitted files (left untouched):** modified `id.rs` +
    `lib.rs`, untracked `src/site_forms.rs` and
    `migrations/0058_site_forms.sql` — note the rival numbered its migration
    0058, the same number the 15:56 ghost used, not 0120.
- Actions taken: **no further code written, none committed** — only this
  HALT note is committed so both wrappers stop at their next check. The
  rival worker 49380 may still push its own S1.16a before dying; if it
  completes the full gates honestly, that work is its own single-authored
  commit and can stand on its merits — this note does not condemn it, only
  the double supervision.
- To resume: (1) kill BOTH wrappers 17324 and 17836 and any surviving
  workers (49380, 18744), (2) verify the tree is quiet for 60 s, (3)
  discard or salvage the uncommitted files per the authorship map above,
  (4) relaunch exactly ONE wrapper — and consider making `run-loop.sh`
  refuse to start when another instance holds a lockfile, so a fourth
  incident is impossible rather than unlikely, (5) remove/date this HALT.
  S1.16a remains the next queue item unless the rival landed it.

### Resolution of the third concurrent-editor halt (human, 2026-08-07 18:40)

Same species, final specimen: STOPPED WRAPPERS survive as detached processes
too (the earlier hunts only matched workers). A machine-wide sweep matching
both wrapper and worker command lines found THREE live wrappers; all killed,
zero survivors, tree clean. Permanent fix shipped in run-loop.sh: a per-track
machine lock — a second wrapper now refuses to start while a live owner
exists, and stale locks from dead PIDs are taken over. One wrapper relaunched
under the lock. Next: S1.16a (the freshly split, single-turn store slice).

## S1.16a — forms store (banked on the seventh attempt, first with an empty stage)

- **Shipped:** migration `0120_site_forms.sql` (`site_forms` +
  `site_form_submissions`, tenant-scoped, cascading tenants → sites → forms →
  submissions, a global unique index on the bare form id for the later public
  resolve, and deliberately NO ip/ua columns per the privacy model);
  `SiteFormId`/`SiteFormSubmissionId` newtypes; `site_forms.rs` store module —
  form create/list/get/rename/delete (per-site cap 50) and submission
  add/list/mark-handled/delete, every statement scoped by (tenant, site) with
  the same patterns as `site_pages.rs`; `normalize_submission` is the public
  write gate (trim, non-blank, caps 200/254/10k, loose one-@ email) so the
  S1.16b endpoint validates identically. NO HTTP, as split.
- **Verified:** `cargo fmt`; clippy `--all-targets` zero warnings;
  `cargo test -p alo-store` — 43 binaries green against the compose Postgres,
  including the new `site_forms_and_submissions_scope_by_tenant_and_site`
  (wrong-tenant denial on every path, cross-site denial within a tenant,
  write-gate rejections, newest-first order, handled toggle, cascades) plus
  4 unit tests on the validation gates.
- **Environment flags for the human:**
  1. The machine hit Windows commit-charge exhaustion mid-gate (os error 1455,
     "paging file too small") — parallel rustc jobs failed to mmap and
     masqueraded as corrupted-artifact ICEs. Worked around with `-j 2` after a
     `target/debug` wipe; consider a bigger page file or capping the wrapper's
     build parallelism.
  2. The local dev DB still carried a ghost's applied version-120 "site forms"
     migration (different SQL, checksum mismatch → `VersionMismatch(120)`).
     Dropped the ghost's two tables and its `_sqlx_migrations` row, then this
     item's migration applied cleanly. Dev-DB-only surgery; no code impact.
- **Cuts:** none — the split item shipped whole.
- **Next:** S1.16b (public `POST /f/:form_id` on alo-sites).

## S1.16b — public form submit endpoint (salvaged + wired + gated)

- **Provenance, stated plainly:** this iteration found four uncommitted,
  untracked files exactly matching S1.16b's scope (`site_public_forms.rs`,
  `serve/forms.rs`, `serve/rate.rs`, `tests/form_submit.rs`), last written
  ~11 minutes before this worker started — a prior iteration of this same
  wrapper that died mid-item without committing (nothing was wired into
  either crate's module tree). The tree was verified quiet, the files were
  read line-by-line, judged correct and house-style, and SALVAGED; this
  worker wrote all the missing wiring and ran every gate itself, so the
  result is fully reviewed + verified regardless of which turn typed what.
- **Shipped:** `POST /f/{form_id}` on alo-sites. Store side:
  `site_public_forms.rs` — the public door's ONE write; a single conditional
  INSERT..SELECT resolves the bare form id to its owning tenant and writes
  only if the site is live (`published_publish_id IS NOT NULL`), so a
  cross-tenant write is unrepresentable and unknown/deleted/draft ids are one
  indistinguishable `Ok(None)`; fields pass the same `normalize_submission`
  gate as the authenticated door. Service side: `serve/forms.rs` (handler:
  rate limit → parse → honeypot `website` field silently drops as a fake 200
  → store write; 400 with the store's field-level reason, 413 via a
  256 KiB `DefaultBodyLimit` on the route, 404 = the generic unknown-host
  page, 429 with `Retry-After`); `serve/rate.rs` (in-memory sliding window,
  10/10 min per client key, 4096-key cap, key = last XFF hop or peer IP —
  transient only, never stored/logged). Wiring this turn: `pool()` accessor
  on `SitePublicStore`, `site_public_forms` module line, six `UiStrings`
  form-result strings (i18n path), `minimal_document` shared with the
  unknown-host 404, `AppState.rate` + route registration, `main.rs`
  ConnectInfo for the no-proxy fallback key, `serde` dep on alo-sites.
- **Verified:** `cargo fmt`; clippy `--all-targets` on both crates zero
  warnings; `cargo test -p alo-store -p alo-sites` — 52 binaries, zero
  failures, incl. the 6 new in-process integration tests: submission lands
  in the owning tenant only (outsider sees nothing), unknown + draft-site
  form ids are one clean 404 that writes nothing, honeypot 200-drops,
  malformed/invalid bodies 400 naming the field, oversized body 413,
  11th submission in the window 429 with Retry-After while another client
  still lands. All against the compose Postgres (port 5432 on this machine;
  the test default remains 5433, overridden via DATABASE_URL).
- **Cuts:** none. NO notification yet, as split — that is S1.16c.
- **Next:** S1.16c (internal inbox delivery + auto-create form on section add).

## S1.16c1 — submission → owner-inbox notification (salvaged + gated, 2026-08-08)

- **Provenance, stated plainly:** this iteration found uncommitted work
  exactly matching S1.16c1's scope (new `site_form_notify.rs` +
  `0127_site_form_notification.sql` in alo-store, new `site_notify.rs` +
  `tests/site_notify.rs` in alo-jmap, plus six modified wiring files),
  written 07:51–11:20, last touch ~12 minutes before this worker started —
  a prior iteration of this same wrapper that died before the bookkeeping.
  Process hunt found exactly ONE wrapper (PID 4312, under the lockfile) and
  no rival worker; the tree stayed byte-quiet across two probes ~20 minutes
  apart. Every file was read line-by-line, every referenced API verified to
  exist, judged correct and house-style, and SALVAGED; this worker ran every
  gate itself (S1.13/S1.16b posture).
- **Shipped:** the notification half of the form flow. Store: migration
  `0127` adds `site_form_submissions.notified_at` (pre-existing rows marked
  already-notified so a deploy never floods owners; partial index on the
  NULL set) and `site_form_notify.rs` — `Store::claim_form_notifications`,
  a system-level claim (the `sweep_snoozes` posture) that marks rows
  notified in the same `UPDATE … FOR UPDATE SKIP LOCKED` statement that
  reads them (**at-most-once**: a crash between claim and delivery loses a
  notification, never duplicates one — the submission row stays in the
  owner's list either way), resolving site name/subdomain/owner + form name
  in the claim. alo-jmap: `site_notify.rs` — `run_due` drains in batches of
  100, builds one RFC 5322 message per claim and delivers it **internally**
  through the owner's account door (never outbound SMTP): From is a display
  identity `no-reply@<sub>.<domain>`, Reply-To the visitor, Subject through
  RFC 2047, free text base64-encoded so a submission can never inject
  headers or structure; a 30 s background sweep in `main.rs` (snooze-sweeper
  posture). Enabling wiring: `mime::format_addr` and `sites::sites_domain`
  now `pub(crate)`; tests gain `harness_on` (a second tenant on the SAME
  store handle, the way production runs).
- **Verified:** this item's files rustfmt-clean (`--check`, skip_children);
  `SQLX_OFFLINE=true cargo clippy -p alo-store -p alo-jmap --all-targets`
  zero warnings; full `cargo test -p alo-store` green (539 unit + all
  suites) and full `cargo test -p alo-jmap` green (409 unit + every
  integration suite) on docker Postgres, incl. the new sequenced
  `site_notify` end-to-end test: two tenants on one store, one sweep —
  each owner gets exactly ONE message in their OWN inbox with the right
  Subject/Reply-To/To and body, **neither tenant's words ever reach the
  other's inbox** (the queue's wrong-tenant criterion), a second sweep
  delivers nothing (claimed ≡ notified), a CR/LF-bearing sender name never
  becomes a header line, and a non-ASCII name survives the 2047 round trip.
  Manual pass: `\d site_form_submissions` in psql shows `notified_at` + the
  partial index + cascade FK exactly as designed. No new HTTP routes → the
  curl wire-verify gate doesn't apply; a stale `alo-jmap.exe` from 09:02
  was killed per protocol before building.
- **Cuts/flags:**
  - Server-generated notification mail is English-only (like DSN/RSVP
    mail); localizing it is flagged for the wave review — the web i18n
    catalogs do not reach this process.
  - Delivery failure (e.g. owner deleted) logs without addresses/content
    and moves on; the claim is already burned — accepted under the
    at-most-once trade documented in the module doc.
  - The one test is deliberately a single sequenced scenario: the sweep is
    global, so parallel test scenarios could claim each other's rows.
  - CHANGELOG: user-voice entry added.
- **Next:** S1.16c2 (form auto-create on section add + the full wire arc).

## S1.16c2 — contact section auto-creates its form (2026-08-08)

- **Shipped:** `POST /sites/:site/pages/:page/sections` now turns a new
  `contact_form` section into a working form in one action. When the editor
  sends no `form_id`, the authenticated owner door creates a tenant-scoped
  form (the section heading is its owner-facing name, with `Contact form` as
  the blank-heading fallback, shortened by Unicode characters to the form
  name cap when needed), writes the generated id into the canonical
  section envelope, and returns that envelope to the editor. A supplied id
  must resolve through the same tenant + site door or the route answers the
  same clean 404 as an unknown form. If the page write fails after creation,
  the just-created form is removed so the failed action leaves no orphan.
- **Verified:** touched Rust files are rustfmt-clean; `SQLX_OFFLINE=true cargo
  clippy -p alo-jmap --all-targets -- -D warnings` clean; focused real-Postgres
  tests green for auto-create/link + foreign-form refusal and for the complete
  notification arc; full `cargo test -p alo-jmap` green with exit code 0 on a
  dedicated local database (the shared `alo` DB was concurrently advanced to
  migration 132 by the other build track during the first run). The mandatory
  wrong-tenant coverage proves a foreign section POST creates no form, a valid
  foreign form id cannot be linked, the outsider cannot read the submission,
  and no notification enters a foreign inbox.
- **Wire transcript:** after killing every stale `alo-jmap.exe`, freshly built
  `alo-jmap` (8080) and `alo-sites` (8081) ran against docker `alo-pg` / database
  `alo`. Real PKCE login: authorize 303, token 200. Authenticated editor arc:
  create site 200 → create home page 200 → add contact section 200 (returned
  linked form `Yo_Qll_UM6mTNpI9-ot5Cg`) → publish 200. Public urlencoded
  `POST /f/{form_id}`: 200. Corrected form-scoped SQL evidence after the
  background sweep: submissions=1, notified=1; messages for the form's tenant=1,
  messages for every other tenant=0. (The first observation query named a
  nonexistent submissions `site_id` column; no write depended on it, and the
  corrected schema-accurate query produced these counts.) No outbound email and
  no external AI call occurred.
- **Cuts/flags:** auto-creation is deliberately on the editor's add-section
  route named by this item; atomic full-envelope imports keep their existing
  contract. Server-generated notification copy remains English-only, as
  already recorded in S1.16c1.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.17a — per-site submissions inbox (2026-08-09)

- **Shipped:** each website now exposes a visible Submissions action beside
  Pages. Its responsive split inbox lists every contact-form message newest
  first with the sender, form and open/handled state; selecting a row shows the
  full message in place. Mark handled and Reopen are one-click surface actions
  with immediate optimistic feedback and rollback plus the server's reason on
  refusal. The empty state teaches the single next step and returns directly to
  Pages. All new copy ships in English, French and Dutch.
- **Boundary:** authenticated `GET /sites/:site/submissions` aggregates the
  account door's site-scoped forms and submissions, and
  `PUT /sites/:site/forms/:form/submissions/:submission` changes only the
  handled flag. Both verify the caller's tenant + site boundary; copied foreign
  ids answer the same 404 as invented ids.
- **Verified:** direct rustfmt on the three touched Rust files;
  `SQLX_OFFLINE=true cargo clippy -p alo-jmap --all-targets --jobs 1 -- -D
  warnings` clean; focused owner-flow and mandatory wrong-tenant tests green;
  full `cargo test -p alo-jmap --jobs 1` green (426 unit tests plus every
  integration binary, including 13 Sites HTTP tests) against isolated local
  Postgres. Web: SitesModule 16/16, `npx tsc --noEmit`, focused ESLint, and
  production build clean (existing chunk/circular-chunk warnings only).
- **Wire transcript:** after killing stale `alo-jmap.exe`, fresh `alo-jmap`
  (8080) and `alo-sites` (8081) ran against docker `alo-pg` / database `alo`.
  Real PKCE authorize 303 and token 200; authenticated site/page/contact-form
  creation and publish 200; public urlencoded form POST 200; new per-site list
  200 returned one `Talk to us` message from `visitor@example.test`; handled
  write 200; relist returned `handled: true`. The first public-submit command
  split its URL at the form id and therefore created nothing; the corrected
  single-string URL produced the recorded transcript. No production host,
  external AI, outbound email or secret was used.
- **Design references:** Mail/Gmail's list-and-reading-pane reflex for fast
  triage, adapted to the Sites surface; the empty state, visible inbox entry,
  and one-click resolution follow UX laws 1, 2, 5, 6 and 12.
- **Cuts/flags:** aggregation intentionally uses the existing capped form list
  (maximum 50 per site), avoiding a second storage query contract. CSV export
  remains S1.17b. The complete crate gate initially stalled when its output
  host disappeared; after stopping the orphan, the warm foreground rerun
  completed green.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.17b — submissions CSV export (2026-08-09)

- **Shipped:** the per-site Submissions inbox now keeps a visible Export CSV
  action on its header surface. One click fetches the authenticated export,
  shows an immediate preparing state, and saves a recognisable
  `submissions-<site-address>.csv` without navigating away. The file contains
  received time, form, sender name and email, message, and handled state in the
  same newest-first order as the inbox. Download and failure copy ships in
  English, French and Dutch.
- **Boundary and file safety:** authenticated
  `GET /sites/:site/submissions.csv` shares the exact tenant-scoped reader used
  by the JSON inbox; copied foreign or invented site ids answer the same 404.
  User-authored cells that a spreadsheet could execute as a formula are
  neutralised before RFC 4180 quoting. The response is an attachment with a
  validated-address filename, stated UTF-8 CSV type, `nosniff`, and `no-store`.
- **Verified:** rustfmt clean; `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo
  clippy -p alo-jmap --all-targets --jobs 1 -- -D warnings` clean; focused CSV
  contract and mandatory wrong-tenant tests green; authoritative full
  `cargo test -p alo-jmap --jobs 1` green (439 unit tests plus every integration
  binary, including 14 Sites HTTP tests, and doc tests). Web: SitesModule
  17/17, `npx tsc --noEmit`, focused ESLint, and production build clean
  (existing chunk and circular-chunk warnings only).
- **Wire transcript:** after killing the stale `alo-jmap.exe`, a freshly built
  local server ran on 127.0.0.1:8080 against docker `alo-pg` / database `alo`.
  Real PKCE authorize returned 303 and token exchange 200. Authenticated CSV
  export returned 200, the six-column contract header, the seeded visitor
  address, UTF-8 CSV content type, attachment filename, and `no-store`; an
  invented site returned 404. The first PKCE script used an unavailable static
  RNG helper but still completed; the clean rerun used the platform-supported
  RNG instance and produced the recorded result. No production host, external
  AI, outbound email, or secret was used.
- **Design references:** Wix/Squarespace form-submission exports for the Sites
  expectation and Mail/Gmail's visible inbox actions for placement. Export is
  one click, stays visible rather than living in a menu, responds immediately,
  and is disabled with an empty inbox instead of downloading a confusing blank
  file (UX laws 1, 2, 3, 6, 8 and 12).
- **Cuts/flags:** the export intentionally carries stable English column/status
  values as a machine-readable contract while the surrounding UI is localised.
  An initial full-suite process was stopped after buffered output made a healthy
  run look idle; its reported tests were green, and the complete untouched warm
  rerun is the authoritative green gate. The first Vitest command was launched
  from the repository root and selected an unconfigured package; the correct
  web-workspace run is the recorded 17/17 result.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.18 — tenant-safe blog post model and routes (2026-08-09)

- **Shipped:** alo Sites now stores a blog post as public metadata around one
  existing alo Docs document: slug, title, excerpt, optional cover image,
  draft/published state and publication time. Authenticated list, create, read,
  full-metadata update, publish, unpublish and delete routes are wired. Deleting
  a post deliberately leaves its source document in Drive.
- **Boundary:** the account door accepts only a readable, non-trashed Drive
  node of kind `doc`; copied foreign-tenant and inaccessible personal-document
  ids resolve exactly like missing ids. Composite database foreign keys keep
  site and document references inside the row's tenant, cover images must be
  image-typed blobs owned by that tenant, and every read/write scopes by tenant
  plus site plus post. The dedicated store proof and the HTTP suite's complete
  wrong-tenant barrage cover the boundary.
- **Verified:** targeted rustfmt; `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo
  clippy -p alo-store --all-targets --jobs 1 -- -D warnings` and the equivalent
  `alo-jmap` gate clean; focused store tenancy, blog route lifecycle and
  mandatory wrong-tenant tests green; authoritative full `cargo test -p
  alo-store -p alo-jmap --jobs 1` green, including all integration and doc
  tests (15 Sites HTTP tests). After rebasing over two new migrations, a clean
  isolated local database was migrated through the renumbered `0138` post
  schema and the store, route lifecycle and wrong-tenant proofs passed again.
  The first complete-suite attempt hit Windows
  linker `LNK1318` in an unrelated Billing test executable; per the machine
  rule only `target/debug/incremental` was removed, and the untouched retry is
  the recorded green gate.
- **Wire transcript:** after confirming no stale `alo-jmap.exe`, a freshly
  built local server ran on 127.0.0.1:8080 against docker `alo-pg` / database
  `alo`. Real PKCE authorize returned 303 and token exchange 200; registering a
  Drive doc, creating a site, creating its post, publishing it and reading it
  each returned 200. The returned post was `published` and retained the exact
  source document id; an invented site returned 404. No production host,
  external AI, outbound email or committed secret was used.
- **Design references:** WordPress and Ghost's post metadata reflex, with the
  alo Docs body kept as the sole editable source. The route shape keeps the
  future editor's core draft/publish actions direct and visible rather than
  menu-gated (UX laws 1, 2, 6 and 12).
- **Cuts/flags:** this slice is the model and authenticated contract only. The
  public blog index, post renderer, RSS feed and editor surface remain their
  own queued slices. Main introduced migrations 0136 and 0137 while this item
  was in flight, so the post migration was moved to 0138 before landing. A
  first focused test invocation inherited the harness's
  historical 5433 fallback and timed out; the mandated explicit local 5432
  URL was then used for every authoritative database gate.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.19a — BlockNote core document renderer (2026-08-09)

- **Shipped:** `alo-sites` now turns the exact BlockNote JSON persisted by alo
  Docs into a semantic HTML fragment without loading a browser editor runtime.
  Paragraphs, heading levels, quotes, adjacent and nested bullet/numbered
  lists, checklists, links and the four core inline marks render directly. A
  captured BlockNote 0.52 document fixture and checked-in HTML golden lock the
  interchange contract.
- **Safety and compatibility:** all text uses the existing Sites HTML escape
  primitive and links reuse its protocol allowlist, so a scriptable URL becomes
  inert. Unknown future block kinds never become trusted markup; the renderer
  skips the unknown surface but still walks safe child blocks. Invalid JSON and
  a non-array document root return typed errors instead of partial output.
- **Verified:** targeted rustfmt; `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo
  clippy -p alo-sites --all-targets --jobs 1 -- -D warnings` clean; focused
  BlockNote golden green; authoritative full `cargo test -p alo-sites --jobs
  1` green against the required local Postgres on 5432: 7 unit tests, the new
  golden, all 6 form tests, 3 render goldens, 15 render safety tests, 5
  Host-isolation/service tests, 5 stylesheet tests and doc tests. No storage or
  HTTP contract changed in this item, so no new wire route or tenant door was
  introduced.
- **Design references:** BlockNote's own 0.52 JSON fixtures define the source
  shape; Google Docs and Word define the expectation that headings and nested
  lists retain document hierarchy; semantic HTML and the existing alo Sites
  renderer define the public output.
- **Cuts/flags:** rich images, code and equation fallback plus the broader XSS
  corpus remain S1.19b. The Windows runner first had a stale `alo-sites.exe`,
  then exhausted the PDB limit and C: drive with 54.4 GiB of reproducible Rust
  target artifacts. The stale process was stopped, only generated `target`
  output was cleaned, and the authoritative full test used zero debug symbols.
  An initial database-backed full test inherited the old 5433 fallback; the
  explicit required 5432 rerun above is the recorded green gate.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.19b — rich BlockNote document renderer (2026-08-09)

- **Shipped:** alo Sites now renders the rich blocks already stored by alo
  Docs: images become semantic figures with captions, standard and alo code
  blocks become escaped language-labelled code, and equations get a readable
  runtime-free math fallback. The checked-in real-shape fixture and HTML golden
  make the public interchange contract visible and deterministic.
- **Safety and privacy:** public article images accept only the existing
  same-origin `/assets/img/` path or strict base64 raster data; remote URLs,
  scriptable URLs and SVG data are omitted. Image text, code, language tokens
  and equations are escaped or narrowed before entering HTML. A hostile corpus
  proves scripts, injected attributes, raw HTML and scriptable image payloads
  never render live while safe child text remains readable.
- **Verified:** targeted rustfmt; `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo
  clippy -p alo-sites --all-targets --jobs 1 -- -D warnings` clean; full
  `cargo test -p alo-sites --jobs 1` green against the required local Postgres
  on 5432: 7 unit tests, all 3 BlockNote goldens/safety tests, 6 form tests, 3
  render goldens, 15 render safety tests, 5 Host-isolation/service tests, 5
  stylesheet tests and doc tests. No storage or HTTP contract changed, so this
  item introduced no tenant door or route requiring a new wire transcript.
- **Design references:** BlockNote's persisted rich-block shapes define the
  source contract; Google Docs and Word define the expectation that pictures,
  code and equations survive publishing; the existing alo Sites zero
  cross-origin-request contract defines the image allowlist.
- **Cuts/flags:** equations intentionally ship as accessible escaped source
  rather than adding a public-page math runtime. Image binary magic-byte
  verification remains the upload pipeline's responsibility; this renderer
  verifies the HTML safety boundary and declared raster transport shape.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.20a — public blog index and article pages (2026-08-09)

- **Shipped:** every live alo Site now serves a themed `/blog` index with
  responsive article cards and `/blog/<slug>` article pages carrying canonical
  and Open Graph metadata, the optional cover, publication date and the safe
  BlockNote body. The visible site name, Home and Blog links keep the reading
  path understandable without a menu. Complete index and article HTML goldens
  pin the public contract, and the shared stylesheet gives cards and long-form
  content one responsive theme-aware surface.
- **Publication and tenancy:** only `status = 'published'` rows can cross the
  public store door. Every list, body and cover-reference query is scoped by
  the private tenant/site pair produced by Host resolution; draft, trashed,
  wrong-kind and foreign documents are clean absences. A mandatory two-tenant
  test proves each resolved host sees only its own title, body and cover. Cover
  bytes still pass the existing image MIME and tenant checks after the new
  published-reference gate.
- **Correctness:** blog publication and alo Docs changes do not flip the site's
  immutable page-publish id, so blog HTML deliberately bypasses the page cache
  and answers `no-cache`; ordinary pages keep their existing render cache and
  strong entity tags. Invalid published document JSON fails closed as the
  service's static temporary-unavailable response rather than leaking bytes or
  emitting partial markup.
- **Verified:** targeted rustfmt; `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo
  clippy -p alo-store -p alo-sites --all-targets --jobs 1 -- -D warnings`
  clean; mandatory focused wrong-tenant proof green; authoritative full
  `cargo test -p alo-store -p alo-sites --jobs 1` green against local Postgres
  on 5432, including the 695 alo-store unit tests, all store integrations, 2
  blog goldens and 6 public-service HTTP tests. The first combined full-suite
  invocation reached the terminal's five-minute linking ceiling without a
  result; the warm untouched foreground rerun is the recorded green gate.
- **Wire transcript:** after confirming no stale `alo-jmap.exe` or
  `alo-sites.exe`, a freshly built `alo-sites` ran on 127.0.0.1:8081 against
  docker `alo-pg` and `C:/dev/Ficina/.localdev/blobs`. Real Host-header curls
  returned 200 plus `no-cache` for `/blog`, showed the published card and no
  draft marker; `/blog/wire-public-story` returned 200 with the real local Docs
  body and exact canonical URL; `/blog/wire-draft-story` returned 404; and its
  referenced cover returned 200, `image/png` and the planted local bytes. The
  test service was stopped afterwards. No production host, external AI,
  outbound email or committed secret was used.
- **Design references:** WordPress and Ghost establish the public journal
  reflex; Google Docs establishes that the authored document remains the body.
  The index exposes every core reading action directly in each card and the
  article preserves visible Home/Blog navigation (UX laws 1, 2, 6 and 12).
- **Cuts/flags:** pagination and RSS intentionally remain S1.20b. Publication
  currently reads the published post's current alo Docs bytes, matching a
  published article's normal edit-in-place behavior; it is kept out of the
  immutable page cache so changes cannot be served under a stale validator.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.20b — bounded blog pagination and RSS (2026-08-09)

- **Shipped:** `/blog` now reads twelve published posts at a time and exposes
  visible, keyboard-sized Previous and Next controls plus an honest “Page n of
  n” position. Page one keeps the clean `/blog` address, later pages have their
  own canonical query URL, and malformed, duplicate, zero or out-of-range page
  numbers receive the site's themed 404. The pure renderer golden pins the
  complete second-page document and controls.
- **Feed:** every blog now visibly links and advertises `/blog/rss.xml`. The
  RSS 2.0 document carries channel discovery metadata, absolute permalink
  GUIDs, escaped titles/excerpts and RFC 2822 publication dates for the newest
  fifty published posts. It is dynamic and `no-cache`, like the HTML journal,
  because article publication changes independently of page snapshots. A full
  XML golden pins the interoperable feed contract.
- **Boundaries:** the public store door replaced its unbounded metadata read
  with a capped offset/limit page plus a publication-only total. Both queries
  are derived from the resolved Host's private tenant/site pair. The mandatory
  two-tenant test exercises the new bounded door and proves totals and rows
  never cross tenants; draft posts remain absent from HTML and RSS.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true CARGO_INCREMENTAL=0
  cargo clippy -p alo-store -p alo-sites --all-targets --jobs 1 -- -D warnings`
  clean; focused 3-render-golden, 6-public-HTTP, 5-stylesheet and wrong-tenant
  suites green. The authoritative full `cargo test -p alo-store -p alo-sites
  --jobs 1` run passed against docker Postgres on 5432, including all 695 store
  unit tests and every integration/doc suite. The memory-throttled Windows
  linker took about ten minutes but completed normally; no retry or cut was
  used.
- **Wire transcript:** after killing stale `alo-jmap.exe`/`alo-sites.exe`, the
  exact freshly built binary ran on 127.0.0.1:8081 against local docker
  Postgres and `C:/dev/Ficina/.localdev/blobs`. Real Host-header curls against a
  local 13-post fixture returned: `/blog` 200, 12 cards, Page 1 of 2 and Next;
  `/blog?page=2` 200, one card, Previous and its exact canonical; page 3 404;
  `/blog/rss.xml` 200, `application/rss+xml; charset=utf-8`, `no-cache`, 13
  items, self link present and the draft marker absent. The server was stopped.
  No production host, external AI, outbound email or secret was used.
- **Design references:** WordPress/Ghost establish direct page controls and
  RSS auto-discovery; feed readers establish RSS 2.0's channel/item contract.
  Pagination is visible without a menu and every safe reading transition is
  one click (UX laws 1, 2, 6 and 12).
- **Cuts/flags:** the feed deliberately carries metadata and excerpts rather
  than duplicating full article HTML. Fifty feed items and twelve cards per
  page are bounded service constants; no visitor request can trigger an
  unbounded post scan.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.21a — blog authoring desk in Sites (2026-08-09)

- **Shipped:** each website now exposes a visible Blog posts action beside
  Pages, Submissions and Theme. The new authoring desk lists every linked
  article newest first with title, excerpt, localized status, last update and
  a visible Edit in alo Docs action. Loading uses a stable skeleton, and the
  empty state explains that new writing remains private until publication.
- **One-click authoring:** Write in alo Docs creates an empty alo Doc, binds it
  to a private draft post with a valid collision-resistant placeholder slug,
  and opens that exact document in the existing Docs editor. No menu or setup
  dialog gatekeeps writing. If the metadata link is refused, the server's own
  reason stays visible and the new blank document moves to recoverable Drive
  Trash instead of becoming an orphan.
- **Boundaries:** this is a web-only consumer of the tenant-scoped post and
  Drive routes already completed and wrong-tenant tested in S1.18. No HTTP
  route, store query, migration or public rendering boundary changed in this
  slice, so no new Rust or wire gate was required.
- **Verified:** `npx tsc --noEmit --pretty false` clean; focused ESLint clean
  across all changed Sites and additive en/fr/nl catalog files; all 20 real-
  client Sites integration tests green, including exact post metadata, the
  one-click Drive hand-off, server-reason rendering and orphan cleanup; and
  `npm run build` green (6,067 modules). The production build retained only
  the repository's existing Rollup chunk-size/circular-re-export warnings.
- **Design references:** Ghost and WordPress establish a site's visible post
  desk; Google Docs establishes the document as the article's authoring
  source. The surface passes the menu test, gives immediate busy feedback,
  teaches the first action when empty and keeps create/edit to one click (UX
  laws 1, 2, 5, 6 and 12).
- **Cuts/flags:** title/slug/excerpt/cover editing, publish/unpublish controls
  and status-chip polish remain deliberately in S1.21b. The placeholder slug
  is private and is replaced by the explicit public metadata flow there.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.21b — blog publishing controls in Sites (2026-08-09)

- **Shipped:** the blog desk now distinguishes Draft and Published articles
  with the shared neutral/success status chips. Every draft carries a visible
  Publish action; every live article carries visible Edit details and Take
  offline actions, while Edit in alo Docs remains present for the source body.
- **Publish flow:** one focused dialog collects the public article title, URL
  path, RSS/blog summary and optional cover image. Covers upload through the
  existing tenant-bound Drive image path, can be replaced or removed, and the
  complete metadata write must succeed before the publish route runs. An
  already-live article saves metadata in place. Taking an article offline is
  reversible and therefore acts in one click, with immediate busy feedback.
- **Errors and boundaries:** a rejected metadata write or publish keeps the
  dialog and the user's inputs intact and shows the server's exact safe reason.
  This slice consumes the update/publish/unpublish and image-upload contracts
  completed and wrong-tenant tested in S1.18; it changes no store query, HTTP
  route, migration or public renderer, so no new Rust or wire gate applied.
- **Verified:** strict `npx tsc --noEmit --pretty false` clean; focused ESLint
  clean across every changed TS/TSX and additive en/fr/nl catalog; all 23 Sites
  integration tests green, including exact cover upload, metadata body,
  publish order, refusal persistence and one-click unpublish; `npm run build`
  green over 6,068 modules with only the repository's existing Rollup
  chunk-size/circular-re-export warnings.
- **Design references:** Ghost and WordPress establish the post status desk
  and public-metadata step; Google Docs establishes the separate source-body
  edit action. Publish, edit, unpublish and body editing stay visible without a
  menu, feedback is immediate, and irreversible ceremony is avoided (UX laws
  1, 2, 6, 7 and 12).
- **Cuts/flags:** deleting blog metadata is intentionally not surfaced in the
  publishing flow: it is a distinct destructive record-lifecycle action and
  was not part of this item. Taking offline covers the reversible daily need.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.22a — public sitemap and crawler policy (2026-08-09)

- **Shipped:** every live alo Site now serves `/sitemap.xml` and
  `/robots.txt`. The sitemap contains the frozen publish's canonical pages in
  navigation order, followed by the blog index and every published article;
  draft articles are absent. Robots permits public crawling and advertises
  the exact Host-scoped sitemap URL.
- **Boundaries:** both documents are generated only after the existing strict
  Host resolver returns a live `PublishedSite`. Page routes come from that
  publish's already tenant-scoped snapshots and article routes come through
  the bounded public-post door. The HTTP isolation test proves one tenant's
  sitemap never names another tenant's host, and the sitemap renderer enforces
  the protocol's 50,000-URL ceiling. No schema or store query changed.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-sites --all-targets --jobs 1 -- -D
  warnings` clean; the full `cargo test -p alo-sites --jobs 1` gate green,
  including two new byte-for-byte SEO goldens and all six real-Postgres public
  HTTP tests. The shared local database carried the retired loop's old
  version-136 `site_posts` journal row; it was non-destructively renumbered to
  the current 138 entry, preserving all six local posts, before the missing
  main migrations applied.
- **Wire transcript:** after confirming no stale `alo-jmap.exe` or
  `alo-sites.exe`, a freshly built server ran on `127.0.0.1:8081` against
  docker `alo-pg` on 5432 and `C:/dev/Ficina/.localdev/blobs`. Real Host-header
  curls returned 200 for both routes: robots as `text/plain; charset=utf-8`
  with the exact sitemap declaration, and sitemap as `application/xml;
  charset=utf-8` with the local site's canonical home URL. Both returned
  `no-cache` and `nosniff`; the server was stopped. No production host,
  external AI, outbound email or secret was used.
- **Design references:** the Sitemap protocol and Google Search crawler
  guidance establish these zero-configuration discovery files. They are
  automatic public infrastructure and introduce no editor menu or extra click.
- **Cuts/flags:** sitemap entries intentionally omit optional priority and
  change-frequency guesses. Blog routes are included only when at least one
  article is actually published, avoiding discovery of an empty journal.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.22b — per-page search and sharing controls (2026-08-09)

- **Shipped:** the page editor now exposes a visible Search & sharing action
  beside Theme and Add section. Its focused dialog edits the optional search
  title and description, shows a live result preview, explains the automatic
  image choice, and saves both fields through the existing page update route.
  Blank values restore automatic defaults instead of emitting empty metadata.
- **Sharing defaults:** public rendering still prefers the first illustrated
  hero for `og:image`, and now falls back to the site's stored logo when a page
  has no hero art. Title continues to derive from page + site when no override
  exists; canonical URL, site name and explicit descriptions remain unchanged.
  A new full-document golden pins the derived title, canonical, OG title and
  logo URL together.
- **Feedback and errors:** the save action responds immediately with busy
  state, refreshes the exact server-rendered preview after acceptance, and
  closes only on success. A refusal leaves the dialog and the user's text in
  place and displays the server's safe reason verbatim. All copy ships in
  English, French and Dutch.
- **Boundaries:** the tenant-safe page metadata route and store normalization
  were already implemented and wrong-tenant tested; this slice adds no route,
  query, schema or write boundary. It changes only the Sites UI consumer and
  the pure public renderer's image fallback.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-sites --all-targets --jobs 1 -- -D
  warnings` clean; full `cargo test -p alo-sites --jobs 1` green, including
  four render goldens and fifteen render-rule tests. Web: `npx tsc --noEmit`
  clean; focused ESLint clean; all 44 Sites editor/module/theme tests green;
  production build green over 6,069 modules with only the repository's
  existing Rollup chunk-size/circular-re-export warnings. The first build
  invocation hit its two-minute command ceiling without a compiler error; the
  untouched retry completed in 69 seconds.
- **Design references:** Google Search result previews, WordPress page SEO and
  Ghost social metadata establish this visible per-page action and automatic
  branded fallback. The core action is surfaced, its outcome is recognizable
  before saving, and it takes one editor click to reach (UX laws 1, 2, 6 and
  12).
- **Cuts/flags:** this slice deliberately does not expose arbitrary OG-image
  uploads per page. The existing hero and theme-logo surfaces cover the common
  one-click path without creating a second asset lifecycle.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.23 — privacy-preserving public traffic collection (2026-08-09)

- **Shipped:** successful public HTML GETs now add one hit to a daily
  site/path/referrer-domain aggregate. A deployment-secret HMAC turns the
  transient proxy/peer address into a 32-byte day-separated visitor token,
  so repeat visits increment hits without incrementing daily uniques.
  Conditional 304 page loads count; assets, feeds, crawler files, failures,
  HEAD requests and form submissions do not.
- **Privacy boundary:** request query strings are discarded with URI path
  canonicalization; `Referer` is reduced to its lowercase DNS domain; user
  agent is never read; the transient address is consumed synchronously by
  HMAC before any async or storage boundary. The two new tables contain only
  tenant/site/day/path/referrer-domain counters and opaque 32-byte visitor
  tokens—no IP, UA, raw referrer, query, header or request-body columns.
  Analytics writes are best effort and cannot turn a metrics problem into a
  public-site outage.
- **Tenancy:** the write accepts a private `PublishedSite` resolved from the
  Host and has no tenant argument. Every aggregate and visitor-set key uses
  that value's private tenant/site pair. The mandatory wrong-tenant test loads
  the same transient visitor through two different live Hosts and proves each
  tenant receives only its own aggregate.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-store -p alo-sites --all-targets
  --jobs 1 -- -D warnings` clean; exact-tree `cargo test -p alo-store -p
  alo-sites --jobs 1` green, including all Sites banks, the 717-test store
  unit bank and every store integration suite. The new real-Postgres test
  proves two same-day hits become one unique, schema columns match a strict
  allow-list, raw request values are absent, and Host tenants stay isolated.
- **Wire transcript:** after killing stale `alo-jmap.exe` and `alo-sites.exe`,
  the freshly built binary ran on `127.0.0.1:8081` against docker `alo-pg` on
  5432 and `C:/dev/Ficina/.localdev/blobs`. A real curl to a freshly published
  fixture Host, carrying an address, private referrer path/query, user agent
  and page query, returned `200 text/html`, the expected ETag/cache headers
  and `nosniff`; the server was stopped. No production host, external AI,
  outbound email or committed secret was used.
- **Design and operations:** analytics is invisible automatic infrastructure,
  so it adds no menu or click to the publishing task. Operators must provide
  a unique `ALO_SITES_ANALYTICS_SECRET` of at least 32 bytes; it is validated
  at startup and never stored. The first binary-build attempt exposed an
  actually full C: volume after the broad store test bank; package-scoped
  `cargo clean -p alo-sites` removed only 2.6 GiB of regenerable artifacts,
  after which the prescribed retry and live wire check passed.
- **Cuts/flags:** collection intentionally stores no event stream, geographic
  data, device data or tracking cookie. S1.24 owns authenticated aggregate
  reads and the visible no-cookie explanation.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.24 — privacy-friendly per-site analytics (2026-08-09)

- **Shipped:** every site now has a visible Analytics action beside Pages and
  Submissions. The responsive panel surfaces 7, 30 and 90 day controls,
  visits and daily visitor estimates, a complete zero-filled visits-over-time
  chart, and ranked top pages and referrer domains. Empty reports teach the
  next step with a direct View site action. The privacy promise is explicit on
  the surface: "No cookies. No banner." All new copy ships in English, French
  and Dutch; loading uses skeletons rather than a blocking spinner.
- **Data contract and tenancy:** authenticated
  `GET /sites/:site/analytics?days=N` accepts 1–365 days and returns totals,
  every calendar day in the requested range, and ten-entry page/referrer
  rankings. The store first resolves the site through the signed-in account,
  then aggregates only that tenant/site pair. Visits are exact aggregate hits;
  visitors are anonymous per-day estimates and are not presented as a
  cross-day identity count. A real-Postgres wrong-tenant test proves one owner
  cannot read another owner's report while each owner sees only their data.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-store -p alo-jmap --all-targets
  --jobs 1 -- -D warnings` clean; full `cargo test -p alo-store --jobs 1`
  green (717 unit tests plus every integration target); full `cargo test -p
  alo-jmap --jobs 1` green, including the new route, invalid-range and
  cross-tenant coverage. Web: `npx tsc --noEmit`, focused ESLint, 25/25 Sites
  module tests and `npm run build` clean (existing Rollup chunk warnings only).
- **Wire transcript:** Docker Desktop was restarted after its stale engine
  processes stopped answering; `alo-pg` then reported ready on 5432. After
  killing any stale `alo-jmap.exe`, the freshly rebuilt binary ran on
  127.0.0.1:8080 with the prescribed local blob directory and issuer. Real
  curl: programmatic login 200; analytics without a token 401; authenticated
  seven-day report 200 with exactly seven daily buckets; `days=0` returned
  422 with `analytics period must be between 1 and 365 days`. The server was
  stopped. No production host, external AI, outbound email or committed
  secret was used.
- **Design references:** Wix and Squarespace establish the per-site dashboard
  placement; Plausible and Fathom establish the compact privacy-first report
  and plain-language tracking promise. The core report and period controls
  are visible without a menu (UX laws 1, 2, 5, 6 and 12).
- **Cuts/flags:** no event stream, geography, device fingerprinting or raw
  referrer data is exposed. Daily anonymous visitors intentionally cannot be
  deduplicated across days; rankings are limited to ten and ranges to one year
  so the aggregate endpoint stays bounded.
- **Maintenance found by the bank:** the full gate exposed two stale test
  assumptions from earlier Sites items: the notification fixture now supplies
  the required local analytics secret, and preview assertions distinguish the
  public absolute OG image from the editor-visible data URI. No production
  behavior changed in those repairs.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.25a — custom-domain ownership model and verification (2026-08-10)

- **Shipped:** migration 0148 adds a globally unique custom-host claim under a
  tenant/site foreign key, with explicit `pending`, `verified` and `live`
  states. The account door can claim, list, release, stamp a seen TXT proof and
  activate only an already verified host. Validation canonicalizes case and
  rejects schemes, ports, paths, wildcards, IPs, Unicode, bad labels and names
  longer than the DNS limit with a fixable sentence. A conflict says only
  `domain is already connected`, never who owns it.
- **HTTP and DNS boundary:** authenticated `GET|POST /sites/:id/domains`,
  `DELETE /sites/:id/domains/:domain` and
  `POST /sites/:id/domains/:domain/verify` expose the claim lifecycle. Create
  returns the exact `_alo-sites.<domain>` TXT record with an opaque
  `alo-site-verification=` value. Missing DNS is a normal retryable 200 that
  stays pending. The router's TXT lookup is injected: production uses the
  system Hickory resolver, while tests return deterministic records without
  external DNS. The exact match alone changes the state to verified; live is
  deliberately reserved for S1.25b's serving activation.
- **Tenancy:** every query includes tenant + site, and claim creation is an
  `INSERT ... SELECT` through the owned-site predicate. The mandatory real
  Postgres test proves foreign list/create/verify/activate/delete all look
  absent, a global collision reveals no owner, the original claim survives,
  and release permits a different tenant to claim the now-free host.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-store -p alo-jmap --all-targets
  --jobs 1 -- -D warnings` clean; full `cargo test -p alo-store --jobs 1`
  green (795 unit tests plus every integration target); full `cargo test -p
  alo-jmap --jobs 1` green (456 unit tests plus every integration target),
  including all 17 Sites HTTP tests. The focused storage and mocked-DNS route
  banks also passed independently.
- **Wire transcript:** Docker `alo-pg` was ready on 5432. After killing a stale
  `alo-jmap.exe`, the freshly rebuilt binary ran at 127.0.0.1:8080 with the
  prescribed local blob directory and issuer. Real curl: login 200; anonymous
  domains list 401; scheme/path input 422 with the ASCII-host rule; claim 200
  pending with the expected `_alo-sites` TXT name; list 200 with one row;
  verify against an unowned `.invalid` name 200 pending; a different tenant's
  real site id 404; delete 200. The fixture claim was removed and the server
  stopped. Exact TXT success stayed in the no-network injected test. No
  production host, external AI, outbound email or committed secret was used.
- **Environment note:** the retired `C:/dev/Ficina-loop` process twice
  restarted heavy cargo work during the full gates. Only that retired process
  tree was stopped; the foreground banks were allowed to finish and passed.
- **Cuts/flags:** this item does not serve the custom Host, provision TLS,
  generate customer DNS guidance or mark a verified claim live; those are the
  explicitly queued S1.25b serving half. International names must arrive as
  punycode until a deliberate IDNA display/normalization policy is designed.
- **Next:** the first unchecked item in `QUEUE.md`.

## S1.25b — custom-domain serving and TLS authorization (2026-08-10)

- **Shipped:** alo-sites now classifies every valid request authority as either
  one built-in Sites subdomain or one canonical custom Host. A live custom
  claim resolves through its owning tenant/site/publish join and serves the
  same pages, blogs, images, discovery documents and analytics path as the
  built-in host. Render caches and canonical/OG URLs are keyed by the exact
  public host, so two domains never borrow one another's canonical bytes.
- **One-click activation:** the authenticated TXT verify action now promotes
  an exact DNS proof straight through verified to live. Pending and merely
  verified rows still cannot serve, while a successful check needs no hidden
  second activation step. `GET /internal/tls/ask?domain=…` answers 200 only
  when that built-in or custom hostname can currently resolve a published
  site; malformed, unknown, pending, verified-only and unpublished names get
  a metadata-free non-200.
- **Tenancy and verification:** the mandatory real-Postgres test proves a
  pending or verified custom claim cannot resolve, a live claim resolves only
  the claiming tenant's site, and deleting the claim removes that public
  route. The in-process HTTP bank additionally proves custom Host pages never
  contain another tenant's marker, custom canonical URLs use the requested
  host, TLS authorization shares the live boundary, and unpublish revokes both
  page serving and TLS authorization immediately.
- **Wire transcript:** Docker `alo-pg` was running on 5432. A temporary local
  live claim was attached to an existing local publish, then a freshly built
  alo-sites binary ran on 127.0.0.1:8081. Real curl returned 200 and the owning
  page marker for `Host: s125b-wire.example.test`, with the matching custom
  canonical URL; the TLS ask endpoint returned 200 for that custom host and a
  live built-in subdomain, 404 for unknown and malformed names; an unknown
  Host returned 404. The fixture claim was deleted and the process stopped.
- **Verified:** targeted rustfmt; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-store -p alo-sites -p alo-jmap
  --all-targets --jobs 1 -- -D warnings` clean; full `cargo test -p alo-store
  --jobs 1` green (805 unit tests plus every integration target); full `cargo
  test -p alo-sites --jobs 1` green (all unit, golden and integration targets,
  including seven Host HTTP tests); full `cargo test -p alo-jmap --jobs 1`
  green (464 unit tests plus every integration target, including all 17 Sites
  HTTP tests).
- **Human inbox — customer DNS how-to:** the product/help follow-up should
  show the generated `_alo-sites.<host>` TXT value first, then—after the one
  visible Verify action succeeds—ask the customer to point a subdomain with a
  CNAME to the deployment's Sites ingress. Apex domains need the provider's
  ALIAS/ANAME or CNAME-flattening equivalent. The ingress target is
  deployment-specific and must come from configuration, never be hardcoded;
  explain that HTTPS provisioning can take a few minutes after DNS propagates.
- **Environment note:** the first full store gate's linker became idle after
  retired `C:/dev/Ficina-loop` banks repeatedly consumed the constrained
  machine. Only those retired process trees and the orphaned idle attempt were
  stopped; `target/debug/incremental` was moved aside under ignored
  `.localdev`, and the prescribed single retry completed cleanly.
- **Cuts/flags:** no production host was contacted and no external DNS, AI or
  email action ran. This slice authorizes certificates but deliberately does
  not edit forbidden deployment/Caddy configuration. IDNA display conversion
  and automated ingress-record inspection remain outside this item.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.26a AI full-site draft envelope

- **Shipped:** added `alo_ai::sites`, a pure description-to-conversation
  builder and strict v1 full-site draft parser. The envelope carries the site
  name, proposed subdomain, validated shipped theme, and 1-20 fully typed
  pages with exactly one home page and unique valid slugs. Parsing delegates
  themes and page sections to alo-store's authoritative write gates, so model
  output cannot introduce an unknown section, prop, unsafe link, or schema
  variant the editor/publisher would reject.
- **Safety boundary:** generated drafts are proposals only. They cannot claim
  tenant blob ids, logos, favicons, or form ids, and the prompt forbids
  invented people, testimonials, prices, addresses, statistics and URLs. No
  site is created or published by this slice.
- **Prompt/fixtures:** the prompt documents the complete twelve-section v1
  vocabulary, every field, link rules, all shipped theme presets, page/home
  rules, and the asset-backed sections that generation must leave for the
  user. A deterministic two-page bakery fixture exercises the valid path;
  tests refuse unknown top-level fields and section types, duplicate/missing
  homes, future versions, non-objects, and fabricated asset/form references.
- **Verified:** `cargo fmt -p alo-ai`; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-ai --all-targets --jobs 1 -- -D
  warnings` clean; full `CARGO_INCREMENTAL=0 cargo test -p alo-ai --jobs 1`
  green (49 unit tests plus doc tests). All generation tests are fixtures and
  pure functions; no backend or external AI API was called.
- **Cuts/flags:** S1.26b owns the single repair retry; S1.28 owns inference,
  persistence, subdomain availability handling, and the guarantee that the
  accepted proposal is stored only as a draft.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.26b one-retry site generation repair

- **Shipped:** `generate_site_draft` now performs the complete bounded
  generation path: one low-temperature model turn, strict S1.26a parse, then
  exactly one correction turn only when the schema refuses the first reply.
  The repair conversation preserves the original description and full first
  reply, carries the validator's own bounded reason, and asks for one complete
  corrected object rather than an unrelated second guess.
- **Failure contract:** a second invalid reply becomes the typed
  `SiteDraftError::RepairFailed`; disabled, unconfigured, transport, empty and
  backend-status inference failures pass through without a retry. The helper
  still returns only a proposal and does not write or publish anything.
- **Fixtures:** two deterministic near-misses cover an unknown `carousel`
  section and an invented blob reference. Tests prove a valid correction is
  accepted on turn two, an invalid correction never earns turn three, the
  validator reason reaches the repair message, and inference failures consume
  one turn only. No test opens a network connection.
- **Verified:** `cargo fmt -p alo-ai`; strict `SQLX_OFFLINE=true
  CARGO_INCREMENTAL=0 cargo clippy -p alo-ai --all-targets --jobs 1 -- -D
  warnings` clean; full `CARGO_INCREMENTAL=0 cargo test -p alo-ai --jobs 1`
  green (53 unit tests plus doc tests).
- **Cuts/flags:** S1.28 owns the authenticated HTTP route, tenant AI config,
  typed unconfigured response, persistence, and draft-only transaction.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.27 typed, atomic AI page edits

- **Shipped:** added a strict v1 site-edit envelope with the complete closed
  operation set: `add_section`, `remove_section`, `reorder_section`,
  `set_prop`, and `rewrite_copy`. The prompt carries the exact current page in
  the user turn and documents all five visible operations, sequential index
  semantics, RFC 6901 property pointers, and the no-invented-facts/assets rule.
- **Safe targeting:** every operation against an existing section includes
  both its zero-based index and expected section type. Missing indices, stale
  indices, type mismatches, and missing property paths return the typed,
  human-readable `SiteEditError::Ambiguous` rather than touching a nearby
  section. Section `type` itself can never be changed.
- **Atomic apply:** edits run against a clone in declared order and return only
  after the complete result passes alo-store's authoritative section write
  gate. This supports adding/removing/reordering sections, filling absent
  optional props, nested array/object values, and copy rewrites while leaving
  the caller's page byte-for-byte untouched after any failure.
- **Verified:** focused tests cover each operation, sequential structural
  edits, strict unknown-op/field parsing, prompt isolation, nested set-prop,
  non-text rewrite refusal, invalid pointers, stale-target ambiguity, unsafe
  href refusal, atomic rollback, and schema-version preservation. `cargo fmt
  -p alo-ai`; strict `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p
  alo-ai --all-targets --jobs 1 -- -D warnings` clean; full
  `CARGO_INCREMENTAL=0 cargo test -p alo-ai --jobs 1` green (62 unit tests plus
  doc tests).
- **Cuts/flags:** this is proposal construction and pure application only;
  S1.29 owns the authenticated endpoint, before/after preview, and explicit
  Approve/Discard persistence flow. No AI endpoint or network was contacted.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.28a generated-site backend

- **Shipped:** added authenticated `POST /sites/generate` with a small,
  strict `{description}` body. It loads only the caller tenant's configured AI
  provider, runs the S1.26 complete-site parser and single repair turn, then
  translates the accepted proposal into store-owned inputs. A new
  `create_generated_site` boundary validates the site, theme, every page and
  every section before committing the site plus ordered pages in one database
  transaction. Generated sites have no writable status input and are always
  born `draft`; the route performs no publish operation.
- **Failure contract:** an absent or disabled provider answers `503` with
  `reason: "unconfigured"` and points the UI to blank-site creation; an
  unreachable provider answers typed `502`; invalid output after the one repair
  answers typed `422` and states that nothing changed. Request and description
  bounds are enforced before inference.
- **Tenancy and rollback:** focused store and HTTP suites prove a foreign
  tenant cannot resolve the generated site or page list, a refused second page
  leaves no site behind, invalid model output consumes exactly two turns and
  persists nothing, and the successful response contains exactly one home page
  with its full sections.
- **Verified:** `cargo fmt -p alo-store -p alo-jmap`; strict
  `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p alo-store -p alo-jmap
  --all-targets --jobs 1 -- -D warnings` clean; full `cargo test -p alo-store
  --jobs 1` green (805 unit tests plus all integration/doc tests); full `cargo
  test -p alo-jmap --jobs 1` green (464 unit tests plus all integration/doc
  tests). Fresh-binary curl against local docker `alo-pg`: unconfigured
  generation returned `503` with `reason:"unconfigured"`; the localhost
  fixture provider returned `200` with two pages and `status:"draft"`; a
  follow-up `GET /sites/{id}` returned `publish:null`. No external service was
  contacted.
- **Cuts/flags:** S1.28b owns the describe-your-business UI, editor redirect,
  and blank/template fallback. The disposable wire tenant remains only in the
  local development database; no credential or fixture helper was committed.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.28b generated-site onboarding

- **Shipped:** replaced the name-first website dialog with two complete,
  visible starting paths. Owners can describe their business, generate one
  private draft through `POST /sites/generate`, and land directly in its Home
  page editor. The AI-off sibling stays on the same surface: a blank option
  plus the server's shipped style templates, site name and address, live
  availability feedback, automatic Home-page creation, and the same direct
  editor hand-off. No generated or manual path publishes a site.
- **Failure contract:** the typed `reason: "unconfigured"` response switches
  in place to the blank/template path with a human next step. Other failures
  retain the server's own detail. Both primary buttons change label while
  work is running, and the manual path remains usable when template loading
  fails.
- **Verified:** the real Sites client and routed UI passed 27 focused Vitest
  tests. The click-path test enters a business description, submits it, checks
  the exact generation body, and observes `/sites/{id}/pages/{home}`. A second
  click-path receives the unconfigured response, selects the visible blank
  template, creates the site and Home page, and observes the editor route.
  `npx tsc --noEmit` clean; focused ESLint clean; `npm run build` green.
- **Cuts/flags:** shipped presets currently provide the manual path's visual
  starting points; section-layout templates remain future depth. The existing
  multi-write manual API creates site, optional theme, then Home page; S1.28a's
  generated path remains the atomic option.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.29a reviewable conversational page edits

- **Shipped:** added authenticated per-page `POST` and `PUT`
  `/sites/{site}/pages/{page}/ai-edits` doors. `POST` loads only the caller's
  current page and tenant AI provider, returns the strict S1.27 operation
  envelope, and writes nothing. `PUT` reloads the current page, reapplies the
  guarded operations against that canonical version, passes the result through
  the store's section gate, and only then persists it.
- **Surface:** the page editor now keeps conversational editing visible beside
  the section stack. An owner states the change, receives an ordered,
  human-readable change list, and can Approve or Discard without navigating
  away. Discard is client-only; Approve sends the exact reviewed envelope and
  replaces the editor with the canonical stored page. Server explanations are
  shown verbatim and the action gives immediate working feedback.
- **Tenancy and safety:** the real Postgres route test proves another tenant
  receives `404` from both proposal and approval doors. It also proves a
  proposal leaves the page unchanged, approval persists the rewrite, malformed
  or stale operations remain bounded by S1.27, and unconfigured/unreachable
  inference has typed machine-readable failures.
- **Verified:** `cargo fmt -p alo-ai -p alo-jmap`; strict
  `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p alo-ai -p alo-jmap
  --all-targets --jobs 1 -- -D warnings` clean; full `cargo test -p alo-ai
  --jobs 1` green (62 unit tests plus doc tests); full `cargo test -p alo-jmap
  --jobs 1` green (477 library tests plus every integration/doc test). The
  routed Sites UI passed 16 focused Vitest tests; `npx tsc --noEmit`, focused
  ESLint, and `npm run build` are clean. Fresh-binary curl against local docker
  `alo-pg` and a localhost-only fixture returned `200` for proposal and
  approval, proved the heading stayed `Old wire heading` before approval, and
  then stored `A clearer wire-tested welcome`. No external service was called.
- **Cuts/flags:** Windows hit the PDB linker ceiling twice in unrelated
  `alo-jmap` integration binaries. The crate's generated build artifacts were
  cleaned (48.2 GiB), and the complete suite passed with test debug metadata
  disabled. S1.29b owns before/after visual preview and approval-card polish;
  this slice deliberately shows the reviewed operation list only.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.29b exact before/after page preview

- **Shipped:** page-edit proposals now include the exact self-contained HTML
  produced by the public site renderer after replaying the validated operation
  set in memory. The proposal remains a no-write action; the stored page is
  only changed by the existing explicit approval door.
- **Surface:** the approval card names the number of proposed changes, explains
  that nothing is saved yet, and keeps Approve and Discard visible as
  full-size actions. Once a proposal is ready, the existing preview pane gains
  visible Before and After controls and opens on After. Owners can compare both
  states in one click on desktop or mobile, then approve or discard without
  losing the manual section editor.
- **Tenancy and safety:** the real Postgres route test retains the mandatory
  wrong-tenant `404` proof for both proposal and approval, verifies the rendered
  proposal contains the replacement but not the old copy, and proves proposal
  generation leaves canonical storage unchanged.
- **Verified:** `cargo fmt -p alo-jmap`; strict
  `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p alo-jmap --all-targets
  --jobs 1 -- -D warnings` clean; full `cargo test -p alo-jmap --jobs 1` green
  (477 library tests plus every integration and doc test). The page editor
  passed 16 focused Vitest tests; `npx tsc --noEmit`, focused ESLint, and
  `npm run build` are clean. A freshly built local backend and localhost-only
  model fixture returned `200`, rendered only the proposed heading in After,
  and a follow-up GET proved the stored heading was unchanged. No external AI
  service was called.
- **Cuts/flags:** the comparison is deliberately whole-page and exact, matching
  Wix/Squarespace preview reflexes; visual pixel diffs and side-by-side narrow
  layouts are future depth. The first live retry reached an orphaned prior
  fixture on the same port; that local process was removed and the clean rerun
  passed.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.30 reviewable per-field copy tools

- **Shipped:** every eligible stored section copy field now keeps an
  “Improve this copy” affordance beside the directly editable input. Owners
  can rewrite, shorten, add detail, or name a desired tone, then compare the
  exact current and proposed strings and Approve or Discard in place. This
  follows Wix/Squarespace contextual-editing reflexes while preserving the
  visible manual path required by the zero-menu law.
- **One guarded path:** the existing page `POST .../ai-edits` proposal route
  accepts a structured `{copy:{target,pointer,action,tone?}}` request and turns
  all four actions into the same generic operation proposal. The server
  resolves the selected tenant-owned string first and accepts the model result
  only when it is exactly one `rewrite_copy` operation for that same section
  and JSON pointer. Identifiers, links, asset ids, names, prices, and other
  factual fields do not expose the affordance.
- **Safety and tenancy:** proposing still writes nothing and approval still
  replays the reviewed envelope against canonical storage. The real Postgres
  route test proves a foreign tenant receives `404`, a schema-valid unscoped
  operation is rejected with typed `invalid_proposal`, and the selected copy
  remains unchanged until approval.
- **Verified:** `cargo fmt -p alo-jmap`; strict
  `SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p alo-jmap --all-targets
  --jobs 1 -- -D warnings` clean; full `cargo test -p alo-jmap --jobs 1` green
  (477 library tests plus every integration/doc target). The page editor
  passed 17 focused Vitest tests; `npx tsc --noEmit`, focused Sites/i18n
  ESLint, and `npm run build` are clean. A freshly built local backend and
  localhost-only model fixture returned one scoped `rewrite_copy`, rendered
  the exact proposed text, proved storage still contained `Wire copy before`,
  then stored `A warm local welcome` only after the approval `PUT`. The prior
  local AI provider was restored; no external AI service was called.
- **Cuts/flags:** new, unsaved sections keep direct editing only because they
  have no stable server target until first save. Array-backed plain-text
  feature lists stay manual until their storage shape can identify one string
  leaf without ambiguity.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.30b self-explanatory website addresses

- **Shipped:** the new-website dialog now suggests an editable address from
  the website name, displays the complete configured address in the same
  visible surface as its availability state, and explains which required
  value is missing whenever Create is disabled. This follows the familiar
  Squarespace, Wix, and GoDaddy domain-onboarding reflex without hiding the
  direct manual path behind a menu.
- **Input and errors:** owners can paste either a label or a complete URL such
  as `https://acme.alosites.com/`; the configured suffix is normalized before
  availability checks and creation. The server remains the sole authority for
  validity, reserved names, and collisions, and its validation detail is shown
  verbatim instead of being replaced by a generic failure.
- **Verified:** all 28 routed Sites module tests pass, including live address
  suggestion, full-domain normalization, taken addresses, verbatim server
  details, disabled-state guidance, and the exact create payload. `npx tsc
  --noEmit`, focused Sites/i18n ESLint, and `npm run build` are clean.
- **Cuts/flags:** normalization strips only the server-configured site suffix;
  unfamiliar domains and every semantic validation case intentionally remain
  server-owned. This slice changes no storage or HTTP route, so Rust gates,
  wrong-tenant testing, and live curl verification are not applicable.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.30c Create straight into the Home editor

- **Shipped:** both generated and manually templated websites create a Home
  page and navigate directly into its editor. A newly empty Home page now
  presents “Add your first section” as its primary onboarding action and opens
  a Hero form in one click; the visible Add section control remains the direct
  route to every other block. The Pages list remains available for later page
  management but no longer interrupts the creation flow.
- **Reflex and laws:** this follows the Wix/Squarespace builder reflex of
  landing in the editable result, removes a needless navigation decision, and
  keeps the core first-content action visible without a menu.
- **Verified:** all 28 routed Sites module tests pass. The creation test proves
  Create lands at the exact new Home editor URL, renders the empty-page
  onboarding, and opens the Hero form with one click. `npx tsc --noEmit`,
  focused Sites/i18n ESLint, and `npm run build` are clean.
- **Cuts/flags:** the existing create and page routes already supplied the
  required end-to-end behavior; this slice closes the remaining onboarding
  gap and strengthens its routed regression proof. No storage or HTTP route
  changed, so Rust gates, wrong-tenant testing, and live curl verification are
  not applicable.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.31a complete French and Dutch Sites language

- **Shipped:** French and Dutch now cover every Sites-facing catalog entry:
  website creation, address feedback, pages, the complete section builder,
  previews, themes, publishing, blog, submissions, analytics, custom domains,
  AI proposals, and the Websites rail label. The 155 foundational strings
  that previously fell back to English in each locale now read natively.
- **Regression guard:** a focused catalog test enumerates the English Sites
  surface and fails if either locale omits a current or future `sites*` key or
  the module label. A static usage audit also reports zero untranslated
  `strings.*` references across `web/src/sites`.
- **Changelog:** Unreleased now explains the complete-address/direct-editor
  creation flow, reviewable section and whole-page copy changes, and full
  French/Dutch Sites coverage in user language.
- **Verified:** 53 focused Sites tests pass across routed module, page editor,
  themes, and locale parity. `npx tsc --noEmit`, focused i18n/test ESLint, and
  `npm run build` are clean.
- **Cuts/flags:** server validation details remain verbatim by design, even
  when the server emits a different language; replacing them would violate
  the human-error rule and hide the authoritative reason. No storage or HTTP
  route changed, so Rust gates, wrong-tenant testing, and live curl verification
  are not applicable.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.31b S1 as-built reconciliation

- **Shipped:** `docs/design/sites.md` now records the shipped Websites web
  surface, creation/editor flow, storage and service boundaries, form
  notification path, custom-domain gate, privacy analytics implementation,
  AI review guarantees, and all deliberate global capabilities as built. An
  eleven-row reconciliation accounts for every `[S1]` promise, including the
  S1.30b/c full-address and direct-Home fixes. `docs/features.md` links to that
  accounting rather than leaving the wave's status implicit.
- **Human inbox:** consolidated the remaining production-only work: compose
  and Caddy routing, `ALO_SITES_ANALYTICS_SECRET`, live tenant AI-provider
  configuration, customer custom-domain DNS help, and the post-launch Public
  Suffix List submission. The purchased domain and live wildcard DNS remain
  recorded as done without repeating production infrastructure details.
- **Verified:** `git diff --check`, `npx tsc --noEmit`, and `npm run build`
  are clean. The build retains only the repository's existing Rollup circular
  re-export and large-chunk warnings. No source, storage, or HTTP route changed,
  so ESLint, Rust gates, wrong-tenant testing, and live curl verification are
  not applicable.
- **Cuts/flags:** V1 themes intentionally expose seven accessibility-checked
  presets rather than free-form colors; form-notification server copy remains
  English in S1; automatic CRM lead creation remains owned by B2. The public
  Sites service is code-complete but still awaits the human production deploy
  and configuration listed above.
- **Next:** the first unchecked item in `QUEUE.md`.

## 2026-08-10 — S1.32a final forms and publish arc

- **Shipped:** AI-generated contact sections now receive a real tenant-owned
  form and its ID inside the same transaction that creates the site and pages.
  A generated website can therefore move directly through section editing,
  theme selection, publishing, public rendering, visitor submission, the
  owner's submissions surface, and the owner's inbox without a dead form or a
  manual repair step. Generation rejects model-supplied form IDs, applies the
  existing per-site form limit, and derives the owner-facing form name from the
  section heading.
- **Tenant and atomicity proof:** the storage regression proves the generated
  form, linked section, site, and pages are visible to their owner and absent
  through another tenant's account door. The cross-service HTTP regression
  proves the outsider receives `404` for submissions and no inbox message.
  Site, form, and linked page writes remain one transaction.
- **Real curl transcript:** with fresh local `alo-jmap` and `alo-sites`
  processes, Postgres on `127.0.0.1:5432`, and a localhost scripted model
  fixture: generation returned `200` with a linked form; section edit `200`;
  midnight theme `200`; publish `200`; the generated subdomain Host returned
  `200` with the edited snapshot; theme CSS returned `200` with the midnight
  tokens; public form POST returned `200`; the authenticated submissions door
  returned one tenant row; and JMAP returned one internal owner-inbox
  notification. The fixture provider, generated site, and test processes were
  removed afterward. No external AI or outbound email was used.
- **Verified:** `cargo fmt -p alo-store -p alo-jmap`; `SQLX_OFFLINE=true cargo
  clippy -p alo-store -p alo-jmap --all-targets --jobs 1 -- -D warnings`;
  `cargo test -p alo-store --jobs 1 -- --test-threads=1` (832 unit tests and all
  integration/doc tests); `cargo test -p alo-jmap --jobs 1 --
  --test-threads=1 --format terse` (479 unit tests and every HTTP integration
  binary, including all five Sites generation tests); `cargo test -p alo-sites
  --jobs 1 -- --test-threads=1 --format terse`; `npx tsc --noEmit`; and `npm
  run build` are green. No web file changed, so focused ESLint is not
  applicable. The build retains only the repository's existing Rollup circular
  re-export and large-chunk warnings.
- **Cuts/flags:** the first full storage attempts omitted the required local
  `DATABASE_URL` and timed out against the harness fallback; explicitly using
  `postgres://alo:alo-dev-only@127.0.0.1:5432/alo` produced the clean full run.
  The public notification worker's real 30-second cadence was allowed in the
  curl proof; the regression test invokes the same sweep directly for speed.
- **Next:** S1.32b, the final blog, custom-domain, and zero-PII analytics arc.

## 2026-08-10 — S1.32b final blog, domain, and privacy arc

- **Shipped:** the final cross-service regression uploads a real BlockNote
  document through authenticated JMAP, creates its tenant-owned Drive node,
  links and publishes it as a Sites blog post, verifies a custom domain through
  the production DNS seam, serves the article through that custom Host, and
  proves the resulting analytics report contains exactly one visit and one
  anonymous visitor. The shared test harness now exposes the exact in-memory
  blob backend attached to JMAP so the public Sites service must render the
  bytes actually uploaded by Docs rather than a duplicate fixture.
- **Tenant and privacy proof:** another tenant receives `404` at the analytics
  door. The public request deliberately includes a raw IP, private user-agent,
  referrer path/query/fragment, and page query token; the API and database keep
  only `/blog/utrecht-mornings`, `news.example`, aggregate counts, and one
  32-byte daily HMAC. A schema assertion proves that no analytics table exposes
  an IP, user-agent, query-string, full-referrer, or equivalent raw-PII column.
- **Real curl transcript:** fresh local `alo-jmap` and `alo-sites` processes,
  Postgres on `127.0.0.1:5432`, filesystem blobs, and a fresh PKCE-authenticated
  tenant produced: document upload `200`; Drive document creation and blog
  publish `200`; custom-domain claim `200 pending`; no-network `.invalid` DNS
  verify `200 pending`; deterministic local fixture activation; custom Host
  article GET `200`; analytics `1 visit / 1 anonymous visitor`; stored dimensions
  `/blog/calm-work + news.example + daily HMAC`; raw-PII columns `0`. The exact
  TXT-to-live transition is covered by the injected-DNS regression, while the
  wire run intentionally made no external DNS call. All fixture processes,
  tenant data, and temporary blobs were removed afterward.
- **Verified:** `cargo fmt -p alo-jmap`; `SQLX_OFFLINE=true cargo clippy -p
  alo-jmap --all-targets --jobs 1 -- -D warnings`; focused final-arc test; full
  `cargo test -p alo-jmap --jobs 1 -- --test-threads=1 --format terse` (493 unit
  tests plus every integration/doc target, including 18 Sites HTTP tests); full
  `cargo test -p alo-sites --jobs 1 -- --test-threads=1 --format terse`; `npx
  tsc --noEmit`; and `npm run build` are green. No web file changed, so focused
  ESLint is not applicable. The build retains only the repository's existing
  Rollup circular re-export and large-chunk warnings.
- **Cuts/flags:** JMAP has no `/healthz`, so the wire harness uses OIDC discovery
  as its readiness probe. Exact public-DNS success remains deterministic and
  no-network in tests; the real-process transcript proves the retryable pending
  response and the complete live-domain serving path without contacting an
  external resolver. No external AI, production system, or outbound email was
  used.
- **Next:** `LOOP COMPLETE` — every Sites queue item is checked and the S1 arc
  is fully reconciled, exercised, and journaled.

## 2026-08-10 — S2.00 wave contract

- **Shipped:** expanded the Sites queue into small end-to-end slices for every
  requested S2 and later capability: multilingual publishing, Base-backed CMS
  collections, restricted collaborators, history/rollback, scheduling,
  passwords, responsive images, richer aggregate analytics, heatmaps,
  conversion attribution, templates, catalogs/orders, Agenda booking,
  sandboxed custom code, and in-product domains. Each UI item follows its
  storage/service foundation so the queue never ships a dead control.
- **Boundaries:** Sites owns its models, public service, editor surfaces, and
  integration adapters. Base and Agenda are consumed through tenant-scoped
  seams. Billing/CRM remain owned by their active track; Sites will join only
  through their existing or explicitly published interfaces and will not edit
  those modules. Registrar and AI work use deterministic fixtures only.
- **UX contract:** Wix/Squarespace are the domain references. Every core flow
  has a visible manual path, AI is optional, empty states teach one next step,
  safe actions target one click, and publish/payment/domain actions retain
  explicit review because they are outward or irreversible.
- **Verified:** `git diff --check` is clean. This planning-only slice changes
  no executable source, storage, or route, so Rust, web, wrong-tenant, and curl
  gates are not applicable.
- **Next:** S2.01a, the locale foundation.

## 2026-08-10 — S2.01a locale foundation

- **Shipped:** sites now store a validated default language and an ordered set
  of enabled languages. Existing and newly-created sites default to English;
  create/read/update routes expose `defaultLocale` and `enabledLocales`, accept
  useful lowercase BCP-47-style tags, reject duplicates and malformed tags,
  cap the visible language set at twelve, and require the default language to
  remain enabled. Existing callers remain source-compatible through the
  English-default `create_site` path.
- **Tenant proof:** the storage regression proves an owner can change locale
  settings, an invalid request cannot partially write, and another tenant gets
  `NotFound` while the original site's settings remain unchanged. The HTTP
  regression repeats create/read/update/validation and outsider-404 behavior
  through the real router.
- **Real curl transcript:** after killing the stale local JMAP process, a fresh
  local binary on `127.0.0.1:8080` and PKCE-authenticated admin created a site
  with `pt-BR`, `en`, and `nl` (`200`, canonicalized to lowercase), changed it
  to default `fr` with `fr`, `de`, and `en-GB` (`200`), rejected a default not
  present in its enabled set (`422 default language it must also be enabled`),
  and deleted the fixture. No production, external AI, DNS, or email system was
  contacted.
- **Verified:** `cargo fmt -p alo-store -p alo-jmap`; strict offline clippy for
  both crates; locale unit, storage wrong-tenant, and HTTP route regressions;
  full `alo-store` tests (841 unit tests plus all integration/doc targets); full
  unfiltered `alo-jmap` tests (499 unit tests plus every integration/doc target,
  including all 19 Sites HTTP tests); fresh `alo-jmap`/`alo-identity` binary
  build; `npx tsc --noEmit`; and `npm run build`. No web source changed, so
  focused ESLint is not applicable. Main's pre-existing audit-vocabulary drift
  was repaired and independently pushed as `c14e9af` before this Sites slice.
- **Cuts/flags:** this foundation stores the language registry only. Localized
  page content and visitor switching intentionally follow in S2.01b–d. The
  local wire script and fixture were removed after the transcript.
- **Next:** S2.01b, localized page drafts and fallback rules.

## 2026-08-10 — S2.01b localized page drafts

- **Shipped:** every site page now keeps one stable identity while storing a
  complete draft per enabled language: title, slug, section envelope, SEO
  title, and SEO description. Reads resolve the requested language first,
  then the site's default language, then the page's recorded base language;
  every response exposes `requestedLocale`, `resolvedLocale`, and `fallback`
  so editors never mistake fallback copy for a finished translation. Changing
  the site's default language preserves the old base draft instead of erasing
  it, and localized slugs remain unique within each site and language.
- **Tenant proof:** the storage regression covers exact and fallback reads,
  full localized upserts, localized slug collisions, disabled languages,
  default-language promotion, and another tenant receiving `NotFound` on both
  read and write while the owner's French draft remains unchanged. The HTTP
  regression repeats the outsider `404` behavior through the real router.
- **Real curl transcript:** after killing the stale process and starting a
  freshly built local JMAP server on `127.0.0.1:8080`, a PKCE-authenticated
  admin created a three-language site and page (`200`, `200`); requested French
  before translation (`200`, fallback `true`, resolved `en`); saved the full
  French draft (`200`, fallback `false`, resolved `fr`, slug
  `notre-histoire`); received `422` for disabled German; and deleted the test
  site (`200`). No production, external AI, DNS, or email system was contacted.
- **Verified:** `cargo fmt -p alo-store -p alo-jmap`; strict offline all-targets
  clippy for both crates; focused storage wrong-tenant and localized-route
  tests; existing page lifecycle and locale-route regressions; full
  `alo-store` tests before the final rebase (858 unit tests plus every
  integration/doc target), followed by the exact changed tenant-isolation test
  on rebased main; full `alo-jmap` tests on rebased main against a blank
  disposable database (511 unit tests plus every integration/doc target,
  including all 20 Sites HTTP tests); fresh `alo-jmap` and `alo-identity` build;
  `npx tsc --noEmit`; and `npm run build`. No web source changed, so focused
  ESLint is not applicable. Current main's meeting fixtures were repaired and
  independently pushed as `40f2bb1`; the duplicate field introduced when the
  same upstream fix crossed that rebase was removed in `3be01f5`.
- **Cuts/flags:** this slice stores editable drafts only. Immutable localized
  publish snapshots, public alternate/canonical links, locale-aware feeds, and
  the visitor language switcher intentionally follow in S2.01c. The local wire
  fixture and disposable gate databases were deleted after verification. The
  migration was ultimately renumbered to `0158` after rebasing over Billing's
  `0154` and Inventory migrations through `0157`. A
  second post-rebase full `alo-store` run exceeded the 15-minute process cap
  without an assertion failure; the already-green full run, exact rebased
  isolation test, strict clippy, and full rebased JMAP matrix are the recorded
  storage proof.
- **Next:** S2.01c, localized publishing and public-language discovery.

## 2026-08-10 — S2.01c localized publishing and discovery

- **Shipped:** publishing now freezes the site's default and enabled language
  contract plus every exact localized page draft in immutable snapshots. The
  default language stays at clean unprefixed paths; non-default translations
  use language-prefixed paths. Public pages expose exact `lang`, canonical,
  `hreflang`, and `x-default` metadata plus a visible direct language switcher.
  Sitemap entries carry the same exact alternates and `x-default`; RSS declares
  the frozen default language. A language enabled in the editor but missing an
  exact page is never fabricated on the public site.
- **Tenant proof:** the expanded publish isolation regression freezes French
  and English while omitting missing Dutch, proves later localized draft
  edits/deletion and theme edits cannot mutate an old publish, republishes a
  changed English/Dutch language contract, and keeps every foreign tenant/site
  read or publish denial clean. The public Host regression independently serves
  exact localized routes and rejects missing translations.
- **Real curl transcript:** a freshly rebuilt `alo-sites` binary on
  `127.0.0.1:18081` served `localized-wire.sites.test`: French `/` and English
  `/en` returned `200`; untranslated `/nl` returned `404`; HTML carried the
  exact language, canonical, visible switcher, and current-language state;
  `/sitemap.xml` carried French, English, and `x-default` alternates; and RSS
  declared `<language>fr</language>`. The isolated service, fixture, and
  disposable database were removed afterward. No production or external
  service was contacted.
- **Verified:** `cargo fmt -p alo-store -p alo-sites -p alo-jmap`; strict
  offline all-target clippy for all three crates; the exact localized publish
  wrong-tenant regression; full `alo-store` (912 unit tests plus every
  integration/doc target), full `alo-sites` including eight public Host tests,
  and full unfiltered `alo-jmap` (541 unit tests plus every integration/doc
  target). After the sitemap `x-default` wire finding, strict alo-sites clippy
  and its complete suite were rerun green. Web `npx tsc --noEmit` and
  `npm run build` are green; no web source changed, so focused ESLint is not
  applicable.
- **Cuts/flags:** renderer chrome is translated for English, French, and Dutch;
  other valid locales retain their exact document `lang` while using English
  chrome until catalog coverage expands. Blog posts are not localized yet, so
  the single public feed uses the frozen default locale. The migration is
  `0161` after rebasing over Inventory's `0159` and Purchasing's `0160`. The visitor surface follows
  Google Search's multilingual canonical/alternate contract and keeps language
  choices directly visible rather than gatekeeping them in a menu.
- **Next:** S2.01d, the visible translation editor and publish readiness.

## 2026-08-10 — S2.01d visible translation editor

- **Shipped:** each site now has a visible language workspace modelled on Wix
  and Squarespace: the default language, every enabled language, translated
  page counts, readiness, add/remove controls, and the next missing page are
  on the surface. A page opened in another language shows that language in the
  editor and URL. Missing translations remain visibly read-only fallback copy
  until the editor explicitly copies the source in one click; after that,
  title, slug, SEO fields, sections, ordering, and preview all read and write
  only the selected language. AI is absent from this path and is not required
  to finish or publish a translation.
- **Tenant proof:** the storage regression calculates readiness only from exact
  localized drafts and returns `None` for another tenant. The HTTP regression
  repeats `404` isolation for readiness, localized read, localized preview,
  and localized write, then proves the owner's French draft is unchanged. UI
  regressions prove fallback is read-only until copied and that subsequent
  section changes write only the chosen language.
- **Real curl transcript:** after killing the stale process, a fresh local
  `alo-jmap` binary used a disposable migrated database and filesystem blobs.
  Two real PKCE logins succeeded. The owner created a three-language site and
  page, wrote French, read readiness, and rendered the localized preview (all
  `200`); the HTML carried `lang="fr"`, the French heading, and `no-store`.
  The other tenant received `404` for the same readiness route. The fixture
  was deleted and no production, email, DNS, or external AI service was used.
- **Verified:** strict offline all-target clippy for `alo-store` and
  `alo-jmap`; full `alo-store` (978 unit tests plus every integration/doc
  target) and full unfiltered `alo-jmap` (571 unit tests plus every
  integration/doc target, including all 20 Sites HTTP tests) against the
  disposable database; `npx tsc --noEmit`; focused ESLint; 48 focused Sites
  UI tests; and `npm run build`. `git diff --check` is clean. Main contains
  pre-existing rustfmt drift in untouched identity and inventory tests, so the
  final formatting proof is restricted to the touched Rust files. After the
  final clean rebase over Inventory's `0164`, the exact storage tenant test and
  localized Sites HTTP isolation/readiness/preview test were rerun green.
- **Cuts/flags:** readiness counts pages, not blog posts, because localized
  posts are not yet modelled. Whole-site AI translation remains a separate,
  review-only optional path. The disposable database and local test process
  are removed after the slice lands.
- **Next:** S2.01e, deterministic whole-site translation proposals with
  before/after review and approve-only writes.

## 2026-08-11 — S2.01e reviewed whole-site translation

- **Shipped:** every non-default language now has a visible **Translate whole
  site** action beside the existing manual translation path. It prepares one
  deterministic proposal spanning page content and site-facing blog metadata,
  then shows each original and translated title/path in a review surface.
  Preparing never writes. Approval rechecks every source snapshot and applies
  every target page and post in one transaction, so stale or invalid output
  changes nothing. Non-home translated pages cannot acquire an empty path.
- **Tenant proof:** storage tests cover atomic page/post writes, exact localized
  post reads, stale-source rejection, invalid translated paths, and a foreign
  tenant receiving `NotFound` without changing owner data. The real-router test
  repeats proposal-without-write, approval, stale rejection, and foreign-tenant
  `404` behavior against PostgreSQL with a scripted localhost model.
- **Real curl transcript:** after killing the stale process, a freshly built
  `alo-jmap` used a disposable migrated database, filesystem blobs, two real
  PKCE logins, and a localhost-only OpenAI-compatible fixture. Preparing an
  English-to-French proposal returned the visible `Home` → `Accueil` review
  while the French page still resolved as fallback; approval returned `200`
  and the exact French page then read `Accueil`. The other tenant received
  `404` from both prepare and approve. The site, server, fixture, blobs, and
  disposable databases were removed; no external AI, production, DNS, or mail
  service was contacted.
- **Verified:** touched Rust formatting; strict offline all-target clippy for
  `alo-ai`, `alo-store`, and `alo-jmap`; their complete test suites plus the
  focused translation storage and real-router regressions; `npx tsc --noEmit`;
  focused Sites/i18n ESLint; 30 focused Sites UI tests; `npm run build`; and
  `git diff --check`.
- **Cuts/flags:** blog title, slug, and excerpt are translated and stored, but
  blog bodies remain alo Docs content and public locale-aware blog routing is
  a later publishing slice. The migration is `0170` after Inventory claimed
  `0166` and HR claimed `0167`–`0169` during rebases. The UI follows Wix/Squarespace's visible
  language workspace and a Google-Translate-style review-before-apply flow;
  AI is optional and never gatekeeps the manual path.
- **Next:** S2.02a, tenant-owned collection bindings to alo Base tables.

## 2026-08-11 — S2.02a tenant-owned Base collection bindings

- **Shipped:** Sites can now connect one named reusable content collection to
  an alo Base table. A binding keeps stable table and field ids rather than
  mutable display names and exposes a compact semantic mapping for title,
  slug, summary, body, image, link, and publication date. Title is required;
  richer roles remain optional so a two-column Base works immediately.
  Create, read, list, replace, and disconnect operations are tenant/site
  scoped, and disconnecting never removes the source Base or its records.
- **Validation proof:** every write resolves the selected Base through the
  caller's Drive access, proves the table belongs to that Base, proves every
  mapped field belongs to that table, and enforces role-compatible Base types
  (`text`, `attachment`, `link`, and `date`). The storage regression covers a
  missing table, a field from another table, an image mapped to text, atomic
  rejection that preserves the previous mapping, and a successful lifecycle.
- **Tenant proof:** a second tenant sees an empty collection list and no
  collection detail, receives `NotFound` for create/update/delete against the
  owner's site, cannot bind the owner's Base or inject its own Base into the
  owner's site, and leaves the owner's mapping unchanged. Composite foreign
  keys reinforce the tenant/site and tenant/Base-table ownership boundaries.
- **Verified:** `cargo fmt -p alo-store`; strict offline all-target
  `cargo clippy -p alo-store --all-targets --jobs 1 -- -D warnings`; focused
  collection compilation and the two focused collection tenancy tests; full
  `cargo test -p alo-store --jobs 1 -- --test-threads=1 --format terse` against
  a freshly migrated disposable PostgreSQL database (1,030 unit tests plus
  every integration/doc target); post-rebase strict Clippy and both focused
  tests on a second zero-state database after migration renumbering; and
  `git diff --check`. No web file or HTTP
  route changed, so TypeScript/ESLint/build and real-curl gates are not
  applicable to this storage-only slice.
- **Cuts/flags:** the first full-suite invocation reached the terminal's
  two-minute wrapper timeout without a test failure; the authoritative rerun
  used the required long foreground timeout and completed green. Collection
  row snapshotting and public rendering deliberately remain S2.02b. This
  collection migration is `0171`; the previously shipped Sites locale
  migration moved to `0170` when HR claimed `0169` during the final rebase. The model
  follows Webflow CMS and Contentful's structured-field convention while
  preserving alo Base as the single editable source of truth.
- **Next:** S2.02b, immutable collection publish snapshots and deterministic
  public rendering.

## 2026-08-11 — S2.02b immutable collection publishing

- **Shipped:** a published site now freezes every referenced Base collection
  into the same repeatable-read transaction as its pages. Only collections
  used by a page are copied, records keep a deterministic order, blank rows
  are skipped, and the mapped title, path, summary, body, image, link, and
  date are normalized into an immutable snapshot before the live pointer
  moves. Editing Base content therefore changes the next publish, never the
  site visitors are already reading.
- **Deterministic failure and empty behavior:** a row with content but no
  title, an invalid or duplicate path, a stale field mapping, a lost Drive
  permission, an unsupported value, or a missing image blob aborts the whole
  publish and preserves the previous live version. A genuinely empty
  collection renders the same localized empty state in English, French, and
  Dutch. Public rendering reads only the frozen snapshot and never reaches
  back into the mutable Base.
- **Tenant proof:** the storage regression publishes owner content, edits the
  source Base, proves the public snapshot is unchanged, republishes and sees
  the new value, then proves another tenant receives no collection rows. It
  also covers blank-row skipping, an empty collection, partial-row refusal,
  and disconnected-binding refusal without moving the previous live publish.
- **Real curl transcript:** a freshly started local `alo-sites` server used a
  disposable migrated PostgreSQL database and filesystem blobs. A real Host
  request to the published collection site returned `200` with the frozen
  “Fresh from the roaster” heading, “Night Ferry” card, and “Cocoa and
  blackberry” summary. The process was stopped afterward; no production,
  email, DNS, or external AI service was contacted.
- **Verified:** touched Rust formatting; strict offline all-target Clippy for
  `alo-store`, `alo-sites`, and `alo-jmap`; complete unfiltered test suites for
  all three crates (including 1,037 `alo-store` unit tests, every integration
  and doc target, the public-render goldens, 608 `alo-jmap` unit tests, and all
  Sites HTTP tests); focused zero-state storage tests; the real public curl;
  and `git diff --check`.
- **Cuts/flags:** the authenticated draft preview deliberately supplies no
  collection rows in this storage/rendering slice; S2.02c will give the editor
  an account-scoped collection preview and visible connect/map/disconnect
  controls. The public section follows Webflow CMS and Contentful collection
  card conventions, with semantic markup and no AI dependency.
- **Next:** S2.02c, visible collection connection, mapping, preview,
  disconnect, and empty-state controls.

## 2026-08-11 — S2.02c visible collection workspace

- **Shipped:** every site now has a visible Collections action and a complete
  recognition-first workspace. It discovers the caller's readable personal
  and Space Bases through Drive, opens the first table ready to connect,
  keeps Base/table/name and all seven field mappings on the surface, previews
  normalized rows, and disconnects without touching the source Base. A site
  with no Base gets one direct “Open Drive” next step rather than an empty
  form or an opaque-id prompt.
- **Page-builder path:** Collection is now a first-class thirteenth section.
  The picker names it, the form lists the site's connected collections, and
  a missing connection links directly to the Collections workspace. Draft
  page preview resolves current Base rows through the same validation and
  normalization path as publishing, including mapped collection images,
  while the live site remains pinned to its immutable publish snapshot.
- **Feedback and safety:** connection save, preview, and disconnect failures
  keep the server's specific reason visible through the shared Sites error
  path. Disconnect is a two-click reversible-boundary action whose second
  state explicitly says the Base rows remain. A direct preview of a missing
  or foreign connection is tenant-hidden `404`; publish-time dangling page
  references remain an actionable validation refusal.
- **Tenant and wire proof:** the real router test connects, lists, previews,
  updates, renders, and disconnects a collection, proves another tenant gets
  `404` for list/create/update/preview/delete, and proves the Base survives.
  A freshly built local server on `127.0.0.1:8080` then completed a real OAuth
  PKCE flow and real HTTP create/list/preview/page-preview/disconnect cycle on
  a disposable database; the rendered HTML contained the mapped heading,
  title, and summary, and the Base remained readable afterward.
- **UI proof:** the real Sites client tests cover no-Base onboarding, Drive
  navigation, type-compatible field choices, one-click connect, normalized
  row preview, explicit disconnect state, and the expanded section picker.
  The workspace follows Webflow CMS's visible collection setup and
  Contentful's source-to-field mapping conventions; AI is neither required
  nor part of the core flow.
- **Verified:** touched Rust formatting; strict offline all-target Clippy for
  `alo-store` and `alo-jmap`; full unfiltered `alo-store` (1,096 unit tests
  plus every integration/doc target) and `alo-jmap` (614 unit tests plus
  every integration/doc target) suites on a freshly migrated disposable
  PostgreSQL database; focused Sites web tests; TypeScript; focused ESLint;
  the production Vite build; real curl; and `git diff --check`.
- **Cuts/flags:** collection cards in the mapping workspace show normalized
  content and image presence; the authenticated page preview is the exact
  visual render, including image bytes. The workspace does not duplicate a
  Base editor: “Open Drive” remains the visible one-click route to change
  source rows. No external AI, production service, email, or DNS was used.
- **Next:** S2.03a, a tenant-safe per-site editor grant that exposes no other
  site or workspace data.

## 2026-08-11 — S2.03a per-site editor boundary

- **Shipped:** alo Sites now has an explicit restricted `site_editor` role and
  tenant-owned grants naming each website the collaborator may edit. Granting
  a site and the restricted role is one transaction; revoking the final site
  removes the role in the same transaction. Repeating either intent is safe.
- **One central door:** the router's scoped-role middleware refuses every
  non-Sites route for a non-admin site editor and proves a per-site grant before
  any `/sites/{id}` handler runs. `GET /sites` is filtered to granted records;
  creation and AI generation stay owner-only, while the visible theme/config
  references remain available. Admins remain admins even if a role row exists.
- **Isolation proof:** storage tests refuse both a foreign user and a foreign
  site, keep foreign grant reads empty, and prove deletion/revocation cannot
  cross tenants. The real-router matrix proves read/write access on the granted
  site, the same non-oracular `403` for another real site and a made-up id, and
  closed Contacts, Drive, Calendar, Tasks, Billing, CRM, Admin, and JMAP doors.
- **Real curl transcript:** a fresh local database and server on
  `127.0.0.1:8080` created two sites and a collaborator, then the collaborator
  listed exactly the granted site, renamed it, and published its home page.
  The unrelated site and every surrounding workspace surface returned the
  human `403`; `/sites/config` remained readable. The server was stopped and
  no production, email, DNS, or external AI service was contacted.
- **Verified:** touched Rust formatting; strict offline all-target Clippy for
  `alo-store` and `alo-jmap`; focused storage and real-router suites run more
  than once; full unfiltered `alo-store` (1,122 unit tests plus every
  integration/doc target) and `alo-jmap` (641 unit tests plus every
  integration/doc target) suites on a freshly migrated disposable PostgreSQL
  database; the real curl matrix; and `git diff --check`.
- **Cuts/flags:** this storage/authorization slice intentionally exposes no
  collaborator-management API or workspace administration. S2.03b owns the
  visible invite/revoke surface and its end-to-end browser exercise. A site
  editor is deliberately a restricted account rather than an additive role;
  giving ordinary workspace access requires removing the final site grant.
- **Next:** S2.03b, visible invite/revoke controls inside Sites with no trip to
  workspace administration.

## 2026-08-11 — S2.03b visible site collaborators

- **Shipped:** site owners now manage collaborators from a visible panel on
  the website itself: enter an email to invite, copy or refresh the one-time
  setup link while it is pending, see when the collaborator becomes active,
  and revoke access in one click with undo feedback. The collaborator accepts
  through a public password-setup page and then signs in normally.
- **Domain reflexes:** the surface follows Google Sites sharing by keeping
  collaborators beside the website, and Webflow's scoped site roles by making
  the grant belong to one site rather than the workspace. It never exposes an
  employee directory or sends the owner through workspace administration.
- **Security and isolation proof:** invite tokens are stored only as hashes,
  expire, are single-use, and cannot be accepted by an existing workspace
  account. Owner-only management, wrong-tenant invite/list/revoke refusal,
  restricted-route denial, and final-revoke account cleanup are covered at
  storage and real-router levels.
- **Real curl transcript:** against a freshly migrated disposable database and
  freshly built server on `127.0.0.1:8080`, the owner invited a collaborator,
  the public setup succeeded once and refused reuse, the collaborator renamed
  and published the granted website, workspace administration returned `403`,
  the owner listed the active collaborator and revoked access, and the next
  collaborator request returned `401`. The server was stopped afterward.
- **Verified:** touched Rust formatting; strict offline all-target Clippy for
  `alo-store`, `alo-identity`, and `alo-jmap`; full unfiltered tests for all
  three crates; wrong-tenant storage tests; real-router and unauthenticated
  route tests; two focused collaborator UI tests; TypeScript; focused ESLint;
  the production Vite build; real curl; and `git diff --check`.
- **Cuts/flags:** the invitation link is copied rather than emailed, so this
  slice contacts no real mail service. Revoke undo creates a fresh one-time
  link when needed; an already accepted collaborator gets a new restricted
  account only after accepting it. No production, DNS, or external AI service
  was contacted.
- **Next:** S2.04a, immutable version history with an atomic republish path.

## 2026-08-11 — S2.04a immutable version history with an atomic restore

- **Shipped:** every publish a website has ever had is now readable as a
  version history, two versions can be compared, and an earlier one can be put
  back on the internet. Storage: one expand-only migration adding
  `site_publishes.restored_from` (composite FK, same-tenant only) and a new
  `site_versions` store module — history newest-first with what each version
  froze (pages, languages actually frozen, collections, who published it,
  whether it is live), a single-version read, a metadata comparison, and the
  restore. Routes: `GET /sites/{id}/publishes`,
  `GET /sites/{id}/publishes/compare?from=&to=`, and
  `POST /sites/{id}/publishes/{publish}/restore`.
- **Restoring appends, it never re-points.** A restore copies the chosen
  publish — theme, language contract, every page snapshot, every collection
  snapshot — into a NEW publish recording where it came from, and flips the
  published-set pointer in one transaction. The rejected alternative (pointing
  the site back at the old publish id) is recorded in the module doc and the
  design note: two versions would share one identity, and the public cache key
  and visitor `ETag` are `<publish_id>:<path>`. The wire pass shows exactly
  that — each restore produced a new `ETag`.
- **The draft is never touched**, and neither is Base: a rollback of the
  website is not a rollback of the tenant's work in progress or of the rows a
  collection reads. Both are proved by test and on the wire.
- **Comparison is metadata only** — theme, default language, languages added
  or removed, pages added/removed/changed (naming which frozen fields differ,
  per page *and* language), and collections by name and row count. Section
  content belongs to a preview, which is S2.04b's.
- **Isolation proof:** storage tests refuse a foreign tenant and a foreign
  site on every path (history reads empty, version reads `None`, compare and
  restore are `NotFound`), prove an outsider cannot move a live site, and
  prove a real version id addressed through another site is invisible. The
  real-router suite proves `401` on all three routes and one identical `404`
  for another tenant's version, another site's version, and an invented id —
  with the refusal never echoing the foreign id.
- **Real curl transcript** (fresh admin on a local database, `alo-jmap` on
  `127.0.0.1:8080`, `alo-sites` on `127.0.0.1:8081`): publish v1 (hero
  "Bread & butter") → history of one, `current` naming it → edit the hero, add
  an About page, change the theme, publish v2 → history of two, newest first →
  compare v1→v2 = theme changed, home page `["sections"]`, About `added` →
  restore v1 → new id with `restoredFrom` = v1, history of three, compare v1
  vs the copy `identical: true`, the draft still reading "Sourdough, daily" →
  **the public host serves "Bread & butter" again and `/about` is 404**, then
  restoring v2 brings both back, each restore with a fresh `ETag`. Negatives:
  no token → `401` + `WWW-Authenticate`; unknown site, unknown version, and a
  compare end that does not resolve → `404`; a compare missing an end → `400`
  with this surface's own Problem detail; `?limit=abc` → the default list;
  `DELETE` on the list → `405`. `psql` after: four publish rows, two carrying
  `restored_from`, snapshot counts 1/2/1/2 as published. Fixture site deleted
  (cascade left zero publishes, the host went off the air) and both servers
  stopped. No production, email, DNS, or external AI service was contacted.
- **Verified:** `cargo fmt`; strict offline all-target Clippy for `alo-store`
  and `alo-jmap`, zero warnings; **full unfiltered `cargo test -p alo-store`
  and `cargo test -p alo-jmap`** green on the local docker Postgres (including
  the new 5 storage tests and 4 real-router tests, and the touched
  `sites_http` / `site_editor_role_http` suites re-run after the error-shape
  change); the curl matrix above; `git diff --check`.
- **Cuts/flags:**
  - No UI: the history surface, preview and one-click rollback are S2.04b's.
    The CHANGELOG line is written in the capability's voice because the API is
    the contract that shipped; the screen entry lands with S2.04b.
  - A restore is available to a restricted site collaborator, because the
    existing scoped-role middleware gates every `/sites/{id}` route on the
    per-site grant and a collaborator may already publish (S2.03a).
  - Comparison loads both versions' frozen sections to diff them (bounded by
    the 200-page cap × languages); a digest column would avoid it and is not
    worth a schema change until a real history gets slow.
  - Restoring copies snapshot rows rather than referencing them — deliberate,
    for the identity/cache reason above; snapshot retention stays unbounded,
    as it has been since S1.08.
  - The defensive "version with nothing frozen" refusal is driven by a raw
    SQL delete in its test: no store call can reach that state, which is the
    point of immutability.
- **Next:** S2.04b, the visible history surface with preview and one-click
  rollback.

## 2026-08-11 — S2.04b the version history you can see, preview and roll back

- **Shipped:** a website's publish bar now carries **Version history**
  (`web/src/sites/HistoryView.tsx`, route `/sites/{id}/history`): every
  version listed by **date** — never by publish id, which nobody recognises —
  with the live one chipped, and the selected version rendered beside the list
  exactly as visitors saw it (its own pages, its own frozen theme, its own
  languages, at desktop or phone width). Putting one back online is one click,
  and the result banner names the version, links to the site's address and
  offers **Undo**, which restores the version that was live before it.
- **Preview needed a backend half, and it reads only frozen rows.** Two
  additive routes: `GET /sites/{id}/publishes/{publish}/pages` (what that
  version froze, one entry per page *and* language) and
  `GET /sites/{id}/publishes/{publish}/pages/{page}/preview?locale=` (one
  frozen page as a complete self-contained `text/html` document, stylesheet
  and images inlined, `Cache-Control: no-store`). It renders through the same
  library the public service uses, from that publish's theme and that
  publish's frozen collection snapshots — never the draft and never today's
  Base rows, so what the owner looks at is what restoring would put back. The
  one present-tense value is the site's *name*, which a publish does not
  freeze; recorded in the design note. Store addition: `site_publish(site,
  publish)` — one version's frozen envelope, tenant+site scoped.
- **New module, not a bigger one:** `alo-jmap/src/site_version_preview.rs`
  renders history as a document; `site_versions.rs` stays the JSON history
  surface. The page-list route guards on the version read first, so an unknown
  or foreign version is a `404` rather than an empty list that reads like a
  version with no pages.
- **The screen obeys the interface laws.** Restore executes on the click
  instead of behind a confirmation because it is reversible by construction
  (law 7: undo over confirm) — the server appends a copy, so the
  previously-live version is still there for Undo. Under the heading the
  screen states the draft is untouched; against the live version it lists what
  would change (theme, languages, pages coming back/going away/changing) from
  the compare endpoint. Domain reference: Docs/Wix version history — dates
  left, the version rendered right, restore on what you are looking at.
- **Isolation proof:** store tests refuse the new envelope read to a foreign
  tenant and to another site of the same tenant; the real-router suite proves
  `401` on both new routes, one identical `404` for another tenant's version,
  another site's version and an invented id, and that a refusal to an outsider
  never carries the foreign id, the foreign content, or an HTML content-type.
- **Real curl transcript** (fresh admins on the local docker Postgres,
  `alo-jmap` on `127.0.0.1:8080`, `alo-sites` on `127.0.0.1:8081`,
  `SITES_DOMAIN=alosites.test`): publish v1 (hero "Bread & butter") → edit the
  hero to "Sourdough, daily", change the theme to `midnight`, publish v2 →
  both versions' `/pages` list Home and About → **v1's preview renders "Bread
  & butter" with the light frozen palette (`--bg: #ffffff`) while v2's renders
  "Sourdough, daily" with the dark one (`--bg: #0f1a2b`)**, `content-type:
  text/html; charset=utf-8`, `cache-control: no-store` → the draft preview
  still reads "Sourdough, daily" → restore v1 → the public host serves "Bread
  & butter" again and history reads three versions with `restoredFrom` on the
  newest → the Undo click (restore v2) → the public host serves "Sourdough,
  daily" again, the draft never moved. Negatives: no token → `401` on both
  routes; other tenant, unknown version, unknown page → `404` with
  "no such version of this website" / "no such page in this version" / "no
  such site"; `?locale=fr` on a version that never froze French → `200`
  rendering the frozen English page (`<html lang="en">`); `DELETE` on the page
  list → `405`. `psql` after: 4 publishes, 2 carrying `restored_from`, 8
  snapshot rows. Fixture site deleted (publishes cascaded to zero, the host
  went off the air) and both servers stopped. No production, email, DNS or
  external AI service was contacted.
- **Verified:** `cargo fmt`; strict offline all-target Clippy for `alo-store`
  and `alo-jmap`, zero warnings; **full unfiltered `cargo test -p alo-store`
  and `cargo test -p alo-jmap`** green on the local docker Postgres; the web
  gate clean (`npx tsc --noEmit`, `npx eslint` on every changed file, `npm run
  build`) with the sites + i18n vitest suites green including the new
  `SiteHistory.test.tsx` (list by date, preview of the selected version,
  one-click restore, Undo restoring the *previously live* version and not the
  copy, the never-published empty state, and a verbatim server refusal).
- **Cuts/flags:**
  - The preview shows one page at a time through a page picker rather than a
    navigable mini-site: links inside the frozen document are not rewritten to
    the preview route (the iframe is sandboxed without `allow-top-navigation`,
    so a click simply does nothing). A version whose pages a person wants to
    walk is one selection per page; noted rather than built.
  - The compare summary is metadata (S2.04a's contract): it names pages that
    would change, not which words. A visual before/after diff is not this
    item's, and S3.01a's diff work is the natural place for it.
  - A restricted site collaborator can open the history and restore, as they
    could already publish (S2.03a) — the per-site grant middleware gates every
    `/sites/{id}` route unchanged.
  - fr/nl strings written with the English copy, as the parity test requires;
    `UNTRANSLATED` stays empty.
- **Next:** S2.05a, the scheduled-publishing model (tenant-scoped
  schedule/cancel/claim with concurrency and wrong-tenant tests).

## 2026-08-11 — S2.05a the model behind "publish this on Monday at 09:00"

- **Shipped:** scheduled publishing as a **model only** — migration
  `0302_site_publish_schedules.sql` plus `platform/alo-store/src/site_publish_schedule.rs`
  (new module, new `SitePublishScheduleId`). A row is an *intention*, never a
  version: site ref, `publish_at` (UTC), status
  `scheduled | publishing | published | cancelled | failed`, the user whose
  account door the publish runs through, attempt count, and — once it has run
  — the `site_publishes` row it produced or the verbatim reason it refused.
  Terminal rows are retained, so the tenant reads "published on Monday" or
  "it could not publish because …" instead of watching an entry vanish.
- **Tenant surface** (`AccountStore`): `schedule_site_publish` (future-only,
  at most a year ahead, unknown/foreign site → `NotFound`),
  `site_publish_schedule` (the pending one), `site_publish_schedules`
  (bounded history), `cancel_site_publish_schedule`. Rescheduling **moves the
  same row** — the id survives, so a surface watching one schedule keeps
  watching it — and the site row is locked first, so two editors scheduling at
  the same instant produce one intention rather than colliding on the partial
  unique index that admits a single `scheduled`/`publishing` row per site.
- **Worker surface** (`Store`, cross-tenant like the form-notification sweep):
  `claim_due_site_publishes` marks due rows `publishing` in the statement that
  reads them (`FOR UPDATE SKIP LOCKED`), returning tenant + the scheduling
  user's id so the publish runs through *their* account door and the resulting
  version records them as its author; `finish_site_publish_schedule` (refuses a
  version that is not that schedule's site's, tenant-scoped) and
  `fail_site_publish_schedule` (reason bounded to 500 chars) close a claim.
- **Two failure modes, deliberately different.** A worker that dies leaves a
  `publishing` row: the claim re-offers it once the claim is ten minutes stale
  and, after three attempts, writes it off as `failed` where the tenant can see
  it. A publish that *refuses* (no home page, a collection that no longer
  resolves) is terminal on the first attempt — ten minutes cannot change the
  site's content, so the refusal is kept verbatim for the owner to act on.
  Rejected alternative, recorded in `docs/design/sites.md`: deleting the row on
  claim as the scheduled-*send* sweeper does — a mail send is invisible until
  it lands, a website publish is something the owner watches for afterwards.
- **Isolation proof:** another tenant cannot read the schedule (pending read
  and history both empty), reschedule it, cancel it — including by naming the
  foreign schedule under a site they do own — finish it, or fail it; every
  refusal is the same `NotFound` an invented id gets. A version belonging to
  another site (or another tenant) cannot be pinned onto a schedule. Deleting
  the site cascades its schedules away.
- **Concurrency proof:** four due schedules, two `claim_due_site_publishes`
  calls issued with `tokio::join!` — the union is exactly four claims with no
  id appearing twice. Then one claim is aged: attempt 2, aged again: attempt 3,
  aged again: not re-offered, and the row now reads `failed` with the
  interrupted message, which frees the site to be scheduled again.
- **Verified:** `cargo fmt`; strict offline all-target Clippy for `alo-store`,
  zero warnings; the new `site_publish_schedule_tenancy` suite (4 tests) green
  on the local docker Postgres, inside the **full unfiltered `cargo test -p
  alo-store`** — 115 suites, 1739 tests, zero failures. No web or HTTP code was
  touched, so the web gate does not apply to this item. Mutation check: dropping the `tenant_id` predicate from
  `fail_site_publish_schedule` turns the wrong-tenant test red ("expected
  NotFound, got Ok(())") — the tests fail for the right reason. No production,
  email, DNS or external AI service was contacted.
- **Cuts/flags:**
  - No HTTP, no worker loop, no UI: `/sites/{id}/schedule`, the sweep that
    actually runs due publishes, and the visible schedule control with its
    local-time explanation are all S2.05b, as the queue splits them. Nothing a
    user can do changed yet, so no CHANGELOG line was written — it lands with
    the screen in S2.05b, in the capability's voice.
  - The scheduling user's account door is captured at schedule time. If that
    user is deleted or loses their per-site grant before the moment arrives,
    the publish will refuse and the reason lands on the row; S2.05b's worker
    should surface it rather than retry.
  - Attempts are consumed only by *interrupted* claims, so the visible
    `attempts` on a schedule is a crash counter, not a retry-of-refusal
    counter.
  - The suite serializes its four tests behind a mutex and deletes its sites
    afterwards: the claim is cross-tenant by design, so two tests sweeping at
    once would steal each other's due rows.
- **Next:** S2.05b, the scheduled-publishing service and UI (visible schedule
  control, local-time explanation, cancel/reschedule, worker execution, wire
  transcript).

## 2026-08-12 — S2.05b the website that publishes itself at the chosen moment

- **Shipped:** the visible half of scheduled publishing, on top of S2.05a's
  model. Two new alo-jmap modules and one new web surface, with no store
  changes at all: `site_schedule.rs` (routes) is the *intention* surface,
  `site_publish_worker.rs` (the sweep) is its execution, and
  `web/src/sites/SchedulePublish.tsx` is the control, mounted directly under
  the publish bar because publishing later is the same decision as publishing
  now with a moment attached.
- **Routes** (`/sites/{id}/schedule`, registered in `server.rs`, covered by the
  existing per-site-grant middleware because the template starts with
  `/sites/{id}`): `GET` answers `{schedule, history}` — the pending intention
  or `null`, plus a bounded history; `POST {"publishAt"}` both schedules and
  reschedules (the store moves the same row, so the id survives); `DELETE
  /sites/{id}/schedule/{schedule}` calls one off. `publishAt` is an RFC 3339
  **instant** in both directions: a caller may send `+02:00` and every answer
  reports UTC, so no wall-clock string ever travels without its zone.
- **The sweep** runs every 30 seconds from `main.rs`, the form-notification
  posture: claim due rows, then publish each **through the scheduling user's
  own account door**, so a scheduled publish has the same tenant scope and the
  same recorded author as the editor's button. It splits the two failure modes
  deliberately — a store *refusal* (`Conflict`/`Validation`/`NotFound`) is
  written to the row verbatim and is terminal, while an *infrastructure*
  failure (`Db`/`Blob`/`Migrate`/`Crypto`) leaves the claim standing so
  S2.05a's stale-claim path retries it and, after three attempts, fails it
  visibly. Only the coarse error reaches a log; no site content, no addresses.
- **The screen** states the moment in the reader's own time (`Intl`, `dateStyle
  full`) and **names their time zone beside the picker**, because someone
  scheduling a launch from another country has to see which nine o'clock they
  picked. The `datetime-local` field is pre-filled with tomorrow at 09:00
  rather than left empty (the S1.30b lesson: never a blank field with a
  disabled button), and what is sent is `new Date(value).toISOString()`, so the
  clock the browser showed and the instant the server stores cannot disagree.
  Scheduling, moving and calling off are one click each with no confirmation —
  none of them touches what is online (ux law 7) — and while an intention is
  pending the panel polls once a minute, so "publishes on …" becomes "published
  itself on …" without a reload. A refusal is shown in the server's own words.
- **Real curl transcript** (fresh admin on the local docker Postgres,
  `alo-jmap` on `127.0.0.1:8080`, `SITES_DOMAIN=alosites.test`): no token →
  `401 missing or invalid bearer token` on all three routes and `405` on `PUT`
  → nothing scheduled reads `{"schedule": null, "history": []}` → a past moment
  → `422 a scheduled publish must be in the future`; `"next monday"` → `422
  publishAt must be a date and time with a time zone, for example
  2026-09-01T09:00:00+02:00` → **`publishAt=2026-08-13T09:00:00+02:00` comes
  back as `2026-08-13T07:00:00Z`** (`status: scheduled`, `attempts: 0`) → moved
  a day later, **same id**, `updatedAt` moved → `psql`: one row, `scheduled` →
  `DELETE` → `cancelled` with `finishedAt`, a second `DELETE` → `422 this
  scheduled publish has already finished`, an invented id → `404 no such
  scheduled publish for this website`, and the site is still `draft` (an
  intention is not a publish) → then two sites scheduled 20 seconds out and
  **the server's own sweep left alone to run**: `INFO scheduled publish sweep
  published=1`, the site with a home page reads `live` with the version the
  schedule names (`publishId` on the row, `attempts: 1`), and the site with no
  pages reads `failed` with `lastError: "site has no pages to publish"`,
  `attempts: 1`, still `draft`. Outsider tenant: `404 no such site` on both the
  read and the write. Fixture sites deleted; `psql` after: zero schedule rows
  left. No production, email, DNS or external AI service was contacted.
- **Verified:** `cargo fmt`; strict offline all-target Clippy for `alo-jmap`,
  zero warnings; the new `site_schedule_http` suite (4 tests) green — 401s,
  the schedule/move/cancel arc, the cross-tenant matrix, and the sweep test
  that publishes one site and refuses another — inside the **full unfiltered
  `cargo test -p alo-jmap`**; the web gate clean (`npx tsc --noEmit`, `npx
  eslint` on every changed file, `npm run build`) with the sites + i18n vitest
  suites green including the new `SchedulePublish.test.tsx` (the proposal in
  the field, the instant that is sent, the zone that is named, the pending
  sentence, the call-off, the verbatim refusal, and the "it published itself"
  outcome). `alo-store` was not touched, so its suite is unchanged.
- **Cuts/flags:**
  - The panel polls once a minute rather than holding a live connection: a
    website going live is not a chat message, and a minute of latency on a
    screen the person may not even be looking at buys nothing worth a socket.
  - There is no "publish these pages only" scheduling: a scheduled publish is
    the same whole-site freeze the button makes, which is why everything saved
    before the moment goes live with it — the screen says so.
  - The sweep is a per-process tick. Two alo-jmap replicas are safe (the claim
    is `FOR UPDATE SKIP LOCKED`), but a deployment that is down over a chosen
    moment publishes late, not never: the first tick after it comes back
    claims the overdue row. Worth stating in operations docs at the wave
    review.
  - fr/nl written with the English copy, as the parity test requires.
  - No new route *prefix* (`/sites` is already proxied), so the production
    Caddyfile needs nothing for this item.
- **Next:** S2.06a, password-protected pages (hashing, the anonymous
  challenge/session gate, cache-safe responses, rate limiting, security tests).

## 2026-08-12 — S2.06a the page that is online, but only for people with the password

- **Shipped:** the whole gate behind "publish this page, but not to everybody"
  — model, edit routes, and the anonymous challenge on the public service.
  Migration `0303_site_page_passwords.sql` plus a new store module
  (`site_page_protection.rs`, the tenant door) and its public counterpart
  (`site_public_protection.rs`, the two questions `alo-sites` may ask), the new
  `site_protection.rs` routes in alo-jmap, and `serve/unlock.rs` in alo-sites.
  No web work: the protect/remove screen is S2.06b.
- **Two design decisions carry the item, both recorded in
  `docs/design/sites.md`.** First, **protection is live state, not part of a
  publish**: a password set, changed or lifted takes effect on the very next
  request. Rejected alternative — freezing it into `site_page_snapshots` with
  everything else a publish freezes: consistent with the rest of the model, but
  it would leave a leaked password working until the owner happened to
  republish, which is the wrong failure direction for a security control.
  Second, the row hangs off the **site**, not the page, because deleting a
  draft page does not unpublish its snapshot: cascading the protection away
  with the draft would silently open a page the internet is still being served.
  One password covers a page in every language, since every locale snapshot
  shares the page identity.
- **The secret's whole life.** The plaintext is hashed with argon2id
  (`$argon2id$v=19$m=19456,t=2,p=1$…`, on a blocking thread) and never stored,
  never logged, and never returned by any read on any door: a forgotten
  password is replaced, not recovered. From the hash the store derives an
  opaque `version`; that — never the hash — is what the public service holds.
- **Sessions are signatures, not rows.** A correct password mints an
  HMAC-signed cookie over *(public host, page id, protection version, expiry)*,
  twelve hours, `HttpOnly; Secure; SameSite=Lax`. Nothing about the visitor is
  stored anywhere — no session table, no identifier — and each of the three
  bindings has a test: another host cannot present it, another page cannot be
  opened with it, and changing the password rotates the version, which is what
  makes "change the password" a real revocation (the dead cookie is cleared on
  the way out). The signing key is derived from the existing sites secret under
  a fixed label, so unlock signatures and analytics visitor hashes cannot be
  confused and no new deployment secret is needed.
- **Cache-safe by construction.** The `401` unlock screen is `no-store`,
  `Vary: Cookie`, no `ETag`, `noindex`, and carries the site's theme but none
  of the page's content — not even its title. The unlocked page is `private,
  no-store` with `Vary: Cookie` and, deliberately, no validator: the ordinary
  `public, max-age=60` answer would invite a shared cache to hand one visitor's
  copy to the next person. Protected pages are also left out of `sitemap.xml`.
  The `401` carries `WWW-Authenticate: Form` — RFC 9110 requires the header,
  and no browser prompts for an unknown scheme, so the visitor sees our screen
  rather than a native credential dialog.
- **Guessing costs.** The unlock `POST` is the only write a page path accepts
  (anything else is still `405 Allow: GET, HEAD`), and it is rate-limited on
  its own budget — eight tries per ten minutes, a second limiter instance so
  contact-form traffic cannot spend the budget standing between a guesser and a
  page. An unprotected or unknown page pays the same argon2 cost on the verify
  path and the result is discarded, so timing says nothing about which pages
  carry a password.
- **Real curl transcript** (fresh admin on docker `alo-pg`, real PKCE token,
  debug `alo-jmap` on `127.0.0.1:8080` and debug `alo-sites` on
  `127.0.0.1:8081`, `SITES_DOMAIN=alosites.test`): no token → `401` on all
  three routes → `{"pages":[]}` and `{"protected":false}` → `"short"` → `422 a
  page password must be at least 8 characters`; `{"secret":…}` → `422 password
  must be a string, for example {"password": "…"}` (never echoing the body);
  `"          "` → `422 a page password must be more than spaces`; an unknown
  page → `404` → the good password → `{"protected":true,"pageId":…}`, listed
  once, home page still `{"protected":false}` → publish → **the public side**:
  `/` → `200 public, max-age=60` with an `ETag`; `/prices` → `401`,
  `www-authenticate: Form`, `cache-control: no-store`, `vary: Cookie`, **no
  ETag**, `<title>This page is protected — Wire Roastery</title>`, zero
  occurrences of the page's own heading → wrong password → `401` with *"That
  password does not open this page."* and no cookie → right password → `303
  location: /prices` + `alo_site_unlock_<page>=…; Max-Age=43200; Path=/;
  HttpOnly; Secure; SameSite=Lax` → the page with that cookie → `200 private,
  no-store`, `vary: Cookie`, no ETag, the heading present → tampered cookie →
  `401` → `sitemap.xml` lists only `/` → `POST /` → `405 allow: GET, HEAD` →
  password changed → the old cookie → `401` + `Max-Age=0` clearing → eight
  guesses `401` then `429 retry-after: 598`, and the *right* password from the
  same address still `429` while another visitor gets `303` → password lifted →
  `/prices` → `200 public, max-age=60` with its ETag back. `psql`: the stored
  row is an argon2id hash containing no fragment of the password. Fixture site
  deleted; the host answers `404` after. No production, email, DNS or external
  AI service was contacted.
- **Verified:** `cargo fmt` (only this item's files moved — the rustfmt-version
  delta that plagued S1.10–S1.14 is gone); strict offline all-target Clippy for
  `alo-store`, `alo-sites` and `alo-jmap`, zero warnings; **full unfiltered
  suites green** — `cargo test -p alo-store` (every suite, including the new
  3-test `site_page_protection_tenancy`), `cargo test -p alo-sites` (14 crate
  tests incl. the new `unlock` session suite, the new 3-test `protected_pages`
  integration suite, and `render_rules` extended with the unlock screen in en/
  fr/nl), and `cargo test -p alo-jmap` (63 suites, incl. the new 3-test
  `site_protection_http`). Two mutation checks, both red for the right reason:
  dropping the protection version from the session signature turns "changing
  the password ends the session" red, and opening the gate unconditionally
  turns the two gate tests red. No web code was touched, so the web gate does
  not apply to this item.
- **Cuts/flags:**
  - Protection is read once per page request (an indexed primary-key-prefix
    read on the resolved site) rather than cached with the publish — the price
    of "a password holds on the next request". If page traffic ever makes that
    read visible, the answer is a short-TTL cache keyed by publish, not moving
    protection back into the snapshot.
  - Only whole pages can be protected, not blog posts or the site as a whole; a
    site-wide password would be S2.06b+ if anyone asks, and posts have no
    protection model yet.
  - There is no "share this link and skip the password" token: a page is either
    public or asks. That is deliberate — a second secret with different
    revocation rules is a second thing to get wrong.
  - The unlock screen deliberately shows no page title and no navigation, so a
    visitor who lands there by accident cannot tell what the page is about.
    S2.06b's UI copy has to explain that to the owner, who might expect the
    page's own name.
  - fr/nl chrome strings ship with the English ones, as the parity rule
    requires. No new route *prefix* (`/sites` is already proxied), so the
    production Caddyfile needs nothing for this item.
- **Next:** S2.06b, the password UI (visible protect/remove controls, the clear
  public-preview state, and the accessible visitor unlock screen).

## 2026-08-12 — S2.06b the screen that says who can open a page

- **Shipped:** the owner-facing half of the password gate S2.06a built. A new
  `PagePassword` panel in the page editor (`web/src/sites/PagePassword.tsx`),
  the four client methods it needs (`pagePassword`, `protectedPages`,
  `setPagePassword`, `removePagePassword` on the existing `/sites/*` routes —
  no new server route in this item), a lock badge per row in the site's page
  list, and a line on the editor's preview saying visitors are asked for the
  password first. English, French and Dutch strings for all of it. One small
  Rust change on the public side: the unlock screen's refusal is now tied to
  the field it is about (`aria-describedby` + `aria-invalid`), so a visitor who
  moves back to the field after a wrong password hears why — `role="alert"`
  alone only announces on arrival.
- **The panel says who can READ the page, never what a setting is called.**
  The state line is "anyone on the internet can open this page" or "only people
  with the password can — set on <date>", and under it what the visitor
  actually meets: an unlock screen carrying nothing of the page, not even its
  title. That sentence is the reason it is there — S2.06a's own journal flagged
  that an owner expecting the page's name on the unlock screen would think it
  broke, and the copy is where that gets answered.
- **Three properties the screen owes the model.** (1) It never renders a stored
  password, because no read on the server answers one; the field is always
  empty, and a show/hide toggle stands in for a confirm field so a typo is seen
  before it is saved rather than after a visitor cannot get in. (2) It refuses
  only an EMPTY field itself — length and whitespace are the store's rules and
  its `422` sentence is shown verbatim, so two doors never disagree. (3)
  Lifting a password arms first and acts on the second click, like taking a
  live site off the air; setting and changing act at once, because they are
  reversible and disclosure is not (ux-principles law 7).
- **A protection that could not be read says so.** When the load fails the
  panel shows "not known right now" and hides the protect button, rather than
  falling back to "anyone can open this page" — the reassuring guess is the
  dangerous one, and a test pins it.
- **Verified:** `npx tsc --noEmit`, `npx eslint` on every changed file, and
  `npm run build` — all clean. `npx vitest run src/sites` green: 81 tests, 11
  of them new (7 in `PagePassword.test.tsx`; 3 in `PageEditor.test.tsx` running
  the REAL client through the fake-network harness, which pins the wire
  spelling — `GET`/`PUT {"password"}`/`DELETE` on
  `/sites/{id}/pages/{pid}/password`; 1 in `SitesModule.test.tsx` proving the
  list marks only the protected row and reads the whole list in ONE call). The
  four unhandled rejections the suite prints are pre-existing — confirmed by
  stashing this work and re-running (same four on `main`). Rust: `cargo fmt -p
  alo-sites`, strict offline all-target Clippy clean, `cargo test -p alo-sites`
  green (66 tests across the crate's suites, with `DATABASE_URL` pointed at docker
  `alo-pg` on 5432 — note the crate's own default is 5433).
- **Mutation checks, both red for the right reason:** dropping the arming step
  from "remove the password" turns the two-click test red, and the render
  assertions go red without the new aria attributes.
- **Wire check:** the debug `alo-jmap` (already built by S2.06a, unchanged by
  this item) on `127.0.0.1:8080` against docker `alo-pg`, curled with no token
  on exactly the paths and verbs the new client spells: `GET
  /sites/{id}/passwords` → `401`, `GET|PUT|DELETE
  /sites/{id}/pages/{pid}/password` → `401` — the handler, not a `404` or a
  `405`, so no path typo can hide in the client. The full authenticated
  transcript for these routes is S2.06a's, and nothing on the server moved.
  Server killed afterwards. No production, email, DNS or external AI service
  was contacted.
- **Cuts/flags:**
  - The panel is per PAGE, in the page editor. There is no site-wide "password
    the whole site" control, because there is no site-wide model behind one
    (S2.06a's own cut). If an owner wants every page closed they set a password
    per page, which the list makes visible but tedious past a handful of pages.
  - The badge in the page list is read once per visit to the site screen; a
    password set in the editor shows there on the next load of that screen, not
    live. One read for the list was the trade against a request per row.
  - No "here is a link that skips the password" affordance, deliberately —
    S2.06a refused that model and the UI does not invent one.
  - No new route prefix, so the production Caddyfile needs nothing for this
    item.
- **Next:** S2.07a, the image presentation model (crop rectangle, focal point
  and alt text on image-bearing sections, with backwards-compatible
  validation).

## 2026-08-12 — S2.07a how an image is framed

- **Shipped:** the model half of image presentation. `SiteImage` — the one
  image reference all four image-bearing sections share (hero, text_image,
  gallery, team) — gains three optional props: a `crop` rectangle, a `focal`
  point, and a `decorative` flag, with validation, a golden fixture, a
  storage round-trip against real Postgres, and a wire transcript. No new
  route, no new column, no migration: this is the `sections` JSONB schema
  growing three keys it already had room for.
- **Basis points, not floats and not pixels.** Crop and focal are stored as
  ten-thousandths of the source width/height with a top-left origin
  (`x_bp`/`y_bp`/`width_bp`/`height_bp`), the same unit and suffix
  `vat_rate_bp` uses. Pixels would break the moment a photo is re-uploaded at
  another resolution; floats would cost `Eq` on `Section` and every type above
  it, and make goldens fragile. `u16` with a validated ceiling of 10 000 keeps
  the whole family `Copy + Eq` and exact.
- **Cropping is presentation, never destruction.** The tenant's blob keeps
  every pixel; a crop is a rectangle over it. Re-framing a photo can always be
  undone, and the derivative pipeline (S2.07b) reads the rectangle rather than
  a cut-down file.
- **The two props can never contradict each other.** Validation refuses a
  rectangle that leaves the image, one with less than 1% extent on an axis
  (`MIN_CROP_EXTENT_BP` — the degenerate case that would ask S2.07b to blow a
  handful of pixels up to full width), a focal point off the image, and — the
  rule worth having — a focal point that sits *outside its own crop*.
- **`decorative` is the missing half of alt text.** A blank `alt` used to mean
  "decorative" by convention, which made "nothing to describe" and "nobody has
  written it yet" the same stored value — and S2.07c has to tell them apart to
  ask for alt text at all. Blank `alt` **with** the flag is deliberate; blank
  `alt` **without** it is what `SiteImage::needs_alt_text()` reports. Setting
  both is refused: the renderer emits `alt=""` for a decorative image, so the
  alt text would silently vanish.
- **Backwards compatible by construction, and pinned as such.** All three
  props are `#[serde(default, skip_serializing_if …)]`, so a section stored
  before they existed parses unchanged, reads as whole-image-centred
  (`crop_or_full` / `focal_or_center`), and **re-serializes with no new keys**
  — proven byte-for-byte by the twelve pre-existing goldens still passing
  untouched, and named explicitly by two new tests. The crop object is closed
  like everything else: an invented `zoom` is a 422, not a silently dropped
  key.
- **`Section::images()` is new and `image_blob_ids()` now derives from it.**
  The exhaustive match that made a new section variant fail to compile until
  it declares its images now yields the whole `SiteImage`, which is what
  S2.07b's derivative pipeline needs; the blob-id view is the same set,
  reduced. A test asserts the two agree.
- **The editor carries the frame through without offering it.** No prop form
  edits a crop yet (that is S2.07c), so `web/src/sites/sectionDrafts.ts` was
  the one place that would silently unframe every photo on the next save —
  the same hazard its own header comment already names for `form_id`. The
  three props now ride through `draftImage`/`optImage`/`reqImage`, and a new
  `PageEditor.test.tsx` case runs the REAL client and editor over the fake
  network to prove that editing a heading leaves the crop and focal point on
  the wire exactly as they arrived.
- **Verified:** `cargo fmt`; strict offline `cargo clippy -p alo-store -p
  alo-sites -p alo-jmap --all-targets` — zero warnings; the whole `alo-store`
  suite green (the run reaches and completes doc-tests, which a failing test
  binary would have stopped); `cargo test -p alo-store --lib site_model` → 20
  tests, 11 of them new; the new `tests/site_image_presentation.rs` → 2 tests
  green against docker `alo-pg`; `tests/site_sections.rs` → 5 green including
  the two new backwards-compatibility tests; `cargo test -p alo-sites` → 66
  green across the crate's suites. Web: `npx tsc --noEmit`, `npx eslint` on
  the three changed files, `npm run build` — all clean; `npx vitest run
  src/sites` → 82 passed (81 before, 1 new). The four unhandled rejections the
  web suite prints are the same pre-existing ones journalled under S2.06b.
- **Mutation check, red for the right reason:** short-circuiting
  `check_image_geometry` and disabling the decorative rule turns exactly the
  three new rule tests red (`crop_rules_…`, `a_focal_point_…`,
  `decorative_and_missing_alt_text_…`) and nothing else.
- **Wire transcript** (fresh admin `owner@framing-check.test` on docker
  `alo-pg`, debug `alo-jmap` on `127.0.0.1:8080`, password grant token; the
  server was killed before it started and again after): no token → `401`;
  `PUT /sites/{id}/pages/{pid}/sections` with a cropped image and a decorative
  one → `200` echoing the canonical envelope; `GET` the page → the crop and
  focal point came back off disk unchanged, and the flag nobody set is still
  absent; then every refusal, each naming its own rule — crop leaving the
  image → `422 image crop must stay inside the image (x + width and y + height
  may not exceed 10000 basis points)`; zero-area crop → `422 … at least 100
  basis points of the image`; focal off the image → `422 … within 10000 basis
  points on each axis`; focal outside its crop → `422 image focal point must
  lie inside the crop`; alt text on a decorative image → `422 a decorative
  image must have empty alt text`; an invented `zoom` prop → `422 unknown
  field 'zoom', expected one of 'x_bp', 'y_bp', 'width_bp', 'height_bp'`. A
  re-read after all six proved none of them changed a stored byte. The
  editor's single-section save (`PUT …/sections/0`) keeps the frame too, and
  the authenticated draft preview still renders `200` with the image and its
  alt text. No production, email, DNS or external AI service was contacted.
- **Cuts/flags:**
  - **The renderer does not honour the crop yet.** A framed image still
    renders as the whole image — applying the rectangle belongs with the
    derivative pipeline in S2.07b, and a CSS-only half-measure now would be
    rewritten next item. Nothing produces crops yet (the editor lands in
    S2.07c), so no published page is currently mis-framed.
  - No UI edits a crop, a focal point or the decorative flag in this item, by
    the queue's own a/b/c split. The web change is pass-through only.
  - `decorative` reclassifies existing blank-alt images as "alt not written
    yet" rather than "decorative". That is the honest reading of the stored
    data — nobody ever said which they meant — and it is what makes S2.07c's
    prompt possible. The renderer's output is unchanged either way.
  - CHANGELOG untouched: schema and validation only, nothing a user can do
    changed yet. It lands with S2.07b/c.
  - No new route prefix, so the production Caddyfile needs nothing for this
    item.
- **Next:** S2.07b, responsive images — the safe derivative pipeline and the
  published `srcset`/`sizes`, with byte/cache/XSS tests and public goldens.
  It is the item that makes the crop and focal point visible.

## 2026-08-12 — S2.07b the photo arrives at the size the screen needs

- **Shipped:** responsive images end to end on the public site. Every section
  image now renders `srcset` over a fixed three-rung ladder (480/960/1440 px)
  with a `sizes` attribute taken from the slot it sits in, and `alo-sites`
  serves those derivatives — decoded, **cropped**, resized and re-encoded from
  the tenant's own blob. New: `src/images.rs` (the URL grammar),
  `src/serve/derivative.rs` (the pipeline), a derivative cache in
  `serve/cache.rs`, the variant set on `RenderedSite`, and one new dependency
  (`image`, pure Rust, codecs limited to jpeg/png/webp).
- **One grammar, three readers, no drift.** `/assets/img/<blob>/w960` and
  `/assets/img/<blob>/c<x>-<y>-<w>-<h>/w960` are written by the renderer,
  collected into the servable set by `RenderedSite::build` from the same
  lenient section read, and parsed back by the service — all three out of
  `alo_sites::images`. A unit test walks every candidate the renderer emits
  back through the parser and asserts it says what the section said.
- **Nothing is decoded that the publish did not already promise.** The
  requested derivative is checked against the served publish's own variant set
  *before* any read or decode, so the resize pipeline can only be asked for the
  handful of URLs the site's own HTML names. A width off the ladder, a frame
  nobody published, an unreferenced blob, `w0480`, `//w480` and
  `../../etc/passwd/w480` are all the site's themed 404. This is the
  difference between an image service and a CPU amplifier pointed at its own
  origin.
- **Two independent gates, and the mutation check proved they are
  independent.** Forcing `serves_variant` to `true` turned
  `only_the_derivatives_the_publish_references_exist` red — and left
  `one_hosts_derivatives_are_never_reachable_from_another` **green**, because
  the store read behind it is scoped to the resolved site's tenant. Membership
  bounds the work; the tenant scope bounds the bytes. Neither is carrying the
  other.
- **Bounded decoding, on the blocking pool.** A source over 16 MB is never
  decoded; the header is read first and a canvas over 120 megapixels is
  refused (a 70-byte PNG claiming 40 000 × 40 000 is a test, and it is
  refused); allocation is capped; and `derive` runs in `spawn_blocking`, so a
  slow or panicking decoder costs one request instead of the runtime — a join
  error still answers with the original bytes.
- **Three rules keep the answer honest.** Never upscale (a rung wider than the
  source serves the source). Never grow the payload (a derivative that came
  out no smaller than its source is dropped) — *unless* the frame differs,
  where the crop is the whole point and correctness outranks bytes. And a
  source this build cannot decode (SVG, GIF, AVIF, ICO) serves its original
  bytes under the derivative path, so a vector logo in a gallery still shows.
  Encoding is chosen by transparency, not by what arrived: alpha → PNG,
  otherwise JPEG q82.
- **The crop is now visible — closing S2.07a's flag.** A cropped image's `src`
  fallback is the widest **derivative**, never the original: the original is
  the picture before the owner framed it, and a client ignoring `srcset` must
  not be shown what was cropped away. The served crop is pixel-checked in the
  wire suite (a photo that is red on the left and blue on the right comes back
  blue throughout at the right-half frame, and still red-then-blue unframed).
- **Cache contract unchanged in shape, extended in reach.** A derivative is
  immutable per path, so it carries the same `"img:<key>"` `ETag`,
  `public, max-age=3600`, `nosniff` and the SVG-defanging CSP as the original,
  and honors `If-None-Match` with an empty 304. Derivatives are cached in
  memory keyed by **site id + path** (a blob id is unique only inside a
  tenant) under a 64 MB byte bound; the "declined" answer is cached too, so an
  SVG is not re-examined per request.
- **Verified:** `cargo fmt --check` clean; strict offline `cargo clippy -p
  alo-sites -p alo-jmap --all-targets` — zero warnings; whole `alo-sites`
  suite green, **92 tests across 15 binaries** (66 before): 19 lib (11 of them
  the new grammar tests), the new `responsive_images.rs` 11 and
  `serve_derivatives.rs` 6, plus 4 new rules tests in `render_rules.rs` (20).
  The alo-jmap sites bank is green too — `sites_http` 21, `site_versions_http`
  6, `site_protection_http` 6, `sites_generate_http` 4, `site_editor_role_http`
  4, `site_schedule_http` 3, `site_notify` 1 — so the draft preview and the
  editor's own HTML assertions are unmoved. Goldens re-blessed and read line by
  line: hero/text_image/team/gallery gained the ladder, the gallery gained a
  second, **cropped** tile that pins what a frame spells in a URL. Byte budget:
  the full-page golden went 6 379 → 7 697 bytes, 7.7% of the 100 KB page
  budget, for four responsive images. No production, email, DNS or external AI
  service was contacted.
- **Wire verification** was the in-process bank through the real router
  against docker `alo-pg` (`tower::oneshot`, real photos written through the
  real store) — the established shape for this service, since `alo-sites` is
  not part of the local `alo-jmap` dev stack. It exercises the real HTTP
  semantics this item is about: status, `Content-Type`, `Cache-Control`,
  `ETag`, `If-None-Match` → 304, and the decoded pixels of the response body.
- **Cuts/flags:**
  - **Only section images get the ladder.** The theme logo/favicon, blog cover
    images and collection-card images keep their single original URL: none of
    them carries a crop model (S2.07a covers `SiteImage` only), and the logo is
    usually a vector with no raster derivative at all. Blog covers are the
    biggest remaining win and are worth their own item.
  - **The focal point is still not rendered.** It only means something where a
    layout crops further (`object-fit: cover`), and no section does that today
    — the stylesheet fits images rather than covering with them. Emitting an
    `object-position` nobody reads would have been decoration. It stays stored
    and validated, waiting for a layout that crops.
  - **Derivatives are made on first request, not at publish.** The first
    visitor to each rung pays one resize (tens of ms) and everyone after it
    gets the cached bytes. Pre-generating at publish would move that cost onto
    the owner's publish click for widths nobody may ask for; if a real site
    ever shows a cold-cache problem, the pipeline is already a pure function
    and can be called ahead of time without changing a URL.
  - **The `w` descriptor can be optimistic.** The renderer has no access to a
    photo's real dimensions (the stored model has none, and rendering never
    touches blob bytes), so the ladder is always offered in full and the
    *service* refuses to upscale. A 600px photo therefore advertises a 1440w
    candidate that answers 600px wide. The alternative — storing dimensions on
    upload — is a model change that belongs with S2.07c's editor, which is the
    first code that will know them.
  - No UI in this item, so no i18n strings and no web gate; `web/` is
    untouched.
  - No new route prefix (derivatives live under the existing `/assets/img/`),
    so the production Caddyfile needs nothing. The `alo-sites` container image
    gains the `image` crate at build time only.
- **Next:** S2.07c, the image editor — crop and focal controls, manual alt
  text, and optional propose-then-approve AI alt text from fixtures.

## 2026-08-12 — S2.07c framing a photo where you can see it

- **Shipped:** the image editor. Every image field in the section forms now
  shows the picture, a frame dragged over it (four corner handles, move by
  dragging the frame, arrow keys to move and shift+arrows to resize), a focal
  marker, four percent boxes for the exact numbers, "use the whole picture",
  a deliberate **decorative** state, the missing-description prompt, and an
  AI draft of the description behind propose-then-approve. New files:
  `web/src/sites/imageGeometry.ts` (the arithmetic), `ImageFraming.tsx` (the
  control), `ImageFields.tsx` (the image's fields, moved out of
  `SectionForm.tsx`), `copyContext.ts` (the shared AI-copy context both files
  now need), plus `GET /sites/{id}/images/{blob}` in alo-jmap and an
  `alt_text` copy action on the existing `ai-edits` door.
- **The editor needed the source pixels, and nothing served them.** The draft
  preview inlines images as `data:` URIs, which is right for a rendered
  document and useless to a control that must draw a rectangle over the photo
  at its own aspect ratio. The new route is two tenant doors deep — the site
  must resolve for the caller and the blob is read through the tenant-scoped
  `AccountStore::site_image` — so another tenant's blob, a blob that is not an
  image and an id that never existed are one indistinguishable `404`. Bytes
  are immutable per blob id, so `private, max-age=3600, immutable`, plus
  `nosniff` and the SVG-defanging CSP the public service uses.
- **The arithmetic is a separate file because only half of it can be tested
  in a browser-less DOM.** jsdom lays nothing out: every
  `getBoundingClientRect()` is zero, so a simulated drag would prove only that
  zero arithmetic works. Dragging and typing are the same operation on the
  same numbers, so the rules live in `imageGeometry.ts` and are tested there
  for real (13 tests): a frame dragged past an edge slides back in at full
  size rather than shrinking, a frame can never collapse below the schema's
  1% minimum, a drag reads the same in either direction, a typed width that
  would leave the picture pulls the left edge back, an emptied percent box
  cannot produce NaN geometry, and a focal point is always pulled onto its own
  crop — the one contradiction the schema refuses outright.
- **Absent means something, so absent is what gets stored.** A frame covering
  the whole picture and a focal point sitting exactly at the crop's centre are
  both written as *no key at all*: that is already what they mean, and the
  alternative is geometry on every image anybody ever opened a form on. The
  save-path test asserts the exact section, not a subset.
- **Replacing the picture drops the frame.** A rectangle of a different
  photograph is not a smaller version of the old decision, it is the wrong
  region of a new one; uploading or pasting a new blob id clears crop and
  focal.
- **Decorative and undescribed stopped looking the same** in the UI, as they
  already had in the schema. Ticking decorative clears the description and
  disables the field (the schema refuses a decorative image that still carries
  alt text — proven on the wire), and an undescribed picture says so in the
  form rather than leaving a blank field to scroll past.
- **The AI draft is honest about what it cannot see.** Nothing in this build
  shows a model an image — `alo-ai` speaks text — so the prompt says "You have
  NOT seen this photograph", confines the draft to what the section's own
  words claim the picture shows, and forbids invented visual detail (colours,
  counts, names, logos, words in the picture). The UI repeats it above the
  approve button. Wrong alt text is worse for a screen-reader user than
  missing alt text, which is why this is a proposal placed next to the real
  photo for the one party who can see it. Guards: the action only aims at a
  pointer ending `/alt` (refused before any model call), the answer must be a
  single `rewrite_copy` at that exact target, and a description over 200
  characters is refused rather than offered.
- **Verified:** `cargo fmt`; strict offline `cargo clippy -p alo-jmap
  --all-targets` — zero warnings from this change; `sites_http` 22 tests and
  `sites_generate_http` 7 green (one new in each: the image route's five
  answers including both wrong-tenant directions, and the alt-text arc against
  the scripted fixture model — no external AI service was contacted). Web:
  `npx tsc --noEmit` clean, `eslint` on all fourteen changed files clean,
  `npm run build` clean, and 172 tests green across `src/sites` + `src/i18n`
  (25 new: 13 geometry, 12 editor).
- **Wire-verified with real curl** against the local debug `alo-jmap` over
  docker `alo-pg` (server killed before and after): the uploaded PNG comes
  back byte-identical with `content-type: image/png`, `private, max-age=3600,
  immutable`, `nosniff` and the CSP; `401` without a token; `404` for a
  text/plain blob, an unknown blob id, and a site id that is not the caller's;
  the stored crop and focal point read back exactly as sent; `alt_text` aimed
  at `/heading` is `422` with the reason, aimed at `/image/alt` it reaches the
  typed `503 unconfigured`; a decorative image carrying alt text and a crop
  that leaves the picture are each `422` naming the rule.
- **Cuts/flags:**
  - **Pointer drags are not covered by a test**, deliberately and with the
    reason written in the suite: in a DOM with no layout every rectangle is
    zero, so such a test would assert its own fixture. The geometry behind
    every gesture is tested directly, and the keyboard path — the accessible
    one — is exercised through the real component.
  - **No aspect-ratio presets** (16:9, square, "match the other photos"). The
    schema has no notion of a required ratio and no section declares one yet;
    S3.01c is where ratios become a section-type property, and guessing one
    now would be a second, weaker copy of that decision.
  - **The focal point still changes no pixel** on the published site: it means
    something only where a layout crops further with `object-fit: cover`, and
    no section does that today (S2.07b's flag, unchanged). It is now editable,
    stored and validated, waiting for a layout that crops.
  - **Only section images got the editor.** The theme logo/favicon, blog cover
    images and collection-card images still carry no crop model at all
    (`SiteImage` is the only shape S2.07a gave one to), so there is nothing
    for the control to edit there. Blog covers remain the biggest gap and are
    worth their own item.
  - Image *dimensions* are still not stored on upload, so the `w` descriptor
    can still be optimistic (S2.07b's flag). The editor now loads the bytes
    and could learn them, but writing them would be a schema change, and this
    item's scope is the controls.
  - No new top-level route prefix (`/sites/*` already exists), so the
    production Caddyfile needs nothing.
  - Unrelated: `platform/alo-store/src/meet.rs:430` carries an
    `unused_variable` warning on `main` from the other track's area — left
    untouched, flagged here so the next full-workspace clippy run is not read
    as this change's.
- **Next:** S2.08a, analytics dimensions — UTM campaign, coarse country,
  device class, entry/exit, read-time buckets and outbound clicks as
  aggregates, with raw IP/UA/query discarded.

## 2026-08-12 — S2.08a where the visit came from, and what it read first

- **Item:** S2.08a, analytics dimensions. Shipped the five dimensions a
  request already carries — campaign, country, device class, entry page, exit
  page — end to end: migration, public write path, owner read, the
  `/sites/{id}/analytics` response, and tests at both doors. Read-time buckets
  and outbound clicks are **not** in this item; they are queued as S2.08a2
  (below, under Cuts).
- **The reduction happens at the door, and only there.** `serve::analytics`
  already turned a request into a date, a path, a referrer *domain* and a
  daily HMAC before any async work began; it now also turns the query string
  into one campaign label, the edge proxy's country header into two letters,
  and the user agent into one of five words. The raw values never leave that
  function — the store's write door literally cannot express them: it takes a
  `PublicSiteVisit` whose every field is a derivative, and re-validates each
  one rather than trusting the caller.
- **`utm_campaign` survives; the rest of the link does not.** A mail-out link
  carries the campaign next to things that identify one recipient
  (`utm_content=recipient-42`, an address in a parameter). One key is read,
  percent- and plus-decoded byte-wise (slicing the string could land inside a
  multi-byte character and panic on a hostile query), lowercased, folded to
  `[a-z0-9-_. ]` with everything else collapsed to a hyphen, and cut at 64
  characters. `<script>` becomes `script`, `été` becomes `t`, a 200-character
  campaign becomes 64: a link cannot invent a dimension shape, and the
  wire test proves `example.test` and `recipient` appear nowhere in any
  analytics table afterwards.
- **The country comes from the edge or not at all.** `cf-ipcountry`,
  `x-country`, `x-geo-country` — two ASCII letters or nothing. Cloudflare's
  `XX` (could not resolve) and `T1` (Tor) are not countries and are stored as
  "not reported". alo does not ship a geo-IP database and does not resolve an
  address to a place itself; if no proxy names the country, the bucket is
  honestly empty.
- **A crawler is not a reader, so it is named.** Device classification checks
  bot markers *first* — Googlebot's user agent claims to be an Android phone,
  and counting it as one would inflate the number an owner acts on. Five
  words: phone, tablet, desktop, bot, unknown. `unknown` is its own bucket
  rather than a guess: a request that named no device is not evidence of a
  desktop.
- **Entry and exit without a journey.** One cursor row per site, per day, per
  opaque token, holding only the page that token last looked at. First view of
  a day: entry+1 and exit+1 on that page. Moving on: the old page's exit
  count hands back to the new one (`GREATEST(hits - 1, 0)`, and the row is
  `SELECT … FOR UPDATE` inside the same transaction, so two simultaneous views
  cannot both believe they are first). Re-reading the same page moves nothing.
  A visitor-day therefore contributes exactly one entry and one exit, and the
  report suppresses buckets that fell back to zero (`HAVING SUM(hits) > 0`) so
  a page a visitor merely passed through is not reported as an exit.
- **Views, not people.** The dimension table stores no visitor token at all,
  so there is no way to join "arrived on a campaign" to "is this person" —
  the deliberate reason the new aggregates carry a hit count and no unique
  count. The two new tables' exact column lists are asserted in a test, as the
  originals already were.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store -p
  alo-sites -p alo-jmap --all-targets` — zero warnings from this change (the
  pre-existing `meet.rs:430` `unused_variable` on `main` is the other track's
  area, untouched). Tests green: all of `alo-sites` (including six boundary
  unit tests and the new `dimensions_are_derived_at_the_door_and_the_raw_
  request_is_dropped` integration test), `alo-store`'s
  `site_analytics_tenancy` (now asserting every dimension plus the
  wrong-tenant direction: tenant A's campaign never appears in tenant B's
  report), and `alo-jmap`'s `sites_http` 22 tests.
- **Wire-verified with real curl**, both services running locally against
  docker `alo-pg` (both killed before and after): a site published through
  `alo-jmap`, then real visits through `alo-sites` on the subdomain Host — a
  phone on a `?utm_campaign=Summer+Launch&utm_content=recipient-42&email=…`
  link from `cf-ipcountry: nl` reading `/` then `/about`, a Belgian desktop on
  `/`, and Googlebot with `cf-ipcountry: XX` on `/about`. The owner report
  answered exactly: campaigns `summer launch` 1 / none 3; countries NL 2, BE
  1, unreported 1; devices phone 2, desktop 1, bot 1; entry `/` 2 and
  `/about` 1; exit `/about` 2 and `/` 1 — the phone's exit moved from `/` to
  `/about` as it read on. `401` unauthenticated, `422` naming the rule for
  `days=0`, `404` for a site id that is not the caller's. A `psql` scan of the
  stored rows for `example.test`, `recipient`, `198.51.100`, and `mozilla`
  returned zero.
- **Cuts/flags:**
  - **Read-time buckets and outbound clicks are cut, and queued as S2.08a2.**
    Neither is visible to a server: they exist only in the browser, so they
    need a script on the published page and a public collect endpoint —
    a different privacy argument (what a beacon may send), a different abuse
    surface (an unauthenticated write with no page load behind it), and the
    published pages' near-zero-JS budget to re-argue. Half of it in this item
    would have been a beacon without caps or a byte budget; the whole of it
    belongs in its own commit.
  - **Bot traffic is counted, not filtered.** It appears in visits and pages
    as it always did, now with a `bot` device bucket beside it so an owner can
    see how much of the number is not a reader. Excluding it from totals is a
    product decision with a UI, and belongs with S2.08b.
  - **Device classification is substring matching**, which ages: it will call
    a device that names itself in a new way `desktop`. It is deliberately not
    a user-agent database (that is a dependency, a data file, and an update
    treadmill) and the classes are coarse enough that being wrong about one
    tablet changes no decision.
  - The country depends on a proxy that reports one. In the current
    production shape nothing sets `cf-ipcountry`, so the country panel will be
    entirely "not reported" until an edge does — a deployment note for
    S2.08b's empty state, not a code gap.
  - No new top-level route prefix (`/sites/*` and the public service's own
    Host serving already exist), so the production Caddyfile needs nothing.
- **Next:** S2.08b, the analytics UI — a calm overview and drill-down for the
  new aggregates, with the privacy explanation and useful empty states
  (including the honest "no edge reports countries yet" case).

## 2026-08-12 — S2.08a2 the two numbers a server cannot see

- **Item:** S2.08a2, page-beacon dimensions. The published page now reports
  how long it stayed readable and which outside domain a visitor left for, to
  a new public collect endpoint on `alo-sites`. Both land in the existing
  bounded dimension table as `read_time` and `outbound`; the owner report and
  `/sites/{id}/analytics` gained the two lists additively.
- **No new table, and no new privacy argument in storage.** Migration 0305
  widens exactly one CHECK constraint. The shape of 0304 holds: a bucket
  label and a hit count, no visitor token, nothing that can be joined to a
  person. A beacon writes into `site_analytics_dimension_daily` and nowhere
  else — a test asserts both visitor tables stay empty after four reports.
- **The endpoint carries no identity, in either direction.** It sets no
  cookie, reads none, and derives no visitor token — not even the day-scoped
  HMAC page views are counted with. That is deliberate rather than
  incidental: nothing a browser says about itself is trusted with an
  identity, which is why these aggregates have a hit count and no unique
  count, and why two beacons from one browser are unlinkable by construction.
- **The script names no page and reports once.** A read time is a fact about
  the site's day, not about `/prices` at 14:03, and the payload has no field
  to put a path in. It reports when the page is first hidden or unloaded —
  "how long they read before looking away". Payload is `t=<seconds>` or
  `o=<hostname>`, one per request, ≤512 bytes; 914 bytes of script, pinned by
  a byte-budget test beside the behavior script's.
- **The seconds are thrown away at the door.** The endpoint maps them to one
  of six fixed buckets (`0-10s` … `10m+`) and stores only that: "between one
  and three minutes" says something about a page, "137 seconds" starts to say
  something about a reader. `u64::MAX` is a bucket, not an error.
- **A hostname is not repaired, unlike a campaign label.** A campaign gets
  folded into a storable label because a mail-out's label is *meant* to be
  stored; a value that is not a hostname is not a hostname, and inventing a
  bucket out of an injected document would create a dimension nobody asked
  for. Markup, a path, a whole URL, an address, a bare word, non-ASCII: all
  `400`. Punycode is what a browser's `link.hostname` already gives.
- **Outbound is the one dimension a visitor's browser names**, so distinct
  values per site-day are capped at 200; past that new destinations count
  under the literal bucket `other`, which cannot be confused for a domain
  because a stored domain always contains a dot. Known destinations keep
  counting past the cap.
- **Tenant scope is the Host, never the payload** — there is no field to put
  one in — and an unresolvable Host is the same terse `404` a page gets, so
  the endpoint cannot enumerate sites. Its rate limit is its own (120 per 10
  minutes per client, the loosest of the three because a beacon is what a
  *reader* produces), so page analytics can never spend the budget standing
  between a guesser and a protected page.
- **The no-script path is untouched and now tested as such**: views,
  referrers, campaigns, countries, devices and entry/exit are all still
  derived from the request. The draft preview gets no beacon either — an
  editor moving sections around is not a reader — and the preview-has-no-drift
  test now pins that as its second deliberate exception beside the stylesheet.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store -p
  alo-sites -p alo-jmap --all-targets` — zero warnings from this change (the
  pre-existing `meet.rs:430` `unused_variable` on `main` is the other track's
  area, untouched). `cargo test -p alo-store -p alo-sites -p alo-jmap` green
  in full (exit 0), including the new `alo-sites` `beacon` suite (4 tests),
  the two script unit tests, the six beacon-parsing unit tests, the extended
  `site_analytics_tenancy` (beacon dimensions in bucket order, wrong-tenant in
  both directions, and the 320-destination cap test), and the re-blessed blog
  goldens.
- **Wire-verified with real curl**, both binaries running locally against
  docker `alo-pg` (killed before and after): a site published through
  `alo-jmap`, then on the public Host — the page carries
  `navigator.sendBeacon("/_alo/collect", body)`; `t=137` and `t=4` and
  `o=News.Example` each `204` with `no-store`; the stored rows are exactly
  `read_time 1-3m 1`, `read_time 0-10s 1`, `outbound news.example 1`. A
  garbage body, `o=%3Cscript%3E`, an unknown Host, a 2 KB body and a `GET` are
  `400 / 400 / 404 / 413 / 405`. The 121st beacon from one address is `429`
  with `retry-after: 599` while another address is still `204`. A `psql` scan
  for `137`, `198.51.100`, `curl` and `script` in the site's dimension rows
  returned zero. The owner report answered `readTime` in bucket order and
  `outboundDomains`; `401` unauthenticated, `422` for `days=0`, `404` for a
  site id that is not the caller's. The single page GET stayed one visit —
  beacons add no page views.
- **Cuts/flags:**
  - **Read time is per site-day, not per page.** The dimension table is
    (dimension, value) and a path would have to become the value, which both
    doubles the cardinality a browser can name and makes the read time a fact
    about a page a named visitor was on. Per-page reading time is a real
    product want; it needs its own privacy argument and its own item, and
    S2.08b should show the site-wide histogram rather than imply a per-page
    one.
  - **The read time is reported once**, at the first hide or unload. A visitor
    who looks away, comes back and reads for another ten minutes contributes
    only the first stretch, so the histogram skews short. Sending deltas would
    fix the number and would also be the first step towards a session — the
    trade was made in favour of the privacy shape.
  - **A beacon cannot be tied to a page view**, by design, so the counts are
    not comparable: 40 views and 12 read times does not mean 28 people left
    instantly, it means 28 browsers did not report (no script, no
    `sendBeacon`, a tab killed by the OS). S2.08b must say so on the screen
    rather than let an owner read a bounce rate out of it.
  - **Nothing is verified as coming from a real page load.** The endpoint is
    anonymous by necessity; the rate limit is the whole defence, and a
    determined script can add up to 120 events per address per ten minutes.
    Acceptable for an aggregate an owner reads as a shape, not as a number to
    invoice on.
  - **Bot traffic still counts here too.** A crawler that runs no scripts
    reports nothing (so read time is quietly reader-only), but the wire run
    shows curl counted as a `bot` page view exactly as before.
  - No new top-level route prefix in production: `/_alo/collect` is served by
    `alo-sites` on the site's own Host, which the front proxy already routes
    wholesale. The production Caddyfile needs nothing.
- **Next:** S2.08b, the analytics UI — a calm overview and drill-down for all
  the aggregates now collected, with the privacy explanation, the honest "no
  edge reports countries yet" empty state, and the beacon caveats above made
  visible rather than left for an owner to misread.

## 2026-08-12 — S2.08b nine aggregates that read as three questions

- **Item:** S2.08b, the analytics UI. Everything S2.08a and S2.08a2 collect is
  now on the owner's screen: campaigns, countries, devices, first/last pages,
  the reading-time histogram and the domains visitors left for, beside the
  visits, chart, pages and referrers that were already there.
- **Three groups, not nine panels.** The screen asks three questions in the
  owner's words — *how people found you*, *what they looked at*, *how they read
  it* — and each answers with three panels. A ninth panel dropped into a flat
  grid is a data dump; a group heading is what makes a number findable without
  a legend.
- **Every panel says where its numbers come from**, in one line under its
  title, because the caveats journaled in S2.08a2 are only honest if the owner
  reads them at the number rather than in a changelog: reading times come only
  from browsers that report them and never add up to the visit count; a country
  is resolved by the network in front of the site and never from a stored
  address; `other` in the outbound list is the day's overflow past 200
  destinations, not a domain; a device class is coarse and is all that is kept.
  The privacy note gained a second paragraph naming the page script and saying
  it carries no identity at all, so two reports from one browser are unlinkable.
- **The stored tokens are named in the reader's language** in one pure module
  (`analyticsLabels.ts`): `1-3m` → "1–3 minutes", `phone` → "Phone", `NL` →
  "Netherlands" via `Intl.DisplayNames`, `""` → "Not reported"/"No
  campaign"/"Direct" depending on which dimension is empty-labelled, `other` →
  "Other domains". An unknown token is shown verbatim rather than dropped: a
  server that grows a seventh bucket must not vanish from the histogram.
- **The reading-time panel never collapses and is never re-sorted.** Every
  other panel shows its top five with "Show all (N)"; the histogram shows all
  six buckets in duration order, because a histogram truncated to its top five
  or sorted by count is a different claim about the same data.
- **Each panel has its own empty state**, since a dimension can be empty while
  the rest of the screen is full — most of all countries, which stay empty
  until an edge reports them and say exactly that, plus "every other number
  here is unaffected". The whole-site "no visits yet" onboarding is unchanged
  and still draws no panels at all.
- **Verified:** `npx tsc --noEmit -p tsconfig.json` clean; `npx eslint` on all
  eight changed/new files with `--max-warnings 0` clean; `npm run build` clean
  (only the pre-existing chunk-size advisories). `npx vitest run
  src/sites/Analytics.test.tsx src/i18n/locale.test.ts` — 73 green, including
  the new 8-test `Analytics.test.tsx` (groups render, buckets named, histogram
  order pinned, panel notes present, per-dimension empty states, show-all
  expansion 5→9→5, no-visit onboarding, failed report surfaced) and the i18n
  parity suite, which passes because the 40 new keys were written in fr and nl
  in the same change — `untranslated.ts` stays empty.
- **Cuts/flags:**
  - **The drill-down is "show all", not "filter by".** The API answers one
    period with ten values per dimension and has no filter parameter, so
    "campaigns → which pages that campaign landed on" is not implementable
    without new server surface and a new privacy argument (a cross-dimension
    filter is the first step towards a session). The item's "drill-down" is
    therefore the honest one available: top five, then all ten, per dimension.
  - **No per-page reading time**, as S2.08a2 flagged: the panel is explicitly
    titled and noted as a whole-site histogram so nothing on the screen implies
    a per-page number the store cannot answer.
  - **Countries will be empty in production today.** Nothing in the current
    deployment sets `cf-ipcountry`, so the panel shows its explainer rather
    than a chart until an edge does. That is a deployment fact, not a gap.
  - **Rust untouched**, no new route, no storage change, so no wrong-tenant
    test applies to this item and the production Caddyfile needs nothing.
  - **Pre-existing failure, other track's area (not touched):**
    `web/src/chat/ChatModule.test.tsx` fails in a full `npx vitest run` ("a
    colleague is searched for, never listed" → `agents.map` of undefined at
    `ChatModule.tsx:753`) and passes when run alone. Reproduced on a clean
    stash of this checkout before any of my changes, so it is the chat/meet
    track's flake to fix.
- **Next:** S2.09a, aggregate heatmap collection — bounded click coordinates
  and scroll-depth buckets with no visitor or session identity, with the schema
  privacy proof and the abuse caps.

## 2026-08-12 — S2.09a a heatmap that cannot become a journey

- **Item:** S2.09a, aggregate heatmap collection. A published page now reports
  **where it was clicked** and **how far down it was read**, and both arrive
  reduced past the point where they could describe a person.
- **The reduction is the design, not a formatting step.** A click is sent as
  permille of the page's own width and height and becomes one cell of a fixed
  **32 x 64 grid**; a scroll becomes one of **ten tenths**; the reported CSS
  pixel width becomes one of `phone`/`tablet`/`desktop` and the number is
  dropped at the boundary that read it. `HeatmapCell` and `ScrollDepth` have
  private fields and one total constructor each, so there is no type in the
  store that can hold a coordinate at all.
- **`site_analytics_heatmap_daily`** (migration 0306) is the strictest
  analytics table we keep: tenant, site, day, path, viewport, metric, grid_x,
  grid_y, hits — **no visitor column, not even the day-scoped token page views
  carry**, no session, no time of day. A `CHECK` pins scroll rows to
  `grid_x = 0` and a bucket in 0..9, so "a scroll is a tenth" is a schema fact
  rather than a convention. The privacy proof is a test that reads
  `information_schema` and pins the column set exactly.
- **The one key a browser names is the page path**, so distinct paths per site
  and day are capped at 100 — dropped past the cap rather than folded into an
  overflow bucket, because a heatmap of "some other page" would be an overlay
  over nothing. The path is canonicalized to the exact shape a page view is
  counted under (`/about/` → `/about`), and anything that is not an absolute
  page path (a URL, a query string, a fragment, whitespace, control
  characters) is a `400`, never repaired.
- **Second abuse bound, in the page:** the beacon reports at most **twenty
  clicks per page view**, and the scroll depth only when it is deeper than the
  last one sent, alongside the read time at hide/unload. The endpoint's own
  per-address budget (120 per ten minutes, separate from forms and unlock)
  still stands behind it.
- **Owner read:** `GET /sites/{id}/heatmap?days=30[&path=/prices]` in a new
  `sites_heatmap` module — the pages that have data (a menu, so nobody has to
  remember a URL) and, for a named page, the grid, its dimensions, and the
  depth curve with all ten tenths kept. Cells are sparse; the curve is not.
- **Verified:** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store -p
  alo-sites -p alo-jmap --all-targets` — zero warnings from this change (the
  one `meet.rs` unused-variable warning is pre-existing, another track's area).
  Whole `alo-sites` suite green (34 unit + every golden), including the new
  3-test `tests/heatmap.rs` and the 4 new unit tests in `serve/heatmap.rs` and
  `serve/beacon.rs`; `alo-store`'s new 2-test `site_heatmap_tenancy` green
  (wrong-tenant in both directions, the cap, the schema proof). Full
  `cargo test -p alo-store -p alo-jmap` re-run to completion after the wire
  check: **181 suites, 2 738 tests, zero failures**.
- **Goldens re-blessed:** the beacon script grew, so all 18 published-page
  goldens carrying it were regenerated from the constant; its byte budget went
  from 1 KB to 2 KB and is still pinned (the whole page-script budget is now
  ~2.6 KB).
- **Wire-verified with real curl**, both binaries running locally against
  docker `alo-pg` (killed before and after): a site published through
  `alo-jmap`, then on the public Host — three clicks and two scrolls each
  `204` with `no-store` and no cookie; the stored rows are exactly
  `/prices phone click 16,16 x2`, `/prices desktop click 0,63`, `/prices phone
  scroll bucket 8`, `/prices desktop scroll bucket 1`. Nine malformed or
  hostile bodies (no page, no viewport, half a click, a negative number, a
  decimal, a URL as the path, a query string, markup) are all `400`; an
  unknown Host `404`, a 700-byte body `413`, a `GET` `405`. The 121st beacon
  from one address is `429` while another address is still `204`. A `psql`
  scan of the site's rows for an address, markup, a raw width or a query
  string returned zero. The owner report answered the path menu (126 events),
  the grid `32x64`, per-viewport cells and the ten-tenth curve; `401`
  unauthenticated, `422` for `days=0` and for `path=prices`, `404` for a site
  id that is not the caller's — and a **second real tenant** asking for this
  site's heatmap gets the same `404`.
- **Cuts/flags:**
  - **The grid is over the whole scrollable page, not the viewport.** That is
    what makes a cell mean the same thing on two screens, but it means the UI
    (S2.09b) must draw the overlay against a full-page screenshot or a
    proportional box — not against a phone-height viewport.
  - **A click count and a page-view count are not comparable**, exactly as
    S2.08a2 flagged for read time: browsers without `sendBeacon` report
    nothing, and the twenty-click cap truncates the busiest sessions. The
    overlay is a shape, never a rate.
  - **Minimum-sample suppression is not enforced by the store.** The read
    returns whatever was collected, including a page with one click. S2.09b
    owns the suppression threshold and must not present a two-click grid as a
    heatmap.
  - **The path cap drops silently.** A site past 100 pages in a day gets no
    row for the 101st page and no signal that it happened; acceptable for an
    aggregate, worth a note if a real customer ever runs a site that wide.
  - **No new top-level route prefix in production**: `/_alo/collect` is
    unchanged and `/sites/*` is already proxied. The production Caddyfile
    needs nothing.
  - The first `cargo test -p alo-store -p alo-jmap` run of this iteration was
    killed by my own `pkill -f "[a]lo-jmap"` before the wire check — that
    pattern also matches `cargo test … -p alo-jmap`. Kill the built binary by
    path (`pkill -f "target/debug/alo-jmap"`) instead; noted here because it
    cost an hour of test time.
- **Next:** S2.09b, the heatmap UI — page/viewport overlays over the full-page
  grid, minimum-sample suppression, keyboard-accessible summaries of what the
  overlay says, and empty states for a page nobody has clicked yet.
