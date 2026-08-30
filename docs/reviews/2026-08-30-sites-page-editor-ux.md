# Sites page editor UX review — 2026-08-30

## Design

- **Surface:** the existing page editor keeps its section operations, reviewed
  AI changes, inline text editing, responsive preview, translations, SEO,
  theme, and page protection. Their presentation is reorganised into one
  command surface and one section outline. Preview is an explicit one-click
  view, starts closed to preserve the building workspace, and can be resized
  against the section outline with a pointer or keyboard on desktop. Page
  access is a command-surface action that keeps its status loaded and opens a
  focused dialog instead of permanently occupying a full-width row. Assisted
  page changes follow the same pattern: the instruction and its reviewable
  proposal live behind one navigation action, and remain intact while its
  dialog is closed. When a page has no sections, its outline uses the remaining
  editor height so the workspace ends on the standard viewport gutter instead
  of leaving an accidental blank region below a content-height card.
- **Errors:** existing server reasons and recovery paths remain unchanged and
  visible beside the operation that failed.
- **Tenancy:** no API or persistence contract changes; every request continues
  through the existing tenant-scoped Sites client.
- **Out of scope:** this pass does not add freeform canvas positioning, new
  section types, publishing behaviour, or cross-device persistence of the
  editor's temporary panel width.

The rejected alternative was keeping Preview permanently visible and only
making the divider draggable. That would still spend half the workspace on a
view the user may not need while assembling sections.

For page access, the rejected alternative was a collapsible row beneath the
toolbar. It would still make a page-level setting look like part of the page's
section content and add height whenever it was used.

For assisted page changes, the rejected alternative was keeping the composer
above the section list. It mixed a page-level command into the document
outline and permanently reduced the space available for building.

For the empty outline, the rejected alternative was a larger fixed minimum
height. Fixed values still leave arbitrary blank space on tall displays and
can overflow shorter ones; the shared flex workspace follows the viewport.

## Interaction references

- [Squarespace Fluid Engine](https://support.squarespace.com/hc/en-us/articles/6421525446541-Edit-your-site-with-Fluid-Engine)
  makes sections the primary page-building unit and exposes insertion directly
  in the editing surface.
- [Framer canvas controls](https://www.framer.com/help/articles/how-to-use-the-canvas/)
  keep viewport controls compact and subordinate to the canvas.
- [Framer on-page editing](https://www.framer.com/help/articles/on-page-editing/)
  makes editable content discoverable where it is rendered.
- [Framer Agents](https://www.framer.com/help/articles/how-to-use-agents/)
  scopes assisted changes to the selected content and preserves review before
  application.

The Alo implementation adopts those interaction reflexes without copying
their visual identity: Terracotta remains the sole action accent, settings are
progressively disclosed without hiding core work, and every operation remains
inside Alo's existing review and persistence model.
