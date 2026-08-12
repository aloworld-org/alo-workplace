# Design note — alo Sites (marketing site + blog + forms)

Status: S1 as built · 2026-08 · ADR 0036 · Sites track wave S1

alo Sites is the AI-native no-code website builder: "tell me about your
business" produces a complete draft site, then editing is conversational
(propose-then-approve, the ADR 0034 trust pattern) or manual through
typed section forms. V1 ships a marketing site + blog + contact forms,
published instantly at `<subdomain>.<SITES_DOMAIN>` and optionally on a
live custom domain. This note records the as-built data model, web surface,
render pipeline, two-service boundary, and privacy posture after the S1 wave
review.

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
  `contact_form`, `footer` — each with typed props. Unknown section
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

## Out of scope (v1 — cuts are decisions)

- E-commerce checkout, catalog storefront, booking pages (S+ / ADR
  0035's later waves).
- Free-form design tools / pixel canvas, custom code injection,
  template marketplace, third-party embeds or trackers of any kind
  (ADR 0036 non-goals; the analytics promise depends on it).
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
