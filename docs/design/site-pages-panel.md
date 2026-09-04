# Site pages panel

## Surface

The site workspace lists pages with a page or home icon, title, publication
status, updated date, and the existing actions menu. New page uses the same
circular structural-add pattern as Sections and quotation content. Search,
filter, sorting, hierarchy, and whole-row opening remain unchanged. The table
uses the shared unfilled header and divider treatment rather than a
module-specific coloured band.

## Errors

This presentation change adds no new requests or failure states. Existing page
loading and action errors continue through the Sites workspace handling.

## Tenancy

The panel receives pages already loaded through the tenant-scoped Sites API and
does not introduce identifiers, reads, or writes of its own.

## Out of scope

Page APIs, publishing, ordering, navigation, and permission rules are unchanged.

The rejected alternative was retaining the bottom circular plus and decorating
the table more heavily; the detached action remained hard to discover and extra
chrome would not improve scanning.
