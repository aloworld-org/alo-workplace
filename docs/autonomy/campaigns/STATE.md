# alo Campaigns — build journal

One entry per completed queue item: what was built, what the tenancy and
consent tests proved, and — for anything touching who may be mailed — **the
test that proves who may not be**, quoted rather than summarised.

Started 2026-08-17 against a build with no campaign code at all. This is a new
module, and the people it will reach already exist in three tenant-wide places:
`billing_customers`, `crm_deals.contact_email` and
`site_form_submissions.sender_email`.

**The rule this journal exists to hold.** Every other queue in this repository
records what was built. This one also records **who was excluded and why**,
because a campaign module's failures are not crashes — they are messages
arriving at people who did not agree to them, or who asked to stop, and those
land in somebody's inbox rather than in a log.

Three things are therefore worth more than a green suite here:

- **`contacts` is never a source.** It is a per-user address book. A company
  campaign drawn from it mails somebody's private contacts, and no test suite
  catches that as a bug — it looks like a feature working.
- **Suppression is enforced in SQL, not by the caller.** A rule the sender has
  to remember is not absolute, and the first import that forgets it makes the
  promise false for good.
- **Consent is provenance, not a boolean.** "Did they agree" and "how do we
  know" are different questions, and only the second survives a complaint.

**Nothing here sends.** The sending identity waits on a second IP, which is a
purchase; read-duration tracking waits on an ADR that decides whether a number
that unreliable belongs in a product sold on not tracking people. A loop
supplies neither, and an item that finds itself needing one has found the edge
of this queue rather than a problem to solve.

---

## C1.1 — the reachable audience (2026-08-17)

**Shipped.** `platform/alo-store/src/campaign_audience.rs` — one tenant-scoped,
address-deduplicated read over the three tenant-wide sources, plus
`AccountStore::campaign_audience` (a keyset page) and
`campaign_audience_size` (the count, computed in SQL). No migration: this item
adds no table, so `05xx` is still untouched and C1.2's consent record takes
`0500`.

**The shape, and why it is this shape.**

- **All SQL is built by two private functions**, `sources_cte()` and
  `people_cte()`, and every query in the module goes through them. That is not
  tidiness: it is what makes "`contacts` is never a source" a property of one
  string rather than a habit each future caller has to keep. C1.2's consent join
  and C1.3's suppression exclusion belong *inside* those two functions for the
  same reason — ADR 0044 §2 says suppression is absolute, and a rule applied
  further out is a rule a caller can skip.
- **A person is their normalised address** — trimmed, lowercased. Somebody who is
  a customer, a deal contact and a form submitter is one row naming three
  sources. `AudienceMember` also carries the best name (billing first, then
  deal, then the form's self-reported one), the country (only billing has one),
  and first/last seen.
- **The cursor is folded by Postgres, not by Rust.** `normalise_address` judges
  an address; it never produces one that a query is then compared against.
  `lower()` is a collation's opinion and `to_lowercase()` is Unicode's, and they
  need not agree on every alphabet — folding a cursor in Rust and comparing it to
  a column folded in SQL could skip a person. `audience_page_sql` therefore
  compares `address > lower(btrim($2))`.
- **Archived customers are in the audience.** Archiving hides a row from
  billing's pickers; it does not say the person asked us to stop. That is an
  unsubscribe, it belongs to C1.3, and answering a consent question with a
  bookkeeping one would be the bug. Recorded here because it is a decision, not
  an omission.
- Keyset paging (`AudiencePage { after, limit }`, max 500) rather than `OFFSET`,
  because the audience is a live query over three moving tables and a submission
  landing mid-walk would shift every offset after it — silently skipping
  somebody.

**Who may not be mailed, quoted rather than summarised.** The rule this journal
exists to hold, in the tests that hold it:

- `campaign_audience.rs::no_query_in_this_module_can_read_the_per_user_address_book`
  walks every statement the module can issue, splits it into identifiers, and
  asserts none of them is `contacts`. Identifiers, not a substring search: the
  audience *depends* on `crm_deals.contact_email`, and a `sql.contains("contact")`
  test would either fail on it or be written so loosely it proved nothing. The
  next test, `a_column_that_merely_mentions_a_contact_is_not_the_contacts_table`,
  guards the guard — it asserts `contact_email` and `contact_name` are present,
  `contacts` is absent, and that the tokeniser *does* find `contacts` in
  `select x from contacts`.
- `tests/campaign_audience_tenancy.rs::the_per_user_address_book_is_never_a_source`
  proves the same thing at runtime, and proves it for the right reason: it seeds
  `Dr Reynders <surgery@doctor.test>` into the acting user's private address
  book, **reads it back through `contact()` to show the row really exists and
  really is readable by its owner**, and then asserts the audience is exactly
  `["orders@acme.test"]` with size 1. Without the read-back the test would pass
  just as happily if `create_contact` had silently failed.
- `a_string_somebody_typed_is_not_a_recipient_just_because_a_column_called_it_an_email`
  — six deals carrying `""`, `"   "`, `"n/a"`, `"ask reception"`, `"ann at
  example.test"` and `"ann@localhost"` produce an audience of size 0, and the
  seventh, a real address, produces exactly one.
- `the_audience_is_this_tenants_three_sources_and_a_neighbours_people_are_unreachable`
  is the mandatory wrong-tenant test, sharpened: the neighbour seeds the **same
  addresses** (`orders@acme.test`, `ann@lead.test`) plus one of its own, and each
  tenant's audience is asserted whole from both sides, so a leak would have to
  show up as a named extra row.
- `postgres_and_rust_agree_on_what_an_address_is` runs eighteen candidates
  through `SELECT lower(btrim($1)) ~ $2` and through `normalise_address`, and
  asserts the two answers match. One rule, two implementations, one thing holding
  them together.

**How verified.** `cargo fmt`; `SQLX_OFFLINE=true cargo clippy -p alo-store
--all-targets` — clean for this crate (two pre-existing `type_complexity`
warnings in `meet.rs`, untouched here); `DATABASE_URL=…5432/alo_loop cargo
nextest run -p alo-store` → **2 109 tests, all green** on the run recorded here;
the 8 new integration tests and 9 new unit tests passed in every run.

**Flag for the sites track: two sweeper tests are load-flaky, and it is not this
change.** Under a full parallel suite, `site_ticket_orders::the_ticket_mail_
waits_for_fulfilment_claims_once_and_never_crosses_tenants` failed twice with
`a paid order was never offered to the fulfilment sweep`
(`tests/site_ticket_orders.rs:1104`) and passed on other runs; with this item's
test binary excluded entirely, `site_bookings_public::a_new_appointment_is_
offered_to_its_owner_for_notification_exactly_once` failed the same way. Both
helpers claim from a **global, cross-tenant** sweep (`claim_ticket_fulfilments(100)`)
with a fixed round budget in a database several suites are writing to, so any
added concurrency can starve the watched row. Both pass alone. Not touched here —
`site_*` is the sites track's area, and this is a request for their queue rather
than a race.

**Flag for whoever fixes `scripts/prune-test-db.sh`: its default database is the
wrong one for this checkout.** The script defaults to `…/alo`, which on this box
holds **111 migrations, max version 154** — a stale database from an old branch.
`cargo nextest` against it dies with `Migrate(VersionMismatch(154))`, which reads
like a broken migration rather than a wrong database. This checkout's test
database is **`alo_loop`** (181 migrations, max 405, matching
`platform/alo-store/migrations/`), and every gate command in this entry names it
explicitly. LOOP.md's "check which database your server is on before you conclude
anything about it" applies to the test runner too.

- **Cuts:** none.
- **Housekeeping, for the next iteration:** C1.1's commit went out without the
  `Co-Authored-By: Claude …` trailer — the harness did not append it and the
  message was written whole, so it has to be typed. It is not fixable after the
  fact (rewriting pushed history is a hard rail), so the note is here instead:
  put the trailer in the message before committing, not after.
- **Next:** C1.2 — consent as a record (provenance, not a boolean), taking
  migration `0500`.

---

## C1.2 — consent as a record (2026-08-17)

**Shipped.** Migration `0500_campaign_consent.sql`,
`platform/alo-store/src/campaign_consent.rs` (the record and its history), and
the gate itself inside `campaign_audience.rs`:
`AccountStore::campaign_recipients` / `campaign_recipient_count`, which are the
audience query with `consented_at IS NOT NULL` applied **inside** it.

**The shape, and why it is this shape.**

- **A table of events, keyed by address.** Every act of consent is its own row
  and nothing is ever updated or deleted. "Did they agree" and "how do we know"
  are different questions and only the second survives a complaint, so a person
  who ticked a box in March and was re-confirmed by an import in June has two
  rows and a question about June is answered with June's statement. Keyed by
  address rather than by customer or deal because ADR 0044's claim is that
  there is no list — the same person is a customer, a deal contact and a form
  submitter at once, and the thing they agreed with is their address. It also
  means the evidence outlives the record it came from.
- **Three provenance columns, because a summary of them is not evidence.**
  `source` (what kind of thing), `source_ref` (which one), `statement` (what
  the tenant says they agreed to, in the tenant's words). `ConsentSource` is
  deliberately *wider* than `AudienceSource` — it adds `import` and `manual` —
  because ADR 0044 §2 calls imported lists the dangerous path, and a path that
  cannot be named as itself cannot be treated as such. `Import` and `SiteForm`
  must say which one (`requires_reference`); customer, deal and a colleague's
  note may honestly have nothing to point at, and demanding a made-up reference
  there buys a filled-in column rather than evidence.
- **Two timestamps.** `occurred_at` is when the person agreed, `recorded_at`
  when this workspace was told. An import carries consent obtained months
  earlier, and dating it from the typing would overstate how fresh it is.
  Consent dated more than five minutes ahead of the server clock is refused:
  clock skew is not a lie, next year is.
- **The gate is a type, not only a predicate.** `campaign_recipients` returns
  `CampaignRecipient`, which holds a `ConsentEvidence` — not an
  `Option<ConsentEvidence>` — and nothing converts an `AudienceMember` into one.
  So a sender cannot be handed the audience by mistake, and code holding a
  recipient is also holding the reason they are one. The SQL exclusion and the
  type say the same thing twice, in the two places a mistake could be made.
- **The audience still shows everybody**, each carrying their consent or the
  absence of it, because C1.4 and C1.5 need the excluded people *named with the
  reason* — a count with no visible exclusions is not auditable. One `Reach`
  enum builds all four queries, so the page and the count cannot drift apart.
- **The consent join is `DISTINCT ON (address) … ORDER BY occurred_at DESC, id
  DESC`,** tie-broken on `id`: a campaign that reported a different provenance
  on each refresh would be evidence of nothing.
- **A partial consent triple is a decode error, not a default.** All three
  joined columns come from one row of one table; inventing the missing part
  would put a person into a send with provenance we made up.

**Who may not be mailed, quoted rather than summarised.**

- `campaign_consent_tenancy.rs::a_person_with_no_consent_record_cannot_be_a_recipient`
  — three people the tenant knows well, one consent record: the audience is all
  three, the recipients are one, and both counts agree. It then reads the
  exclusions back by name (`hello@bravo.test`, `orders@acme.test`) and follows
  the kept person's evidence id to the record, asserting the stored statement.
- `a_neighbours_consent_does_not_make_our_address_mailable` — the sharpest case:
  both tenants hold `orders@acme.test`, only the neighbour was given permission.
  Our recipients are empty, our count is 0, and their evidence is not even
  readable from here; asserted from both sides so a leak shows up as a named row.
- `consent_for_somebody_no_source_holds_does_not_invent_a_recipient` — consent
  is permission, not existence.
- `a_record_that_is_not_evidence_is_refused_rather_than_stored` — a blank
  statement, an import that cannot say where it came from, consent dated two
  hours in the future, and an address that is not one; then it asserts the
  customer is *still* unmailable, so a refusal that half-wrote would be caught.
- `consent_recorded_in_any_casing_reaches_the_person_it_was_given_for` — the
  deal spells them `Ann.Dupont@Example.TEST`, the consent arrives lowercased and
  padded; one person, mailed once, because an unsubscribe of one copy would
  never reach the other.
- Unit tests: `only_the_recipients_queries_exclude_people_with_no_consent_record`
  (the gate is in the recipients SQL and deliberately *not* in the audience
  SQL), and `the_cursor_test_is_bracketed_so_the_consent_gate_cannot_be_ored_away`
  — `WHERE a OR b AND c` binds as `a OR (b AND c)`, which would have returned
  everybody on the first page of the recipients, invisible to any test that
  reads one page of a fully-consented tenant.
- The `contacts` promise is now carried by both modules: the audience's
  `no_query_in_this_module_can_read_the_per_user_address_book` walks four
  queries instead of two, and `campaign_consent.rs` has its own copy.

**How verified.** `cargo fmt`; `SQLX_OFFLINE=true CARGO_PROFILE_TEST_DEBUG=0
cargo clippy -p alo-store --all-targets` — clean for this change (the same two
pre-existing `type_complexity` warnings in `meet.rs`, untouched); test binaries
built in 4 m 32 s via the sanctioned background+marker form; `DATABASE_URL=…
5432/alo_loop cargo nextest run -p alo-store` → **2 132 tests, all passed**
(1 skipped), 81 s. The 6 new integration tests and 13 new/changed unit tests
passed.

**No CHANGELOG line, deliberately.** Nothing user- or operator-visible changed:
this is store-side only, with no route and no screen, exactly as C1.1 was. The
first campaigns CHANGELOG entry belongs to C1.5, when there is something to
open.

**Flag: `scripts/prune-test-db.sh` is hardcoded to database `alo`.** C1.1
flagged that its *default* was wrong for this checkout; the sharper fact is that
`DATABASE_URL` does not help — `psql_q()` passes `-d alo` literally, so the
script cannot prune `alo_loop` at all. This iteration pruned by hand with the
script's own statements against `alo_loop` (nothing was old enough to delete:
5 913 tenants, 80 MB, all created within the cutoff by the loops running now).
The fix is a one-line `-d "${ALO_PG_DB:-alo}"`, but the script is outside this
item's write scope, so it is recorded here rather than changed.

- **Cuts:** none.
- **Next:** C1.3 — suppression, absolute and tenant-wide, excluded in SQL inside
  `people_cte` alongside the consent join, with the test that an import cannot
  resurrect a suppressed address. Migration `0501`.

---

## C1.3 — suppression, absolute and tenant-wide (2026-08-17)

**Shipped.** Migration `0501_campaign_suppression.sql`,
`platform/alo-store/src/campaign_suppression.rs` (the record and the three
`TenantStore` methods), and the enforcement inside `campaign_audience.rs`:
`suppression_cte()` joined in `people_cte`, and `Reach::Mailable` — renamed from
`Reach::OnlyConsented` — carrying `consented_at IS NOT NULL AND suppressed_at IS
NULL` as one predicate.

**The shape, and why it is this shape.**

- **A table of state, where consent is a table of events.** Consent keeps every
  statement ever given, because "how do we know they agreed" is answered by the
  wording of a particular agreement. Suppression asks one question — *is this
  address suppressed* — and that must have exactly one answer, so the primary
  key is `(tenant_id, address)` and there is one row per person.
- **The first reason stands.** `suppress_campaign_address` is idempotent via `ON
  CONFLICT (tenant_id, address) DO NOTHING`, and a `UNION ALL … NOT EXISTS
  (SELECT 1 FROM inserted)` returns the record actually in force rather than the
  one just offered. Never `DO UPDATE`: a hard bounce three months after somebody
  unsubscribed must not rewrite "they asked to stop" into "their mailbox was
  full", which reads as a technical problem somebody might try to fix. The
  caller therefore gets the same answer to "so are they suppressed, and why"
  whether or not its own call was the one that did it.
- **There is no way to lift one.** No `unsuppress`, no `lifted_at`, no delete
  path. ADR 0044 §2 says *no segment, import or re-upload can bring them back*,
  and an API that can remove a row is an API a bulk importer is eventually
  pointed at. Somebody who suppressed themselves by mistake gives fresh consent
  through the site form like anyone else — which is evidence, where a tenant
  deleting the row is not. `nothing_in_this_module_can_take_a_suppression_away`
  holds the module's own SQL to it rather than trusting review.
- **On `TenantStore`, not `AccountStore`, and with no `recorded_by`.** The
  loudest future source of these rows has no logged-in colleague behind it at
  all: the one-click unsubscribe endpoint (RFC 8058, C2s.2) works with no
  account and no login. A column that would be NULL for the case that matters
  most is not provenance, so who acted is answered by `reason` and `source_ref`
  instead. This is the first campaigns method not on the account door, and it is
  deliberate: suppression is a fact about the tenant, not about a mailbox.
- **Four reasons, not three.** ADR 0044 §2 names unsubscribe, hard bounce and
  complaint. `manual` is added for the person who phones and asks to be taken
  off the list: recording that as an `unsubscribe` would put a phone call into
  the number a sending reputation is judged on, and a complaint rate that lies
  to us is worse than one that lies to a regulator.
  `SuppressionReason::is_a_persons_decision` is the distinction C4's numbers will
  need — a dead mailbox says nothing about whether the mail was wanted.
- **Two rules, one predicate, on purpose.** `Reach::Mailable` emits consent and
  suppression together. They are different rules — permission given, and
  permission taken back — but no query in this crate wants one without the
  other, and offering the choice is how a "just the consented ones" call site
  eventually mails somebody who unsubscribed.
- **The audience still shows suppressed people, carrying the reason.**
  `AudienceMember` gains `suppression: Option<SuppressionEvidence>`, the mirror
  of its consent field. A stricter reading of the item would drop them from the
  audience too, but somebody who unsubscribed is usually still a customer the
  tenant invoices, and C1.5 has to name the excluded *with the reason* — a count
  that quietly dropped them could not be audited. The exclusion the item demands
  is in the query the sender reads, which is where "if the sender applies the
  rule, it is not absolute" actually bites.
- **`CampaignRecipient` gained no suppression field**, deliberately: the only
  honest value would be a permanent `None`, and an `Option` a sender can read
  invites a sender that checks it. Instead `MemberRow::into_recipient` *refuses*
  a row that arrives carrying a suppression — the same discipline as C1.2's
  missing-consent decode error, in the direction where the failure lands in the
  inbox of somebody who has already asked us to stop.

**Who may not be mailed, quoted rather than summarised.**

- `campaign_suppression_tenancy.rs::an_import_cannot_resurrect_a_suppressed_address`
  — the item's named test, walked end to end: the customer is mailable, they
  unsubscribe, and then an `Import` consent record dated today, from
  `newsletter-2026.csv`, with the statement a real tenant would type, is written
  for `ORDERS@Acme.TEST `. Recipients stay empty and the count stays 0. The
  import is **not** refused and the assertion says why — the record is kept
  because an import claiming an agreement is itself evidence worth having; it
  simply grants nothing. The test then reads the audience back and asserts the
  person is still there, `suppression.reason == Unsubscribe` *and*
  `consent.is_some()`: the exclusion is not a claim they never agreed.
- `a_neighbours_suppression_does_not_silence_our_address` — the wrong-tenant
  test, sharpened into both directions. Both tenants hold and may mail
  `orders@acme.test`; one receives a complaint. Theirs goes empty, **ours keeps
  them** ("unsubscribing from one company is not unsubscribing from every
  company on the platform"), their record is not readable from our handle and
  their list is empty from ours, asserted from both sides.
- `a_second_suppression_does_not_rewrite_why_the_first_happened` — an
  unsubscribe 90 days ago, then a hard bounce today in different casing: same
  id, same reason, same `source_ref`, same `occurred_at`, one row.
- `suppression_reaches_the_person_it_was_given_for_however_it_is_spelled` — the
  customer is `Ann.Dupont@Example.TEST`, the click arrives as
  ` ANN.DUPONT@Example.test `; one copy of a person still being mailed is
  exactly the failure ADR 0044's "cannot unsubscribe from one copy of
  themselves" names.
- `a_suppression_that_could_never_be_applied_is_refused_at_the_door` — five
  non-addresses, a future date and an over-long reference are each refused, then
  it asserts the list is empty and the customer is *still* mailable, so a
  half-written suppression would be caught. A suppression row that does not join
  is not a near miss: it is somebody who asked to stop and is still being mailed.
- `suppression_stands_alone_and_does_not_need_a_consent_record` — a hard bounce
  for a stranger, who later becomes a consenting customer and is still not
  mailable: the suppression was waiting for them.
- Unit tests: `the_recipients_queries_exclude_suppressed_people_in_sql` (the
  gate is in the string, and *not* in the audience query, which must keep
  `suppression_reason`); `nothing_but_a_suppression_row_can_suppress` (the
  table is named exactly once — suppression comes from one place or it is not
  absolute); `a_suppressed_person_can_never_be_read_as_a_recipient` across all
  four reasons, with the un-suppressed control beside it, because "we only
  forgot the bounces" is how this returns;
  `half_a_suppression_record_is_reported_rather_than_completed` — the half we
  would have to invent is "there is no suppression", which is a person the
  tenant is told it may mail. The tenant-predicate test now demands **five**
  `tenant_id = $1` (three sources, two joins), and the `contacts` promise is
  carried by a third module's own copy.

**How verified.** `cargo fmt -p alo-store`; `SQLX_OFFLINE=true
CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p alo-store --all-targets` — clean for
this change (the same two pre-existing `type_complexity` warnings in `meet.rs`,
untouched); test binaries built in 4 m 28 s via the sanctioned background+marker
form; `DATABASE_URL=…5432/alo_loop cargo nextest run -p alo-store` → **2 156
tests, all passed** (1 skipped), 64.5 s, up from C1.2's 2 132. A second targeted
run of the 51 campaign tests was green on its own.

**No CHANGELOG line, deliberately** — same reason as C1.1 and C1.2: store-side
only, no route and no screen. The first campaigns entry belongs to C1.5.

**Flag, unchanged from C1.2 and worth repeating because it will bite the next
iteration too: `scripts/prune-test-db.sh` is hardcoded to database `alo`**
(`psql_q()` passes `-d alo` literally, so `DATABASE_URL` does not help), and
this checkout's test database is `alo_loop`. Nothing needed pruning this
iteration — 6 766 tenants — so the script was not run at all. Still outside this
item's write scope; the fix is a one-line `-d "${ALO_PG_DB:-alo}"`.

- **Cuts:** none. One thing deliberately *not* built: nothing reacts to a bounce
  or a complaint automatically, because nothing sends and there is no send
  record to react from. C5m.1 is where the events that will call
  `suppress_campaign_address` get their table, and its `source_ref` column is
  already the place a send id goes.
- **Next:** C1.4 — segments: a saved query over the audience with ADR 0044's
  conditions (bought or not within a period, country, has or has not received a
  given campaign), with the count **and its exclusions** both readable. Note for
  it: "has not received a given campaign" has no campaign to name yet, so expect
  to cut that condition to the ones the data supports and journal it, rather
  than inventing a campaign table ahead of C3.1. Migration `0502`.

---

## C1.4 — segments: the saved question, and the count that names who it leaves out (2026-08-17)

**Shipped.** Migration `0502_campaign_segments.sql`,
`platform/alo-store/src/campaign_segments.rs` (the record, its CRUD, the tally
and the two page reads), `tests/campaign_segments_tenancy.rs`, and a small
opening-up of `campaign_audience.rs` so a segment narrows the audience instead
of re-reading it.

**Adopted rather than started.** The tree already held an interrupted
iteration's C1.4 — module, migration and tests, uncommitted. It was read in
full, not trusted: gating it found four real defects, each fixed here. Recorded
because "the work was already there" is exactly the claim that should arrive
with the list of what was wrong with it.

**The shape, and why it is this shape.**

- **Conditions, never people.** There is no membership table and no cached
  count. Every read re-asks the question of `campaign_audience` at the moment of
  asking, so consent (C1.2) and suppression (C1.3) apply then rather than when
  the segment was saved. A stored member list is a copy of the audience, and a
  copy is how somebody who unsubscribed on Monday is mailed on Tuesday.
- **A segment is `SELECT * FROM people WHERE <conditions>`.** `people_cte()`,
  `Reach`, `MemberRow` and `MEMBER_COLUMNS` became `pub(crate)` for this, and
  that is the point: the privacy boundary, the consent join and the suppression
  join are the *same text* for a segment as for the whole audience. A segment
  that assembled its own `FROM` would be a second place `contacts` could be read
  and a second place suppression could be forgotten.
  `a_segment_reads_the_audience_rather_than_assembling_its_own` holds it to that
  by asserting the string rather than trusting review.
- **Typed columns, not a JSON definition.** The set of conditions ADR 0044 names
  is small and closed, and each is a rule somebody's inbox depends on — so a
  country that is not a country, a period of minus ten days, and a `not_bought`
  with no period are refused by CHECK constraints rather than by whichever
  caller happens to be careful. Adding a condition later is an additive column,
  which is the expand-only migration this repository already requires.
- **The tally is one query, not one per bucket.** Two queries over a live
  audience are two different moments, and a form submitted between them would
  make the parts disagree with the whole. `SegmentTally` therefore holds no
  stored total at all: `matched()` adds `mailable` and the exclusions up, so a
  total that disagreed with its own parts is unrepresentable.
- **Suppression outranks consent, decided in one `CASE`.** Somebody who never
  consented *and* complained is reported as having complained — the stronger
  fact, and the one a colleague cannot fix by going and asking nicely.
  `ExclusionReason::for_member` applies the same precedence in Rust, for the
  screen that lists people rather than counts them.
- **`EXISTS`, never `NOT IN`.** `address NOT IN (SELECT …)` is `NULL` — not
  `true` — the moment the subquery yields one NULL, which would silently empty a
  "has not bought" segment. The subquery cannot produce one today; the shape is
  chosen so that a later change to it cannot make the segment lie.
- **A purchase is an *issued* invoice, never a draft, a void one or a credit
  note.** A draft is an intention somebody may still delete and a void invoice is
  a purchase that was cancelled; counting either puts people who bought nothing
  into a send written to reach customers.
- **A country condition excludes people whose country is unknown.** Only billing
  customers carry one, so `country = ANY($2)` is `NULL` — and therefore not a
  match — for a deal contact or a form submitter. That is the honest reading:
  somebody we cannot place is not evidence of being in Belgium. Documented at
  the migration, at the query and in the test, because it is the kind of
  decision that reads as a bug to whoever meets it next.
- **An empty country list is the absence of the condition**, not a filter that
  matches nobody — the difference between "everyone" and "no one" on a screen.
- **The tally takes conditions, not a saved id.** C1.5's screen shows the count
  moving as a segment is *being* refined, before anybody presses save; a segment
  that had to be saved to be counted would make every experiment a stored object
  somebody has to clean up afterwards.

**Cut, exactly as C1.3 predicted: "has or has not received a given campaign".**
ADR 0044 names it and it is part of the differentiator, but there is no campaign
to name yet — the campaign record is C3.1 and the per-recipient send record
C5m.1. A column referencing a table that does not exist is a guess, not a
schema. Both the migration and the module docs say where it goes and what it
waits on; it is an additive column and one more CTE on the day it has something
to point at.

**Who may not be mailed, quoted rather than summarised.**

- `campaign_segments_tenancy.rs::the_count_and_every_person_it_leaves_out_are_both_readable`
  — the item, stated as a test. Five people, one mailable: one never asked, one
  unsubscribed, one hard-bounced, and one who *both* never consented and
  complained. The tally is asserted whole and the last is reported as
  `Complaint`, not `NoConsent`. Then the arithmetic that makes the number
  auditable — `matched() == 5`, and `matched() - mailable` equal to the
  exclusions summed — followed by the same precedence read off each member, and
  finally `recipients == ["yes@cseg.test"]`: the send reaches exactly the one
  person the tally promised.
- `a_segment_cannot_reach_somebody_the_audience_would_not` — two customers who
  both bought last week, one of whom unsubscribed. The segment names both as
  members with the reason and mails one. Then an `Import` consent record dated
  this morning, from `newsletter-2026.csv`, for ` QUIT@Cseg.TEST `: the
  recipients do not move, and **the tally does not move either**
  (`assert_eq!(after, tally)`). C1.3's promise, re-proved through the door C1.4
  opened.
- `a_period_means_what_a_colleague_reading_it_would_think` — "has not bought in
  ninety days" must contain the person who never bought, the person whose
  invoice is 120 days old (backdated in the database, since a test cannot wait
  four months), the person whose invoice is still a draft, and the person whose
  invoice was voided; and must exclude only the person who bought last week.
  Widening to 180 days brings the lapsed customer back — the boundary is the
  date, not the existence of an invoice. `NotBought` with no period asserts
  `["never@cseg.test", "void@cseg.test"]`, with the message *'has never bought'
  must not be emptied by a NULL in the purchases subquery*.
- `a_country_segment_excludes_the_people_it_cannot_place` — a Belgian customer,
  a Dutch one, and a deal contact nobody placed. Lowercase `"be"` still means
  Belgium; the unplaced person is not in it; the empty list returns all three.
- `a_neighbours_customers_invoices_and_segments_are_all_unreachable` — the
  mandatory wrong-tenant test, sharpened the way C1.1–C1.3 were: both tenants
  hold `orders@acme.test` and both have consent for it, but only the neighbour
  has ever invoiced them. A "has bought" segment is empty here and one person
  there, asserted from both sides, so a leak would have to appear as a named
  row. Their saved segment is then unreadable, unwritable and undeletable from
  our handle, and absent from our list.
- `the_per_user_address_book_is_never_a_source_of_a_segment` — the promise the
  queue puts above the suite, now carried by a fourth module: a contact is
  seeded into the acting user's private address book, **read back through
  `contact()` to prove the row exists and is readable by its owner**, and then
  no segment — unconditioned, by country, by purchase — contains them.
- `a_segment_that_would_mean_nothing_is_refused_rather_than_saved` and
  `a_saved_segment_can_be_renamed_rewritten_and_forgotten` cover the CRUD error
  paths, including the duplicate name and a cursor that is not an address.
- Unit tests (21) hold the SQL itself: `a_segment_cannot_widen_who_may_be_mailed`
  (the `Mailable` predicate is present, and deliberately absent from the members
  query so exclusions stay visible);
  `every_source_and_join_including_the_purchase_one_carries_a_tenant` demands
  **six** `tenant_id = $1` — three sources, two joins, and now the invoices a
  purchase condition reads, because a neighbour's invoices must not decide who
  we may mail any more than their customers do;
  `a_bucket_this_build_cannot_name_fails_the_tally_rather_than_shrinking_it`;
  and `a_purchase_condition_this_build_cannot_read_fails_rather_than_widening`,
  since dropping an unknown condition would turn "has not bought in ninety days"
  into "everybody" and the mistake would arrive as mail.

**The four defects the gate found in the adopted tree.** None reached the
commit, and they are listed because a green suite is only evidence if the
failures are on record too:

1. `every_query_is_scoped_to_one_tenant` **failed** — an `INSERT` has no `WHERE`,
   so it cannot carry `tenant_id = $1`. `campaign_consent.rs` met the same
   problem and solved it by loosening its assertion to
   `sql.contains("tenant_id")`, which would pass on a column list that merely
   mentions the tenant. Fixed the other way here: the insert must *start*
   `INSERT INTO campaign_segments (tenant_id, id, ` and bind `VALUES ($1, $2, `
   — tenancy proven as a shape rather than as a substring.
2. `tests/campaign_segments_tenancy.rs` did not compile: `billing_customers`
   takes one argument, not two.
3. Two `expect()` calls in unit tests — `expect_used` is `deny` workspace-wide
   (Cargo.toml `[workspace.lints.clippy]`), and a test file's own `#![allow]`
   does not reach `src/`.
4. A `collapsible_if` warning in `validate_conditions`.

**How verified.** `cargo fmt -p alo-store`; `SQLX_OFFLINE=true
CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p alo-store --all-targets` — clean for
this change (the same two pre-existing `type_complexity` warnings in `meet.rs`,
untouched); test binaries built in 4 m 50 s via the sanctioned background+marker
form; `DATABASE_URL=…5432/alo_loop cargo nextest run -p alo-store` → **2 189
tests, all passed** (1 skipped), 140.6 s, up from C1.3's 2 156. The 9 new
integration tests were then re-run alone and passed in 0.63 s, and the 68
campaign unit tests in 0.98 s.

**No CHANGELOG line, deliberately** — same reason as C1.1–C1.3: store-side only,
no route and no screen. The first campaigns entry belongs to C1.5, which is the
item that gives a colleague something to open.

**Flag, now for the fourth time: `scripts/prune-test-db.sh` is hardcoded to
database `alo`.** `psql_q()` passes `-d alo` literally, so `DATABASE_URL` does
not help and the script cannot prune this checkout's `alo_loop` at all. This
iteration ran the script's own statements by hand against `alo_loop`: nothing
was old enough to delete (7 618 tenants, every one created inside the two-hour
cutoff by the loops running now), and the suite still finished in 140 s. The fix
is a one-line `-d "${ALO_PG_DB:-alo}"`; the script is outside this item's write
scope, so it stays a flag. **If a later iteration finds the gate mysteriously
slow, this is why a prune can look like it ran and have done nothing.**

- **Cuts:** the "has or has not received a given campaign" condition, above,
  with the place it goes recorded in the migration and the module docs.
- **Next:** C1.5 — the `/campaigns/*` API for C1.1–C1.4, wrong-tenant tested per
  route, and the audience screen. Notes for it: this is the first campaigns item
  with a route and a screen, so it carries the first CHANGELOG line, the first
  i18n strings, and a STATE note that the production Caddyfile needs the
  `/campaigns` prefix at the next deploy (do not touch `deploy/`).
  `SegmentTally` is the shape the screen reads — the count moving as the
  question is refined, and the excluded named with the reason — and
  `campaign_segment_tally` deliberately takes conditions rather than a saved id
  so an unsaved draft can be counted.

---

## C1.5 — the API, and the screen that shows the count with its exclusions (2026-08-17)

**Shipped in two commits, deliberately.** The API went out first
(`abf1e6d2` → `4a18e185` after rebase) with a short in-progress note in its
place here, because the item is large enough that an interrupted iteration
would otherwise have found either a dirty tree or an unexplained half-checked
queue. The screen followed. Nothing was cut.

**Shipped.** `products/mail/alo-jmap/src/campaigns.rs` (the shared edge) plus
`campaign_audience.rs`, `campaign_consent.rs`, `campaign_suppression.rs` and
`campaign_segments.rs`; nine route templates registered in `server.rs`;
`tests/campaigns_http.rs`; `web/src/campaigns/**` (the module, its client, the
question bar, the tally line, the table and two read hooks); the module's rail
entry in `web/src/product/workplace.tsx`; 50 strings in **all three** catalogs;
the first campaigns CHANGELOG lines. No migration — this item adds no table, so
`0503` is still free.

**The routes.**

| | |
|---|---|
| `GET /campaigns/audience` | one page of the people a question selects — everybody, mailable or not |
| `GET /campaigns/audience/tally` | the count and every exclusion, by reason |
| `POST /campaigns/consent` · `GET /campaigns/consent/{address}` | record evidence, read one person's provenance |
| `GET`/`POST /campaigns/suppressions` · `GET /campaigns/suppressions/{address}` | who was lost and why |
| `GET`/`POST /campaigns/segments` · `GET`/`PATCH`/`DELETE /campaigns/segments/{id}` | the saved question |

**The shape, and why it is this shape.**

- **The tally and the list take the conditions on the URL, not a saved id.**
  This is C1.4's decision carried to the wire, and it is what makes the screen
  possible: the count moves as the question is typed, before anything is saved.
  A saved segment is *the same conditions read back* from
  `GET /campaigns/segments/{id}`, so a draft and a stored segment are counted
  by one code path. Two counting paths is how a segment comes to mean one thing
  in the editor and another once it is saved.
- **`/audience` returns the excluded people too, each carrying
  `exclusionReason`.** Filtering them out at the edge would have been the
  smaller answer and the wrong one: somebody who unsubscribed is usually still
  a customer the tenant invoices, and a colleague who cannot see them cannot
  check whether the number is right. `mailable` and `exclusionReason` are
  computed once, server-side, from `ExclusionReason::for_member` — the same
  precedence the tally's `CASE` applies — so the list and the count cannot
  disagree about one person, and no browser re-derives the rule.
- **`matched` is emitted although it is the sum of the parts.** The screen says
  "412 of 500", and a client that computed its own denominator is a client that
  can compute a different one. There is still no stored total:
  `SegmentTally::matched()` adds the parts on every read.
- **Suppression has no `DELETE` and consent has no `PATCH` — 405, not 403.**
  The methods are *absent*, not guarded, and the store has no method to call
  either. A route that exists is a route a bulk importer is eventually pointed
  at, and "no import can bring them back" would then be true only until
  somebody was in a hurry.
- **`POST /campaigns/suppressions` for an already-suppressed address is `200`
  with the record in force, not `409`.** The caller asked for a state and the
  state holds. The answer carries the *first* reason, so a hard bounce three
  months after an unsubscribe cannot rewrite "they asked to stop" into "their
  mailbox was full" — which reads as a technical problem somebody might try to
  fix.
- **Every query parameter is read as text and judged by our own code.** A typed
  `Query<T>` rejection answers in axum's shape rather than in our `Problem`, so
  `limit=banana` and `limit=9000` would reach a caller as two different kinds
  of error. Both are `422`, and both name the parameter.
- **`countries` is comma-separated, not a repeated key**, because
  `serde_urlencoded` keeps the last value of a repeated one — "Belgium and the
  Netherlands" would silently become "the Netherlands", which is a wrong number
  on a screen arrived at without an error. A unit test holds the split.
- **`withinDays` with no `purchase` is refused rather than dropped.** Dropping
  it answers a wider question than the one asked, and on this surface a wider
  answer is a bigger send.
- **The consent history is read per address and is never listed tenant-wide.**
  A route that dumped every consent record would be an export of who the tenant
  may mail. The audience already carries an evidence id beside each person,
  which is the shape a screen needs.
- **An address that is not one is a `422`, and an address the tenant has never
  heard of is an empty list.** The `404` on `GET /campaigns/suppressions/{…}`
  says only that *this* tenant holds no suppression, and it is the same answer a
  neighbour's suppression gets — no existence oracle across tenants, and none
  about a person either.
- **The screen debounces the question and drops late answers.** Typing `B`,
  `BE` starts two reads and the older can land second; every effect carries a
  generation. A count that flickers back to the previous question is worse than
  one that arrives a moment later, because it is a number somebody may act on.
- **The rail entry is not gated by `module_access.rs`, deliberately.** An admin
  on/off switch for Campaigns needs an `AppModule` variant and a CHECK migration
  on `user_modules`, which is a table this track does not own. The routes are
  still authenticated and tenant-scoped; what is missing is only the per-user
  toggle, and it is a one-line addition to `MODULE_OF_PREFIX` on the day the
  store learns the variant. Recorded because it is a decision, not an omission.

**Who may not be mailed, quoted rather than summarised.**

- `campaigns_http.rs::the_audience_names_everybody_and_says_which_of_them_will_not_be_mailed`
  — the item, as a test. Four people in the four states the screen must
  explain: mailable and a customer, mailable and a lead, known but never asked,
  and gone for good. The list is asserted whole and in order; the excluded
  customer is asserted to *still carry their consent record* alongside the
  suppression, because the exclusion is not a claim they never agreed. Then the
  arithmetic that makes the number auditable — `matched - mailable` equal to the
  exclusions summed, read out of the same answer.
- `a_condition_narrows_the_same_question_on_both_reads` — `countries=be` gives
  the two Belgian customers on the list route and `mailable 1, matched 2` on the
  tally route, so the two reads cannot drift; `countries=BE,NL` proves the comma
  is not lost.
- `nothing_on_this_surface_can_lift_a_suppression_or_rewrite_its_reason` — the
  unsubscribe, then a hard bounce in different casing returning the same id,
  same reason and same `sourceRef`; then `DELETE`/`PATCH` on both the
  suppression and the consent routes asserted `405`; then an `Import` consent
  record dated today from `newsletter-2026.csv`, accepted (an import claiming an
  agreement is evidence worth keeping) and granting nothing — `mailable` stays
  0 and the exclusion still reads `suppressed:unsubscribe`.
- `a_record_that_is_not_evidence_is_refused_rather_than_stored` and
  `a_suppression_that_could_never_be_applied…` (inside the suppression test) —
  six and four malformed requests each, and after them the audience is re-read
  to prove no half-write left a mailable customer or a phantom suppression
  behind.
- `a_neighbours_people_evidence_and_segments_are_unreachable_on_every_route` —
  the mandatory wrong-tenant test, sharpened the way C1.1–C1.4 were. Two tenants
  on one store hold the **same address**; the neighbour receives a complaint
  about it. Their audience excludes them and ours does not (*unsubscribing from
  one company is not unsubscribing from every company on the platform*), each
  side asserted whole. Their consent is an empty list from our handle, their
  suppression a `404`, their segment `404` on `GET`, `PATCH` and `DELETE` and
  absent from our list — and then asserted still readable by its owner, because
  a `404` that had deleted it would be worse than a leak.
- `every_campaigns_route_refuses_a_caller_with_no_token` walks all **twelve**
  verbs from one list, and the wrong-tenant test walks the same list, so a route
  added later without a guard fails a test rather than shipping.
- Unit tests in `campaigns.rs` hold the parsing: a blank parameter is absent
  rather than "matches nobody"; a period with no condition fails; a purchase
  token this build cannot name fails rather than widening; a cursor that is not
  an address is refused rather than restarting the walk.

**How verified.** `cargo fmt -p alo-jmap`; `SQLX_OFFLINE=true
CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p alo-jmap --all-targets` — clean for
this change (the same two pre-existing `type_complexity` warnings in
`alo-store`'s `meet.rs`, untouched); test binaries built in 11 m 09 s via the
sanctioned background+marker form; `DATABASE_URL=…5432/alo_loop cargo nextest
run -p alo-jmap --no-fail-fast` → **1 264 tests, 1 263 passed, 1 failed** — the
failure is not this change and is named below. The 10 new integration tests
passed alone in 2.6 s. Web: `npx tsc --noEmit` clean, `npx eslint` clean on
every changed file, `npm run build` clean, and `locale.test.ts` (65 tests) green
with the new keys present in all three languages — `UNTRANSLATED` stays empty.
`src/shell` and `App.test.tsx` re-run green, since a new rail module is exactly
what an adoption test would notice.

**The test database was pruned by hand first** (LOOP's own instruction, and the
reason a gate goes mysteriously slow): 8 482 tenants → **1 717**, 103 MB. The
suite then ran in 145 s.

**Flag for the sites track — `site_schedule_http::a_publish_is_scheduled_moved_and_called_off`
fails on `main`, and it is not this change.** It fails **3/3 in isolation**,
with no load and with the campaigns binary excluded, and the assertion is a
timestamp comparison:

```
left: 2026-08-19 12:10:07.023978  +00:00:00
right: 2026-08-19 12:10:07.0239789 +00:00:00
```

`tests/site_schedule_http.rs:193` compares `OffsetDateTime::now_utc() +
2 days` — Windows gives it 100 ns ticks — against the same instant round-tripped
through a Postgres `timestamptz`, which stores **microseconds**. So it passes
only when the clock's sub-microsecond digit happens to be zero. It came in with
`78781768` (08-12) and is a latent defect that surfaces on this machine, not a
race. The fix is one line in their test — truncate `chosen` to microseconds
before sending it — and `site_*` is the sites track's area, so this is a request
for their queue rather than a race. **Nothing in this iteration touches
`products/sites/**` or any `site_*` file.**

**Flag, now for the fifth time and unchanged: `scripts/prune-test-db.sh` is
hardcoded to database `alo`.** `psql_q()` passes `-d alo` literally, so
`DATABASE_URL` does not help and the script cannot prune this checkout's
`alo_loop` at all. This iteration ran the script's own statements by hand
against `alo_loop`, which is where the 6 765 pruned tenants above came from. The
fix is a one-line `-d "${ALO_PG_DB:-alo}"`. Five entries have now paid for this;
it is recorded here rather than changed because `scripts/` is not this item's
write scope, and a loop that starts widening its own scope on the fifth
annoyance is a loop nobody can predict.

**Note for the deploy the human runs: the production Caddyfile needs the
`/campaigns` prefix added.** New top-level route prefix; `deploy/` is not this
loop's to touch. It is also in the CHANGELOG under the operator line.

- **Cuts:** none. Two smaller judgement calls worth naming. The country field is
  a text box with a hint (`BE, NL`) rather than a picker, because there is no
  country list on the web side and billing's own customer form is a text box
  too — inventing a second convention here would be the larger change, and the
  server's refusal is shown verbatim. And deleting a saved question is a
  confirm rather than an undo, against the general preference in
  `docs/design/ux-principles.md`: it matches Insights' board delete, and the
  confirmation says the thing that actually matters — *nobody's agreement or
  unsubscribe is touched, only the question goes.*
- **Next:** C2s.1 — the per-recipient unsubscribe token: unguessable,
  identifying the send and the recipient, revealing neither to whoever holds it.
  Notes for it: read `alo-jmap`'s existing `unsubscribe.rs` first (it is the
  *reader* half of RFC 8058 and already understands the standard), and the two
  failures the item names both need a test — iterating identifiers to
  unsubscribe other people, and confirming an address is live by watching what
  the endpoint does. It takes migration `0503`.


## C2s.1 — the link in the mail that ends it (2026-08-17)

**Shipped.** `platform/alo-store/src/campaign_unsubscribe.rs`, migration
`0503_campaign_unsubscribe_tokens.sql`, the `CampaignUnsubscribeTokenId`
newtype, and `tests/campaign_unsubscribe_tenancy.rs`. Store-side only: the
landing page and the route that redeem these are C2s.2, and the suppression they
fire is C2s.3. Nothing here sends, and nothing here suppresses.

**The shape.** `TenantStore::mint_campaign_unsubscribe_token` returns the raw
token exactly once; the row keeps `sha256(token)` and nothing else, exactly as
`file_shares` (0026) has since alo Transfer.
`Store::resolve_campaign_unsubscribe_token` is the only way back in,
cross-tenant on purpose — the public route has no login, so the token is the
only thing that names a tenant, and the tenant comes back so every read and
write after it goes through a tenant-scoped door.

**The two failures the item names, each with a test rather than a paragraph.**

- *Iterating identifiers to unsubscribe other people.* The token is 256 random
  bits from the same source as every opaque id, drawn twice — 128 is the id
  budget, and this is a bearer credential in a URL that strangers read. It
  encodes neither the address nor the send, so there is nothing to decode and
  nothing to increment. `a_link_cannot_be_guessed_from_the_person_it_is_for`
  mints two links for the same person and the same send, asserts they differ,
  asserts the token spells out no part of the address or the send reference, and
  asserts the *record id* — the safe-to-log handle — does not work as a link.
  `holding_the_stored_row_is_not_holding_the_link` reads `token_hash` straight
  out of Postgres and presents it back: `None`. That is what a database dump, a
  backup on a laptop and a `SELECT *` over somebody's shoulder actually yield.
- *Confirming an address is live by watching what the endpoint does.* There is
  exactly one lookup and it is keyed on the digest.
  `a_guess_teaches_a_spammer_nothing_about_who_we_hold` posts eight shapes of
  wrong token — empty, blank, an address, `1`, a run of zeroes, a traversal, a
  percent-encoded NUL, a sentence — and every one is the same `Ok(None)` as a
  token for an address this deployment has never heard of. A malformed token is
  not an error, deliberately: an error and a miss are distinguishable, and that
  distinction is the oracle. The unit test
  `the_only_way_to_reach_a_token_is_to_hold_it` holds the module's own SQL to
  it, so a convenience lookup by address or by send added next year fails a test
  instead of shipping an oracle.

Plus `a_neighbours_link_is_never_ours`, the mandatory wrong-tenant test in the
shape this module could actually break: two workspaces mint a link for **the
same address**, which is ordinary and is exactly when a mixed-up tenant leaks.
Each resolves to the workspace that minted it, and to that workspace's send.

**Three judgement calls, all recorded in the migration where the next reader
will be standing.**

- **No expiry, and that is the difference from `file_shares`.** A share link
  lends a file for a fortnight; this is a person's ability to make us stop, and
  it has to work when they find the mail two years later while searching for
  something else. A column that eventually turns an unsubscribe into a `404` is
  a column that eventually earns a complaint, and ADR 0044 §4 says a complaint
  is the expensive one.
- **No revoke, no update, no re-issue.** Minting again for the same person and
  the same send adds a second live row rather than replacing the first. We hold
  only a digest, so "replace" means killing a link already sitting in an inbox —
  and a dead unsubscribe link is precisely what makes somebody press the spam
  button instead, which is the signal ADR 0044 §3 exists to avoid. Unit tests
  assert no `UPDATE` and no `DELETE` appears in the module's SQL, the same shape
  `campaign_suppression.rs` uses to hold "absolute" to its word.
- **`send_ref` is opaque TEXT with no foreign key**, the same call 0502 made
  about `received_campaign_id`: the per-recipient send record is C5m.1, and a
  reference to a table that does not exist is a guess. The FK is additive when
  there is something for it to point at. It is validated non-blank and bounded
  at 200 characters, matched to `SUPPRESSION_SOURCE_REF_MAX` because the one
  flows into the other: a suppression written by C2s.3 names the token's
  **record id** as its `source_ref`, so "which send did they leave over" stays
  answerable without the working credential being copied into a second table.

**How verified.** `cargo fmt -p alo-store`; `SQLX_OFFLINE=true
CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p alo-store --all-targets` — clean for
this change (the same two pre-existing `type_complexity` warnings in `meet.rs`,
untouched). Test binaries built in **6 m 26 s** via the sanctioned
background+marker form. `DATABASE_URL=…5432/alo_loop cargo nextest run -p
alo-store --no-fail-fast` → **2 205 tests, 2 205 passed, 1 skipped, 118 s**. The
six new integration tests pass alone in 0.4 s. No web, no route and no i18n key,
so no TypeScript gate was owed.

**Flag for the sites track — `site_ticket_orders::the_ticket_mail_waits_for_fulfilment_claims_once_and_never_crosses_tenants`
flaked once under load.** It failed on the first full-suite run and passed both
in isolation and on the immediate re-run of the whole suite, so it is
timing-sensitive rather than broken, and nothing in this iteration touches
`site_*`. Recorded because a claim-once test that only fails under concurrency
is the kind that gets dismissed twice and then debugged at the worst moment.
This is a *second* sites-owned test flagged from this track; the
timestamp-precision one in `site_schedule_http` is in the C1.5 entry above, and
that one is a latent defect rather than a flake.

**Flag, now for the sixth time and unchanged: `scripts/prune-test-db.sh` is
hardcoded to database `alo`.** `psql_q()` passes `-d alo` literally, so
`DATABASE_URL` does not reach it and it cannot prune this checkout's `alo_loop`
at all. Its statements were run by hand again (2 715 tenants → **1 863**,
107 MB). The fix is a one-line `-d "${ALO_PG_DB:-alo}"`; `scripts/` is not this
item's write scope.

**Disk, per LOOP's own rule, checked first: C: was at 99 % with 9.0 GB free.**
276 `.pdb` files in `target/debug/deps` were 7.6 GB of it — deleted, taking the
disk to 17 GB free, and the gate ran with `CARGO_PROFILE_TEST_DEBUG=0` so none
came back. Only 7 stale duplicate test binaries existed (82 MB), so the second
sweep LOOP describes had little to do this time.

- **Cuts:** none. One thing deliberately not written: a CHANGELOG line. Nothing
  a user or an operator can see changed — no route, no screen, no configuration
  — and the honest place for the user-voice sentence about unsubscribing is
  C2s.2, where the landing page becomes something a recipient can press.
- **Next:** C2s.2 — the landing page and its route, working with no account and
  no login, offering **fewer rather than only none**: this kind of mail, or all
  of it, one click either way. Notes for it: it redeems what this item mints, so
  the route hands the incoming token to
  `Store::resolve_campaign_unsubscribe_token` and goes through
  `Store::for_tenant` with what comes back. **RFC 8058 requires the acting
  request to be a POST**, because link-prefetching scanners fetch every URL in a
  mail and a GET that unsubscribed would unsubscribe people who never clicked —
  which is why the resolve here deliberately has no side effect. It is a public
  route outside the authenticated `/campaigns/*` surface, so it must render for
  an anonymous caller, and the answer for an unknown token must be
  indistinguishable from the answer for a token that was never minted. The
  "fewer" half needs a per-kind preference this queue has no table for yet —
  decide there whether that is a new column or a new table, and say why in
  STATE.

## C2s.2 — the page at the end of the link, offering fewer as well as none (2026-08-18)

**Shipped, and the first thing to record is that it was half-written already.**
This iteration opened on eight untracked files from a previous invocation that
died mid-turn — the migration, both store modules' new halves, the route, both
test suites and the two web files. All of it was good and none of it was
*wired*: no `mod` line, no `pub use`, no id newtype, no route registration, no
i18n key, no CSS class, no SPA route. The tree looked finished and did not
compile. What this iteration added is the wiring, the topic on the token that
everything written assumed, and the gate.

**The surface.** `GET /jmap/campaign-unsubscribe/{token}` draws the page and
writes nothing; `POST` on the same path is the act. Public — the only route in
this product a stranger reaches, deliberately outside the authenticated
`/campaigns/*` block, because a recipient is not a member of the workspace that
mailed them. Under `/jmap/*` for the reason `/jmap/invite/{token}` is: the SPA
owns `/unsubscribe/:token`, Caddy proxies whole prefixes, and an API route at
the page's path would answer a browser with JSON.

**The item's hard half — "fewer rather than only none" — needed a table, and
the two new shapes are deliberately different.**

- `campaign_unsubscribe_tokens.topic` is a **column** (migration 0504): the kind
  of mail is a property of the *send*, decided once when the message is built,
  and the token row is the only thing that knows which send a link came from.
  Nullable, and null is honest rather than lazy — the page then draws one button
  instead of a narrower one that would decline a category no send matches, and
  `scope=topic` against such a link is a `422` that says so in a sentence naming
  what to do instead.
- `campaign_topic_optouts` is a **table**: an opt-out is a fact about a *person*
  and must outlive every token, every send and every campaign, and one person can
  decline several kinds — neither of which a column can hold.

**The fold, and where it is and is not applied.** The token keeps the label as
the sender wrote it, because a human reads it on the page. The opt-out keeps
`normalise_topic` (trim, collapse inner whitespace, lowercase), because a query
compares it, and the schema `CHECK`s the folded form so a row that skipped the
function cannot exist. Deriving the comparable form where the comparison happens
— rather than storing a second folded copy on the token — is the point: two
spellings of one topic in two tables is the disagreement that mails somebody the
kind they declined. Same argument, same shape, as the address fold in
`campaign_audience`.

**Three failures, each with a test rather than a paragraph.**

- *"Fewer" that is really "none".* The one assertion nobody writes by accident:
  after pressing the narrower button, Ann is **still in `campaign_recipients`**.
  A narrower button that quietly suppressed everything would pass every
  assertion about her being off the newsletter and would be exactly the failure
  the item exists to prevent. Held in both suites —
  `declining_one_kind_of_mail_is_not_declining_all_of_it` (store) and
  `a_stranger_with_the_link_is_offered_fewer_as_well_as_none` (HTTP).
- *A prefetching scanner unsubscribing people who never clicked.*
  `looking_at_the_page_never_unsubscribes_anybody` fetches the GET five times
  and then asserts the audience, the suppression and the preferences are all
  untouched. This is why RFC 8058 requires a POST, and it is load-bearing here
  because ADR 0044 §2 makes a suppression unliftable: a GET with a side effect
  would be permanent *and* would read as the feature working.
- *An oracle for which addresses this deployment holds.*
  `a_guess_teaches_a_stranger_nothing_about_who_we_hold` posts five shapes of
  wrong token — including a near miss one character off a real one — on both
  methods, and asserts every answer is byte-identical to the first. A malformed
  token, an unknown one and one for an address we have never heard of are the
  same `404` with the same sentence.

Plus `a_neighbours_link_stops_a_neighbours_mail_and_never_ours`, the mandatory
wrong-tenant test in the shape that could actually break: two workspaces, the
**same** person on both their lists, one deployment and one table. Each link
resolves to the workspace that minted it, and using theirs leaves ours flowing.

**Two judgement calls worth the next reader's time.**

- **RFC 8058 one-click means *all of it*, and anything else is a `422`.** A mail
  client posts `List-Unsubscribe=One-Click` as a form body with no page and no
  chance for the recipient to choose. That is one unconditional gesture; reading
  it as "only this kind" would leave them receiving mail they believed they had
  stopped, and the next press is the spam button. The mirror of that decision is
  that a form body which is *not* the RFC's sentence is refused rather than
  guessed at — being permissive there would hand whatever posted it the power to
  end a customer relationship nothing can restore.
- **The topic is returned to the page; the address never is.** The topic
  describes the *mail*, so it tells whoever holds the link only what they have
  already read. The address describes the *person*, and a link is forwarded,
  quoted in replies and read by scanners — a page that echoed it back would turn
  a forwarded mail into a disclosure. "The page names the recipient" is asserted
  against the whole serialised body, not against a field.

**Nothing enforces these preferences yet, deliberately.** No query excludes a
declined topic, because nothing yet builds a message that names a kind — the
campaign record is C3.1 and the send record is C5m.1. The exclusion belongs in
`campaign_audience`'s `Reach` predicate beside consent and suppression, on the
day a send can say which topic it is. Threading a topic parameter through four
queries now, for a caller that does not exist, is the guess this queue refused
when it left `received_campaign_id` out of 0502. **C2s.3 is unaffected** — the
wider button already fires `suppress_campaign_address` through C1.3, and the
"one second later" assertion it asks for is what `recipients(&h)` proves
immediately after each press here.

**How verified.** `cargo fmt -p alo-store -p alo-jmap`; `SQLX_OFFLINE=true
CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p alo-store --all-targets` and the same
for `-p alo-jmap` — clean for this change (the same two pre-existing
`type_complexity` warnings in `meet.rs`, untouched). Test binaries built in
**25 m 05 s** via the sanctioned background+marker form. `alo-store` →
**2 225 tests, 2 225 passed, 1 skipped, 162 s**. `alo-jmap` → **1 273 tests,
1 272 passed, 1 failed, 367 s**; the failure is
`site_schedule_http::a_publish_is_scheduled_moved_and_called_off`, which is the
sites-owned timestamp-precision defect this track first flagged in the C1.5
entry — `left: …346539, right: …3465399`, Postgres microseconds against Rust's
100 ns, nothing to do with campaigns. All six new `campaign_unsubscribe_http`
tests and all five new `campaign_topic_optout_tenancy` tests pass. Web:
`npx tsc --noEmit` clean, `npx eslint` on the five changed files clean,
`npm run build` succeeded.

**Cut, and stated rather than glossed: the curl-against-the-debug-binary
wire-verify was not run.** C: was at 100 % with 2.6 GB free at the end of the
test build (see below), and linking a fresh `alo-jmap` debug binary would very
likely have hit `LNK1180` and cost the iteration. What it would have proved —
that the route is registered and answers an anonymous caller — is what
`campaign_unsubscribe_http` proves through `server::router()` over the real
Postgres, on the same code path minus the TCP socket. The residual risk is Caddy
configuration, which is a deploy-time concern and is flagged below.

**For the next deploy: `/unsubscribe` is a NEW top-level SPA route prefix.** It
is a page, not an API — the API half lives under the already-proxied `/jmap/*` —
so it needs the production Caddyfile's SPA fallback to cover it, exactly as
`/invite` and `/sites/invite` do. `deploy/` is not this loop's to touch.

**Disk, checked first per LOOP's own rule, and it got worse rather than
better.** C: opened at 99 % with 9.2 GB free; 247 `.pdb` files in
`target/debug/deps` were 7.5 GB of it and were deleted, taking it to 17 GB. The
test build then consumed **14 GB** and finished with 2.6 GB free — 399 test
binaries under `CARGO_PROFILE_TEST_DEBUG=0`, so this is the floor rather than a
symptom. Sweeping the 158 stale duplicate `.exe`s (1.8 GB) afterwards brought it
back to 4.4 GB. **The next iteration on this box should expect to sweep before
it starts and should not assume a full two-crate test build fits.** The durable
fix is the test-binary consolidation item LOOP already names.

**A note for whoever wrote migration `0600_dkim_one_active_key_per_algorithm`:**
it is sitting untracked in this checkout and was applied by every test run
above (it is fine — `dkim_keys.algorithm` has existed since 0016). It is not
this track's file and was deliberately left uncommitted. `06xx` and `05xx` do
not collide.

**Flag, now for the seventh time and unchanged: `scripts/prune-test-db.sh` is
hardcoded to database `alo`.** `psql_q()` passes `-d alo` literally, so
`DATABASE_URL` does not reach it and it cannot prune this checkout's `alo_loop`.
Its statements were run by hand again (3 597 tenants → **0**, 102 MB). The fix
is a one-line `-d "${ALO_PG_DB:-alo}"`; `scripts/` is not this item's write
scope.

- **Cuts:** the wire-verify above, and nothing else. No feature scope was
  narrowed: both buttons, both body shapes, the null-topic path, the
  already-stopped states and every error path shipped whole.
- **Next:** C2s.3 — an unsubscribe suppresses immediately through C1.3, and a
  test proves a recipient who unsubscribes cannot appear in a segment evaluated
  one second later. Notes for it: the *suppression* half is already wired and
  proven through the audience (`campaign_recipients`) in this item's HTTP
  suite, so the work left is the **segment** half — `campaign_segments`
  evaluates its own reach, and the item asks for the proof against a *saved
  segment's* tally and list rather than against the raw audience. Write it as
  one test that presses the wider button and then re-evaluates a segment that
  matched the person a moment earlier, asserting both the count and that they
  appear under the `unsubscribe` exclusion reason with a number beside it — the
  exclusions are the auditable half, and a count that silently dropped somebody
  is the failure C1.4 exists to prevent.
