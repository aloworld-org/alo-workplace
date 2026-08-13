# sites/QUEUE.md — alo Sites work queue (ADR 0036, track: SITES)

Ordered; the loop takes the first item not `[x]`/`[!]`. One item = one
iteration = one commit+push. Standard gates always (clippy/tests/tsc/eslint,
wrong-tenant on storage, curl wire-verification on the local backend). Detail
source: features.md → alo Sites. Code areas for THIS track: `platform/alo-store`
site_* modules, `products/sites/**`, `alo-jmap` `/sites/*` routes + module
registration, `web/src/sites/**`, `platform/alo-ai` sites generation module.
Do not touch billing/crm/business areas — that's the Mac's track.

## Wave S1 — site + blog + forms + both domain modes

- [x] S1.01 Design note `docs/design/sites.md`: data model (sites, pages, sections JSON versioning, themes, posts, forms, domains), render pipeline, the two-service boundary (edit API in alo-jmap vs public `alo-sites`), form flow, analytics privacy model, tenancy, out-of-scope. Done when: implement-skill's four blocks + the rejected alternative for the section model are written.
- [x] S1.02 Migration + store: `sites` (name, subdomain globally-unique + reserved-word list, status draft/live, theme JSON, created/updated) tenant-scoped CRUD + wrong-tenant tests + subdomain validation (dns-safe, 3–40 chars).
- [x] S1.03 Section schema v1 as typed Rust (serde) in a `site_model` module: nav, hero, features, text_image, gallery, testimonials, pricing, team, faq, cta, contact_form, footer — each with typed props + a `schema_version` envelope; exhaustive serde round-trip tests + golden fixture JSON per section.
- [x] S1.04 Migration + store: `site_pages` (slug, title, ordered sections JSON validated against S1.03, SEO meta, nav order, home flag) CRUD + slug rules + wrong-tenant tests.
- [x] S1.05 Theme model: palette+typography presets (≥6 shipped) + logo/favicon blob refs; validation + tests.
- [x] S1.06 Renderer: `products/sites/alo-sites` crate, `render` module — page JSON + theme → complete HTML document (semantic landmarks, alt required on images, meta/OG/canonical) — golden HTML tests per section type and a full-page golden.
- [x] S1.07 Stylesheet generation from theme tokens (one CSS file, responsive, no JS beyond menu toggle + form submit); byte-budget test (CSS < 50KB, page HTML < 100KB for the golden site).
- [x] S1.08 Publish flow in store: immutable per-page published snapshots + site publish state; republish creates new snapshot; tests prove drafts never leak to the published set.
- [x] S1.09 `alo-sites` public service: axum, resolves Host header (`<sub>.<SITES_DOMAIN>` env) → tenant site → serves published snapshots with in-memory cache + proper cache headers; 404 page; /healthz; in-process integration tests incl. Host isolation (site A's host can never serve site B).
- [x] S1.10 Edit API in alo-jmap: `/sites/*` — site CRUD, page CRUD, section ops (add/update/move/remove), theme set, publish — auth + Problem errors + wire transcript (401/422/happy paths) in sites/STATE.md.
- [x] S1.11 Web module skeleton: `web/src/sites` — rail entry (workspace surface), site list + create (name → live subdomain check), page list; i18n en.
- [x] S1.12 Web editor core: section stack (add from a picker with thumbnails, drag-reorder, delete) + per-type prop forms + save; tsc/eslint/build clean.
- [x] S1.13 Live preview: authenticated draft-render endpoint in alo-jmap reusing the render lib; iframe preview pane refreshing on save; mobile/desktop width toggle.
- [x] S1.14 Theme UI: preset picker + logo/favicon upload via Drive; preview updates.
- [x] S1.15 Publish UI: publish button with "goes live at <sub>.<domain>" copy, live/draft status chips; STATE human-inbox note: production needs the alo-sites container + wildcard DNS/TLS + SITES_DOMAIN purchase.
- [x] S1.16a Forms store ONLY: migration + `site_forms` (per-site, links a contact_form section token) + `site_form_submissions` (name/email/message, received_at, handled flag) + tenant-scoped CRUD + wrong-tenant tests. NO HTTP in this item. (Split after six oversized attempts — one turn must bank it.)
- [x] S1.16b Public submit endpoint: POST `/f/:form_id` on alo-sites — validation, size caps, honeypot field, per-IP rate limit — writing S1.16a submissions; in-process tests incl. rate-limit + a foreign form_id yielding a clean 404. NO notification yet.
- [x] S1.16c1 Submission notification ONLY: on new submission, INTERNAL delivery of a notification message to the owner's inbox (never outbound SMTP) — build the RFC822, deliver via the existing store path, tests prove the message lands in the right tenant's inbox and a foreign tenant sees nothing. NO editor wiring in this item. (Split again 2026-08-08 03:45 after six hours of oversized attempts.)
- [x] S1.16c2 Form auto-create on section add: adding a contact_form section in the editor creates/links the form record; wire transcript of the complete arc (add section → public POST → submission row → owner-inbox notification).
- [x] S1.17a Submissions UI, list: per-site list with view + mark-handled; wire-verified.
- [x] S1.17b Submissions UI, export: CSV export; wire-verified.
- [x] S1.18 Blog model: `site_posts` (doc node ref, slug, title, excerpt, cover blob, published_at, status) + store + routes + tests (a post can only reference the tenant's own doc).
- [x] S1.19a BlockNote→HTML core: paragraphs, headings, lists, quotes — goldens from real doc fixtures.
- [x] S1.19b BlockNote→HTML rich: images, code, equations-fallback + the XSS-safety tests (script content never renders live).
- [x] S1.20a Blog pages on alo-sites: post pages + /blog index cards; goldens.
- [x] S1.20b Blog extras: pagination + RSS feed; goldens.
- [x] S1.21a Blog UI, authoring: posts tab list + "write in alo Docs" creates/links the doc + edit opens the Docs editor.
- [x] S1.21b Blog UI, publishing: publish flow with slug/cover/excerpt + status chips.
- [x] S1.22a SEO, served: sitemap.xml + robots.txt on alo-sites; goldens.
- [x] S1.22b SEO, edited: per-page meta editor UI + OG defaults from theme/logo; render goldens.
- [x] S1.23 Privacy analytics collection on alo-sites: daily aggregates (path hits, referrer domain, unique-ish via daily-salted hash), explicitly NO ip/ua storage — a test asserts the stored schema contains no PII columns and raw request data is dropped.
- [x] S1.24 Analytics UI: per-site panel (visits over time, top pages, top referrers) + the "no cookies, no banner" explainer string.
- [x] S1.25a Custom domains, model half: `site_domains` migration + store (domain validation, TXT verify token generation, status pending/verified/live) + `/sites/{id}/domains` routes + the verify-check endpoint (DNS TXT lookup mockable for tests) + wrong-tenant tests + wire transcript.
- [x] S1.25b Custom domains, serving half: alo-sites resolves verified custom Hosts + the Caddy on-demand-TLS "ask" endpoint (200 only for live domains/subdomains) + in-process Host-header tests; human-inbox note for the customer DNS how-to.
- [x] S1.26a AI generation, envelope: `sites` module in alo-ai — full-site draft envelope (description → site JSON) with strict schema parse; deterministic fixture tests (NO live calls); prompt documents the section schema.
- [x] S1.26b AI generation, repair: the one-retry repair path for near-miss outputs + its fixture tests.
- [x] S1.27 AI edit ops (alo-ai): typed op envelope (add/remove/reorder section, set prop, rewrite copy) + apply-to-page pure fn + tests; ambiguous op → typed error the UI can surface.
- [x] S1.28a Generation backend: POST `/sites/generate` (description → S1.26 envelope → draft site+pages created, never auto-published); unconfigured-AI path returns a typed "unconfigured" the UI can branch on; wire-verify fixture + unconfigured paths.
- [x] S1.28b Generation UI: the "describe your business" onboarding screen → calls S1.28a → opens the editor on the draft; unconfigured path lands on blank-site + a template picker instead; build/tsc/eslint clean, manual click-path journaled.
- [x] S1.29a Conversational editing, plumbing: per-page AI panel calling the ops endpoint; proposed ops as a human-readable change list; Approve applies / Discard; structural verify.
- [x] S1.29b Conversational editing, preview: before/after page preview for a proposed op-set; approval-card polish.
- [x] S1.30 AI copy tools per section (rewrite/tone/shorter/longer) as one generic op path + UI affordance on each text field; propose-then-approve.
- [x] S1.30b Owner feedback (2026-08-07, from first hands-on test): the New-website dialog must show the FULL resulting address live as the user types (e.g. typing `axon` renders `axon.alosites.com — available`), not just a bare ADDRESS field with rules text; same full-address display wherever a subdomain is entered. Second finding from the same test: the user naturally typed the FULL domain (`axon.alosites.com`) and the UI showed generic "could not be checked/saved" while the server had returned a perfectly clear 422 detail ("subdomain may only contain lowercase letters, digits, and hyphens") — surface the server's detail message in the dialog, and strip/normalise an accidentally-typed suffix instead of failing. Third finding: the empty Address field left Create disabled with no hint why — the address must PRE-FILL from the site name as a slug (type "Axon" → suggests `axon`), editable but never blank by default, and a disabled Create must say what's missing. Done when: the dialog previews the complete address with availability state, shows server validation messages verbatim, typing the full domain just works, and the address self-suggests from the name.
- [x] S1.30c Owner feedback (2026-08-07, hands-on test #2): after Create, the user landed on an empty Pages list and could not find "the editor" — creating a site must AUTO-CREATE the Home page and navigate STRAIGHT into the page editor (empty-state there invites "Add your first section"); the Pages list is for later, not the landing. Done when: Create → editor with Home open, one click from adding a hero.
- [x] S1.31a Wave review, language: fr/nl strings for the whole sites UI; CHANGELOG sweep.
- [x] S1.31b Wave review, reconciliation: docs/design/sites.md as-built; features [S1] reconciliation incl. the S1.30b/c dialog fixes; human-inbox summary (production compose+Caddy additions, AI key).
- [x] S1.32a FINAL arc, forms+publish: fixture-generate → edit sections → theme → publish → serve on subdomain Host → form submission → owner-inbox notification + submissions UI; transcript.
- [x] S1.32b FINAL arc, blog+domains+privacy: blog post from a real doc → custom-domain verify+serve → analytics counted with zero PII; transcript; then `LOOP COMPLETE`.

## S2 — multilingual publishing, CMS, collaboration, and growth

- [x] S2.00 Wave contract: split every requested S2/S+ capability into tenant-safe, independently shippable slices; name cross-product seams and no-AI/manual siblings; preserve the one-item-per-commit rule.
- [x] S2.01a Locale foundation: site default locale + enabled locales + locale validation in storage/routes; migration, wrong-tenant tests, and wire transcript.
- [x] S2.01b Localized pages: one stable page identity with per-locale title, slug, SEO, and section drafts; fallback rules and wrong-tenant tests.
- [x] S2.01c Localized publishing: immutable locale snapshots, alternate/canonical links, locale-aware sitemap/RSS, language switcher, and public Host goldens.
- [x] S2.01d Translation editor: visible language controls, missing-translation states, manual copy/create path, and publish readiness without requiring AI.
- [x] S2.01e Whole-site AI translation: deterministic proposal envelope across pages/posts, before/after review, approve-only writes, and fixture tests with no external calls.
- [x] S2.02a Collections model: tenant-owned site collection binding to an alo Base table plus validated field mapping; wrong-tenant and missing-table tests.
- [x] S2.02b Collection publishing: resolve Base rows into immutable publish snapshots with deterministic empty/error behavior; public render goldens.
- [x] S2.02c Collections UI: visible connect-table, field mapping, preview, disconnect, and empty-state controls; AI remains optional.
- [x] S2.03a Site-editor grants: per-site editor role that cannot read Mail, Drive, CRM, Billing, or unrelated sites; storage and wrong-tenant authorization matrix.
- [x] S2.03b Site-editor UI: invite/revoke collaborators and exercise edit/publish access end to end without exposing workspace administration.
- [x] S2.04a Version history API: list immutable publishes, compare metadata, and atomically republish a chosen version without mutating history; tenant tests and wire transcript.
- [x] S2.04b Version history UI: visible history surface, preview, and one-click rollback with undo/result feedback.
- [x] S2.05a Scheduled publishing model: tenant-scoped schedule/cancel/claim semantics with concurrency and wrong-tenant tests.
- [x] S2.05b Scheduled publishing service/UI: visible schedule control, local-time explanation, cancel/reschedule, worker execution, and wire transcript.
- [x] S2.06a Password-protected pages: strong password hashing, anonymous challenge/session gate, cache-safe responses, rate limiting, and security tests.
- [x] S2.06b Password UI: visible protect/remove controls, clear public-preview state, and accessible visitor unlock screen.
- [x] S2.07a Image presentation model: crop rectangle, focal point, and alt text on image-bearing sections with backwards-compatible validation.
- [x] S2.07b Responsive images: safe derivative pipeline and published `srcset`/`sizes`; byte/cache/XSS tests and public goldens.
- [x] S2.07c Image editor: crop/focal controls, manual alt text, and optional propose-then-approve AI alt text using fixtures only.
- [x] S2.08a Analytics dimensions: aggregate UTM campaign, coarse country, device class, entry/exit, read-time buckets, and outbound clicks while discarding raw IP/UA/query. (Shipped the five server-derivable dimensions; read-time and outbound clicks need a page beacon and are S2.08a2.)
- [x] S2.08a2 Page-beacon dimensions: a tiny published-page script and a public collect endpoint on alo-sites for read-time buckets and outbound-click domains — size caps, per-IP rate limits, no identity in the payload, byte-budget and privacy-schema tests, and the no-JS path still counting page views.
- [x] S2.08b Analytics UI: calm overview and drill-down surfaces for the new aggregates, with privacy explanation and useful empty states.
- [x] S2.09a Aggregate heatmap collection: bounded click coordinates and scroll-depth buckets with no visitor/session identity; schema privacy proof and abuse caps.
- [x] S2.09b Aggregate heatmap UI: page/viewport overlays, minimum-sample suppression, keyboard-accessible summaries, and empty states.
- [x] S2.10a Conversion events: aggregate form-view/start/submit attribution with stable site-owned source IDs and no individual journey storage.
- [x] S2.10b CRM/Billing attribution seam: tenant-safe handoff/read model joining site conversion totals to existing lead, deal, and invoice identities without editing the Billing/CRM-owned modules.
- [x] S2.10c Funnel UI: site → form → lead → deal → invoice aggregate funnel with clear unavailable/dependency states and no re-entry of known data.
- [x] S2.11a Template catalog: curated, versioned templates covering common site types with preview and deterministic instantiate tests.
- [x] S2.11b Template gallery UI: visual, keyboard-accessible manual creation path beside AI generation; one-click preview and create.
- [x] S2.12a Catalog sections: tenant-owned catalog/category/item model, Base import seam, publish snapshots, and public render goldens.
- [x] S2.12b Order forms: public order submission with validation, abuse controls, owner inbox/review/export flow, and no checkout dependency.
- [x] S2.12c1 Catalog editing API + item management UI: the `/sites/{id}/catalogs` routes S2.12a deferred (catalog, groups, items — typed prices, derived handles, tenant-hidden 404s) and the visible Catalog screen with its empty states. (Split from S2.12c 2026-08-13; one turn cannot hold the routes, the management screen, the section mapping and the order inbox at full depth.)
- [x] S2.12c2 Catalog section mapping + order inbox UI: the catalog section in the page editor (pick a catalog, optionally one group) and the per-site order inbox — lines, totals, status, CSV export, empty states — over the S2.12b routes.
- [x] S2.12c3 Catalog item photo: the Drive picker in the item dialog (the store, the route and the renderer already carry `imageBlobId`; only the UI is missing) + the published card's image, alt text and empty state. (Deferred from S2.12c1 and again from S2.12c2 — recorded here rather than in a journal entry.)
- [x] S2.13a Booking section model: availability source binding and booking-field schema that references Agenda through a Sites-owned seam.
- [x] S2.13b Public booking flow: the `booking` section variant (deferred from S2.13a, so that the renderer ships a form that actually works) + availability read, race-safe reservation, privacy/security tests, and real-process transcript. (Owner notification split out as S2.13b2 — the appointment already lands in the owner's calendar, so nothing is silently lost.)
- [x] S2.13b2 Booking notification: on a new appointment, INTERNAL delivery of a notification message to the owner's inbox (never outbound), claimed at-most-once like the form and order sweeps; tests prove the message lands in the right tenant's inbox and a foreign tenant sees nothing. (Split from S2.13b 2026-08-13, mirroring the S1.16b/S1.16c1 split.)
- [x] S2.13c Booking UI: visible Agenda connection, preview, booking management link, and dependency/error states.
- [x] S2.14a Sandboxed custom-code model: explicit HTML/CSS/JS capabilities, size caps, CSP contract, and validation that rejects unsafe host-page escape. (Shipped the published rendering too — the CSP/sandbox contract is meaningless until something emits it — so S2.14b is the editor half.)
- [x] S2.14b Sandboxed custom-code UI: the block in the web section mirror, its editor (three fields, capability switches, height), the draft preview, and the visible risk boundary; render goldens already exist from S2.14a.
- [x] S2.15a Domain commerce adapter: EU registrar interface, fixture provider, availability/pricing model, and no external calls in tests.
- [x] S2.15b Domain purchase state machine: quote → explicit approval → payment reference → register/configure/renew states with idempotency and tenant tests; Billing remains behind its owned public seam.
- [x] S2.15c1 Domain buy API: the registrar and nameservers as an injectable router boundary (typed `unconfigured` when unwired) + `/sites/domain-catalog`, `/sites/domain-search` and the `/sites/{id}/domain-purchases*` routes — create at the seller's own price, the registrant behind its own door, approve-the-price-you-saw, cancel; site-owner-only; wire transcript. (Split from S2.15c 2026-08-13: one turn cannot hold the routes, the registration worker and the screen at full depth — S2.15b already noted the item had to carry all three.)
- [x] S2.15c2 Payment handoff + registration worker: the checkout route that records Billing's opaque reference and the settle path, then the background sweep that claims paid purchases → `DomainRegistrar::register`/`renew` → complete/retry/fail, and the automatic Sites domain attachment (`configure`) that follows a successful registration.
- [x] S2.15c3 Domain purchase UI: search with the full name and its price as the user types, honest renewal pricing beside what is paid today, the registrant form, the approve-then-pay handoff, and progress/recovery states over the S2.15c1/c2 routes; the unconfigured deployment shows the connect-a-domain path instead.
- [x] S2.16a Wave review, security reconciliation: the authenticated `/sites/*` surface re-walked against the site-editor grant and the money/CRM doors, at BOTH mounts (`/sites/*` and the `/api/*` one production actually proxies); fix what it finds; as-built authorization matrix in the design doc. (Split from S2.16 2026-08-13 — one turn cannot hold the arcs, the accessibility sweep, the reconciliation and the docs at full depth.)
- [x] S2.16b Wave review, accessibility and responsiveness: every sites screen against `docs/design/ux-principles.md` and the keyboard/screen-reader basics — labels, focus order, dialog semantics, phone widths — fixing what it finds; language parity re-checked (the catalog ratchet is green, the new strings are the risk). (Carried the audit that can be made a test: dialog keyboard contract, landmarks, accessible names, the stylesheet's unwrappable layouts. The eyes-on-a-phone walk is S2.16b2.)
- [x] S2.16b2 Wave review, the walk a test cannot do: every sites screen opened in a real browser at 360px and driven by keyboard alone — reading order and focus order through the page editor's stack and its drag-reorder, the analytics and heatmap surfaces at phone width, contrast on the chips and status colours, and reduced-motion. Fix what it finds. (Split from S2.16b 2026-08-13: the loop has no browser, and a review that only reads source cannot claim to have looked.)
- [x] S2.16c Wave review, as-built docs and reconciliation: `docs/design/sites.md` reconciled with what S2 actually shipped, features.md [S2] table, CHANGELOG sweep, and the human-inbox summary (deployment keys, Caddy prefixes, the AI key, the open decisions this wave flagged).
- [x] S2.16d Wave review, final arcs: complete cross-service transcripts on the real local stack — generate → edit → publish → serve → convert (form/order/booking) → owner inbox → analytics — plus performance/byte budgets; then wave 3 begins.

## Wave 3 — commerce, the chatbot, and editing on the page

Settled in **ADR 0040** (what the bot may read, do and cost), **ADR 0041** (the
shop as a surface over one catalog) and **ADR 0042** (direct manipulation
without a canvas). Read the ADR before the item; the arguments are not repeated
here.

Two rules hold across the whole wave. **No item may edit a file the Billing,
CRM, Inventory or Agenda track owns** — where a seam is missing, the item that
adds it is listed here as owned by that module and must land first. And
**nothing keeps a second copy**: stock, price and availability are read and
reserved through their owner, never stored again (ADR 0041).

### Editing on the page (ADR 0042)

- [x] S3.01a Inline text editing: edit a typed section's text on the page rather than in a sidebar form, producing the same change shape an AI edit produces; undo/redo, and a test proving both paths yield an identical diff.
- [ ] S3.01b Reorder by dragging, with the page reflowing live and a keyboard-accessible equivalent; ordering is a change to typed sections, with diff goldens proving it.
- [ ] S3.01c Constrained resize: each section type declares its allowed ratios and shapes, and the editor offers only those; responsive goldens at phone, tablet and desktop, and a test that no gesture can produce free positioning.
- [ ] S3.01d Section palette: drag a new section in, previewed with the tenant's own content rather than lorem ipsum; keyboard path and goldens.
- [ ] S3.01e Editor arc review: one browser arc from blank page to published site using only direct manipulation, checked for accessibility, mobile and the reviewable-diff property.

### The chatbot that answers (ADR 0040)

- [ ] S3.02a Grounding model: the corpus is the **published** site plus a named Public knowledge collection; drafts and scheduled-but-unpublished versions are excluded by construction; tenant isolation tests, and a test that no unpublished string can ever be retrieved.
- [ ] S3.02b Answering with citations: retrieval over the corpus, every answer naming the page it came from, and a refusal rather than an answer when it cannot cite; fixture-only tests, no live model calls.
- [ ] S3.02c Cost and abuse: a per-site monthly **spend** ceiling that is defaulted rather than blank, per-visitor and per-IP rate limits below it, a graceful unavailable message that offers the contact form, and the tenant told when it is hit.
- [ ] S3.02d Source-adding UI: the screen that publishes a source to the bot says *anyone on the internet will be able to read this* above the button, every time; the ceiling is set in the same screen the bot is switched on.
- [ ] S3.02e Visitor chat UI: on-site widget, keyboard-accessible, mobile, citations as links, and an honest empty/unavailable state.
- [ ] S3.02f Appearance and voice model (ADR 0040 §5): the widget inherits the site's preset palette, logo and favicon; the tenant owns the welcome message, bot name and avatar, up to three suggested questions, a tone note, launcher position and icon, and the offline message. Colour is a choice among the site's own palette roles — no free-form colours, no custom CSS, no custom fonts. Validation tests proving no stored value can produce failing contrast, and a test that nothing in the tone note widens what §1 and §2 allow.
- [ ] S3.02g Appearance UI: a live preview of the real widget beside the fields, a written default welcome message rather than an empty box, suggested questions drafted from the published site's own headings and editable, and an accessibility check shown in the screen rather than discovered later.

### The chatbot that acts (ADR 0040)

- [ ] S3.03a **Agenda-owned**: a public seam exposing published availability for a site, without exposing the calendar behind it.
- [ ] S3.03b Booking from the conversation: create the meeting, send the confirmation, put it in the visitor's calendar, and include a cancellation link — with the reversible-only rule enforced in code rather than in the prompt.
- [ ] S3.03c **CRM-owned**: a public seam to create a contact and a lead from a site conversation.
- [ ] S3.03d Lead capture through that seam, storing aggregate attribution only and no individual visitor journey.
- [ ] S3.03e What the bot did: a tenant-facing transcript showing each action, the fact it used and the page that fact came from.

### Commerce wave one — tickets and dated products (ADR 0041)

- [ ] S3.04a **Billing-owned**: a read seam exposing published catalog items and their prices to a site, with no write path and no second copy.
- [ ] S3.04b Hold-with-expiry: capacity is reserved *before* payment and released if the buyer does not finish; concurrency tests proving two simultaneous buyers cannot oversell the last seat. This is the first commit of the wave, not hardening.
- [ ] S3.04c Hosted payment handoff: a provider adapter with a fixture provider and no external calls in tests, an order → payment-reference → paid state machine, idempotent webhook handling, and a test that no card data can reach alo. Mollie or Adyen ahead of Stripe.
- [ ] S3.04d Fulfilment: the ticket by email and in the buyer's calendar, the contact in CRM and the invoice in Billing, each through its owned seam.
- [ ] S3.04e Place-of-supply VAT for event tickets as a rules table with tests — reviewed by a tax professional before this item is taken, because wrong here is wrong in Finance years later.
- [ ] S3.04f Shop sections and checkout on the published site: typed sections like every other, public render goldens, mobile.
- [ ] S3.04g The arc: a visitor asks the bot about an event, is offered tickets at a price read from the catalog, pays on the provider's page, and the ticket, the contact and the invoice all exist. No price the model invented.

### Commerce wave two — stock items (ADR 0041)

- [ ] S3.05a Simple stock items: one price, one tax, one shipping rate, with stock read and reserved through Inventory's seam and never copied.
- [ ] S3.05b Propose the configuration: from a sentence about the business, draft the catalog, the VAT treatment per item and the shipping, and present it for approval with every guess flagged — the screen where Odoo loses the customers who cannot afford a consultant. Fixtures only.
- [ ] S3.05c Shop UI for stock items, sharing the checkout built in wave one.

- [ ] S3.06 Wave review: browser arcs for all three strands, accessibility, responsiveness, language parity in English, Dutch and French, privacy and security reconciliation, changelog and as-built docs.
