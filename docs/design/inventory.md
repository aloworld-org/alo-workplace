# Design note — alo Inventory (purchasing, stock, and the two chains that move it)

Status: **design** (B5.01) · ADR 0035 · Business track wave B5

alo Inventory is the fifth Work OS module and the first one whose records
are not the truth. A customer is what our table says a customer is; an
invoice is what we issued; a journal entry is an accounting fact because we
posted it. A **stock level is not any of those things** — it is a claim
about a shelf in a room, and the shelf does not read our database. Every
decision below follows from taking that seriously: the software's job is not
to hold the quantity, it is to **explain** it.

That gives the module its one central rule, and everything else in this note
is a consequence of it:

> **A quantity is never written. Only movements are written.**
> On-hand is what the movements add up to, and every movement says where the
> goods came from and where they went.

The rejected alternative, stated once so the rest of the note can lean on
it: a `qty_on_hand` column on the product-location pair, edited in place,
with a movement table written beside it as an audit trail. It is the obvious
design, it is what most small systems do, and it is wrong for the reason two
sources of truth are always wrong — they drift, silently, and the drift is
discovered months later at a stocktake with no way to tell which of the two
was lying or when it started. Here there is one source, the movements, and
the cached figure this note does keep is provably a fold of them, checked by
a test rather than trusted.

The parallel with wave B4 is exact and deliberate. The ledger stores
postings and derives balances; this module stores moves and derives on-hand.
The ledger's invariant is that debits equal credits; this module's is that
every move has a source and a destination, so **the quantity of a product
summed over every location, real and virtual, is always zero**. A reader who
has the finance note in their head already has half of this one.

> **Wave gate, flagged for a human.** `ROADMAP.md` gates wave B2 on "B1 live
> with ≥1 real tenant", and B1, B2, BI-1, B3 and B4 are all code-complete
> and undeployed. This note is design work, which is what belongs ahead of an
> unmet gate; **B5.02 is the first item that writes a migration**, and a human
> should confirm or move the gate before it ships. Recorded in
> `docs/autonomy/STATE.md` rather than decided here.

## Surface

- **Inputs:** authenticated workspace users driving `/inventory/*` on
  `alo-jmap` — suppliers and their prices, locations, stock moves and manual
  adjustments, purchase orders and their receipts, sales orders and their
  deliveries, reorder rules, and stocktakes. The catalog fields this wave adds
  (SKU, barcode, kind, purchase price, photo) arrive through the **existing**
  `/billing/products` routes, additively; there is no second door onto the
  price list. The Inventory agent (ADR 0034, item B5.10) is a second caller of
  the same store functions, never of a parallel code path.
- **Outputs:** JSON resources; CSV for the stock and shortage lists; a printed
  PDF for a purchase order and a delivery note, rendered by the machinery
  B1.16/B1.17 already built; **draft** bills in alo Billing when goods are
  received; **draft** invoices in alo Billing when goods are delivered;
  **proposed** purchase orders when the agent answers "what needs
  reordering" — which are not orders until a human approves them.
- **Who calls it:** `web/src/inventory` (the module UI, B5.09) calls
  `alo-jmap`; the `alo-ai` inventory module produces propose-then-approve
  envelopes that `alo-jmap` executes. Nothing external calls Inventory. In
  particular **no supplier and no carrier system talks to us** — EDI is named
  in the cuts below.

`/inventory` is a **new top-level route prefix**: the production Caddyfile
needs it added at the next deploy, the same standing human action `/billing`,
`/crm`, `/insights`, `/projects` and `/finance` carry, and it must join
`API_PATHS` in `web/vite.config.ts` in the same item that registers the first
route or every call 404s into the dev SPA (the lesson S1.11, BI1.04, B3.04 and
B4.05b have each now paid for). To be noted in STATE.md at B5.04a, not touched
by the loop.

The prefix doubles as the SPA path, exactly as the other five do: the dev
proxy bypasses itself for HTML navigations, so one word serves the API and the
router without a second name to keep in sync.

### Routes

All under the authenticated `alo-jmap` router, following the convention
`/billing/*` established and `/crm/*`, `/projects/*` and `/finance/*`
confirmed: the `authenticate` extractor, typed `Problem` errors, the
store-error map in [`billing::map_store_err`] reused rather than copied,
registration in `server.rs`.

Two shapes are constrained rather than chosen. **The collection is always the
second segment**, because `audit_action::event_for` derives a record's history
mechanically from the matched route template, and `tests/audit_routes.rs`
fails the build for a mutating route it cannot derive an action for — the rule
B3 learned by renaming four routes after the fact. And **a sub-resource event
files against its parent record** (a receipt against its purchase order, a
counted line against its stocktake), the rule B2.13 established, because that
is what makes a record's history complete.

| Route | Purpose |
|---|---|
| `GET/POST /inventory/suppliers` | the supplier list and a new one (B5.03) |
| `GET/PATCH /inventory/suppliers/{id}` | one supplier |
| `POST /inventory/suppliers/{id}/archive` | stop buying from them without deleting the orders that name them |
| `GET /inventory/suppliers/{id}/products` | what this supplier sells us: their price, their code, their lead time (B5.03) |
| `PUT/DELETE /inventory/suppliers/{id}/products/{product_id}` | set or remove one such offer. Idempotent `PUT`, so a form saves in one call |
| `GET/POST /inventory/locations` | the places stock can be (B5.04a) |
| `GET/PATCH/DELETE /inventory/locations/{id}` | one location. `DELETE` only while it has never carried a move; after that it archives, because a location's name is part of the explanation of a movement |
| `GET /inventory/stock?product_id=&location_id=` | on-hand: the derived figure, per product, per location, with the value at purchase price beside it. CSV twin |
| `GET /inventory/moves?product_id=&location_id=&from=&to=` | the ledger itself, newest first — every movement with its reason and the document that caused it |
| `POST /inventory/moves` | the one route that writes a movement **directly**: a transfer between two of the tenant's locations, or an adjustment against a virtual one, with a reason code (B5.04b). Every other movement in the system is a consequence of a document |
| `GET/POST /inventory/purchase-orders` | the order list (with a status filter) and a new draft (B5.05a) |
| `GET/PATCH/DELETE /inventory/purchase-orders/{id}` | read; edit or delete **while draft only** |
| `POST /inventory/purchase-orders/{id}/send` | write the covering mail draft with the PO attached, and move the order to *sent* (B5.05a) |
| `POST /inventory/purchase-orders/{id}/cancel` | stop expecting the goods |
| `GET /inventory/purchase-orders/{id}/print` · `/pdf` | the printed order, reusing B1.16/B1.17 |
| `GET/POST /inventory/purchase-orders/{id}/receipts` | what has arrived, and booking an arrival: the movements in, and the draft bill (B5.05b) |
| `GET/POST /inventory/sales-orders` | the order list and a new draft (B5.06a) |
| `GET/PATCH/DELETE /inventory/sales-orders/{id}` | read; edit or delete while draft only |
| `POST /inventory/sales-orders/{id}/confirm` · `/cancel` | promise it, or stop promising it |
| `GET/POST /inventory/sales-orders/{id}/deliveries` | what has gone out, and shipping some of it: the movements out and the delivery note (B5.06a) |
| `GET /inventory/sales-orders/{id}/deliveries/{did}/print` · `/pdf` | the delivery note as paper for the box |
| `POST /inventory/sales-orders/{id}/invoice` | raise a **draft** invoice in alo Billing for what has been delivered (B5.06b). No body: what may be billed is what shipped |
| `GET /inventory/sales-orders/{id}/invoices` | what has been billed against this order — each document, where it has got to, and the quantities it carried (B5.06b) |
| `GET/POST /inventory/reorder-rules` | the minima, per product per location (B5.07) |
| `PATCH/DELETE /inventory/reorder-rules/{id}` | change one, or stop watching |
| `GET /inventory/shortages` | the computed answer: everything at or under its minimum, with the quantity that would bring it to target and the supplier who sells it. CSV twin |
| `GET/POST /inventory/counts` | stocktakes, and opening one for a location (B5.08a) |
| `GET/PATCH /inventory/counts/{id}` | the count sheet with its variances |
| `PUT /inventory/counts/{id}/lines/{product_id}` | record what was actually on the shelf |
| `POST /inventory/counts/{id}/apply` · `/cancel` | turn the variances into adjustment movements (B5.08b), or walk away |

Nine path segments are reserved words under `/inventory` — `suppliers`,
`locations`, `stock`, `moves`, `purchase-orders`, `sales-orders`,
`reorder-rules`, `shortages`, `counts`. Ids are base64url'd 16-byte random
tokens (`id.rs`), so a record can never *be* one of them, and matchit prefers
a static segment to a capture; this is the shape `/tasks/labels` beside
`/tasks/{id}` and `/projects/time` beside `/projects/{id}` already have.

Multi-word segments are kebab-case (`purchase-orders`), matching
`/billing/invoices/{id}/credit-note`, `/crm/deals/{id}/next-steps` and
`/drive/base-tables`.

One route in the table is a deliberate deviation from billing's spelling and
is called out here so a reviewer does not have to find it: **`POST
/inventory/purchase-orders/{id}/send` both writes the mail draft and moves the
state**, where billing keeps the two apart (`/billing/quotes/{id}/send` is a
lifecycle transition that touches no mail; `/billing/invoices/{id}/send`
writes a draft and changes nothing). The reason is that a purchase order's
*sent* state means precisely "we have asked them" — the mail is not a
notification about the transition, it **is** the transition — and splitting it
would let a tenant have an order marked sent that nobody ever sent, which is
the state that makes a shortage report lie. The mail still only ever reaches
Drafts (below).

### Web surface

`web/src/inventory`, the module pattern Billing, CRM, Insights, Projects and
Finance share: one `InventoryModule.tsx` owning the tab layout, one `api.ts`
owning the fetches, one view per screen, `.module.css` for layout, `ds` tokens
for everything else, and every string through `i18n/en.ts` under an
`inventory*` prefix (fr/nl at the wave review, B5.11). Every screen obeys
`docs/design/ux-principles.md` — the empty states are the onboarding, undo
beats confirm, and an error repeats the server's sentence rather than
inventing a friendlier one.

- **Catalog** (B5.09a) — the product list seen as things rather than as
  prices: SKU, barcode, photo thumbnail, stocked-or-service, and on-hand
  across all locations. It reads `/billing/products` for the record and
  `/inventory/stock` for the quantity, and its editor is billing's product
  dialog with the new fields added — not a second product form.
- **Stock** (B5.09a) — on-hand by location, with a product's movement history
  behind a click. The history is the screen that makes the model
  comprehensible: every row says *from → to, how many, why, and which
  document*, and a person who reads it once never asks why they cannot type a
  new number into the quantity field.
- **Purchasing** (B5.09b) — the purchase-order list and editor, the send
  action, and the receive sheet (ordered vs already received vs arriving
  now).
- **Sales orders** (B5.09b) — the same three shapes for the outbound chain,
  plus the invoice hand-off, which opens in Billing because Billing owns the
  document from that moment.
- **Shortages** — the reorder view: what is short, how much to buy, from whom,
  and a button that drafts the orders. It is also the agent's screen: the
  proposal it makes is this list with the orders pre-filled.
- **Stocktake** — open a count for a location, work down the sheet on a phone,
  see the variance, apply.
- **Scanning** (B5.09c) — a barcode input that accepts a keyboard-wedge
  scanner (which types digits and an Enter, so it needs no permission and no
  camera) and, where the browser allows it, the phone camera. Both resolve to
  the same "find the product with this barcode" call, and the camera is the
  fallback rather than the headline because a warehouse's actual hardware is a
  wedge scanner.

## The catalog (B5.02)

`billing_products` already exists (B1.04): name, unit, unit price in cents,
VAT rate in basis points, archived-or-not, tenant-wide. Wave B5 needs five
more facts about the same rows — an SKU, a barcode, whether the thing is
*stocked* or a *service*, what it costs us, and a photo.

**The decision: extend the table, additively. Do not create a sibling.** The
rejected alternative was `inv_items`, keyed by product id, holding the
inventory facts and joined when needed. It is tempting because it keeps the
billing migration untouched, and it is wrong here for the reason CLAUDE.md
already gives: *extending an owner beats creating a sibling that half-overlaps
it.* A product is one thing in a tenant's head, it has one name and one SKU,
and a two-table split immediately raises the question of what happens when a
row exists in one and not the other — a question with no good answer and three
bad ones. `ALTER TABLE ADD COLUMN` with defaults is an expand-only migration,
which is exactly what our schema rules permit.

So B5.02 adds to `billing_products`:

| Column | Shape | Why |
|---|---|---|
| `sku` | `TEXT NOT NULL DEFAULT ''` | the tenant's own code for the item. Unique **within the tenant** when non-blank; blank is legitimate and unconstrained, because a services business has none |
| `barcode` | `TEXT NOT NULL DEFAULT ''` | the code on the box — GTIN-8/12/13/14, validated (below). Unique within the tenant when non-blank |
| `stocked` | `BOOLEAN NOT NULL DEFAULT false` | whether this product has a quantity at all. Default `false` so every product that exists today stays a service and no existing tenant acquires a stock ledger by upgrade |
| `purchase_price_cents` | `BIGINT NOT NULL DEFAULT 0` | what we pay, in integer cents, in the tenant's own currency. The sale price is `unit_price_cents` and stays where it is |
| `photo_node_id` | `TEXT` (nullable) | a Drive node, referenced by id and never copied, exactly as a receipt is (`fin_expenses.receipt_node_id`, B4.05a) |
| `default_supplier_id` | `TEXT` (nullable) | who we usually buy it from — the seed of a reorder proposal (B5.07) |

**As built (B5.02):** all six columns exist from migration `0154` (renumbered
from `0153`, which the meet track took first), but `default_supplier_id` was
**reserved and not writable**: `inv_suppliers` is B5.03's table, and the
composite foreign key that makes the id necessarily the same tenant's supplier
could only arrive with it.

**As built (B5.03):** the key and the write path arrived together in migration
`0155`. `default_supplier_id` is now writable through `PATCH
/billing/products/{id}` as `defaultSupplierId` (nullable, so a supplier can be
taken off again), and a supplier that is not this tenant's answers `404` — the
same gate the photo goes through. The *picker* is still B5.09a's.

Both unique indexes are **partial and tenant-scoped**: `UNIQUE (tenant_id,
sku) WHERE sku <> ''`. A global unique index on a barcode would be a
cross-tenant information leak of the plainest kind — tenant B's insert failing
because tenant A sells the same book — and it would also be wrong on the
facts, since two businesses legitimately stock the same GTIN.

`stocked` is the flag that decides whether the move ledger will accept the
product at all: a movement of a service is refused with a `Validation` error
naming the product, because "3 hours of consulting moved from the warehouse to
the van" is not a sentence the system should be able to hold. Turning
`stocked` **off** for a product that already carries movements is refused too
(`Conflict`), for the same reason a location that has carried movements cannot
be deleted: the history has to stay explainable.

The photo goes through `drive_require_read` at write time, the gate B4.05a
established: the caller must be able to see the node they are attaching, so a
guessed node id attaches nothing.

### Barcodes — the decision

A barcode is stored as **text**, not as a number, and it is **validated by its
check digit** on write.

Text, because a GTIN's leading zeros are part of it and an integer column
eats them — the classic bug that makes `0012345678905` and `12345678905`
different codes on the box and the same row in the database.

Validated, because the check digit exists precisely so that a mistyped or
misread code can be rejected at the point of entry rather than discovered when
the wrong item ships. The validator is a pure function over the digits
(GTIN-8, -12, -13 and -14: sum the digits with alternating weights 3 and 1
from the right, the total must be a multiple of 10), unit-tested against known
good and known-bad codes, and it has the shape `vat_id.rs` (B1.03) already
established — characters in, a verdict out, no store handle and no door. An empty barcode is always
allowed: plenty of stock has no barcode at all.

Rejected: accepting any string. It costs nothing to type and it makes the
scan-to-find call unreliable forever, because one bad row means a scan can
match the wrong product.

## Suppliers (B5.03)

`inv_suppliers` is a new table, and the interesting part is what it does *not*
do.

`billing_bills` already carries a `Supplier` **copied onto the document** —
name, VAT id, address, IBAN — with a comment saying the master record is
B5.03. That copy stays. A bill must remain readable exactly as it arrived,
years later, whatever has since happened to the supplier record, which is the
same snapshot rule a document line has held since B1.06. **B5.03 does not add
a foreign key from a bill to a supplier**, and this note records that as a
decision rather than an omission: the link that matters (which supplier, for
grouping and reporting) is recoverable through the same `Supplier::key()` the
duplicate constraint already uses, and a nullable FK that is sometimes filled
would give reports two ways to answer the same question. If a later wave wants
the link, it is an expand migration on `billing_bills` with a backfill, which
is a decision to make with real data in front of us.

**Rejected alternative: reuse `billing_customers` with a `supplier` flag.**
The two records genuinely overlap — name, address, VAT id, country, payment
terms — and a single "company" table is a real design that real products
ship. It is rejected for two reasons, one structural and one about
consequences. Structurally, the fields that matter diverge immediately: a
customer has payment terms *we* grant and an invoice address; a supplier has
lead times, their own code for our products, and an IBAN we pay into.
Consequentially, one flagged table means a mistake in the flag puts a supplier
in the customer picker of an invoice, and the failure mode of that is
invoicing a supplier. Two tables cannot make that mistake.

`inv_suppliers` holds: name (required), VAT id (validated by the B1.03
validator, blank allowed), legal id, address lines, postal code, city, country
(ISO 3166-1 alpha-2), email, phone, IBAN, default currency, payment terms
days, default lead-time days, note, and `archived_at`. Archived, never
deleted, for the reason every other master record in this codebase is: an
order that names them must stay explainable.

`inv_supplier_products` is the price list *they* quote us, keyed
`(tenant_id, supplier_id, product_id)`: their own article code, purchase price
in integer cents, currency, minimum order quantity, and lead-time days
overriding the supplier default. It is what makes a reorder proposal able to
say "buy 40 from Hoffmann at €3.15 each, here in nine days" instead of "you
are short 40".

Prices here are a **reference**, not a snapshot: a purchase-order line copies
the price at the moment it is drafted, the same rule `billing_line` holds
about the sale price, so re-negotiating with a supplier never rewrites an
order that was already placed.

**As built (B5.03).** Migration `0155_inv_suppliers.sql`; store
`inv_suppliers.rs` and `inv_supplier_prices.rs`; routes
`inventory_suppliers.rs` and `inventory_supplier_prices.rs`. Six things the
note did not settle, decided here:

- **Country is required on a supplier**, exactly as on a customer. It decides
  which member state's rules the VAT id is judged by and whether the purchase
  is reverse-charged, so an id that arrives without one could only be judged
  against a guess. The refusal is reported *before* the VAT id, since the id
  can only be judged once the country is known.
- **The quoted minimum order quantity is in milli-units** (`min_order_qty_milli`),
  the thousandth-precision quantity a document line already carries (B1.06), so
  "half a kilo" is `500` and no fraction is ever a float.
- **The offer's lead time is nullable and the fallback is server-side.**
  `null` means "as the supplier says"; the read answers both `leadTimeDays` and
  `effectiveLeadTimeDays`, so the fallback lives in one place and no client can
  get it wrong.
- **`PUT` states the whole offer, it does not merge.** The resource *is* the
  offer, and a partial `PUT` would leave a price and a currency disagreeing
  about which quote they belong to. `PATCH` on the supplier itself does merge,
  as every other master record's does.
- **Deleting an offer is allowed** where deleting a supplier is not: an order
  already placed copied the price onto its line, so nothing that has happened
  depends on the row. `DELETE` on an offer that is not there is a `404`.
- **`billing_products.default_supplier_id` is writable from this item.** The
  composite key `(tenant_id, default_supplier_id) → inv_suppliers` arrives with
  the table, and the store checks the supplier is the tenant's own before the
  write, so a guessed id links nothing and answers `404`. The key names its
  column in `ON DELETE SET NULL (default_supplier_id)` — the plain form would
  try to null `tenant_id`, which is part of the key.

Two deliberate non-additions, so a reviewer does not go looking: **no
uniqueness rule on a supplier's name** (two branches of one group are two rows,
and a name is not an identifier), and **no `/inventory` audit trail yet** —
`inventory` joins `audit_action::AUDITED_MODULES` with the first stock write
(B5.04b), which is where the note already placed it and where the abusable
write actually is. It joined there, and the supplier routes above joined with
it.

## Locations and the move ledger (B5.04)

### Locations

`inv_locations`: code (short, unique within the tenant), name, kind, and
`archived_at`. A tenant that ships from one room needs exactly one row, and
it is seeded **on first use, not in the migration** — the shape
`fin_accounts` (the default chart) and `crm_pipelines` (the default board)
both have, recorded once in a seeds ledger so a tenant who deletes what they
were given does not find it back the next morning. The names come from the
caller in the reader's language, as the chart's do, because a warehouse called
`Warehouse` in a Dutch tenant is a hardcoded English string in a European
product. An empty state that is a working state: the interface law about
onboarding, applied to a table.

The `kind` is the load-bearing column, and it has two values a person picks
and four the system owns:

- **`stock`** — a real place: a warehouse, a shop floor, a van. On-hand here
  is a claim about physical goods and may never go negative (below).
- **`transit`** — a real place, too, but one nobody counts: goods that have
  left one location and not yet arrived at another. A tenant with two
  warehouses gets one; a tenant with one does not need it.
- **`supplier`**, **`customer`**, **`adjustment`**, **`production`** — the
  four **virtual counterparties**, seeded per tenant, not creatable and not
  deletable through the API. Goods received come *from* `supplier`; goods
  delivered go *to* `customer`; a stocktake variance is moved to or from
  `adjustment`; `production` is seeded and unused in B5 (assembly is a cut,
  below) so that the day it is needed there is no migration and no new kind.

### Why virtual locations — the decision

The rejected alternative is nullable columns: `from_location_id` null means
"from outside", `to_location_id` null means "to outside". It is one fewer
concept and it is what a first draft writes.

It is rejected because every single query then has to remember the null.
`SUM(qty) WHERE to_location = X` minus `SUM(qty) WHERE from_location = X` is
correct with virtual locations and quietly wrong with nulls the first time
somebody writes `WHERE from_location IS NOT NULL` for an unrelated reason. And
the invariant below — the property that makes the whole ledger checkable —
simply cannot be stated with nulls: "every quantity sums to zero across all
locations" is a sentence about a closed system, and nulls are the hole in it.
This is exactly the argument B4 made for booking a document against a real
account rather than "and the other side went somewhere".

Both columns are therefore `NOT NULL` and both reference `inv_locations`,
composite-keyed on `(tenant_id, id)` so a movement can never reach across
tenants even by a bug in the store.

### The move

`inv_moves`, append-only, one row per movement:

| Column | Shape |
|---|---|
| `id` | opaque, unique within the tenant |
| `product_id` | the product moved — must be `stocked` |
| `from_location_id`, `to_location_id` | both required, and different |
| `qty_milli` | quantity in **milli-units**, strictly positive |
| `reason` | a closed vocabulary: `purchase`, `sale`, `transfer`, `adjustment`, `count`, `return_in`, `return_out` |
| `note` | free text, bounded — what a person typed about a manual adjustment |
| `ref_kind`, `ref_id` | the document that caused it (`purchase_order`, `sales_order`, `count`, or none for a manual move) |
| `occurred_at` | when it physically happened |
| `created_by`, `created_at` | who recorded it, and when |

**Quantities are milli-units** (1.5 kg = 1500), the same representation
`billing_line` chose for a document quantity and for the same reason: a third
of an hour and a kilo-and-a-bit are both exact, no float touches money or
measure, and a purchase-order line and the movement it produces speak the same
units without a conversion between them. The bound is the one
`billing_line` already carries — |qty| ≤ 10^9 milli-units — which keeps every
sum four orders of magnitude below `i64::MAX`.

**Quantity is strictly positive and direction is expressed by the pair of
locations.** Rejected: a signed quantity with one location column. Signed
quantities make "how much moved" a query about absolute values, make the
zero-sum property harder to state than to hold, and reintroduce exactly the
sign confusion the finance note spent a section on.

The table is append-only: **no update route and no delete route will exist.**
A movement recorded in error is corrected by a movement in the other
direction, with reason `adjustment` and a note — the same discipline the
journal has, one layer down, and for the same reason: what happened, happened,
and the correction is itself a fact worth keeping.

### The invariant, and how it will be proven (B5.04a)

> For every product, the sum of `qty_milli` over all movements **into** every
> location minus the sum over all movements **out of** every location is
> exactly zero.

It is true by construction — every row contributes `+q` to one location and
`−q` to another — which is the point: it makes a whole class of bug
*impossible to write* rather than merely tested for. The property tests it
buys, mirroring `fin_journal`'s ten:

- **P1** A generated month of movements (random products, locations, reasons)
  sums to zero per product across all locations.
- **P2** On-hand at a stock location computed as a fold over the movement rows
  equals the cached figure (below) after every single write — the test that
  makes the cache trustworthy rather than merely fast.
- **P3** On-hand is order-independent: the same set of movements applied in a
  shuffled order yields identical balances.
- **P4** A movement and its reversal leave every balance byte-identical.
- **P5** A purchase order fully received and then fully returned leaves the
  supplier virtual location at zero, and the stock location at zero.
- **P6** No sequence of API calls can produce a negative balance at a `stock`
  location (below).
- **P7** Tenant A applying a whole generated month leaves **every one of
  tenant B's balances and reports byte-identical** — the wrong-aggregate test
  B4 introduced, which a single-row read test cannot catch and a `SUM` that
  forgot its `tenant_id` fails instantly.

**As built (B5.04a), where the note left a choice open.**

- **The seed writes one `stock` location and the four virtuals, not five plus
  transit.** A tenant with one warehouse does not need a transit location and
  a tenant with two creates it themselves, so seeding one would be handing
  everybody an empty place to explain. The seed's *names* come from the caller
  in the reader's language, exactly as the chart of accounts' do; the *codes*
  (`MAIN`, `SUPPLIER`, `CUSTOMER`, `ADJUST`, `PRODUCTION`) are minted in the
  store, because a code is an identifier — `fin_accounts`' split, reused. Every
  one of them is renameable, including the virtuals, since the virtuals are
  found by `kind` and never by code.
- **A location's `kind` is immutable.** Renaming is a tenant's business;
  re-kinding retroactively rewrites the meaning of every movement already
  recorded there, turning a shelf into a counterparty without a quantity
  moving. Refused as `Validation`.
- **A virtual location can be neither archived nor deleted** (`Conflict`), and
  at most one of each exists per tenant — a partial unique index on
  `(tenant_id, kind)` over the four virtual kinds. A receipt that could choose
  between two supplier locations makes every balance on it a half-truth.
- **Archiving a location that still holds stock is allowed.** A shed being
  emptied is archived before the last pallet leaves it; archiving takes it out
  of the pickers, and the movements *out of* it are exactly what must keep
  working. What is refused is deleting a location that has ever carried a
  movement — and the refusal is the database's as well as the store's, because
  `inv_moves` and `inv_stock` reference locations with `NO ACTION` keys rather
  than cascades. A cascade would silently delete history to make a delete
  succeed.
- **The negative-stock check is asked of the *departing* end only**, and only
  when that end is real. The receiving end can only have gone up, so a second
  check there would be dead code with a wrong message in it.
- **The two cached rows are written in a fixed order by location id.** Each
  upsert holds its row's lock until commit, so two concurrent transfers in
  opposite directions between the same two places would deadlock if each took
  its locks in its own order. The concurrency proof is in the suite: six
  simultaneous shipments of a stock of one yield exactly one success and five
  clean `Conflict`s.
- **`inv_stock` rows are written only where something moved.** A stocked
  product that has never arrived anywhere has no row and reads as zero; "we
  have none" and "we have never had any" are the same answer to the question a
  shortage query asks, and a row per product per location per tenant would be
  almost entirely zeros.
- **Stock is valued at the purchase price**, in integer cents, by
  `billing_totals`' rounding convention reused rather than restated — a
  warehouse valued by one rule and a document totalled by another disagree by a
  cent exactly when somebody is reconciling them.
- **P5 is proven in its ledger form, not its purchase-order form.** "Received
  in full then returned in full leaves both ends at zero" is asserted over the
  movements; the same assertion over a real `inv_po` arrives with B5.05b, which
  is the item that can write one.
- **Un-stocking a product that carries movements is refused** (`Conflict`), the
  guard B5.02 deferred until there were movements to count. It is asked only
  when the flag actually changes from stocked to not, so an unrelated edit to a
  moved product is untouched.

### The cached balance, and its single writer

On-hand is derived, but a fold over every movement since the tenant's first
day is not a read a stock screen can do per product per location, so
`inv_stock` caches it: `(tenant_id, product_id, location_id) → qty_milli`,
plus the timestamp of the last movement folded into it.

Three rules keep a cache from becoming a second source of truth:

1. **One writer.** The row is updated only inside `record_move`, in the same
   transaction as the movement, by an upsert of the delta. No route, no other
   store function and no migration writes it. This is the single-writer
   discipline `fin_journal::post` established.
2. **Proven, not trusted.** P2 above recomputes the fold and compares, after
   every write in every test — and a `GET /inventory/stock?verify=1` debug
   read is *not* added, because a discrepancy is a bug for a test to catch,
   not an operational condition for a tenant to inspect.
3. **Rebuildable.** A pure function recomputes every cached row from the
   movements; it is what a future maintenance command would call, and its
   existence is what makes the cache disposable.

**Rejected alternative: a Postgres trigger** maintaining the cache. It would
be a third language in the repo for logic that belongs in the one function
that already exists, it is invisible to `cargo test` and to a code reviewer,
and it checks rows without knowing intent — the same argument, verbatim, that
B4.03a used to reject a plpgsql balance trigger.

**Also rejected: no cache at all**, folding on read with an index on
`(tenant_id, product_id, location_id)`. Honest and correct, and the reason not
to is the reorder query: shortages need on-hand for *every* stocked product at
once, which is a fold over the entire movement history on every page load.

### Negative stock — the decision

**A movement that would drive a `stock` or `transit` location's balance for a
product below zero is refused**, with a `Conflict` naming the product, the
location, the available quantity and the requested one. Virtual locations are
unbounded by construction: `supplier` goes ever more negative as we buy, which
is the correct reading — it is how much has come from outside.

The alternative, permitting negative stock with a warning, is what several
larger systems do, and their reason is real: a warehouse that ships before the
paperwork catches up should not be blocked by software. It is rejected for
this product's actual users. Negative on-hand means the data is already known
to be wrong, and a system that accepts a known-wrong number will be asked to
report on it — at which point the stock value on a balance sheet is negative
and nobody can say why. The escape hatch is not a negative balance; it is the
manual adjustment (B5.04b), which is one call, requires a reason, and leaves a
row that says a person decided this.

Two consequences follow, and both are deliberate:

- **The check is against the current balance, not the balance at
  `occurred_at`.** Back-dating a movement is allowed (paperwork does catch up
  late) but it is validated against what is on the shelf now, because goods
  physics is not retroactive: a shipment that left yesterday cannot be
  recorded today if the stock is not there today.
- **The check serialises per product-location.** The upsert of the cached row
  holds its lock until commit, so two concurrent shipments of the last unit
  queue rather than race, and exactly one of them fails. This is the same
  trade `billing_sequence` made for gapless numbering, and at SME volumes it is
  free. A concurrency test proves it: N parallel attempts to ship a stock of
  one yield exactly one success and N−1 clean conflicts.

### Adjustments and transfers (B5.04b)

`POST /inventory/moves` is the only door that writes a movement without a
document behind it, and it serves two jobs that are the same operation:

- **A transfer**: `stock` → `stock` (or via `transit`), reason `transfer`.
- **An adjustment**: `adjustment` → `stock` for a surplus found, `stock` →
  `adjustment` for a loss, reason `adjustment`, and a **reason code required**
  — damaged, lost, expired, found, sample, internal use, correction. The codes
  are a closed list because "why is stock disappearing" is a question with a
  small number of real answers and a free-text field answers it with the empty
  string.

The route refuses any move that names a `supplier` or `customer` virtual
location: those two are reachable only through a receipt or a delivery, so a
purchase can never be booked without an order behind it. That refusal is what
keeps the three-way match meaningful.

**As built (B5.04b), where the note left a choice open.**

- **The reason code is a column of its own** (`inv_moves.reason_code`,
  migration `0158`), not a word inside the note: a closed vocabulary that a
  screen filters and a report groups by cannot live in free text.
  `alo_store::inv_adjust::AdjustReason` spells the seven, and the pairing rule —
  **a code is present exactly when `reason` is `adjustment`** — is enforced in
  `inv_moves`' `normalize`, at the moment of writing, so a receipt written by
  B5.05b cannot carry one and an adjustment cannot arrive without one. The
  database carries the same rule as a `CHECK … NOT VALID`: it binds every row
  written from here on, and does not re-read a ledger that is append-only and
  already correct by construction (validating history instead would fail the
  migration on any database that ran B5.04a's property suite, and rewriting
  those rows to please a constraint is the destructive DDL this module refuses).
- **The manual door is its own store function and its own input type.**
  `record_manual_move(NewManualMove)` cannot express a document reference at
  all, so the one door a human reaches can never claim a purchase order stands
  behind a movement no order produced. It writes nothing itself:
  `record_move_in` stays the single writer of the ledger and the cached
  balance, and the door's own rules are one pure function over the two
  locations' kinds.
- **Coherence between the reason and the places is enforced**, beyond the
  supplier/customer refusal the note already stated: a `transfer` has two real
  ends, and an `adjustment` touches the `adjustment` location at exactly one
  end (out of stock for a loss, into it for a surplus). Without it, "why did
  stock disappear" could be filed against a movement where nothing left the
  building. `production` is therefore unreachable through this door, which is
  correct while assembly is a cut.
- **Moving *into* an archived location is refused** (`Conflict`, naming the
  place); moving *out of* one is not. Archiving means "being emptied", and the
  movements that empty it are exactly what must keep working — the other half
  of B5.04a's decision to allow archiving a location that still holds stock.
- **Locations get their HTTP surface in this item**, since a movement cannot be
  written without one: `GET/POST /inventory/locations`,
  `GET/PATCH/DELETE /inventory/locations/{id}` and — an addition to the route
  table above — `POST /inventory/locations/{id}/archive`. Archiving is its own
  door for the reason `/billing/customers/{id}/archive` is: an ordinary rename
  must never be able to drop a warehouse out of every picker because a stale
  form carried the flag. `DELETE` stays strict rather than silently archiving,
  because answering a different request than the one made is worse than a `409`
  that says what to do instead. The seed's names arrive in the caller's
  language through `inventory_location_names.rs`, `finance_chart_names.rs`'
  mechanism reused whole.
- **Query parameters are camelCase** (`productId`, `locationId`,
  `includeVirtual`, `includeZero`, `includeArchived`), as every other route on
  this service spells them; the snake_case in the table above was shorthand.
  `from`/`to` on the ledger read are RFC 3339 instants and unreadable text is a
  `422` — a history page that quietly answers "everything" when it was asked
  for "since Monday" is worse than one that says the date was unreadable.
- **`inventory` joined `audit_action::AUDITED_MODULES`** here, as planned, and
  with it the existing supplier routes: nine mutating `/inventory/*` routes now
  resolve to an action, and `tests/audit_routes.rs` holds every one added after
  them to the same promise. Registering `PUT /inventory/suppliers/{id}/products/
  {product_id}` as `axum::routing::put(…)` had hidden it from that test's
  source reader; it is spelled `put(…)` now, and the tenth line is in the
  vocabulary.
- **`inventory` also joined `scoped_roles::READ_ONLY_FOR_ACCOUNTANT`**, which
  the note had not settled. An accountant values the stock on a balance sheet,
  so they must see the shelves and the ledger; a stock adjustment is the write
  that can make theft look like paperwork, and it is not a books-only role's to
  make.

## Purchase orders (B5.05)

### The state machine

```
draft ──send──▶ sent ──receipt(partial)──▶ partially_received ──receipt(rest)──▶ received
  │               │                              │
  └─────────────cancel───────────────────────────┘
```

- **draft** — editable: lines, supplier, dates, everything. Deletable.
- **sent** — we have asked them. The order is frozen except for cancelling and
  receiving; changing a line after the supplier has the paper would make our
  copy disagree with theirs, and the correction for that is a new order.
- **partially_received** — some quantity of some line has arrived. Entered
  automatically by a receipt, never by a person.
- **received** — every line's received quantity equals its ordered quantity.
  Terminal.
- **cancelled** — from `draft` or `sent`, and from `partially_received` only
  when the tenant explicitly accepts a short delivery as final (the route
  takes `short_close=true`, so it is a decision and not a slip). Terminal.

Transitions are validated in the store and an invalid one is a `Conflict`
naming the current status — the shape `billing_quotes` (B1.11) uses and whose
allowed-transition tests are the model for these.

### The number

A purchase order carries `PO-YYYY-NNNNN`, drawn from the **existing**
`billing_sequences` row-locked counter with a new kind `purchase_order`,
beside `invoice` and `quote`.

The nuance, recorded because it will otherwise be re-litigated: a PO number is
**not legally required to be gapless**. Nothing in §14 UStG or its equivalents
says anything about it. We reuse the gapless machinery anyway, because it
exists, it is tested to 100 parallel iterations (B1.08), and its cost —
serialising per tenant at the moment of numbering — is the same cost we are
already paying for invoices. Writing a second, weaker numbering mechanism to
save a row lock would be a new thing to get wrong.

The number is drawn **when the order is sent, not when the draft is
created** — the rule an invoice already follows, and for the transposed
version of the same reason: a number is what the counterparty quotes back, and
a draft nobody sent should not consume one.

### Sending (B5.05a)

`POST /inventory/purchase-orders/{id}/send` renders the order to PDF through
the machinery B1.16 (a print view under the tenant's own branding) and B1.17
(the PDF path chosen there), composes a short covering email to the supplier's
stored address, and **saves it as a draft** in the caller's Drafts folder. It
does not send.

That is ADR 0034's standing rule, and B1.18's implementation of it, applied
unchanged: anything this product writes on a user's behalf lands where they
can read it, change it and send it themselves through the one submission path
that signs, records and audits. The recipient is the supplier's stored address
and is **not** a request field — a request must not be able to choose where a
purchase order goes. The author is the caller's canonical address. The
attachment is rendered here, now, from the stored order, never uploaded and
never referenced by a client-supplied id.

The order moves to `sent` in the same transaction that writes the draft. The
mail draft is a fact about the user's mailbox; if writing it fails, the
transaction rolls back and the order is still a draft, which is the honest
state.

### As built (B5.05a — the record; B5.05a2 — the sending)

**B5.05a shipped the order and its life up to the point of sending**, and the
send was split into its own item rather than half-built. The reason is the rule
two paragraphs above: sending is one act — the number, the date, the paper and
the covering mail draft together — and the paper needed a generalisation of the
print/PDF machinery (B1.16/B1.17 rendered a `PrintDocument` whose counterparty
was a `billing_customers::Customer`; a purchase order's is a supplier). Doing
that generalisation badly, or in a hurry alongside the record, is how a route
appears that moves an order to *sent* without writing the mail — precisely the
state this note says makes a shortage report lie.

**B5.05a2 shipped the sending, and the generalisation it needed.** Five
decisions, recorded because each is the kind that gets re-litigated:

- **The party is a struct, not a record.** `billing_print::Party<'a>` is the
  eight facts a printed document needs about whoever it is *to* — name, four
  address lines, country, VAT id, and the address a covering letter goes to.
  `PrintDocument.customer: &Customer` became `PrintDocument.party: Party<'a>`,
  and whoever holds the stored record builds the party from it
  (`Party::customer` for billing, a literal in `inventory_po_print.rs` for a
  supplier). The renderers no longer know which record a party came from, which
  is what stops a second document type meaning a second renderer.
- **What differs between document types lives on the kind.**
  `DocumentKind::PurchaseOrder` answers the questions both renderers ask:
  which words label the two dates (order date, expected delivery), whose
  reference the reference is (ours, not theirs), what the party heading says,
  what the closing block is called (Delivery, not Payment), and — the one with
  teeth — `prints_bank_details()`, false for everything but an invoice. Our own
  IBAN on an order we placed would be an invitation to pay ourselves. The
  closing *sentence* moved into one shared function, since the page and the PDF
  had been computing it twice from the same match.
- **The covering letter is its own module.** `document_mail.rs` holds the
  `MailStrings` tables, the recipient rule, the subject, the body and the
  `Outgoing` builder; `billing_send.rs` and `inventory_po_send.rs` are two
  routes over it. B1.18's file had quietly become "the machinery *and* the
  invoice route", which is the second responsibility Law 3 says to split at the
  moment it is discovered. The refusal wording follows the document:
  `DocumentKind::party_noun()` makes the missing-address `422` say *supplier*
  on an order, because "this customer has no email address" would send a buyer
  to the wrong screen.
- **Atomicity is a callback inside the transaction.**
  `AccountStore::send_inv_purchase_order(id, letter)` locks the order, refuses
  anything that is not a draft, refuses one with no lines, draws the number,
  writes `status`/`number`/`ordered_date`, reads the order back *through the
  same transaction*, and only then calls `letter` — the route's closure, which
  renders the PDF and writes the draft. A letter that fails rolls the whole
  placement back, and the row-locked counter gives the number back rather than
  leaving a hole. The store takes the caller's error type (`E: From<StoreError>`,
  which is why `From<StoreError> for Problem` now exists and `map_store_err` is
  a one-line delegation to it), and the callback is handed an owned copy of the
  order rather than a borrow into the open transaction — a borrow that has to
  outlive nothing is one lifetime puzzle nobody needs to solve at a call site.
  The one crack, stated rather than hidden: the draft is written on its own
  connection, so a commit that fails *after* it was written leaves a draft
  email for an order that is still a draft. Visible, harmless, correctable. The
  opposite is the state this design refuses.
- **The pre-flight is not the authority.** The route refuses a non-draft, a
  supplier with no address and an account with no send address *before* the
  placing transaction opens, so the ordinary mistakes never draw a number and
  never render a PDF. Every one of those is re-decided under the row lock by
  the store, which is what makes a send that raced another send lose cleanly.

`inventory_po_receipt.rs` (B5.05b) is still the door that writes the states
beyond `sent`, and they remain in the vocabulary, the transition table and the
database's CHECKs until it arrives.

Migration `0160_inv_purchase_orders.sql` (B5.05a2 needed none: a new numbering
series is a row in `billing_sequences`, whose `kind` CHECK already admits
`purchase_order`); store `inv_po.rs` (the record and `PoStatus`),
`inv_po_lines.rs` (the line) and `inv_po_send.rs` (the placing transaction);
routes `inventory_po.rs` (`GET/POST /inventory/purchase-orders`, `GET/PATCH/DELETE
/inventory/purchase-orders/{id}`, `POST /inventory/purchase-orders/{id}/cancel`),
`inventory_po_print.rs` (`GET …/{id}/print`, `GET …/{id}/pdf`) and
`inventory_po_send.rs` (`POST …/{id}/send`), over the generalised
`billing_print.rs`/`billing_pdf.rs` and the new `document_mail.rs`.
Six things the note had not settled, decided here:

- **The line is a `billing_line` plus a product.** The same five fields, the
  same rules, the same validator — and a nullable `product_id` beside them. A
  line that names a product is goods, so its quantity must be **positive** (it
  becomes a movement into stock); a line that names none is a charge in words —
  freight, packaging, a discount — and may be negative, the latitude a billing
  line has. `ON DELETE SET NULL` on the product link: deleting a catalog item
  must never delete the record of having ordered it.
- **The product link is a reference; everything else on the line is a
  snapshot.** Receiving has to name a product the stock ledger knows, so that
  one field cannot be a copy; the description, unit, price and rate are copied
  at the moment the line is drafted, so re-negotiating never rewrites an order
  already placed.
- **A cancelled order may be unnumbered.** Cancelling is legal from `draft`, so
  the "a placed order has a number" CHECK is written over the three placed
  states rather than as `status <> 'draft'`. A draft is cancelled rather than
  deleted when the tenant wants the decision on the record; `DELETE` is still
  there for one nobody ever meant.
- **`closed_date` is stamped for both terminal states** — received and
  cancelled — and the CHECK ties it to them, so B5.05b's completion cannot land
  without a date and no later touch of `updated_at` can be mistaken for one.
- **`late` is derived on every read** (`PurchaseOrder::is_late`), like an
  invoice's overdue flag: an order still open whose expected day has passed. An
  unplaced or finished order is never late, whatever the dates say. The
  expected date itself is **not** derived from the supplier's lead time — an
  arrival date for an order that has not been placed would be a date about
  nothing — and a date in the past is accepted, because typing up a paper order
  from last week is ordinary.
- **An archived supplier cannot be ordered from, and an archived product cannot
  be ordered** (`422` naming the line, never the product's name). Both are the
  archived-customer rule of B1.11 applied where it belongs: archiving means "we
  have stopped", and the refusal says what to do instead.

### Receiving, and the three-way-lite match (B5.05b)

`POST /inventory/purchase-orders/{id}/receipts` takes a location and a
quantity per line, and does three things in one transaction:

1. **Movements in**: for each line, `supplier` → the chosen stock location,
   reason `purchase`, `ref` the order.
2. **The order's state**: received quantities accumulate on the lines; the
   order becomes `partially_received` or `received`.
3. **A draft bill**: a `billing_bills` record in status `received` — the
   status B1.24 already means "arrived, nobody has decided" — carrying the
   supplier copied from `inv_suppliers`, the lines as received, and our
   purchase prices.

The third is the "three-way match, lite" features.md promises, and *lite* is
the honest word. A full three-way match compares purchase order, goods receipt
and the supplier's own invoice, and blocks payment on a discrepancy. What B5
builds is the first two legs: the receipt is matched against the order (an
over-receipt is refused, below), and the bill it drafts states what we
*ordered and received*, not what the supplier billed. When the supplier's real
e-invoice arrives through B1.24's import, it is a **second** bill, and
reconciling the two is the third leg — named in the cuts, not built.

The draft bill therefore carries `source_syntax = None` (it came from no
file) and is explicitly marked as *ours*, so nobody mistakes it for the
supplier's document. It is a draft in every sense that matters: it is
`received`, not `approved`, so it enters no payment run until a person decides
on it.

**Over-receipt is refused**, with a `Conflict` naming the line, the ordered
quantity and the total that would result. The alternative — a tolerance
percentage, which real procurement systems have — requires knowing what the
right tolerance is, and the right tolerance is a per-supplier commercial
agreement we have no field for and no way to guess. A genuine over-delivery is
recorded as a receipt of what was ordered plus a manual adjustment with a
reason, which is two calls and leaves a person's note explaining the third
pallet.

**Under-receipt is ordinary**: partial receipts are the normal case, the order
stays open, and the shortage report knows the difference between ordered and
received (below).

### As built (B5.05b)

Migration `0161_inv_po_receipts.sql`; store `inv_po_receive.rs` (the receiving
transaction, the receipt record and its reads); route
`inventory_po_receipts.rs` (`GET/POST /inventory/purchase-orders/{id}/receipts`)
over the existing `record_move_in` and a new transactional door onto
`billing_bills`. Seven decisions the note had not settled, decided here:

- **The received quantity is a column on the ordered line, not a fold over the
  ledger.** Two lines of one order may name the same product — two deliveries at
  two prices is an ordinary way to buy — so a movement cannot say which line it
  belongs to, and "ordered 40, received 25" would be a question with no answer.
  `inv_purchase_order_lines.received_qty_milli` is written only by the receiving
  transaction, which writes the movements in the same breath, and the database's
  own CHECK (`≤ GREATEST(qty_milli, 0)`) is what makes an over-receipt
  impossible rather than merely refused. That bound is deliberately **not**
  written in terms of `product_id`: phrasing it that way would re-evaluate on the
  `ON DELETE SET NULL` that deleting a catalog item performs, and a received line
  would then block the deletion.
- **A receipt is a document.** It has an ordinal within its order
  (`sequence_no`, drawn under the order's row lock), a location, a date from the
  database's clock, the note the person unpacking wrote, and a line per ordered
  line that arrived — each carrying the id of the movement it wrote. Without the
  document, "what came on the second lorry" is a question only a ledger scan can
  answer, and the drafted bill has no number to be built from.
- **An unstated delivery is the whole outstanding order.** `lines` absent means
  "what was ordered arrived", the ordinary case a warehouse should not have to
  type out; `lines: []` is not that and is refused, because an empty set states
  that nothing arrived and guessing between the two would book stock nobody
  claimed. A line that names a charge in words is refused by name — freight does
  not arrive on a pallet — and such a line never holds an order open either.
- **The drafted bill is numbered `PO-2026-00001/R1`, and says so.** A bill is
  keyed by `(supplier, number)`, so a number of our own shape can never collide
  with the supplier's real invoice when it arrives, and a person reading the list
  sees at a glance which document is theirs and which is our record of a
  delivery. It carries **no syntax and no checksum** — `billing_bills` learned
  that a bill read from no file records none, rather than being handed a hash of
  our own bytes that would claim a provenance it does not have — and its note is
  deliberately empty: a stored English sentence about the receipt would be
  untranslatable prose in a French tenant's ledger, and the number already says
  everything it would have.
- **The bill copies the supplier from our master record, not from the order.** A
  bill has to read as a document about a company — their address, their VAT id,
  the account we pay into — and the order carries none of that. The terms are
  theirs too, counted from the day the goods arrived; a supplier with no stated
  terms is due on receipt.
- **Whose order it is, first.** The refusal order is itself a tenancy rule: the
  order's ownership is checked before the locations are resolved, so a delivery
  booked against another tenant's order is a bare `404` and never a complaint
  about the caller's own warehouse — which would say the order was at least worth
  looking at. Found on the wire, not in review.
- **`inv_po_receipt_lines.move_id` cascades.** Not a licence to delete history —
  no door removes a movement — but this table reaches `tenants` only through its
  receipt, and without the cascade dropping a whole tenant fails on a movement
  that is still referenced.

The audit trail needed no change: `POST …/{id}/receipts` is a sub-resource, so
it derives as `inventory.purchase_order.receipt.create` and files against the
order it happened to. `/inventory` is already in the vite dev proxy and the
production Caddyfile, so no deploy change.

Two cuts, both stated rather than hidden. **The receipt date is today**: a
delivery typed up on Monday for goods that came on Friday is dated Monday, since
back-dating needs a field, a bound and a rule about movements that precede it,
and the ledger's own `occurred_at` already carries the same question. **A
receipt cannot be corrected or reversed** — no `PATCH`, no `DELETE`. Goods
received in error are corrected by an adjustment or a return movement, which is
the module's standing answer and leaves a person's note explaining it; a
reversal that unwound the accumulator, the movements and a bill somebody may
already have approved is a document of its own, and not this item's.

## Sales orders (B5.06)

### The state machine

```
draft ──confirm──▶ confirmed ──delivery(partial)──▶ partially_delivered ──delivery(rest)──▶ delivered
  │                    │                                   │
  └──────────────────cancel───────────────────────────────┘
```

Same shape as purchasing, mirrored, and for the same reasons. Two differences
worth stating:

- **Confirming does not move stock and does not reserve a row.** It changes
  what the shortage query counts (below) and nothing else. A sales order is a
  promise; goods move when they are picked.
- **Cancelling a `partially_delivered` order** does not un-deliver anything.
  What has gone out has gone out; the cancellation closes the remainder, and
  the customer is invoiced for what they received.

A sales order carries `SO-YYYY-NNNNN` from the same counter, drawn at
confirm — the same rule the PO follows, and this one has a further reason: the
number goes on the delivery note that travels in the box.

### Delivery

`POST /inventory/sales-orders/{id}/deliveries` takes a quantity per line and a
source location, and writes movements from that location to the `customer`
virtual location, reason `sale`. The negative-stock rule applies here in its
sharpest form: **you cannot ship what you do not have**, and the refusal names
what is available. This is the single most useful thing the module does, and
the whole moves-only design exists to make the refusal trustworthy.

The delivery gets a note — a printed document (`/print`, `/pdf`) with the
order number, the lines and the quantities, **and no prices**, because the
person unpacking the box is not the person who negotiated it.

### The invoice (B5.06b)

`POST /inventory/sales-orders/{id}/invoice` raises a **draft** invoice in alo
Billing for what has been **delivered and not yet invoiced**, one line per
sales-order line with the delivered quantity and the price snapshotted on the
order, and records the invoice id against the order.

The decision here is *when*: **at delivery, not at order**. Invoicing an order
before it ships means invoicing goods that may never leave, which is a VAT
event asserted on a hope. Invoicing what has actually gone out is the accrual
basis the B4 note already commits to, and it makes partial deliveries invoice
correctly with no extra concept.

The seam is `crm_handoff`'s (B2.08), followed rather than reinvented: one-way,
one-shot, raising a draft that Billing owns from that moment. Inventory never
issues, never sends, never touches an invoice it did not just create. What is
new is the idempotency: delivering more later and invoicing again must produce
a **second** invoice for the new quantity only, not a duplicate of the first,
so each order line tracks `invoiced_qty_milli` and the route raises nothing
(a `422` naming the order) when there is nothing left to bill.

**As built (B5.06b).** Three things came out differently from the paragraph
above, and each is a decision rather than a slip.

- **`invoiced_qty_milli` is a fold, not a column.** `inv_so_invoices` records
  each raising and `inv_so_invoice_lines` which ordered line contributed which
  quantity to it; how much of a line is billed is the sum of those rows over
  invoices that still stand. The delivered figure had to be an accumulator
  because two lines may name one product and the movement ledger could not say
  which line a movement belonged to; here the link names the line itself, so the
  sum is unambiguous — and it buys the release paths for free. Throwing away the
  draft removes the link rows by cascade; voiding the issued document is skipped
  by the fold; a **credit note does not release**, because crediting corrects a
  document and the goods stay billed against it. An accumulator would need a
  hook on each of those paths (`time_invoice::release_billed_hours` is exactly
  that), and a hook is a thing that gets forgotten on the fourth path.
- **A charge in words rides on the first invoice, in full, once** — and only
  once goods have actually gone out. It never leaves on a pallet, so "what was
  delivered" cannot answer for it; prorating it across consignments would be
  arithmetic nobody agreed to, and billing it before anything shipped would
  charge for a van that never came. An order that sells no goods at all is the
  one exception: there is no first consignment to wait for, so its charges are
  billable as soon as it is confirmed.
- **Only a `draft` order is refused** (`409`). `cancelled` is invoiceable on
  purpose: closing the remainder of a part-delivered order leaves the customer
  to be invoiced for what they received, which is what the short-close refusal
  says out loud. Whether there is anything left to bill is answered from the
  lines, not from the state, and the `422` says which of the two reasons it is —
  nothing has gone out yet, or everything that has is already on a document.

The customer's own reference travels from the order onto the invoice; the
order's internal note does not, and the store writes no sentence of its own onto
a document a customer reads, because it has no language. Each ordered line
reports `deliveredQtyMilli`, `invoicedQtyMilli` and `invoiceableQtyMilli`, the
last computed by the same code the button uses, so what a screen offers to bill
and what pressing it bills cannot disagree.

## Reorder rules and the shortage query (B5.07)

`inv_reorder_rules`: `(product_id, location_id)` → minimum quantity, target
quantity, and an `active` flag. Per location, because "we keep ten in the shop
and none in the warehouse" is the normal case, not a refinement.

The shortage query is the module's one genuinely interesting read, and it is
pure arithmetic over four numbers per product-location pair:

```
available = on_hand
          + on_order      (ordered − received, on sent/partially_received POs)
          − committed     (confirmed − delivered, on confirmed/partially_delivered SOs)
short when available < minimum
buy = max(target − available, the supplier's minimum order quantity)
```

**As built, the supplier's minimum is a floor and not a pack size.**
`inv_supplier_products.min_order_qty_milli` says the smallest quantity they will
sell, so a need of 12 against a minimum of 50 buys 50 — and a need of 60 buys
60, not 100. Rounding up to a *multiple* would be a pack size, which is a
different fact the supplier has not told us and which we therefore do not
invent.

**A rule's quantities are bounded by what a purchase-order line can carry**
(`REORDER_QTY_MAX_MILLI`, a million units) rather than by the larger bound a
supplier's minimum order may state: what a rule exists to produce is a proposed
order line, so a target that could not fit on one would be a number the module
can compute and never act on.

**`committed` is computed, not stored.** The rejected alternative is a
reservation table — a row per sales-order line holding stock aside. Real
systems have one, and the reason is real too: with reservations you can answer
"is this specific unit spoken for". We do not build it because it introduces a
second thing that must be kept in step with the orders (the drift argument
again, one level up), and because the question it answers is not the question
an SME asks. The question they ask is "do I need to buy more", and a fold over
open order lines answers it exactly, from data that cannot disagree with
itself.

`on_order` is what makes the report usable rather than annoying: without it,
a shortage that has already been ordered is reported every day until the goods
arrive, and a report that repeats itself is a report people stop reading.

**The pipeline numbers are per product, tenant-wide; only `on_hand` is per
location.** Neither document names a location until the goods are actually
received or picked — a purchase order says nothing about which shelf the lorry
will unload onto, and a sales order says nothing about which van picks it — so
attributing an open order to one shelf would be a guess dressed as a fact. Each
row therefore reports `onHandQtyMilli`, `onOrderQtyMilli` and
`committedQtyMilli` as three separate numbers, and a reader can see which part
of the answer came from where. With one stock location — the seeded case, and
the overwhelmingly common one — the two readings coincide exactly; with rules
on the same product at two shelves, one open order counts toward both, which is
the honest reading of "we have thirty coming, somewhere". Naming a destination
on a purchase-order line is the change that would make this exact, and it is a
B5.09/B5.10 question rather than a silent guess here.

A rule that watches an **archived** product or an **archived** location reports
nothing: a shelf being emptied on purpose is not a shortage, and reporting it
every morning is how a report loses its reader. A **parked** rule (`active`
false) keeps its numbers and stops producing shortages — what a seasonal
product needs out of season — which is why a rule is parked rather than
archived, and deleted outright when nobody wants it back (no document copied
anything from it, so nothing that happened depends on it).

The supplier on a row is the tenant's `default_supplier_id` for the product when
that supplier actually quotes for it, otherwise the first supplier who did;
archived suppliers are never proposed. `estimated_cost_cents` is the quantity to
buy at **their** quoted price, or at the product's own recorded purchase price
when nobody quotes for it — the number `inv_stock` values the shelf by, so "what
is there" and "what the gap costs" are quoted consistently. There is
deliberately **no grand total** on the report: each row's cost is in the
currency its supplier quotes in, and adding francs to euro to reach one number
would be a conversion nobody asked for.

## Stocktake (B5.08)

**Counting (B5.08a).** `POST /inventory/counts` opens a count for one
location and snapshots the expected quantity of every stocked product there,
at that moment, onto `inv_count_lines`. The count is a document with its own
state (`open` → `applied` | `cancelled`), and a person works down it — on a
phone, with the scanner — putting a counted figure against each line via `PUT
/inventory/counts/{id}/lines/{product_id}`. A line nobody counts is not
assumed to be right; it stays uncounted and is skipped when the count is
applied.

**Applying (B5.08b).** `POST /inventory/counts/{id}/apply` turns each counted
line into an adjustment movement — `adjustment` → location for a surplus,
location → `adjustment` for a loss, reason `count`, `ref` the count — in one
transaction.

The decision that matters is what "the variance" means, because a warehouse
does not stop while it is counted. **The variance is recomputed against
on-hand at the moment of applying, not taken from the snapshot.** If a
movement happened to a product between the snapshot and the apply — a delivery
went out at the far end of the room — then applying the frozen difference
would silently erase it. Instead, such a line is **flagged and not applied**,
and the response says which lines were skipped and why; the person re-counts
those few items rather than losing a shipment.

The snapshot is still worth keeping: it is what makes the sheet printable and
what shows the counter what was expected. It is a reading, not an authority.

Rejected alternative: locking a location during a count (no movements
permitted until it is applied). Correct in a system where a count takes ten
minutes, and unusable in a shop that counts a shelf on a Tuesday afternoon.

## The inventory agent (B5.10)

ADR 0034's shape, unchanged: a product-scoped tool set plus its description in
`alo-ai`, and executors in `alo-jmap` that run against the caller's own
tenant-scoped store handle. Two tools:

- **`reorder_proposals`** (draft) — "what needs reordering?" / "draft POs for
  everything under minimum". It reads the shortage query, groups the shortages
  by supplier, and proposes one **draft** purchase order per supplier. It
  never sends one. A proposal names quantities and prices the store computed;
  nothing in the executor adds money up.
- **`stock_answer`** (answer) — "how many blue chairs are left?", "when is the
  Hoffmann order due?", "what did we buy from them last quarter?" — answered
  from the tenant's own rows with the records it read cited, the shape
  `project_status_summary` (B3.10a) and `vat_summary` (B4.14b) established.

The model speaks names; the store speaks ids. Resolving "the blue chairs" to
exactly one product goes through `agent_args`, shared with the other product
agents, and an ambiguous name is a refusal that lists the candidates, never a
guess. Verification is **structural** — routes exist, guards return 401/422,
the execute path writes the expected rows against the local database — with no
live model calls, which is the loop's standing safety rail.

Deliberately absent, and worth the sentence: **no demand forecast**. "You will
need 40 next month" is a claim about the future that a small business would
act on, and the only honest version of it needs seasonality, lead-time
variance and a stated confidence. What the tool says is what is true today:
you are under your own minimum by this much.

## Errors

One map, `billing::map_store_err`, used and not copied — the call CRM,
Projects and Finance each made, for the same reason: it is a store-error map,
not a billing rule.

| Condition | Store | Wire |
|---|---|---|
| no or bad token | — | `401` (the `authenticate` extractor) |
| product, supplier, location, order, count or rule not this tenant's | `NotFound` | `404` — existence is never disclosed |
| a movement of a product that is not `stocked` | `Validation` | `422` naming the product |
| a movement whose two locations are the same | `Validation` | `422` |
| a movement with a non-positive quantity, or one over the bound | `Validation` | `422` |
| a movement that would take a `stock` location below zero | `Conflict` | `409` naming product, location, available and requested |
| a manual move naming a `supplier` or `customer` location | `Validation` | `422` — those move only through a document |
| an adjustment without a reason code, or with an unknown one | `Validation` | `422` listing the codes |
| an invalid barcode check digit | `Validation` | `422` naming the field, never the value |
| a duplicate SKU or barcode within the tenant | `Conflict` | `409` |
| un-stocking a product that carries movements | `Conflict` | `409` |
| deleting a location that carries movements | `Conflict` | `409` — archive it instead |
| editing or deleting a sent order | `Conflict` | `409` naming the status |
| a state transition that is not allowed from the current status | `Conflict` | `409` naming the current status |
| receiving more than was ordered | `Conflict` | `409` naming the line, ordered and resulting |
| delivering more than was ordered, or shipping without stock | `Conflict` | `409` |
| invoicing an order with nothing left to bill | `Validation` | `422` |
| sending an order whose supplier has no email address | `Validation` | `422` naming the supplier |
| applying a count line whose stock moved since the snapshot | — | `200` with the line reported as skipped, and why — not an error, a fact |
| applying a count that is already applied or cancelled | `Conflict` | `409` |
| a Drive photo the caller cannot see | `NotFound` | `404` |
| database error | `Db` | `500`, opaque — the wire never sees a raw error |

Validation messages are authored in the store and name the rule and the
field. They remain the one place a message crosses in English today — the
standing cross-cutting item B1.27, B2.14, B3.11 and B4.15 each left for a
human, and this wave adds no new kind of it.

## Tenancy

Every statement carries `tenant_id` from the handle, never from request input
— the invariant `for_tenant`/`for_account` make structural rather than
remembered. Every table is composite-keyed `(tenant_id, id)` and every foreign
key between them is composite, so a movement cannot reference another tenant's
location even if the store had a bug.

Three isolation tests are mandatory:

- **Wrong tenant** (law 1, every wave): tenant A's handle cannot read, move,
  order, receive, deliver, count or report on tenant B's product, supplier,
  location, movement, order or count. Clean denial, not data and not a `500`.
- **Wrong aggregate** (B4's addition, inherited and sharpened here): tenant A
  applying a whole generated month of movements leaves **every one of tenant
  B's balances, shortage rows and stock values byte-identical** — P7 above. A
  single-row read test cannot catch a `SUM` that forgot its `tenant_id`; this
  module *is* sums.
- **Wrong uniqueness** (new here): tenant B can store the SKU and the barcode
  tenant A already uses, and neither insert fails. The partial unique indexes
  are `(tenant_id, sku)` and `(tenant_id, barcode)`; a global index would leak
  the existence of another tenant's product through a constraint violation,
  which is a real and easily-missed side channel.

Two data-handling rules the module adds:

- **`inventory` joins `audit_action::AUDITED_MODULES`** beside `billing`,
  `crm`, `projects` and `finance`, in the item that registers the first
  mutating route (B5.04b), after which `tests/audit_routes.rs` requires every
  mutating `/inventory/*` route to be audited by reading the router's own
  source. "Who adjusted this stock down by forty, and when" is the question
  the audit trail exists for, and stock adjustment is the most abusable write
  in the business modules — it is the one that can make theft look like
  paperwork.
- **Nothing a human typed reaches a log.** Adjustment notes name people and
  incidents; a supplier's name is a party to a commercial relationship.
  `tracing` spans carry ids, counts and quantities — the rule mail bodies have
  had since Phase 1, applied to the warehouse.

## Files this wave will add

Store (`platform/alo-store/src`), one file one reason:

```
inv_locations.rs      locations, the seeded virtuals, the kind vocabulary
inv_moves.rs          record_move(): the one writer of a movement and of the
                      cached balance; the negative-stock rule
inv_adjust.rs         the manual door: the reason-code vocabulary, and which
                      movements a person may make without a document
inv_stock.rs          the reads: on-hand per product/location, the value, the
                      rebuild-from-movements fold
inv_suppliers.rs      the supplier master record
inv_supplier_prices.rs  what a supplier quotes us, per product
inv_barcode.rs        GTIN check-digit validation, pure (the shape `vat_id.rs`
                      already has: characters in, a verdict out, no door)
inv_po.rs             purchase orders: the record and the state machine
inv_po_receipt.rs     receiving: the movements, the state, the draft bill
inv_so.rs             sales orders: the record and the state machine
inv_so_delivery.rs    delivery: the movements and the note
inv_so_invoice.rs     the billing hand-off (the crm_handoff shape)
inv_reorder.rs        the rules and the shortage arithmetic, pure where it can be
inv_count.rs          the stocktake: snapshot, lines, variance, apply
migrations/0153…      billing_products columns (expand-only); inv_locations +
                      inv_seeds; inv_moves + inv_stock; inv_suppliers +
                      inv_supplier_prices; inv_po + lines; inv_so + lines;
                      inv_reorder_rules; inv_counts + lines
```

Migration numbers are taken in order at the moment each is written — the sites
track pushes to the same branch and shares the sequence — so "0153 onward" is
a starting point, not a reservation.

Routes (`products/mail/alo-jmap/src`): `inventory.rs` (the module's edge
concerns and the error-map reuse), `inventory_suppliers.rs`,
`inventory_locations.rs`, `inventory_location_names.rs` (the seed's words, one
table per language), `inventory_stock.rs`, `inventory_moves.rs`,
`inventory_po.rs`,
`inventory_po_send.rs`, `inventory_po_receipt.rs`, `inventory_so.rs`,
`inventory_so_delivery.rs`, `inventory_reorder.rs`, `inventory_counts.rs`,
plus `agent_inventory.rs` (B5.10) and the additive lines in `server.rs`,
`lib.rs` and `audit_action.rs`. The catalog's new fields are additive edits to
the existing `billing_products.rs`, not a new file.

Web (`web/src/inventory`): `InventoryModule.tsx`, `api.ts`, `types.ts`,
`CatalogView.tsx`, `StockView.tsx`, `MoveHistory.tsx`, `AdjustDialog.tsx`,
`SuppliersView.tsx`, `PurchaseOrdersView.tsx`, `PurchaseOrderEditor.tsx`,
`ReceiveDialog.tsx`, `SalesOrdersView.tsx`, `SalesOrderEditor.tsx`,
`DeliverDialog.tsx`, `ShortagesView.tsx`, `CountView.tsx`, `ScanInput.tsx`,
`index.ts`; the `inventory*` block in `i18n/en.ts`, `fr.ts` and `nl.ts`; the
module entry in `product/workplace.tsx`; `/inventory` in `vite.config.ts`.

## Out of scope for B5 (cuts are decisions)

- **★ Stock valuation and the ledger.** B5 posts **nothing** to the journal:
  no inventory asset account, no cost of goods sold, no purchase-price
  variance. This is the largest cut in the wave and it is deliberate. A
  valuation needs a *method* — FIFO, weighted average, standard cost — the
  choice is a per-tenant accounting policy with tax consequences, and the
  method decides what every receipt and every delivery posts. Choosing one on
  a tenant's behalf would be a compliance statement made by a machine, which
  the loop's own rules forbid. The stock screens therefore show quantity, and
  a *reference* value at the product's purchase price, clearly labelled as
  such and never called a balance. **Flagged for a human**: this is what an
  accountant will ask for first, and B4.11 (documents posting to the ledger at
  all) is the prerequisite that is itself still open.
- **Serial numbers, lot/batch tracking and expiry dates.** The move ledger's
  shape is ready for them — a lot is a column on the movement and a dimension
  on the balance — but every screen, every picker and every count sheet
  changes, and the businesses that need lots (food, pharma) need them
  *enforced*, not optional. It is a wave, not a field.
- **The third leg of the three-way match**: reconciling the supplier's own
  e-invoice (imported by B1.24) against the receipt we booked, and blocking
  payment on a discrepancy. B5 books both documents and links neither; a human
  reads them side by side.
- **Manufacturing, assemblies and bills of materials.** The `production`
  virtual location is seeded so that this needs no migration when it comes,
  and nothing in B5 writes to it.
- **Bin locations, pick paths and wave picking.** A location is a room, not a
  shelf-and-slot. The businesses this product is for count in rooms.
- **Multi-unit-of-measure conversion** (buy by the pallet, sell by the piece).
  A product has one unit in B5. Conversions look small and are not: every
  quantity in every query acquires a unit it must be read with.
- **Landed cost** — freight, duty and insurance apportioned onto the goods.
  Correct costing needs it; it is meaningless until there is a valuation to
  apportion onto.
- **EDI, Peppol ordering, supplier portals and carrier integrations** — label
  printing, track-and-trace, rate shopping. Every one is a handshake with a
  specific third party, which ADR 0035 puts outside the build.
- **Dropshipping and consignment stock** — goods that are ours but not here,
  or here but not ours. Both are location kinds plus an ownership dimension.
- **Customs and Intrastat reporting**, and **country-of-origin** on a product.
- **Demand forecasting and seasonality**, as the agent section says.
- **Barcode label *printing*.** We read codes; we do not generate, lay out or
  print them.
- **RFID, weigh-scale and PLC integrations** — hardware.

## Open questions flagged for a human

- **Does inventory need its own Insights datasets?** BI-1's semantic layer is
  a closed catalog of four datasets over billing and CRM (ADR 0037), and
  "stock value over time" and "purchases by supplier" are obvious tiles. They
  are also queries over a fold rather than over rows, which is a new shape for
  that layer. Deferred to BI-2 deliberately, not forgotten.
- **Who may adjust stock?** B5 lets any member of the tenant do it, like every
  other business module write. It is also the write most worth restricting,
  and `tenant_user_roles` (B4.12) is the mechanism that now exists. A
  warehouse role is a second row in that table and a decision about who a
  tenant's warehouse staff are.
- **What currency is a purchase order in?** B5 writes it in the supplier's
  default currency and stores the FX snapshot the way B1.21 does for an
  invoice. Whether the *stock value* is then held in the tenant's currency at
  the rate of the receipt (the accounting answer) or re-translated (the
  reporting answer) is a valuation question, and valuation is cut above.
- **Should a sales order be reachable from a deal?** CRM's won-deal handoff
  (B2.08) raises a quote or an invoice; for a business that ships goods, the
  natural third option is a sales order. It is a one-line addition to
  `crm_handoff` and a product decision about which of the three is the
  default.
