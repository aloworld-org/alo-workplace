# Shared billing document studio

## Surface

Quotation and invoice drafts use one versioned JSON design envelope and the
same web controls for brand colours, heading/body typography, logo, table
presentation and customer-facing content. The existing quotation design
routes remain compatible. Additive invoice routes expose the same envelope at
`GET/PUT /billing/invoices/{id}/design`. A draft invoice may replace its whole
design; an issued, paid or void invoice is read-only. HTML, PDF and covering
mail all load the same stored snapshot.

Draft invoices expose the same edit, customize, preview and PDF workflow as
quotations. Issued invoices retain read-only preview and PDF access. A
converted services quotation and its invoice show their relationship in both
document action bars; the stored invoice `quote_id` remains the single source
of truth rather than a second client-side relationship.

When accepting a services quotation creates an invoice, the store copies the
quotation design into the new invoice in the same transaction. A manually
created invoice starts from the workspace brand when the client first saves a
design. Missing designs continue to render with the existing neutral layout.

## Errors

- Unknown or foreign-tenant document: indistinguishable `404`.
- Non-object or oversized design: typed `422`.
- Writing a non-draft invoice or non-draft quotation: `409` explaining that
  the customer-facing document is frozen.
- Renderers read unknown design fields and block kinds leniently; unsupported
  content is skipped rather than making a legal invoice unavailable.

## Tenancy

Every design row carries `tenant_id` and has a composite foreign key to the
same tenant's billing document. All reads and writes go through
`AccountStore`. Wrong-tenant GET and PUT behavior is tested as byte-equivalent
to an unknown id. Quote-to-invoice copying occurs inside the authenticated
tenant transaction and never accepts a tenant id from the request.

## Rollout and operations

The schema change is additive. Existing quotation routes and rows are not
renamed. Existing invoices have no design and retain their current output.
The feature can be disabled by reverting the invoice studio entry point; the
server continues rendering stored designs and old clients remain compatible.
Validation/conflict response counts and PDF-render failures are the operational
signals; rollback never deletes design rows.

## Out of scope

Campaign composition and website themes remain consumers of the workspace
brand, not billing documents. E-invoice XML remains semantic and is never
decorated. Arbitrary removal of statutory invoice fields is not supported.

Rejected alternative: duplicating the quotation editor and renderer under
invoice-specific names, because two studios would drift and give one design
two reasons to change.
