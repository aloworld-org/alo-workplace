# HR letter templates — UI delivery review

## Surface

HR members can list, create, edit, and delete the tenant's letter templates at `/hr/templates`. The editor obtains its placeholder vocabulary from `GET /hr/letter-templates` and inserts those exact machine names; it does not maintain a competing browser vocabulary.

## Errors

Loading, saving, and deleting each have a visible localized failure. Blank drafts cannot submit. Server validation and conflict details travel through the shared HR REST error mapping. Deletion requires an explicit destructive confirmation and explains that existing drafts do not change.

## Tenancy

No storage contract changed. Every request uses the authenticated HR client and the existing HR-only routes, whose tenant store and wrong-tenant tests remain the authority. The tab and route are hidden when the HR door says no; the server repeats that decision on every request.

## Out of scope

This screen authors templates; it does not send mail or select an employee. Drafting a personal letter remains the agent's propose-and-approve action. Rich-text layout is excluded because the stored contract is plain subject/body text.

We rejected a browser-owned placeholder list because it would drift from the server and eventually offer fields the server refuses or omit fields it supports.
