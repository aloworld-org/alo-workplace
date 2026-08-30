# Sites page editor UX review — 2026-08-30

## Design

- **Surface:** the existing page editor keeps its section operations, reviewed
  AI changes, inline text editing, responsive preview, translations, SEO,
  theme, and page protection. Their presentation is reorganised into one
  command surface, one section outline, and one dominant canvas.
- **Errors:** existing server reasons and recovery paths remain unchanged and
  visible beside the operation that failed.
- **Tenancy:** no API or persistence contract changes; every request continues
  through the existing tenant-scoped Sites client.
- **Out of scope:** this pass does not add freeform canvas positioning, new
  section types, publishing behaviour, or persistence.

The rejected alternative was another visual restyle of the existing card
stack. It would retain the duplicated actions, long instructional paragraph,
and blank preview hierarchy that made the editor difficult to scan.

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
