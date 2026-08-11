# Design note — alo Finance (the books: expenses, the ledger, the bank, the return)

Status: **as built** (B4.15, the wave review; written as a design note at
B4.01, before the first migration) · ADR 0035 · Business track wave B4

> The `## As built` sections through this note were added by the slice that
> shipped each part. § **"What B4 promised, and what B4 shipped"** at the end
> reconciles every `[B4]` line of `docs/features.md` against the code — each is
> shipped, or a cut with its reason.

alo Finance is the fourth Work OS module and the first one whose output is
not a screen a colleague reads but a **statement a stranger audits**. B1
raises documents, B2 wins them, B3 fills them with hours; B4 answers the
question those three cannot: *what does this business actually own, owe and
earn* — and it must answer it the same way a tax inspector, an accountant
and a bank each expect, in a country whose rules alo does not get to choose.

The module is one sentence long, and every decision below is answerable by
asking which part of it is being protected: **every document that becomes
real posts a balanced journal entry the moment it becomes real, the bank
statement is matched against those entries by a human confirming a
suggestion, and the four reports are nothing but that journal, added up.**

Two consequences of that sentence are worth stating before the detail,
because they are what make this wave different from the three before it:

- **The ledger is not a view.** It is stored, append-only, and never edited.
  A wrong entry is corrected by a reversal, exactly as a wrong invoice is
  corrected by a credit note (B1.09) — the same discipline, one layer down.
- **The ledger never invents a figure.** Every cent it posts comes from a
  document that already exists and was already checked: `billing_totals`
  computed it, `billing_fx` crossed it, `billing_einvoice_import` refused it
  if it did not add up. B4 books what B1 says; it does not recompute it and
  it does not disagree with it.

This note records the surface, the chart of accounts, the journal and its
invariant (with the property-test plan the queue item asks for in its own
section), the posting rule for every document type, expenses and receipts,
the bank and reconciliation, period locking, the four reports, the first
scoped role alo has ever had, the error map, the tenancy rules, and the
out-of-scope list — each central decision with the alternative it rejects.

> **Wave gate, still flagged for a human, and now four waves deep.**
> `ROADMAP.md` gates wave B2 on "B1 live with ≥1 real tenant". B1, B2, BI-1
> and B3 are all code-complete and undeployed. A design note is exactly the
> work that belongs ahead of an unmet gate; **B4.02 is the first item that
> writes a migration**, and this wave is the one where shipping late costs
> most — a ledger that opens six months after the invoices it should have
> booked needs a backfill and an opening balance, which is § "When the books
> open" below rather than an accident.

## Surface

- **Inputs:** authenticated workspace users driving `/finance/*` on
  `alo-jmap` — the chart of accounts, manual journal entries, expense claims
  and their approval, mileage, uploaded receipts, uploaded bank statements
  (CAMT.053, MT940, CSV), the confirmation of a match, the period lock, and
  the four reports. Plus **one input nobody drives**: the postings written
  automatically inside somebody else's transaction, when an invoice is
  issued, a payment recorded, a credit note raised, a bill approved or an
  expense reimbursed.
- **Outputs:** JSON resources; CSV for every report and for the journal
  itself; balanced journal entries; **proposed** matches and **draft**
  expense categorisations, which are suggestions until a human says
  otherwise.
- **Who calls it:** `web/src/finance` (B4.13) calls `alo-jmap`; the `alo-ai`
  finance module (B4.14) produces propose-then-approve envelopes that
  `alo-jmap` executes against the same store functions; `billing_invoices`,
  `billing_payments`, `billing_bills` and `fin_expenses` call the posting
  functions **in-process, inside the transaction that made the document
  real**. Nothing external calls Finance, and Finance calls nothing external
  — no bank API, no tax portal, no rate service beyond the ECB file B1.21
  already imports from disk.

`/finance` is a **new top-level route prefix**: the production Caddyfile
needs it added at the next deploy (the standing human action `/billing`,
`/crm`, `/audit`, `/insights` and `/projects` already carry), and it must
join `API_PATHS` in `web/vite.config.ts` at B4.05b or every call 404s into
the dev SPA — the lesson S1.11, BI1.04 and B3.04 have each paid for once.
The prefix doubles as the SPA path, as all five of its siblings do.

### Routes

All under the authenticated `alo-jmap` router, following the convention
`/billing/*` established: the `authenticate` extractor, typed `Problem`
errors, the store-error map in [`billing::map_store_err`], registration in
`server.rs`. Every collection is the **second** path segment, because
`audit_action::event_for` derives a record's history from the matched route
template and `tests/audit_routes.rs` fails the build for a shape it cannot
read (the rename B3.11 paid for; it is not paid twice).

| Route | Purpose |
|---|---|
| `GET/POST /finance/accounts` · `GET/PATCH/DELETE /finance/accounts/{id}` | the chart of accounts: list (with balances for a period), add a custom account, rename, deactivate. Deleting an account that carries a posting is a `409` — a chart is history, not a preference (B4.02) |
| `GET /finance/entries?from&to&account&source&kind` | the journal, newest first, with its postings (B4.03) |
| `GET /finance/entries/{id}` | one entry, its postings, and the document it came from |
| `POST /finance/entries` | a **manual** journal entry: the accountant's escape hatch. Balanced or `422`, with an optional Drive attachment |
| `POST /finance/entries/{id}/reverse` | the only correction: a mirror entry dated today or later. There is no `PATCH` and no `DELETE` on this collection, deliberately |
| `GET /finance/entries.csv?from&to` | the journal as a file for the accountant's own tooling |
| `GET/POST /finance/categories` · `PATCH/DELETE /finance/categories/{id}` | expense categories and the account each books to (B4.05a) |
| `GET/POST /finance/expenses` · `GET/PATCH/DELETE /finance/expenses/{id}` | expense claims — the caller's own through the account door (shipped B4.05b, with the flow that needed them) |
| `GET /finance/expenses/pending` | **approver-only**: the claims of this tenant awaiting a decision, oldest purchase first, each with its claimant's address and its category's name. A view of the same collection rather than a second one, because the decisions are on the claim itself (B4.05b) |
| `GET /finance/expenses/reimbursable` | **approver-only**: the claims this tenant has approved and **still owes an employee** for, oldest decision first, same three facts beside each. Added by B4.13a, when the payer's screen needed a list it could work through: `pending?status=approved` would have been the wrong list, because an approved claim a company card paid is approved and owes nobody anything — it would sit in a payer's queue forever, refused by `reimburse` every time. The two reads share one joined statement in the store and differ only in their predicate |
| `POST /finance/expenses/{id}/submit` · `/withdraw` · `/approve` · `/reject` · `/reimburse` | the transitions; `submit`/`withdraw` are the claimant's, the last three approver-only (B4.05b) |
| `POST /finance/expenses/{id}/category/accept` · `/decline` | answer the category the agent SUGGESTED for one claim (B4.14a). The claimant's own verbs on their own claim, which is why they sit here and not beside the tool: accepting *is* picking a category and obeys every rule picking one by hand does (still theirs to change, word still offered). Declining clears the suggestion and is remembered, so nothing suggests that claim again |
| `POST /finance/receipts` | read a receipt already in the caller's Drive (`{nodeId}`), get the **parsed fields back for confirmation**. Writes nothing at all, and joins `READ_ONLY_POSTS` (as built, B4.06b) |
| `GET/POST /finance/mileage` · `DELETE /finance/mileage/{id}` | mileage claims; each becomes an expense at the tenant's per-km rate (B4.07) |
| `GET/PUT /finance/mileage/rates` | the per-km rate table, effective-dated (B4.07) |
| `POST /finance/imports/bank` | a statement file (CAMT.053, MT940, or CSV with a mapping) → a statement header and staged lines (B4.08). *Admin-or-accountant from B4.13b, with the seven routes below it: a statement is the whole company's money moving past a bookkeeper, not one colleague's own record the way a claim is, and `scoped_roles` leaves `/finance/*` to gate itself per route. Gating the two reads matters as much as the writes — a statement names every counterparty the company banks with.*|
| `POST /finance/imports/bank/preview` | the CSV mapping wizard's dry run: columns, sample rows, what would import. Writes nothing, and joins `READ_ONLY_POSTS` beside `/crm/imports/leads/preview` (B4.08c) |
| `GET /finance/bank/statements` · `GET /finance/bank/lines?status=&statement=` | what was imported and where each line stands (B4.08) |
| `GET /finance/bank/suggestions?statement=` | the ranked match candidates for every unmatched line — a read, never a write (B4.09c). *As built it is the bulk read, not the per-line one first sketched here: the ranking folds the open ledger once, so asking per line would fold it once per line.* |
| `POST /finance/bank/lines/{id}/match` · `/unmatch` · `/ignore` · `/unignore` | say what this line settled (which is what creates the payment and its postings), take that back, say it is not ours to book with the reason, or take *that* back (B4.09c) |
| `GET/POST /finance/rules` · `DELETE /finance/rules/{id}` | the per-tenant learned matching rules, listed and editable because a rule nobody can read is a rule nobody can trust (B4.09b) |
| `GET /finance/periods` · `POST /finance/periods` · `POST /finance/periods/{id}/close` · `/reopen` | the fiscal periods and the soft close (B4.10). *As built the two acts are named on the period they act on, not on the collection: `/lock` and `/unlock` were first sketched here, but a close is a decision about **one** period — it is what the audit trail records, what a screen shows beside that period, and what a refusal has to name. The list carries the derived `lockDate` beside the periods.* |
| `GET /finance/reports/pl?from&to` · `.csv` | profit and loss (B4.11a) |
| `GET /finance/reports/balance?on` · `.csv` | balance sheet at a date (B4.11b) |
| `GET /finance/reports/aged?on&side=receivable\|payable` · `.csv` | aged receivables and payables (B4.11c) |
| `GET /finance/reports/vat?from&to` · `.csv` | the VAT-return figures (B4.11d) |

Twelve path segments are reserved words under `/finance` — `accounts`,
`entries`, `categories`, `expenses`, `receipts`, `mileage`, `imports`,
`bank`, `rules`, `periods`, `reports`, `settings`. Ids are base64url'd
16-byte random tokens (`id.rs`), so a record can never *be* one of them, and
matchit prefers a static segment to a capture — the shape `/tasks/labels`
beside `/tasks/{id}` has had since ADR 0021. `pending` and `reimbursable` are
reserved the same way one level down, under `/finance/expenses`.

**There is no `POST /finance/entries/{id}` and no route that posts a
document.** Posting is not a verb a client may use: an entry exists because
a document became real, and the only door to that is the document's own
route. The one exception is the manual entry above, which is a document of
its own kind and says so in `kind = 'manual'`.

### Web surface

`web/src/finance`, the module pattern Billing, CRM, Insights and Projects
share: one `FinanceModule.tsx` owning the tabs, one `api.ts` owning the
fetches, one view per screen, `.module.css` for layout, `ds` tokens for
everything else, every string through `i18n/en.ts` under a `finance*` prefix
— **en/fr/nl, complete** since B4.15 (§ Languages, below).

- **Expenses** — my claims, and (for an approver) everybody's: submit,
  approve, reject, mark reimbursed. This is the screen most employees will
  ever see of the module, so it is the one the empty state onboards.
- **Bank** — import a statement, then the reconciliation screen: unmatched
  lines on the left, the suggestion and its evidence on the right, one
  confirm per line and an undo beside it.
- **Accounts** — the chart, editable, with each account's balance for the
  period; the journal behind it, filterable, with the CSV button.
- **Reports** — P&L, balance sheet, aged receivables/payables, VAT return;
  each a table with a CSV button and the period picker the VAT summary
  (B1.20) already uses.

Four tabs, not five: the manual journal entry is a dialog on the Accounts
tab rather than a screen, because it is the rarest action in the module and
a screen for it would rank it above the four things people do daily.

#### As built: the expenses slice (B4.13a)

`web/src/finance` exists with **all** of the four tabs drawn — Expenses
(B4.13a) plus an Approvals tab that only an approver sees, Bank and Match
(B4.13b, two tabs rather than one screen with a mode), and Accounts and Reports
(B4.13c, below). What Accounts does *not* yet carry is the journal behind the
chart and the manual-entry dialog: `/finance/entries` has no HTTP door, and a
tab that opens an empty screen is a promise the module has not kept.

Five decisions this slice took, none of which move anything above:

- **Two tabs, not one screen with a mode.** "My claims" and "claims I
  decide" are different data behind different doors, and one screen that
  changed meaning depending on who opened it would be the place a
  cross-user read eventually leaks. The Approvals tab is **hidden** for
  anybody who is not admin-or-accountant (the session's `alo:isAdmin` /
  `alo:roles`, read through `JmapClient.canWorkTheBooks`), while its route
  stays mounted — a bookmark works for the people who have it, and the
  server refuses everybody else regardless, because a client is never an
  access decision.
- **The approver's tab holds two lists, not two tabs.** Waiting for a
  decision, and approved-but-owed. One inbox, worked top to bottom.
- **What a row offers is the server's `editable`, never this file's reading
  of `status`.** An Edit button that always fails would teach the freeze by
  refusal, which is the one way a rule must never be taught.
- **The client computes no money and invents no currency.** Amounts are
  parsed at the edge with Billing's own parser (one comma rule for the whole
  suite) and sent as integer cents; an empty currency box is *omitted* from
  the request, so the workspace default is the server's decision.
- **The claim form is the same form for recording and correcting**, because
  the server takes the same shape for both.

#### As built: the bank and the reconciliation screen (B4.13b)

Two tabs, not one, both behind the same admin-or-accountant gate the Approvals
tab is behind — and, from this slice, behind the server's own gate as well
(above). **Bank** is the record of what has been imported and the door that
imports more; **Match** is the pile that import leaves and the afternoon of
small decisions it takes to clear it. One screen with a mode would have made
the import banner the thing a bookkeeper scrolls past four hundred times.

Six decisions this slice took:

- **The import is two steps, and the first writes nothing.** The dialog calls
  `/finance/imports/bank/preview` before it calls anything else, and shows the
  server's own reading — the format it sniffed, the encoding and delimiter, the
  columns it mapped, the first transactions as it understood them. Correcting
  any of that marks the reading *stale* rather than blanking it (the sample is
  what a person is correcting against), and the primary button goes back to
  "check this file". **What is shown and what is committed are always the same
  reading**; an Import button that staged a mapping nobody previewed would make
  the dry run advisory.
- **The browser never parses the file.** A second reader would disagree with
  the store on exactly the files that matter — a Windows-1252 export with comma
  decimals and a `Soll/Haben` column. The bytes go up; the reading comes back.
- **Every mapping control is a `<select>` over the file's own header**, never a
  text box. A column name typed by hand is a `422` waiting to happen, and the
  file already says what its columns are called. They appear for a CSV only: a
  CAMT.053 states its dates, currency and account itself, and offering to
  override them would invent a question the format has answered.
- **A `422` is rendered as the report, not as its sentence.** `BankImportRefused`
  carries the server's per-row report onto the client's error, and the dialog
  renders it — the line numbers and the rule each broke. This is the one refusal
  in the module a person is *expected* to act on.
- **The evidence is a token on the wire and a sentence on the screen.** The
  server sends `{"kind":"partPayment","remainingCents":69300}`; `evidenceLabel`
  writes "it is part of the invoice — €693.00 would be left" in the reader's
  language. A token this client has not learned is **dropped**, not printed:
  an untranslated identifier in a list of reasons reads as a bug, and the guess
  is still worth showing with the reasons that did translate.
- **The confirm sends the line's own `amountCents`.** The person picks *which*
  document, never *how much*; the store compares what was sent with what the
  bank said the line moves, under the row locks, so a screen a colleague changed
  underneath is a refusal instead of a payment for the wrong money.

The manual pick (B4.09c stage 3) is a dialog over Billing's own list of
documents that can still take money — issued, unsettled, not a credit note,
every one of those a fact the server computed. Without it the screen would be a
*suggestions* screen, and a line the two guessing stages had nothing to say
about could only ever be set aside.

Cut from this slice, and listed so the gap is a decision: **no split of one
transaction across several invoices** (the store refuses it — `bank_matches` is
unique per line, and lifting that is an additive migration, not a screen), **no
editor for the learned matching rules** (`/finance/rules` has no screen yet;
the rules still fire and are still named in the evidence), **no bank-side
posting for a line that is not an invoice payment** (an expense or a charge
settled from the bank is B4.13c's Accounts tab, which is where a person can
choose an account at all). The slice shipped English only; **B4.15 translated
it** with the rest of the module.

Cut from this slice, on purpose, and listed so the gap is a decision rather
than an oversight: **no category picker and no receipt attachment**. Both
need doors that do not exist yet — `/finance/categories` and the chart it
points into arrive with the Accounts tab (B4.13c), and the receipt path is
`POST /finance/receipts` over a Drive file, which wants the Drive picker.
A picker that is always empty is worse than no picker. An approver already
sees a claim's category name, because the queue read carries it.

#### As built: the chart and the reports (B4.13c)

The module's fourth and fifth tabs, both behind the same admin-or-accountant
gate — including the chart's **reads**, because the chart says what the company
owes, is owed and earns, and because the list is also what seeds it, so a read
here writes.

The doors this slice added are the ones the routes table above has always
promised: `GET/POST /finance/accounts` and `GET/PATCH/DELETE
/finance/accounts/{id}` (`finance_chart.rs`), with the default chart's *names*
in a table of their own (`finance_chart_names.rs`, en/fr/nl, checked against
`alo_store::CHART` by its own tests). Six decisions, none of which move anything
above:

- **The chart seeds itself on first read, in the caller's language**, and the
  answer carries `seeded` so the screen can say where twenty accounts nobody
  typed came from. `?lang=` is the mechanism Insights' Business overview already
  uses; the store holds no English at all.
- **Retiring is a field of the `PATCH`, deleting is its own door.** The routes
  table said `deactivate` and this is it. A `PATCH` is *merged* onto the stored
  record here, because the store's update is a full replace — so a rename that
  says nothing about the role cannot silently unhook a posting rule, which on
  this table means the next invoice stops booking.
- **`includeInactive` is `camelCase` on the wire.** Stated because serde's
  default snake case had quietly made it a parameter the server ignored: the
  screen that exists to bring a retired account back could not see one. The wire
  check found it; a unit test could not have.
- **Balances are optional and are the journal's.** `?from&to` folds
  `fin_trial_balance` once for the whole chart and states the accounting
  currency beside it; an account the period never moved carries a zero, and
  without the two days every movement field is `null` rather than `0` — a zero
  nobody read would be a claim about the books.
- **Reports are a second row of tabs**, one route each (`/finance/reports/pl`,
  `…/balance`, `…/aged`, `…/vat`), each a table with the period picker the VAT
  summary (B1.20) already uses and a CSV button that fetches the server's own
  `.csv` twin through the authenticated client. Nothing on these screens is
  arithmetic: the vitest fixtures deliberately state totals that differ from the
  sum of their own lines, so a browser that re-derived one would fail the suite.
  The balance sheet renders `balances`/`differenceCents` as a **failure banner**
  when they disagree, rather than printing a figure that looks exactly like a
  correct one.
- **Every link in the module is absolute** (`FINANCE_ROOT`). React-router
  resolves a relative `to` inside a splat route against the *current location*,
  so `to="reports"` clicked from `/finance/expenses` went to
  `/finance/expenses/reports`, which matched the catch-all, which redirected
  relatively again — a path that grew a segment per render. `Reports.test.tsx`
  is the regression. **The same defect is still present in Billing, CRM,
  Projects and Insights**, whose tabs are relative in exactly this way; it is
  flagged for the human in `docs/autonomy/STATE.md` rather than fixed here,
  because four other modules are not this item.

Cut from this slice, and listed so the gap is a decision: **the journal behind
the chart and the manual-entry dialog** (this note's Accounts tab describes
both; `/finance/entries` has no HTTP door yet, and a screen for a route that
does not exist is a promise the module would not be keeping), **expense
categories** (`/finance/categories` is still doorless, so the claim form still
has no category picker — B4.13a's cut, unchanged). The slice shipped English
only; **B4.15 translated it** with the rest of the module.

## The chart of accounts (B4.02)

```
fin_accounts          (grain: account)
  tenant_id, id       PK
  code                TEXT — what an accountant types; UNIQUE (tenant_id, code)
  name                TEXT
  type                'asset' | 'liability' | 'equity' | 'income' | 'expense'
  role                TEXT, '' for an ordinary account; UNIQUE per tenant when set
  active              BOOLEAN
  system              BOOLEAN — seeded by us; renameable, never deletable
  created_at, updated_at

fin_seeds             (grain: tenant × seed) — the insight_seeds pattern, exactly
  tenant_id, system_key PK, seeded_by, seeded_at
```

Two decisions carry this table.

**A posting rule finds its account by `role`, never by code.** `role` is a
closed set of our own words — `ar`, `ap`, `bank`, `cash`, `vat_output`,
`vat_input`, `revenue`, `expense_default`, `employee_payable`, `fx_diff`,
`rounding`, `opening_balance`, `suspense` — and exactly one account per
tenant may hold each. So "where does the receivable go" is answered by a
lookup that survives a tenant renumbering their whole chart to match their
accountant's, which they will.

*Rejected: hardcoding account codes in the posting rules.* A code is a
national convention (SKR03 and SKR04 disagree with each other before either
disagrees with the French PCG), and a rule that knows `1400` is a rule that
is wrong in every country but one, silently, in the direction of a
misfiled tax return.

**The default chart is a neutral EU-SME chart, seeded per tenant on first
read of `/finance/accounts`, recorded in `fin_seeds` so a tenant who deletes
it is not handed it again the next morning** — the BI1.06 mechanism, reused
whole, including the primary key that makes two simultaneous first visits
race-free (`ON CONFLICT DO NOTHING`, and the winner writes the accounts).

*Rejected: shipping national charts (SKR03/04, PCG, the Belgian MAR) in B4.*
Each is large, some are the property of somebody else, and every one of them
is a **compliance claim** — publishing a chart labelled "SKR04" asserts it is
SKR04, which a loop iteration is not entitled to assert. The neutral chart
plus roles gets a tenant booking correctly on day one, and adopting a
national chart is a recode their accountant does with a CSV import (named in
out-of-scope, flagged for a human).

**VAT is a dimension on the posting, not an account per rate.** One
`vat_output` and one `vat_input` account; the rate travels in
`fin_postings.vat_rate_bp`. *Rejected: an account per rate*, which is what
many charts do — a rate change (Germany's 19→16→19 in 2020, and the ViDA
changes now queued) then mints new accounts, and every report must know both
the old and the new to add up one year.

## The journal (B4.03)

```
fin_entries           (grain: entry — one document event)
  tenant_id, id       PK
  entry_date          DATE — the accounting date (the document's tax point)
  kind                'invoice' | 'credit_note' | 'payment' | 'bill' |
                      'bill_payment' | 'expense' | 'reimbursement' | 'mileage' |
                      'manual' | 'opening' | 'reversal'
  source_kind         '' | 'invoice' | 'payment' | 'bill' | 'expense' | 'bank_line'
  source_id           TEXT — the document
  source_event        TEXT — 'issue', 'void', 'settle', 'approve', 'reimburse'
    UNIQUE (tenant_id, source_kind, source_id, source_event) WHERE source_kind <> ''
  memo                TEXT
  reverses_entry_id   TEXT nullable → fin_entries
  attachment_node_id  TEXT nullable → a Drive node (manual entries)
  currency            TEXT — the document's currency
  fx_base_currency, fx_rate_micro, fx_rate_date   — the B1.21 snapshot triple
  created_by, created_at

fin_postings          (grain: posting — one line of one entry)
  tenant_id, id       PK
  entry_id            → fin_entries (composite FK, ON DELETE CASCADE)
  position            INTEGER
  account_id          → fin_accounts (composite FK, ON DELETE NO ACTION)
  amount_cents        BIGINT signed: positive = debit, negative = credit
  base_cents          BIGINT signed — the same money in the tenant's base currency
  vat_rate_bp         INTEGER nullable — which rate this tax belongs to
  customer_id         TEXT nullable   \
  supplier_key        TEXT nullable    | the dimensions a report groups by:
  project_id          TEXT nullable    | who, what engagement, whose expense
  user_id             TEXT nullable   /
  memo                TEXT
```

**As built (B4.03a), one correction to the line above:** the account link is
`ON DELETE NO ACTION` (the default) rather than the `ON DELETE RESTRICT` this
note first wrote. The rule it exists for is unchanged — deleting an account
that carries a posting fails with SQLSTATE 23503, which
`delete_fin_account` maps to "an account that carries postings cannot be
deleted" — but RESTRICT is checked *immediately*, so dropping a whole tenant
could fail depending on which of the two cascades from `tenants` Postgres runs
first. NO ACTION is checked at the end of the statement, by which time the
postings are gone too. This is 0106's lesson, and the tenancy suite deletes a
tenant with a journal to prove it.

There is no `updated_at` on either table, and that is a signal rather than an
omission: **nothing in this module ever updates a posted row.** The columns
that would need one do not exist.

### Signed amounts — the decision

**One signed `amount_cents`: positive is a debit, negative is a credit, and
the invariant is `Σ = 0`.**

*Rejected: separate `debit_cents` and `credit_cents`, each non-negative, at
most one non-zero* — the shape most textbooks draw and several ledgers store.
It costs a `CHECK` that only one is set, doubles every aggregate into
`SUM(debit) - SUM(credit)`, and turns the invariant from "adds to zero" into
"two sums are equal", which is the same statement written so that a reader
must check which column a report forgot. Signs are how the arithmetic is
actually done, and `billing_bills` already stores a credit note in **ledger
direction** (negative) for exactly this reason — 0111 says so in its header.
The debit/credit words survive where humans read them: the journal screen and
the CSV render two columns from the sign.

### Immutable, append-only, single-writer — the decision

**An entry is written whole, in one transaction, by one store function
(`fin_journal::post`), and is never updated or deleted afterwards.** There
is no API to add a posting to an existing entry, and none to change one.

This is not conservatism; it is what makes the invariant enforceable at all.
An entry that can never be edited can only be unbalanced at the instant it is
written, and exactly one function writes it — so one check, in one place,
covers every path forever. A correction is a **reversal**: a mirror entry
with `reverses_entry_id` set, dated on or after the original, which is what
an auditor expects to see and what a void or a credit note already does one
layer up.

*Rejected: a deferred `plpgsql` constraint trigger checking `SUM = 0` per
entry at commit.* It is the textbook defence-in-depth answer and it was
seriously considered. Three reasons against: CLAUDE.md's two-language rule
(a third language in our repos is a bug, and this repo currently contains no
stored procedure at all); a trigger duplicates the rule in a place our test
suite does not reach the way it reaches Rust; and it can only see rows, not
intent — it would happily pass an entry that balances and books the wrong
account. What replaces it is cheaper and catches more:

- the write path refuses an unbalanced entry with a typed `Validation`, and
  it is the only write path;
- `fin_journal::unbalanced_entries()` — `GROUP BY entry_id HAVING SUM(...)
  <> 0` — is a store function that the test suite asserts is empty after
  every property run, and that the admin health surface can call on a live
  tenant;
- the property tests in § below, which are the real defence.

### Idempotency, because a document must post exactly once

`UNIQUE (tenant_id, source_kind, source_id, source_event)` is the whole
mechanism. Issuing invoice X posts `('invoice', X, 'issue')`; a retry, a
double-click, or a re-run of the backfill hits the constraint and is a typed
`Conflict`, not a second set of postings. Voiding it posts `('invoice', X,
'void')` — a different event, so the reversal is representable without
weakening the key.

*Rejected: a `posted` boolean on the document.* Two places to be right, and
the one that is written outside the posting transaction is the one that lies
after a crash.

### Two currencies, and the balancing figure

Every posting carries both the document's currency amount and its base
equivalent, crossed through `billing_fx::convert_cents` at the snapshot rate
frozen on the entry — the same rate the document itself carries (B1.21, EU
VAT Directive art. 91: the rate is fixed at the tax point and an auditor
recomputes from the number printed on the paper).

**An entry must balance in the base currency, always, and in each document
currency present.** Two facts follow that a reader would otherwise trip on:

- **Conversion is per posting, and the residual has a home.** Rounding is
  not linear, so converting each posting at one rate can leave the base
  column off by a cent or two even though the document column is exact. The
  residual is posted to the `rounding` account, never absorbed into whichever
  posting happens to be last — that would misstate a real account to tidy an
  arithmetic artefact. A residual larger than one cent per posting is an
  internal error, not a rounding: the write refuses. This is
  `billing_fx::convert_totals`' own doctrine (cross the parts, sum the parts,
  never cross the whole) applied to postings.
- **A posting may have `amount_cents = 0` if and only if `base_cents ≠ 0`.**
  That posting is the exchange difference: a payment settling a USD invoice
  moves exactly the invoice's dollars (so the document column balances) but a
  different number of euro than the invoice was booked at (so the base column
  does not). The difference goes to `fx_diff`. Both columns zero on one
  posting is refused — a posting that moves no money in either currency is a
  typo.

## The invariant, and how it is proven (B4.03b)

The queue item asks for the debits==credits invariant "stated as a property
test plan". Here it is. `alo-store`'s tests run against the real database, so
these are integration properties, not pure-function ones; the generator lives
in the test module and is seeded deterministically so a failure is
replayable.

**The generator** produces a random *business month* for one tenant: 1–20
customers and suppliers, 1–50 invoices with 1–20 lines each (random
quantities in milli-units, random unit prices in cents, VAT rates drawn from
{0, 500, 700, 900, 1900, 2100, 2500} bp), a random subset issued, a random
subset of those credited, 0–3 payments per issued invoice (partial, exact, and
deliberately over), 0–30 approved bills, 0–30 expense claims across random
categories and payment methods, 0–10 mileage claims, in 1–3 currencies with
random-but-valid FX snapshots. Everything is generated through the **real**
store functions, never by inserting rows: a property that holds only for
hand-built fixtures proves nothing about the code that will run.

| # | Property | Why it is the one that would catch the bug |
|---|---|---|
| **P1** | Every entry balances: `Σ amount_cents = 0` per currency present, and `Σ base_cents = 0`. | The invariant itself. Asserted per entry *and* re-derived from the database with `unbalanced_entries()`, so an in-memory check that lies about what was written is caught. |
| **P2** | Every posting is non-trivial: `amount_cents ≠ 0` or `base_cents ≠ 0`, never both zero. | The FX-difference exception cannot become a hole. |
| **P3** | An issued invoice's AR posting equals `billing_totals::totals(lines).gross_cents` **exactly**; its VAT postings per rate equal that struct's per-rate subtotals; its revenue postings equal net. | The ledger books what B1 computed. If B4 ever recomputes totals, this fails — which is the point. |
| **P4** | For every document and its full credit note, the postings of both sum to zero **per account and per dimension**. | B1.09's "ledger of original + credit sums to zero", now true of the ledger and not only of the documents. |
| **P5** | After any sequence of payments totalling the gross, the AR balance attributable to that invoice is zero; after a partial, it equals the outstanding `billing_payments::Settlement` reports. | The two sub-systems agree about what is still owed. |
| **P6** | At any date, the `ar` account's balance equals the sum of outstanding issued invoices at that date, and `ap`'s equals the sum of unpaid approved bills. | The reconciliation between the ledger and the documents, asserted rather than assumed — and the reason B4 does **not** keep an open-item sub-ledger (§ Reports). |
| **P7** | Posting the same document event twice yields exactly one entry and a typed `Conflict`; the tenant's total posting count is unchanged. | Idempotency, including under the backfill. |
| **P8** | No sequence of API calls changes or removes a posted row: after the run, every posting's `(entry_id, account_id, amount_cents, base_cents)` tuple written earlier is still present byte-for-byte. | Append-only, proven behaviourally rather than by reading the source. |
| **P9** | Tenant A's generated month leaves tenant B's every balance, report and journal read **identical to before** — and A's ids are `NotFound` through B's handle. | Law 1, in the form that catches an aggregate missing a `tenant_id` rather than a single read. |
| **P10** | Every figure on the P&L, the balance sheet and the VAT report equals the same figure derived independently from the generated documents; and the balance sheet balances (`assets = liabilities + equity + P&L`). | The reports are the journal added up, and the journal is the documents. All three agree or the build is red. |

Two further tests that are not properties but belong beside them: a
**hand-written golden** entry per document type (B4.04a–c) so a reader can
see the expected debits and credits written out by a human, and a **no-float
grep test** in the same shape the billing suite already carries — money is
`i64` cents, rates are basis points, FX is micro-units, and `f64` appears
nowhere on the path.

### As built (B4.03b)

`tests/fin_journal_properties.rs` holds **P1, P2, P4, P7, P8 and P9** plus a
sixth test for the period window. P3, P5, P6 and P10 are each an assertion
about a *rule* or a *report*, and neither exists yet: they land with B4.04a–c
and B4.11a–d respectively rather than being weakened into something that
passes today. What stands in for them in the meantime is not a stub — the
generator keeps its own running tally as it posts, and `fin_trial_balance`,
`fin_dimension_balances` and `fin_account_ledger` are each checked against it
account by account, customer by customer and rate by rate. That is P10's
shape one layer below the reports.

The generator is the note's, with one honest narrowing: it produces the
*entries* B4.04's rules will produce (issue, settle, approve, credit) rather
than the *documents*, because there is no rule yet to turn a document into an
entry. Everything still goes through `post_fin_entry`, never an insert. Four
seeds run per invocation, and the month each produces must contain a
foreign-currency document, a rounding residual and an exchange difference —
asserted, because a generator that quietly stops producing the hard case
leaves a suite that is green and tests nothing.

Both queries were **mutation-checked** rather than merely run: summing the
document column instead of the base column fails three tests, and dropping
the `tenant_id` predicate from the ledger read fails P9.

### The posting-query API (B4.03b), which the reports are folds over

`fin_ledger.rs` is the read side, and it is deliberately three functions
rather than four reports' worth of queries:

| Function | Answers | Feeds |
|---|---|---|
| `fin_trial_balance(from, to)` | what every account moved in a window, with the two totals that must be equal | P&L (income + expense for a period), balance sheet (no lower bound, at a date) |
| `fin_account_ledger(account, from, to, limit)` | one account line by line, with the opening balance and a running column | every drill-down |
| `fin_dimension_balances(scope, dimension, from, to)` | what each value of one dimension moved, over the accounts a scope names | receivables by customer, payables by supplier, cost by engagement, VAT by rate |
| `fin_journal_range(from, to, limit)` (B4.14b) | a period of the journal, oldest first, **with every posting attached** | `flag_anomalies` — the one reader that judges entries rather than folding them |

The fourth was added by B4.14b and is a *read*, not an aggregate: the sketch
above pointed `flag_anomalies` at `fin_account_ledger`, and that was wrong.
A ledger answers "what did this account do", one account at a time; the three
anomaly rules ask about the *pairing* of an entry's postings — the same
counterparty, the same account, the same signed amount — and a per-account read
cannot see a pair. `fin_journal_range` orders by accounting date so that a
`limit` cuts a **contiguous range of days**, which is what lets the rule about
the rhythm of a cost trust the window it was given: a scan missing a scatter of
entries would invent gaps that are not there.

Three decisions inside them a later wave should not relitigate. **Every
aggregate is in the accounting currency** (`base_cents`): a total that adds
dollars to euro means nothing, and one that silently reports the majority
currency means something false. **A scope names a `role`, not a code** — so
receivables-by-customer stays right after an accountant recodes the chart.
And **the dimension is a closed Rust enum mapping to a column name written
here**, which is what keeps the one interpolated fragment in the module safe
by construction; a unit test asserts every variant is a plain posting column
and that a hostile account id travels as a bind parameter.

Two caps, both visible rather than silent: `LEDGER_PAGE_MAX` on a drill-down
and `LEDGER_GROUPS_MAX` on a grouped read, each with a `truncated` flag,
because a partial sum presented as a period's total is the one defect a
financial report must not have.

## Posting rules, per document type (B4.04)

Each rule is a pure function from a document to a `NewEntry` — no database
access, no clock, no tenant lookup beyond the account roles handed to it — so
every one of them is unit-testable against a hand-written golden before it is
ever wired into a transaction. `fin_rules.rs` owns them; `fin_journal.rs`
owns writing what they return.

| Event | Debit | Credit |
|---|---|---|
| **Invoice issued** (B4.04a) | `ar` gross, dimension `customer_id` | `revenue` net (per line, dimension `project_id` when the line came from B3); `vat_output` per rate, dimension `vat_rate_bp` |
| **Credit note issued** (B4.04c) | `revenue`, `vat_output` — the exact mirror | `ar` |
| **Payment recorded** (B4.04b) | `bank` (or `cash` — by the method map) | `ar`, dimension `customer_id`; plus `fx_diff` for the base-column difference |
| **Bill approved** | `expense_default` or the category's account, net; `vat_input` per rate | `ap`, dimension `supplier_key` |
| **Bill paid** (the SEPA run's line, reconciled) | `ap` | `bank` |
| **Expense approved**, paid by the employee | the category's account, net; `vat_input` | `employee_payable`, dimension `user_id` |
| **Expense approved**, paid by company card | the category's account, net; `vat_input` | `bank` |
| **Expense reimbursed** | `employee_payable`, dimension `user_id` | `bank` |
| **Mileage approved** | the mileage category's account (no VAT — a per-km allowance carries none) | `employee_payable` |
| **Manual entry** | whatever the accountant typed | whatever the accountant typed — balanced or refused |
| **Opening balances** | per account, as stated | `opening_balance` as the counterweight |

Four rules about *when*, which matter more than the table:

- **A draft posts nothing.** Draft invoices, unsubmitted expenses,
  unapproved bills and staged bank lines are not events; they are intentions.
  The ledger learns about a document the moment it becomes irrevocable —
  which for an invoice is `issue` (B1.08's gapless number), for a bill is
  `approve`, for an expense is `approve`, and for money is the day the bank
  says it moved.
- **The posting happens inside the document's own transaction.** Issuing an
  invoice and booking it either both happen or neither does. A "post later"
  queue would create the state this module exists to make impossible: a legal
  document that is not in the books.
- **A posting failure fails the document.** If the chart is missing a role
  the rule needs, the *issue* is refused, naming the role and pointing at the
  Accounts screen. *Rejected: issuing anyway and posting to `suspense`* — a
  suspense account is for money whose owner is unknown, not for a
  configuration mistake, and a silent suspense posting is discovered at the
  year end by somebody who cannot remember the invoice.
- **`entry_date` is the document's date, not today.** The invoice's issue
  date, the payment's `paid_on`, the expense's date — a ledger keyed on when
  a clerk typed is a ledger no period report can trust. This is what makes
  the period lock (B4.10) load-bearing rather than decorative.

### As built (B4.04a), the invoice rule

`fin_rules::invoice_issue_entry` is the table's first row, and
`fin_booking.rs` is the thin layer that reads the document, resolves `ar`,
`revenue` and `vat_output` **by role**, applies the rule and posts what it
returns (`AccountStore::post_invoice_issue`; `fin_invoice_entry` answers "is
this document booked?" without catching a conflict). Four things it does that
this note did not say in advance, each a decision rather than a shortcut:

- **Revenue is one posting per VAT rate, not one per line.** The table above
  says "per line, dimension `project_id` when the line came from B3" — and a
  billing line carries no project today (`billing_line::Line` is description,
  quantity, price, rate; B3's handoff writes the hours into the description).
  Per-line postings would turn a 400-line invoice into 400 identical credits
  carrying nothing the rate grouping does not already carry. When a line gains
  a project link, the rule splits the per-rate credit by project and nothing
  else about it changes.
- **The rate travels on the revenue posting too**, not only on the tax
  posting. A VAT return needs the taxable base per rate as well as the tax per
  rate, and taking both from the journal is what makes the return and the
  books provably one statement rather than two that agree today.
- **The receivable's base amount is the sum of the crossed parts**, never the
  crossed gross — `billing_fx::convert_totals`' doctrine with the parts being
  the revenue and tax postings. So the invoice rule can never leave a
  rounding residual, and the receivable the books carry
  is to the cent the figure `billing_fx::restated_into` reports for the same
  document — which is what P6 will need. Both suites pin this with a rate
  where the two ways of doing it **disagree** (1 EUR = 1.0880 USD on the
  golden document: €1 201.28 the parts, €1 201.29 the whole), because an
  example where they agree proves nothing about which one the code does.
- **A `paid` invoice is bookable and a `void` one is not.** Paid was issued
  first, and a backfill meets documents that have since been settled; void is
  booked by its issue entry and corrected by its `void` reversal (B4.04c's
  neighbour), so booking it as an issue alone would misstate the ledger.
  Draft and credit note are refused as `Conflict`, naming the rule that owns
  them.

**P3 is now asserted**, in both shapes the note asked for: a hand-written
golden entry in `src/fin_rules.rs` (the debits and credits written out as an
accountant writes them) and, on the wire in `tests/fin_invoice_posting.rs`,
the receivable equal to `billing_totals::totals(lines).gross_cents` with the
per-rate postings equal to that struct's own rows — checked against a total
this suite computes independently, and again one layer up in
`fin_trial_balance`.

**Not yet wired into `issue_billing_invoice`.** The rule above ("the posting
happens inside the document's own transaction") stands, and this is the
function that transaction will call — but making it fire today would make
issuing depend on a chart the tenant has never visited and on a books-opening
date that does not exist until B4.10. So the wiring lands with the periods and
the backfill, and until then the caller is explicit. This is a cut with a
date, not a permanent seam.

### As built (B4.04b), the settlement rule

`fin_rules::payment_settle_entry` is the table's third row, and
`AccountStore::post_payment_settle` is its booking layer
(`fin_payment_entry` answers "is this payment in the books?").

- **The two money legs are crossed at two different rates, and that is the
  whole point.** The bank leg is what the accounting currency actually
  received, so it crosses at the rate published for the day the money arrived;
  the receivable leg has to remove what the *invoice* put there, so it crosses
  at the rate frozen on the document (art. 91 again: the tax point's rate is
  the document's rate forever). The difference is the gain or loss made by
  being paid later, and it is posted to `fx_diff` on its own line with
  `amount_cents = 0` — the one posting shape the journal's "moves no money"
  rule deliberately allows.
- **The receivable relieved is cumulative, not per payment.** The rule is
  handed the total paid *before* this payment; the relief is what the whole
  prefix relieves minus what the shorter one did. That makes a settled
  document's receivable go to **exactly** zero in both columns, because the
  last payment carries the cent by which the crossed gross differs from the
  sum of crossed parts the issue entry booked. Relieving each payment at the
  plain crossed amount instead leaves a one-cent phantom receivable that no
  aged-debtors report can explain and no payment can ever clear; both suites
  pin this with a mutation-tested golden ($1 307.00 at 1.0880 settled at
  1.1000 and 1.0500). *So `rounding` is still unused:* every cent a settlement
  leaves over is an exchange difference, which has a better-named home.
- **The `fx_diff` role is required only when it can be reached** — that is,
  when the document is not in the accounting currency
  (`settlement_needs_exchange_account`). A chart missing it must not refuse an
  ordinary euro payment over a posting that rule provably never writes.
- **A payment refuses to book before its invoice does** (`Conflict`), because
  relieving a receivable nobody booked leaves the customer's ledger negative.
  Ordering within a document is `billing_payments`' own order read back to
  front: the reliefs telescope under *any* stable order, so booking payments
  out of sequence is safe.
- **The method map is a closed default, matched on whole words.**
  `payment_settlement_role` reads the words that mean physical cash (en/fr/nl
  plus German) as `cash` and everything else as `bank` — never a substring,
  because "cashless" is the bank. The per-tenant table this note promises
  replaces the constant behind the same signature when the Accounts screen
  grows a place to edit it.

**Not yet wired into `record_billing_payment`**, for the same reason and with
the same date as the invoice rule. One consequence to carry into that work:
`delete_billing_payment` is B1's correction path, so once payments book
automatically, deleting a booked one must post a **reversal** (which
`fin_journal` already supports) rather than silently leaving its entry
behind.

### As built (B4.04c), the credit-note rule

`fin_rules::credit_note_entry` is the table's second row, and
`AccountStore::post_credit_note_issue` is its booking layer. A credit note is
an invoice row (B1.09), so `fin_invoice_entry` already answers "is it booked?"
for one — there is no second reader.

- **It is the invoice rule, applied to the credit note's own document.** Both
  now call one private `sales_entry`, with the kind and the reversal link as
  the only difference. That is not code-sharing for its own sake: a credit
  note's lines are the original's with the quantity negated, `billing_totals`
  rounds half away from zero so `totals(−lines) == −totals(lines)` per rate,
  a credit note **inherits its original's frozen rate**, and `convert_cents`
  rounds half away from zero too — so every posting of the mirror is the
  negation of the original's posting **on the same account with the same
  dimensions, in both money columns**, by construction rather than by
  arithmetic that happens to agree.
- *Rejected: negating the original's entry.* Shorter, and wrong for the case
  that matters — a **partial** credit note, whose lines were edited before
  issue and are the negation of nothing. Booking the credit note's own
  document is right for both, and keeps P3 (the ledger books what billing
  computed) true of credit notes too.
- **The entry names the one it corrects** (`fin_entries.reverses_entry_id`),
  so a journal reader walks from a correction to what it corrected without
  parsing a memo. Which is why **a credit note refuses to book before its
  original does** (`Conflict`) — the same rule, for the same reason, as a
  payment refusing to settle an unbooked invoice.
- **Each refusal names the document the reader is looking at**: a draft
  *credit note* is an intention, an ordinary invoice is refused by the
  credit-note rule and vice versa, each message naming the rule that owns it.

**P4 is now asserted**, in both shapes: pure, in `src/fin_rules.rs`
(posting for posting, the pair sums to zero in both columns, at the 1.0880
rate where the crossed whole and the crossed parts differ by a cent), and on
the wire in `tests/fin_credit_note_posting.rs` — after the pair, every account
is flat in both columns, the customer's receivables group is zero and **each
VAT-rate group is zero**, which is P4's "per account and per dimension". A
partial credit note is asserted the other way round: what it leaves standing
is exactly the uncredited part, on the right rate and the right customer.

**Not yet wired into `issue_billing_invoice`** — which is where a credit note
is issued too — for the same reason and with the same date as the other two
rules.

### When the books open

A tenant that has been invoicing since B1 has documents older than its
ledger. Two mechanisms, both explicit, neither automatic:

- **`POST /finance/periods/open`** — an admin states the day the books
  begin. Documents dated before it are never posted; the balances they left
  behind arrive as an **opening-balances manual entry** the accountant
  writes (kind `opening`, counterweighted by the `opening_balance` account),
  which is exactly how a business migrating from another ledger opens ours.
- **A backfill**, run once per tenant and recorded in `fin_seeds`, posting
  every already-real document dated on or after that day, in date order,
  through the same rules. P7 is what makes running it twice harmless.

*Rejected: posting retroactively for every document that ever existed.* A
tenant's first two years may be in another system entirely; booking them here
would produce a second, disagreeing set of books for a period an accountant
has already closed and filed.

## Expenses, receipts and mileage (B4.05–B4.07)

```
fin_categories   tenant_id, id PK, name, account_id → fin_accounts,
                 default_vat_rate_bp nullable, active
fin_expenses     tenant_id, id PK, user_id (whose claim), spent_on DATE,
                 category_id, merchant TEXT, description TEXT,
                 gross_cents, vat_cents, vat_rate_bp, currency,
                 method 'personal' | 'card' | 'cash',
                 project_id nullable (the B3 bridge, and the rebill hook),
                 receipt_node_id nullable → a Drive node,
                 status 'draft'|'submitted'|'approved'|'rejected'|'reimbursed',
                 submitted_at, decided_by, decided_at, decision_note,
                 reimbursed_on, created_at, updated_at
fin_mileage      tenant_id, id PK, user_id, travelled_on, km_milli,
                 rate_cents_per_km (snapshotted), from_place, to_place, reason,
                 expense_id → fin_expenses
fin_mileage_rates tenant_id, id PK, effective_from DATE, cents_per_km, note
```

**An expense claim is personal data about an employee, and it uses B3's two
doors unchanged.** A person creates, edits, submits and withdraws their own
claims through `AccountStore` (every statement carries `user_id = self.user`,
so a colleague's claim is unrepresentable rather than merely forbidden); an
approver reads and decides through `TenantStore` behind a role gate. B3's
note wrote the rule for hours; a receipt is worse — it names a restaurant, a
pharmacy, a city on a date — and the same door answers it.

**A refusal hands the claim back; an approval keeps it** (as built, B4.05b).
The flow is `draft → submitted → approved → reimbursed`, with `rejected` beside
`draft` as the other state in which the claim is still its claimant's own —
editable, deletable and submittable. Refusing a claim is only useful if the
person can fix it and hand it in again, and a refused claim that could only be
deleted and retyped would lose the receipt link and the note explaining it (the
call `time_weeks` made for a refused week). Handing a rejected claim in again
**clears the decision**: a refusal that no longer stands must not still be
displayed on the record, and its history is in the audit log. Two more rules,
both refusals rather than silent no-ops: only a **submitted** claim is decidable
(a decided one is resubmitted by its claimant, not re-decided), and only an
**approved claim the employee's own money paid** can be marked reimbursed — a
company card left nobody owed anything, and recording a repayment against one
would book money out of the bank twice.

**Every field a machine extracted is confirmed by a human before it books.**
`POST /finance/receipts` returns parsed fields and **writes no expense**; the
client shows them in the create form for editing; the create call is an
ordinary create. *Rejected: creating a draft expense from the upload* — a
draft in a list is a thing somebody approves without reading, and the whole
value of the confirmation step is that the numbers are looked at once by
somebody who was there.

**The extractor is a trait with a deterministic implementation first**
(B4.06a): text layer or filename and a set of well-tested European receipt
patterns — dates in six spellings, `Summe`/`Total`/`Totaal`/`Montant`, VAT
lines with a rate beside them, IBANs, amounts with a comma decimal. It
returns *fields with confidences and the span each came from*, so the UI can
show why. An AI backend is a second implementation of the same trait, wired
by a human (ADR 0029, EU-only inference); **the loop never calls a model**,
and the fixture suite is what proves the seam.

*As built (B4.06a),* `fin_receipt.rs` is `ReceiptExtractor` +
`PatternExtractor`, `default_extractor()` being the one call site to change on
the day a second implementation exists. Every field of `ParsedReceipt` is an
`Option<Found<T>>` — value, `Confidence` (high/medium/low, deliberately coarse
so no threshold can grow into an auto-approval), and `Evidence`, either a
character span into the normalised lines the struct carries or "the file's
name". `today` is an argument, not a clock read, and it buys one rule: a date
in the future or more than ten years old is not when the money was spent. Three
readings the paper forced and the note had not written down: a **line naming
several rates yields the sum of the printed taxes and no single rate** (a hotel
folio at 7% and 19% states no one rate); a **tax amount is only ever an amount
printed with its cents**, which is what keeps `VAT Registration No. GB 123 4567
89` from becoming a tax; and the **amount grammar is shared**, not copied —
`money_text.rs` now holds the one answer to "is `1.234,56` a thousand or one
and a bit", used by both this extractor and the CRM lead import. Cut from the
slice: **IBAN detection** (the expense model has no field for one, so it would
have been an unreachable reading) and the OCR of a receipt with no text layer,
which is the AI seam's whole reason for existing.

*As built (B4.06b),* the receipt reaches finance **as a Drive node id, not as
bytes**: `POST /finance/receipts {"nodeId"}` → the file it read, whether there
was any text in it, the normalised lines, and one nullable candidate per form
field (`value`, `confidence`, `evidence`). The upload itself is Drive's own two
calls (`POST /jmap/upload`, then `POST /drive/files`), which is what this note
already required of the file — *in Drive under the claimant's own node,
referenced by id* — and it keeps the module from growing a second answer to
where a person's files live, with its own quota, naming and permissions. Three
consequences worth stating: the route **writes nothing at all**, so it joins
`READ_ONLY_POSTS` (an audit line saying somebody created what they only looked
at would be a false line in the log); the mandatory isolation test attaches
here, because the node is read through `drive_node` and a **colleague's**
private receipt is therefore as absent as another tenant's; and a claim can only
ever cite a file its claimant could already open. Two readings of the error map
that the paper forced: a node holding no bytes (a folder) is the `422` its row
promises, but **a media type we cannot read is a `200` with `textLayer: false`**
— refusing a photographed till roll would make the ordinary case an error, and
that row was written for an upload door this route does not have. `MAX_RECEIPT_BYTES`
is Drive's own index ceiling (12 MB), checked against the node's declared size
before any byte is fetched *and* against the blob's real length after.

**VAT on an expense is stated, never derived.** A receipt showing €119 with
19% is entered as gross 119 / VAT 19; a receipt showing only a total is
entered with VAT 0 and books nothing to `vat_input`. *Rejected: computing the
VAT from the gross and a category default* — reclaiming input VAT that the
receipt does not evidence is a false statement on a return, and the
difference between "the receipt does not show it" and "the receipt shows
zero" is exactly the difference a tax inspector asks about.

**Mileage is a claim at a rate table, not an expense with a made-up amount.**
`km_milli × cents_per_km` in integer arithmetic, rate snapshotted at the rate
effective on the travel date, and the resulting expense is ordinary from
there. Whether a given per-km rate is tax-free in a given member state is a
**human's statement**: the table ships empty with a note, not pre-filled with
Germany's €0.30.

*As built (B4.07),* `fin_mileage.rs` writes **both rows in one transaction** —
the journey and the draft claim it is worth — through the same
`insert_expense_in` the ordinary create uses, so a journey whose claim did not
land and a claim with no journey to explain it are both unreachable states
rather than states we clean up. Six readings the code forced and this note had
not written down:

- **The rate is picked in Rust, not in SQL.** The whole table (bounded at 50
  rows) is read inside the transaction and `rate_effective_on` — the latest row
  whose `effective_from` is on or before the travel day — chooses. A pure
  function with its own tests beats an `ORDER BY … LIMIT 1` nobody can exercise
  without a database, and the table is configuration, not a data set.
- **The rate table is replaced whole** (`PUT`), not edited row by row: it is
  read as one document, and per-row CRUD makes an intermediate state in which a
  period is missing and a journey in it is refused. Replacing is only safe
  because the journey snapshots its rate — which the suite asserts by rewriting
  the table under an existing claim and reading the claim back unchanged.
- **`GET` on the rates is everybody's, `PUT` is `require_admin`'s.** A traveller
  must know what a kilometre is worth before deciding to drive; a rate table
  anybody could raise is a self-service pay rise. The gate is at the edge, as
  the approvals inbox's is, because the store's job is that the write is the
  tenant's and the edge's is that it is the right person's.
- **The claim is `personal` and VAT-free by construction**, not by the caller's
  choice: those are what mileage *is* (the posting rule above credits
  `employee_payable`), and an allowance is not a purchase with input tax on it.
  Its currency is the tenant's accounting currency, read in the same
  transaction (`base_currency_in`), and its description is the traveller's own
  `reason` — never a sentence we composed, which would be hardcoded English.
- **There is no `PATCH` on a journey and no delete of its own.** Correcting one
  is deleting it and stating the right one, which re-reads the rate table;
  `DELETE /finance/mileage/{id}` refuses through the *claim's* rule
  (`is_editable`) and deletes the *claim*, the journey following by
  `ON DELETE CASCADE`. That is also what makes `DELETE
  /finance/expenses/{id}` on a mileage claim leave nothing behind.
- **A journey worth less than half a cent is refused**, not rounded up: 13 m at
  1 c/km is a claim of a cent that appears in every report and says nothing.

Cut from the slice: **no web surface** (B4.13a is the expenses screen and this
is the route it will call) and **no mileage category role** — a tenant points a
journey at whichever of their own categories they mean, and the posting rule's
"the mileage category's account" resolves through that ordinary link rather than
through a seeded word we would have had to name in English.

## The bank (B4.08) and reconciliation (B4.09)

```
bank_statements  tenant_id, id PK, account_iban, currency, source 'camt'|'mt940'|'csv',
                 file_sha256, opening_balance_cents, closing_balance_cents,
                 from_date, to_date, imported_by, imported_at
                 UNIQUE (tenant_id, file_sha256)
bank_lines       tenant_id, id PK, statement_id, line_no,
                 booked_on DATE, value_on DATE, amount_cents signed, currency,
                 counterparty_name, counterparty_iban, remittance TEXT,
                 bank_ref TEXT, line_hash TEXT,
                 status 'unmatched'|'matched'|'ignored',
                 ignored_reason TEXT (B4.09c; '' unless status = 'ignored'),
                 UNIQUE (tenant_id, line_hash)
bank_matches     tenant_id, id PK, line_id, target_kind 'invoice'|'bill'|'expense'|'entry',
                 target_id, amount_cents, confirmed_by, confirmed_at, rule_id nullable
fin_match_rules  tenant_id, id PK, match_on 'counterparty'|'remittance'|'iban',
                 pattern TEXT, target_kind, account_id nullable, customer_id nullable,
                 supplier_key nullable, hits INTEGER, created_by, created_at
```

**Three parsers, one contract.** CAMT.053 is XML and goes through
`billing_xml_tree` — the reader B1.24 hardened against DTDs, entity
expansion, unbounded depth and prefixes, which is precisely the tool for a
file a bank generated with software we have never seen. MT940 is a line
format (`:61:` statement lines, `:86:` remittance, `:60F:`/`:62F:` balances)
and gets its own small parser. CSV goes through `csv_read` — BOM and
encoding detection, delimiter sniffing, the CP1252 fallback that is half the
files in Europe — plus a mapping the user confirms in a preview. All three
produce the same `ParsedStatement`, and each has golden files from public
samples. A malformed file is a typed `422` naming the line, never a partial
import.

**A file imports once and a line imports once.** The statement's SHA-256 and
the line's hash (bank reference, date, amount, remittance, normalised) are
unique per tenant. Re-uploading last month's file is a `409` naming the
import; a statement that legitimately overlaps another loses only its
duplicate lines, and the import report says how many and why. *Rejected:
trusting the bank's own reference alone* — some banks reuse it, some omit it.

**Matching is three stages, and only the first is arithmetic.**

1. **Exact** (B4.09a): amount, sign and currency equal, and the remittance
   contains our own invoice number (`INV-YYYY-NNNNN`, which B1.08 makes
   unambiguous) or a bill's number, within a date window. This is the only
   stage that can be right by construction, and it is still a *suggestion*.
2. **Heuristic** (B4.09b): amount within a window of days, counterparty name
   similar to a customer or supplier, or a learned rule that fired before.
   Ranked, with the evidence shown — "amount and IBAN match, 3 days late".
3. **Manual** (B4.09c): the human picks, or marks the line `ignored` with a
   reason.

**Nothing is ever auto-confirmed** — ADR 0023's rule, and here it is also a
money rule: a wrong automatic match marks an invoice paid that is not, and
the customer stops being chased. Confirming a match is what creates the
`billing_payments` row (which posts, through B4.04b's rule), links the line,
and — if the user asks — writes the rule that would have matched it next time.
Rules are listed, editable and deletable, and each shows its hit count,
because a learned rule nobody can read is a rule nobody can trust.

**Unmatching is real.** `POST .../unmatch` deletes the payment it created and
**reverses** its entry (a reversal, never a deletion), returning the line to
`unmatched`. Refused once the period is locked, which is the next section.

### As built: the first parser (B4.08a)

`bank_statements` and `bank_lines` exist as written above, with three
differences the CAMT reader forced and which MT940 and CSV will inherit.

**The balances are nullable.** A balance the bank did not state is absent, not
zero: a zero would be a reconciliation target that quietly disagrees with
reality, and refusing such a file instead would throw away every line in the
month over a figure that is a check rather than the point of the import.

**`statement_ref` was added** — the bank's own name for the statement
(`<Stmt><Id>`, and `:28C:` when MT940 lands). It is the number a person
cross-checks against the paper, it costs one column, and both formats state it
for free. A CSV export has none, hence the empty default.

**The line hash carries an occurrence number.** Two genuinely distinct
transactions can be identical in every field a bank states — two €3.40 coffees
at the same shop on one day, with no reference between them — so the n-th line
with identical content hashes with `n` in it. Without that, the second coffee
would vanish for ever as a duplicate. Re-importing the same file re-derives the
same numbering, and an overlapping file lists the same pair in the same order,
so de-duplication still holds exactly. The **value date is deliberately not in
the hash**: some banks restate it when a booking is corrected, and a line whose
hash moves is a line that imports twice.

Three CAMT-specific readings, each of which would be a money bug if taken the
other way, are settled in `bank_camt.rs` and tested against the golden files:
a **reversal** (`RvslInd`) turns a credit into money leaving; the
**counterparty** is the debtor on money in and the creditor on money out (a
file states both roles, and one of them is always the account holder — so the
other role is read only as a fallback, only when the statement names the
account holder, and never when the fallback *is* them: banks disagree about
whether a reversal restates the original roles or swaps them, and the tenant's
own name on a reconciliation screen looks like data and is not); and a
**batched entry** (several `TxDtls` under one `Ntry`) is one line at the
entry's total with no counterparty, because the bank moved money once and
naming one of the several would be a false statement on the screen where a
human decides what a payment was. Entries the file marks as not yet booked are
counted in the import report and never staged.

The slice is store-deep and has **no HTTP route and no screen** — those are
B4.08c and B4.13. Nothing here posts to the journal, by design.

### As built: the second parser (B4.08b)

MT940 lands in `bank_mt940.rs` behind `import_bank_mt940`, and **nothing below
the parser changed** — same validation, same duplicate rules, same import
report, same tables. That is the contract's first real test, and
`bank_import_tenancy.rs` states it as one: the German January exists as both a
CAMT.053 and an MT940, and importing both stages **four lines and four
duplicates**, not eight lines. A bookkeeper who downloads a month in each format
does not book it twice.

Five readings are settled here, each of which would be a money bug taken the
other way.

**The dates are in the opposite order to how a person says them.** `:61:` opens
with the *value* date and only then, optionally, the *entry* date. The entry
date is the day the bank posted the transaction and is therefore `booked_on`;
the value date is `value_on`. An entry date states no year, so it takes the year
that puts it **nearest its own value date** — `:61:2601011231…` is booked on 31
December of the old year, not eleven months later. A two-digit year is 20xx:
MT940 states no century and has been the SEPA-era file format throughout this
one.

**The sign.** `C` in, `D` out, and `RC`/`RD` for the reversal of either, which
turns the direction around exactly as CAMT's `RvslInd` does.

**The counterparty comes out of `:86:` or not at all**, because the standard has
no field for one. German banks write `?`-coded subfields (`?32`/`?33` the name,
`?31` the account, `?20`–`?29` the remittance, `?00` the posting text); other
banks write free text, which is the whole remittance and no counterparty. A
blank field is the honest answer, and `BankStatement.source` is what tells a
reader which silence they are looking at — the reason that column exists.

**The `?2n` chunks are one string, joined with nothing at all.** They are
27-character slices of what the payer typed and banks split them mid-word; the
empty join reconstructs the original, *including an invoice number split across
two chunks* — which is precisely the string B4.09 will search for. Joining with
a space would break that number in half for ever. Free text is the opposite
case: there the line breaks are the bank's own width and read as spaces.

**A paged statement is one statement.** `:62M:` says "more to come" and the next
page reopens with `:60M:`; the period is the first opening balance to the last
closing one. A file that closes `:62F:` and then opens another `:20:`, or a page
naming a different account, is two statements and is refused whole — the same
answer a multi-`Stmt` CAMT gets, for the same reason.

Three smaller decisions: SWIFT's `{1:}{2:}{4: … -}` transport blocks are
stripped when present and anything above the first tag is dropped (a bank's
covering text is not a transaction); the file is read as UTF-8 and, failing
that, as Windows-1252, sharing `csv_read`'s decoder, because MT940's own
character set has no umlauts and German banks write them anyway; and the period
is **widened to hold every line it stages**, since `:60F:` often carries the day
the previous statement closed.

**The one limitation, and it is a real one:** `:25:` must state an **IBAN**.
Every `/`-separated part is offered to `crate::iban` with and without an
appended currency code, which covers the four SEPA-era spellings, but a
pre-SEPA domestic file (`Bankleitzahl/Kontonummer`) is refused with a message
that says to ask the bank for the SEPA format. `bank_lines` are keyed to the
account they moved on, so importing one under a guess would file a month against
the wrong account. If real files force it, the fix belongs at B4.08c's upload
route — which already has to ask a person things — as an account the uploader
names, never as a guess in the parser.

### As built: the third parser, and the door (B4.08c)

CAMT.053 and MT940 are specifications; a CSV export is not a format at all, so
`bank_csv.rs` reads what a **person confirmed** rather than deciding alone. Two
questions no export answers about itself are asked rather than guessed, and both
would be money bugs taken the other way.

**`03/04/2026` is two different days.** The order is inferred from the file as a
whole — one row with a day past the twelfth settles it for every row, and a dot
separator settles it outright, since no month-first locale writes `03.04.2026`.
A file whose whole date column stays ambiguous is **refused** with the words
that say to state the order (`?dates=dmy`), and a file that disagrees with
itself is refused too. An ISO date in a day-first file is still read as ISO: a
four-digit first component cannot be a day.

**`1.234` is a thousand or one and a bit.** `money_text`'s refusal is inherited
whole; `?decimal=comma|dot` makes it exact by rewriting the number in the one
convention that cannot be misread before it is parsed. Three shapes of sign are
taken, because all three are in the wild: one signed column (including the
German trailing minus and the accountant's parentheses), a debit and a credit
column, or an amount plus a `S/H`, `D/C`, `Af/Bij` indicator. What comes out is
signed integer cents, decided once.

**The mapping is guessed and then corrected, never guessed and applied.**
`BankCsvMapping::infer` reads the header in English, German, French and Dutch;
the preview shows what it produced; the commit carries the mapping back so a
corrected column is never silently re-guessed. A mapping naming a column the
file has not got is a `422` before a row is read, and so is a mapping with no
date or no amount in it.

**A CSV names no account, so the caller must.** `?account=` is required for a
CSV — `bank_lines` are keyed to the account they moved on — and for the other
two formats it becomes a **guard**: a file that names a different account is
refused rather than filed under this one, which is the ordinary mistake (right
screen, wrong download) that otherwise survives for weeks.

**One door, three formats.** `POST /finance/imports/bank` sniffs which parser a
file wants (`<` is CAMT, a `:nn:` tag or a `{1:` block is MT940, anything else
is a CSV) unless `?format=` says. `POST /finance/imports/bank/preview` is the
same reading with nothing after it: `read_bank_file` is a **pure function**, so
"the preview writes nothing" is not a rule somebody keeps but a thing that has
no way to happen. It joins `READ_ONLY_POSTS` for that reason. The commit's audit
line is `finance.import.bank`.

**Nothing is imported halfway.** A row that cannot be read is a `RowError`
naming its line and the rule — never the row's content, which is the tenant's
own money — and one of them means the file stages nothing at all, answered as a
`422` carrying the whole report. The one row skipped rather than refused is the
row blank in *every mapped column*: a running-balance footer is not a
transaction and not a mistake either. It is still counted, because a person told
"3 of 4 rows" must be able to find the fourth.

The report also carries a **sample** of at most fifty transactions, not the
file: the counts are exact, and a year of a busy account is read through `GET
/finance/bank/lines`, which lands here with `GET /finance/bank/statements`.

**Golden files.** `csv_de_january.csv` is the same January as
`camt053_de_january.xml` and `mt940_de_january.sta`, transaction for
transaction, in Windows-1252 with semicolons, dotted dates and comma decimals —
so the store de-duplicates all three against each other, and importing the month
twice in two formats stages four lines and four duplicates, not eight.
`csv_uk_february.csv` is the other half of the world (ISO dates, dot decimals,
paid-out/paid-in columns, a footer row that is skipped), and
`csv_broken_rows.csv` is what a person actually uploads on a Tuesday.

Cut from the slice, and named: **no saved mappings** (the mapping travels with
the upload; remembering one per tenant is worth doing once real files have shown
which repeat), **no `account` column** in the mapping (the uploader states the
account, which is the same answer for every row), and **no screen** — B4.13b is
the reconciliation UI and this is the route it will call.

### As built: the exact stage (B4.09a)

`bank_matches` exists as written above, with three differences.

**It carries `payment_id` and `entry_id`.** The design's row records *what* a
line was; those two record *what confirming it did* — the payment created and
the settlement posted — because unmatching (B4.09c) has to remove one and
reverse the other, and looking them up by their source keys would mean trusting
that nothing else ever posted against the same document. Both are `NULL` for a
kind that produces neither, and a CHECK requires them exactly for `'invoice'`.

**One line, one match**, as `UNIQUE (tenant_id, line_id)`. It is the invariant
`bank_lines.status` projects: a line is `matched` exactly when a row here names
it. Splitting one transfer across three invoices drops this unique and keeps
`amount_cents`, which is why the amount is stored per match rather than read off
the line.

**`target_kind` is `'invoice'` and nothing else, yet.** A supplier's number is
free text (B1.24) and an expense has no number at all, so neither can be matched
by the rule the exact stage *is*: "our own number, printed by us, unambiguous
since B1.08". Bills and expenses arrive as kinds, not as columns.

**The rule.** `bank_match.rs` is pure — no clock, no tenant, no query. It reads
the document numbers out of a remittance and decides four things at once: the
number is quoted, the money arrives (a debit never settles a receivable), the
currency is the document's, and the amount equals **what the document still
owes** rather than its gross — so the second instalment of a part-paid invoice
matches exactly and its gross no longer does. The window is the issue date to
two years after it: the year is in the number already, so the window is belt and
braces, and it is generous because an invoice paid four hundred days late is a
real event.

Two readings inside the extractor would each be a money bug taken the other way.
A **run of digits is read whole or not at all**, so `INV-2026-000078` is a
different counter and never ours with a digit stuck on. **Letters on either side
are not a boundary**, because MT940 joins its `?2n` chunks with nothing and the
number arrives welded to the words around it; what keeps that safe is not
punctuation but the conjunction of the four facts — and a person still confirms.

**Confirming is one transaction**, and it is the first thing in alo that books
anything from a request. It re-derives the exact rule **under the row locks** of
the line and the invoice (a suggestion a client sends back is not evidence, and
a colleague may have keyed the same money in meanwhile), records the payment
dated the day the *bank* booked it, and posts the settlement. Where the invoice
was never booked — which is every invoice today, since nothing else calls
`post_invoice_issue` — it books the issue too, at the document's own issue date:
relieving a receivable that was never there would leave the customer's ledger
negative and every aged-debtors report wrong. `ConfirmedMatch.invoice_booked_now`
says when that happened. The one thing it will not do is invent a chart: a
tenant with no `ar` account is refused, naming the role and the screen.

Making that transaction possible added in-transaction forms of three doors that
already existed — `post_fin_entry_in`, `record_billing_payment_in`,
`billing_payments_on` — each of which is the public door minus its `BEGIN` and
`COMMIT`, so no rule is stated twice.

Cut from the slice, and named: **no unmatch, no ignore, no manual pick** (all
B4.09c), **no learned rules** (B4.09b — hence a `rule_id` that is always
`NULL`), **no split across documents**, **no bills or expenses**, and **no HTTP
route and no screen**: B4.13b is the reconciliation UI and this is what it will
call.

### As built: the heuristic stage and the rules (B4.09b)

`bank_match_heuristic.rs` ranks what the exact stage cannot claim, and
`fin_match_rules.rs` holds what a tenant has taught it. `bank_suggest.rs` is the
read that feeds both — split out of `bank_reconcile.rs` in the same change,
because suggesting and confirming had become two responsibilities in one file.

**A score is a sum of named evidence, never a percentage.** Each
`MatchEvidence` carries its own points and its own sentence for the screen:
the payer quoted our number but paid part of it (60), a rule the tenant saved
points at this customer (45), the counterparty *is* the customer word for word
(35) or resembles them (20), the line moves exactly what is owed (30), it is the
only open document owing exactly that (15), it was booked around the due date
(10 within a week, 5 within a month). `SCORE_MIN` is **45**, which is precisely
what the weakest *identifying* combination — the amount fits, and fits nothing
else — is worth. That is the precision argument, and a test states it: **no soft
signal reaches the floor alone.** A name that merely resembles, an amount that
merely fits, a payment that merely arrived near its due date: none of them is a
suggestion.

**Three readings are settled here, each of which would be a money bug taken the
other way.** *Uniqueness is a claim about the ledger*, so when the read had to
cap the open documents it looked at (`OPEN_LEDGER_MAX`), the stage stops making
it and the answer says `ledger_capped`. *More than is owed is never offered*: a
line moving a cent more than the document owes is a split, a duplicate or a
mistake, and attributing it would record a payment larger than the debt.
*Whatever the exact stage claims is not offered again as a guess*, so the screen
never argues with itself.

**The name fold is base letters, not transliterations** — `Müller` folds to
`muller`, and a bank writing `MUELLER` therefore does not match it. Undoing the
German transliteration would also turn `Bauer` into `Bar`, and a signal that
manufactures resemblances is worse than one that misses some. That miss is
exactly what a rule is for: the tenant says once that this counterparty (or this
IBAN, or this fragment of a remittance) is that customer, and every later
statement recognises them. Rules are **plain folded text in one named field** —
no globs, no regular expressions, which a tenant could otherwise use to write a
denial of service — stored folded so the unique on `(tenant, match_on, pattern)`
is the real "one rule per thing to look at". `learn_fin_match_rule` takes the
pattern off the line a person is looking at; it **refuses the remittance**,
because what a payer wrote on one transfer names that transfer and would never
match again. Hits are counted by a confirmation, never by a read, and never
change what a rule scores: a heuristic that quietly re-weights itself is one
nobody can predict.

Cut from this slice, and named: **`account_id` and `supplier_key`** on the rules
table (they belong with the `bill` target kind B5 brings — nullable columns
added additively then, rather than dead schema now); **confirming a heuristic
suggestion**, which is B4.09c's manual pick and the caller that will write
`rule_id` and call `fin_match_rule_hit`; and, as before, **no HTTP route and no
screen**.

### As built: the manual stage, the undo, and the routes (B4.09c)

Three files, one verb each — `bank_manual.rs` (the pick), `bank_unmatch.rs`
(taking it back), `bank_ignore.rs` (the line that is nobody's) — plus
`finance_bank_match.rs`, the first HTTP surface reconciliation has had. The
settling transaction itself moved into `bank_reconcile.rs`'s
`settle_bank_line`: the exact stage and the manual one differ in **exactly one
thing**, the rule re-run under the row locks, so that rule is an argument
(a plain `fn` pointer) and everything after it — the locks, the issue booked if
it is not in the books, the payment, the settlement, the row, the line's status
— is stated once.

**The manual stage refuses less, on purpose.** A pick is not a guess, so the
date window does not apply to it: `ensure_matchable` (the window) split into
`ensure_settleable` (the states, the direction, the currency) plus the window,
and the manual rule takes the first. The exact stage's own refusal already says
"match it by hand if it really is its payment", and a pick bound by the same
window would take that sentence back. Money that arrived before the document was
issued is likewise allowed — a deposit taken in advance is real, and B1.19 has
always allowed it. What the manual stage adds is two rules of its own: **never
more than the document owes** (the heuristic stage's reading, for the same
reason — a payment larger than the debt is a split, a duplicate or a mistake)
and **the whole line or nothing**, because `bank_matches` is still unique per
line and attributing part of a transfer would mark it settled with the rest
attributed to nobody. The amount the client sends is therefore **compared, never
trusted**: it is what the person saw on the screen they clicked, so a stale
screen is a `422` instead of a payment for the wrong money.

**Unmatching is asymmetric, and that is the design.** The entry is *reversed*
(`fin_journal::reversal_entry`, the first reversal alo posts: the same postings
with both money columns negated, the same dimensions, the same date, the same
rate snapshot, `reverses_entry_id` set) and the payment is *deleted*. The
journal records the books, where a correction is an event with a date; the
payment table records money received, and money that was never received has no
event to record — the view B1.19 took when it made deletion a payment's only
correction. The invoice's **issue entry stays**: the document is still issued
and still owed.

**Only the newest payment on a document can be taken back**, and this is the one
place B4.09c is stricter than the design note. A settlement's receivable relief
is cumulative (`payment_settle_entry` telescopes prefixes so a settled document
lands on exactly zero in both columns); removing a payment from the middle of
that sequence would leave every later entry standing on a prefix that is gone,
and for a foreign-currency document a base-column residue no document explains.
So a match with a later payment on the same document refuses, naming what to do.
One extra click in the rare case, against a class of unexplainable ledger
residue in the common one. *Rejected: applying the rule only to foreign-currency
documents* — a rule that holds sometimes is one nobody can predict, and the
sometimes is exactly the case a tenant meets once a year.

**Ignoring costs one column.** `bank_lines.ignored_reason` (migration 0145),
with a CHECK making the reason and the status move together, so un-ignoring
cannot leave a stale sentence behind. A blank reason is refused: "ignored" with
nothing beside it is the state a bookkeeper cannot audit or hand over. Who and
when are **not** stored — the audit middleware already writes an entry naming the
actor, the act and the line for every mutating `/finance/*` route (B2.13), and a
second answer to a question that has one is how two answers start disagreeing.
Saying it again with a corrected sentence is allowed; dismissing a *matched* line
is not (take the match back first).

**The routes**, all under the existing `/finance` prefix (no new prefix, so
nothing for the Caddyfile): `GET /finance/bank/suggestions?statement=` (the read
the screen is drawn from — the bulk read, because a per-line route would fold the
ledger once per line), and `POST /finance/bank/lines/{id}/match` · `/unmatch` ·
`/ignore` · `/unignore`. Four named acts rather than a settable `status`: each
has different consequences, and the audit log records them by name. Evidence
travels as a **token and its numbers** (`{"kind":"partPayment","remainingCents":
80700}`), never as a sentence — the sentence is the screen's, in the reader's own
language.

Cut from this slice, and named: **no split across documents** (still the change
that drops `UNIQUE (tenant_id, line_id)`); **no bills or expenses** as match
targets (B5 brings the kind); **no period rule** on the reversal's date (B4.10
locks periods, and this is where "reverse today instead" will have to be
decided); and **no screen** — B4.13b is the reconciliation UI and these are the
routes it calls.

## Fiscal periods and the soft close (B4.10)

```
fin_periods   tenant_id, id PK, from_date, to_date, status 'open'|'closed',
              closed_by, closed_at, note
              — plus the derived tenant-level lock date: max(to_date) of closed
```

**Posting with `entry_date` on or before the lock date is a typed error, and
so is the document action that would have caused it.** Issuing an invoice
dated in a closed quarter is refused at `issue`, naming the lock date, not
half-completed with a floating entry. A close is **soft**: an admin may
reopen a period, the reopen is audited with the reason, and every entry
posted afterwards is visible as such (its `created_at` is after the close it
follows). *Rejected: a hard close* — a small business finds a missing receipt
in week three of every quarter, and a lock nobody can lift is a lock people
work around by backdating the next period, which is worse.

*Rejected: deriving the lock from a single `lock_before` date in settings.*
Named periods are what the reports offer as a period picker and what an
accountant asks for by name ("is Q2 closed?"); a bare date answers the
posting question and none of the others.

**As built (B4.10).** Migration 0146 and `fin_periods.rs`, with four decisions
the sketch above left open:

- **Closed periods are a contiguous prefix, and that is enforced.** Because the
  lock date is `max(to_date)`, closing Q3 while Q2 was still open would shut Q2
  too — by arithmetic rather than by anybody's decision. So a close refuses
  while an earlier period is open (`close the periods in order`), a reopen
  refuses while a later one is closed (`reopen the periods newest first`), and
  a period cannot be *defined* wholly inside shut books. Together they make
  "the books are closed through X" literally true, which is the sentence every
  refusal says out loud.
- **One `note` column holds the note of the current state** — what the closer
  said, or (after a reopen) why it was reopened. A period does not accumulate a
  history of notes; the audit log is that history, and it already records
  `finance.period.close` and `finance.period.reopen` with the actor. The reopen
  reason is **required**, on the same reasoning as the bank line's dismissal
  (B4.09c): a period that was reported and is open again is the one state an
  accountant must be able to explain six months later.
- **Who closed it and when are the period's own state**, unlike the ignore
  reason where only the sentence is a column. "Is Q2 closed, and by whom?" is a
  question about the period, answerable without reading a log.
- **The refusal is one sentence in one place** (`ClosedThrough::refusal`),
  raised by `post_fin_entry_in` inside the caller's transaction — so the
  *document act* that would have posted (issuing an invoice into a closed
  quarter, confirming a bank match against it) is refused whole. It names the
  period, the day the books are closed through and the day they were closed. In
  particular it is what a bank **unmatch** now meets: the reversal is dated the
  original's date on purpose, so when that period is closed the correction is
  refused rather than silently re-dated into an open one (the open question left
  by B4.09c).

**Not promised:** a posting already in flight when a close commits still lands.
The journal reads the lock date inside its own transaction; serialising every
posting against every close would make the books' hot path queue behind an act
taken four times a year. The close is a rule about writes that start after it,
and `created_at` tells that story honestly. **Not built here:** deleting a
period defined by mistake — the shape of that (what happens to a closed one, to
one that has been reported) is a decision, not an omission, and no screen needs
it before B4.13c.

## The four reports (B4.11)

All four read the **journal**, and nothing else. That is the point of having
one: a P&L that read invoices, a balance sheet that read bank lines and a VAT
return that read documents would be three systems that agree until the first
manual entry.

- **P&L** — income and expense accounts, grouped by type then code, for a
  period, with the comparative period beside it.

  **As built (B4.11a).** `fin_pl.rs` holds no query at all: it asks
  `fin_trial_balance` twice — the period, then the comparative — and folds the
  two account types that make a result. Three decisions worth not relitigating.
  The **signs are flipped once**, in `natural_cents`: the ledger keeps income
  negative, a P&L shows revenue and cost as positive amounts, and the result is
  one subtraction. The **comparative is derived, not asked for** — the period of
  the same length ending the day before, so a quarter compares with the ninety-
  odd days before it and a year with the year before it; *rejected: a second
  pair of dates on the request*, which every caller would compute and three of
  them would compute differently. And **a line appears when either period moved
  it**, so an account that earned ten thousand last quarter and nothing this one
  is on the page at zero rather than missing — with `postings: 0` saying the
  zero is real. `GET /finance/reports/pl` and its `.csv` twin are **admin only**
  (a P&L is the whole tenant's result, not the reader's own work); B4.12 widens
  that gate additively to the accountant role.
- **Balance sheet** — asset, liability and equity balances at a date, plus
  the period's result; it must balance, and P10 says so.

  **As built (B4.11b).** `fin_balance.rs` is the same shape as the P&L and holds
  no query either: one `fin_trial_balance(None, Some(on))` — no lower bound,
  because a balance sheet is cumulative by definition — split into the three
  types that stand on it, with the other two folded into the result in the same
  pass. Four decisions worth not relitigating. **One date, not a period**: `?on`
  and no `?from`, because "what was in the bank between March and June" is a
  ledger question and answering it here would produce a sheet that does not
  balance. **The result sits beside equity, not inside it** — alo writes no
  year-end closing entry (a close is a rule about *writes*), so income less
  expense to the date is carried as its own figure; that is what makes
  `assets = liabilities + equity + result` hold, and an accountant who wants it
  inside equity books the entry, after which it is inside equity here too. **The
  sheet says whether it balances**: `differenceCents` and `balances` are on the
  wire and `difference` is a row of the file — always zero on books written
  through `post_fin_entry`, and stated rather than assumed because the figure a
  broken sheet prints looks exactly like a correct one. And *rejected: a
  comparative column* — a balance sheet's comparative is the **previous
  financial year end**, a fact about the tenant's fiscal calendar rather than
  about the date asked for, and "the same day a year earlier" would be a guess
  printed under a heading nobody chose; a caller who wants two dates asks twice.
  Signs flip once, in that file's own `natural_cents` (assets and expenses
  debit-positive, the other three credit-positive), and every line carries its
  `role` so a screen can tell the bank from the receivables without reading
  codes it does not own. Admin only, on the same gate as the P&L.

  The **HTTP surface split** with this item: `finance_reports.rs` now holds only
  what all four reports share (the `PeriodQuery`/`OnQuery` date parsing, the
  `admin` gate, the spreadsheet-safe `text`), and each report is its own file
  (`finance_report_pl.rs`, `finance_report_balance.rs`). A column added to one
  report is not a change to another, and B4.11c/d add a file each rather than
  growing a shared one.
- **Aged receivables/payables** (B4.11c, as built) — the one report that reads
  **documents** (`billing_invoices` + `billing_payments`, `billing_bills`),
  bucketed current / 1–30 / 31–60 / 61–90 / 90+ from the due date. It reads
  documents because ageing is a property of a document, not of an account
  balance — a receivable account holds one number, and only the invoices behind
  it know which part of it has been owed since March. *Rejected: an open-item
  sub-ledger inside the journal* — it duplicates B1's payment state machine, and
  two implementations of "what is still owed" drift.

  What the built report settles, beyond the bands. **The day is a boundary in
  both directions**: a document counts when it was issued on or before `on`, and
  money against it counts when it *arrived* on or before `on` (`paid_on`, not the
  day it was keyed in) — so re-running last quarter answers last quarter.
  **Only documents that stand are read**: `issued` and `paid` on the receivable
  side (a document settled today may well have been owed on the date asked for),
  `approved` on the payable side — a received-but-undecided bill is an intention,
  which is the line this note already draws for the journal. **Credit notes are
  included, negatively**, in the counterparty's own group, and each row says
  whether it is one. **A document with nothing open is not a row**; an
  **over**paid one is, negatively, because money held for a customer is a fact a
  bookkeeper needs here. **The bands are added in the accounting currency**, each
  document crossed at the rate frozen on it (`billing_fx::restated_open_cents`,
  the scalar sibling of `restated_into` — a document already in the books'
  currency needs no snapshot, which is also how a bill, written by somebody
  else's system, is added at all); anything that cannot be crossed honestly is in
  **no** band and is counted in `unconvertedCount`, exactly as the VAT summary
  does. Every document also carries its own currency and its own open amount, so
  the paperwork behind a converted figure stays readable. A **bill that states no
  due date** (BT-9 is optional) is payable on receipt, so it ages from its issue
  date. Grouping is by customer id on the receivable side and by the supplier's
  comparable key (`Supplier::key`) on the payable one, because a bill copies its
  supplier rather than linking to a record.

  **P6 is not yet asserted**, and deliberately: the tie between these totals and
  the ledger's `ar`/`ap` balances can only be tested once issuing a document
  books it, which is not wired (see `docs/autonomy/STATE.md`). Nothing posts to
  `ap` at all today, so the payable side has no ledger counterpart to tie to yet.
  A bill also has no payment rows of its own — a SEPA export is an instruction,
  not a payment — so a payable stays open until reconciliation can settle it.
  **B6.12b proved on the wire that reconciliation cannot** (`docs/autonomy/STATE.md`):
  `POST /finance/bank/lines/{id}/match` takes an `invoiceId` and refuses a
  money-out line outright ("an invoice is settled by money arriving"), and bills
  carry no other settling verb — so an approved, SEPA-exported, actually-paid
  bill ages forever, and the payable side is a list a tenant can only add to.
  **Flagged for a human beside the posting gap above**: a bill needs a
  settlement of its own *and* an `ap` posting rule; neither is a rule that is
  wrong, both are calls that do not exist.
- **VAT return figures** — output VAT per rate, input VAT per rate,
  the net payable, from postings carrying `vat_rate_bp`. It reconciles
  against `billing_vat_report` (B1.20), which reads documents; the two are
  compared in a test on the seeded year, because "a chart and a tax return
  cannot disagree" (BI1.01) applies hardest where the tax return is literal.

  **As built (B4.11d).** `fin_vat_return.rs` holds no query either: it is four
  `fin_dimension_balances` reads grouped by `LedgerDimension::VatRate` — the tax
  by `Role(VatOutput)` and `Role(VatInput)`, the **taxable base** by
  `Type(Income)` and `Type(Expense)`, which is the one scope this item added to
  `fin_ledger` (a tenant's own expense accounts carry no role to name them by,
  and B4.04a's rule that *the rate travels on the revenue posting too* is what
  makes the base readable from the journal at all). Four decisions worth not
  relitigating. The **signs flip once**, in that file's own `natural_cents`: the
  output side is credit-positive and the input side is not, so a return is two
  positive columns and one subtraction. **Only postings that state a rate are on
  the return** — a rate is what makes a posting a taxable base — but what states
  none is *reported* rather than dropped (`unratedBaseCents`, and
  `unratedVatCents` for tax on a VAT account with no rate, which is a posting
  rule with a bug); a return whose base is far below the period's turnover is a
  fact the filer has to see. **A period whose rates cannot all be read is
  refused**, not half-summed: `LEDGER_GROUPS_MAX` caps a grouped read, and a
  legal document summed from part of a period would be a plausible wrong number
  — unreachable from books alo writes, and a `422` if it ever is. And *rejected:
  a comparative column*, for the balance sheet's reason — a VAT period compares
  against the same period of the fiscal calendar, which is a fact about the
  tenant rather than about the dates asked for.

  **The reconciliation is asserted**, in `tests/fin_vat_return.rs`: over
  documents raised through the billing store, issued through the gapless
  sequence and booked through the real rules, the journal's output side equals
  `billing_vat_period`'s base rows rate for rate and cent for cent. The two can
  only differ if something was billed and not booked or booked and not billed.
  `/finance/reports/vat` and its `.csv` twin are admin only, on the same gate as
  the other three, and the file is named `vat-return-…` so it does not overwrite
  `/billing/reports/vat.csv`'s `vat-…` for the same quarter in a downloads
  folder.

CSV follows `billing_reports` exactly: ISO dates, `.` decimals, untranslated
column headers (a file read by a spreadsheet and an accountant's tooling must
not move with the reader's locale), amounts in units with two decimals, and
no personal data beyond the counterparty name a document already prints.

Two things B4.11a settled about that file for all four reports. The headers it
is served under (`attachment`, `nosniff`, `no-store`, a stated charset) moved
into `csv::attachment`, so every export in alo carries them and a header added
for one cannot be missing from another. And **user-authored text is neutralised
where it is chosen**, not in the CSV writer: an account a tenant named `=cmd|…`
is written with a leading apostrophe, while a negative amount keeps its `-` and
stays a number. The P&L is the first alo export to carry text a user wrote.

**These are figures for a return, not a return.** ADR 0035's non-goal is
unchanged: alo produces correct, exportable numbers; filing goes through the
national portal or a partner. The screen says so.

## The accountant role (B4.12) — alo's first scoped role

Until now alo has had two access facts: `users.is_admin` (a tenant admin) and
Spaces membership with a `SpaceRole` (ADR 0026/0028, which governs Drive and
what attaches to a Space). B4.12 adds the third, and it is deliberately the
narrowest thing that solves the actual problem: **an external accountant
needs the books and must not have the mail.**

```
tenant_user_roles   tenant_id, user_id, role PK   — role ∈ {'accountant'}
                    granted_by, granted_at        — who handed it out, and when
```

- Finance routes accept **admin or accountant**. Read of everything in
  `/finance/*`; write of manual entries, matches, expense decisions and the
  period lock.
- Billing and CRM are **read-only** for an accountant (they must see the
  document behind a posting); Projects, Tasks, Drive, Insights and Mail are
  unchanged — which is to say an accountant, being a user with no shares and
  no Space memberships, already sees only their own empty mailbox and no
  files. The tests prove that literally: an accountant handle gets `403` on
  every admin route, `404` on another user's expense, and nothing but their
  own on every account-door read.

### As built

**The grant.** `POST /admin/users/roles` `{userId, role, granted}`, admin-only,
audited as `user.role`. Its own route rather than a field beside `isAdmin`,
because a body that could set both would make "make them an accountant" and
"make them an admin" one call. Granting and revoking are both idempotent — the
caller's intent is a state, not an event — and a grant proves tenant membership
before it writes (`users.id` is globally unique, so the naive `INSERT` would
have made another tenant's user an accountant here; the refusal is `404`, the
same answer an id that was never issued gets). `GET /admin/users` carries
`roles` per row, read for the whole company in one query, and the session
resource advertises `alo:roles` beside `alo:isAdmin` so a client can show what
the role opens — the server refuses regardless, because a client is never an
access decision.

**The finance gate** is one function, `Account::require_finance` — admin **or**
accountant — and it replaced `require_admin` in exactly three places: the four
reports (through `finance_reports::reader`, the single gate all eight report
routes already shared), the expense approvals inbox and its three decisions,
and the fiscal periods' define/close/reopen. **The mileage rate table kept
`require_admin`**, deliberately: what a company pays per kilometre is a pay
decision, not a bookkeeping one, and it is not on the list of accountant writes
above.

**The billing/CRM read-only rule is a layer**, `scoped_roles::enforce_scoped_
roles`, mounted over the router beside the audit trail (B2.13) and for the same
reason: sixty handlers with a gate each are sixty chances to forget, and the
sixty-first is the hole. It refuses a mutating method on `/billing/*` or
`/crm/*` to a caller who holds the role and is not an admin, short circuits
before touching the store for everything else (including every `GET`), passes a
tokenless request through so the handler still answers its own `401`, and lets
the dry runs through — `audit_action::writes_nothing`, one list shared with the
audit layer, so a preview is a read to both.

**The one extra query is not extra.** `authenticate` runs on every request in
the product, so the roles are read *with* the admin flag in a single
`access_facts` call rather than beside it. A delegated handle (ADR 0017)
carries no roles for the same reason it carries no admin flag: the grant is
about one mailbox, and the roles belong to the person who signed in.

**The web surface** is the admin console's user modal — a named checkbox with
the whole rule written beside it, not a bare switch, because an access grant is
the one control where "what does this do?" must be answerable without trying it
— plus a badge in the user list. The finance screens themselves are B4.13.

*Rejected: modelling finance as a Space.* A Space is a container with
members, and the ledger is not in a container — it is the tenant. Making it
one would mean "who can see the books" and "who can see this folder" are the
same question answered by the same table, and the first tenant who removes an
accountant from a Space to tidy a sidebar would silently revoke their access
to the year-end.

**This contradicts a sentence written three times in `ROADMAP.md`, and the
contradiction is deliberate rather than overlooked.** B2.11, B3.8 and BI-1.6
each defer their access question to B4.12 with the words "designed **on
Spaces** rather than invented twice". That phrasing was written before
anybody had looked at what the first scoped role would actually be; the role
that turned up is tenant-wide and cross-module, which is the one shape a
Space cannot express. The queue item itself hedges — "via Spaces/roles". So
B4.12 delivers *roles*, and the three deferred items above are answered by
the same table when their turn comes: a per-module role is a row, and a
per-record share (which board, which engagement) is still Spaces' job and
still unbuilt. **Flagged in `docs/autonomy/STATE.md` for a human**, because
correcting three ROADMAP lines is not a loop decision.

*Rejected: a general RBAC engine (roles × permissions × resources).* We have
one role to ship. A permission matrix built for one caller is a matrix that
encodes that caller's accidents; when the second role arrives (B6's HR role
is the likely one), the table above widens by a row and the gates by a word.

**Flagged for a human:** an external accountant is still a *user*, and a user
today gets a mailbox. A no-mailbox account type is an identity change, not a
finance one, and it is named in the open questions below rather than invented
here.

## The finance agent (B4.14)

Three tools in the ADR 0034 allowlist, executed by `alo-jmap` against the
same store functions the routes use — never a parallel path — and verified
**structurally** (routes exist, guards hold, the execute path writes the
right rows against the local database). No live model call in the loop, ever.

| Tool | Kind | What it may do |
|---|---|---|
| `categorise_transactions` | **draft** | propose a category (hence an account) for the caller's own uncategorised expense claims. Writes proposals; a human answers each |
| `vat_summary` | **answer** | read `/finance/reports/vat` for a period and answer with the figures as sources. No writes |
| `flag_anomalies` | **answer** | read the journal for a period and name what looks unusual — a duplicate amount to the same counterparty, an expense far outside its category's range, a month with no rent — **with the entries as sources**. No writes, no scores, no "risk" |

### As built, B4.14a — `categorise_transactions`

Four decisions the slice made, none of them reversible by a later one without
saying so:

- **The suggestion is a different column from the decision.** `fin_expenses`
  gained `proposed_category_id` / `proposed_at` / `proposed_reason` (0150), and
  nothing — no posting rule, no report, no VAT return — reads them.
  `category_id` stays what a *person* chose. A guess written into the decided
  column would be indistinguishable from a decision the moment it landed.
  Accepting **moves** the value between the two columns.
- **The model chooses nothing.** The classification is the store's
  (`fin_categorise::plan_categorisation`, a pure function): for each
  unclassified claim, the category this person has most often agreed to for the
  same merchant, ties broken by recency, and **no suggestion at all** for a
  merchant they have never classified. The tool's arguments are a period and
  nothing else — there is no category argument to smuggle a guess through, and
  the prompt says so.
- **Only the claimant's own history is read**, which is a privacy rule and not
  a convenience: a tenant-wide merchant map built out of everybody's receipts
  would answer "who has been to that pharmacy" as a side effect. The personal
  door has no cross-user read, and this slice did not open one.
- **A "no" outlives the suggestion it was about** (`proposal_declined_at`).
  Without it the next run offers the same rejected word, and a suggestion a
  person has to decline twice is one they stop reading. A claim that already
  carries an unanswered suggestion is skipped for the same reason.

**Cut, and named:** *unmatched bank lines*. The sketch above offered them
beside the claims, but a bank line has no category — booking one to an account
is a verb that does not exist on any door (`match` settles a line against a
document; there is no "book this line to 6100"). Suggesting a category for a
thing nothing can then classify would be an offer with no accept. It belongs
with whatever item opens that verb, not here.

### As built, B4.14b — `vat_summary` and `flag_anomalies`

The two rules the third tool needed were held literally. It **names entries,
never people**: no rule reads a posting's `user_id`, so no finding can carry
one, and the tool's own description tells the model it cannot answer a question
about somebody's spending. And it **explains every flag with the rows that
caused it** — `Anomaly::sources` is the whole of the argument, and an
unexplained flag is an accusation.

Five decisions the slice made:

- **Both tools are behind the finance gate.** `require_finance` — an admin or
  the accountant — exactly as `GET /finance/reports/*` is. An agent is the
  obvious way round B4.12's wall: the proposal is composed by a model, but the
  execution is a request from a browser holding a token, and it is gated like
  every other. `categorise_transactions` needs no such gate because it reads
  only the caller's own claims; these two read the whole tenant's books.
- **`vat_summary` states its period or it does not run.** Both days are
  required, with no default — the rule `finance_reports::day` already holds for
  the report routes, and this is the figure most likely to be copied into a
  filing. It renders through `finance_report_vat::report_json`, so the agent and
  the report cannot disagree about a cent; there is no second path to a VAT
  figure in this product.
- **Three deterministic rules, no score.** `find_anomalies` is a pure function
  with no confidence, no ranking and no percentage in it, because a number
  attached to a suspicion is read as evidence for it. A duplicate is the same
  counterparty, account and **signed** amount inside a week — signed, so an
  invoice and the payment that settles it (equal, opposite, days apart on the
  same receivable) are never reported as a double booking. An unusual amount is
  measured against its own account's median in the same period, with a €100
  floor so a tenant whose median is €2 is not flagged on every lunch. A missing
  month is only ever an **interior** one: a cost that started in March is not
  eleven missing months, and one cancelled in October is not a hole in November.
- **What was not looked at is part of the answer.** `truncated` (the period
  holds more entries than one scan reads), `notComparable` (entries naming no
  counterparty, which the duplicate rule cannot compare) and `found` vs `shown`
  all travel, for the reason `categorise_transactions`' skipped list does:
  silence reads as "nothing was wrong" when it means "I stopped looking".
- **Nothing is written, and there is no dismissal.** No anomaly table, no
  "reviewed" flag, no state on a finding. A finding is a question asked of a
  period, and the answer to it is a correcting entry in the journal — a
  dismissal would be a second place the books are said to be right.

**Known limit, named:** the duplicate rule can only compare entries that name a
counterparty, and today only invoices, credit notes and payments do (they carry
`customer_id`). Nothing sets `supplier_key` yet, because bill and expense
auto-posting is not built — so on today's books the rule effectively watches the
sales side. It widens with no code change the day a bill posts, and until then
`notComparable` says how much it could not see.


## Errors

One map, `billing::map_store_err`, used and not copied — the call CRM and
Projects both made, for the same reason: it is a store-error map, not a
billing rule.

| Condition | Store | Wire |
|---|---|---|
| no or bad token | — | `401` (the `authenticate` extractor) |
| an approver-only or accountant-only action without the role | — | `403` |
| account, entry, expense, statement, line or rule not this tenant's | `NotFound` | `404` — existence is never disclosed |
| another user's expense claim through the personal door | `NotFound` | `404` — not `403`, which would confirm it exists |
| a manual entry that does not balance | `Validation` | `422`, naming the difference in both currencies |
| a posting with both amounts zero, or an unknown account, or an inactive one | `Validation` | `422` |
| a duplicate account code, or deleting an account that carries postings | `Conflict` | `409` |
| the chart is missing a role a posting rule needs | `Validation` | `422` naming the role — and the **document action is refused**, not completed |
| posting or acting into a closed period | `Conflict` | `409` naming the period and its close date |
| a document event posted twice | `Conflict` | `409` |
| editing or deleting a posted entry | — | no such route exists; the reversal is the answer |
| a malformed CAMT/MT940/CSV, or a CSV mapping that does not cover the required columns | `Validation` | `422` naming the line and the field, never a partial import |
| re-importing a file, or a line already imported | `Conflict` | `409` with the earlier import named |
| confirming a match on a line that is already matched | `Conflict` | `409` |
| an expense transition that is not allowed from the current status | `Conflict` | `409` |
| a receipt over the size cap, or a media type we do not read | `Validation` | `422` |
| database error | `Db` | `500`, opaque — the wire never sees a raw error |

Validation messages are authored in the store and name the rule and the
field; they remain the one place a message crosses in English today — the
standing cross-cutting item B1.27, B2.14 and B3.11 each left for a human, and
this wave adds no new kind of it.

## Tenancy

Every statement carries `tenant_id` from the handle, never from request input
— the invariant `for_tenant`/`for_account` make structural rather than
remembered. Three isolation tests are mandatory, and the third is this wave's
own:

- **Wrong tenant** (law 1, every wave): tenant A's handle cannot read, post,
  match, approve, lock or report on tenant B's account, entry, expense,
  statement or line. Clean denial, not data and not a `500`.
- **Wrong user** (B3's addition, inherited): user B's `AccountStore` cannot
  read, edit, submit or withdraw user A's expense claim — a `404` inside the
  same tenant.
- **Wrong aggregate** (new here): tenant A posting a whole generated month
  leaves **every one of tenant B's balances and reports byte-identical** —
  P9. A single-row read test cannot catch a `SUM` that forgot its
  `tenant_id`; a report comparison can, and reports are what this module is.

Three data-handling rules the module adds:

- **A receipt is a file with somebody's life in it.** Receipt images live in
  Drive under the claimant's own node, referenced by id; they are never
  copied into a finance table, never logged, and an approver reads them
  through the same door that shows them the claim.
- **Nothing a human typed reaches a log.** Remittance text names people;
  expense descriptions name occasions; a counterparty name is a party to a
  transaction. `tracing` spans carry ids, counts and amounts — the rule mail
  bodies have had since Phase 1, applied to the bank.
- **`finance` joins `audit_action::AUDITED_MODULES`** beside `billing`,
  `crm` and `projects` at B4.05b, after which `tests/audit_routes.rs`
  requires every mutating `/finance/*` route to be audited by reading the
  router's own source. Sub-resource events file against their parent record
  (an approval against the expense, a match against the bank line), the rule
  B2.13 established. "Who closed the period, and when" is a question an
  auditor will ask.

## Files this wave will add

Store (`platform/alo-store/src`), one file one reason:

```
fin_accounts.rs      the chart, the roles, the default seed
fin_journal.rs       post(), the balance check, reads of the journal
fin_rules.rs         document → NewEntry, pure, one function per document type
fin_booking.rs       read the document, resolve the roles, post what the rule
                     returns (added at B4.04a: the rules stay pure and the
                     journal stays ignorant of invoices)
fin_categories.rs    expense categories and their accounts
fin_expenses.rs      claims: the account door, the approval transitions
fin_mileage.rs       the rate table and the claim it becomes
fin_receipt.rs       the extractor trait + the deterministic implementation
fin_receipt_read.rs  the tenant-scoped step: a Drive node → its text → the
                     candidates (added at B4.06b; fin_receipt.rs stays a pure
                     function from characters to guesses, with no door on it)
bank_camt.rs         CAMT.053 → ParsedStatement (over billing_xml_tree)
bank_mt940.rs        MT940 → ParsedStatement
bank_csv.rs          CSV + mapping → ParsedStatement (over csv_read)
bank_read.rs         the one upload door: sniff the format, read the file,
                     stage it (added at B4.08c; bank_import.rs stays the
                     staging and the storage, with no format in it)
bank_import.rs       staging, dedupe, the import report
bank_match.rs        the exact rule, pure (B4.09a, as built)
bank_match_heuristic.rs  the ranked stage and its evidence, pure (B4.09b)
bank_suggest.rs      the read that folds both stages over the ledger (B4.09b)
bank_reconcile.rs    the settling transaction, and the exact stage's door into
                     it (B4.09a/c)
bank_manual.rs       the pick a person makes: the rule, pure, and the verb
                     (B4.09c)
bank_unmatch.rs      taking a match back: the reversal, the payment, the line
                     (B4.09c)
bank_ignore.rs       the line that is not ours to book, and its reason (B4.09c)
fin_match_rules.rs   the per-tenant saved rules (named `fin_rules_learn.rs`
                     when this was written; `fin_rules.rs` was already the
                     posting rules, and the table's own name is the clearer one)
fin_periods.rs       periods and the soft close
fin_pl.rs            the P&L (as built; `fin_reports.rs` when this was
                     written — one file per report reads better than one
                     file for four)
fin_balance.rs       the balance sheet, at a date
fin_aged.rs          aged receivables/payables (the one report over documents)
migrations/0129…     fin_accounts + fin_seeds; fin_entries + fin_postings;
                     fin_categories + fin_expenses; fin_mileage(+rates);
                     bank_statements + bank_lines; bank_matches +
                     fin_match_rules; fin_periods; tenant_user_roles
```

Migration numbers are taken in order at the moment each is written — the
sites track pushes to the same branch and shares the sequence (0127 went to
sites between two of B3's), so "0129 onward" is the starting point, not a
reservation.

Routes (`products/mail/alo-jmap/src`): `finance.rs` (the module's edge
concerns and the error map reuse), `finance_accounts.rs`, `finance_entries.rs`,
`finance_expenses.rs`, `finance_receipts.rs`, `finance_mileage.rs`,
`finance_bank.rs`,
`finance_match.rs`, `finance_periods.rs`, `finance_reports.rs` (what the four
reports' doors share) with `finance_report_pl.rs` and
`finance_report_balance.rs` beside it, plus
`agent_finance.rs` (B4.14) and the additive lines in `server.rs`, `lib.rs`
and `audit_action.rs`.

Web (`web/src/finance`): `FinanceModule.tsx`, `api.ts`, `types.ts`,
`format.ts`, `ExpensesView.tsx`, `ExpenseDialog.tsx`, `ReceiptDialog.tsx`,
`BankView.tsx`, `ImportDialog.tsx`, `MatchPanel.tsx`, `AccountsView.tsx`,
`JournalView.tsx`, `EntryDialog.tsx`, `ReportsView.tsx`, `index.ts`; the
`finance*` block in `i18n/en.ts`, `fr.ts` and `nl.ts`; the module entry in
`product/workplace.tsx`; `/finance` in `vite.config.ts`.

## Out of scope for B4 (cuts are decisions)

- **Payroll calculation** — ADR 0035's explicit non-goal. B6.10 exports the
  columns a payroll provider needs; alo never computes a social-security
  contribution.
- **Tax filing** — figures, not submissions. No ELSTER, no
  impots.gouv, no Intervat.
- **National charts of accounts as shipped artefacts** (SKR03/04, PCG, MAR)
  and **DATEV export** — the latter is `[B+]` in features.md and is a
  handshake with a specific German product, not a general capability.
- **Live PSD2 bank feeds** — ADR 0035 and ADR 0009: PSD2 goes through a
  licensed aggregator, integrated by a human with a contract. What we build
  is file import, which needs no licence and works with every bank.
- **Reverse-charge and intra-community purchase VAT.**
  `billing_einvoice_import` already **refuses** a bill whose lines carry
  category `AE`, `K`, `G` or `E`, because our line holds a rate and not a
  category, and understating a return is worse than refusing a file. So a
  reverse-charge purchase cannot be imported today and therefore cannot be
  booked — the limitation is inherited, named here, and lifting it is a
  billing-side change (a category on the line) before it is a ledger one.
  **Flagged for a human**: a business buying software from another member
  state hits this in its first month.
- **Partial VAT deductibility** (the entertainment and vehicle rules several
  member states apply) — a per-category percentage is easy; knowing which
  percentage is legal is a compliance statement.
- **Depreciation, fixed-asset registers, accruals, prepayments and
  provisions** — real accounting, and each one a wave's worth of policy. The
  manual journal entry is the escape hatch until then, which is exactly what
  it is for.
- **Multi-entity consolidation, inter-company postings, cost centres beyond
  the `project_id` dimension.**
- **Automatic FX revaluation of open balances at period end** — the
  settlement difference is posted (B4.04b); revaluing what is still open is a
  policy choice per tenant.
- **Cash-basis accounting** — B4 books on the accrual basis (an invoice
  posts at issue). Several member states permit a cash basis for small
  businesses, and offering it is a per-tenant setting that changes every
  posting rule's timing: a decision, not a variation.
- **Budgets and forecasts** — that is Insights' shape (BI-2, which the
  ADR 0037 note already queues after B4).

## Open questions flagged for a human

- **Can an accountant be a user without a mailbox?** The role lands in
  B4.12; the identity model gives every user a mailbox. It is an identity
  change, and until it is made, "no mail" means "an empty mailbox and no
  shares" — which is honest but is not the same sentence features.md uses.
- **Which country's books is a tenant keeping?** The neutral chart works
  everywhere and is optimal nowhere. Whether alo ships national charts (and
  therefore makes a per-country claim) is a product and legal decision.
- **What is the retention period for the journal?** Member states require
  between 6 and 10 years for accounting records, and alo's tenant-delete
  cascade is immediate by design. A tenant leaving with unfiled books is a
  contract question before it is a schema one.
- **Should approving an expense notify the claimant?** The same question
  B3.01 left open for a rejected week, and the same answer for now: the
  status is on their screen and the module drafts nothing by itself.
- **Compliance, flagged and not guessed:** the German GoBD and the French
  and Italian equivalents require accounting records to be unalterable,
  traceable and completely retained. This design's append-only journal,
  reversal-only correction, per-event idempotency key and audit trail are
  built to that shape *deliberately* — but whether alo **claims** GoBD
  conformity (which involves a documented procedure, not only a schema) is a
  legal statement for a human, exactly as the working-time-record claim was
  in B3.

## Languages (B4.15)

The module reads in **English, French and Dutch**, end to end: the claim form
an employee fills in, the approver's queue, the bank import and the
reconciliation screen, the chart of accounts, the four reports, and the three
agent cards that read the books — 350 keys per language, pinned by
`locale.test.ts` § "alo Finance is fully translated (B4.15)" so a key added
later without its translations turns the suite red.

Three choices worth recording, because each is a place a transliteration would
have been wrong rather than merely clumsy:

- **The words are the documents', not the English's.** French says *note de
  frais*, *relevé bancaire*, *plan comptable*, *déclaration de TVA*; Dutch says
  *declaratie*, *rekeningafschrift*, *rekeningschema*, *btw-aangifte*. Dutch
  uses **afletteren** for the bank work — what a bookkeeper says, where
  "matchen" would be a loanword on the one screen an accountant opens daily.
- **No participle agrees with an interpolated amount.** A money value arrives
  as a formatted string, so French "1,00 € restent dus" would be ungrammatical
  for every singular amount and correct only by luck. Four sentences were
  therefore re-authored as invariable ones — *restant à payer*, *un écart de
  …*, *Nous avons reçu …*, *Retour de … à …* — and a test asserts each with a
  singular amount. The expense **statuses** do agree (*Approuvée*, *Refusée*,
  *Remboursée*), which is safe in the opposite direction: their subject is
  always *la note de frais* and never another document, unlike B2.14's record
  history, which had to drop participles entirely.
- **A word shared with Billing stays one word.** A payment settles a *billing*
  invoice and is read on a *finance* screen, so `issued` is *Émise* /
  *Uitgegeven* here exactly as it is there (B1.27), pinned by its own test. The
  same document must not appear to have two states in two modules.

Untranslated on purpose, and stated so the gap is a decision:

- **The CSV column headings stay English in every language** (`finance_reports`
  says so at the top of the file). They are a contract read by scripts and by
  an accountant's own tooling; what a *person* reads is the screen.
- **The server's refusal sentences are English in every language** —
  unchanged since B1.27, still the same cross-cutting item (a typed error
  vocabulary across `StoreError`), and still a human's roadmap call rather
  than a wave review's.
- The **default chart of accounts** is not a catalog string at all: it is
  seeded per tenant in the reader's own language by `finance_chart_names.rs`
  (en/fr/nl, checked against `CHART` by a test) and is ordinary tenant data
  from the moment it is written.

## What B4 promised, and what B4 shipped (B4.15)

Every `[B4]` line of `docs/features.md`, against the code. A line is shipped,
or it is a cut with its reason — nothing is silently missing.

| `[B4]` feature | Status |
|---|---|
| ★ Finance agent — categorise, "anything unusual?", VAT summary | **Shipped** (B4.14a/b) as `categorise_transactions`, `flag_anomalies`, `vat_summary`, propose-then-approve, answers with the entries behind them. **One narrowing**: categorise reads *expense claims*, not bank transactions — a bank line is attributed on the reconciliation screen (B4.09), which is a confirm-a-suggestion flow of its own, and two doors onto the same act would be two ways to book it. |
| ★ Receipt capture: photo/PDF → vendor, date, amount, VAT, human confirms | **Shipped as the deterministic half** (B4.06a/b): the extractor behind a pluggable trait, fixture-proven, `POST /finance/receipts` returning parsed-fields-for-confirmation, confirmed → claim. **Cut: no AI backend and no upload screen.** The trait is the seam a model plugs into; wiring one is a human item (ADR 0034's rules, a hosted model, a cost), and the claim dialog has no file picker until the Drive picker is reusable. |
| Expense record: category, project link (billable → B1 rebill), method; submit → approve → reimburse | **Shipped** (B4.05a/b, B4.13a) — record, project link, method, the three transitions, the approver's queue and the reimbursement list. **Two cuts**: the category has no picker (`/finance/categories` has no HTTP door, so the form cannot offer the chart it would point into), and **an expense is never rebilled to a customer** — B3.06 rebills *hours*; rebilling a cost is an invoice-line rule nobody wrote. |
| Mileage claims at a per-km rate table | **Shipped API-only** (B4.07): per-tenant rate table, `/finance/mileage*`, entry → claim, tested. **Cut: no screen.** A claimant types kilometres through the API. |
| Chart of accounts: EU SME default, editable, per-tenant | **Shipped** (B4.02, B4.13c), seeded in the reader's language, editable, retire-not-delete, posting rules resolving by role and never by number. |
| Double-entry ledger: every invoice/expense/payment posts automatically | **Partially shipped, and this is the wave's largest gap.** The journal, the balanced-entry enforcement and the property tests are real (B4.03a/b); the posting rules for invoice issue, payment settlement and credit note are written and golden-tested (B4.04a/b/c); a **reconciliation confirm books the invoice and the payment** (B4.09a), which is the one path that actually opens a tenant's books. But **no `/billing` route calls a posting rule**: issuing an invoice over HTTP does not post it, and settling a payment over HTTP does not post it. And **there is no expense posting rule at all** — `SourceKind::Expense` exists in the model with nothing writing it. So a tenant who issues invoices and never reconciles a bank statement has an empty journal, and every report over it is empty rather than wrong. **Flagged for a human as the first item of any B4 follow-up**: post on issue/settle/credit, idempotently, inside the document's own transaction. **Proved on the wire by B6.12a** (`docs/autonomy/STATE.md`), which adds two details: `post_credit_note_issue` has *no* caller anywhere — a credit note cannot enter any tenant's books by any sequence of HTTP requests, because reconciliation books invoices only — and until this is wired, `/billing/reports/vat` (documents) and `/finance/reports/vat` (journal) answer different numbers for the same quarter, the second of them zero. |
| Manual journal entries with description + attachment | **Not shipped.** `AccountStore::post_fin_entry` exists, is tested and is the escape hatch the design note leans on for depreciation and accruals — but it has **no HTTP route and no screen**, so today it is reachable only from Rust. It was never a queue item; the queue went from the posting rules straight to the reports. |
| Bank statement import: CAMT.053, MT940, CSV mapping wizard | **Shipped** (B4.08a/b/c) — three parsers against public goldens, staged lines, a mapping wizard, partial-import reports, duplicate detection, and the import screen (B4.13b). |
| ★ Reconciliation with AI matching, one-click confirm, rules learned per tenant | **Shipped** (B4.09a/b/c, B4.13b) — exact matching, windowed heuristics, per-tenant learned rules, manual pick, set-aside, and an undo on each. **Named honestly: no model is involved.** The matching is deterministic code and the "learning" is a per-tenant rules table, which is why every suggestion can state its own evidence. |
| Fiscal periods with soft close | **Shipped** (B4.10) — postings before the lock date refused typed, admin unlock audited. |
| Reports: P&L, balance sheet, aged receivables/payables, VAT — exportable CSV/PDF | **Shipped, CSV only** (B4.11a–d, B4.13c). Four reports, goldens on a seeded year, a CSV per report. **Cut: no PDF.** Billing's PDF pipeline (B1.17) renders a *document*; a report is a table an accountant re-sorts, which is what the CSV is for. A PDF is one print view away if a human asks. |
| Accountant access role: read + journal-only, no mail/files | **Shipped** (B4.12) — the first scoped role in alo, proven by tests that a finance reader reaches finance and nothing else. **The open question is unchanged**: an accountant is still a user with a mailbox, because the identity model gives everybody one; "no mail" today means an empty mailbox and no shares. |
| `[B+]` PSD2 live feeds, DATEV export | **Out of scope by the ADR**, unchanged: PSD2 needs a licensed aggregator and a contract (ADR 0009), DATEV is a handshake with one German product. |

Two cross-cutting notes the reconciliation turned up, both belonging to a
human rather than to this note:

- **`/finance` still needs adding to the production Caddyfile** at the next
  deploy, beside `/billing`, `/crm`, `/audit`, `/insights` and `/projects`.
  The loop does not touch `deploy/`.
- **Nothing in wave B4 is deployed.** Like B2, BI-1 and B3, it is code,
  migrations and tests behind a gate a human moves.
