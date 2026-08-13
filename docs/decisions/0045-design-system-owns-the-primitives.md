# ADR 0045 — The design system owns the primitives

**Status:** accepted — **styling half superseded by ADR 0046** (2026-08-13)
**Date:** 2026-08-12
**Context:** `web/src/ds/`, `web/src/ds/primitives.test.ts`, ADR 0042
(direct-manipulation editing)

> **Read this alongside ADR 0046.** The decision below — that primitives are
> components in `ds/` rather than classes each module rewrites for itself,
> enforced by a test — **still stands, and is the load-bearing half of this
> ADR**. What no longer holds is the styling mechanism: the paragraph rejecting
> Tailwind was overruled by the owner on 2026-08-13, on the weighting of
> editing cost against migration cost, not on any measurement here — none of
> which was disputed. Styles are now Tailwind utilities generated from these
> same tokens, so the 98.6% token discipline this ADR measured is preserved
> rather than discarded. The composition argument is untouched: utility class
> strings do not compose either, which is exactly why `ds/` still owns the
> primitives.

## The decision in one line

A button, an input, a field, a modal, a card, a table, a badge, a chip and a
toolbar are **components in `ds/`**, not CSS classes each module writes for
itself — and a test enforces it, because a convention could not.

## What was actually wrong

The instinct was that CSS had been the wrong choice. The measurements say
otherwise:

| | |
|---|---|
| `.module.css` files | 84, 29,111 lines |
| `var(--token)` references | **7,422** |
| Hard-coded hex colours | **108** |
| Components in `ds/` | **8** |
| Stylesheets defining a primitive | **46**, with 136 definitions |
| `.input` written independently | **22 times** |

Token discipline is 98.6% clean. The values were never the problem. The layer
above them was missing: there is no `<Input>`, no `<Field>`, no `<Modal>`, so
every screen writes its own and the product stops looking like one product.

**CSS Modules made it invisible.** Scoping is the feature — a file's styles
cannot leak — and the cost is that nobody can see the twenty-one other
implementations of the thing they are about to write. The tool worked exactly
as designed; the design had a hole in it.

*Rejected: switching to Tailwind, Sass, or CSS-in-JS.* Tailwind constrains
values, which tokens already do here; it would produce twenty-two
differently-composed inputs in class strings instead of stylesheets, at the
price of rewriting 29,000 lines and losing the token layer. Sass changes the
syntax and nothing else. Runtime CSS-in-JS adds a second theming system beside
the tokens and a runtime cost. None of them addresses composition, which is the
actual failure.

*Rejected: Flutter.* It is a third language against a Rust-and-TypeScript
doctrine, and Flutter Web paints to a canvas — which would cost alo Sites its
semantic HTML and therefore its SEO (the whole argument of ADR 0036), degrade
screen-reader support in a product sold into European procurement, and break
text selection, find-in-page and password managers in a product whose core
activity is reading and writing text.

## What we build instead

`ds/` grows from 8 components to cover what the codebase kept re-deriving:
**Field, Input, Modal, Card, Table, Badge, Chip, Toolbar**. Each is built from
the best implementation already in the repository rather than invented, so a
migration is a deletion rather than a redesign.

Accessible behaviour — focus traps, keyboard handling, ARIA — is worth taking
from a headless library rather than hand-rolling per module. It is the same
move Odoo made in taking Bootstrap: spend the years on the business logic, not
on a twenty-third input field.

## How it is enforced, and why enforcement is the point

`ds/primitives.test.ts` fails when a stylesheet outside `ds/` declares one of
the primitive classes. The 46 that already do are listed in `ds/redefined.ts`,
**and that list may only shrink**.

This is deliberately the same mechanism as `i18n/locale.test.ts`, which is the
only convention in this repository that has actually held: name what is already
wrong, forbid anything new joining it, and let the number go down. Every other
approach here has been a preference, and preferences produced commits whose
entire purpose was to "unify" and "canonicalize" styling that then drifted
again.

Both failure directions are proven rather than assumed: a new stylesheet
declaring `.input` fails, and a line left in the list after its stylesheet was
migrated also fails, so the list cannot silently become permanent.

## Consequences

- Migration is incremental and each step is a net deletion. A screen that
  adopts `ds/` loses its local rules, and the CSS shipped — 252 KB across three
  bundles today — falls with it.
- The work touches every module and therefore all three streams. It wants to be
  one agent's job for a stretch rather than three people editing in parallel.
- New modules get faster to build, which is the compounding half: the first
  screen after the primitives exist is shorter than the last one before them.
- A genuine exemption is still possible — a line in `redefined.ts` — but it is
  now a visible, argued act rather than the path of least resistance.
