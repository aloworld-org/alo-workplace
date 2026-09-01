# Mail conversation to Sales opportunity

## User flow

The conversation action **Create opportunity** opens a review dialog. The user
chooses the pipeline and stage and may edit the title, company, contact, value,
currency, and expected close date before confirming. Nothing is created by
opening or dismissing the dialog.

The dialog keeps the source conversation visible but not editable. A successful
confirmation creates the opportunity and its conversation relationship as one
operation, then leaves the user in Mail with a success notification. The linked
conversation is available from the Sales opportunity detail.

## API contract

`POST /crm/deals` accepts the existing deal body plus optional `threadId`. When
`threadId` is present, the server verifies that the authenticated user holds a
message in that conversation and writes the deal and `crm_deal_threads` row in
one database transaction. The response remains the existing `{ "deal": ... }`
shape, so this is additive for existing clients.

## Security and failure behavior

- An unknown, another user's, or another tenant's thread returns `404` without
  revealing whether it exists.
- Pipeline, stage, deal, user, and conversation checks remain tenant-scoped.
- Any validation, permission, database, or link failure rolls back the deal;
  the system never leaves an unlinked opportunity from a failed Mail handoff.
- The existing `crm.deal.create` audit event covers the confirmed creation. The
  relationship records who linked it and when.

Real-store and real-router/Postgres tests cover the successful atomic handoff,
rollback, and wrong-tenant boundary. Frontend tests cover the Mail action and
the request contract.
