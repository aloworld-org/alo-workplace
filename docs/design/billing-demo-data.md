# Billing development data

## Screen audit

| Surface | Persisted owner | Large-data behaviour before this change | Demo coverage |
|---|---|---|---|
| Customers and customer editor | `billing_customers` | Client search and archive filter; every row rendered at once | 100 EU customers, including archived and incomplete optional contact fields |
| Price list, product editor and CSV import | `billing_products` | Client search and archive filter; every row rendered at once | 100 goods and services, varied units, rates, prices and archived items |
| Price connections, connect/share dialogs and status cards | Browser `localStorage` only | Client direction/search filters; not tenant-scoped and not shared across browsers | Move to a tenant-scoped database/API collection and seed 100 received/shared connections linked to products |
| Quotes, quote editor, preview, studio and history | `billing_quotes`, lines, designs and `audit_log` | Status filter and client search; every row rendered at once | 100 linked quotes across every supported lifecycle state; rich designs and multiple pricing/content blocks |
| Invoices, editor, preview, payments, reminders and history | `billing_invoices`, lines, payments and `audit_log` | Status filter and client search; every row rendered at once | 120 linked invoices with draft/issued/void/paid, overdue and partial-payment cases; 100 feed the VAT report |
| Recurring billing editor and status list | `billing_schedules` and lines | Every row rendered at once | 100 weekly/monthly/quarterly/yearly schedules with active, paused, future and ended examples |
| VAT report and CSV export | Derived from issued invoices and credit notes | Period query grouped by currency and VAT rate; no source-row navigation | At least 100 issued source invoices across dates, rates and currencies |
| Your details and FX panel | `billing_settings` and `billing_fx_rates` | One tenant singleton, correctly not a list | One comprehensive fictional issuer plus deterministic reference rates |

The customer/product/quote/invoice/schedule/connection collections use deliberate
client pagination after filtering and sorting. Editors continue to address the
row id from the route; opening an existing draft never calls a create endpoint.

## Design

**Surface.** `identityctl billing-demo seed <login-email>` and
`identityctl billing-demo reset <login-email>` resolve the explicit login to its
tenant and acting user, then call a tenant/account-scoped store service. The seed
returns per-feature counts. Price connections gain additive tenant-scoped HTTP
CRUD under `/billing/price-connections`.

**Errors.** The CLI refuses an unknown login, a non-development deployment, a
non-loopback database, or the production database name. Validation/database
failures abort the seed transaction and leave no partial data. Reset deletes
only ids owned by the versioned Billing demo namespace; ordinary Billing rows
are untouched. Existing issuer details and exchange rates win over demo
defaults and survive reset.

**Tenancy.** Every seed/reset and price-connection statement is issued through
`AccountStore` and binds its private tenant id. Composite foreign keys bind
connection products, customers, documents and schedules to the same tenant.
Tests seed two tenants with the same deterministic ids and prove each reads only
its own rows.

**Out of scope.** The seed does not contact supplier APIs, send mail, execute a
recurring run, or issue drafts automatically. Price-connection “sync” updates
the stored health/timestamp only; implementing an external catalogue protocol
requires its own contract and credentials.

The rejected alternative was a set of component mock arrays or a browser-only
fixture: it would not survive reloads, exercise numbering/VAT/storage, or prove
tenant isolation.

## Assets

A few small repository-owned placeholder illustrations are reused by many demo
records. The database stores their paths, never base64 image bodies.

## Operations

Both commands require `ALO_ENV=development`, a loopback `DATABASE_URL`, and the
preserved local database name `alo` (test databases are accepted only when
`ALO_ENV=test`). Production builds contain the code so tests can cover it, but
the runtime guard makes the operation unavailable against production.
