# Design note — alo HR (people, their time off, and the two things we refuse to build)

Status: **design** (B6.01, written before the first migration) · ADR 0035 ·
Business track wave B6

alo HR is the sixth and last Work OS module, and the first one whose records
are **about people who have rights over them**. Every other module in this
repo stores facts the tenant owns outright: an invoice is ours to issue, a
stock move is ours to explain, a journal entry is ours to post. An employee
record is different in kind. The person it describes can demand a copy of it
(GDPR Art. 15), demand it be corrected (Art. 16), and in several cases demand
it be erased — and separately, the law tells us to keep parts of it for years
after they leave. Two obligations pulling opposite ways, on the same row.

That gives the module its central rule, and most of this note is a consequence
of it:

> **A person's record is readable by them, by the people who must act on it,
> and by nobody else — and every act on it is recorded.**
> "Everyone in the tenant" is never the answer to who can see an HR field.

The rejected alternative, stated once so the rest of the note can lean on it:
**HR as an ordinary tenant-wide module like Billing**, where any authenticated
member reads the directory and an admin flag guards the writes. It is what the
five modules before this one do, it would be half the code, and it is wrong
here for a reason that has nothing to do with taste: a tenant-wide read of
`hr_employees` is a tenant-wide read of everybody's date of birth, home
address, pay and sickness pattern. The blast radius of forgetting one `WHERE`
in Billing is a customer list; here it is the workforce's private lives. So
access is a **door you must hold**, the doctrine
`docs/design/account-scoped-access-door.md` states and `docs/design/projects.md`
first applied to personal data ("the hours of a person are personal data"). B3
needed two doors. B6 needs three.

The second thing that makes this wave unlike the five before it: **the largest
design decision in it is a decision not to build a feature.** `docs/features.md`
promises CV screening as "suggest-only with mandatory human decision". This
note declines to build screening in any form — not suggest-only, not ranked,
not scored — and § *The EU AI Act posture* gives the reasoning at length. The
queue item B6.09 already reads "screening explicitly absent per design note";
this is that design note, and the features.md line needs a human's amendment.

> **Wave gate, flagged for a human.** `ROADMAP.md` gates wave B2 on "B1 live
> with ≥1 real tenant", and B1, B2, BI-1, B3, B4 and B5 are all code-complete
> and undeployed. This note is design work, which is what belongs ahead of an
> unmet gate; **B6.02a is the first item that writes a migration**, and a human
> should confirm or move the gate before it ships. Recorded in
> `docs/autonomy/STATE.md` rather than decided here. The gate matters more in
> this wave than in the last four: the first migration here creates the table
> that holds employees' home addresses and pay, and a tenant who has never
> agreed to that being in our database should not have it created for them by
> a loop.

## Surface

- **Inputs:** authenticated workspace users driving `/hr/*` on `alo-jmap` —
  their own record and leave balance; a manager's decisions on their direct
  reports' leave; HR's directory, employment records, documents, leave
  policies, holiday calendars, onboarding checklists, job openings and
  applicants; and the payroll export. The HR agent (ADR 0034, item B6.09) is a
  second caller of the same store functions, never of a parallel code path.
- **Outputs:** JSON resources; one CSV (the payroll export); **draft** letters
  in alo Mail when the agent fills a tenant-authored template, which are not
  letters until a human reads and sends them; task projects in the existing
  Tasks module when an onboarding checklist is instantiated; and a read-only
  **absence layer** the Agenda draws, which writes no calendar events (§ *The
  absence layer, and why it is not a calendar*).
- **Who calls it:** `web/src/hr` (the module UI, B6.08) calls `alo-jmap`; the
  `alo-ai` HR module produces propose-then-approve envelopes that `alo-jmap`
  executes. **Nothing external calls HR** — no HRIS, no payroll bureau, no job
  board. Every integration named in that sentence is in the cuts below, and the
  payroll bureau's is a CSV a human downloads.

`/hr` is a **new top-level route prefix**: the production Caddyfile needs it
added at the next deploy, the same standing human action `/billing`, `/crm`,
`/insights`, `/projects`, `/finance` and `/inventory` carry, and it must join
`API_PATHS` in `web/vite.config.ts` in the same item that registers the first
route or every call 404s into the dev SPA — the lesson S1.11, BI1.04, B3.04,
B4.05b and B5.04a have each now paid for. To be noted in STATE.md at B6.02a,
not touched by the loop.

The prefix doubles as the SPA path, exactly as the other six do: the dev proxy
bypasses itself for HTML navigations, so one word serves the API and the router
without a second name to keep in sync.

### Routes

All under the authenticated `alo-jmap` router, following the convention
`/billing/*` established and `/crm/*`, `/projects/*`, `/finance/*` and
`/inventory/*` confirmed: the `authenticate` extractor, typed `Problem` errors,
the store-error map in [`billing::map_store_err`] reused rather than copied,
registration in `server.rs`.

| Route | Purpose | Door |
|---|---|---|
| `GET /hr/me` | my own record, my balances, my requests, my checklist — the whole of the module for somebody who is not a manager (B6.02a) | own |
| `GET/POST /hr/employees` | the directory, and a new record (B6.02a) | mixed / HR |
| `GET/PATCH /hr/employees/{id}` | one record; **the fields returned depend on the door** (§ *Three doors*) | mixed |
| `POST /hr/employees/{id}/archive` | somebody left: the record stops being current and stays for its retention period | HR |
| `GET/POST /hr/employees/{id}/documents` | contracts and letters — Drive nodes in the HR-only area, listed and attached by reference (B6.02b) | HR |
| `DELETE /hr/employees/{id}/documents/{doc_id}` | detach a document filed against the wrong person | HR |
| `GET /hr/org` | the chart, derived from the manager links; names and roles only (B6.02b) | any member |
| `GET/POST /hr/leave-policies` | the policies a tenant runs (B6.03a) | HR |
| `GET/PATCH/DELETE /hr/leave-policies/{id}` | one policy. `DELETE` only while no employment has ever been on it; after that it archives, because a balance is only explicable beside the policy that produced it | HR |
| `GET /hr/leave-balances?employee_id=&year=` | accrued, carried, taken, booked, remaining — with the working that produced each (B6.03a) | own / manager / HR |
| `GET/POST /hr/leave-requests` | the list (filtered `scope=mine\|team\|all`), and asking for time off (B6.03b) | own / manager / HR |
| `GET/PATCH /hr/leave-requests/{id}` | one request; editable only while it is still a request | own |
| `POST /hr/leave-requests/{id}/withdraw` | take back a request nobody has decided | own |
| `POST /hr/leave-requests/{id}/approve` · `/reject` | the decision, with an optional note | manager / HR |
| `POST /hr/leave-requests/{id}/cancel` | cancel **approved** leave that has not started; the balance comes back | own / manager / HR |
| `GET /hr/absences?from=&to=` | who is away on each day in the range — a name and the word *away*, never the reason (B6.03b) | any member |
| `GET /hr/holidays?calendar=&year=` | the public holidays of one calendar (B6.04) | any member |
| `GET/PUT /hr/holiday-calendars` | which calendars this tenant observes, and which is the default | HR |
| `GET/POST /hr/checklist-templates` | onboarding and offboarding templates (B6.05) | HR |
| `GET/PATCH/DELETE /hr/checklist-templates/{id}` | one template | HR |
| `POST /hr/employees/{id}/checklists` | instantiate a template for this person: a real task project, assigned and dated (B6.05) | HR |
| `GET/POST /hr/openings` | job openings, and a new one (B6.06a) | HR |
| `GET/PATCH /hr/openings/{id}` · `POST .../publish` · `/close` | one opening and its two transitions | HR |
| `GET/POST /hr/openings/{id}/applicants` | the pipeline for an opening, and recording an application | HR |
| `GET/PATCH /hr/applicants/{id}` | one applicant: their documents by reference, their notes, their stage | HR |
| `POST /hr/applicants/{id}/move` | move to another stage — **the only way a stage changes**, and always a person's act (§ *The EU AI Act posture*) | HR |
| `POST /hr/applicants/{id}/notes` | an interview note, written by the person who was in the room | HR |
| `DELETE /hr/applicants/{id}` | erase a candidate whose retention date has passed — the record, its notes and their CV. Added at B6.06a: § *Applicants are different, and get a deadline* promises a person presses a button, and a button needs an endpoint | HR |
| `POST /hr/payroll-exports` | draw the period's CSV **and file the fact that somebody drew it** (B6.10) | HR |

Thirteen path segments are reserved words under `/hr` — `me`, `employees`,
`org`, `leave-policies`, `leave-balances`, `leave-requests`, `absences`,
`holidays`, `holiday-calendars`, `checklist-templates`, `openings`,
`applicants`, `payroll-exports`. Ids are base64url'd 16-byte random tokens
(`id.rs`), so a record can never *be* one of them, and matchit prefers a static
segment to a capture; this is the shape `/tasks/labels` beside `/tasks/{id}`
and `/inventory/stock` beside `/inventory/moves` already have.

#### Why `leave-requests` and not `leave/requests` — the decision

The obvious spelling groups the module's three leave surfaces under a `leave`
segment: `/hr/leave/policies`, `/hr/leave/requests/{id}/approve`. It reads
better and it would break the audit trail.

`audit_action::event_for` derives a record's history **mechanically from the
matched route template**: the module is the first segment, **the collection is
the second**, and the record's id is the third. `/hr/leave/requests/{id}/approve`
would file its entry against a kind of record called `hr.leave` with no id at
all — every approval in the tenant landing on one nameless pile, and the
question this trail exists to answer ("who approved *my* leave, and when")
unanswerable. `tests/audit_routes.rs` reads the router's own source and fails
the build for a mutating route it cannot derive an action for, so the mistake
would be caught; it would be caught after the routes were written, which is
the mistake B3 made and paid to undo.

So the grouping is in the name, not the path: `leave-policies`,
`leave-requests`, `leave-balances`, `checklist-templates`, `holiday-calendars`
— kebab-case multi-word segments, matching `/billing/invoices/{id}/credit-note`,
`/crm/deals/{id}/next-steps` and `/inventory/purchase-orders`. The derivation
then produces exactly what it should: `hr.leave_request.approve` against the
request's own id, `hr.employee.document.create` against the employee the
document was filed on.

#### The payroll export is a POST — the decision

Every other export in the product is a `GET` with a `.csv` twin: `/finance/entries.csv`,
`/inventory/shortages`, the VAT report. The payroll export is a `POST` that
creates an `hr_payroll_exports` row and answers with the CSV body.

The reason is that the audit trail only records mutations (`is_mutating`), and
**this particular read deserves a line more than most writes do.** It returns
every employee's pay in one response. "Who downloaded the payroll file, and
when" is a question a works council, a data-protection officer and a fraud
investigation all ask, and answering it with "we do not record reads" is a poor
answer when the read is *this* one. Making the export a record rather than a
response is the smallest honest way to get it into the log that already exists.

*Rejected: a general read-audit for HR routes.* Auditing every `GET /hr/*`
would file a line each time somebody opens their own leave balance — noise that
buries the handful of lines that matter, and a second audit mechanism beside
the derived one. One route is the exception; the exception is a `POST`
because that is how the existing machinery sees it.

### Web surface

`web/src/hr`, the module pattern Billing, CRM, Insights, Projects, Finance and
Inventory share: one `HrModule.tsx` owning the tab layout, one `api.ts` owning
the fetches, one view per screen, `.module.css` for layout, `ds` tokens for
everything else, and every string through `i18n/en.ts` under an `hr*` prefix
(fr/nl at the wave review, B6.11). Every screen obeys
`docs/design/ux-principles.md`.

The rail entry is **visible to every member**, which is a departure from the
five business modules before it and is deliberate: the most-used screen in HR
is an employee's own — *how much leave have I got left, and can I have next
Thursday off* — and hiding the module behind a role would make the answer
something you ask a person for. What varies by door is the tabs:

| Tab | Who sees it |
|---|---|
| **My leave** — balance with its working, request form, my requests, my checklist | every member |
| **Team** — my direct reports' requests awaiting me, their booked absence | anyone with a direct report |
| **Directory** — the people list and the org chart | every member (public fields only) |
| **People** — the full records, employments, documents | admin or HR |
| **Policies** — leave policies, holiday calendars, checklist templates | admin or HR |
| **Hiring** — openings and the applicant board | admin or HR |
| **Payroll** — the period export | admin or HR |

Three UX laws bite hardest here and are worth naming with their screen:

- **Recognition over recall.** A leave request is picked on a calendar showing
  the team's already-booked absence and the tenant's public holidays *behind*
  the selection, so "can I take this week" is answered by looking rather than
  by asking. The request form never asks for a number of days: it computes them
  from the dates and the person's own working pattern, and shows the working.
- **Empty states are the onboarding.** A tenant that has never opened HR sees
  one screen: *add the first person, or import nobody and just set up leave.*
  The policy screen seeds one policy from the tenant's country (§ *Policies*),
  so a tenant that presses nothing still has a workable annual-leave policy
  rather than an empty table.
- **Undo over confirm.** Approving leave is undoable (`cancel` returns the
  balance) and therefore is not confirmed. Two acts *are* confirmed, because
  they are not undoable: archiving an employee, and drawing a payroll export
  (the confirm sentence says what the file contains and that the download is
  recorded).

#### As built (B6.06b), and the four decisions the module's first screen made

The module exists with **one** tab, `Hiring` (`HrModule.tsx`, `HiringView.tsx`,
`HiringBoard.tsx`, `ApplicantDrawer.tsx`, `OpeningDialog.tsx`,
`ApplicantDialog.tsx`, `parts.tsx`, `api.ts`, `types.ts`, `format.ts`,
`hr.module.css`). The rail entry is every member's, as this section says — but
the member-facing tabs were B6.08, so a member who opened People then was told
in the module's own words what would live there rather than shown a tab that
answers nothing. (**Superseded by B6.08a**: Directory is every member's tab and
what they land on; the "coming soon" screen and its two strings are gone. My
leave and Team remain B6.08b.) The Hiring tab itself is **hidden, not
disabled**, for anybody who is
not HR: the pattern Finance set for its bookkeeper tabs, and for the same
reason — a tab that exists only to refuse advertises a door.

1. **The columns are the served vocabulary.** `GET …/applicants` answers
   `stages`, and the board draws exactly those, in that order. A build that
   gains a stage gains a column with no web release, and the test proves it by
   serving three stages and counting three columns.
2. **A drag has no position, unlike the CRM board it otherwise copies.** An
   applicant row has no order within a stage, deliberately: two people at
   `interview` are not ranked, and a board that let one be dragged above the
   other would be a hand-drawn ranking of candidates. Cards read in the order
   the applications arrived — the order the server sends.
3. **The stage picker in the drawer is not a convenience.** A board that can
   only be worked by dragging cannot be worked from a keyboard, and deciding
   somebody's candidacy is the last place an interface may require a mouse.
   Both paths post the same audited `POST …/move`.
4. **The client is never the access decision.** `canWorkHr()` (session
   `alo:isAdmin` or the `hr` role) decides whether the *tab is drawn*; every
   `/hr` route asks `require_hr` again for itself, so a stale session hides a
   tab at worst and opens nothing at all.

Two acts here are confirmed, for the reason the law above gives: **closing a
round** (terminal, and it freezes what the role said) and **erasing a
candidate** (the record, its notes and the CV really go). Publishing is not:
nothing worse than closing the round undoes it.

## Three doors

B3 established two doors — `AccountStore` for a person's own data,
`TenantStore` for cross-user reads behind `require_admin` — and this module
needs a third, because the person who must decide your leave is neither you nor
an admin. It is your manager, and "manager" is not a role anybody grants: it is
a shape in the org chart.

- **The first door — your own.** `AccountStore` functions carrying
  `user_id = self.user` in every statement: `/hr/me`, your balances, your
  requests, your withdraw and your cancel. Reaching a colleague's leave through
  this door is unrepresentable in the API rather than merely rejected. This
  door is also, and not incidentally, **the subject-access answer**: an
  employee asking what we hold about them opens a screen instead of writing to
  a controller.
- **The second door — your reports.** A `TenantStore` read narrowed by the org
  chart: the requests of the employees whose `manager_employee_id` resolves to
  the caller's own employee record. Not a role, not a flag — a link somebody
  drew when they said who reports to whom. **Direct reports only**; the chain
  is a cut (below).
- **The third door — HR.** `TenantStore` behind `require_hr`, which accepts a
  tenant admin **or** the new `TenantRole::Hr`: the whole directory including
  private fields, employments and pay, documents, policies, hiring, and the
  payroll export.

*Rejected: one door with an explicit `employee_id` argument after a
permission check.* That turns "a person's HR record is their own" into a rule
every future caller must remember, and the caller that forgets leaks a
colleague's home address. It is the same rejection B3 made for the same reason;
the data here is worse.

### The HR role

`TenantRole` gains a second value, `Hr`, which is precisely what
`platform/alo-store/src/tenant_roles.rs` and migration `0149` both said the
second role would be — the module doc names B6's HR role by name, and the
migration comment says "the second role widens this table by a value in the
CHECK and the gates by a word". This wave takes them at their word:

- `TenantRole::Hr` with `as_str()` `"hr"`, parsed and displayed like the first;
  `TenantRole::parse`'s refusal message widens to "role must be one of:
  accountant, hr" — a wire-visible string, additively changed.
- A migration that widens the CHECK: `ALTER TABLE tenant_user_roles DROP
  CONSTRAINT tenant_user_roles_known, ADD CONSTRAINT tenant_user_roles_known
  CHECK (role IN ('accountant','hr'))`. Expand-only, no data lost, no column
  dropped — inside the append-only rule, and the only way to widen a CHECK in
  PostgreSQL.
- `Account::require_hr` in `alo-jmap`'s state, beside `require_finance`: admin
  or the role, refusing with `403` and a sentence naming what the role is for.

**`scoped_roles`' middleware is untouched by this wave, and that is a
checked fact rather than an omission.** It refuses billing/CRM/inventory writes
to a caller who *holds the accountant role and is not an admin* — it keys on
the accountant specifically, not on "holds some scoped role" — so an HR holder
who is also an ordinary employee keeps every ordinary capability they had. The
HR role only ever **adds**. Two consequences, both intended:

- An **accountant may not read `/hr/*`.** The role is "the books and none of
  the mail", and an external bookkeeper reading everybody's contract and home
  address is exactly the failure that role exists to prevent. An accountant who
  genuinely runs payroll is granted the HR role by an admin, deliberately, with
  the grant's provenance recorded — which is the whole point of roles being
  rows.
- A person can hold **both**, and then holds both sets: the union, never a
  product.

*Rejected: putting `hr` in `READ_ONLY_FOR_ACCOUNTANT`-style deny lists.* The
accountant's gate is a subtraction (a reader who must not write); HR's is an
addition (a member who may also see the workforce). Expressing an addition as a
subtraction from everything else would mean listing every module HR *cannot*
touch, a list that is wrong the day somebody adds a module.

### What each door sees of an employee record

One table, three projections. The projection is chosen in the store by which
function was called, not by a field filter applied at the edge — a filter at the
edge is a filter somebody forgets on the second route.

| Field group | Directory (any member) | Own | Manager (of them) | HR |
|---|---|---|---|---|
| name, job title, team, work email, work phone, manager | ✓ | ✓ | ✓ | ✓ |
| start date | ✓ | ✓ | ✓ | ✓ |
| leave balance | — | ✓ | ✓ | ✓ |
| leave **requests and dates** | — | ✓ | ✓ | ✓ |
| leave **type** of a request (incl. sick) | — | ✓ | — *(see below)* | ✓ |
| date of birth, home address, personal email/phone, emergency contact | — | ✓ | — | ✓ |
| national id / social-security number, IBAN | — | ✓ | — | ✓ |
| employment: contract type, working pattern, end date | — | ✓ | ✓ (pattern only) | ✓ |
| pay amount and period | — | ✓ | — | ✓ |
| documents | — | ✓ *(their own)* | — | ✓ |
| notes on an applicant | — | — | — | ✓ |

**A manager sees that you are away, not what is wrong with you.** The leave
*type* is withheld from the manager door and the reasoning is in § *Sickness is
health data*. A manager who must know — because a sick day and a holiday
consume different balances — sees which **policy** the request draws on, which
is the operational fact, not the medical one. In a tenant whose policy set has
exactly one sick policy the two are the same fact, and that is the tenant's
choice of policy names, not our disclosure.

## The data model

### Employees, and the employments under them

Two tables, not one, and the split is the wave's second-largest modelling
decision:

```
hr_employees        the person: identity, contact, the link to a user account
  id                base64url token (id.rs)
  tenant_id
  user_id           NULLABLE — see below
  staff_number      TEXT, unique per tenant when set
  given_name, family_name, preferred_name
  work_email, work_phone
  personal_email, personal_phone      -- private
  date_of_birth     DATE               -- private
  address_*         (line1/line2/postcode/city/region/country) -- private
  national_id       TEXT               -- private, see § Data minimisation
  iban              TEXT               -- private, validated by iban.rs
  emergency_name, emergency_phone      -- private
  manager_id        NULLABLE REFERENCES hr_employees (id)
  photo_node_id     NULLABLE — a Drive node
  status            'active' | 'archived'
  created_at, updated_at

hr_employments      the terms: what changes while the person does not
  id, tenant_id, employee_id
  job_title, team
  contract_kind     'permanent' | 'fixed_term' | 'part_time' | 'apprentice'
                    | 'contractor' | 'intern'
  started_on        DATE
  ended_on          NULLABLE DATE
  pattern_minutes   INT[7]             -- minutes normally worked, Mon..Sun
  leave_policy_ids  the policies this employment is on
  holiday_calendar  the calendar this employment observes
  pay_amount_cents  NULLABLE BIGINT    -- private, HR door only
  pay_period        'hour' | 'month' | 'year'
  pay_currency      CHAR(3)
```

**Why two tables.** A promotion, a move to four days a week, a pay rise and a
fixed-term renewal are all changes to the *terms*, and a leave balance computed
last March must still be explicable next March — which requires knowing the
working pattern that was in force **then**, not the one in force now. One table
with in-place edits would make every historical balance unreproducible the
moment somebody went part-time. So employments are **appended**: a change ends
the current row (`ended_on`) and starts the next, and any date-bound
computation asks which employment covered that date.

This is the same shape B1 used for the FX rate snapshot and B3 for the rate on
a time entry — *the figure that was true when the fact happened is stored with
the fact*, never re-derived from today's settings. It is worth stating as the
module's second rule:

> **A balance is always recomputable from the requests, the policies and the
> employments that were in force on each day.** Nothing about leave is stored
> as a running total.

*Rejected: a `balance_minutes` column decremented on approval.* It is the
`qty_on_hand` mistake B5 refused for stock, applied to time off: two sources of
truth that drift, discovered months later by an employee who counts their own
days and gets a different number, with no way to say which was lying or when it
started. The fold is cheap (a year's requests for one person is tens of rows)
and a cached figure, if it is ever needed, is a cache proven a fold by a test —
the pattern `inv_stock.rs` already carries.

### `user_id` is nullable — the decision

Not every employee has a login. A warehouse hand, a shop-floor worker, a
seasonal picker: they are employed, they take leave, they appear on the payroll
export, and they will never open a mailbox we host. A model that made the user
account mandatory would force a tenant to buy a seat for somebody who cannot
use one, which is both wrong and a sales objection.

Consequences, all of them handled explicitly rather than left to fail:

- **The org chart links employees, not users** (`manager_id` is an employee
  id). The chart is complete even where the accounts are not.
- **The second door needs an account on the manager's side**, because a
  manager must log in to decide. A request whose approver has no user account
  falls through to the HR door, and the request says so in its body
  (`approverFallback: "hr"`) rather than silently sitting in nobody's queue —
  a decision nobody can make is the failure mode this whole surface exists to
  avoid.
- **A leave request for an account-less employee is entered by HR on their
  behalf**, and the audit line records who entered it. The `enteredBy` field is
  on the request for exactly this.
- **`user_id` is unique per tenant when set**, so two employee records cannot
  claim the same colleague.

### The org chart, and the cycle it must refuse

`GET /hr/org` folds `manager_id` into a tree. Three rules, each tested:

- **A cycle is refused on write, not detected on read.** Setting a manager
  walks up from the proposed manager; if the walk reaches the employee being
  edited, the write is a `422` naming both people — "A would then report to
  themselves through B". A chart that can be cyclic is a chart whose renderer
  must defend itself forever.
- **Depth is bounded** at 64 levels, which is far past any real organisation
  and stops a pathological chain from turning a read into a walk.
- **The chart is the one HR read every member gets**, and it carries public
  fields only: name, job title, team, manager. A company where you cannot find
  out who your colleague's manager is has an org chart in a filing cabinet, and
  we are replacing filing cabinets.

### Data minimisation, and the fields we refuse

GDPR Art. 5(1)(c) is a design constraint, not a policy document: the fields
above are the ones an employer needs to administer employment, and the list is
closed. **We do not add** — and a field request for any of these is refused
with this paragraph as the reason:

- nationality, ethnicity, religion, trade-union membership, sexual orientation,
  health condition, disability status (Art. 9 special categories, none of which
  a general workspace has a lawful basis to hold);
- marital status or dependants (payroll's business, and payroll is a permanent
  non-goal — the bureau asks the employee directly);
- a photo as anything but optional (a mandatory face on a directory is a
  discrimination surface, and it is the tenant's decision to make, not ours to
  require);
- performance scores, ratings, or anything that reads as an evaluation of a
  person by a system (§ *The EU AI Act posture*).

`national_id` exists because a payroll export without one is useless in most
member states, and it is the single most sensitive plain field in the schema.
It is HR-door only, it is never in a list response (only on the single-record
read), it is never in a log line, and the payroll export is the only place it
leaves the system — which is the other half of why that export is an audited
`POST`.

### Sickness is health data

A sick day is data concerning health under Art. 9, even when nobody wrote down
a diagnosis. The lawful basis for holding it at all is Art. 9(2)(b) —
processing necessary for obligations in the field of employment law — which is
a basis for **administering the absence**, not for circulating it. So:

- **The leave type is stored; a reason, a diagnosis or a doctor's words are
  not.** There is no free-text "reason" field on a sick request. A note field
  exists and its placeholder does not invite one.
- **A sick note is a document, not a field**: a Drive node in the HR-only area,
  referenced by id from the request. The file's *contents* are never read by
  us, never parsed, never summarised.
- **The absence layer says *away***, never why (§ below), and neither does the
  Agenda, the team tab, or any notification.
- **The manager door does not return the leave type** (§ *What each door
  sees*).
- **Nothing about a sick day reaches a `tracing` span.** Spans carry the
  request id, the employee id, day counts and minute counts. Not a name, not a
  type, not a note, not a pay figure. The same rule mail bodies have had since
  Phase 1, and the audit entry obeys it too: an entry says *who did what to
  which record*, never a field value.

### Retention: archived, never deleted

Employment records carry statutory retention in every member state — commonly
five to ten years after the contract ends, for payroll and tax — and the exact
period is national law we do not encode. The stance:

- **Archiving is the only removal HR performs.** `POST /hr/employees/{id}/archive`
  sets `status` and the employment's `ended_on`; the record leaves the
  directory, the org chart and the absence layer, and stays readable through
  the HR door with an "archived" marker.
- **An erasure request is a human act.** When a person's retention period truly
  has expired, deleting the row is an admin's deliberate act through the
  database, taken with legal advice. A scheduled job that deletes people is not
  something this loop is going to build unattended, and § *Out of scope* says
  so.
- **Applicants are different, and get a deadline.** An unsuccessful applicant's
  data has no employment-law retention behind it; the common European guidance
  is weeks to months unless the person agrees to a talent pool. So an applicant
  row carries `retain_until` (six months by default), and the hiring screen
  shows what is past its date. **The deletion is still a person pressing a
  button** (`DELETE /hr/applicants/{id}`, which takes the notes and the CV with
  it) — but the module is the thing that remembers to ask, which is the
  difference between a policy and a promise. *As built at B6.06a: the six
  months is a constant with the caller free to state any date, not yet a
  per-tenant setting — a setting needs an HR-settings surface, which no screen
  exists for until B6.08.*

### The HR area of Drive — decided at B6.02b

This note said documents were "Drive nodes in the HR-only area" without saying
what that area *is*. Drive (ADR 0027) had two location kinds, `personal` and
`space`, and access follows location — there is no per-node permission. So the
area is a **third location kind, `hr`: one per tenant, whose read and write gate
is [`TenantRole::Hr`] or being a tenant admin**, and a non-holder is answered
`NotFound` rather than `Forbidden`, because here the existence of a file is part
of what is being kept. `hr_documents` refuses to file a node that is not in it,
so a filing row can never claim "HR-only" over a file a colleague can open.

Two consequences worth stating, both asserted by tests:

- **The protection is Drive's own, not this module's.** A contract is fetched
  with the ordinary `GET /drive/nodes/{id}/download`, which refuses the
  colleague who learns the node id. There is deliberately no second download
  route under `/hr`, because a second path is a second access rule to keep in
  step with the first.
- **Nothing indexes it.** `search.rs` whitelists `personal` and `space`, and
  `drive_find` is personal-only, so an HR file's *name* never surfaces in a
  colleague's search — which a Space with a careful membership list would not
  have given us for free.

*Rejected: a Space called "HR".* A Space's members are managed per Space by
whoever manages it, so access to everybody's contract could drift away from the
HR role without anyone deciding it had. An access rule with two sources of truth
is the failure this module exists to prevent.

**Not yet decided (open, B6.03b at the latest):** an employee reading *their
own* contract. The door table above grants it, and the area's gate is the role,
so it needs either a per-node exception in the location rule or an HR-served
copy — a decision, not an oversight. Until it is made, `GET /hr/me` returns the
record and the terms and does not list documents.

## Leave

### Minutes, and the working pattern that makes a day mean something

Leave is stored, computed and carried in **integer minutes**. Never days as a
decimal, never hours as a float — the rule money has had since B1 ("integer
cents, never floats") applied to the second quantity in this product that
people will check by hand.

Days are the unit humans speak, and a day is not a fixed quantity: it is
whatever that person normally works on that weekday. `pattern_minutes` (seven
integers, Monday first) is what converts between the two, and it is read from
the employment **in force on the day in question**. A person who works
Mon–Thu 8h and Fri 4h has a 2 220-minute week; the Friday off costs them 240
minutes and the Tuesday 480, and the screen shows "0.5 days" for the Friday
because that is what half a *Friday* is. Displaying a balance in days divides
by the person's average working day over the leave year, and the screen shows
the minutes behind it on hover, because a number nobody can reproduce is a
number people distrust.

*Rejected: days with a half-day flag.* It is what most small systems do and it
cannot express a 4-hour Friday, a 30-hour contract, or a mid-year move from
five days to four — three things that are ordinary in Europe and that all turn
into either a wrong balance or a manual correction.

### Policies

```
hr_leave_policies
  id, tenant_id, name
  kind                'annual' | 'sick' | 'unpaid' | 'other_paid'
  entitlement_minutes INT     -- per full leave year, at a full-time pattern
  accrual             'up_front' | 'monthly'
  leave_year_start    (month, day)   -- 1 Jan default; April and other starts exist
  carryover_cap_minutes INT   -- 0 = no carryover
  carryover_expires_after_months  NULLABLE INT
  allow_negative      BOOL    -- may an approval take a balance below zero
  requires_approval   BOOL    -- a sick policy is often recorded, not approved
  paid                BOOL
  status              'active' | 'archived'
```

A new tenant is seeded **one** policy: annual leave, entitlement set from the
statutory minimum of the tenant's country in the same seed table the holiday
calendars use, `monthly` accrual, 1 January, no carryover. It is the smallest
policy set that is not empty, and the empty-state screen says plainly that it
is a starting point to edit. Directive 2003/88/EC Art. 7 sets the European
floor at four weeks; several member states are above it, and the seed table
carries the national figure with a comment naming its source. **A seeded
entitlement is a default, not advice** — the screen says so, because we are not
this tenant's employment lawyer.

### The arithmetic, and the property it must have

`hr_leave_math.rs` is a **pure module**: no database, no clock, no tenant — the
same shape `billing_totals.rs`, `time_hours.rs` and `fin_rules.rs` have, and
for the same reason (arithmetic that money or a statutory entitlement depends
on must be testable without a fixture).

Four computations:

1. **Entitlement for an employment year** — the policy's full-year figure
   scaled by the employment's pattern against a full-time week, and pro-rated
   by the days employed inside the leave year for a joiner or a leaver.
2. **Accrual to a date** — `up_front` grants the whole entitlement on the leave
   year's first day; `monthly` grants a twelfth at each month start.
3. **The cost of a request** — the minutes it consumes: for each day in the
   range, the pattern's minutes for that weekday, **minus** any day that is a
   public holiday on the employment's calendar, minus any day already covered
   by another approved request (an overlap is refused, but the arithmetic is
   defined so the refusal can say by how much).
4. **The balance** — `carried_in + accrued − taken − booked`, where *taken* is
   approved leave whose days have passed and *booked* is approved leave still
   ahead. Requests awaiting a decision consume nothing and are reported beside
   the balance as `pending`, so a manager approving the second of two
   overlapping requests is told what the first one already costs.

**The property, and the test that proves it.** Integer division loses
remainders, and twelve twelfths of 12 500 minutes is not 12 500 if each is
rounded independently. So `monthly` accrual carries the remainder forward:
month *n*'s grant is `entitlement × n / 12 − entitlement × (n−1) / 12`. The
property test asserts, over randomly generated entitlements, patterns, joining
dates and leaving dates:

- the twelve monthly accruals **sum exactly** to the full-year entitlement;
- pro-rating a joiner and a leaver whose employments partition a year gives two
  entitlements that sum exactly to one full year's;
- no computation anywhere returns a non-integer, and no `f64` appears in the
  module (a `#![deny]`-style test asserting the module's source contains no
  float type, the trick `billing_totals` uses);
- the cost of a request over a range equals the sum of the costs of its
  single-day sub-ranges — so a week booked at once and five days booked
  separately cost the same, which is the one arithmetic surprise employees
  actually notice.

#### As built (B6.03a), and the five decisions the code had to make

The table is `hr_leave_policies` (migration `0201`), the arithmetic is
`platform/alo-store/src/hr_leave_math.rs`, the CRUD is
`hr_leave_policies.rs`, and the country figures are `hr_statutory_leave.rs`.
Five things the note above left open were decided while building it:

- **A leave year starts on a day-of-month 1..=28**, refused above that by both
  the store and a CHECK. 29 February is a date three years in four cannot
  construct, and a balance fold must never guess one; every national start we
  have met (1 January, 1 April, 1 May, 1 July, 1 October) is inside the bound.
- **The seed reads the country from the tenant's billing settings** — the
  identity they invoice under is the only country the workspace knows — and
  falls back to the Directive's four weeks for a country the table does not
  carry. The seeded policy's **name comes from the caller**, so an HTTP route
  passes the reader's language rather than the store hardcoding an English
  string; a caller with no locale gets `Annual leave`.
- **Deleting a policy is not implemented; archiving is.** The note's `DELETE`
  is "only while no employment has ever been on it", and *ever been on it* is
  not answerable until leave requests exist (B6.03b). A rule that cannot bind
  yet would be a rule that silently permits everything, so the door is
  archive-and-restore until the tables that record use are there.
- **A carryover cap may not exceed the entitlement it carries, and an expiry
  needs a cap** (1..=24 months). Both are typos rather than policies, and both
  would otherwise produce a balance nobody can explain.
- **An archived policy is not editable** (`409`, naming restore). It exists to
  explain balances already folded from it; editing it would restate them
  silently. Its name is freed for a live policy the moment it is archived, and
  restoring it is refused while that name is in use.

The cost function takes **resolved days** — one entry per day carrying the
pattern minutes, the holiday flag and the already-covered flag — rather than a
range plus one pattern. It is what makes a request that spans a change of terms
(or a cross-border employee's own holiday calendar) fold correctly, and it keeps
the module pure.

### The request, and its state machine

```
              withdraw
        ┌──────────────────────► withdrawn
        │
   requested ──approve──► approved ──cancel──► cancelled
        │                     │
        └──reject──► rejected └──(days pass)──► taken (derived, not stored)
```

- **`requested`** is editable by its owner and by nobody else. A manager who
  wants a different date rejects with a note; editing somebody's request into a
  different request they did not make is not a thing this module does.
- **`approved`** is not editable at all. Changing approved leave is `cancel`
  followed by a new request, so the history reads as what happened.
- **`cancel` works while the leave has not started.** Leave already begun is
  amended by HR through an explicit correction (`PATCH` on the HR door, audited,
  with a reason note that *is* required there) — the fact that somebody was
  absent last Tuesday is not something an employee should be able to erase.
- **`taken` is derived from the calendar, never stored.** One less state to get
  wrong, and no nightly job to write it.
- **Overlap is a `409`, not a silent second booking**, naming the request that
  already covers those days.
- **A request in a locked past** — before the employment started, after it
  ended, or covering a day already taken — is a `422` naming the rule.

### Who approves

The employee's **manager**, resolved through `manager_id`, or **admin-or-HR**.
Nobody else, and specifically not "a team lead" or "a project owner" — the same
rejection `docs/design/projects.md` made for timesheets and
`docs/design/finance.md` made for expenses, for the same reason: leave is a
*person's*, not a team's or a project's.

Three sharp cases, decided here rather than by whichever screen ships first:

- **A manager may not approve their own leave.** `409`, naming that it must go
  to their own manager or to HR. An admin may (a one-person tenant has nobody
  else), and the audit entry records that it was self-approved.
- **A manager's manager is not an approver.** The chain is a cut (below);
  escalation today is HR, who can decide anything.
- **Requests requiring no approval** (a `requires_approval: false` sick policy)
  are created directly in `approved`, with `decidedBy` naming the requester —
  so the record does not pretend somebody decided.

#### As built (B6.03b), and the seven decisions the code had to make

The table is `hr_leave_requests` (migration `0202`), the record and its state
machine are `platform/alo-store/src/hr_leave_requests.rs`, the fold is
`hr_leave_balances.rs`, the layer is `hr_absences.rs`, and the surface is
`products/mail/alo-jmap/src/hr_leave_{policies,requests,balances,door}.rs`.
Seven things the note above left open were decided while building it:

- **A request stores its days, never its cost.** There is no `cost_minutes`
  column: what an absence consumes is folded at read time from the working
  pattern of the employment in force on each of its days. A frozen figure would
  be the `qty_on_hand` mistake one table further on — a corrected pattern, a
  holiday added to a calendar or an employment ended early would each leave a
  stored cost nothing can reconcile, and the person it is wrong for is the one
  person guaranteed to check it by hand.
- **Overlap is refused against `requested` as well as `approved`.** The note
  says "overlap is a `409`", and the sharper question is *overlap with what*.
  Two undecided requests for the same Tuesday are two answers to "am I off",
  and the second one is always a mis-click; the refusal names the dates of the
  request that already covers them. Rejecting or withdrawing frees the days
  again, immediately.
- **A range that costs nothing is a `422`, not an absence of zero minutes.** A
  weekend booked on a Monday-to-Friday pattern is a mis-typed date, and a
  zero-minute absence in the record would appear in the absence layer as a day
  somebody was away.
- **Leave cannot reach outside the employment**, on either side, naming the
  bound it crossed. Somebody cannot take leave from a job they did not hold —
  and an open period has no end to reach past, so a request for next year is
  ordinary.
- **The balance is checked as at the day the leave *starts***, not as at today.
  A monthly accrual that has not arrived yet is not a balance somebody can
  spend, and approving March's leave in January against December's accrual is
  how a company ends a year owing days it never granted. A rejection never
  consults the balance: saying no to leave somebody cannot afford must always
  be possible.
- **Carryover carries one year, not a chain.** Last year's remainder is folded
  with nothing carried into *it*. A day granted in 2024, unused in 2025 and
  still claimed in 2026 is exactly what the statutory expiry rules exist to
  stop, and every member state alo has met caps carryover at 15 or 18 months.
- **Reading the policies is every member's, writing them is HR's.** The route
  table above says HR for both, and building the request form proved it wrong:
  somebody asking for time off has to choose what kind, and a picker they may
  not read is a form they cannot fill in. What a company grants is a rule it
  publishes to its staff. `includeArchived` stays HR's, and `DELETE` on a
  policy is still not implemented — the verb is
  `POST /hr/leave-policies/{id}/archive`.

**Who may do what**, resolved once in `hr_leave_door.rs` rather than spelled
into six handlers: *mine* (the employee record linked to my login), *my team*
(one level of `manager_id`), and *HR* (admin or the HR role). Editing and
withdrawing belong to the person who asked and to nobody else — HR included,
because editing somebody's request into a different request they did not make
is not a thing this module does. Deciding is the manager's or HR's, never the
requester's own unless they are the tenant's admin. Cancelling is any of the
three, because it gives the balance back and takes nothing from anybody. A
refusal about somebody else's record is a `404`, never a `403`, so no answer is
an existence oracle.

**Not yet, and deliberately.** Public holidays are B6.04: the day fold has the
flag and passes `false` for every day until the calendars exist, so a tenant's
balances today are computed on the working pattern alone — correct rather than
degraded. The Agenda *drawing* the absence layer, and every leave screen, are
B6.08b; this item ships the layer the Agenda reads.

### Public holidays (B6.04)

A per-country (and where it matters, per-region) seed table of holidays,
selected per tenant and overridable per employment for cross-border staff.
Three decisions:

- **Seed data, in the repo, with its source named per country.** No external
  calendar service, no network call — the loop's rails forbid the call and the
  product's sovereignty promise forbids the dependency. The seed covers the
  member states alo sells into first, with the years it covers stated
  explicitly; a year past the seed's end answers `422` naming the gap rather
  than quietly returning no holidays, because "no holidays in 2031" and "we do
  not have 2031 yet" must not look the same to a balance computation.
- **Movable feasts are computed, not listed.** Easter and the four holidays
  that hang off it (Good Friday, Easter Monday, Ascension, Whit Monday) come
  from an anonymous Gregorian Easter algorithm in the pure module, unit-tested
  against a published table of Easter dates. Listing them per year per country
  is how a seed table silently runs out.
- **A holiday inside a leave range costs nothing**, which is the only reason
  this table is in scope at all. A tenant that observes no calendar gets a
  balance computed on the working pattern alone, which is correct rather than
  degraded.

#### As built (B6.04), and the five decisions the code had to make

The seed is `platform/alo-store/src/hr_holiday_seed.rs` (pure: fifteen national
calendars, the rules, the computus), the per-tenant choice is
`hr_holidays.rs` over migration `0203`, and the surface is
`products/mail/alo-jmap/src/hr_holidays.rs` —
`GET /hr/holidays?calendar=&year=` and `GET`/`PUT /hr/holiday-calendars`.
Five things the note above left open were decided while building it:

- **Two files, not one.** The note listed one `hr_holidays.rs`; the data and
  the computus have no database in them and the choice has nothing else, so
  they split the way `hr_leave_math.rs` and `hr_leave_balances.rs` already do.
  The pure half is testable without a fixture, which is what a table of
  statutory dates deserves.
- **The choice is one row per tenant, and it distinguishes "none" from "not
  yet".** `hr_holiday_selection` holds an array of observed calendars and the
  one the arithmetic uses. No row means nobody has chosen, and the first read
  seeds it from the country the company invoices under (`billing_settings`) —
  the same zero-setup rule that seeds their first leave policy. An **empty
  array** means a company deliberately observes none, and the seed never
  overwrites it. A row per calendar could not tell those two apart.
- **One calendar counts, the rest are for looking at.** A company with staff in
  three countries observes three, but the leave fold uses the default only:
  "is this day free" with two answers is a balance nobody can explain. The
  per-employment override the note anticipated (a cross-border employee on
  their own calendar) needs a column on `hr_employments` and is **not built**;
  the resolver (`TenantHolidays`) is the single seam it will grow from.
- **National days only, and the omission is on the wire.** German *Länder*,
  Spanish *comunidades*, Italian patron saints' days and Belgium's
  employer-chosen replacement day for a holiday falling on a Sunday are not
  carried. Each calendar has a `note` field saying so in the country's own
  language, because a missing holiday costs an employee leave they should have
  kept.
- **Reading the calendars is every member's, writing is HR's** — the same
  correction B6.03b made to this table's row for leave policies, for the same
  reason: an employee whose Christmas week costs four days is entitled to know
  which day was free. The route table above says HR for both; the code says HR
  for the `PUT` only.

Two names on one date is one day off (Luxembourg's Europe Day fell on Ascension
in 2024): the days list carries both names and the fold sees the date once.
Nothing is stored per request — a company that adds a calendar in March sees
January's approved leave recomputed with January's holidays in it, which is the
same "no stored figure" rule the balance itself follows.

### The absence layer, and why it is not a calendar

`GET /hr/absences?from=&to=` answers with, per day, the people who are away —
their name and their employee id, nothing else. The Agenda draws it as a layer
behind the week and month views, and the leave request form draws the same
layer behind its date picker.

*Rejected: writing approved leave as events into a shared calendar.* It is the
obvious design — the Agenda already renders events, shares calendars and
handles all-day items, so leave could simply be events and no new rendering
code would exist. Three things kill it:

- **Every calendar has an `owner_user_id`.** There is no tenant-owned calendar
  in this schema, so the absence calendar would belong to a *person* — who
  could then edit or delete an approved absence they never decided, from a
  screen that knows nothing about leave. An approval that a calendar delete can
  silently undo is not an approval.
- **It is a second source of truth**, and it would drift the first time a
  cancelled request failed to remove its event — the `qty_on_hand` mistake
  again, this time where the drift is somebody being marked absent for a week
  they worked.
- **Events are indiscreet.** A calendar event has a title, and a title in a
  shared calendar is the thing most likely to end up saying "Sick — hospital".
  A derived layer has exactly the fields we chose to expose and cannot acquire
  a fourth by somebody typing into it.

The layer is a read over `hr_leave_requests` joined to `hr_employees`, tenant
bound, `status = 'approved'`, and it never returns the policy, the type or the
note. It is the module's one read that every member gets on other people, and
it discloses precisely what a team needs to plan: *who is not here*.

## Onboarding and offboarding checklists (B6.05)

A checklist template is a list of steps, each with a title, an owner (by role —
*the manager*, *HR*, *IT* — resolved to a person at instantiation), and an
offset in days from the start or end date. Instantiating one for an employee
**creates a real task project** in the existing Tasks module, with the steps as
tasks, assigned and dated, linked back to the employee by the source link ADR
0021 already gives every task (`source_kind = 'hr_employee'`, `source_id`).

*Rejected: an `hr_checklist_items` table with its own status, assignee, due
date and comments.* That is a fifth board in a product that has one, and it
would need its own notifications, its own overdue view and its own mobile
screen. Reusing tasks means an onboarding step arrives where the assignee
already looks, and the only new thing in the schema is a link column.

The account-creation steps ("create the mailbox", "grant the Spaces") are
**tasks, not automation.** Provisioning an account from an HR record is a
capability that turns a badly-scoped HR write into a security incident, and it
is named in the cuts.

#### As built (B6.05), and the six decisions the code had to make

The templates are `platform/alo-store/src/hr_checklists.rs` over migration
`0204` (`hr_checklist_templates` + `hr_checklist_steps`), and the surface is
`products/mail/alo-jmap/src/hr_checklists.rs` — `GET`/`POST
/hr/checklist-templates`, `GET`/`PATCH`/`DELETE /hr/checklist-templates/{id}`,
and `GET`/`POST /hr/employees/{id}/checklists`. Six things the note above left
open were decided while building it:

- **The steps are rows, not a JSON column.** Every step has the same four
  fields, so four columns say what a schema-in-a-string would only imply. (The
  typed-JSON shape used for insight tiles and site sections earns its keep where
  the shape varies by kind.) An edit rewrites them as a block: a checklist is a
  short ordered list, and a per-step diff would be a reordering protocol between
  two screens to save writing sixty rows nobody is racing over.
- **No instance table at all, not even a link row.** A run's *only* record is
  the task board it created, found through the source link the tasks already
  carry — so a person's checklists are folded from `tasks` grouped by project,
  and a board that lost its link row is impossible because there is no link row
  to lose. Progress is `count(done)/count(*)`, never a stored figure: the
  `qty_on_hand` refusal (B5.01), here with somebody's first week in it.
- **DELETE, not archive** — the one place this module departs from the leave
  policies beside it. A balance is only explicable beside the policy that
  produced it; a checklist template explains nothing after the fact, because an
  instance is a *copy*. Deleting a template leaves every board it ever produced
  untouched, and a test says so.
- **A fourth owner role, `employee`, and `it` without a tenant role.** The note
  named manager/HR/IT; reading the handbook and returning the laptop are the
  arriving and leaving person's own steps, so the vocabulary is
  `hr | manager | it | employee`. `it` deliberately has no counterpart in
  `tenant_user_roles`: in a company this size "IT" is often the same person as
  "HR". It is resolved from what the caller states, falling back to whoever drew
  the checklist.
- **Every role falls back to the person drawing the checklist**, and the
  resolution is returned with the run. On the day an onboarding is drawn the
  newcomer usually has no login (that is one of the steps) and their manager may
  be a record without an account. A task assigned to nobody is a task nobody
  does; one on the desk of the person who just drew the checklist is one they
  hand on in a gesture — and they are looking at the screen where a wrong
  assignment is obvious, because the answer names each assignee.
- **A run is refused for an archived record, and never refused for being the
  second one.** Archiving is the *last* act of an employment, after the
  offboarding has been worked through, so drawing a checklist for an archived
  person is either a mistake or work landing on a record nobody opens again. A
  repeat run of the same template for the same person, by contrast, is a rehire
  or a moved start date — refusing it would mean deleting a board to be allowed
  to draw it again.

The kind is not editable after creation: turning an onboarding into an
offboarding silently reverses what every offset in it means. Reading a person's
runs is the leave door (`hr_leave_door.rs`) rather than a second spelling of
"whose record is this?" — HR, their manager, or the newcomer looking at their
own first week.

**Not built here:** the seeded starter templates. A tenant's first template is
created by the client from its own i18n catalogue, because step titles are
strings a person reads and the store must not carry English ones; the
empty-state that offers them belongs to the HR screens (B6.08).

**★ The consequence a human should weigh: a checklist board is a `team` board,
and a team board is visible tenant-wide in v1.** That is right for an
onboarding — the point is that the whole company can see somebody is arriving
and who owes what — and it is a **disclosure for an offboarding**, whose board
name says a named colleague is leaving before anybody has been told. Neither
this module nor Tasks has a narrower shared visibility to put it on today: the
alternatives are a personal board (visible to one of the four people who owe
steps, so the checklist stops working) or per-board membership in Tasks, which
is a Tasks-module change with its own design. Until that exists, the honest
mitigations are the ones a client can apply: name an offboarding board
neutrally, and draw it when the departure is known. Recorded here rather than
solved quietly.

## Recruitment-lite (B6.06)

```
hr_openings     title, team, location, employment kind, status
                ('draft' | 'open' | 'closed'), opened_on, closed_on
hr_applicants   opening_id, name, email, phone, source, stage,
                cv_node_id (Drive), retain_until, created_at
hr_applicant_notes  applicant_id, author_user_id, body, created_at
```

The stage vocabulary is **closed and ordered**: `applied`, `reviewing`,
`interview`, `offer`, `hired`, `rejected`, `withdrawn`. The board is the shared
board interaction the Tasks and CRM kanbans already have, and a move is always
`POST /hr/applicants/{id}/move` — one route, audited, with a person's id on it.

*Rejected: configurable stages per opening, like `crm_pipelines`/`crm_stages`.*
CRM needed them because a sales process genuinely differs by product line and a
tenant's pipeline is their own invention. A hiring process for a company small
enough to be replacing Microsoft 365 with us has seven stages and always the
same seven; a pipeline table would be two more tables, a seeding path and a
migration, to express a preference nobody has yet stated. It stays a cut with
its reasoning, and becomes two tables the day a tenant asks.

Applicant CVs are Drive nodes in the HR-only area, referenced by id and
**never parsed** — see the next section for why that sentence is load-bearing.

## The EU AI Act posture

This is the section a human should read most carefully, because it declines
something `docs/features.md` promises.

### What is not built, and will not be

**No CV screening. No ranking, scoring, shortlisting, matching, "fit"
assessment or automated evaluation of an applicant, in any form — including
suggest-only forms with a human decision after them.** Nothing in the module
reads a CV's contents. Nothing scores a person. The AI tools this module ships
(§ below) cannot see the applicant tables at all.

Regulation (EU) 2024/1689 (the AI Act) classifies as **high-risk**, via Art.
6(2) and Annex III point 4(a), AI systems intended to be used "for the
recruitment or selection of natural persons, in particular to place targeted
job advertisements, to analyse and filter job applications, and to evaluate
candidates". Point 4(b) covers systems used to make decisions on terms of work,
promotion and termination, task allocation, and monitoring or evaluating
performance and behaviour. The obligations attached to Annex III systems apply
from **2 August 2026** (Art. 113) — that is, already.

The derogation in Art. 6(3) does not rescue a screening feature. It exempts
systems performing a narrow procedural task, improving the result of a previous
human activity, detecting decision patterns, or doing preparatory work — but its
final subparagraph says a system is **always** high-risk when it performs
**profiling of natural persons**, and ranking candidates against a job
description is profiling in the plain sense of Art. 4(4) GDPR. Even a provider
who believed the derogation applied must document that assessment before
placing the system on the market and register the system in the EU database
(Art. 6(4), Art. 49(2)) — obligations, not an exit.

Being the **provider** of a high-risk system means: a risk-management system
(Art. 9), data and data-governance requirements on the training data (Art. 10),
technical documentation (Art. 11), automatic logging (Art. 12), transparency
and instructions for use (Art. 13), human oversight designed into the system
(Art. 14), accuracy, robustness and cybersecurity (Art. 15), a quality
management system (Art. 17), conformity assessment (Art. 43), CE marking, EU
database registration (Art. 49) and post-market monitoring (Art. 72). That is a
compliance programme, not a feature flag, and a team of this size cannot
discharge it honestly.

So the strict reading, which CLAUDE.md's standing rule on legal items requires
of this loop ("implement the strict reading of the cited spec; flag any
ambiguity for human review — never guess loosely on compliance"), is: **do not
build it.** The tenant reads the CVs; the tenant decides; we hold the files, the
stages and the notes, and we can prove we never scored anybody.

Two adjacent lines, for completeness:

- **The board itself is not an AI system.** A kanban of applications moved by a
  person, with notes a person typed, meets none of Art. 3(1)'s definition — it
  infers nothing and has no autonomy. Shipping it carries no AI Act obligation,
  and this note says so explicitly so that a future reader does not conclude
  the whole module is compromised by the section it is in.
- **Emotion inference in the workplace is prohibited outright** by Art. 5(1)(f),
  not merely high-risk. Nothing here infers a state of mind, and no future item
  in this module may — sentiment on interview notes, tone analysis of a
  colleague's mail, engagement scoring. It is named here so that the refusal is
  a written decision rather than an oversight.

**This deviates from `docs/features.md`**, which promises screening
"suggest-only with mandatory human decision, every decision logged". The
`[B6]` line needs amending to match; the queue item B6.09 already anticipates
this note's conclusion. Flagged for the human in `docs/autonomy/STATE.md` —
it is a product decision, and the loop does not make product decisions.

### The two tools that do ship (B6.09)

Both go through ADR 0034's propose-then-approve envelope and the allowlist, and
both are verified structurally against the local database — never by a live
model call.

- **`who_is_off`** — an *answer*, not an act: who is away over a stated range,
  from the absence layer, with the same discretion the layer has (names and
  dates, never a reason, never a type). It reads only what the caller could
  already read, and cites the days it counted.
- **`draft_letter_from_template`** — a *draft*: merges an employee's fields into
  a **tenant-authored** letter template (an employment confirmation, a salary
  certificate for a landlord, a reference) and leaves the result in the user's
  Drafts, exactly as the Billing and Inventory agents leave documents. It has
  no free-form generation path: a template the tenant has not written is a
  `422`, not an improvisation. It cannot address a template about somebody the
  caller may not read, and it never composes a decision about a person — the
  decision is the human's, made before they asked.

Neither tool can reach `hr_applicants`, `hr_applicant_notes` or any pay field.
That is enforced by which store functions the executors call, and asserted by a
test that the HR executor module names no applicant or pay symbol at all.

## Payroll export (B6.10)

A per-period CSV of what a payroll bureau needs, **with no calculation
anywhere**: no gross-to-net, no tax, no social contributions, no accruals. That
is not a scope cut for this wave — `ROADMAP.md` records payroll calculation as a
**permanent non-goal**, because it is per-country statutory software with a
compliance obligation per member state and an update cycle we would be signing
up to forever.

The file's columns are the facts we hold: staff number, name, national id, IBAN,
contract kind, working pattern, pay amount/period/currency, and, for the period,
the leave taken per policy (paid and unpaid separated, because unpaid leave is
the one absence that changes what somebody is paid), plus reimbursable expense
and mileage totals already approved in alo Finance (B4).

**A per-country column mapping** sits over that: a named mapping chooses which
columns appear, in which order, under which headings, with which date and
decimal formats — because DATEV, SD Worx, Loket and a Polish accountant's
spreadsheet all want different sheets and none of them will change for us. The
mappings are seed data plus a tenant-defined one; the *data* is the same in
every mapping, which is what keeps this a formatting layer rather than a second
export.

Money stays integer cents internally and is rendered per the mapping's format
on the way out — the one place a decimal separator is a per-country decision
rather than a bug.

## The approvals inbox (B6.07)

One manager view unifying leave (B6), expenses (B4.05b) and timesheets
(B3.05), with counts.

**It adds no server route.** The web composes it: three parallel calls to the
three existing queues, merged client-side, each item deciding through its own
already-gated route. The badge is the sum.

*Rejected: `GET /approvals` aggregating server-side.* It would need to hold
three different role gates in one handler (manager-scope for leave,
`require_finance` for expenses, `require_admin` for weeks), and answer with a
merged shape that gains a reason to change every time any of the three queues
does — a contract with three owners, which is the one-file-one-reason law
broken at the API layer. It would also be a fourth place where "who may decide
this" is decided, and there are already exactly three, each next to the data it
governs.

The client-side merge has one honest cost, named so it is not discovered:
**the three lists page independently**, so a manager with hundreds of pending
items sees "first 50 of each" rather than a single ranked stream. For the
number of approvals a company of this size has, that is not a real limitation;
if it becomes one, it becomes one endpoint with a stated contract, not a fix.

### As built (B6.07)

Server routes added: **none**, as decided above. What shipped is six web files
and two narrow exports:

```
platform/approvals.ts    the shared row shape + queue interface — no client,
                         no rules, no React
projects/approvals.ts    weeks   → the shape (exported as `useWeekApprovals`)
finance/approvals.ts     claims  → the shape (exported as `useExpenseApprovals`)
hr/leaveApprovals.ts     leave   → the shape
hr/queues.ts             which of the three are this caller's to work
hr/inbox.ts              the merge, the counts, and the decision
hr/ApprovalsView.tsx     the screen; hr/ApprovalsWidget.tsx the rail count
```

Five decisions the first build made, each of which could have gone otherwise:

1. **A module hands over rows it has already put into words.** `Approval` carries
   `what`, `detail` and `figure` as finished strings, so Finance's cents are
   formatted by Finance's formatter and a week's minutes by Projects'. An inbox
   that formatted three kinds of record itself would be a fourth place those
   decisions live, and money is the one thing in this suite that never gets a
   second formatter.
2. **Who may decide is asked once and drawn, never enforced.** `queues.ts` maps
   three doors — `canWorkHr` (or a direct report) for leave, `canWorkTheBooks`
   for claims, `isAdmin` for weeks — and a caller with none of them gets an
   empty list and no tab. Every route behind the queues asks its own door again,
   so a stale session hides a queue at worst.
3. **The manager case costs two reads**, and they are the only reads this adds:
   nothing on the session says "somebody reports to you", so a caller who is not
   HR is resolved from `/hr/me` plus the directory, both every-member surfaces.
   The answer is cached per signed-in session (a `WeakMap` on the session's own
   fetch), because the tab, the list and the rail badge all ask for it.
4. **A queue that fails is named, never counted as zero.** The three reads are
   settled rather than raced: an unreachable Finance leaves a manager able to
   decide the leave in front of them, and the screen says the list is short. A
   silently short inbox reads as "nothing is waiting", which is the one wrong
   thing an inbox can say.
5. **HR still lands on Hiring.** The inbox is usually empty, and a module that
   opened on an empty screen would read as a module with nothing in it. A
   manager who is not HR has the inbox as their only tab and lands there.

Cuts, recorded: **reimbursement is not in the queue** (paying somebody back is
not a decision, and it stays on Finance's own screen beside what is owed); the
row links to the owning module rather than opening a record in place, because
the member-facing leave screens are B6.08b; and the badge re-reads rather than
decrements, so it can never disagree with the list it links to.

## The directory and the org chart

### As built (B6.08a), and the six decisions the screen made

Server routes added: **none**. The two reads were shipped and wire-verified at
B6.02b — `GET /hr/employees` (the public projection, every member's, HR's by one
flag wider) and `GET /hr/org` (the tenant's reporting tree, roots first) — and
this item is the surface over them:

```
hr/directory.ts        the search, the manager lookup, the tree narrowing —
                       pure, and the whole of the screen's thinking
hr/DirectoryView.tsx   the screen: toolbar, the people table, the reads
hr/OrgChart.tsx        the tree, drawn — presentational, recursive, no HTTP
hr/directory.test.ts   the pure functions, at their edges
hr/Directory.test.tsx  the promises, against a recorded network
```

1. **It is every member's, and no door is asked for it.** The tab is drawn for
   everybody, the reads carry no role, and the screen is the same screen for a
   warehouse operative and the managing director. This is the tab that ends the
   module's "coming soon" state: somebody with no board and no inbox now lands
   here rather than on a promise.
2. **The projection is the server's, not a filter.** Nothing is stripped in the
   browser, because nothing private is sent: `directory_json` is folded from a
   type that has no home address, no birthday and no IBAN on it. What HR sees
   extra is exactly one thing — the people who have left — and the control for
   it is drawn only when the answer says `hr`, because a control that exists to
   be ignored teaches the wrong thing about what you can see.
3. **The chart is the server's tree, never a fold of `managerId` done here.**
   Somebody whose manager has left is served as a root; re-deriving that in a
   browser is how a branch quietly disappears. The test proves it by serving a
   three-level tree the rows alone would not produce.
4. **One search box over both readings.** The list narrows to the people who
   match; the chart keeps a match **with everybody beneath them** and **with the
   line of managers above them**, because a tree filtered like a list would hang
   somebody under the wrong person and say something false about who they work
   for. Both narrowings are local: the people are read once and typing asks the
   server nothing.
5. **Where somebody sits is a place in the chart, not a filtered chart.**
   Pressing *Where they sit* opens the tree with that person marked and scrolled
   to, among the people around them — the address carries it (`?view=org&person=`)
   so it is a link a colleague can send, and the whole of it is one write to the
   address, because two writes in one act lose each other.
6. **The chart does not collapse.** A company this suite is built for reads its
   structure in one screen, and a chart that opens folded makes finding somebody
   a series of guesses. Depth is a rule down the left of each level, which
   survives a wide branch without becoming a diagram.

Cuts, recorded: **no photos** (the record carries `photoNodeId`; the screen
draws initials, and fetching a Drive blob per row is a screen's worth of
requests for a decoration); **no record opens from a row** — the full record is
the People tab, admin-or-HR, a later item, and this screen deliberately shows
only what a colleague may see; **no export**; and **the leavers view is the
people list only**, because `/hr/org` is the people who are here.

One cost, unchanged and now paid by two readers: a member opening HR reads the
directory twice — once for the approvals resolver's *does anybody report to me*
(B6.07) and once for this screen. The honest fix is still the additive `manages`
count on `/hr/me` flagged there; it is a server change, and this item is a web
item.

## Errors

One map, `billing::map_store_err`, used and not copied — the same call CRM,
Projects, Finance and Inventory made, for the same reason: it is a store-error
map, not a billing rule.

| Condition | Store | Wire |
|---|---|---|
| no or bad token | — | `401` (the `authenticate` extractor) |
| an HR surface without admin-or-HR | — | `403` naming the role |
| a decision on somebody who is not your report, without admin-or-HR | — | `403` |
| employee, employment, policy, request, opening or applicant not this tenant's | `NotFound` | `404` — existence is never disclosed |
| another employee's record through the own door | `NotFound` | `404` — not `403`, which would confirm it exists |
| a private field requested through the directory door | — | absent from the response, never `403` — a refusal per field is an oracle for which fields are filled in |
| manager link that would create a cycle, or exceed depth | `Validation` | `422` naming both people |
| `staff_number` already used in this tenant | `Conflict` | `409` naming the holder's record id, not their name |
| malformed IBAN, unknown country, unknown currency, bad date, `ended_on` before `started_on` | `Validation` | `422` naming the rule |
| pattern minutes negative, or a day over 1 440 | `Validation` | `422` |
| a leave request outside the employment, or in a year the holiday seed does not cover | `Validation` | `422` naming the gap |
| a leave request overlapping an approved one | `Conflict` | `409` naming the covering request |
| an approval that would take a balance negative on a policy that forbids it | `Conflict` | `409` with the shortfall in minutes |
| approving/rejecting a request that is not `requested` | `Conflict` | `409` naming its state |
| a manager approving their own request (non-admin) | `Conflict` | `409` |
| withdrawing a decided request; cancelling leave that has started | `Conflict` | `409` |
| archiving an employee with a request awaiting a decision | `Conflict` | `409` with the count |
| deleting a policy an employment has ever been on | `Conflict` | `409` — archive instead |
| moving an applicant to a stage that is not in the vocabulary | `Validation` | `422` listing the stages |
| editing a closed opening, applying to one, or closing it twice | `Conflict` | `409` naming the state |
| publishing an opening that is not a draft | `Conflict` | `409` naming the state it is in |
| a CV that is not a live node in this tenant's HR area | `NotFound` | `404` — the same answer for another tenant's node, a personal file and a trashed one |
| a retention date more than ten years out | `Validation` | `422` — a slipped digit, not a policy |
| a payroll export over a period with no employments | `Validation` | `422`, never an empty file that reads as "nobody is paid" |
| a document node that is not this tenant's, or not in the HR area | `NotFound` | `404` |
| database error | `Db` | `500`, opaque — the wire never sees a raw error |

Validation messages are authored in the store and name the rule and the field;
they are the one place a message crosses in English today, the standing
cross-cutting item B1.27, B2.14, B4.15 and B5.11 have each left for a human,
and this wave adds no new kind of it — but it adds the sharpest instance,
because a refusal an employee reads about their own leave is a sentence from
their employer's software in a language they may not speak.

## Tenancy

Every statement carries `tenant_id` from the handle, never from request input —
the invariant `for_tenant`/`for_account` make structural rather than
remembered. **Four** tests are mandatory before B6.02a is done, two more than
any previous wave:

- **Wrong tenant** (law 1, every wave): tenant A's handle cannot read, edit,
  archive, approve, export or report on tenant B's employee, employment,
  policy, request, opening or applicant. Clean denial, not data and not a 500.
- **Wrong user** (B3's addition): user B's `AccountStore` cannot read or act on
  user A's leave, balance or record through the own door — a `404`, inside the
  same tenant.
- **Wrong role** (this module's first addition): a member with neither admin
  nor `TenantRole::Hr` gets `403` from every HR-door route and, critically,
  **cannot read a private field through any door** — the test asserts the
  absence of `date_of_birth`, `iban`, `national_id`, `address_line1` and
  `pay_amount_cents` from every response shape a non-HR caller can obtain,
  including the directory list, the org chart, the absence layer and the
  approvals queue. This is the test that would catch the leak this module
  exists to prevent, so it enumerates fields rather than trusting a projection.
- **Wrong manager** (this module's second addition): a manager may decide their
  direct reports' requests and **not** their reports' reports', not a peer's,
  and not their own. Asserted on a three-level chart so the "chain is not the
  door" decision is a fact the tests hold rather than a sentence in this note.

An accountant, specifically, gets `403` from `/hr/*` — asserted, because the
one role that already exists is the one most likely to be assumed sufficient.

### Audit

`hr` joins `audit_action::AUDITED_MODULES` beside `billing`, `crm`, `projects`,
`finance` and `inventory` at B6.02a — a one-word additive change, after which
`tests/audit_routes.rs` requires **every** mutating `/hr/*` route to be audited
by reading the router's own source.

This module has the strongest claim on that trail of any so far. "Who approved
my leave, and when", "who changed my pay", "who opened my record", "who drew
the payroll file" are questions an employee, a works council, a data-protection
officer and an auditor each have standing to ask, and the answer must not be a
reconstruction. Two rules specific to HR:

- **An audit entry never carries a field value.** It names the record, the
  verb, and who. That a pay figure changed is in the log; what it changed to is
  in the record, behind the HR door. An audit log that quotes personal data is a
  second copy of it with different access rules.
- **Sub-resource events file against the parent** (a document against the
  employee, a note against the applicant, a decision against the request), the
  rule B2.13 established, because that is what makes a record's history
  complete.

## Files this wave will add

Store (`platform/alo-store/src`), one file one reason:

```
hr_employees.rs          the person: CRUD, the three projections, archive
hr_employments.rs        the terms, appended: which employment covered a date
hr_org.rs                the chart, the cycle check, the manager resolution
hr_documents.rs          Drive nodes filed against a person, HR-gated
hr_leave_policies.rs     policies, the country seed, archive-not-delete
hr_leave_math.rs         PURE: entitlement, accrual, request cost, balance
hr_leave_requests.rs     the state machine and its three doors
hr_leave_balances.rs     the fold over requests × policies × employments
hr_absences.rs           the derived layer (who is away, never why)
hr_holidays.rs           the seed table, Easter, the per-tenant selection
hr_checklists.rs         templates, and instantiation into a task project
hr_openings.rs           job openings and their two transitions
hr_applicants.rs         applicants, stages, notes, the retention deadline
hr_payroll_export.rs     the period fold + the per-country column mappings
tenant_roles.rs          widened by one value (existing file)
migrations/…             hr_employees, hr_employments, hr_leave_policies,
                         hr_leave_requests, hr_holidays, hr_checklist_templates,
                         hr_openings, hr_applicants, hr_payroll_exports,
                         and the CHECK widening on tenant_user_roles
```

Migration numbers are drawn when each item ships, not reserved here — the next
free number at the time of writing is `0167`, and the sites track mints from the
same sequence.

Routes (`products/mail/alo-jmap/src`):

```
hr_employees.rs          the directory, the record, the archive
hr_org.rs                the chart, and /hr/me
hr_documents.rs          documents on a person
hr_leave.rs              policies and balances
hr_leave_requests.rs     requests and the four decisions
hr_absences.rs           the layer + holidays + calendar selection
hr_checklists.rs         templates and instantiation
hr_recruitment.rs        openings and applicants
hr_payroll.rs            the audited export
agent_hr.rs              executing the two approved tools
state.rs                 `require_hr` (existing file)
scoped_roles.rs          untouched, and a test that says why
```

Model (`platform/alo-ai/src`): `agent_hr.rs` — the two tools' descriptions and
their strict argument envelopes.

Web (`web/src/hr`): `HrModule.tsx`, `api.ts`, `MyLeaveView.tsx`,
`TeamView.tsx`, `DirectoryView.tsx`, `OrgChart.tsx`, `PeopleView.tsx`,
`EmployeeDrawer.tsx`, `PoliciesView.tsx`, `HiringBoard.tsx`,
`PayrollView.tsx`, `LeaveRequestDialog.tsx`, `AbsenceLayer.tsx`,
`hr.module.css`, plus the approvals merge in the shell and `hr*` keys in
`i18n/en.ts`.

## Out of scope for B6 (cuts are decisions)

- **★ Payroll calculation, in any country.** A permanent non-goal, recorded in
  `ROADMAP.md` and restated here. We export; we never compute gross-to-net.
- **★ CV screening, ranking, scoring, matching or any automated evaluation of a
  person.** § *The EU AI Act posture*. Not a scheduling cut — a refusal.
- **Performance management**: reviews, ratings, goals, 360s. Annex III 4(b)
  territory the moment anything infers from it, and a culture product rather
  than an administration one.
- **Time and attendance** (clock-in, shift rotas, overtime rules). alo Projects
  already records hours worked on client work; a rota engine is a different
  product, and the CJEU's *CCOO* working-time-recording obligation is met by
  B3's timesheets for the tenants who need it.
- **Automated account provisioning from an HR record.** An onboarding checklist
  produces a *task* to create the mailbox. A write path from HR into identity
  would make a badly-scoped HR permission into an account-creation capability.
- **The manager chain as an approval path.** Direct reports only; escalation is
  HR. A chain approval is a delegation feature (out-of-office approvers,
  deputies, escalation timers), and half of one is worse than none.
- **Absence notification to the team**, beyond the absence layer being visible.
  No "X is off today" mail, no chat post — an automated announcement of
  somebody's absence is exactly the kind of disclosure § *Sickness* is careful
  about.
- **Working-time compliance checking** (rest periods, maximum weekly hours
  under Directive 2003/88/EC). It is per-member-state law with collective
  agreements layered over it, and a compliance claim we would have to stand
  behind.
- **Scheduled erasure of expired records.** The applicant screen shows what is
  past its retention date; a human deletes. A job that deletes people
  unattended is not something this loop builds.
- **HRIS / payroll-bureau / job-board integrations**, and any import from
  Personio, BambooHR, AFAS or SD Worx. The CSV export is the integration.
- **Benefits, company cars, equipment inventory, training records.** Each is a
  small table and a large screen, and none is in `docs/features.md`.
- **Multi-country employment for one person** (an employee on two contracts in
  two states). The model allows sequential employments, not simultaneous ones;
  a genuinely dual-contract employee is two employee records today, and the
  screen does not pretend otherwise.

## Open questions flagged for a human

- **★ `docs/features.md` promises CV screening and this note refuses it.** The
  `[B6]` line needs amending to say what we actually build, and the refusal
  needs a product owner's signature rather than a design note's. Until then the
  document and the code disagree, and the code is the conservative one.
- **★ The seeded statutory leave entitlements and retention defaults are
  research, not advice.** Each seed row will carry its source in a comment, and
  a human with an employment lawyer should confirm the countries alo actually
  sells into before a tenant relies on a default.
- **★ Whether an employee may see their own pay history** (every employment
  row, or only the current one). The table above grants the current one. A pay
  history is arguably theirs by Art. 15 anyway; making it a screen is a
  decision about what an employer wants to publish, and it is theirs.
- **Works councils.** In several member states (Germany's *Betriebsrat*, the
  Netherlands' *OR*), introducing software that processes employee data
  requires consultation. That is our *customer's* obligation, not ours, but it
  is a sales fact and a documentation obligation: the tenant needs a plain
  statement of what this module holds and who can see it. Naming it here so it
  reaches the product docs rather than a support ticket.
- **Whether the HR role should imply the approvals inbox for expenses and
  timesheets.** Today those are `require_finance` and `require_admin`; an HR
  holder sees leave only. Widening them is a word each, and it is a question
  about how the customer's company is organised, not about our code.
- **The wave gate at the top of this note is still unanswered**, and it matters
  more here than in B2–B5: the first migration of this wave creates the table
  that holds home addresses and pay.
- **The server's refusal sentences are still English in every language** — the
  standing cross-cutting `StoreError` vocabulary item from B1.27, B2.14, B4.15
  and B5.11. This module makes it sharpest: the refusals an employee reads are
  about their own leave.
