# Branding workspace

## Surface

`/branding/*` owns one shared brand kit and exposes four sibling areas:
Foundation, Visual identity, Brand applications, and Guidelines. Foundation
stores the verbal identity; Visual identity owns the existing shared colour
roles plus one logo and heading/body font roles; Applications renders realistic
workspace outputs; Guidelines derives a printable reference from the same
draft. The existing `BrandKit.primary`, `secondary`, and `supporting` fields stay
additive and stable because Sites already consumes them.

## Errors

Invalid colour values disable saving and identify the problem in the editor.
Logo uploads refuse unsupported formats and files over 500 KiB with an inline,
announced message. A failed browser persistence write leaves the draft intact
and exposes a save error instead of reporting success. Older stored colour-only
kits are normalized into the expanded model.

## Tenancy

This slice preserves the existing browser-local brand repository: it makes no
server read and cannot address another tenant's records. Server-backed sharing
must use an authenticated tenant-scoped endpoint with wrong-tenant tests before
it replaces the local repository. The local repository is an interim single-
browser surface, not the eventual collaboration contract.

## Out of scope

Approval workflows, role-based publishing, audit history, historical version
recovery, multi-brand portals, and a server asset library belong to the `[B+]`
governance feature. Application previews demonstrate the shared contract; they
do not silently rewrite existing documents.

## Rejected alternative

Keeping one progressively larger colour page was rejected because foundation,
identity editing, output review, and published guidance have different users
and different reasons to change.
