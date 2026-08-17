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
