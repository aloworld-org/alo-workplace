# ADR 0042 — Meet guest links and lobby admission

## Status

Accepted, 2026-08-11.

## Surface

The host creates one invitation per external attendee. The API returns its raw URL token once. Public callers may inspect that invitation, request admission, and exchange it for a short-lived media token only after the host admits it. Authenticated hosts list, admit, deny, or revoke lobby entries.

## Errors

Unknown, expired, revoked, wrong-tenant, and ended-meeting invitations all appear as not found. A valid invitation awaiting admission returns `202`; a denied invitation returns `403`. Invalid names and expiries return `400`. Media not configured returns `503` only after admission succeeds.

## Tenancy

Invitation creation and moderation first resolve the meeting through `AccountStore`, preserving its tenant visibility rule, and require `created_by` to equal the caller. Public resolution hashes the presented token and obtains tenant and meeting from the matching row; callers never supply either. Raw tokens are never persisted or logged.

## Out of scope

Calendar/iTIP email delivery remains owned by Agenda. Meet returns a shareable per-invitee URL that the existing composer can send. Persistent reactions and hand history are deliberately excluded; these are ephemeral call state.

We rejected one reusable room link because it cannot identify, expire, revoke, or audit a guest independently and turns a leaked URL into continuing access.
