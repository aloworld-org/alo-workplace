# Sales to Billing provenance

## Rule

A Billing document is related to an opportunity only when it was raised through
that opportunity's confirmed **Raise quote** or **Raise invoice** action. A
shared customer is not evidence of provenance and is never used to populate the
related list.

## Storage and API

`crm_deal_billing_documents` stores one deal and exactly one quote or invoice.
Composite foreign keys bind the deal and document to the same tenant. Deleting
an unissued Billing draft removes its relationship; deleting a related deal is
already refused once it has delivery provenance and otherwise removes only the
relationship, never an issued Billing document.

`GET /crm/deals/{dealId}/documents` returns the linked document kind, id, live
status, current number (or `null` for a draft), and relationship timestamp.
Missing and foreign-tenant deals return the same `404`.

The handoff attaches provenance as soon as the draft exists. If line replacement
is refused, Billing's existing safe behavior leaves an empty, editable draft;
that draft remains visible from Sales so it cannot become hidden orphan work.

## Interface

The opportunity drawer has a Related Billing card. It shows an honest empty
state until a document is raised, refreshes after a successful handoff, and
links each row to the canonical Billing editor. Status and document labels use
the existing Billing lifecycle translations and design-system primitives.

