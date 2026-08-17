# alo Campaigns — build queue

ADR 0044 (bulk email that cannot poison the mailbox), which `ROADMAP.md` orders
as waves C1–C6. **This queue does not execute all of them, and the reason is
the first thing to read.**

**Read ADR 0044 first, then `docs/design/campaigns-gaps.md`.** 0044 decided the
shape: a separate sending identity, consent as a record, absolute suppression,
one-click unsubscribe, and money rather than opens as the number reported. None
of that is open here. The gaps note says which competitor complaints are
structural and which are merely weak, and it is the reason the differentiator is
the segment and the attribution rather than the editor.

## What this queue builds, and what it must not

**Builds: C1 (the audience) and C3 (building the email), plus the store-side
half of unsubscribe.** None of it sends anything, and none of it needs a second
IP.

**Does not build: C2's sending path, C4, C5 beyond its data model, and C6.**

- **C2 is blocked on a purchase.** ADR 0044 §1 requires a dedicated IP or pool,
  and there is one IP on that server. A loop cannot buy the second one, and
  building the sending path against the transactional IP would collapse the
  whole argument of the ADR on the day it shipped. The *store-side* pieces —
  the unsubscribe token, the landing page, suppression firing from it — need no
  IP and are in scope; **generating and sending a campaign message is not.**
- **C5.4, read duration, is not this queue's to decide.** It needs a tracking
  pixel held open and reporting. Whether a number that unreliable belongs in a
  product sold on not tracking people is an ADR an owner writes, exactly as
  Meet-as-a-live-participant was left out of the agents queue. Do not build it,
  and do not write the ADR either.

## What is actually true today

- There is **no campaign code at all**. This is a new module, not a change to
  one.
- The people a campaign can reach already exist, in three tenant-wide places:
  `billing_customers`, `crm_deals` (`contact_email`), and
  `site_form_submissions` (`sender_email`).
- **`contacts` is a per-user address book** (`tenant_id, user_id, id, …`). It is
  somebody's personal contacts. A company campaign drawn from it would mail
  them, and that is a privacy boundary rather than a preference. **No query in
  this module may read that table**, and C1.1 carries the test.
- `unsubscribe.rs` in `alo-jmap` is the **reader** half of RFC 8058 — what our
  pane does when *we receive* mail offering one-click unsubscribe. The writer
  half does not exist. Read it before building the other end; it already
  understands the standard.

## Areas this track owns

`platform/alo-store/src/campaign*.rs`, `products/mail/alo-jmap/src/campaign*.rs`
and its routes, `web/src/campaigns/**`, and its own migrations.

**Migrations are `05xx`.** Agents hold `04xx`, sites `03xx`. Check the directory
again immediately before rebasing, not once at the start of the item.

**Do not edit** CRM, Billing or Sites modules to make an audience query easier.
Read them. If a join genuinely cannot be written without changing one of their
files, that is a request to put in their queue, not a race — the same rule that
kept the website agent out of the sites track's files.

---

## Wave C1 — the audience, and the two rules that make it safe

- [x] C1.1 The reachable audience: one tenant-scoped, address-deduplicated view over billing customers, CRM deal contacts and site form submissions. A person is one row however many sources hold them. **A test proves `contacts` is never read** — assert it against the module's own SQL, not by inspection.
- [x] C1.2 Consent as a record: when, from which source, from which address. Provenance stored rather than a boolean, because "did they agree" and "how do we know" are different questions and only the second survives a complaint. A person with no consent record cannot be a recipient, proven by a test rather than by a filter a caller remembers.
- [x] C1.3 Suppression, absolute and tenant-wide: unsubscribe, hard bounce and complaint each suppress, and **the audience query excludes them in SQL**. A test proves an import cannot resurrect a suppressed address. If the sender applies the rule, it is not absolute — that is the whole item.
- [x] C1.4 Segments: a saved query over the audience with the conditions ADR 0044 names — bought or not bought within a period, country, has or has not received a given campaign. The count **and its exclusions** are both readable; a number without them is not auditable.
- [ ] C1.5 The `/campaigns/*` API for the above, wrong-tenant tested per route, and the audience screen: the segment reading as a question with the count moving as it is refined, and excluded people named with the reason.

## Wave C2s — the store side of unsubscribe *(no IP required)*

- [ ] C2s.1 A per-recipient unsubscribe **token**: unguessable, identifying the send and the recipient, revealing neither to whoever holds it. Two failures it prevents and both need a test — iterating identifiers to unsubscribe other people, and confirming an address is live by watching what the endpoint does.
- [ ] C2s.2 The landing page and its route, working with **no account and no login**, offering **fewer rather than only none** — this kind of mail, or all of it. One click either way, no confirmation maze. A recipient offered only all-or-nothing presses the spam button instead, and that is the signal that ends a sending reputation.
- [ ] C2s.3 An unsubscribe suppresses immediately through C1.3, and a test proves a recipient who unsubscribes cannot appear in a segment evaluated one second later.

## Wave C3 — building the email

- [ ] C3.1 A campaign record: subject, preheader, and content as the **Docs block model** — one editor, not a second one.
- [ ] C3.2 The renderer: blocks → **email-safe HTML**, table layout and inline CSS, because Outlook renders through Word. A compiler, not a stylesheet, and the wave's hard part. Golden-file tests: the same blocks must produce the same HTML, so a regression is visible rather than discovered by a customer's recipients.
- [ ] C3.3 A plain-text alternative from the same blocks, assembled as `multipart/alternative`. Not optional — a campaign with no text part is scored as spam by filters older than this project.
- [ ] C3.4 Personalisation with a **visible fallback for every merge field**. "Hi ," is the classic bulk-mail failure and it comes from a field nobody defaulted; a field with no fallback is a validation error at save time, not a surprise at send time.
- [ ] C3.5 The mail must read with **images blocked**: alt text on every image, colour never the only carrier of meaning, and a dark-mode-safe palette. Half of recipients see that version and they are not a degraded audience.
- [ ] C3.6 Preview and seed test send within the tenant — the rendered HTML, the text part, and the merge fields resolved against a real record. The screen states honestly that a preview is our renderer's opinion and not proof of how Outlook 2016 will draw it. *(A seed send inside the tenant uses the existing transactional path and is not a campaign send; it does not touch C2.)*

## Wave C5m — the shape the numbers will land in *(model only, nothing measured)*

- [ ] C5m.1 The per-recipient send record and its event model — queued, sent, delivered, bounced (hard/soft), complained, clicked — with the suppression rules of C1.3 firing off the events that warrant it. **No sending and no tracking is built here**; this is the table those facts will be written into, and building it now keeps C4 from inventing a schema under time pressure.

---

## Done means

The implement skill's definition, plus one thing specific to this queue: **an
item that touches who may be mailed is not done without a test that proves who
may not be.** Consent, suppression and the `contacts` exclusion are each a rule
somebody's inbox depends on, and a rule with no failing case written down is a
comment.

When these are all `[x]`, append `LOOP COMPLETE` and stop. The waves left —
sending, measurement, automations — wait on an IP that has to be bought and an
ADR that has to be argued, and neither is a loop's to supply.
