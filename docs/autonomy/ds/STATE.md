# alo design system — migration journal

One entry per completed queue item: what was migrated, what the ratchet said
before and after, and anything the adoption changed about how a screen looks.

Started 2026-08-12 with 43 stylesheets on `web/src/ds/redefined.ts`, of which
one — `sites/SitesModule.module.css` — belongs to another track and is not this
queue's to take.

**A screen that looks worse after adopting a shared component is a finding, not
a licence to redesign.** Record it here and keep going; a redesign inside a
migration commit makes an intended change indistinguishable from a regression.

## D1.01 — `Card`

Twelve copies reconciled. They agreed on the idea and on nothing else: padding
from `--space-4` to `--space-8` and, in two of them, a hardcoded `10px 12px`;
radius md through xl; shadow absent, `sm`, `md`, or a hand-mixed
`rgba(40, 30, 20, 0.04)`. **`crm` and `hr` were byte-identical**, hardcoded
values included — copied from one to the other, which is honestly how most of
this happened.

Took the most token-clean build and kept the real differences as `pad`
(`sm`/`md`/`lg`), `flat` for a card inside another surface, and `interactive`
for one that is genuinely clickable. Hover states on things that do not respond
to a click are a promise the screen does not keep, so that one is opt-in.

## D1.02 — `Badge` and `Chip`

The copies had blurred the distinction, so it is now stated in the file:
**a badge is read, a chip is acted on.** "Admin" beside a name is a badge; a
recipient you can remove is a chip. If it contains a button, it is a chip.

Several chips carried asymmetric padding — `0 3px 0 var(--space-2)` — to make
room for a trailing remove button each caller drew itself. The component owns
that now, and the remove button takes a label naming *what* it removes, because
a row of buttons all called "Remove" is useless read aloud.

Badge tones are named rather than left to the caller, and the file records that
a tone must never be the only signal: colour alone is invisible to somebody who
cannot distinguish it.

## D1.03 — `Table`

Ten copies, and **five were byte-identical** — crm, finance, hr, inventory,
projects, hardcoded `10px 14px` included. That majority is the base, with the
two hardcoded values replaced by the tokens they were approximating. The three
that really differed differed for reasons: billing wants a header that stays
put over a long list, insights wants a dense table inside a card, both want a
footer of totals. Those became `stickyHeader`, `density="compact"` and plain
`<tfoot>` styling.

Reconciled rather than offered as options: **no zebra** (not one of the ten had
it, and a stripe fights every status tint a cell puts on top of it); a **quiet
header** (billing shouted in uppercase, the other nine did not — a column
header is a label, not a heading); **row hover opt-in**, same argument as
`Card.interactive`.

The styling was the easy half. What all ten were missing:

- **A keyboard-reachable scroll region.** Every copy put `overflow: auto` on a
  plain `<div>` — scrollable with a mouse, unreachable without one (WCAG
  2.1.1). The region is now focusable, with a role and a name so the tab stop
  is explicable.
- **A name.** `label` is required and becomes a `<caption>`, read always and
  drawn only with `showLabel`. Without it a table is announced as "table, 7
  columns" and nothing more.
- **An empty state inside the table.** Screens put "no matches" in a sibling
  `<p>`; anyone navigating by table found an empty grid with no reason.
  `TableEmpty` spans every column.

Base rules target `th`/`td`, so ordinary markup inside `<Table>` is already
right and each migration is a deletion. `Th`/`Td` only carry what a class name
was carrying: alignment, tabular figures, and a header present for a screen
reader but not on screen. No sorting — nothing in the repository sorts from a
header, and inventing `aria-sort` for no caller is how a primitive grows a
surface nobody asked for.

Verified: 7 behaviour tests (name, hidden-vs-shown caption, focusable region,
`scope="col"`, sr-only header text, empty row `colspan`, numeric alignment);
`tsc`, `eslint`, `npm run build`, `npx vitest run src` (70 files, 637 tests)
all green. No screenshot — the component has no caller until D2.01, so the
visual check lands with the first migration. `vitest` reports 4 unhandled
rejections from `sites/Theme.test.tsx`; pre-existing, another track's area,
non-failing.

**Next:** D1.04 `Toolbar`.
