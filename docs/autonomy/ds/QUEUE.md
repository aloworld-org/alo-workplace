# alo design system — migration queue

Forty-three stylesheets still declare a primitive that `ds/` should own, and
`web/src/ds/primitives.test.ts` names every one of them. This queue empties
that list.

**Read `docs/decisions/0045-design-system-owns-the-primitives.md` first.** The
short version: token discipline here is excellent — 7,422 `var(--token)`
references against 108 hard-coded colours — so the values were never the
problem. What was missing is the layer above them, and CSS Modules made the
duplication invisible rather than caused it. Do not "improve" this by reaching
for Tailwind or a rewrite; that argument is settled in the ADR.

## The loop for every migration item

The same four steps, every time:

1. **Adopt** the `ds/` component in the module's `.tsx`.
2. **Delete** the now-dead rules from the module's `.module.css`.
3. **Remove** the file's line from `web/src/ds/redefined.ts`.
4. **Look at it.** One screenshot of a screen from that area, opened and read.
   `npx vitest run src` must stay green, and `primitives.test.ts` will tell you
   immediately if step 2 or 3 was incomplete.

A migration is a **net deletion**. If a file grew, something was reimplemented
rather than adopted.

**When a component cannot express what a screen needs**, widen the component —
a variant, a prop — and say so in its file. Do not leave the local rule behind
and do not add a line to `redefined.ts`; that list only shrinks. An exemption
is possible but must be argued in the commit message.

**Do not restyle while migrating.** Adopting a shared component already changes
how a screen looks; mixing a redesign into the same commit makes it impossible
to tell an intended change from a regression. If a screen looks worse
afterwards, that is a finding worth reporting, not a licence to redesign it in
place.

## Build the missing primitives first

Demand across the remaining stylesheets, counted rather than guessed:
`chip` 14 · `card` 12 · `toolbar` 11 · `table` 10 · `badge` 8 · `toggle` 7 ·
`select` 7. `Input`, `Field` and `Modal` already exist.

Each item: build from the best implementation already in the repository rather
than inventing one, keep the differences that were decisions and drop the ones
that were accidents, and write tests for **behaviour** — the thing sixteen
hand-built modals were missing was never the border radius, it was the focus
trap.

- [x] D1.01 `Card` — the surface a screen's sections sit on. Twelve copies; reconcile padding, radius and border before writing it.
- [x] D1.02 `Badge` and `Chip` together — they are the same object at two weights, and eight and fourteen copies respectively disagree on which is which. Decide the distinction and state it in the file.
- [x] D1.03 `Table` — ten copies. Header, zebra or not, alignment, and the empty state. Keyboard and screen-reader semantics matter here more than the styling.
- [x] D1.04 `Toolbar` — eleven copies. The row of controls above a list or an editor; the interesting part is how it wraps at narrow widths.
- [x] D1.05 `Select` — seven copies. A native `<select>` styled to match `Input`, not a bespoke listbox; a custom one is a large accessibility liability for no gain here.
- [x] D1.06 `Toggle` — seven copies. A checkbox that looks like a switch, with the label bound and the state announced. Shipped as `Toggle` **and** `Checkbox`: only two of the seven drew a switch, four drew a checkbox row under the same name.

## Migrate, area by area

**`web/src/sites/**` is not in this queue.** An autonomous loop is building
through the sites wave and cannot negotiate with you over a file. Those
stylesheets stay on the allow-list until that track migrates them itself, or
until it is idle and somebody says so — which means `redefined.ts` will not
reach zero from this queue alone, and D3.01 accounts for that.

- [x] D2.01 Migrate **authoring** (7 stylesheets — badge,chip,input,modal,table,toolbar) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [x] D2.02 Migrate **mail** (7 stylesheets — btn,card,chip,field,input,modal,select,toolbar) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [ ] D2.03 Migrate **shell** (7 stylesheets — badge,button,card,field,input,modal,select,toggle) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [ ] D2.04 Migrate **drive** (3 stylesheets — chip,dialog,input) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [ ] D2.05 Migrate **admin**, **agenda**, **auth** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D2.06 Migrate **billing**, **chat**, **contacts** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D2.07 Migrate **crm**, **finance**, **home** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D2.08 Migrate **hr**, **importer**, **insights** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D2.09 Migrate **inventory**, **invite**, **meet** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D2.10 Migrate **platform**, **projects** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D2.11 Migrate **tasks** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D3.01 Wave review: everything outside `web/src/sites/**` is off `ds/redefined.ts`; walk the product in a browser at desktop and phone width; check the CSS actually shipped has fallen from its 252 KB; and record in `CHANGELOG.md` what a user would notice.
