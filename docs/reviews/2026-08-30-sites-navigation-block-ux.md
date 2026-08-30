# Sites navigation block UX review — 2026-08-30

## Research

- [Squarespace](https://support.squarespace.com/hc/en-us/articles/360000667707-Build-your-site-header)
  treats the header as a repeated site landmark, derives its menu from pages,
  keeps the primary action separate, and collapses it on mobile.
- [Wix](https://support.wix.com/en/article/wix-editor-managing-your-sites-pages)
  makes pages and menus one management surface, including page order and
  whether a page appears in navigation.
- [Webflow](https://help.webflow.com/hc/en-us/articles/33961304628627-Navbar)
  combines brand, menu links, and a responsive menu button; its guidance
  recommends a reusable component and explicit breakpoint review.
- [Framer](https://www.framer.com/help/articles/using-layout-templates/) uses
  shared layout templates for navigation and responsive variants across pages.
- [W3C](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/) uses a native
  disclosure button with `aria-expanded`/`aria-controls`; its
  [menu guidance](https://www.w3.org/WAI/tutorials/menus/structure/) marks the
  current destination with `aria-current="page"`.

These products differ in editing freedom, but agree that navigation is a
header system, not an ordinary body block.

## Design review

- **Surface:** the page editor pins Navigation above body sections, removes
  meaningless section-move actions, and opens a dedicated header editor. The
  editor can fill missing links from existing site pages, retains manual URLs,
  supports pointer drag and named keyboard ordering controls, and separates the
  optional primary action. The published header marks the current page and its
  mobile disclosure closes after activation or Escape.
- **Errors:** a page-list read failure leaves every manual field usable and
  explains that only the shortcut is unavailable. Existing section save
  failures stay in the dialog with typed values intact. A second Navigation
  block is disabled in the picker and rejected by the editor before a write.
- **Tenancy:** page suggestions use the existing tenant-authorized
  `GET /sites/{site}/pages`; section reads and writes retain their existing
  site/page scoping. No new persistence or cross-site lookup is introduced.
- **Out of scope:** the schema still stores navigation per page. A future
  shared-layout contract may make one header update every page, but that needs
  an additive, versioned storage design rather than a UI-only assumption.

Rejected alternative: adding more generic controls to the existing section
row. It would preserve the false promise that Navigation can be positioned in
the body even though the renderer always emits it before `<main>`.

## Operations

No feature flag is required: the wire schema and routes are unchanged. Watch
the existing Sites section-save failure rate and public render error logs.
Reverting the navigation UX/render commit is the off-switch.

## Visual refinement

- **Surface:** the section outline now uses one compact heading row and a
  full-width navigation card. The card shows a restrained header miniature
  built from the stored menu labels and primary action, so users can recognise
  its contents before opening the editor. Edit and delete remain grouped at
  the trailing edge.
- **Errors:** this is presentation-only; the existing save, delete and loading
  errors remain unchanged and no content is hidden behind the miniature.
- **Tenancy:** the miniature renders only the section already returned by the
  tenant-scoped page read. It performs no additional reads or writes.
- **Out of scope:** this does not turn the outline into a second live preview;
  responsive and published rendering remain in the optional Preview panel.

Rejected alternative: adding more badges, borders and instructional copy to
the existing peach card. That would increase noise without making the stored
navigation easier to recognise.
