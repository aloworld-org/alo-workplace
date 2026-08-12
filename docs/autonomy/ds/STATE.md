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

## D1.04 — `Toolbar`

Eleven copies, and **four were byte-identical** again — crm, finance,
inventory, projects, right down to the `flex-wrap`. That is the base. The rest
differed in three ways worth keeping and no others: whether the row draws
chrome (`surface` — nothing, agenda/tasks/mail's rule-under-a-bar, billing's
raised card), how tight the gap is (`density`), and whether controls line up on
their centres or their baselines (`align="end"`, hr's row of labelled fields).

Reconciled rather than offered: one separator colour (`--border-subtle`, what
every other rule in the system uses — agenda and tasks had drifted to
`--border-default` via a hardcoded `1px`), and one cluster gap (`--space-1`;
mail's formatting bar packed its buttons at `var(--border-width)`, a token
doing duty as a length).

What none of the eleven had:

- **A name or a role.** Every one was a bare `<div>`. `tasks` has two on one
  screen (`.toolbar` and `.toolbar2`) which from the outside are
  indistinguishable. `label` is required.
- **Wrapping.** Seven could not wrap, so at phone width their last controls sat
  outside the pane — gone, not merely unseen (WCAG 1.4.10). Wrapping is the
  base, and `ToolbarGroup` is what stops a wrap landing in the middle of a
  segmented control.
- **Arrow keys.** mail's formatting bar is a dozen icon buttons, so a dozen tab
  stops between the message list and the message body.

`keyboard` is a choice rather than a default, and that is the one judgement
call in the item. APG's `role="toolbar"` — one tab stop, arrows inside — assumes
a toolbar of buttons; most of ours also hold a search field or a select, where
arrow keys belong to the caret. So `tab` (default) is a named `role="group"`
with every control keeping its tab stop, and `keyboard="roving"` opts into the
toolbar role *and* the behaviour that role promises: roving tabindex, arrows,
Home/End, disabled controls skipped, and a control that appears later arriving
outside the tab order. Announcing a role without its keyboard model would be a
promise the component does not keep.

Also `ToolbarSpacer` (three copies had it, all `flex: 1`) and `ToolbarDivider`.

Verified: 9 behaviour tests; `npm run build` (tsc + vite) clean, `npx eslint`
clean, `prettier --check` clean, `npx vitest run src` green — 71 files, 646
tests. No screenshot: like `Table`, the component has no caller until the D2
migrations, so the visual check lands with the first adoption. The 4 unhandled
rejections from `sites/Theme.test.tsx` are still there — pre-existing, another
track's area, non-failing.

**Next:** D1.05 `Select`.

## D1.05 — `Select`

Seven copies, and this time the interesting agreement was not among the selects
but beside them: **four of the seven wrote `.input, .select, .textarea` in one
rule** (finance and projects byte-identical again, billing and inventory a line
apart), which is the codebase saying out loud that these two controls stand in
the same form row. So the base is `Input`'s box, not the majority's: 40px and
`--text-base` rather than the ~33px each module had derived from `padding: 8px
12px`. **That is a visible change** — every select in the product gets taller at
its migration — but its neighbouring search field and text input are making the
same move in the same commit, so the row stays level. Recorded here rather than
softened with an `sm` size, which would have re-created the mismatch it was
meant to fix.

Reconciled rather than offered: one focus treatment (there were four, including
none at all in both shell sections); one disabled state (`Input`'s, over
shell's `opacity: 0.55`, which takes a control's text under the contrast floor
— a control you cannot use still has to be readable); and room on the right for
the arrow the platform draws, which every copy padded over.

Kept as real differences: `variant="ghost"` (mail's formatting bar, borderless
on `--bg-raised` hover — the same hover as the `IconButton`s it sits between),
`fullWidth` (finance and projects, a select taking its column's width), and
`size="lg"` to match `Input`.

No `appearance: none`, and no listbox. The native control is the entire reason
a select is usable on a phone, and the seven copies were unanimous — every one
was a native `<select>` with a border drawn round it. Nobody was asking for the
thing that would have cost typeahead, Home/End and the platform picker.

What the copies were missing:

- **A name.** `shell/FiltersSection` ships two unnamed selects — the field and
  the operator of every filter condition — with no label, no `aria-label` and
  no wrapping `<label>`. A screen reader reads the current value where the
  question should be: "combo box, From". Nothing about a hand-rolled select
  could surface that, so `Select` says it in `console.error` in development
  only (`import.meta.env.DEV`, so the check leaves the production bundle). A
  required `label` prop was wrong here, unlike `Table`'s: a select has three
  legitimate ways to be named and two of them are already in use.
- **The empty option, and what it means.** Six call sites open with
  `<option value="">` and mean two different things by it. inventory's "All
  locations" is an answer somebody must be able to return to; billing's "Pick a
  product" is a prompt. `placeholder` renders it, and disables it only when the
  select is `required` — which is also the only spelling the browser's own
  required check understands, since a sentinel value passes it.
- **`max-width`.** Only billing had it. It is the difference between a long
  product name and a broken toolbar.

Verified: 7 behaviour tests (wrapping-label naming, `Field` naming + describing
+ invalid, the unnamed-select report and its absence when named, the
placeholder's position/value/selection, choosable when empty is an answer,
disabled when it is a prompt); `npx tsc --noEmit`, `npx eslint`, `prettier
--check`, `npm run build` and `npx vitest run src` (72 files, 653 tests) all
green. No screenshot — like `Table` and `Toolbar`, the component has no caller
until the D2 migrations, so the visual check lands with the first adoption. The
4 unhandled rejections from `sites/Theme.test.tsx` are still there:
pre-existing, another track's area, non-failing.

No CHANGELOG line: a primitive with no caller changes nothing a user can see.
The taller selects become user-visible at D2, and D3.01 is where the wave is
written up.

**Next:** D1.06 `Toggle`.

## D1.06 — `Toggle` and `Checkbox`

The seven copies were not seven copies of one thing. Two of them — `admin` and
`shell/SettingsModal` — drew a switch: a visually-hidden checkbox, a
`--radius-full` track, a `::after` knob, `translateX(16px)`. The other five put
the same class name on a row holding an ordinary checkbox and, in three of
them, on a row holding a `<select>`. `.toggle` had stopped meaning anything,
which is why the item ships two components and states the split in both files:

**A toggle is a setting; a checkbox is one option among several.** A toggle
applies itself and is announced on/off — "give this person admin access". A box
beside a search field that narrows a list is a checkbox. The `.toggle` rules
wrapping a `<select>` (billing/InvoicesView, hr/LeaveView, hr/HiringView,
crm/ReportView, billing/VatReportView) are neither: they are a labelled inline
field, and D2 resolves them with `Select` rather than a third primitive.

The two switches agreed on almost everything, including the 16px of travel they
had each derived independently, so the larger geometry (40×24, 18px knob) is the
base — 24px is the short side of the smallest target WCAG 2.5.8 accepts. The
four checkbox rows agreed on `inline-flex`, `--text-sm`, `--text-secondary` and
a gap of 6/8/8px; crm and hr were byte-identical, again.

What none of them had:

- **A focus ring on the switch.** The input is `opacity: 0` and neither copy
  styled `:focus-visible`, so tabbing down the admin user list — twenty
  switches, one per row — showed nothing at all (WCAG 2.4.7). The ring is drawn
  on the track, which is the control the eye sees.
- **A name.** `admin/UsersPage` names its switch with `title` on the wrapping
  `<label>` — a tooltip, not a name, and the label's own text is empty, so the
  control is announced as "checkbox, not checked" twenty times over.
  `admin/UserModal` puts `aria-label` on the `<label>` while the visible text
  sits in a sibling `<span>` bound to nothing. `label` is required and really
  bound. `billing/ProductDialog` and `shell/SettingsModal` put the checkbox row
  class on a `<span>`, so clicking their text does nothing — impossible now,
  because the label is the component's.
- **The state said as a state.** Both switches were plain checkboxes:
  "out of office" read as "checked". `role="switch"` on the native input, so the
  checkedness stays the platform's and only the wording changes.
- **The hint.** `shell/SettingsModal` draws one under every switch and describes
  none of them.
- **A disabled state.** Both switches disable on real conditions (your own admin
  row, an unconfigured provider) and drew the result identically to a live
  control. None of the four checkboxes had one at all.

Two visible changes, recorded rather than softened. **The off track gets
darker**: both copies filled it with `--border-strong`, 1.7:1 against the
surface, under the 3:1 WCAG 1.4.11 asks of a control's boundary — and the white
knob was 1.7:1 against the track it sits in. `--text-tertiary` is the same quiet
role in a fill as in type, and it is 4.8:1. **Every checkbox becomes
terracotta**: unstyled, the box is whatever the platform draws, which on Chrome
is a blue tick — the one place in the product where the accent colour was not
ours. One `accent-color` declaration, with the native control left alone.

Dropped as a caller's concern: billing's `margin-right: auto` on the checkbox
row, which was pushing a toolbar about; `ToolbarSpacer` is that. No
indeterminate state and no `ToggleGroup` — nothing in the repository has either,
and building for no caller is how a primitive grows a surface nobody asked for.

Verified: 11 behaviour tests (switch role and checkedness, label really bound,
hidden label still the name, hint described, reported state, drawn disabled
state, track not announced twice; and for `Checkbox`: own bound label, *not* a
switch, disabled, reported state). `npx tsc --noEmit`, `npx eslint`, `prettier
--check`, `npm run build` and `npx vitest run src` (73 files, 664 tests) all
green. No screenshot — like `Table`, `Toolbar` and `Select`, there is no caller
until the D2 migrations, so the visual check lands with the first adoption. The
4 unhandled rejections from `sites/Theme.test.tsx` are still there:
pre-existing, another track's area, non-failing.

No CHANGELOG line: a primitive with no caller changes nothing a user can see.
The darker off track and the terracotta checkboxes become visible at D2, and
D3.01 is where the wave is written up.

**Next:** D2.01, migrate authoring.
