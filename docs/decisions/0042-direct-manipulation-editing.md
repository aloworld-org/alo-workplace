# ADR 0042 — Editing a site by touching it, without a pixel canvas

**Status:** accepted
**Date:** 2026-08-11
**Context:** ADR 0036 (typed sections; the canvas rejected),
`docs/design/site-chatbot-and-commerce.md`

## The decision in one line

Every edit happens **on the page**, directly on the thing being changed — and
every edit is still a change to a typed section, so it still renders as a
reviewable diff.

## The mistake this avoids

People asking for "a canvas like Figma" are asking for **direct
manipulation**: click the thing, change the thing, see it. They are not asking
for absolute positioning. Those two get conflated because Figma has both, and
conflating them here would cost the three properties that make alo Sites
different from Wix:

- **Reviewable AI edits.** A diff of a typed section is readable by a human. A
  diff of moved pixels is not, and "Approve" over an unreadable diff is theatre.
- **A static renderer with a finite, golden-tested surface.** Free-form
  positioning makes the output space infinite and the goldens meaningless.
- **Semantic HTML**, which is where the SEO and the accessibility come from
  for free. Absolutely-positioned divs have neither.

ADR 0036 rejected the canvas for these reasons and that rejection stands. This
ADR says what we build *instead*, because "no canvas" was never an answer to
the request underneath it.

## What "better than the others" means here

Squarespace and Webflow have the editing feel. Wix has the AI. **Neither has a
reviewable diff**, because neither has typed sections to diff. So the target is
not to match their editor — it is to have their editor *and* the thing they
cannot build.

Four things, each of which fits inside the section model without bending it:

1. **Edit text where it lives.** Click the headline on the page and type. Not a
   sidebar form with a field called "Headline" — that form is why people say
   the builder feels like filling in a tax return.
2. **Drag a section to reorder, and the page reflows live.** Reordering a list
   is not positioning; it is the same typed sections in a different order, and
   it diffs perfectly.
3. **Resize within the section's own constraints.** A two-column split moves
   between its allowed ratios; an image picks from its allowed shapes. The
   constraint is what keeps the output good on a phone, and it is also what
   makes the result printable as a diff.
4. **A palette to drag new sections from**, showing what each one looks like
   with the tenant's own content rather than lorem ipsum.

Combined with "ask for a change and review the diff", that is something neither
Squarespace nor Webflow has.

## The rule that keeps it honest

> **Anything the editor can do, the section schema can express.** If a gesture
> cannot be written as a change to typed fields, the gesture is wrong — not the
> schema.

This is the line that stops the editor growing into a canvas one convenience at
a time. The first "just let them nudge it 4px" is the commit where the diff
stops being readable, the goldens stop meaning anything, and the AI review
becomes a spinner and a leap of faith.

## Consequences

- Mobile stays good by construction: there is no arrangement to break, because
  a section knows its own responsive behaviour.
- The AI edit path and the human edit path produce the *same* kind of change,
  so they share the same preview, the same diff and the same undo. There is no
  second code path for "AI changes".
- Some layouts remain impossible. That is the trade, stated plainly: a site
  that is always accessible, always fast, always mobile-correct and always
  reviewable, in exchange for not being able to put a text box at 37% × 12%.
- The editor's surface grows by adding section types, which is a bounded,
  testable act — not by adding freedom, which is not.
