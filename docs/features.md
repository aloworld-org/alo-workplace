# alo — features.md

Feature inventory per module. Three tiers, mapped to the roadmap:
**[L]** = launch (must exist to cancel M365) · **[2]** = fast-follow, first year after launch · **[3]** = later, revenue-funded.
**★** marks differentiators — features Microsoft either lacks, charges extra for, or does badly.

Rule of the file: nothing gets built that isn't listed here, and nothing gets listed without a tier. Additions go through the scope gate (product doc, Non-goals).

---

## AI — an agent for every product (ADR 0034)

Cross-cutting principle: **every product has its own dedicated agent**, scoped to
that product's data + actions, all under **propose-then-approve** (never silent),
**access-scoped** (only what the user can already see/do), and **EU-only** models.
Above them sits the **"Ask alo" orchestrator** (ADR 0029) that routes across
products. One shared framework (the `alo-ai` crate + a tool registry + the
propose/approve UI); each agent is a thin, product-scoped tool set + prompt.

### How an agent behaves — the three rules that shape every one of them

- [L] ★ **A question is answered, not proposed.** Propose-then-approve governs
  *changing* things. Asking "is the X100 in stock?" or "are we still in contact
  with ABC?" returns the answer in the room, immediately, with the record it
  came from. Only a write — send, book, invoice, publish — waits for a tap.
  Making a read wait for approval is the difference between a colleague and a
  form.
- [L] ★ **Each agent has its own eyes.** An agent's retrieval is scoped to its
  own product: the Mail agent searches correspondence, the Inventory agent
  queries stock. A shared full-text search over the whole workspace answers
  every question equally badly, and makes "per-product agent" a name rather
  than a capability.
- [L] ★ **The asker's door, always.** An agent reads and writes only what the
  person who asked already could — in a DM, in a private channel, and in a
  cross-org channel. An agent is an identity, never an authority (ADR 0034).
- [2] ★ **Complete over its app, and nothing more (ADR 0057).** An agent can do
  everything its app can do: its tools are derived from the module's own
  capability manifest, every route reachable or explicitly excluded, tested.
  "I could not find it" for a record the app holds is a bug, not an answer.
- [2] ★ **Only when asked — then anything.** An agent speaks and acts when a
  person asks: by mention, in a DM, or by a **standing instruction** ("every
  Monday, post the open quotes here"), which is asking once, in advance, and
  cancelling any time. No unsolicited offers.
- [2] ★ **Agents hand work to each other, visibly.** Billing may ask Sales for
  the deal behind a quote — inside one run, as the asker, to a depth of two,
  with the room seeing who asked whom. Never a conversation between agents.
- [2] ★ **A channel is what an agent may remember.** What was shared in a
  channel, its agents may remember — whoever wrote it — and use in that
  channel only. Per-channel switch, a *What I remember* page, and memories
  that die with the message or the channel.

### Where you meet an agent

- [L] ★ **In its own product** — the in-module assistant.
- [L] ★ **In a channel** — `@mention` it where the work is discussed, so the
  answer lands in front of everyone who needed it.
- [2] ★ **One-to-one** — a DM with an agent, like a DM with a colleague. The
  place to ask the thing you would not put in the team channel.
- [3] ★ **In a meeting** — see Meet, below.

### The agents

- [L] ★ **Mail agent** — triage, summarize a thread, draft / smart-reply, extract tasks, "why flagged"; **answer from correspondence** ("are we in contact with ABC?", "who last replied to them?") *(largely built)*
- [L] ★ **Tasks agent** — propose action items, "what's on my plate", prioritise, chase *(built, ADR 0023)*
- [L] ★ **Docs agent** — write / edit / clean-paste / inline-diff, agent mode *(built in the editor; contributes no chat tools yet)*
- [2] ★ **"Ask alo" — the top-level agent** — not just search: a workspace-wide agent you ask in plain language to **answer AND act** across products ("summarise the Acme thread and draft a reply", "block two hours tomorrow and email the team"), running multi-step tasks by orchestrating the product agents below — cited, propose-then-approve, access-scoped. The universal command bar for the whole workspace *(cross-product cited search + doc AI built today; acting/orchestration is the growth)*
- [2] ★ **Agenda (Calendar) agent** — find times, schedule, summarize the day/week, prep a meeting, propose events from mail *(reads built)*
- [2] ★ **Chat agent** — catch up on a room, find what was said and where it was decided *(reads built)*
- [2] ★ **Drive agent** — find & organise files, summarize a document, extract from attachments *(find built)*
- [2] ★ **Sheet agent** — formulas from intent, explain a formula, clean a column, answer from the data with the cells cited *(built)*. **Chart-from-intent is not here**, and deliberately: an agent cannot propose a chart into a product that has no charts (see alo Sheets below). It returns when charts do.
- [2] ★ **Website (Sites) agent** — answer from the live site ("what do we promise on pricing?"), draft and edit a page, translate the whole site, review SEO; publishing is proposed, never silent (ADR 0036)
- [2] ★ **Insights agent** — answer from the numbers, explain a change, build a report
- [L] ★ **Billing agent** — invoice, convert a quote, chase *(built, wave B1)*
- [L] ★ **CRM agent** — deals from a thread, move a stage, follow up *(built, wave B2)*
- [L] ★ **Projects agent** — log time, status, timesheet from calendar *(built, wave B3)*
- [L] ★ **Finance agent** — categorise, VAT summary, flag anomalies *(built, wave B4)*
- [L] ★ **Inventory agent** — reorder proposals, and **stock answers in the room** ("is the X100 in stock?") *(built, wave B5)*
- [L] ★ **HR agent** — who is off, draft a letter from the company's own template *(built, wave B6)*
- [3] ★ **Meet agent** — after a meeting, minutes and actions into the meeting's thread, where they can become tasks and events like any other agent output. A **live** in-call participant that hears the room and answers mid-call is a separate decision (own ADR) — a media path, not a tool set.
- [3] Per-agent skills users can create and share; the browseable agent directory

---

## Orders and delivery - the goods half (ADR to write)

alo bills well and ships nothing. A quote becomes an invoice in one step, so
there is no record of **ordered but not yet delivered** - which for anyone
selling physical goods is most of the business at any moment. This is the
clearest gap against Odoo and SAP and the honest reason not to claim parity for
manufacturing yet.

- [2] * **Sales order** - what a customer has ordered, and how much of it has been reserved, delivered and invoiced. Created when an accepted quote contains stocked lines; a services quote still becomes an invoice directly, because there is nothing to reserve or deliver.
- [2] * **Reservation** - confirming an order commits stock, shown beside on-hand and on-order. Selling the same fan twice is the failure this prevents.
- [2] * **Delivery notes** - goods leave against an order, stock moves, partial deliveries are ordinary, and the note is a document a driver can carry.
- [2] * **Invoice what shipped, not what was promised** - a part-delivered order bills correctly and the remainder stays visible.
- [2] * **The order book** - ordered, reserved, delivered, invoiced, outstanding. The screen a manufacturer opens first, and the one alo cannot draw today.
- [3] * **Orders agent** - where is this order, what is short, what can ship today.
- [3] Bill of materials, works orders and capacity. Taking an order you cannot build moves the problem rather than solving it, but an order book with no reservation is the more urgent absence.

---

## alo Campaigns — bulk email that cannot poison the mailbox (ADR 0044)

Mailchimp's weakness is that it is a separate company holding a copy of your
customers, and charges rent on it. Ours is that our sending IPs have no history.
The moves and the honest limits are in `docs/design/campaigns-gaps.md`; the
differentiator is **the segment and the attribution**, and everything else —
the editor included — is built to good enough rather than to better than theirs.

**The audience is assembled from tenant-wide records only** — billing
customers, CRM deal contacts, and site form submissions. Never the `contacts`
table, which is a *per-user* address book: a company campaign drawn from it
would mail people out of an individual's private contacts, which is a privacy
boundary rather than a preference.

- [2] ★ **The audience is a query, not a list** — "bought in the last 18 months but not the last 90 days, in Belgium" is a join across CRM, Billing and site events. Nothing to sync, because there is no list; no duplicate people, because there are no audiences to be in twice.
- [2] ★ **Consent is a record** — when, from which form, from which address, shown on the person. A campaign cannot go to somebody without one, and an import must state where the addresses came from.
- [2] ★ **Suppression is absolute and enforced in the store** — an unsubscribe, hard bounce or complaint removes somebody from every future send, and no segment, import or re-upload can bring them back. Enforced at send time in SQL rather than applied by the sender, or it is not absolute.
- [2] ★ **A separate sending identity** — own subdomain, own DKIM selector, own IP or pool, own queue, so a marketing reputation can never reach the domain carrying invoices and password resets. Architecture, not a paid tier. **Blocked on a second IP: this is a purchase, not a decision.**
- [2] ★ **Warm-up is visible in the send flow** — a new subdomain and IP mailing thousands on day one is filtered however correct the DKIM is. The cap and the reason are shown where the send happens, never in a footnote.
- [2] ★ **Unsubscribing is two doors, not one** — the `List-Unsubscribe` header the mail client turns into its own button (RFC 8058, one POST, no login), **and** a visible link in the footer, because only some clients render the button and everyone else scrolls looking for the word. The link is a per-recipient unguessable token, never an address in a URL, so nobody can unsubscribe somebody else and no scraper can confirm an address is live by watching the page. The landing page offers **fewer rather than only none** — this kind of mail, or all of it — because a recipient who only wanted the newsletter stopped and is offered a single all-or-nothing button reaches for the spam button instead, and that is the signal that ends a sending reputation. Honoured immediately and tenant-wide, including for batches of the same send still going out. **Transactional mail never carries one:** an invoice, a password reset and a meeting invitation are not marketing, and the separate sending identity is what makes that structural rather than a flag.
- [2] ★ **One-click unsubscribe every time** (`List-Unsubscribe`, RFC 8058) — alo already implements the reader half in `unsubscribe.rs`; this is the same standard from the other end. A recipient who cannot find it presses "spam" instead, and that is the signal that ends a reputation.
- [2] ★ **Bounces and complaints act on themselves** — hard bounce suppresses immediately, soft bounce retries then suppresses, a complaint suppresses and counts against the tenant's rate.
- [2] ★ **The number reported is money, not opens** — delivered → clicked → visited → converted → **invoiced**, in euros, because the invoice is in the same database. No tracking pixel by default: a per-campaign choice, off unless chosen, and disclosed.
- [2] ★ **Building the mail** — the Docs block model compiled to email-safe HTML (table layout, inline CSS, because Outlook renders through Word), a plain-text alternative from the same blocks, personalisation with a visible fallback for every merge field, and a mail that still reads with images blocked. Preview and a seed test send are required before an audience can be chosen; the screen states honestly that a preview is our renderer's opinion, not proof of how Outlook 2016 will draw it.
- [2] ★ **Sending** — a durable per-recipient job so a crash resumes rather than restarts and nobody is mailed twice; paced in batches under the warm-up cap; and **pause/stop mid-send**, because the first thing anyone notices after pressing send is the typo.
- [2] ★ **What actually happened, ranked by how much it can be trusted** — delivery, bounces and complaints are **facts** from our own SMTP; clicks are **reliable**, being a redirect somebody followed; opens are **weak** and opt-in per campaign, because Apple pre-fetches images so an unknown share are machines; **read duration is weaker still and undecided** — it needs a pixel held open, which most clients block, and whether a number that unreliable belongs in a product sold on not tracking people is its own ADR. The report screen says in the interface which of its numbers are facts and which are estimates.
- [3] Automations: a send triggered by a CRM stage, an unpaid invoice, or a form submission — same consent, suppression and identity rules, no new sending path.
- [3] A/B on subject and content, decided on **clicks and revenue** rather than opens: the metric a test optimises had better mean something.
- [2] **The composer reuses the Docs block model**, compiled to email-safe HTML — one content model and a second renderer, the same shape as Sites compiling sections to static HTML. Not a second editor.
- [3] ★ **The Campaigns agent turns a sentence into a segment you can see and edit** — not writing the copy, which everyone ships. The query is the artefact.
- [3] Automations: a send triggered by a CRM stage, an invoice going unpaid, or a form submission.

---

## Mail

- [L] Mailboxes, folders/labels, aliases, plus-addressing (`user+tag@`)
- [L] Nested subfolder hierarchy with drag-and-drop, unread counts per folder (Outlook muscle memory — non-negotiable)
- [L] Flags / follow-up marking with optional due date, and a "flagged" smart view
- [L] Categories: color tags, multiple per message, filterable — shared org-wide category sets optional
- [L] Conversation view (threaded) with per-user toggle back to flat list — both camps exist and both must be happy
- [L] Archive as a first-class one-keystroke action
- [2] Smart folders — saved searches that behave as folders ("unread from customers", "flagged this week")
- [2] ★ Internal recall that actually works — within a tenant we control the store, so recalling an unread internal message genuinely deletes it, unlike Exchange's famously fake recall
- [2] Quick steps: one-click multi-action macros (mark read + move + forward), the power-user retention feature
- [L] Shared mailboxes with delegation (send-as, send-on-behalf)
- [L] Native distribution lists (the Mailman replacement — no bolt-on)
- [L] Server-side rules (Sieve) with a visual rule builder
- [L] Signatures (per-identity, org-enforced footer option)
- [L] Out-of-office with scheduling
- [L] Undo send (30–60s delay window)
- [L] Send later / scheduled send
- [L] Snooze ("return this thread Monday 09:00")
- [L] Full-text search that is actually fast (store-level index)
- [L] Spam/phishing filtering with a visible "why was this flagged" banner
- [L] One-click unsubscribe surfacing (RFC 8058)
- [L] Large files as expiring share links (alo Transfer) — a file too big to
  attach uploads once and rides the message as a private, expiring download link
  instead of an inline attachment, sidestepping recipient attachment-size limits.
  This is the Drive share-link capability (password/expiry/download-off) surfaced
  in compose; v1 ships an unguessable-link + expiry, with password/download-off
  tracked to the Drive work.
- [2] ★ Follow-up nudges — "no reply after 3 days" resurfacing, per-thread
- [2] ★ Shared-inbox collaboration: assign a thread to a colleague, internal comments on a thread, collision alert ("Kevin is already replying") — Front-style teamwork on info@/sales@ boxes, which Outlook simply cannot do
- [2] Templates / snippets with variables
- [2] Read/delivery status for internal mail (never tracking pixels — privacy is the brand)
- [3] Email client keyboard-shortcut parity (Gmail-style j/k culture)
- [3] S/MIME and OpenPGP for the customers who ask

## Outlook toolbar audit — keep / do-better / drop

Outlook's toolbar is a thin core of email actions wrapped in a ring of Microsoft-ecosystem hooks and third-party add-ins. Almost everything we **drop** below is one of those hooks or add-ins — not email — so alo's mail toolbar ends up *cleaner* than Outlook's (the daily actions minus the clutter) **plus** the AI actions Outlook lacks (summarize, draft, why-flagged).

**Keep** — core mail actions, table stakes:

| Outlook | alo |
|---|---|
| Reply · Reply All · Forward | [L] the spine of email — identical |
| Delete · Archive | [L] daily one-keystroke actions |
| New Email / New Items | [L] compose |
| Move | [L] into folders |
| Flag · Categories / Tags | [L] flags + color categories, multiple per message, filterable |
| Report junk/phishing | [L] ★ feeds the visible "why was this flagged" banner |
| Quick Steps | [2] one-click multi-action macros (the power-user retention feature) |
| Rules | [L] ★ server-side Sieve — stronger than Outlook's client-side rules, runs even when you're offline |
| Address Book · Search People | [L] from CardDAV contacts |
| Filter Email | [L] sort/filter the list |

**Do better** — keep the capability; alo's version is superior:

| Outlook | alo |
|---|---|
| New Meeting · Scheduling Poll | [L] alo Meet + [2] ★ native meeting polls (kills the Doodle bolt-on) |
| Translate | [2] ★★ AI-native and EU-hosted — the Belgium differentiator, not an add-in |
| Read Aloud | [3] accessibility, later tier |
| Recall | [2] ★ actually works inside a tenant — we own the store, unlike Exchange's famously fake recall |

**Drop** — Microsoft ecosystem tentacles and third-party add-ins; dropping them *removes lock-in*, which is the pitch, not a lost feature:

| Outlook | Why it's gone |
|---|---|
| Share to Teams | replaced by "share to alo Chat" |
| Viva Insights | a Microsoft analytics add-in, not our product |
| TeamViewer | a third-party add-in, never a mail feature |
| Browse Groups | an M365 Groups construct — our distribution lists + shared mailboxes cover the real need |
| All Apps | the Microsoft app-grid launcher, irrelevant to a focused workspace |

This table doubles as the answer to a prospect asking "where's feature X?" — every row is kept, done better, or deliberately dropped to cut a lock-in tentacle.

## Agenda

- [L] Personal + shared calendars, free/busy, invitations (iTIP/iMIP)
- [L] Recurring events with exceptions (the interop minefield — done right)
- [L] Room and resource booking
- [L] Working hours, time-zone sanity for cross-border teams
- [2] ★ Booking pages — public "book a slot with me" links (kills the separate Calendly subscription; M365 hides this in Bookings and does it badly)
- [2] ★ Meeting polls — "which of these three slots works?" (kills the Doodle subscription)
- [2] Travel-time blocking between physical meetings
- [3] Team scheduling: round-robin and collective availability for sales/support teams

## Tasks

The third leg of the mail + calendar + tasks wedge — one record, board and
list views over the same data, personal and team. ADRs 0021–0023.

- [L] Tasks: title, description, assignee, due date, priority, subtasks, comments, activity history, attachments — one record, tenant-scoped
- [L] ★ Board (kanban) and list are two views of the SAME task — instant, lossless switch; drag to move between columns / reorder (ADR 0022)
- [L] Personal tasks (private) and team tasks (shared projects with assignees) — one data model, different scoping (ADR 0021)
- [L] ★ Task detail as a slide-in side panel (never navigates away): description, subtasks, comments, activity, source link
- [L] ★ Source link: a task remembers the email or calendar event it was created from, and can jump back to it (email→task, meeting→task)
- [L] ★ AI proposes action items from a meeting/email; the user accepts the real ones — propose-then-approve, never silent creation (ADR 0023)
- [L] Task ↔ calendar: a task with a due date surfaces on the calendar alongside events
- [L] ★ "What's on my plate today" — an aggregate the AI assembles from tasks (+ calendar + mail as they connect)
- [2] Per-project membership + roles for team projects (v1 scopes team projects tenant-wide)
- [2] Recurring tasks, task dependencies, custom board columns
- [3] Workload view, timeline/Gantt, task templates

## Chat

- [L] Channels (public/private), DMs, real threads, reactions, mentions
- [L] **Rich, modern chat UI** — the visual bar is Slack/Teams-grade, not a bolt-on. Design reference: **Sila (silahq.com)**: a left sidebar (DMs, channels, agents, shared, search), a clean message feed with human avatars + distinct agent icons, message bubbles with sender + timestamp, inline media/link previews, hover actions (react/reply/more), typing + presence indicators, unread badges.
- [L] File sharing into Drive (one storage, not a parallel one — SharePoint's original sin)
- [L] Powerful search across full history — ★ no paywalled memory, ever (Slack's most-hated limit)
- [L] Guest access for externals, per-channel
- [2] ★ **Agent-native chat** (the AI-native differentiator, à la Sila) — the per-product agents (ADR 0034) are first-class participants in channels/DMs with their own avatars/indicators. @mention an agent and it **talks back in-thread AND takes actions in its product**: the Mail agent drafts/sends, the Sheet agent updates a range, the Agenda agent books a slot, the Docs agent edits — every *action* still **proposed then approved** (never silent), **access-scoped** to the asking user (even in shared/cross-org channels), cited/auditable. Chat becomes the shared human+agent command surface. Browseable agent directory with usage. EU-only inference.
- [2] ★ AI/natural-language search **and** notifications — "notify me when the Acme deal is mentioned", "where did we decide the price?" — over full history, no paywalled memory.
- [2] Expiring messages and time-limited groupchats (ephemeral conversations that clean themselves up), à la Sila.
- [2] Reminders ("remind me about this message tomorrow"), saved items
- [2] Instant huddle — one-click voice in a channel, no calendar event; ★ with call transcription + auto notes posted back to the thread
- [2] ★ Cross-org channels between two alo tenants (agencies ↔ clients) — incl. shared human **and agent** coordination across tenants
- [3] Message workflows/automations (approval emoji triggers, simple bots)

## Meet

- [L] Scheduled + instant meetings, calendar-native links, screen share, lobby
- [L] Recording to Drive (with consent indicators)
- [2] ★ AI minutes: transcript, summary, decisions, and action items posted to the meeting's chat thread — included, not a €30/user add-on
- [2] Live captions
- [3] ★ Live translated captions — a Flemish/Walloon/German meeting where everyone reads their own language; the most European feature possible
- [3] Webinar mode (one-to-many, registration)
- [2] ★ Remote control — take a colleague's screen to fix it, not just watch it (ADR 0039). Desktop app only, granted per session by the person at the machine, suspended the moment they touch their own mouse, visibly indicated throughout, and fully audited. **No unattended access, ever** — that is MDM, and MDM goes to partners.

## Drive & Docs

- [L] Files/folders, per-user and per-team spaces, permissions, trash/restore
- [L] Desktop sync client (the OneDrive replacement)
- [L] Share links with password, expiry, and download-off option
- [L] **Native editors — alo's own UI on embedded open engines** (ADR 0033, replacing the earlier Collabora-embedded approach). Real Office files import best-effort into the native types; the original file is kept and stays downloadable; pixel-faithful round-trip to desktop Office is no longer promised. Per editor:

  **alo Docs (Word-like)** — alo's own block editor on **BlockNote** (MPL-2.0)
  - [L] Rich text: styles, headings, lists, tables, images, code blocks, math equations (KaTeX)
  - [L] Propose-then-approve document AI (ADR 0029)
  - [L] Open a real `.docx` → best-effort import into an alo Doc
  - [2] Real-time co-editing; comments; export to PDF

  **alo Sheets (Excel-like)** — alo's own ribbon UI on **Univer** (Apache-2.0)
  - [L] Grid + formula engine, multi-sheet, cell formatting, number formats, alignment, merge, freeze panes
  - [L] Open a real `.xlsx` → best-effort import; **export any sheet back to `.xlsx`**
  - [2] Pivot tables, sorting/filtering, data validation (Univer plugins, wired incrementally)
  - [2] ★ **Charts, owned by alo rather than by the grid engine (ADR 0051)** —
    bar, line and pie over a range you pick, drawn by the chart engine alo
    already ships (Apache ECharts, Apache-2.0, already bundled and already
    themed for Insights). A chart is **alo's own record** — kind, title, and
    the ranges it reads — stored beside the Univer snapshot rather than inside
    it, so it survives an engine change and cannot be held hostage by a
    plugin. It stores **ranges, not values**, so a chart can never disagree
    with the cells it came from. Deliberately *not* `@univerjs-pro/sheets-chart`:
    a commercial licence in a sovereignty product, for a renderer we already
    own outright.
  - Known honest limit on charts: **a chart does not survive a round-trip to
    Excel.** Import reads `.xlsx` into cells and drops chart parts by
    construction; export writes none. Worth having anyway, and worth saying
    plainly rather than discovering.
  - Known honest limits: **VBA macros do not run**; complex `.xlsx` styling/charts may not survive import (see product doc §6)

  **alo Slides (PowerPoint-like)** — native canvas built in-house (no open engine covers it; ADR 0033)
  - [2] Slides, text boxes, shapes, images; best-effort `.pptx` import
  - [2] Present directly in the browser; present into a Meet call

- [L] Format fidelity guarantee: files round-trip to desktop Office without layout mangling — tested in CI with a corpus of real customer documents (fidelity is the whole ballgame; a mangled offer letter loses the customer)
- [L] Editors in the desktop app: Docs/Sheets/Slides work identically in the installed (Tauri) app — same frontend, no extra build
- [L] Offline story: synced files open in local LibreOffice/Office while disconnected, changes sync back on reconnect (same model as OneDrive + desktop Office)
- [3] True offline in-app editing (bundled editor engine) — only if customer demand proves it
- [L] Version history with restore
- [2] Document templates (org-branded letter, offer, invoice skeletons)
- [2] Full-text + ★ semantic search inside file contents ("the pdf about the Antwerp lease")
- [3] E-signature workflow (eIDAS-aware — European advantage)
- [3] Retention policies and legal hold per space

## alo Docs — the AI-native document editor

Not a cheaper European Word. alo Docs differentiates on being **AI-native,
whole-suite, and sovereign**, attacking documented, widespread Word/Docs
frustrations that Microsoft/Google structurally cannot fix without dismantling
their own architecture. The editor is **alo's own block editor on the embedded
BlockNote framework** (MPL-2.0); alo owns the UI, the AI layer, and the four
inventions below (ADR 0033, superseding the Collabora shell of ADR 0010). Base
.docx import lives under
**Drive & Docs** above; this is the differentiator layer. UX source of truth:
Figma page "10 · Docs".

The four inventions:

- [2] ★ **Clean paste** — on paste from external sources (Word, the web), strip
  foreign formatting **by default** and match the destination document's
  styles; show a dismissible toast ("Pasted from <source> — formatting
  cleaned") with a "Keep original" escape hatch. Targets the #1 documented
  Word/Docs pain: foreign styles corrupting a document on paste.
- [2] ★ **Ask-AI-from-your-docs** — an in-editor AI panel that answers from the
  user's *actual* documents and workspace, not just the open doc ("What did we
  offer Proceq last quarter?" → pulls the real file from Drive). Every answer
  carries a **source citation** (which file it came from); cross-suite (Mail,
  Drive, Calendar); with suggested actions ("insert into the doc", "summarize
  this section"). It is **agentic, not just Q&A** — see below.
- [3] ★ **Semantic-conflict flag** — beyond CRDT text-merge: when two
  collaborators' edits no longer reconcile in *meaning* (one changes a unit
  price, another the total, so they no longer add up), the AI surfaces an
  inline flag ("alo noticed a possible conflict — these no longer add up")
  with keep-A / keep-B / let-me-fix. Directly targets the documented
  silent-corruption of Word/Docs real-time co-authoring, which merges
  conflicting edits into nonsense with no warning.
- [3] ★ **Draft-from-workspace-context** — on a new/empty doc, offer to draft it
  from real workspace context: the AI lists the sources it will use (the
  relevant email thread, a meeting recording + its AI notes, related
  spreadsheets) and generates a first draft from them. The cross-suite killer
  move — only possible because alo owns Mail + Meet + Drive + Docs in one
  sovereign place.

**Ask-AI is agentic** — it acts on the document, always **proposing, never
silently changing**:

- [2] ★ Inline command: select text → a command bar (Rewrite / Shorten / Fix
  grammar / custom instruction).
- [2] ★ Proposed edit: AI changes are shown as an inline **diff** (old struck
  through, new highlighted) with **Accept / Reject** — nothing applies without
  approval.
- [3] ★ Agent mode: multi-step tasks ("add a delivery-terms section and tighten
  the intro") execute as a visible **plan** with per-step status
  (done/doing/pending), a live progress note, workspace-context grounding, and
  a **Stop** control; the doc shows where the AI is actively writing.
- **Core principle:** the AI proposes and diffs; the user accepts. It never
  overwrites the document without explicit approval — the trust model that fits
  a sovereignty product.

**Technical authoring** — specs with math, equations, and code, for engineers,
finance, and technical writers. A alo-owned shell capability (ADR 0015)
rendered **browser-local** (no draft equation or source line leaves the client);
KaTeX + Prism (both MIT); the numbering/reference layer is alo's own. UX
source of truth: the Figma technical-authoring screens.

- [2] ★ **Equations** — an equation editor with LaTeX input and a **live
  rendered preview** (KaTeX), a LaTeX/Visual toggle, and a common-symbols quick
  bar; supports both **inline math** (within a sentence) and **numbered display
  equations**.
- [2] ★ **Code blocks** — syntax-highlighted code (Prism) with a **searchable
  language picker** (explicit, never auto-detected), a copy button, and line
  numbers.
- [2] ★ **Cross-references + auto-numbering** — equations, tables, figures, and
  sections get **auto-numbers**, and reference chips ("Eq. 3", "Table 1",
  "Section 2.3") **stay correct automatically** when items are reordered or
  inserted (references point at an item's identity, resolved to its current
  number). Includes the insert-cross-reference picker (tabs for Equations /
  Sections / Tables / Figures).

Cross-cutting Docs principles:

- [L] No hidden formatting: visible structure, an always-available "clean
  formatting", and block-safe editing that can't be accidentally broken —
  while preserving a **print-perfect "paper" view** for formal documents
  (offers, contracts).
- [L] Version confidence: persistent plain-language save/version state ("Saved ·
  v14 · Kevin edited 2 min ago") with a human-readable timeline.
- [L] ★ Web-first, single-version — no desktop-vs-browser split (the one thing
  everyone praises Google Docs for).

## alo Sheets — the AI-native, auditable spreadsheet

Not a cheaper European Excel. Differentiates on **AI-native + auditable +
whole-suite + sovereign**. Finance teams abandon spreadsheets over two things
the research documents clearly: **error-blindness** (a CFO study found 41%
struggle to identify and correct errors) and **lack of auditability / data
lineage**. alo attacks both directly. The editor is **alo's own ribbon UI on
the embedded Univer engine** (Apache-2.0), the same pattern as Docs (ADR 0033,
superseding ADR 0010); alo owns the UI, the AI layer, and the inventions below.
Base .xlsx import + `.xlsx` export lives under **Drive & Docs** above. UX source of truth:
Figma page "11 · Sheets".

The four inventions:

- [2] ★ **Explain-and-fix errors** — replace cryptic #REF!/#VALUE!/#NAME? with a
  plain-language card: *why* it broke ("row 14, referenced by D5, was deleted")
  plus one-click fixes (re-point the range / restore the row). AI proposes, user
  accepts.
- [2] ★ **Natural-language formulas** — type plain English ("average revenue per
  region, excluding France"); alo generates the formula, **shows the actual
  formula**, and explains it in one line. Never a black box — transparent and
  auditable (treat NL as a draft, keep the transparent formula).
- [2] ★ **Formula paste-guard** — when a raw value is about to overwrite a
  formula cell, warn ("E5 holds =SUM(D2:D13) — paste as value anyway, or keep
  the formula?"). Defends against the documented "pasted value silently ruined
  my model" failure that Excel has no guard for.
- [3] ★ **Ask-your-data** — an NL question panel ("which region is trending
  down?") → an answer with the **source cells cited**, the cells highlighted,
  and a chart. Cross-suite (can pull from Drive/Mail). Every answer traceable
  to its cells.

Cross-cutting Sheets principles:

- [2] ★ Auditability first: cell lineage ("where did this number come from + who
  changed it"); answers and AI edits always cite their source cells.
- [L] AI proposes, user accepts — never silently changes a value or a formula
  (the trust model shared with Docs; critical for audit-ready finance models).
- [L] Cross-platform migration safety: handle Excel-dialect formulas
  (semicolon/comma) on import so formulas don't break moving in.
- [3] Optional agent mode for multi-step data tasks ("build a Q3 forecast from
  the actuals tab") with a visible plan + approval, mirroring the Docs agent.

## alo AI (the differentiator layer — every item ★)

- [L] Semantic search across mail, chat, files in one query bar
- [L] Thread summarization ("catch me up on this 40-mail thread")
- [L] Drafted replies in the user's own tone, user-invoked
- [L] Attachment understanding — incoming .docx/.xlsx read, summarized, figures extractable
- [L] "Where did X go?" migration assistant (change management as a feature)
- [2] Daily digest: "what did I miss" across mail, chat, meetings — the demo that sells
- [2] Inbox triage: priority surfacing, low-value mail folded away, per-user trainable
- [2] ★★ Auto-translation of mail and chat — read and reply across NL/FR/DE/EN transparently; for Belgian and cross-border SMEs this alone justifies switching
- [2] MCP server — customers' AI agents read/search/send under per-agent permissions; the "AI-era workspace" claim made concrete
- [3] Cross-suite actions: "summarize this thread and update the offer sheet" — the Copilot-killer, EU-hosted
- [3] Org memory: "what did we decide about the pricing?" answered from three months of channels and mails

## Admin & platform

- [L] Tenant admin: users, groups, domains, quotas, license seats
- [L] ★ Deliverability autopilot: DNS wizard, DKIM rotation, DMARC monitoring, blacklist alerts — self-hosted mail's killer solved in-product
  - [L] Domain verify + record guidance, registrar-universal (read-only DNS checks; works at any registrar) — done, ADR 0012
  - [2] Per-tenant DKIM keys with selector-rollover rotation — ADR 0014
  - [3] ★ "Just works" onboarding: change your nameservers, we run authoritative DNS and manage the whole zone (MX/SPF/DKIM/DMARC/mta-sts) automatically — the universal, sovereign path (not per-registrar APIs). DKIM-CNAME makes rotation no-touch on top of it. Engine + direction: ADR 0013
- [L] Audit log, GDPR subject-access export, tenant data export (no lock-in — exit is a feature)
- [L] SSO: alo as OIDC/SAML IdP + 2FA enforcement
- [L] Backup status visibility ("last verified restore: date")
- [2] ★ White-label/reseller mode: MSP branding, multi-tenant management console — the channel play productized
- [2] Per-tenant feature flags and AI on/off switches
- [parked] Personal email (self-service): individuals self-register an address (e.g. `johnsmith@alomails.com`) on a platform-operated domain — one tenant per person, verification-gated signup, consumer sending reputation isolated from B2B. **Parked indefinitely — off the active path (ADR 0020): the focus is the business workspace.** What shipped under ADR 0018 (provisioning, `/signup/*`, the signup page, password reset) stays live on alomails.com for dogfooding + existing accounts, but no further consumer investment (ADR 0018 slice 5 / consumer growth) is on the roadmap. "Someday, maybe" — un-parked only with a written case and business traction — ADR 0018, ADR 0020
- [3] DLP-lite: outbound rules ("warn on external send with attachment X")
- [3] Compliance packs: NIS2 evidence exports, processing-record templates

## alo Migrate

- [L] Everything in product-doc §6 — audit, identity, mail furniture, calendars, files+permissions, autodiscover, cutover safety, subscription retirement. Migrate is launch-critical and fully specified there; it is listed here so no one forgets it is a *product*, not a script.

---

## Business modules — the Work OS (ADR 0035)

alo's second act: the operational backbone SAP and Odoo sell, rebuilt from
scratch on our own foundations, **one wave at a time, each with its agent on
day one**. Wave tags map to the ROADMAP's Business track:
**[B1]**…**[B6]** = build order · **[B+]** = later waves, post-traction.
Same rule as above: nothing gets built that isn't listed, nothing listed
without a wave. AI lines follow ADR 0034: propose-then-approve, access-scoped,
EU models, suggest-only where the EU AI Act calls a use high-risk.

### [B1] Branding — one identity across the workspace

- [B1] Brand foundation: name, tagline, purpose, audience, positioning, personality, and voice in one reusable profile
- [B1] Visual identity: logo, accessible colour roles, and heading/body typography without app-specific copies
- [B1] Brand applications: live website, quotation, campaign, and document previews driven by the shared identity
- [B1] Guidelines: a generated, printable reference that explains the saved foundation and visual rules
- [B+] Governance: approval workflows, role-based publishing, audit history, historical version recovery, and external partner portals

### [B1] Billing — Quotes & Invoices (the wedge: EU e-invoicing mandates)

> Wave B1 is built. `docs/design/billing.md` § "What B1 promised, and what
> B1 shipped" reconciles every line below against the code — each one is
> shipped, or a cut with its reason. Two are **not** shipped and are named
> there: Peppol via an access point (a human contract, not loop work) and
> the cross-cutting record ↔ thread/file/task links (designed in B2).

- [B1] ★ **Billing agent** — "invoice Acme €2,400 for July consulting", "make that quote an invoice", "chase everyone overdue >14 days, politely" — drafts, user approves, alo Mail sends
- [B1] Customer records: billing address, VAT ID (VIES format check), payment terms, default currency — linked to existing Contacts
- [B1] Products/services price list: name, unit, price, VAT rate — the reusable line-item source
- [B1] Quote record: customer + dated line items (qty × unit price, per-line VAT), totals computed server-side, money as integer cents — never floats
- [B1] Quote lifecycle: draft → sent (as PDF via alo Mail) → accepted/declined/expired; accepted converts to invoice in one click
- [B1] Invoice record: same line model + issue date, due date, payment terms
- [B1] Legal sequential numbering: per-tenant, gapless, assigned at issue (drafts unnumbered), immutable once issued
- [B1] Credit notes: negative-total invoice referencing the original — the only legal way to "undo" an issued invoice
- [B1] PDF rendering: clean invoice/quote PDF with tenant branding (logo, footer, bank details). Invoice drafts now share the quotation document studio; accepted service quotations copy their design transactionally, and issue freezes the invoice design used by print and PDF.
- [B1] ★ **EN 16931 e-invoice**: Factur-X (PDF with embedded XML) + XRechnung/UBL output — the format German/French law requires; validated against the official schematrons
- [B1] ★ Peppol sending/receiving via an access point (integrate a certified AP first; own membership is an open decision)
- [B1] E-invoice **receiving**: inbound Factur-X/XRechnung parsed into a bill record (DE already mandates being able to receive)
- [B1] Payment tracking: mark paid (date, method, reference), partial payments, overdue view
- [B1] Payment reminders/dunning: manual first, then scheduled polite sequences — drafted by the agent, approved by you
- [B1] VAT summary report per period (the numbers your accountant asks for), CSV export
- [B1] Multi-currency invoices with a stored ECB rate at issue date
- [B2] Recurring invoices (subscriptions-lite): monthly/annual schedules, auto-draft for approval
- [B2] Payment links on invoices (integrate an EU PSP; never store card data)
- [B2] SEPA: pain.001 credit-transfer batch export for paying bills
- [B+] Customer portal page per invoice (view + pay, no login)

### [B2] CRM & Sales — deals that live on real email

> Wave B2 is built. `docs/design/crm.md` § "What B2 promised, and what
> B2 shipped" reconciles every `[B2]` line — here, in Billing above, and
> the two cross-cutting ones below — against the code: each is shipped,
> or a cut with its reason. Two are **not** shipped and are named there:
> payment links via a PSP (a human contract, not loop work) and
> role-based access per module (queued as B4.12, where it gets designed
> once rather than invented twice).

- [B2] ★ **CRM agent** — "turn this thread into a deal", "what's stalled in my pipeline?", "draft a follow-up for every deal quiet >1 week" — the mail-native advantage no standalone CRM has
- [B2] Lead/deal record: company + contact, value, currency, expected close, stage, owner, source
- [B2] Pipeline board: drag-between-stages kanban (same interaction as Tasks board), per-team pipelines
- [B2] ★ Deal ↔ mail-thread linking: the full email history of a deal in one place, automatically (same-domain matching, user-confirmed)
- [B2] Activities on a deal: notes, calls logged, next-step with due date (surfaces in Tasks/Agenda)
- [B2] Lost reasons + simple win/loss reporting; pipeline value by stage
- [B2] Quotes from a deal (bridges to B1); won deal → invoice
- [B2] Import leads from CSV/Excel; dedupe by email domain
- [B+] Web-form → lead capture; enrichment; forecasting; territory management

### [B3] Projects & Timesheets — from shipped Tasks to billable work

> Wave B3 is built. `docs/design/projects.md` § "What B3 promised, and what
> B3 shipped" reconciles every `[B3]` line below against the code: each is
> shipped, or a cut with its reason. **Billable hours → invoice is now a
> complete browser flow**: project overview, profitability report and approval
> completion all lead to one shared selector that raises the real draft and
> opens it in Billing. **Two of the agent's three
> example sentences are not tools**: setting a project up from a template
> (one screen, and nobody asked a machine for it) and "what's over budget?"
> across engagements (a portfolio question, which is a chart — Insights').
> Per-project access roles remain B4.12's, as in B2 and BI-1.

- [B3] ★ **Projects agent** — "set up the Acme onboarding project from our template", "what's over budget?", "draft this month's timesheet from my calendar" (draft only — you approve)
- [B3] Client projects: a project typed as client work (links a customer), budget in hours or money
- [B3] Milestones + simple timeline view over existing task boards
- [B3] Time entry: start/stop timer + manual entry, per task/project, billable flag, hourly rate
- [B3] Approval flow: submitted → approved timesheets (weekly), locked after approval
- [B3] ★ Billable hours → invoice lines in one click (feeds B1); unbilled-work view
- [B3] Project profitability: hours × rates vs budget, per project
- [B3] Project templates (recurring engagement setup)
- [B+] Gantt with dependencies; capacity planning; field-service work orders

### [B4] Expenses & Accounting core — the books

> Wave B4 is built. `docs/design/finance.md` § "What B4 promised, and what
> B4 shipped" reconciles every `[B4]` line below against the code: each is
> shipped, or a cut with its reason. One is load-bearing enough to repeat
> here. **The ledger does not yet post from the billing screens**: the posting
> rules for issue, settlement and credit note are written and golden-tested,
> and a reconciliation confirm books the invoice and its payment — but no
> `/billing` route calls a posting rule, and no rule posts an *expense* at
> all. A tenant who invoices and never reconciles has an empty journal, so
> the four reports over it are empty rather than wrong. Wiring the posting
> into each document's own transaction is the first item of any B4 follow-up.
> Also cut, each named there: no AI receipt backend (the extractor is
> deterministic behind the seam a model plugs into), no manual-journal-entry
> route or screen, no mileage screen, no expense category picker, no expense
> rebilling, and reports export CSV but not PDF.

- [B4] ★ **Finance agent** — "categorise last month's bank transactions", "anything unusual in March?", "prepare the Q2 VAT summary" — suggestions with sources, accountant approves
- [B4] ★ Receipt capture: photo/PDF → AI extracts vendor, date, amount, VAT — human confirms every field before it books
- [B4] Expense record: category, project link (billable → B1 rebill), payment method; approval flow (submit → approve → reimburse)
- [B4] Mileage claims at a per-km rate table
- [B4] Chart of accounts: sensible EU SME default, editable, per-tenant
- [B4] Double-entry ledger: every invoice/expense/payment posts journal entries automatically — correctness proven by property tests (debits always equal credits)
- [B4] Manual journal entries with description + attachment (accountant escape hatch)
- [B4] Bank statement import: CAMT.053, MT940, CSV mapping wizard — built by us, works with every bank, no licence needed
- [B4] ★ Reconciliation with AI matching: statement line ↔ open invoice/expense suggestions, one-click confirm, rules learned per tenant
- [B4] Fiscal periods with soft close (lock postings before a date)
- [B4] Reports: P&L, balance sheet, aged receivables/payables, VAT return figures — all exportable (CSV/PDF) for the accountant
- [B4] Accountant access role: read + journal-only rights, no mail/files
- [B+] PSD2 live bank feeds via a licensed EU aggregator (integrate, ADR 0009); DATEV export (the German accountant handshake)

### [B5] Purchasing & Inventory — things and stock

> Wave B5 is built. `docs/design/inventory.md` § "What B5 promised, and what
> B5 shipped" reconciles every `[B5]` line below against the code: each is
> shipped, or a cut with its reason. Two are load-bearing enough to repeat
> here. **Stock is counted, never valued**: no inventory asset account, no cost
> of goods sold, nothing posted to the ledger at all — a valuation needs a
> *method* (FIFO, weighted average, standard cost) which is a per-tenant
> accounting policy with tax consequences, so the stock screen shows a
> reference value at today's purchase price and refuses to call it a balance.
> And **half the wave has no screen**: suppliers and their price lists, manual
> adjustments and transfers, stocktakes, and the reorder rules with their
> shortage report are all shipped, routed and tested but reachable only
> through the API or the agent card. Those screens are the natural first items
> of any B5 follow-up. Also cut, each named there: the photo on a product has
> no picker, the delivery note is a record rather than a printed document, the
> third leg of the three-way match is not reconciled, and the agent writes no
> demand forecast from sales history.

- [B5] ★ **Inventory agent** — "what needs reordering?", "draft POs for everything under minimum", demand notes from sales history — always draft-then-approve
- [B5] Product catalog: SKU, barcode, unit, purchase/sale price, VAT, photos; services vs stocked goods
- [B5] Supplier records + per-supplier prices and lead times
- [B5] Purchase order: draft → sent (via alo Mail as PDF) → received; receiving updates stock and creates the bill (three-way match-lite)
- [B5] Stock on hand per location (multi-warehouse), stock moves with full history — quantities never edited in place, always moved
- [B5] Manual adjustments with reason codes; stocktake (count sheet → variance)
- [B5] Minimum-stock reorder rules feeding the agent's PO proposals
- [B5] Sales order: order → delivery note → invoice chain (bridges B2 → B1)
- [B5] Barcode scanning via phone camera in the web app
- [B+] Lot/serial tracking, expiry dates; landed costs; manufacturing-lite (BOM + production order); POS

### [B6] HR — people, without the payroll trap

> Wave B6 is built. `docs/design/hr.md` § "What B6 promised, and what B6
> shipped" reconciles every `[B6]` line below against the code: each is
> shipped, or a cut with its reason. Three are load-bearing enough to repeat
> here. **CV screening is refused, not deferred** — not suggest-only, not
> ranked, not scored, in any form; the line below still promises it and needs a
> product owner's amendment, and until then the document and the code disagree
> with the code being the conservative one. **The absence calendar does not
> render in Agenda**: the layer, the route and a month view all shipped, but
> the month is in People → Who's away and `web/src/agenda/` makes no HR call.
> And **five surfaces work without a screen** — leave policies, holiday-calendar
> selection, onboarding checklists, letter templates and the payroll export are
> all shipped, routed and tested, reachable only through the API. Four of those
> are administrative; the letter-template one is not, because the agent's
> `draft_letter_from_template` refuses any template the tenant has not written
> and there is no way in the product to write one. Also cut, each named there:
> the agent proposes no onboarding checklist, the letter tool has no proposal
> card of its own, and the payroll export has no tenant-defined column mapping.

- [B6] ★ **HR agent** — "who's off next week?", "draft a contract letter from the template", onboarding checklist proposals. CV screening is **suggest-only with mandatory human decision** (EU AI Act high-risk class), every decision logged
- [B6] Employee records: personal data, role, team, manager (org chart from this), documents (contract PDFs in Drive with HR-only permissions)
- [B6] Leave management: request → manager approval, balances per policy (annual, sick, unpaid), team absence calendar (renders in Agenda)
- [B6] Public-holiday calendars per country/region
- [B6] Onboarding/offboarding checklists (account creation ties into alo admin)
- [B6] Recruitment-lite: job openings, applicant pipeline board (CV in Drive, interview notes, stage kanban — same board pattern again)
- [B6] Expense/leave/timesheet approvals unified in one "approvals" inbox for managers
- [B+] Performance check-ins; training records; **payroll data export to local providers — payroll calculation itself is a permanent non-goal (ADR 0035)**

### alo Insights — see the whole business (ADR 0037) · waves [BI-1] after B2, [BI-2] after B4

> Wave BI-1 is built. `docs/design/insights.md` § "What BI-1 promised, and
> what BI-1 shipped" reconciles every `[BI-1]` line against the code: each is
> shipped, or a cut with its reason. Two narrowings are named there — tiles
> are rearranged from a menu rather than by dragging (the layout is an order
> plus a 1–4 column span, so there is nowhere to drag *to*), and **Spaces-
> scoped sharing is not shipped**: every member of a tenant sees every board
> until the first scoped role lands with **B4.12**.

- [BI-1] ★ **The zero-setup "Business overview"**: a pre-built dashboard that exists from day one — revenue, outstanding invoices, pipeline value, and more as modules land. No connectors, no ETL, no data person: everything already lives in one tenant-scoped database, which is the 80% of BI-tool complexity alo customers never pay for.
- [BI-1] **Insights tab**: dashboards of tiles (number, bar, line, pie, table), drag-arranged; shared via Spaces permissions (finance sees finance, sales sees pipeline).
- [BI-1] Gallery of ready-made tiles per module (Billing: revenue by month, overdue aging, VAT; CRM: pipeline by stage, win rate) — one click each.
- [BI-1] ★ **Ask-to-chart**: "revenue per customer this quarter as bars" → chart → Approve pins it. The AI emits a typed ChartSpec (measure/dimension/period/filters — never SQL), compiled against a whitelisted semantic layer; propose-then-approve (ADR 0034).
- [BI-1] Chart rendering via an embedded Apache-2.0 chart library under alo chrome (ADR 0033 precedent) — never a from-scratch chart engine.
- [BI-2] ★ **Visualization excellence — the Tableau bar**: every chart presentation-grade by construction — disciplined axes and gridlines, tabular numerals, emphasized latest values, legible direct labels over legends where possible, coherent per-measure color, dark/light correct; hover reveals exact figures; drill from a bar into its records; any tile exports as a clean image/CSV for a slide or a board meeting. The pitch: a data-viz professional's output from one typed sentence — Tableau's quality with none of its training course.
- [BI-2] Finance depth after B4: profit, cash flow, aged receivables/payables tiles; projects utilization + stock tiles.
- [BI-2] ★ Module-embedded overview strips (the same tiles atop Billing/CRM/Inventory) and the **Monday digest mail** — your numbers in your own inbox.
- [BI-2] Tile → CSV/PNG export; dashboard on a Meet/TV view (auto-refresh, no chrome).

### Cross-cutting (every business module gets these for free)

- [B1] Everything is tenant-scoped records with the same isolation guarantees (and tests) as mail
- [B1] Every record links to its mail threads, files, tasks — the "one point" promise in the data model
- [B1] Every module's numbers visible to **Ask alo** ("how much did Acme pay us this year?") — cited, access-scoped
- [B2] Audit log per record (who changed what, when) — sold as a feature, required for accounting anyway
- [B2] CSV/Excel import per module the day the module ships; Odoo import mappings grow over time (ADR 0035 migration story)
- [B2] Role-based access per module (finance vs sales vs HR see different worlds), on Spaces permissions

---

## alo Sites — the AI-native website builder (ADR 0036)

Tags: **[S1]** = v1 (site + blog + forms + both domain modes) · **[S2]** =
fast-follow · **[S3]** = wave 3 (editing on the page, the assistant,
commerce — settled in ADRs 0040/0041/0042/0050) · **[S+]** = later. Built
by the Sites track loop (`docs/autonomy/sites/QUEUE.md`).

- [S1] ★ **AI builds the first draft**: "tell me about your business" → a complete site — pages, sections, real copy — then you edit; every AI change is a preview-then-approve diff (ADR 0034 pattern)
- [S1] Section-based editor: add/reorder/remove typed sections (hero, features, text+image, gallery, testimonials, pricing, team, FAQ, CTA, contact form, nav, footer), each edited by a simple form — no pixel canvas
- [S1] Themes: palette + typography presets, logo + favicon (from Drive), one token-driven stylesheet
- [S1] ★ Static Rust rendering: fast, secure, SEO-correct HTML with near-zero JS — no WordPress attack surface
- [S1] Publish flow with immutable page snapshots (rollback later); live at `<name>.<sites-domain>` instantly
- [S1] ★ Custom domains: TXT-token verification + Caddy on-demand TLS, reusing the mail DNS-onboarding flow
- [S1] Contact forms: tenant-stored submissions, rate-limited + honeypot, internal-mail notification; CSV export; CRM-lead creation when B2 lands
- [S1] ★ Blog written in **alo Docs** — the existing editor publishes to the site (BlockNote→HTML renderer), with index page + RSS
- [S1] SEO: per-page titles/descriptions, OG tags, sitemap.xml, robots, canonical
- [S1] ★ Privacy-first analytics: daily aggregate visits + referrer domains, no IPs, no cookies, no consent banner — proven by tests
- [S1] ★ AI copy tools per section: rewrite, tone, shorten/lengthen — propose-then-approve

> **S1 reconciliation (2026-08-10):** all eleven S1 promises are accounted
> for in [`docs/design/sites.md`](design/sites.md#what-s1-promised-and-what-s1-shipped-s131b),
> including the full-address and direct-Home creation fixes from S1.30b/c.
> The only product dependency remains the one already stated above: automatic
> CRM lead creation waits for B2. Production container, Caddy, analytics-secret,
> and tenant AI-provider configuration remain explicit human operations.

- [S2] ★ Whole-site AI translation (multilingual EU sites in one click) with language switcher
- [S2] ★ **Collections — structured content from alo Base** (the CMS layer): repeatable content (menu items, team members, portfolio projects, FAQs) lives as an alo Base table, and a collection-backed section renders its rows — add a row, the site updates on next publish. One brain: the tables product IS the CMS; AI can fill/translate rows propose-then-approve. Includes per-collection field mapping (column → card title/image/price) and empty-state handling.
- [S2] **Site-editor role**: invite a colleague (e.g. the marketing person) who can edit and publish the site — and only the site: no mail, no files, no workspace admin (on Spaces permissions, same pattern as the accountant role).
- [S2] Version history + rollback UI; scheduled publishing; password-protected pages
- [S2] Image handling: crop/focus, AI alt-text, responsive srcset
- [S2] ★ **Site Insights — the EU answer to Google Analytics** (extends the consent-free S1 base; still no cookies, no banners, no individuals tracked — the rulings that outlawed GA in AT/FR/IT are the sales pitch): referrers + UTM campaigns, countries (coarse geo at ingest, IP discarded), device class, entry/exit pages, approximate read time, outbound-link clicks — all aggregate.
- [S2] ★ **Aggregated heatmaps**: per-page click positions + scroll depth from anonymous coordinates never linked to a visitor — the Hotjar wow without the consent problem (privacy posture reviewed at design time; stays aggregate-only forever).
- [S2] ★ **Conversions + full-funnel attribution**: form starts→submissions per page; submission → CRM lead (B2) → deal → invoice (B1) means alo Insights can answer "how much revenue did LinkedIn bring this quarter?" — the chain Google Analytics cannot see because it ends at the form. Site tiles join the Insights tab (ADR 0037).
- [S2] **Deliberate non-goal, written here on purpose:** no individual visitor journeys, no session replay, no fingerprinting — surveillance features would require consent banners and contradict the brand; the aggregate line is permanent.
> **S2 reconciliation (2026-08-13):** all nine `[S2]` lines above are
> accounted for in [`docs/design/sites.md`](design/sites.md#what-s2-promised-and-what-s2-shipped-s216c),
> together with the four `[S+]` lines the wave reached early (catalog
> storefront, booking section, sandboxed custom code, template gallery).
> Three dependencies are stated rather than hidden: AI translation and
> alt-text suggestions need a tenant-configured provider (their manual
> siblings do not), the country breakdown needs a country header from the
> edge proxy, and domain **selling** stays switched off until an ADR names an
> EU reseller and a PSP — the model, routes and screen exist behind an
> unconfigured registrar. Automatic CRM lead creation shipped as the
> deliberate one-click handoff rather than an automatic one.

- [S3] ★ **Editing on the page** (ADR 0042): type into the headline, drag a section and watch the page reflow, resize among each section's declared shapes only, and drag new sections in previewed with your own content — every gesture lands as the same reviewable typed change a form or an AI proposal makes; no pixel canvas, ever
- [S3] ★ **The site assistant that answers** (ADR 0040): a visitor's chatbot grounded in the published site, published posts and a deliberately published knowledge collection — every answer cites its page or the assistant refuses; a defaulted monthly spend ceiling with rate limits below it; appearance inside the site's own palette with contrast proven, never free-form CSS
- [S3] ★ **The assistant that acts** (ADR 0040): book a real free slot (with the visitor's own cancellation link), capture a lead into CRM through its owned seam (aggregate attribution only, no visitor journeys), and point at ticketed events — three verbs closed in code; it can never pay, invoice, discount or invent a price
- [S3] ★ **Tickets on the site** (ADR 0041): dated events referencing the Billing price list (never a copied price), seats held before payment so the last seat cannot be sold twice, hosted payment (card data structurally cannot reach alo), and a paid sale that fulfils itself — ticket page + calendar file, settled invoice, CRM contact — plus the ticket by email under ADR 0050's abuse rules
- [S3] ★ **Stock items on the site** (ADR 0041): sell what Inventory says is on the shelf — availability computed from the ledger at every read, the sale recorded as a real stock movement, one flat delivery rate per site, honest failure paths that name the refund instead of overselling
- [S3] **Shop setup proposed from one sentence**: the catalog, per-item VAT treatment and delivery rate drafted for review with every guess flagged in the type system — a price the model was not told arrives as a blank, never a number it made up; approval applies through the normal owned routes
- [S3] **Place-of-supply VAT rules table** — *blocked by design*: a tax professional reviews it before it is built; consumer-price VAT carving and invoice country stand as flagged provisional choices until then

> **S3 reconciliation (2026-08-16):** all Wave-3 strands above are accounted
> for in [`docs/design/sites.md`](design/sites.md#what-wave-3-promised-and-what-wave-3-shipped-s306c).
> Three dependencies are stated rather than hidden: the assistant answers
> only with a tenant-configured AI provider (until then it is honestly
> unavailable); **no live payment provider adapter exists yet** — checkout
> stays off until a human names Mollie/Adyen-class PSP credentials behind
> the built boundary (ADR work); and the ticket email is off until the
> deployment configures its own sender address (ADR 0050). The VAT rules
> table waits for its human-arranged tax review. The wave review also
> narrowed the commerce write-doors and price-list reads to owners — an
> invited site editor can look at what is on sale but not change it.

- [S+] Simple catalog storefront (order-by-form, no checkout) — **shipped in S2**; booking-page section (ties to Agenda) — **shipped in S2**; custom-code blocks (sandboxed) — **shipped in S2**; template gallery — **shipped in S2**
- [S+] ★ **Sell domains in-product** (S2 shipped the model, the routes and the screen; production ships an unconfigured registrar, so nothing can be bought until the ADR below is written) — buy `acme.com` inside alo and mail + site + ERP are live on it in minutes, zero DNS steps, because alo hosts the zone from second one. Built as a **reseller** over an EU wholesale-registrar API (Openprovider / Realtime Register / INWX class — never own ICANN accreditation at this scale); honest flat pricing, no first-year-bait renewals. Thin margin by design: this is the onboarding/retention closer, not a profit line. **Decided in ADR 0049** (Openprovider, an EU wholesaler; the customer is always the registrant; DNS phase 1 hosts the zone at the wholesaler so alo-run DNS stops being a blocker). **Remaining prerequisite:** the EU PSP checkout (B2 billing extension) — nothing can be bought until money can be taken. The item that sets `SITE_REGISTRAR` comes after that, in that order.
- [S+] **alo-run authoritative DNS** — host customer zones ourselves (integrate a proven server, PowerDNS-class, as a sealed container per ADR 0009; alo's Rust control plane owns records). The enabler for domain selling AND the universal mail onboarding (NS-delegation instead of per-registrar record copying), one zone infrastructure for both.

---

## Deliberately absent

No tracking pixels, no ad surface, no engagement mechanics, no consumer free tier, no dark-pattern storage nags. On the business side: **no payroll calculation, no tax filing, no bank connections built from scratch** (ADR 0035 — export/integrate instead). Every absence here is a sales argument; see Non-goals in the product doc for the build-side list.
