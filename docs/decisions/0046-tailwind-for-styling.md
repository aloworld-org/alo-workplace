# ADR 0046 — Tailwind is how we write styles

**Status:** accepted
**Date:** 2026-08-13
**Supersedes:** the styling half of ADR 0045 (its component decision stands)
**Context:** `web/src/**`, 95 `.module.css` files, ~30 000 lines, ADR 0045

## The decision in one line

New styling is written with **Tailwind utility classes**; `.module.css` is no
longer the default, and the existing stylesheets migrate as the files around
them are touched rather than in one sweep.

## What this changes about ADR 0045, and what it does not

ADR 0045 was written yesterday and rejected Tailwind on the grounds that the
token layer already constrained values and the real failure was composition —
twenty-two independently written inputs, because CSS Modules scope so well that
nobody can see the other twenty-one.

**That diagnosis stands and is not reversed here.** A button, an input, a
field, a modal, a card, a table, a badge, a chip and a toolbar remain
**components in `ds/`**, not classes each module composes for itself, and the
test that enforces it stays. Utilities do not fix composition: twenty-two
inputs assembled from class strings are no better than twenty-two stylesheets,
and arguably worse, because a class string cannot be imported.

What changes is the layer *below* that: how a `ds/` component, and any layout
around it, expresses its styles. That is now Tailwind.

## Why, given the measurements said the values were fine

The measurements were about correctness, and they were right: 7 422 token
references against 108 hard-coded hex values is 98.6% discipline, and no
styling system would improve a number that good. The reasons to move are not
about correctness.

- **Editing cost.** A change to one screen means finding the rule in a separate
  file, in a scope deliberately isolated from everything else. Utilities put
  the style where the markup is, so the change is where the reader already is.
- **Deletion cost.** Dead CSS is invisible. A `.module.css` accumulates rules
  for markup that no longer exists and nothing tells you; utilities disappear
  with the element.
- **Familiarity.** Tailwind is what most people arriving at this codebase will
  already know, and a convention people know is worth more than a convention
  that is marginally better.

Owner decision, 2026-08-13. ADR 0045's author is not overruled on the facts —
the facts have not changed — but on the weighting of editing cost against
migration cost.

## The token layer is preserved, not lost

ADR 0045's strongest objection was that adopting Tailwind would mean "losing
the token layer". That objection is answered rather than accepted:

`tailwind.config` is generated **from the existing CSS custom properties**, so
`--color-surface` is available as `bg-surface` and remains one definition with
two spellings. No token is duplicated, no palette is re-entered by hand, and a
change to a token still moves everything. Arbitrary values (`bg-[#ff0000]`)
are lint-forbidden, exactly as the 108 hard-coded hexes are today.

If that generation ever drifts from the tokens, this decision has failed and
should be revisited — that is the tripwire.

## Migration: opportunistic, not a sweep

Rewriting 29 000 lines in one change would be an enormous, untestable diff
touching every screen at once, and ADR 0045 was right that the cost is real.
So:

- **New code is Tailwind.** No new `.module.css` file is created.
- **Existing files migrate when touched** for another reason, and the migration
  is part of that change rather than a separate one.
- **`ds/` components migrate first**, because everything else composes them and
  they are the smallest surface with the widest effect.
- A file is not migrated for its own sake. There is no deadline and no ticket
  for finishing; a `.module.css` that nobody touches is not a problem.

## Consequences

- The `ds` track's queue is rewritten against Tailwind before it continues. Its
  nine shipped primitives keep their behaviour, tests and API and change only
  in how they are styled — the accessibility work in them is the expensive part
  and is untouched.
- Both stylesheets and utilities exist in the tree for a long time. That is
  accepted, not a transitional embarrassment to be rushed.
- `web/src/i18n`, the token definitions, and every Rust surface are unaffected.
- The one thing that would make this a mistake is the token generation drifting
  from the tokens, leaving two sources of truth for colour. Watch for it.
