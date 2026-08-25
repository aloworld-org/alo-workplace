# Billing UI audit — 2026-08-25

## Scope and representative data

The audit covers every route mounted by `BillingModule`: Customers, Products,
Quotes, Invoices, Recurring invoices, Reports, Business details, quote and
invoice editors, payments, printing, and the customer, product, and schedule
dialogs.

The populated test fixtures exercise an active and archived customer, a priced
service, draft and issued documents, accepted and expired quotes, overdue and
settled invoices, a partial payment, a credit note, recurring schedules, and a
multi-currency VAT period. Empty, loading, filtered-empty, invalid-input,
read-only, rejected-request, and first-run states are covered alongside them.

## Screen-by-screen review

| Screen | Primary controls checked | Data and edge states checked | Result |
| --- | --- | --- | --- |
| Customers | Search, archived toggle, create, edit, archive, restore | Populated, empty, no matches, archived, invalid VAT ID | Clear and complete; shared toolbar and action spacing upgraded |
| Products | Search, archived toggle, create, edit, archive, restore | Populated, empty, archived, European money and VAT input, invalid amount | Clear and complete; shared toolbar and action spacing upgraded |
| Quotes | Search, status filter, create, open, send, accept, decline, expire, convert | Draft, sent, accepted, declined, expired, rejected transition | Complete; document navigation now sits in the upgraded Billing shell |
| Invoices | Search, status filter, create, open, issue, remind, void, credit, print | Draft, issued, overdue, part-paid, settled, void, credit note | Complete; Billing now opens here because invoices are its returning-user task |
| Payments | Record and remove payment | No payments, partial payment, settled invoice, invalid/refused payment | Complete; server totals remain authoritative |
| Recurring | Run due, open, pause, resume, delete | Empty, active, paused, ended, already-raised schedule | Complete; compact actions now retain protected padding |
| Reports | Date range, show, quarter shortcuts, download CSV | Populated currencies, empty period, conversion exception, invalid dates | Complete; shortcut controls now retain protected padding |
| Business details | Identity, accounting, FX, contact, bank, footer, save | First run, populated, dirty, saved, refused save | Fixed a broken accent-surface token and retained the sticky save state |
| Dialogs/editors | Cancel, save, line add/remove, lifecycle actions, keyboard close | Long forms, read-only documents, failed saves, incomplete lines | Existing behavior preserved; shared hierarchy and actions remain consistent |

## Gaps fixed

- Replaced the flat module heading and underline-style tabs with a framed
  Billing identity header and spaced pill navigation.
- Added a concise translated workspace purpose in English, Dutch, and French.
- Changed the module landing route from Customers to Invoices, matching the
  product's own stated returning-user priority.
- Constrained wide list pages to a readable workspace width and upgraded the
  shared list toolbar to a contained premium surface.
- Protected compact Billing action padding from the global button reset.
- Fixed the invalid `bg--soft` class in Business details, which previously
  rendered no icon background.
- Kept all navigation and actions free of underlines in every state.

## Verification

The focused populated-data gate is the eleven Billing and locale test files:
158 tests covering display, controls, API payloads, server refusals, monetary
precision, document lifecycle, payments, printing, reports, and translations.
The production type-check, lint, build, and full frontend suite remain the final
merge gate.
