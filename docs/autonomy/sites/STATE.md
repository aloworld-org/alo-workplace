# sites/STATE.md — Sites-track loop journal (append-only; newest at the bottom)

One entry per iteration: item id, what shipped, how verified, cuts/flags,
next item. The end-of-queue / emergency-stop control markers the wrapper
watches for are defined in LOOP.md — never write those exact phrases here
except to actually fire them.

Human-action inbox (things the loop must not do itself):

- ~~Buy/choose the public sites domain and set wildcard DNS~~ **DONE
  2026-08-07: alosites.com purchased (Namecheap). Verified live on public
  DNS: apex/`*`/www A → 152.53.179.142, null SPF + DMARC reject. The env for
  the alo-sites service is `SITES_DOMAIN=alosites.com`.**
- At next deploy: add the alo-sites container to production compose + Caddy
  wildcard/on-demand-TLS config (the loop never touches deploy/).
- Configure an AI provider key on the live server before real "generate my
  site" runs (loop verifies with fixtures only).
- Post-launch hardening (not urgent): submit alosites.com to the Public
  Suffix List so browsers isolate customer subdomains from each other.

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
