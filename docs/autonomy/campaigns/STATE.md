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
