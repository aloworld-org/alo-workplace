# Design note — alo Sites (marketing site + blog + forms)

Status: S2 as built · 2026-08-13 · ADR 0036 · Sites track waves S1–S2

alo Sites is the AI-native no-code website builder: "tell me about your
business" produces a complete draft site, then editing is conversational
(propose-then-approve, the ADR 0034 trust pattern) or manual through
typed section forms. V1 ships a marketing site + blog + contact forms,
published instantly at `<subdomain>.<SITES_DOMAIN>` and optionally on a
live custom domain. This note records the as-built data model, web surface,
render pipeline, two-service boundary, and privacy posture. S1 built the site,
the blog, the forms and both domain modes; S2 added publishing in several
languages, collections over alo Base, the restricted site collaborator,
version history, scheduling, page passwords, image framing, the second
generation of analytics, the catalog and its orders, bookings, sandboxed
custom code and the domain buy-box. Each section names the item that built
it, and the two reconciliation tables at the end account for every feature
line the product doc promised.

## Surface

- **Inputs (edit side):** authenticated workspace users driving
  `/sites/*` routes on `alo-jmap` — site CRUD, page CRUD, section
  operations (add / update / move / remove), theme selection, blog-post
  linking, publish, domain verification, form-submission review, and
  AI generation/edit envelopes (fixture-verified in the loop).
- **Inputs (public side):** anonymous browsers hitting the new
  **`alo-sites`** service (`products/sites/alo-sites`) — `GET` of
  published pages resolved by `Host` header, `POST /f/:form_id` for
  contact forms, `/blog` index + post pages + RSS, `sitemap.xml`,
  `robots.txt`, `/healthz`, and the Caddy on-demand-TLS "ask" endpoint.
- **Outputs:** complete static HTML documents (semantic landmarks,
  meta/OG/canonical) plus one theme-token-driven stylesheet and
  near-zero JS (menu toggle + form submit only); stored form
  submissions with an internal-mail notification; daily aggregate
  analytics rows.
- **Who calls it:** the web module `web/src/sites` (editor UI) calls
  `alo-jmap`; the public internet calls `alo-sites`; `alo-ai`'s sites
  module produces generation/edit envelopes that `alo-jmap` applies.

### Web surface — as built

The Websites module follows the Wix/Squarespace builder reflex without
hiding the core path behind a menu. Its list surface keeps **New website**
visible. Creation shows the complete configured address while the user types,
checks availability, accepts either a slug or pasted full URL, and repeats the
server's validation reason verbatim. The name suggests a slug but never locks
it. Both AI generation and manual creation atomically create a Home page and
open its editor; an empty Home page exposes **Add your first section**, which
opens the Hero form in one click. The general **Add section** control remains
visible for every other section type.

The page editor keeps page management, section insertion, preview, theme,
publish, submissions, analytics, domains, and blog visible on the surface.
Typed forms edit section data; move and remove actions remain beside each
section. AI changes are review surfaces, never direct writes: per-field copy
names the exact field and shows old and proposed text, while whole-page edits
show the exact before/after rendered HTML. Approve is the only write.

### Data model (tenant-scoped unless noted)

All tables carry `tenant_id` and are reached only through the
store's tenancy doors; ids are newtypes; timestamps are
`timestamptz`. New store modules are `site_*` files in
`platform/alo-store` (one file, one responsibility).

- **`sites`** — name, `subdomain` (**globally unique** across tenants,
  DNS-safe `[a-z0-9-]{3,40}`, checked against a reserved-word list:
  `www`, `mail`, `admin`, `api`, product names, …), status
  `draft | live`, theme JSON, created/updated. The subdomain column is
  the single deliberate cross-tenant surface: the claim check touches a
  global unique index but reveals only *taken / free*, never the owner.
- **`site_pages`** — site ref, `slug` (unique per site, 1–80 characters
  from `[a-z0-9-]`, with empty allowed only for the home page), title,
  `sections` JSON
  (validated against the typed schema below on every write), SEO meta
  (title/description overrides), nav order, home flag.
- **Section JSON versioning** — a page's `sections` value is an
  envelope `{ "schema_version": 1, "sections": [ … ] }`. Each section
  is one variant of a typed Rust enum (serde, `#[serde(tag = "type")]`)
  in a `site_model` module: `nav`, `hero`, `features`, `text_image`,
  `gallery`, `testimonials`, `pricing`, `team`, `faq`, `cta`,
  `contact_form`, `collection` (S2.02a), `catalog` (S2.12a), `footer` —
  each with typed props. Unknown section
  types or props are a **validation error on write** (the editor and AI
  are the only writers, both speak the schema) but tolerated as
  *skip-with-log on read* by the renderer, so an old renderer never
  500s on a newer snapshot mid-deploy. Version bumps ship an explicit
  pure upgrade function (v1 → v2) applied on read; stored JSON is
  rewritten lazily on the next save. Prices inside the `pricing`
  section are **display strings**, not money values — nothing computes
  on them, so the integer-cents law is not in play.
- **Image presentation** — every image a section carries (`hero`,
  `text_image`, `gallery`, `team`) is a `blob_id` + `alt` plus three
  optional presentation props: a `crop` rectangle, a `focal` point, and
  a `decorative` flag. Crop and focal are **basis points**
  (ten-thousandths) of the source width/height with a top-left origin —
  never pixels, so a crop survives re-uploading the photo at another
  resolution, and never floats, so a stored value compares and
  round-trips exactly. Cropping is presentation, never destruction: the
  tenant's blob keeps every pixel. Validation refuses a rectangle that
  leaves the image or has less than 1% extent on an axis, and a focal
  point outside the image or outside its own crop, so the two props can
  never contradict each other. `decorative` is the missing half of alt
  text: blank `alt` **with** the flag means "nothing to describe";
  blank `alt` **without** it means "not written yet" — the distinction
  the editor's write-the-alt-text prompt is driven off. All three are
  additive: sections stored before they existed parse unchanged, mean
  whole-image-centred, and re-serialize with no new keys.
- **`site_page_snapshots`** — immutable per-page published copies:
  page ref, snapshot of sections + meta + slug + nav order, theme
  snapshot on the site's publish record, `published_at`. Publish
  creates new snapshot rows and flips the site's published-set pointer
  atomically; **the public service reads only snapshots**, so drafts
  are unreachable from the internet by construction, not by filtering.
- **Themes** — a JSON value on `sites`: palette + typography preset id
  (≥ 6 shipped presets) plus optional logo/favicon blob refs
  (tenant blobs via the existing Drive/blob store). Validated against
  a typed theme struct on write.
- **`site_posts`** (blog) — site ref, doc node ref (**must reference
  the tenant's own doc** — enforced in the store query, tested),
  slug, title, excerpt, cover blob ref, `published_at`, status
  `draft | published`. Post bodies live in alo Docs; publishing renders
  BlockNote JSON → HTML through a dedicated renderer with an
  XSS-safety test (script/style/event-handler content never reaches
  the published HTML).
- **`site_forms` / `site_form_submissions`** — a `contact_form`
  section references a form id; submissions store the posted fields
  (size-capped), `received_at`, and handled flag. **No IP and no
  user-agent are ever stored.**
- **`site_domains`** — globally unique normalized domain name, TXT
  verification token, status `pending | verified | live`. The verification
  action promotes an exact TXT proof directly to `live`; public Host serving
  and the Caddy on-demand-TLS "ask" endpoint require `live`, so a pending or
  otherwise non-serveable claim never earns a certificate.
- **`site_page_passwords`** — one row per password-protected page:
  site ref, the page identity, an argon2id hash of the password, and an
  opaque session `version` derived from it. Deliberately *not* part of a
  publish snapshot, and hanging off the site rather than the page — see
  **Password-protected pages** below for why both are load-bearing.
- **`site_analytics_daily`** — (tenant, site, date, path, referrer
  domain, hit count, unique count), plus
  **`site_analytics_daily_visitors`**, which stores only an opaque 32-byte
  HMAC token scoped to one site and one day. The source address is used
  transiently to derive that token with a deployment secret; query strings
  are stripped, Referer is reduced to its lowercase domain, and user-agent is
  never read. No raw IP, UA, full referrer URL, or cross-day visitor identity
  is persisted. Exact daily uniqueness therefore survives a process restart
  without becoming a visitor profile.
- **`site_analytics_dimension_daily`** — (tenant, site, date, dimension,
  value, hit count) for the five dimensions that answer *where did they come
  from and what did they read*: `campaign` (the `utm_campaign` label only —
  every other query parameter, including whatever a mail-out put in
  `utm_content`, is dropped at the door), `country` (the two-letter code the
  edge proxy reports; alo never resolves an address to a place itself),
  `device` (one of `phone`, `tablet`, `desktop`, `bot`, `unknown`, derived
  from the user agent, which is then discarded), and `entry`/`exit` (the page
  a visitor-day started and ended on). These count views, not people: no
  visitor token is stored per dimension, so there is nothing to join a
  campaign to a person with. Its companion **`site_analytics_visitor_day`**
  holds one row per site, day, and opaque token — the page that token last
  looked at — which is what makes an exit page computable without storing a
  journey; it reveals no more than the per-day visitor set above, and dies
  with the day's token. Values are bounded and lowercased into a small ASCII
  label at the boundary, so a hostile link cannot invent dimensions.
  Two further dimensions live in the same table but arrive by a different
  road, because no request carries them: `read_time` (one of six fixed
  buckets — `0-10s`, `10-30s`, `30-60s`, `1-3m`, `3-10m`, `10m+` — computed
  from a number of seconds the collect endpoint then discards) and `outbound`
  (the DNS host a visitor followed a link to). Both are reported by the
  published page's own beacon; see **The page beacon** below for what that
  script may say and what the endpoint refuses. `outbound` is the one
  dimension a *visitor's browser* names rather than the site, so distinct
  values per site and day are capped at 200 — past that, new destinations
  are counted under the literal bucket `other`, which cannot be mistaken for a
  domain because a stored domain always contains a dot.

- **`site_analytics_heatmap_daily`** — (tenant, site, date, path, viewport
  class, metric, grid column, grid row, hit count): *where* a published page
  was clicked and *how far down* it was read. Both are beacon-reported like
  the two dimensions above, and both are reduced before they are stored: a
  click becomes one cell of a fixed **32 x 64 grid** over the whole scrollable
  page (never a coordinate), a scroll becomes one of **ten tenths**, and the
  reported CSS pixel width becomes one of `phone`, `tablet`, `desktop` (never
  a size — a viewport width is a fingerprinting signal). There is **no visitor
  column in this table at all**, not even the day-scoped token, and no time of
  day: two clicks by one reader are indistinguishable from one click by two
  readers the moment they are written. The page path is the one key a
  *browser* names, so distinct paths per site and day are capped at 100; past
  that a new page is dropped rather than folded into an overflow bucket,
  because a heatmap of "some other page" would be an overlay over nothing.
  Owners read it through `GET /sites/{id}/heatmap`, which answers the pages
  that have data and, for one named page, the grid and the depth curve.
  **Presentation adds a floor the store deliberately does not:** the attention
  map draws nothing for a screen class with fewer than twenty clicks (or fewer
  than twenty depth reports for the curve), and says how many were counted
  instead — a map made from three clicks is a picture of three people wearing
  the authority of a thousand. It draws the grid as the whole page in
  proportion rather than over a screenshot or a screen-height box, because the
  grid spans the scrollable page, and every shaded square is repeated in words
  ("Centre, 30–40% down") so the finding survives without the colours.

### The page beacon

Four numbers an owner asks for are invisible to a server: how long a page was
read, which outside link a visitor took, where the page was clicked, and how
far down it was read. They exist only in the browser, so they need a script on
the page and a public endpoint to report to — an unauthenticated write with no
page load behind it, which is a different argument from every other collection
above and gets its own bounds.

- **What the script may say.** Four payloads, at most one per request:
  `t=<seconds>`, `o=<hostname>`, a click `x=<permille>&y=<permille>`, and a
  scroll `d=<permille>`; the two heatmap payloads add `p=<path>` and
  `w=<viewport width>`. It carries **no identity of any kind** — no cookie, no
  storage, not even the day-scoped visitor token page views are counted with.
  The read time **names no page**, so it is a fact about the site's day rather
  than about `/prices` at 14:03; a heatmap event is the one report that must
  name its page, because an overlay is drawn over one page, and it names
  nothing else. It reports the read time once, when the page is first hidden or
  unloaded, sends the scroll depth alongside it and only when it is deeper than
  the last one sent, and reports **at most twenty clicks per page view**.
- **What the endpoint refuses.** Tenant scope is the `Host` and never the
  payload (there is no field to put one in); an unresolvable Host is the same
  terse `404` a page request gets, so the endpoint cannot enumerate sites. The
  body is capped at 512 bytes, seconds become a bucket server-side, and a
  hostname that is not a bounded lowercase DNS host is refused outright rather
  than repaired into a storable label. A heatmap report is refused unless it is
  complete and every part of it is a measurement: a path that is not an
  absolute page path (a URL, a query string, a fragment, anything with
  whitespace or control characters) and a coordinate that is not a bounded
  integer are `400`, never repaired. Every answer is a bare status with no
  body — `204` on success — because `sendBeacon` cannot read a response and a
  chatty endpoint would only help someone probing.
- **Its own rate limit**, separate from the form and unlock budgets, so page
  analytics can never spend the budget standing between a guesser and a
  protected page.
- **The no-script path is unaffected.** Page views, referrers, campaigns,
  countries, devices and entry/exit are all still derived from the request at
  the door. A visitor who runs no scripts is a fully counted visit; only these
  four dimensions go unreported. The beacon is therefore never on the draft
  preview either — an editor moving sections around is not a reader.

### Render pipeline

```
page JSON (typed sections) + theme JSON
        │  validate/upgrade to current schema_version
        ▼
render lib (products/sites/alo-sites, `render` module — pure fns)
        │  section renderer per type → semantic HTML fragments
        ▼
full document: <head> meta/OG/canonical + landmarks + footer
        +  one generated stylesheet from theme tokens (CSS < 50 KB,
           page HTML < 100 KB for the golden site — byte-budget tests)
```

The renderer is a **library first**: `alo-sites` serves it publicly
from snapshots; `alo-jmap` reuses the same library for the
authenticated draft-preview endpoint, so preview and production HTML
cannot drift. Golden-HTML tests per section type plus a full-page
golden pin the output.

**Responsive images (S2.07b).** Every section image renders with a
`srcset` over a fixed three-rung ladder (480/960/1440 px) and a `sizes`
attribute derived from the slot it sits in — banner, half column, or
grid card — so the browser picks a candidate before it has any CSS.
Grid cards additionally carry `loading="lazy"`; a hero never does (it
is often the largest element painted). The URL grammar lives in one
module (`alo_sites::images`) read by three parties who must not drift:
the renderer writes the paths, `RenderedSite` collects the same paths
into the servable set, and the service parses one back.

```
/assets/img/<blob>                      the uploaded bytes, unframed
/assets/img/<blob>/w960                 the whole image at 960px
/assets/img/<blob>/c<x>-<y>-<w>-<h>/w960   a crop of it at 960px
```

Three rules make the pipeline safe. **Nothing is decoded that the
publish did not reference** — membership in the served publish's own
variant set is checked before any read, so a derivative cannot be
*asked* for, only *offered* (a query-parameter resizer is a CPU
amplifier pointed at its own origin). **Decoding is bounded**: a source
over 16 MB or 120 megapixels is never decoded, and the resize runs on
the blocking pool, so a hostile or slow image costs one request rather
than the runtime. **The answer is never worse than the original**:
nothing is upscaled, nothing that came out larger than its source is
served, and a source this build cannot decode (SVG, GIF, AVIF, ICO)
serves its original bytes under the derivative path. Derivatives are
cached in memory keyed by *site* + path — a blob id is unique only
inside a tenant — and are immutable, so they carry the same
`max-age=3600` + `ETag` contract as the original.

A cropped image's `src` fallback is the widest derivative, not the
original: the original is the picture *before* the owner framed it, and
a client that ignores `srcset` must not show what was cropped away. The
draft preview offers no ladder at all — it inlines `data:` URIs, and
there is no origin behind the sandboxed iframe to fetch a second copy
from.

### Two services, one boundary

- **`alo-jmap`** (existing, authenticated): all editing/management —
  tenant-scoped like every module, `Problem` errors, routes registered
  in `server.rs`. It never serves public traffic.
- **`alo-sites`** (new, `products/sites/alo-sites`): the only
  unauthenticated surface. Resolves `Host` → (tenant, site) via one
  indexed lookup (subdomain of `SITES_DOMAIN`, or a verified custom
  domain), then serves **published snapshots only**, with an in-memory
  cache + correct cache headers, a branded 404 page, and `/healthz`.
  It accepts form POSTs and page-beacon POSTs (`/_alo/collect`) plus the
  analytics tick — nothing else writes.
  Host isolation (site A's host can never serve site B's content) is
  an in-process integration test, not an assumption.

Dependency direction stays legal: `products/sites` depends on
`platform/alo-store` and never on another product; the web editor
talks only to `alo-jmap`.

### Publishing in more than one language (S2.01)

A European website is rarely written once. The model is **one page, several
languages** — never one site per language, which would double every later
edit and let the versions drift apart.

- **The contract** (`sites.default_locale`, `sites.enabled_locales`, S2.01a) —
  a site names the language it is written in and the languages it publishes
  in, at most twelve. Tags are normalized to one spelling (`FR-BE` → `fr-be`,
  2–3 letters then 2–8-character subtags, so `en` and `EN` can never both be
  enabled) and validated before they reach the column, and the database keeps
  the two structural
  invariants itself: at least one enabled language, and the default among
  them. Disabling a language is allowed and does not delete its translations:
  the next publish simply stops freezing them, which is the reversible
  reading of a decision an owner may take back.
- **The drafts** (`site_page_locales`, S2.01b) — the page row stays the one
  identity (nav order, home flag, the section stack the editor writes) and
  carries `content_locale`, the language its own content is actually in.
  Every other language is a row beside it: title, slug, SEO fields and its
  own sections. A URL spelling is unique **per site and language**, so
  `/contact` and `/fr/contact-nous` are both free to be the natural spelling
  rather than a transliteration of the default. Deleting the page takes its
  translations with it (one cascade, one identity).
- **The publish** (S2.01c) — a publish freezes the language contract onto the
  version and then freezes one snapshot per *page and language*: the base row
  at its own `content_locale`, plus every translation whose language is still
  enabled. A page nobody has translated yet is therefore absent from that
  language rather than served in the wrong one — the same
  unreachable-not-hidden construction the drafts/snapshots split uses.
- **What a visitor gets** — the default language lives at `/` and `/<slug>`;
  every other language is prefixed, `/<locale>` and `/<locale>/<slug>`.
  Rendered pages carry `hreflang` alternates and `x-default` computed from
  the sibling snapshots that actually exist, a canonical URL, and a language
  switcher listing only those siblings. The sitemap and the blog feed are
  locale-aware for the same reason.
- **Translating by hand comes first** (S2.01d). `GET
  /sites/{id}/translation-readiness` answers exact per-language coverage —
  which pages and posts are translated and which are not — and the editor's
  language controls copy the default language's content into a new language
  as a starting draft. Nothing about publishing in five languages requires an
  AI provider.
- **Translating with AI is a proposal** (S2.01e). `POST
  /sites/{id}/translation-proposals` returns a whole-site envelope — every
  page and post, before and after — which the owner reviews and approves;
  approval is the only write, and it lands in one transaction through the
  same validation a hand-typed translation gets (`site_translations`). The
  loop verified it on fixtures only.

### Collections — content whose home is alo Base (S2.02)

A menu, a team, a portfolio: repeatable rows that already live in a table and
should not be retyped into a page. A **collection** (`site_collections`,
S2.02a) binds one site to one alo Base table in the same tenant, plus a
`mapping` from that table's **stable field ids** to the card roles the
renderer knows — `title` (the only required one) plus optional `slug`,
`summary`, `body`, `image`, `link` and `published_at`, each checked against
the column's actual Base type on every write. Display names are
deliberately not stored — a Base user may rename a column this afternoon, and
a mapping that remembered the name would silently point at nothing. Deleting
the Base table takes the binding with it (one cascade); a mapped field that
has since disappeared, a row with content but no title, a bad link or a
repeated slug **fails the publish by name** rather than quietly publishing a
half-row — the live site stays as it was until the row is fixed.

- **Publishing freezes the rows** (`site_collection_snapshots`, S2.02b). Each
  publish copies the resolved rows into an immutable per-(publish, collection)
  JSON snapshot, and the public service reads only that. Editing the Base
  table afterwards therefore cannot change what visitors see until the next
  publish — the guarantee the whole snapshot model exists for. The snapshot
  keeps no foreign key to the draft binding, so disconnecting or deleting a
  collection never rewrites history. An empty collection renders a calm
  localized line rather than a hole; a row that fails validation leaves the
  live site as it was.
- **The screen** (`CollectionsView.tsx`, S2.02c) connects a table, maps the
  fields, previews the exact rows the next publish would freeze, and
  disconnects — all visible, all manual. AI can fill or translate the Base
  rows through Base's own propose-then-approve path; nothing here needs it.
- **The section** is one typed variant like every other (`collection`), so
  the editor, the AI op vocabulary and the renderer's golden tests treat it
  the way they treat a gallery.

### Version history and rollback (S2.04)

Every publish already is a version — an immutable `site_publishes` row
with its frozen page and collection snapshots. The history surface only
reads them, plus one addition: `site_publishes.restored_from`, naming
the version a publish was copied from.

- **Reading** — `GET /sites/{id}/publishes` lists the versions newest
  first with what each froze (page count, languages, collections, who
  published it, whether it is the live one).
  `GET /sites/{id}/publishes/compare?from=&to=` answers what a visitor
  would see differently: theme, language contract, pages added /
  removed / changed (naming the frozen fields that differ), and
  collections by name and size. **Metadata only** — the section content
  itself belongs to a preview, not a diff.
- **Restoring** — `POST /sites/{id}/publishes/{publish}/restore`
  appends a NEW publish holding a copy of the chosen one and flips the
  published-set pointer to it, in one transaction. Rejected
  alternative: pointing the site back at the old publish id — cheaper,
  but two versions would then share one identity (the public cache key
  and visitor `ETag` are `<publish_id>:<path>`), "live" would appear in
  the middle of the list, and a rollback would leave no trace.
- **The draft is not touched.** Restoring is a statement about what the
  internet serves; the editable pages remain the tenant's work in
  progress. Nothing in a rollback rewrites Base rows either — a
  collection snapshot comes back, its source table does not move.
- **Previewing a version** — `GET /sites/{id}/publishes/{publish}/pages`
  lists what that version froze (one entry per page *and* language), and
  `GET /sites/{id}/publishes/{publish}/pages/{page}/preview?locale=`
  renders one of them as a complete self-contained document through the
  same renderer the public service uses, with the stylesheet and images
  inlined (the draft preview's contract, for the same reason: public
  asset paths do not resolve on the edit origin). It reads snapshot rows
  only — that version's theme, that version's frozen Base rows — so what
  the owner looks at is what restoring would put back. The one value taken
  from the present is the site's *name*, which a publish does not freeze.
  A language the version never froze for that page falls back to the
  version's default language rather than refusing; history is read, not
  edited.
- **The screen** (`web/src/sites/HistoryView.tsx`, reached from the
  publish bar) is the version history people already know from Docs and
  from Wix: dates down the left — never publish ids, which nobody
  recognises — the selected version rendered beside them, and one button
  that puts it back. Restoring executes on the click rather than behind a
  confirmation, because it is reversible by construction: the result
  banner carries **Undo**, which restores the version that was live
  before. Below the heading the screen states that the draft is untouched,
  and against the live version it lists what would change (theme,
  languages, pages coming back / going away / changing) from the compare
  endpoint — the diff explains the preview rather than replacing it.

### Scheduled publishing (S2.05)

Going live at a chosen moment is a second *moment* to call the publish
path, never a second way to freeze a site.

- **`site_publish_schedules`** — an **intention**, not a version: site
  ref, `publish_at` (UTC), status
  `scheduled | publishing | published | cancelled | failed`, the user
  whose account door the publish runs through, attempt count, and — once
  it has run — the `site_publishes` row it produced or the reason it
  refused. Terminal rows are kept, so a tenant reads "published on
  Monday" or "it could not publish because …" instead of watching an
  entry disappear.
- **One future per website.** A partial unique index admits one
  `scheduled`/`publishing` row per site, and scheduling takes the site
  row's lock first: two editors scheduling at the same instant produce
  one intention, and rescheduling moves that row (keeping its id) rather
  than racing a second one into existence. A site being published right
  now can neither be rescheduled nor cancelled — the version it makes can
  be rolled back (S2.04), which is the honest remedy.
- **At-most-once claiming.** `Store::claim_due_site_publishes` marks due
  rows `publishing` in the statement that reads them, with
  `FOR UPDATE SKIP LOCKED`, so a second sweeper walks past a claimed row
  instead of publishing the same website twice. Rejected alternative:
  deleting the row on claim, as the scheduled-*send* sweeper does
  (`schedule.rs`) — a mail send is invisible until it lands, whereas a
  website publish is something the owner watches for and asks about
  afterwards, so the row has to survive its own execution.
- **No silent stall, and no pointless retry.** A worker that dies leaves
  a `publishing` row; the claim re-offers it once the claim is ten
  minutes stale and, after three attempts, writes it off as `failed`
  where the tenant can see it. A publish that *refuses* (no home page, a
  collection that no longer resolves) is terminal on the first attempt:
  ten minutes will not change the site's content, so the store keeps the
  refusal verbatim for the owner to act on.
- **Validation** — the moment must be in the future and at most a year
  ahead; the site's *content* is deliberately not checked at scheduling
  time, because the author has until that moment to finish it. Times are
  stored in UTC; explaining them in the reader's own time is the
  surface's job (S2.05b).
- **Routes** (`site_schedule.rs`, S2.05b) — `GET /sites/{id}/schedule`
  answers `{schedule, history}` (the pending intention, `null` when there
  is none, plus what previous ones did); `POST` with `{"publishAt"}` both
  schedules and reschedules; `DELETE /sites/{id}/schedule/{schedule}`
  calls one off. `publishAt` is an RFC 3339 **instant** in both
  directions — a caller may send any offset and every answer reports UTC,
  so no wall-clock string ever travels without its zone. Same guards and
  error contract as the rest of `/sites/{id}`: the per-site grant
  middleware, `404` for anything outside the caller's tenant, `422`
  carrying the store's own sentence.
- **The sweep** (`site_publish_worker.rs`, S2.05b) — a 30-second tick in
  `alo-jmap` claims due intentions and publishes each **through the
  scheduling user's own account door**, so a scheduled publish has the
  same tenant scope and the same recorded author as the button in the
  editor. It answers the two failures differently: a store *refusal* is
  written to the row verbatim and is terminal, while an *infrastructure*
  failure (database, blob backend) leaves the claim standing so the
  stale-claim path retries it and, after three attempts, fails it
  visibly. Nothing but the coarse error reaches a log.
- **The surface** (`SchedulePublish.tsx`, S2.05b) — a panel directly
  under the publish bar, because publishing later is the same decision as
  publishing now with a moment attached. The picker is a
  `datetime-local`, pre-filled with tomorrow at 09:00 rather than left
  empty, and beside it the screen states the moment in full and **names
  the reader's own time zone** — the person scheduling a launch from
  another country has to be able to see which nine o'clock they picked.
  What is sent is `new Date(value).toISOString()`, so the wall clock the
  browser showed and the instant the server stores cannot disagree.
  Scheduling, moving and calling off are one click each with no
  confirmation, since none of them touches what is online; the panel
  polls once a minute while an intention is pending, so "publishes on …"
  becomes "published itself on …" without a reload, and a refusal is
  shown in the server's own words.

### Password-protected pages (S2.06)

A page that is published but only for people who have the password — a
price list for dealers, a rehearsal programme, a page shared with one
client. The gate is the only place on the public service where bytes are
withheld from an anonymous visitor, so it is deliberately narrow.

- **`site_page_passwords`** (migration `0303`) — one row per protected
  page: tenant, site, the page identity the snapshots also carry, an
  argon2id PHC hash, and an opaque `version` derived from that hash. The
  plaintext is never stored and no read on any door returns it: an owner
  who has forgotten the password replaces it.
- **Live, not frozen.** Protection is deliberately kept *out* of the
  immutable publish: setting a password, changing it, or lifting it takes
  effect on the very next request. Rejected alternative: freezing it into
  `site_page_snapshots` like everything else a publish freezes —
  consistent with the rest of the model, but it would leave a leaked
  password working until the owner happened to republish, which is the
  wrong failure direction for a security control.
- **Fail-closed on a deleted page.** Deleting a draft page does not
  unpublish its snapshot, so the row hangs off the *site*, not the page:
  the still-served snapshot stays closed until somebody lifts the
  protection or the site goes. One password covers a page in every
  language, because every locale snapshot shares the page identity.
- **Sessions are signatures, not rows** (`serve/unlock.rs`). A correct
  password mints an HMAC-signed cookie over *(public host, page id,
  protection version, expiry)*, valid twelve hours,
  `HttpOnly; Secure; SameSite=Lax`. Nothing about the visitor is stored —
  no session table, no identifier — and the three bindings each carry a
  test: another host cannot present it, another page cannot be opened
  with it, and changing the password rotates the version, which is what
  makes "change the password" a real revocation. The signing key is
  derived from the deployment's existing sites secret under a fixed
  label, so unlock signatures and analytics visitor hashes cannot be
  confused and no new secret has to be deployed.
- **Cache-safe answers.** The `401` unlock screen is `no-store` with
  `Vary: Cookie` and no `ETag`; the unlocked page is `private, no-store`
  with `Vary: Cookie` and, again, no validator — a shared cache must
  never be able to hand one visitor's unlocked copy to the next person,
  which is exactly what the ordinary `public, max-age=60` answer would
  invite. The screen carries the site's theme but none of the page's
  content, not even its title, and is marked `noindex`; protected pages
  are also left out of `sitemap.xml`.
- **Guessing costs.** The unlock `POST` is the only write the page path
  accepts, is rate-limited per client key on its own budget (eight tries
  per ten minutes, separate from the contact-form limiter so form traffic
  cannot spend it), and answers `429` with `Retry-After` when the budget
  is gone. Verification runs argon2 on a blocking thread; an unprotected
  or unknown page pays the same cost and is discarded, so timing says
  nothing about which pages carry a password.
- **Routes** (`site_protection.rs`) — `GET /sites/{id}/passwords` lists
  the protected pages of a site in one read (for a page list to mark
  them); `GET/PUT/DELETE /sites/{id}/pages/{pid}/password` reads whether
  a page is protected, sets or changes the password, and lifts it.
  `PUT` takes `{"password"}` and answers `{protected, pageId, createdAt,
  updatedAt}` — never the password, and never echoing the body on a
  refusal. Same guards as the rest of `/sites/{id}`: the per-site grant
  middleware, `404` outside the caller's tenant, `422` carrying the
  store's own sentence.
- **Owner surface** (S2.06b, `web/src/sites/PagePassword.tsx`) — a panel
  in the page editor stating who can open the page rather than naming a
  setting, plus a lock badge per row in the site's page list (one
  `GET /sites/{id}/passwords`, not one call per page) and a line on the
  editor's preview saying visitors are asked for the password first, so
  a preview is never mistaken for what the internet serves. Three
  properties the screen owes the model. It never renders a stored
  password (there is none to render) and says so, offering a show/hide
  toggle instead of a confirm field. It only refuses an EMPTY field
  itself; length and whitespace rules belong to the store and its
  sentence is shown verbatim. And lifting a password — the one gesture
  that discloses a page to the internet and cannot be undone — arms
  first and acts on the second click, while setting and changing, both
  reversible, act immediately (ux-principles law 7). When the protection
  state cannot be read at all the panel says it does not know rather
  than defaulting to "anyone can open this page": the reassuring guess
  is the dangerous one.

### Form flow

```
visitor → POST /f/:form_id on alo-sites
  → size caps + honeypot field (silent drop) + per-IP rate limit
    (in-memory sliding window; the IP is used transiently and never
    persisted)
  → insert into site_form_submissions (tenant-scoped via the form's
    site)
  → an alo-jmap background sweep claims each unnotified row at most once
    and delivers an INTERNAL message to the site owner's inbox (the existing
    local-delivery path — never outbound SMTP); From is the site's no-reply
    address and Reply-To is the visitor when supplied
  → a person decides whether the enquiry is an opportunity, and hands it
    off (below). Nothing creates a CRM deal on its own: a lead nobody
    chose is a board full of spam.
```

### Conversions, counted three times and never joined (S2.10a)

`site_conversion_daily` is (tenant, site, day, source kind, source id, stage,
hits) — how often a conversion point was **seen**, **started** and
**submitted**. Three properties make it a funnel that needs no visitor
identity:

- **The id belongs to the site, not the visitor.** The source is a form id
  the page's own markup already publishes (`<form action="/f/{id}">`), so
  attribution needs no cookie, no tracking parameter and no visitor token —
  there is no column one could be stored in. Cardinality is bounded the same
  way: the id must resolve to a form of the site the Host named, and forms
  are capped per site, so a browser cannot invent buckets the way it can with
  a page path.
- **Three counters, never a journey.** Nothing records that one browser did
  two of the three, so a funnel is a ratio of totals and cannot be resolved
  to a person. The day is the finest grain, as everywhere else in this family.
- **The submit is counted where the row is written**, not from the page:
  `record_public_form_conversion` runs in the submit path, because a script is
  easier to lie to than a socket. Only *view* and *start* — the two facts a
  server genuinely cannot see — come from the beacon.
- `source_kind` is a word rather than a flag, so the order form and the
  booking form can convert on their own site-owned objects later without
  changing what today's rows mean. The id is deliberately **not** a foreign
  key: deleting a form must not rewrite last month's report.

### The seam to CRM and Billing (S2.10b)

The submission row is where Sites stops and the business starts. One
tenant-scoped table owned by Sites, `site_lead_attribution`, records the
only new fact — *this enquiry became that opportunity* — and everything
else is read live from the modules that own it. Nothing about a deal or
an invoice is copied: a copied value is wrong the moment somebody edits
the deal.

```
site_form_submissions.id  ──link──▶  crm_deals.id  ──customer_id──▶  billing_invoices
        (Sites owns)                  (CRM owns)                      (Billing owns)
```

- **The handoff is a person's decision, and one enquiry becomes at most
  one lead** (a unique constraint, not a convention). Re-linking the same
  deal answers the existing link; a second, different deal is refused by
  name. Creating the opportunity uses CRM's own writer inside one
  transaction with the link, so a handoff produces both rows or neither,
  and the card carries the enquirer's own name and address rather than
  re-typed ones. The words on the card are the caller's; the only fact
  Sites supplies unasked is the site's subdomain as the deal's `source`.
- **The invoice join is stated, not guessed.** Billing records no
  opportunity on a document, so an invoice counts when it was raised for
  the customer the lead became, **after** the link was made, and is
  issued or paid — never a draft, never a void, and never a customer's
  back catalogue. The payload names the rule (`invoiceRule:
  customerSinceLead`) so a screen cannot quietly upgrade it to "revenue
  this page generated".
- **The period selects the leads, not the money.** `?days=` bounds the
  conversion counts and the handoffs; the deals and documents of those
  handoffs are then reported as they stand now, because an invoice
  raised in March for a January enquiry is January's doing.
- **Money is per ISO 4217 code, in integer cents, never converted** — as
  CRM's own pipeline report does, and for the same reason: a forecast has
  no issue date to convert at.
- **Permission is the reason this is a separate route module.**
  `/sites/{id}/*` is the one surface a site editor may use, so the
  attribution routes refuse the site-editor role outright, honour the
  per-user CRM switch, and refuse an accountant the *write* while
  allowing the read. With Billing switched off the pipeline figures
  remain and `invoicedCents` is `null` rather than `0` — "not yours to
  see" and "nothing was invoiced" are different statements.
- **Erasure is respected.** The link holds two ids, a user and a time,
  and nothing about the visitor; deleting the submission takes the link
  with it, while the aggregate conversion counters — which never held an
  identity — are untouched. Deleting the deal removes the claim that a
  form produced it, never the other way round.

### AI posture (S1.26–S1.30)

Generation and editing are structured envelopes in `alo-ai`'s sites
module: full-site draft (business description → site JSON) and typed
edit ops (add/remove/reorder section, set prop, rewrite copy) with
strict schema parse + one repair retry. Everything is
propose-then-approve; a draft site is never auto-published. The loop
verifies with **fixture model outputs only** — live calls require a
human-configured, tenant-scoped OpenAI-compatible provider (base URL, model,
and optional key in Settings), and the unconfigured path degrades to
blank-site + templates. Per-field copy operations are constrained to an
exact stored section and JSON string pointer; proposing writes nothing.

### Editing text on the page (S3.01a, ADR 0042)

The preview the editor shows is not a picture of the page — it is the
page, and its text is typed into where it sits. The rendering the edit
API serves for the draft (`GET /sites/{id}/pages/{pid}/preview`) is the
same document publishing produces, with two additions:

- every element whose text is **exactly one typed string** carries
  `data-alo-text="<section index><JSON pointer>"` — `2/items/0/title`,
  `0/heading`, `12/text`;
- a small script (`alo-sites`'s `render::script`) makes those elements
  plain-text fields, and reports a finished edit to the app with
  `postMessage({alo:"site-text-edit", key, text})`.

That coordinate is **the same coordinate a `rewrite_copy` operation
names**, so the edit travels the reviewed-edit door
(`PUT /sites/{id}/pages/{pid}/ai-edits`) as a one-operation envelope —
the identical request an approved AI proposal makes. There is no inline
save route, no second validation, and one undo history for both paths.
`products/mail/alo-jmap/tests/site_inline_text.rs` pins the property:
every mark the renderer emits is an applicable `rewrite_copy` target and
changes that property and nothing else.

Three rules bound it:

- **One element, one string.** An element carrying two properties (a
  testimonial's `figcaption`, which holds the author and the role; a
  pricing tier's price, which holds the amount and the period) is not
  marked — one gesture may not rewrite two properties, or the diff is a
  guess. Those keep the prop form until their markup separates them.
  Link labels are likewise unmarked: a label and its href are one
  decision, and that is the next slice.
- **The page never writes.** The preview document has an opaque origin
  inside a `sandbox="allow-scripts"` frame; it can reach nothing. The
  editor proves the sender is its own frame (an opaque origin makes
  `event.origin` worthless), resolves the coordinate against the sections
  it is holding, and refuses a stale one rather than aiming a rewrite at
  whatever moved into that index.
- **Nothing a visitor receives changes.** Marks and script exist only in
  the editable draft preview: not on a published page, not in a version
  preview, not in the "after" half of a proposal — none of which is a
  thing you may type into. A `custom_code` block is never marked either,
  because the edit door refuses to write custom code and an outline
  inviting a refused click is worse than no outline.

Undo and redo are the same operation with the previous text, applied
through the same door, in the editor's toolbar and on ⌘Z/Ctrl+Z.

### Moving a section on the page (S3.01b, ADR 0042)

The second gesture on the same document. A section's root element in the
editable preview carries `data-alo-section="<index>"` — the coordinate a
`reorder_section` operation names — and the script makes `<main>`'s own
children draggable and focusable. Dragging one **reflows the page under
the pointer**: the node is really moved in the DOM on every `dragover`,
so what is previewed during the drag is the arrangement itself. Nav and
footer are landmarks rather than stack positions and take no part.

- **The frame reports a neighbour, not a destination.**
  `postMessage({alo:"site-section-move", from, before})`, where `before`
  is the index the section now sits above and `null` means the end. Both
  doors that can move a section splice — remove, then insert — so the
  destination is one off when a section travels *down* the page, and that
  arithmetic lives in the app (`web/src/sites/sectionMove.ts`), where it
  is unit-tested, rather than in a string of JavaScript in the renderer.
- **One door, three ways to ask.** The drop, the stack's own move
  buttons and an assistant's `reorder_section` all end at the same stored
  ordering. `sites_http.rs` proves the first two on the wire produce the
  byte-identical envelope as the third; `site_section_move.rs` proves the
  diff of a move is *only* a permutation — every value byte-identical
  before and after, over every (from, to) pair on a page carrying every
  section type — and pins it as a golden.
- **A keyboard equivalent on the page.** Each section is `tabindex="0"`;
  `Alt`+`ArrowUp`/`ArrowDown` sends the same message, and after the
  document is replaced the app posts focus back to the section that
  moved, so a section can be walked down a page without leaving it. The
  stack beside the preview keeps its own labelled move buttons.
- **The editor supplies the words.** A section's accessible name is
  posted into the frame (`{alo:"site-edit-chrome", labels, focus}`)
  rather than rendered by `alo-sites`: it is *editor* chrome and must be
  in the language of the person editing, which need not be the language
  of the site. A press that begins inside a text field is a text gesture,
  never a drag.
- **The preview may not lie about layout.** The editing stylesheet is
  layout-neutral by construction — `outline`, `cursor`, `opacity` and
  nothing else. In particular nothing sets `position`, which would make a
  section the containing block of an absolutely positioned descendant and
  lay the draft out differently from the published page.

A move is a step in the same undo history as a text edit
(`web/src/sites/editHistory.ts`); its inverse is the move back, applied
through the same door, so ⌘Z covers both gestures in the order they
happened.

### Resizing a section within its constraints (S3.01c, ADR 0042)

The third gesture, and the one that decides whether the editor stays an
editor or becomes a canvas. ADR 0042 allows "resize within the section's
own constraints" — so the constraints are **written down as data**, in
`alo_store::site_layout`, and everything else reads them:

- **The vocabulary is words, never numbers.** `ColumnSplit`
  (`wide_image | half | wide_text`), `GridColumns` (`two | three |
  four`) and `ImageShape` (`natural | wide | square | tall`) are serde
  enums stored on the sections that offer them — `text_image.split`,
  `features/gallery/team.columns`, `SiteImage.shape`. A percentage, a
  pixel count or a fraction is not a value the schema can hold, so "no
  gesture can produce free positioning" is a property of the type, not a
  rule an editor is trusted to keep (`site_layout.rs`,
  `a_free_value_is_not_expressible`).
- **One declaration, served.** `layout_controls(kind)` names, per
  section type, each resizable property, the JSON pointer it lives at,
  its values *in order* and what an absent value means. `GET
  /sites/config` publishes it as `sectionLayouts`; the editor renders one
  choice per declared value and can therefore offer nothing else. It is
  served rather than mirrored in TypeScript so the declaration and the
  validation cannot drift apart.
- **A resize is a `set_prop`.** The same operation an approved AI
  proposal carries, through the same `PUT …/ai-edits` door, recorded as
  one step in the same undo history (`kind:"layout"`, inverted by
  swapping its two declared values). No resize endpoint exists, because a
  second door would be a second thing to get wrong.
- **The gesture on the page carries a direction.**
  `Alt`+`ArrowLeft`/`ArrowRight` on the focused section posts
  `{alo:"site-section-layout", index, step}` with `step` ∈ {-1, +1}. The
  preview document is never told what the values *are*, so nothing
  running inside it can name a ratio; the app resolves the direction
  against the declaration and the section it is holding. The visible
  choices live beside the section in the stack, in the language of the
  person editing.
- **Absent renders as it always did.** Every property is optional and
  every `None` emits no class, so a page nobody has resized is
  byte-identical to the page it was before this schema gained them
  (`an_unset_layout_adds_nothing_to_the_page`).
- **The choice is a ceiling, not a promise.** The stylesheet gives a
  phone one column and a tablet at most two whatever was chosen.
  `layout_responsive.rs` resolves the generated sheet at 360, 768 and
  1280 px for every declared choice and pins the resulting column counts
  and track lists as a golden — mobile stays good by construction, and
  the proof is a test rather than an intention.

What was deliberately left out: a pointer drag on a splitter. The edit
stylesheet is layout-neutral by construction (above), a handle would need
`position`, and HTML5 drag does not fire on phones — the choice row and
the keyboard step cover both without a preview that lays the page out
differently from the published one.

### The template catalog (S2.11a)

The manual sibling of generation, and the path a tenant without a
configured AI provider lands on. A template is **curated content shipped
with the build** — `alo-store`'s `site_templates` module parses
`site_templates/catalog.json` once into the same `SectionsEnvelope` types
the editor writes — so there is no table, no migration and no per-tenant
row, and every tenant sees the same catalog. Four rules hold, checked at
load and asserted by the suite:

- **No images.** Every picture is a tenant blob and the catalog is
  tenant-less, so `text_image`, `gallery` and any image-bearing prop are
  refused; the copy invites the owner to add pictures in the editor.
- **No claim only the customer can make** — no testimonial, no team
  member, no price. A tenant who publishes a template unedited must not
  thereby publish a lie; the pricing page ships the em-dash placeholder.
- **Every internal link resolves** inside the template's own page paths.
- **One persistence door.** `SiteTemplate::draft` produces a
  `NewGeneratedSite` and `create_generated_site` commits it — the same
  atomic transaction, validation, contact-form linking and
  born-as-a-draft rule the AI path gets.

Templates are versioned individually (`version`, bumped when copy
changes) and the instantiate answer names the version, so a support
conversation can name the exact content a site started from. The version
is not stored on the site: a site is the tenant's from the moment it
exists and nothing reaches back into it.

Routes: `GET /sites/templates` (the catalog),
`GET /sites/templates/{id}/preview?page=` (one page rendered by the
public renderer, self-contained, `no-store`),
`POST /sites/templates/{id}` `{name, subdomain}` (a draft site, never
published). All three authenticate; an unknown template id is `404`.

### The gallery (S2.11b)

The catalog's screen is the second half of "New website", beside — never
behind — the description field: `web/src/sites/TemplateGallery.tsx`, a real
radio group (roving `tabindex`, arrow keys, Home/End, selection following
focus) whose first option is the blank start, so a person who ignores the
gallery still gets the path that existed before it. Choosing a card is one
click and immediately renders that template through
`GET /sites/templates/{id}/preview`, in a sandboxed iframe fed by `srcdoc`;
the other pages are reached through tabs above the frame.

The frame takes **no pointer events**. A rendered page's navigation points at
a real host, and letting a link be clicked inside an opaque-origin frame
would blank the preview to no one's benefit — so the preview is a picture and
the tabs are the navigation. Creating goes through
`POST /sites/templates/{id}`, one transaction, and lands in the new site's
Home page exactly as generation does (S1.30c); a catalog that fails to load
costs the gallery and not the screen, since the blank card below it still
creates a site and its Home page. The old creation-time theme-preset picker
is gone: a template carries its own preset, and a blank site's theme is
chosen in the theme dialog that owns it (S1.14) rather than in a weaker
second copy.

### The catalog (S2.12a)

A **collection** binds to a table in alo Base and is re-read on every publish.
A **catalog** is the opposite choice and exists because most of what a small
business sells is not in a Base at all: the rows live in `site_catalogs` /
`site_catalog_categories` / `site_catalog_items`, tenant- and site-scoped like
every other Sites object, and the tenant edits them here. Both exist on purpose
— one for data that has another home, one for data whose home is the website.

- **Money** is integer minor units (`price_cents`) plus the catalog's own ISO
  4217 `currency`; the decimal count comes from `currency_exponent`, the single
  place that knows yen has none and dinars have three. Nothing in the path is a
  float, and formatting happens at the very edge, in the renderer, per locale
  (`render/money.rs`: separators and symbol placement are `UiStrings` fields,
  so a new language is a new const). An absent price means *no price shown* —
  an enquiry-only service — which is not zero.
- **Availability** is `available | sold_out | hidden`. `sold_out` renders with
  a label; `hidden` is removed at freeze time, so a withheld item is absent
  from the published copy rather than filtered out of it by the renderer. That
  is the same construction the drafts/snapshots split uses: unreachable, not
  hidden.
- **A photograph carries its own words (S2.12c3).** An item's picture is a
  tenant blob (`image_blob_id`, uploaded through Drive like every other picture
  in Sites) plus `image_alt`, what it shows. The two are one decision: the
  store refuses a description without a picture, replacing or removing the
  picture clears the description, and the published card falls back to the item
  *name* when nobody has written one — never to an empty `alt`, which would
  claim the photograph says nothing. The Base import maps no description
  column, so it keeps a hand-written one for exactly as long as the picture is
  unchanged and drops it when the attachment is replaced.
- **The Base import is a seam, not a binding.** `site_catalog_import` copies a
  mapped table into the catalog once; each imported row remembers its Base
  record in `source_key`, so a second import updates what it created before
  instead of duplicating it, and rows typed by hand are never touched. A price
  it cannot read *unambiguously* — `1,234`, which is 1234 in Amsterdam and
  1.234 in Boston — stops the import naming the row. Guessing there would put a
  wrong number on a public page, and a wrong price is worse than no import.
- **Publishing freezes it.** `site_catalog_snapshots` holds one immutable copy
  per (publish, catalog): name, currency, categories, and visible items, with
  each item carrying its category's *handle* rather than an editable id, so a
  published page never depends on a row that can still move. The public service
  reads only this copy; the draft preview resolves through the same function
  without writing, so the editor sees what the next publish would freeze.

### Booking services and the Agenda seam (S2.13a)

A **booking service** (`site_bookings`, tenant- and site-scoped) is what a
visitor may book: a name, a duration, the weekly hours it is offered in, the
extra questions asked, and the Agenda calendar it lives in. The model half is
S2.13a; reading availability and taking a reservation are S2.13b, and the
editor screens are S2.13c.

- **Agenda is reached through one seam.** `site_agenda` is the only place in
  the Sites code that asks Agenda anything: it turns the account's calendars
  into `SiteAvailabilitySource { calendar, name, writable }` and resolves one
  by id. Nothing in Sites edits `calendar.rs` or queries `calendars` directly,
  so the day a share works differently, Sites changes in one function. A source
  must be **writable** (owner or editor), because the booking that follows has
  to put the appointment somewhere; a read-only share is *listed* — so the
  picker can explain it — and refused at binding time by name. A source is
  resolved on every read and never cached: a calendar that has since been
  deleted or unshared reads back as `null` beside the stored `calendarId`,
  which is a broken connection the editor can show rather than an empty week a
  visitor discovers.
- **Opening hours are declared, not inferred.** A dentist whose Sunday is empty
  is not open on Sunday, so availability starts from weekly windows the owner
  wrote — ISO 8601 weekday (1 = Monday … 7 = Sunday) and minutes from midnight
  in the service's own IANA `time_zone` — and the calendar can only ever take
  slots away. Storing wall-clock minutes plus a zone rather than UTC instants
  is what makes a daylight-saving change move the appointments *with* the
  clock. The store sorts the week before writing it, refuses two windows that
  overlap on one day, and refuses a window shorter than the appointment it
  offers, because that window can only ever produce zero slots.
- **Name and email are structural.** They are not part of the field schema and
  cannot be removed: an appointment nobody can be told about is not a booking.
  `fields` is what a *particular* business needs on top of them — a phone
  number, a registration plate, which treatment — each with a machine-stable
  `key` that outlives its label, a kind (`text`, `long_text`, `phone`,
  `choice`), and options exactly when it is a choice.
- **The calendar carries no foreign key.** Agenda owns a calendar's lifetime; a
  booking service must neither block its deletion (`RESTRICT`) nor disappear
  with it (`CASCADE`). Tenancy is enforced the way every other site-owned row
  enforces it — `(tenant_id, site_id)` referencing `sites`, every statement
  scoped by both — and the binding's validity is a resolution, not a constraint.
- **Rejected:** deriving availability from the calendar's free/busy alone. It
  needs no model and no screen, and it offers a stranger 03:00 on Sunday
  whenever the week happens to be empty.

### The public booking flow (S2.13b)

The `booking` section is the fourteenth section type, and it carries only the
stable service id and an optional heading. Everything a visitor reads about the
service — its name, its length, where it happens, the questions it asks — comes
from `site_booking_snapshots`, frozen at publish exactly as a catalog is: an
owner who shortens a consultation on Tuesday afternoon has not changed what the
page promised on Tuesday morning. A service switched off before the publish
renders the sentence that says so, rather than a form that could only fail.

- **Free time is never frozen and never cached.** A published page is bytes
  held per publish; a Tuesday afternoon is not. So the section renders what is
  offered plus a day field, and the free times live one navigation away on
  `GET /b/{booking id}?date=…` — `no-store`, read live, no JavaScript at all.
  The visitor picks a time, answers the questions, posts to the same path, and
  lands on a confirmation. Two requests, exactly like the order form's one.
- **Slots are arithmetic, and the arithmetic is pure.** `site_booking_slots`
  takes the published week, the busy intervals, a day and an instant, and
  returns the free slots. A window is cut into appointment-plus-buffer steps;
  the notice and the horizon trim both ends; a busy interval removes what it
  overlaps once the buffer is applied on both sides. Wall times are resolved
  through the service's own zone, so the hour that does not exist on the day
  the clocks go forward is never offered and the hour that happens twice is
  offered once — a property that is cheap to test precisely because nothing
  here touches a database.
- **Busy means the calendar *and* the ledger.** Availability subtracts the
  bound calendar's events — read through the `site_agenda` seam, so recurrence
  expansion and moved occurrences stay Agenda's to decide, and nothing but a
  start and an end ever crosses — plus every live appointment already taken on
  that calendar. The second half is what keeps availability correct in the
  instant between committing a reservation and writing its calendar event.
- **Two visitors, one slot: settled by Postgres.** A live appointment is unique
  on `(tenant, booking, starts_at)`; the reservation takes a transaction-scoped
  advisory lock on the calendar, re-checks for an overlap, and inserts. The
  loser is told the time has just been taken. Six concurrent bookers of one
  slot produce one appointment, and a test proves it.
- **The row is the reservation; the event is the owner's view of it.** The
  appointment is committed first, then written into Agenda through the owner's
  own account door (`site_agenda::agenda_door` — the one place an anonymous
  request crosses into an owner-scoped store, reachable only with a tenant and
  a calendar owner a published row already named). If the event cannot be
  written the reservation is withdrawn: an appointment the owner will never see
  is worse than a visitor asked to try again.
- **One uniform absence.** Unknown, unpublished, superseded, switched off, and
  calendar-deleted all resolve to nothing, and the wire answers `404` for every
  one of them — no existence leak, exactly as the order door behaves.
- **Privacy.** An appointment stores what the visitor typed — a name, an
  address to confirm to, and the answers, each labelled as it was read — and
  nothing about their connection. A test asserts the ledger's schema carries no
  IP, user agent, referrer, session or fingerprint column.
- **Rejected:** rendering the free times into the published page. It would make
  every page uncacheable, or wrong.

### Order forms (S2.12b)

A catalog carries `orders_enabled`, and the flag is **frozen into the
snapshot** with everything else. What the published page offers and what the
public door accepts are therefore the same fact, and switching ordering on or
off takes effect at the next publish — exactly like a price. A rendered
catalog with the flag set becomes one `<form method="post" action="/o/{catalog
id}">`: a quantity field per available item (`qty-<handle>`), then name, email,
optional phone and note, the same visually-hidden honeypot the contact form
uses, and the sentence saying nothing is paid here. There is no JavaScript
behind it — the browser posts and lands on the service's own result page — so
an order can be placed with scripts off entirely.

An order is a **request**, not a sale. It is the deliberate no-checkout half of
ADR 0041: no payment, no reservation, no stock. `site_orders` /
`site_order_lines` record who asked, for what, at which price, and the owner
moves it through `new → confirmed → fulfilled → cancelled` (and back — a
mistaken press is corrected, not re-typed). Three properties are structural
rather than checked:

- **Prices come from the publish.** The door
  (`site_public_orders::place_public_order`) resolves the catalog id to the
  snapshot of the site's *current* publish and reads names and unit prices
  there; the request carries handles and quantities only, so a posted price is
  unrepresentable. An unpriced item is ordered as "price on request" and its
  line total stays NULL — never zero, which would read as free.
- **The tenant is never named from outside.** It comes out of the resolving
  read and is what the insert scopes itself to, the same construction the
  public form write uses. An unknown id, a draft site, an unpublished site and
  a catalog published with ordering off are all one `Ok(None)` → one uniform
  `404`.
- **An item that is not on the published page cannot be ordered.** An unknown
  handle and a sold-out item are refused with a sentence the visitor can act
  on ("reload the page and try again"), because a stale page is a real and
  recoverable situation.

Abuse controls are the contact form's, reused: the per-client rate limiter
(shared budget with `/f/…`), a body cap on the route, the honeypot's silent
success, and caps in the write gate (50 distinct items, 999 of one item, a
2 000-character note). Nothing about the connection is stored — an order is
what was typed into it plus what the publish said it costs.

New orders reach the owner the way submissions do: `notified_at` is claimed
at-most-once by a sweep (`claim_order_notifications`) and delivered
**internally** to the site creator's inbox, listing the lines and the total,
with `Reply-To` set to the customer. Nothing is ever sent outbound. The
tenant-side routes are `GET /sites/{id}/orders`, `PUT /sites/{id}/orders/{o}`
(status), `DELETE /sites/{id}/orders/{o}` (a customer's erasure request takes
the lines with it) and `GET /sites/{id}/orders.csv` — one CSV row per ordered
line, so the numbers can be summed, with the same spreadsheet-formula
neutralisation the submissions export uses. There is deliberately **no
tenant-side create route**: the only writer of an order is the public door,
which prices it from the publish.

### Sandboxed custom code (S2.14a)

ADR 0036 ruled custom code out of the **first** wave; `docs/features.md` carries
it back at tier `[S+]` as "custom-code blocks (sandboxed)". Both halves of that
phrase are honoured: what the ADR forbids without a time limit — third-party
embeds and trackers of any kind — stays forbidden, because **no capability
opens a network**. A block cannot fetch, cannot load a remote script, font, or
pixel, and cannot phone home. The "no cookies, no banner" promise the analytics
model rests on therefore survives a tenant pasting a snippet from the internet;
what it costs is the YouTube embed, and that is a decision, not an oversight.

The block is a **document, not a fragment** (`alo_store::site_custom_code`):

- `html`, `css`, and `js` are stored **apart**, so nothing is smuggled — a
  `<script>` inside the markup is a validation error, not a surprise. So are the
  document's own tags (`<html>`, `<head>`, `<body>`, `<base>`, `<meta>`),
  anything that loads or embeds (`<link>`, `<iframe>`, `<object>`, `<embed>`),
  and `<form>` (its post has nowhere to go — that is what the contact form
  section is for).
- **Capabilities are explicit and default-deny.** There are two: `scripts` (the
  block's `js` runs) and `inline_images` (`data:` images decode). A script
  without its capability is refused, and so is the capability without a script —
  least privilege is checked, not merely offered.
- **The contract is computed, not written twice.**
  `CustomCodeCapabilities::sandbox_attribute` and `content_security_policy` are
  the exact strings the renderer emits, so the write gate's promise and the
  browser's instruction cannot drift. The policy floor is `default-src 'none';
  base-uri 'none'; form-action 'none'; style-src 'unsafe-inline'`, and each
  declared capability adds exactly one directive.
- **Sizes are bytes**: 16 KiB of markup, 8 KiB each of style and script, 24 KiB
  together — under the sum, so one page cannot carry fifty maximal blocks past
  the page budget. Height is authored (40–2 000 px): a sandboxed frame cannot be
  measured from the page without sharing an origin with it, which is the one
  thing that must never happen.

The renderer serves the block as `<iframe sandbox="…" srcdoc="…">` carrying a
complete document with the policy in a `<meta http-equiv>`. `allow-same-origin`
is unreachable from any capability, and so are `allow-top-navigation`,
`allow-popups`, and `allow-modals` — a block cannot read the page around it,
move it, or cover it. The quotes in the policy are escaped **twice**, because
there are two parsers between the source and the browser's policy engine.

**Where the boundary is.** The isolation is the browser's: an opaque origin plus
a closed policy. The write-gate refusals are the *helpful error* — they catch
the snippet that would silently do nothing and the shapes that would break the
wrapper document (`</` inside an inlined block, a control byte). Security that
depends on string-matching hostile input is not security, and the module says so
rather than implying otherwise.

**Two things the assistant may not do.** It may not write a custom-code block
(`alo-ai` refuses a generated draft that contains one, and refuses `add_section`
/ `set_prop` / `rewrite_copy` on one), and a template may not ship one — a
template is code we put in other people's sites, and shipping executable
JavaScript that way would make the catalog a supply chain. Moving and deleting a
block stay allowed: both are reversible arrangement that changes no code.

**The editor (S2.14b)** is `web/src/sites/CustomCodeFields.tsx`, and it says the
boundary out loud *before* the first field: sealed from the site, no network,
and nobody vets it. The two capabilities are switches, default off, each with
the consequence of leaving it off written beside it. The script field exists
only while the block is allowed to run one, and the biconditional the store
checks holds by construction on the way out: switching the capability off saves
the block **without** its script rather than saving bytes the browser is
forbidden to execute, and the form says so while the switch is being flipped
rather than after the refusal. The only rule the web repeats is the byte caps,
and only to *count*: the counters never block a save, so a cap that moves in
Rust makes a counter stale, never a save impossible. There are no copy tools
anywhere in this form — the assistant refuses to touch code by name, so the
affordance would only produce a refusal. The draft preview needs nothing of its
own: the pane renders the page server-side with the publishing renderer, and its
own `sandbox="allow-scripts"` frame can only *narrow* what the block's nested
frame is granted, so what the owner sees is never more capable than what the
internet gets.

### Buying a domain name (S2.15)

Connecting a domain you already own (`/sites/{id}/domains`, S1.25) and *buying*
one are two different surfaces, deliberately. The first proves ownership with a
TXT record; the second spends money, hands a registry somebody's home address,
and has a state machine behind it — `alo_store::site_domain_purchases`, whose
row *is* the state (`quoted → approved → awaiting_payment → paid → registering
→ registered → configured`, with `cancelled` reachable only before money moved).

The edit-side routes (`products/mail/alo-jmap/src/sites_domain_purchases.rs`):

| Route | What it does |
|---|---|
| `GET /sites/domain-catalog` | the endings sold, both prices on each, plus who the reseller is and whether its calls spend money |
| `GET /sites/domain-search?q=&tlds=` | one offer per candidate: available/taken/blocked/unsupported, priced **only** when available |
| `GET`/`POST /sites/{id}/domain-purchases` | this website's purchases; start one |
| `GET /sites/{id}/domain-purchases/{p}` | one purchase |
| `GET /sites/{id}/domain-purchases/{p}/registrant` | the personal data, behind its own door |
| `POST /sites/{id}/domain-purchases/{p}/approve` | agree to **this exact price** |
| `POST /sites/{id}/domain-purchases/{p}/checkout` | record the payment's opaque reference; nothing is charged by this |
| `POST /sites/{id}/domain-purchases/{p}/cancel` | call it off, before money moved |
| `POST /sites/domain-payments/settle` | **not a tenant's route** — the payment bridge saying a charge arrived |

Three properties are the reason this is a module of its own:

- **The price is never posted.** A create request names a domain and a term;
  what it costs is asked of the registrar in that same request and stored from
  that answer. No client bug and no tampered body can put a number on a
  purchase that the seller did not state. Approval is the mirror image: it
  *must* echo the six numbers that were on screen, and the store refuses any
  disagreement — a price that moved stops there rather than being silently
  re-quoted. The renewal price is one of the six, because that is the half a
  bait price hides in.
- **Buying is the site owner's, not the site editor's.** The whole surface sits
  behind the same guard collaborator management uses (admin, or the person who
  created the site), so a site-editor collaborator (S2.03a) may write the
  website and may neither spend the tenant's money nor read a registrant.
- **An unconfigured deployment says so.** Two environment variables decide:
  `SITE_REGISTRAR` (`fixture` selects the deterministic in-memory reseller for
  local development; anything else, including unset, is the
  `UnconfiguredRegistrar`) and `SITE_NAMESERVERS` (comma separated — a name we
  cannot point anywhere is a name we do not sell). Without them every door
  answers `503` with `{"reason":"unconfigured"}`, the same typed shape the AI
  paths use, so a buy box can hide itself instead of failing at the price.
  **Production leaves `SITE_REGISTRAR` unset**: wiring a real reseller is an
  ADR, not a deployment guess.

#### The payment handoff and the registration sweep (S2.15c2)

"The money arrived" is the one statement a buyer may not make about their own
purchase: it is what queues the registration, so a tenant who could say it would
register domains nobody paid for. The two doors therefore sit on opposite sides
of the money. `…/checkout` is the tenant's and only records the opaque reference
whatever charges them minted — recording a reference charges nobody.
`/sites/domain-payments/settle` carries no user token at all; it presents the
deployment's secret (`SITE_PAYMENT_SETTLEMENT_SECRET`, in the `X-Alo-Settlement`
header) and names the charge by tenant and payment reference, which the schema
makes unique per tenant. **Unset secret → `503 unconfigured`**, never an open
door. The settlement is still written through the door of the person who
approved that exact price: a row that says a machine did it is a row that says
nobody did.

`site_domain_worker` then does the rest, one tick a minute, and only in a
deployment that sells domains at all:

- the claim marks each paid purchase `registering` in the statement that reads
  it, so two sweepers never register one name twice; the registrar call carries
  the purchase id as its idempotency key, so a sweep that dies after the
  registry answered replays rather than buys again;
- a fault that repeating could survive (provider timeout, a registrar unwired
  under a paid purchase) goes back in the queue, bounded by
  `SITE_DOMAIN_PURCHASE_MAX_ATTEMPTS` so it ends visibly instead of circling;
  a refusal is terminal with a sentence about the refund, because a person now
  has to act and a retry loop only delays them finding out;
- a registered name **attaches itself**: the custom-domain claim is written
  straight to `live` (alo registered it, on alo's nameservers — there is nothing
  left to prove by TXT record), which is the whole point of buying inside alo.
  An attachment that is refused leaves the purchase `registered` rather than
  `failed`: the tenant does hold the name, and saying otherwise would be a lie
  about their money.

#### The screen (S2.15c3)

`web/src/sites/DomainsView.tsx`, at `/sites/{id}/domains` and reached from the
publish bar — beside the line that says where the website lives, not among the
content screens. It composes three panels in the order of the decision:

- **`ConnectedDomains.tsx`** — the path that works on every deployment. The TXT
  proof is the server's own three strings, copyable one by one; a check that has
  not found the record is a normal answer with a sentence about DNS travel time,
  never a red failure; and the CNAME/ALIAS last step is stated out loud once the
  claim is verified, because proving ownership is not pointing the domain at
  anything.
- **`DomainBuyPanel.tsx`** — search as you type (400 ms after the last key,
  sent verbatim: the server understands `acme`, `Acme.com` and a pasted URL).
  It computes nothing. Every number on screen came from the answer being
  displayed, both halves of every price appear together, and an offer nobody can
  buy carries none. A fixture reseller is badged from `registrar.spendsMoney`
  rather than hidden, and `buyable: false` disables buying while leaving prices
  readable. On `503 unconfigured` the panel becomes the server's own sentence,
  which already names the way on — this is what production shows today.
- **`DomainPurchaseDialog.tsx`** — two deliberate steps. Step one sends no price
  at all (`domain`, `years`, `autoRenew`, `requestKey`, `registrant`) and holds
  one replay token for its lifetime, so a network wobble cannot buy a second
  name; a country typed in capitals is lowercased rather than refused, which is
  a form normalizing its input, not a second copy of a rule. Step two shows the
  stored quote and approves by echoing its exact six values.
- **`DomainPurchaseList.tsx`** — the record and the recovery surface, since half
  the arc happens with nobody watching. Each row states what is happening and
  what happens next (`domainPurchaseState.ts`, the only place a state becomes a
  sentence), a failure reads the server's own words about the money, and the two
  actions that still have routes — approve a `quoted` price, call off anything
  before `moneyMoved` — live here so closing the dialog strands nothing.

**No pay button.** `…/checkout` records a reference a payment bridge minted, and
nothing in alo mints one yet (S2.15c2), so the screen states the arc — approval,
then payment, then automatic registration and attachment — rather than offering
a button no route completes. Wiring a real PSP is ADR work (S3.04c's shape).

## Errors

Edit side (`alo-jmap`, RFC 9457 `Problem` bodies like every module):

- Unauthenticated → `401`; authenticated but wrong tenant/user →
  the id simply does not resolve → `404` (the account-door pattern:
  wrong-tenant is indistinguishable from nonexistent).
- Validation (bad subdomain, reserved word, slug collision, section
  JSON failing the typed schema, oversized theme/logo, post doc ref
  outside the tenant) → `422` with a field-level detail.
- Subdomain taken (any tenant) → `422 subdomain_taken` — taken/free
  only, no owner information.
- Publish with zero pages / no home page → `422`.
- AI envelope that fails schema parse after one repair retry → typed
  error the UI surfaces as "couldn't apply, nothing changed".

Public side (`alo-sites` — terse, static, no internals on the wire):

- Unknown host or unpublished site → branded `404` page (no tenant
  existence leak: unknown subdomain and unpublished site are
  identical).
- Unknown path on a live site → the site's `404` page.
- Form: unknown form id → `404`; body over size cap → `413`;
  malformed → `400`; rate-limited → `429` with `Retry-After`;
  honeypot tripped → `200` (silent drop — bots learn nothing).
- TLS "ask" endpoint: unverified domain → non-200, so Caddy never
  issues a certificate for a domain we haven't verified.

## Tenancy

- Every `site_*` table carries `tenant_id`; store access goes through
  the existing doors (`for_tenant` / `for_account`) so the scope
  predicate is baked into every statement — wrong-tenant reads return
  `NotFound`/empty, never data, never 500. **Wrong-tenant tests are
  mandatory on every `site_*` store module** (the queue repeats this
  per item).
- The public service holds no session: its tenant scope is derived
  **from the Host lookup result** — one global indexed read maps host
  → (tenant, site), and every subsequent read (snapshots, posts,
  forms, analytics) is scoped by that pair. The Host-isolation
  integration test proves site A's host cannot serve site B.
- Deliberate global surfaces, and the only ones: the `subdomain` unique index
  (leaks taken/free only), the globally unique normalized custom-domain claim
  (one Host can name only one site), the host→site resolver (public data by
  definition), and the opaque random form id used as the anonymous submission
  capability. All subsequent reads and writes are tenant/site scoped; the
  form id reveals no tenant data and has no public read-back route.
- Form submissions and analytics write **into** a tenant's scope from
  anonymous traffic; the writable set is exactly {insert submission,
  bump aggregate} for the resolved site — no read-back surface exists
  publicly.

### The restricted collaborator, as a model (S2.03)

The marketing person who builds the website must not thereby read the mail,
the files, or the customers. Two mechanisms, deliberately separate:

- **The role is the global signal.** `site_editor` joins `accountant` and `hr`
  in `tenant_user_roles`; carrying it closes every non-Sites API door in the
  workspace by default, so a module added tomorrow is shut to a collaborator
  without anyone remembering to shut it.
- **The grant is the resource boundary.** `site_editor_grants` names the sites
  that are then *open*, one row per site. Closed-by-role plus opened-by-grant
  is what makes "this website and nothing else" expressible; the authorization
  matrix that walks the resulting surface is the S2.16a section below.
- **The invitation is a one-time link** (`site_editor_invites`, S2.03b). The
  raw token exists only in the URL shown to the inviter — the database keeps
  its SHA-256 hash, with an expiry, and a token is redeemable once and only
  while unaccepted. The accepted row is retained so the account can be
  recognised as invite-created, which is what lets **revoking the final site
  grant remove the restricted account's role with it** rather than leaving a
  member of the workspace behind with nothing to do and a login that works.

### Who may use the edit surface, as built (S2.16a)

The wave review re-walked the whole authenticated surface — 85 route templates
under `/sites/*`, against the grant that was written when there were a dozen.
What it found and what it fixed:

- **The API answers at two addresses, and the browser only uses the second
  one.** `server::routes` mounts the entire router a second time under `/api`,
  and production Caddy proxies `/api/*` and nothing else — so every
  authenticated request a real user makes is `/api/sites/…`, and the bare path
  is a test-only address. Three middlewares read the matched route template;
  only [`module_access`] normalised the mount away. The site-editor branch of
  [`scoped_roles`] matched `"/sites/{id}"` literally, so **every restricted
  collaborator was refused every route, including their own site**, at the only
  address the product uses — while the bare-path tests stayed green. Fixed by
  reading both the template and the path through `without_api_mount`, and both
  mounts are now asserted in the same test. The rule for anything added later:
  a gate that reads a template must normalise the mount, and its test must
  knock at `/api/…`, because that is the door.
- **The middleware decides the *surface*, the handler decides the *door*.**
  `enforce_scoped_roles` says yes to any `/sites/{id}…` template once
  `can_edit_site` holds — by construction, so a route added tomorrow inherits
  it. What separates building a website from running the business behind it is
  therefore a per-handler guard, and the review knocked on each one:
  `require_site_manager` for the collaborator list and for every
  `domain-purchases` route (money), and `require_crm_reader` for the leads and
  attribution routes (CRM/Billing identities). All held. The matrix is pinned
  by `the_editor_matrix_holds_over_the_surface_added_after_the_grant`.
- **A collaborator reads the records the website produced** — form
  submissions, orders, bookings, analytics — because those are Sites' own
  records and answering an enquiry is part of the job the invitation is for.
  It is a wider grant than "edit these pages", and it is stated here rather
  than left to be discovered from a route table: an outside contractor invited
  to a site can read the names and email addresses of the people who wrote to
  it. Narrowing it is a product decision, not a bug fix; if it narrows, the
  place to say so is the invite screen, which today promises only editing.
- **Two doors are deliberately open to anyone**, and neither is a session:
  the invitation token routes (a person accepting an invitation has no account
  yet) and `POST /sites/domain-payments/settle`, which carries the deployment's
  settlement secret because a tenant may not declare their own payment
  settled. Both are documented at their handlers.
- **The Wave-3 re-walk (S3.06a).** The assistant's doors all held: every
  `chat-*` route — the switch and monthly spend ceiling, appearance, the
  action transcript, the Public-knowledge collection — is owner-only by its
  own handler (ADR 0040), and the shop-setup proposal, which names Billing
  prices and VAT, sits on a static path the collaborator allowlist never
  matches. The commerce doors did not hold: the ticket and shop pickers
  (`ticket-products`, `shop-products`) answered a collaborator with the
  tenant's whole active price list — a Billing read the role exists to
  close — and the sale verbs (event create/capacity/delete, shelf
  add/remove, the delivery rate) let an invited outsider change what the
  business sells. All of those are now owner-only through
  [`require_commerce_site`], with the refusal spoken: what a website sells
  and charges is the business's decision, not the website builder's. What
  stays a collaborator's read — the event list, the shelf as listed, and the
  delivery rate — are facts the published pages already state to strangers,
  and what the page editor's section forms need. The pin test knocks on the
  whole Wave-3 surface at the `/api` mount, and the Shop and Tickets screens
  degrade to a stated read-only view for a collaborator rather than
  discovering the refusals one click at a time.

## Out of scope (v1 — cuts are decisions)

- E-commerce checkout (S+ / ADR 0041's later waves). The catalog storefront
  has since shipped as order-by-form, and the booking *model* — the service,
  its week, its questions, and the Agenda seam — landed with S2.13a; reading
  availability and taking a reservation are S2.13b, the editor screens S2.13c.
- Free-form design tools / pixel canvas, template marketplace,
  third-party embeds or trackers of any kind (ADR 0036 non-goals; the
  analytics promise depends on it). Custom code was on this list for v1
  and has since shipped **sandboxed** at tier `[S+]` — with no network,
  so the embeds-and-trackers half of the ADR is untouched (above).
- Version history, rollback, whole-site AI translation, scheduled
  publishing, password-protected pages and responsive image derivatives
  have since shipped and are described above.
- CRM lead creation from form submissions (waits for business-track
  B2; the seam is the stored submission).
- Production serving infrastructure: the public domain and wildcard DNS are
  ready, but adding the `alo-sites` container and Caddy wildcard/custom-host
  routing remains a human deploy action recorded in the Sites STATE inbox —
  the loop never touches `deploy/`.
- Multi-site-per-tenant limits, quotas, and billing integration —
  unlimited sites per tenant in v1; revisit with billing.

**Rejected alternative (section model):** a free-form block/canvas
model (arbitrary nested layout tree, absolute positioning) was
rejected because the AI cannot reliably read or edit it, every AI
change would be an un-reviewable pixel diff, and rendering it fast
and accessible is a research project — typed sections give the model
a closed vocabulary, give users a form-based editor, and give the
renderer a finite, golden-testable surface (ADR 0036).

**Rejected alternative (serving):** serving public traffic from
`alo-jmap` behind a path prefix was rejected because it would put an
unauthenticated, internet-facing surface inside the workspace API
process — a separate `alo-sites` binary keeps the blast radius, cache
behavior, and scaling profile of anonymous traffic away from tenant
data paths.

## What S1 promised, and what S1 shipped (S1.31b)

Every `[S1]` line in `docs/features.md` § alo Sites is reconciled here. No
feature is silently absent: each is shipped or names its explicit dependency.

| `[S1]` feature | State | Where / narrowing |
|---|---|---|
| ★ AI builds the first draft | **Shipped** | `alo-ai` and the authenticated Sites routes exchange a strict full-site envelope, allow one repair turn, and store only after review. Fixture providers cover unattended tests; production still needs a human-configured tenant provider. Creation now shows the full address and opens the generated Home editor directly (S1.30b/c). |
| Section-based editor | **Shipped** | All twelve typed section variants have visible add/edit/move/remove controls and schema-checked writes. A new empty Home page opens its Hero form in one click; no pixel canvas or hidden core action. |
| Themes | **Shipped** | Seven validated palette/typography presets and Drive-backed logo/favicon references feed the single generated stylesheet. V1 deliberately keeps colors inside accessible presets rather than exposing unsafe free-form token editing. |
| ★ Static Rust rendering | **Shipped** | `alo-sites` renders semantic, escaped, SEO-complete HTML with near-zero JS from the same library used by authenticated preview; golden and byte-budget tests pin it. |
| Publish snapshots and instant subdomain | **Shipped** | Atomic immutable snapshots are the public service's only page source. Drafts and unpublished sites resolve like unknown hosts. The configured full address is visible and checked during creation. |
| ★ Custom domains | **Shipped**, deployment pending | TXT proof creates a `live` normalized claim; Host serving and the Caddy ask endpoint accept only currently serveable published sites. Customer DNS guidance is recorded below; production Caddy/container work remains human-owned. |
| Contact forms | **Shipped**, one stated dependency | Contact sections create their form records; public submit has caps, honeypot and rate limiting; submissions are reviewable/exportable CSV; internal notifications are claimed at most once. Notification server copy is English in S1. **CRM lead creation remains intentionally deferred to B2**, exactly as the feature line states. |
| ★ Blog written in alo Docs | **Shipped** | Tenant-owned Docs become draft/published posts; the public index paginates, post HTML is sanitized, and RSS is served. |
| SEO | **Shipped** | Per-page overrides, Open Graph, canonical URLs, sitemap and robots are rendered from published state. |
| ★ Privacy-first analytics | **Shipped** | Daily path/referrer aggregates and exact day-scoped uniques use opaque HMAC tokens, plus campaign, country, device-class and entry/exit aggregates derived at the door, and read-time buckets and outbound domains reported by the page beacon. No cookies, raw IP, UA, query, full referrer, exact durations, or cross-day visitor identity is stored; a scriptless visitor is still a counted visit. |
| ★ AI copy tools per section | **Shipped** | Exact-field and whole-page proposals show reviewable before/after content; approve is the sole write and selecting a different field cannot silently retarget a proposal. |

**Languages.** The complete Sites surface is translated in English, French,
and Dutch. A catalog parity test fails when a new Sites key lacks either
translation.

**Human production inbox.** At the next production deploy, add the
`alo-sites` service with `SITES_DOMAIN=alosites.com`, blob/database settings,
and a strong `ALO_SITES_ANALYTICS_SECRET` (at least 32 bytes); route workspace
`/sites` traffic to `alo-jmap`; route wildcard and custom public Hosts to
`alo-sites`; and connect Caddy on-demand TLS to `/internal/tls/ask`. Configure
the live tenant's OpenAI-compatible provider in Settings (base URL, model,
and key when the provider requires one). Customer DNS help must say: keep the
shown TXT proof until verification succeeds, then CNAME a subdomain to the
deployment ingress; an apex needs the DNS host's ALIAS/ANAME or CNAME
flattening equivalent. Certificate issuance may take a few minutes. After
launch, submit `alosites.com` to the Public Suffix List.

## What S2 promised, and what S2 shipped (S2.16c)

Every `[S2]` line in `docs/features.md` § alo Sites is reconciled here, on the
same rule as the S1 table above: shipped, or naming its explicit dependency.
Four `[S+]` lines were reached early in the same wave and are listed after it,
because a promise kept ahead of time still has to be written down somewhere a
stranger will find it.

| `[S2]` feature | State | Where / narrowing |
|---|---|---|
| ★ Whole-site AI translation with language switcher | **Shipped**, one stated dependency | One page identity with per-language drafts (`site_page_locales`), a language contract frozen into every publish, prefixed locale URLs with `hreflang`/`x-default`/canonical, locale-aware sitemap and feed, and a switcher built from the siblings that actually exist. The **manual** path is complete on its own — readiness coverage, copy-to-new-language, publish — so multilingual publishing never requires a model. The AI proposal envelope is whole-site, before/after, approve-only, and was verified on fixtures; live calls need the tenant's configured provider. |
| ★ Collections from alo Base (the CMS layer) | **Shipped** | A site-scoped binding to one Base table plus a field mapping by stable field id; publish freezes the resolved rows into immutable snapshots, so editing the table cannot change the live site until the next publish. Connect / map / preview / disconnect are visible manual controls. Filling or translating the rows is Base's own propose-then-approve surface, not a second copy here. |
| Site-editor role | **Shipped** | `site_editor` closes every non-Sites door; `site_editor_grants` opens the named sites; invitations are one-time hashed tokens, and revoking the last grant removes the restricted account's role. The wave review found and fixed the defect that mattered: the role check missed the `/api` mount the browser actually uses, so collaborators were refused everywhere including their own site. Both mounts are asserted now. A collaborator does read the records the site produced (enquiries, orders, bookings) — stated in the authorization matrix above rather than left to be discovered. |
| Version history + rollback; scheduled publishing; password-protected pages | **Shipped** | History lists and compares the immutable publishes and restores one by appending a new publish (never by re-pointing an id), with Undo; scheduling is an intention swept every 30 seconds through the scheduling user's own door; page passwords are argon2id with an opaque session version, rate limits, cache-safe responses, and protected pages kept out of the sitemap. |
| Image handling: crop/focus, AI alt-text, responsive srcset | **Shipped**, one stated dependency | Crop and focal point are basis points of the source (never pixels, never floats) with validation that the two cannot contradict each other, `decorative` distinguishing "nothing to describe" from "not written yet"; publishing emits `srcset`/`sizes` over a 480/960/1440 ladder whose cropped fallback is the widest derivative. Alt text is typed by hand in the editor; the AI suggestion is propose-then-approve and fixture-verified, live calls needing the tenant's provider. |
| ★ Site Insights (campaigns, countries, device class, entry/exit, read time, outbound clicks) | **Shipped**, one deployment dependency | Five dimensions are derived at the door and two more (read-time buckets, outbound domains) are reported by the page beacon, all as daily aggregates with bounded label sets. **Countries stay empty until the edge proxy sends one** — `cf-ipcountry`, `x-country` or `x-geo-country`; alo never resolves an address to a place itself, so this is a deployment step, and the screen says so rather than showing a false zero. |
| ★ Aggregated heatmaps | **Shipped** | Clicks become cells of a fixed 32×64 grid over the whole scrollable page, scroll depth becomes one of ten tenths, viewport becomes one of three classes, and the table has **no visitor column at all**. Presentation adds a floor storage does not: nothing is drawn below twenty samples, and every shaded square is repeated in words. |
| ★ Conversions + full-funnel attribution | **Shipped**, the stated B1/B2 seam | View/start/submit counted per site-owned form id with no visitor identity; the handoff to CRM is a person's decision creating at most one lead per enquiry, and the invoice join is stated on the wire (`invoiceRule: customerSinceLead`) rather than inferred. Deals and invoices are read live from their owners — Sites keeps one link row and copies nothing. With Billing switched off the money reads `null`, not `0`. |
| **Non-goal:** no individual journeys, no session replay, no fingerprinting | **Held** | Every table in the analytics family is proved by a schema test to have no column for an address, a user agent, a full referrer, a query string, an exact duration, or a cross-day identity. The one visitor token that exists is day- and site-scoped and dies with the day. |

Reached early, from `[S+]`:

| `[S+]` feature | State | Where / narrowing |
|---|---|---|
| Simple catalog storefront (order-by-form, no checkout) | **Shipped** | Tenant-owned catalog/categories/items in integer minor units with the currency's own exponent, a Base import that updates what it created rather than duplicating it and refuses an ambiguous price, publish-time snapshots, and public order forms whose totals are the published ones. No payment, no reservation, no stock — exactly as ADR 0041 draws the line. |
| Booking-page section (ties to Agenda) | **Shipped** | A Sites-owned booking service reads availability through one seam onto Agenda, reservation is race-safe, the appointment lands in the owner's calendar, and a second telling arrives in their inbox by the same claimed-once sweep the forms use. |
| Custom-code blocks (sandboxed) | **Shipped** | A whole document in an opaque-origin `srcdoc` frame with an explicit CSP, no network, capability switches off by default, size caps, and an editor that states the boundary before the first keystroke. The ADR's no-third-party-embeds line is untouched: there is no network to embed from. |
| Template gallery | **Shipped** | Six curated templates parsed from the build with no table and no per-tenant row, four rules asserted by the suite (no images, no claim only the customer can make, every internal link resolves, one persistence door), and a keyboard-navigable gallery beside — never behind — the description field. |
| ★ Sell domains in-product | **Model, routes and screen shipped; not sellable** | The registrar is an injectable boundary with a fixture provider, the purchase is a state machine from quote through explicit approval to registration and attachment, and the screen shows both prices before anything is approved. **Production ships `UnconfiguredRegistrar`**: no reseller is named, `SITE_NAMESERVERS` is empty, so buying is off and the screen offers connecting a domain you already own. Naming the EU reseller and the PSP is an ADR, below. |
| ★ alo-run authoritative DNS | **Not started** | Still `[S+]`. Nothing in S2 depends on it: custom domains verify by TXT at the customer's own DNS host, and a bought domain would be created with whatever nameservers `SITE_NAMESERVERS` configures. |

**Languages.** The complete Sites surface — every S2 screen included — is
translated in English, French and Dutch, and the catalog parity test fails the
build when a new Sites key lacks either translation.

**Human production inbox, S2 additions.** Everything in the S1 inbox above
still applies. On top of it:

- **New public paths on `alo-sites`**, all of them POST doors that must reach
  the service rather than the SPA: `/f/{form}` (enquiries), `/o/{catalog}`
  (orders), `/b/{booking}` (appointments) and `/_alo/collect` (the page
  beacon). They are served on the *public site* hosts, not on the workspace
  host — a wildcard/custom-Host route that already sends everything to
  `alo-sites` needs no per-path rule; a rule-by-rule proxy needs these four.
- **Workspace API prefix:** the browser reaches every authenticated Sites
  route at `/api/sites/…`. Caddy proxies `/api/*` already; **no new top-level
  prefix was added by S2** — the bare `/sites/*` mount is test-only.
- **Country dimension:** set one of `cf-ipcountry`, `x-country` or
  `x-geo-country` on the edge proxy if the country breakdown should have
  values; without it the panel stays honestly empty.
- **Background sweeps run inside `alo-jmap`** and need no separate process:
  contact-form, order and booking notifications (30 s), scheduled publishing
  (30 s), and domain registration (60 s, started only when the deployment
  sells domains at all).
- **Domain selling stays off** unless `SITE_REGISTRAR` names a provider
  (only `fixture` exists today), `SITE_NAMESERVERS` lists the authoritative
  hosts, and `SITE_PAYMENT_SETTLEMENT_SECRET` is at least 24 bytes. Any
  one missing leaves the buy box showing the connect-a-domain path, which is
  the correct production state until the ADR below is written.
- **Tenant AI provider** remains the single switch for generation,
  conversational editing, translation proposals and alt-text suggestions.
  Every one of them degrades to a stated manual path when it is absent.

**Open decisions this wave flagged, for a human.**

1. **Which EU reseller, and which PSP.** ADR 0041 named the shape; the
   provider is unnamed, and until it is, domain purchase is code without a
   counterparty. The payment handoff records an opaque Billing reference —
   the mint for it does not exist in alo yet either.
2. **How wide the site-editor invitation really is.** A collaborator can read
   the enquiries, orders and bookings the site produced, which is more than
   the invite screen promises ("edit and publish this website"). Narrow the
   grant or widen the sentence — a product decision, not a bug fix.
3. **The accent ramp fails AA for small text and for the primary button**
   (white on `--accent` is 3.09:1). Sites worked around it per rule; the fix
   is a ds-track decision about the brand ramp, not a per-module one.
4. **Place-of-supply VAT** for anything sold through a site is untouched by
   design: today's catalog takes orders, never money. The tax rules table is
   listed in wave 3 with the standing instruction that a professional reviews
   it before it is built.
