# Sites navigation destinations and colours

## Surface

The existing navigation-section dialog gains one destination picker per menu
item. It lists site pages and the addressable content sections on those pages,
while retaining a custom-link choice for web, email and telephone targets. A
collapsed Appearance panel lets an owner choose reusable Site theme roles for
the background, text and hover state. Site theme owns the editable base palette
(background, text and border) and five brand accents. The saved `nav` section
adds one optional role-based `appearance` object; older sections remain
byte-for-byte valid. The public renderer gives body sections predictable
type-based fragment ids (`features`, `features-2`, …) and resolves navigation
roles through scoped CSS custom properties.

## Errors

If site pages or their section details cannot be loaded, the dialog keeps the
custom target field usable and shows the existing non-blocking load message.
Malformed theme colours or unreadable base text/background contrast are refused
as a 422 by the theme schema. The navigation dialog previews the chosen role
combination and warns when its text states miss WCAG AA. A failed update leaves
the dialog and all entered values open, using the existing save error path.

## Tenancy

Page and section reads continue through the authenticated `/sites/{site}/pages`
and `/sites/{site}/pages/{page}` routes, whose store lookups are scoped to the
signed-in account. Appearance stays in tenant-owned page JSON and the palette
in the tenant-owned site theme; neither creates a new public lookup path.

## Out of scope

V1 does not add arbitrary custom fragment names, per-item raw colours, font controls,
transparent headers, sticky/overlay modes or multi-level menus. Generated
fragment ids are semantic by section type; when a page has duplicate section
types, their numeric suffix follows their order.

Rejected alternative: storing raw hex inputs in each section was rejected
because it duplicates brand decisions, allows pages to drift, and makes a
site-wide rebrand unnecessarily difficult.

## Operations

Malformed values are visible through the existing HTTP 422 theme-update traces.
Rollback is a code revert; because both `colors` and `appearance` are optional
and render defaults remain unchanged, older sites continue to render.
