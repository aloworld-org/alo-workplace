# Sales opportunity → project

## Surface

`GET /crm/deals/{dealId}/project` reads the delivery relationship and `POST`
to the same URL confirms conversion of a won deal with `name`, optional
`color`, and optional `customerId`. `GET /projects/{projectId}/deal` provides
the reverse read. The Sales drawer is the first caller: it asks for review in a
dialog, creates only after confirmation, and then opens the linked project.

## Errors

- Conversion of a missing or foreign-tenant deal is `404`; relationship reads
  return `null`, preserving the no-existence-oracle convention of list seams.
- An open or lost deal and an invalid project name are `422`.
- An absent or archived customer is refused before any project is written.
- A retry is successful and returns the original relationship with
  `created: false`.
- A converted deal cannot be deleted; the API returns `409` so provenance is
  never silently removed.

## Tenancy

Every read and write binds `tenant_id`. Composite foreign keys require the
deal, project, and relationship to belong to that same tenant; the creator is
the authenticated account user and uses the repository's global user key. The
conversion uses one database transaction for customer validation, project
creation, client facts, and provenance. Real-database tests verify a neighbour
can neither discover nor convert another tenant's identifiers.

## Out of scope

This slice does not auto-create Drive folders, Chat rooms, kickoff meetings,
or project templates. Those are optional setup choices and need their own
reviewable contract rather than hidden side effects in the core conversion.

The rejected alternative was a browser sequence of “create project, then save
the deal link”; a dropped second request would leave orphan work and retries
could create duplicates.

## Operations

Database or route failures already enter the service's structured request
error telemetry. Rollback is a code revert; the additive table may remain
unused safely. The UI action is available only for won deals and makes no
background conversion.
