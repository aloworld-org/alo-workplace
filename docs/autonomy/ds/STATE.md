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

## D2.01 — authoring, the first adoption

Seven stylesheets off the list, 43 → 36. The authoring `.module.css` files lost
243 lines and gained 77, so the migration is a net deletion of 166 lines; `ds/`
grew by 64 for the two widenings below.

**Three of the seven were real adoptions, four were names that were lying.**
That split is the finding of this item, and it is worth stating plainly because
the next nine migrations will meet it again: the ratchet matches class *names*,
and in a document editor `.input` usually means "the surface you type the
document into", not "the form control".

Adopted:

- **`AuthoringInsertModal`** and **`EquationEditor`** → `ds/Modal`, `Button`,
  `IconButton`, `Checkbox`. Both hand-rolled the overlay and the panel, and
  between them they handled Escape once, on a keydown bound to the panel — so
  the equation dialog, which is the one with a symbol palette you can get lost
  in, had no keyboard way out at all. Neither trapped Tab. Both now inherit all
  of it.
- **`ParagraphBlock`'s** insert row → `ds/Toolbar`. `keyboard="tab"`, not
  `roving`: the reference button opens a picker *inside* the row, and roving
  focus would have swept the picker's own buttons into the toolbar's arrow-key
  set. That is the first real use of the choice D1.04 argued for.
- **`TableBlock`** → `ds/Table` with `grid` and `flat`. It had exactly the two
  defects D1.03 found in all ten data tables — a scroll region a mouse could
  reach and a keyboard could not, and no name — plus no `scope` on any column
  header. A numbered table is now named by its number, which is what a
  cross-reference points at.
- **`CrossReference`'s** `.chip` → `ds/Badge`. By the law D1.02 wrote down it
  was never a chip: nothing happens when you click it.

Renamed, with the argument in each file:

- `CodeBlock.input` → `.codeArea` — a transparent-text textarea laid over the
  highlighted `<pre>` so the caret sits on Prism's glyphs. It has no box and no
  height of its own; a `ds/Input` on top of the code is not the same object.
- `CodeBlock.badge` → `.langBadge` — a 22px square carrying the language's own
  brand colour, one of eighty. A `Badge` states a fact in one of four *system*
  tones.
- `HeadingBlock.input` → `.title` — 2rem type, no border, `font: inherit`.
  Putting a 40px bordered field around a document's heading turns a page into a
  form.
- `ParagraphBlock.input` → `.textArea` — the box, the border and the focus ring
  belong to the `.editor` around it, so that what you are editing still looks
  like the prose it will become.
- `EquationEditor.input` → `.latex`. This is the one that is genuinely
  form-shaped, and it stays local because **the design system has no multi-line
  text control**. There are 27 `<textarea>` elements across 25 files, each
  styled where it stands; that is a `Textarea` primitive with more demand than
  `select` had (7) and it should be argued for as its own item rather than
  invented mid-migration. **Flagged for D3.01.**

Two widenings, both stated in their files:

- `Modal.icon` — a decorative, `aria-hidden` mark before the title. Both
  authoring dialogs open with an accent glyph (Σ, `</>`) that says which editor
  you are in before the words do; dropping them would have been a loss the
  migration did not need to cause.
- `Modal.tall` — a fixed panel height with the body's own scrolling handed to
  whichever child asks for it. The symbol palette is a browser inside a dialog:
  without this, every keystroke in the search field resizes the popup under the
  pointer, and the equation you are writing scrolls away from the symbols you
  are picking.
- `Table.grid` — every cell bounded on all four sides and no cell padding,
  because each cell is filled edge to edge by the control that edits it. A data
  table is read and a grid is typed into; that is the whole difference, and the
  name, the caption and the reachable region are shared. Verified in the
  *shipped* CSS, not just the source: `._grid_… th, td` and `._grid_…  tbody
  tr:last-child td` both emit after the `._table_…` rules they tie with on
  specificity, so the editable grid still draws exactly as it did.

**Visible changes, recorded rather than softened.** Both dialogs move from
`--bg-app` porcelain to the shared `--bg-surface` ivory, from 640/600px to the
shared 720px `wide`, and onto the system's darker backdrop — and two rules
inside the equation palette had to follow the surface they sit on, or they
would have disappeared into it: the symbol tiles were `--bg-surface` (a tile on
the panel's own colour is not a tile) and the sticky category heading was
`--bg-app`. A broken cross-reference moves from copper to the `danger` tone:
copper is the *secondary accent* in this system, so an error drawn in it read
as decoration. The paragraph's two insert buttons become `Button ghost sm` —
larger and bordered where they were 3px chips.

Verified: 16 behaviour tests in `authoring/blocks.test.tsx`, each naming what
the hand-built version did instead — the reachable named scroll region, the
table named by its number, `scope="col"`, Escape and the Tab trap on the
equation dialog, the bound "Numbered" checkbox, the toolbar as one named group
with every control keeping its tab stop, ⌘/Ctrl+Enter still inserting code, and
a broken reference that says so in words. `npx tsc --noEmit`, `npx eslint
src/authoring src/ds src/i18n --max-warnings 0`, `npm run build` and `npx
vitest run src` (74 files, **680** tests) all green. The 4 unhandled rejections
from `sites/Theme.test.tsx` are still there: pre-existing, another track's
area, non-failing.

**Cut: no screenshot.** This environment has no browser and no screenshot tool
(no Playwright or Puppeteer in `web/`, and nothing in `scripts/`), so the
queue's "look at it" step cannot be done here as written. What replaced it: the
16 DOM-level assertions above, a check that no `styles.x` reference in the
seven files points at a rule that no longer exists, and the cascade check
against the built CSS described under `Table.grid`. That is not the same as
looking, and it does not catch a screen that has become ugly — **the first
human to open Docs should read this entry first.** D3.01 walks the product in a
browser and is where this is settled; if the loop is expected to keep this
step, the environment needs a headless browser and that is a request for a
human, not a thing to work around.

Also noticed and left alone (pre-existing, not this item's): `EquationEditor`
references `styles.catSection` and `styles.math`, neither of which has ever
existed in that stylesheet.

**Process defect in this iteration's own commit:** `cc92ac5` went out without
the `Co-Authored-By: Claude …` trailer LOOP step 8 asks for — the harness did
not append one and the message was written without it. It is already pushed and
rewriting history is forbidden, so it stands as it is. Every commit from here
writes the trailer explicitly rather than relying on the harness.

**Next:** D2.02, migrate mail.

## D2.02 — mail

Seven stylesheets off the list, 36 → 29. The mail `.module.css` files and their
`.tsx` lost 419 lines and gained 181 — a net deletion of 238 — and `ds/` grew by
95 for the two widenings below.

**Six of the seven were real adoptions and one was a name that was lying**, the
reverse of D2.01's split. `mail` is chrome rather than documents, so a `.chip`
here really was a chip.

Adopted:

- **`FlagDueControl`** → `ds/Chip` (button form) and `ds/Input`. The pill was a
  hand-drawn button carrying a `title` and nothing else: a screen reader
  announced a button that did something unspecified when pressed, with no hint
  that a menu was behind it and no way to know whether it was open. It now says
  both.
- **`RecipientInput`** → `ds/Chip` (removable form). This one was already close
  — it is where `Chip.removeLabel` came from at D1.02 — so the migration is
  mostly deletion.
- **`SnoozeMenu`** → `ds/IconButton`. It drew *two* triggers, one for the
  toolbar and one for the list's bulk bar, differing only in corner radius and
  two pixels of icon. Neither difference was a decision anybody made twice.
- **`InvitationCard`** → `ds/Card` (`pad="sm"`) and `ds/Button`. Its three RSVP
  buttons were styled by a bare `.actions button` descendant rule with an
  `!important` triplet on top of it for Accept — and two of the three referenced
  `styles.tentative` and `styles.decline`, which have never existed in that
  stylesheet.
- **`ReadingPane`** → `ds/Toolbar` (`surface="bar"`, `density="compact"`),
  `ds/Button`, `ToolbarSpacer`. Its own media query — "on a phone the toolbar
  can overflow; let it wrap" — is gone, because a `ds/Toolbar` wraps at every
  width by default.
- **`RichTextEditor`** → `ds/Toolbar`, `ds/Select variant="ghost"`,
  `ds/IconButton`, `ToolbarDivider`. This is the bar that argued three of the
  D1 components into existence, and it is the one that had `role="toolbar"`
  with no arrow keys, no Home and no End behind it. It is now honestly a named
  group (`keyboard="tab"`): the row holds two selects and two colour pickers,
  where arrow keys belong to the control and not to us.

Renamed, with the argument in the file:

- `ComposeModal.modal` → `.window`. Not a styling call: `ds/Modal` is a
  blocking dialog — centred, focus trapped, Escape closes it, the page behind
  inert — and this is the opposite object. It docks in a corner so the rest of
  mail stays usable while you write, it minimizes to its own title bar, and
  Escape must not throw a half-written message away. Only its `full` view is
  modal and that view already says so. The name is `window` so nothing invites
  the confusion again.
- `ComposeModal.fields` → `.headers`. A container that only decides where the
  recipient and subject rows sit; each row is its own control.
- `RecipientInput.input` → `.entry`, `.field` → `.tokens`. The tail you type
  the next address into has no box of its own: the border and the rule under it
  belong to the row, which is one field holding many recipients. A `ds/Input`
  there would draw a second field inside the first — the same argument as
  `ParagraphBlock.textArea` at D2.01.

Also adopted although the ratchet does not match their names, because leaving
them would have been the D2.01 finding in reverse: `ComposeModal.iconBtn` (four
call sites) and `.fromSelect` → `ds/IconButton` and `ds/Select`;
`ReadingPane.textBtn` → `ds/Button variant="ghost"`.

Two widenings, both stated in their files:

- **`Chip` gained a button form.** A chip is either a button (`onClick`, mail's
  follow-up date, which opens a menu) or a chip with a button in it
  (`onRemove`, a recipient). Both at once nests a button inside a button, which
  renders perfectly happily and then swallows one of the two clicks, so asking
  for both is reported in development rather than quietly resolved. It also
  gained `tone`, named and matched to `Badge`'s, since the follow-up chip
  carries three states.
- **`IconButton` stopped claiming to be a toggle.** It set `aria-pressed` on
  every button it rendered, so the twelve tools of the formatting bar were each
  announced "not pressed" over text that may well have been bold — by a control
  that tracks no state at all. `active` is now optional and `aria-pressed` is
  written only when a caller passes it. The five callers that do (the rail, the
  flag, sites' preview width) are unchanged.

**Visible changes, recorded rather than softened.** The follow-up chip moves
from an outlined pill to a filled one — it was the only chip in the product
drawn with a border, and `danger`/`accent` say what its border colour used to.
Reply all and Forward become outlined `ghost` buttons beside Reply, where they
were borderless text. The invitation card moves from `--bg-raised` to the
system's `--bg-surface` with `--shadow-sm`, so it is lifted off the reading pane
rather than tinted against it. The compose window's From picker and the two
formatting dropdowns get the shared select box (the D1.05 height change,
arriving here). The formatting bar takes `Toolbar`'s `bar` padding, so the
editor wrapper handed its horizontal padding to the toolbar and the body
separately.

Verified: 12 behaviour tests in `mail/components/adoption.test.tsx` and 3 in
`ds/Chip.test.tsx`, each naming what the hand-built version did instead — the
due chip's `aria-haspopup`/`aria-expanded`, overdue said in words, the picked
date still reporting end-of-day rather than midnight, the snooze trigger no
longer announced as an unpressed toggle, each recipient's remove button naming
its own recipient, removal keeping the rest, the formatting bar as a named group
with no `role="toolbar"`, `mousedown` still prevented so a tool does not steal
the caret from the body, and both dropdowns still named. `npx tsc --noEmit`,
`npx eslint src/mail src/ds src/i18n --max-warnings 0`, `npm run build` and
`npx vitest run src` (76 files, 694 tests) all green. A dangling-class check
across all seven files found no `styles.x` pointing at a rule that no longer
exists. The 4 unhandled rejections from `sites/Theme.test.tsx` are still there:
pre-existing, another track's area, non-failing.

**Two test files arrived as inherited work.** `ds/Chip.test.tsx` and
`mail/components/adoption.test.tsx` were sitting untracked in the checkout at
the start of this iteration — written by an earlier D2.02 attempt whose
implementation the wrapper discarded, exactly as the headless-discipline
section warns. They were treated as a specification and the implementation
written to meet them; both pass. Worth knowing because it means the tests were
written before this code and not against it.

**Cut: no screenshot, same as D2.01.** There is still no browser and no
screenshot tool in this environment (no Playwright or Puppeteer in `web/`), so
the queue's "look at it" step cannot be done as written. What replaced it: the
15 DOM-level assertions above, the dangling-class check, and a production build.
That is not the same as looking. The shipped CSS is now 909.8 KB across four
files — the number D3.01 is asked to compare against 252 KB, which does not
match anything measured here and should be re-derived rather than trusted.

**Flagged for D3.01, carried forward from D2.01:** the design system still has
no multi-line text control, and mail adds callers to the 27 `<textarea>`
elements already counted. Still worth its own item rather than an invention
mid-migration.

**Next:** D2.03, migrate shell.

## RESOLVED HALT: two loop invocations are running on this one checkout

Nothing was built this iteration, and nothing should have been. Two headless
`claude -p "… execute exactly ONE iteration of the loop for track 'ds' …"`
processes were running against `C:\dev\Ficina-loop` at the same time:

| PID | Started | Track |
|---|---|---|
| 25244 | 2026-08-13 00:42:32 | ds |
| 28176 | 2026-08-13 00:45:53 | ds — **this invocation** |

Both had picked up the same first-undone item, D2.03. This is the condition
CLAUDE.md forbids outright: *"Concurrent editors on one checkout are forbidden
— a second editor produces uncommitted, ambiguously authored work that cannot
be trusted."*

**How it was noticed, which is the part worth keeping.** The tree was clean at
the start of this iteration (`git status` after the pull showed only the
untracked `shell/adoption.test.tsx`). Six tool calls later — while this
invocation had done nothing but read — `git status` showed seven modified
files, and `AgentActionCard.module.css` joined the list between two consecutive
calls. Reading them showed work that was plainly D2.03: `ComingSoon.module.css`
had already renamed `.badge` to `.plate`, `ComingSoon.tsx` had grown the
`aria-hidden` plate, and `AgentActionCard.tsx` had already adopted
`<Card pad="sm" flat>`. A file changing under a reader who has not written is
the whole signature of this bug, and it is invisible unless the tree is
re-checked rather than assumed.

**Why this invocation is the one that stopped.** Walking its own process
ancestry put it at PID 28176 — the *younger* of the two, by three minutes. The
older process owns the tree and is mid-item; overwriting its half-written files
to "finish" D2.03 would produce exactly the ambiguously-authored commit the
rule exists to prevent. So this iteration touched no code, no CSS, no
`redefined.ts` and no QUEUE entry. D2.03 stays `[ ]` and belongs to PID 25244.

Exiting non-zero stops *this* wrapper, which leaves exactly one loop running —
the outcome we want — but it does not fix the cause. **For the human:** find
the second wrapper (its parent is a Git-bash `sh.exe` running
`/c/Users/SBW/AppData/Roaming/npm/claude -p …`) and stop it before restarting,
or every future iteration is doubled too. A loop that races itself does not run
twice as fast; it discards one of the two results and cannot say which.

**Inherited work still sitting in the tree, unclaimed:**
`web/src/shell/adoption.test.tsx`, untracked — 15 behaviour tests written as a
D2.03 specification by an earlier attempt whose implementation the wrapper
discarded, the same way `mail/components/adoption.test.tsx` arrived during
D2.02. Whichever invocation completes D2.03 should treat it as the spec and
commit it with the implementation.

**Next:** D2.03, still — by the single loop that remains.

## The race is still running, and now we know why it survived a halt

Nothing was built this iteration either. The previous entry's diagnosis was
correct and its instruction ("find the second wrapper and stop it") was never
carried out — but blaming the human is the wrong reading. **Two mechanical
bugs kept the race alive, and both are in `scripts/run-loop.sh`.** They are
worth more than another head-count of processes, so this entry is mostly them.

Live evidence at 00:55, gathered before touching anything:

| PID | Started | What it is |
|---|---|---|
| 25244 | 00:42:32 | `claude -p … 'ds'` — **orphaned**; its wrapper (38432) is dead |
| 39840 | 00:52:18 | `claude -p … 'ds'` — **this invocation**, spawned by wrapper 41372 |
| 41372 | 00:45:52 | `bash scripts/run-loop.sh … ds` — holds `~/.alo-loop-ds.lock` (PID 19658) |

The orphan is not idle. `web/src/shell/SettingsModal.module.css` was written at
00:53:22 and `web/src/ds/redefined.ts` at 00:53:27 — the latter *twelve seconds
after* a `git status` in this session that did not list it. A file appearing in
the tree between two consecutive reads, again, by a reader that has written
nothing. D2.03 is visibly mid-flight in another process: `redefined.ts` is
being edited right now, which is step 3 of the migration loop.

### Bug 1 — the halt marker was written in a form the wrapper cannot see

`run-loop.sh` stops the loop with `grep -qE '^LOOP HALT'` (line 62), anchored
deliberately so that prose *quoting* the marker is not mistaken for the marker.
The previous iteration wrote its marker as a markdown heading:

    ## LOOP HALT: two loop invocations are running on this one checkout

`^LOOP HALT` does not match `## LOOP HALT`. Checked directly: `grep -nE
'^LOOP HALT' docs/autonomy/ds/STATE.md` returns nothing, while an unanchored
grep finds it at line 521. So the wrapper read the journal at 00:52, saw no
halt, and started this invocation six minutes after the halt that was supposed
to stop it. The halt was real, committed (`fc44571`), correct — and invisible.

`LOOP.md` says *append `LOOP HALT: <reason>`*, which means at column 0. A
heading is the natural thing to write in a prose journal and it silently
disarms the stop. **This entry's marker is a bare line at column 0**, which is
the whole difference between this halt and the last one. The same trap applies
to `^LOOP COMPLETE`; the sites journal already lost an iteration to the mirror
image of it (see the comment at `run-loop.sh:52`).

### Bug 2 — the lock guards wrappers, but a worker outlives its wrapper

The single-wrapper lock (lines 28–43) does exactly what it claims and is not at
fault for what it covers: one live wrapper per track per machine, stale locks
taken over. The gap is that `trap 'rm -f "$LOCK"' EXIT` releases the lock
without killing the `claude -p` child it started. So:

1. Wrapper 38432 starts worker 25244 at 00:42:32.
2. Wrapper 38432 dies. Its trap removes the lock. **Worker 25244 keeps running**
   — an unattended `claude -p` is not in the wrapper's process group on Git
   Bash and nothing signals it.
3. Wrapper 41372 starts at 00:45:52, finds a dead PID in the lock, correctly
   reports "stale lock — taking over", and starts a rival worker beside a
   process it has no idea exists.

Every step is the lock behaving as designed. The invariant it enforces is "one
wrapper", and the invariant that matters is **"one worker"**. The suggested
fix, for the human rather than for this iteration: on takeover of a stale lock,
kill any surviving `claude -p` for this track before iterating, and change the
`EXIT` trap to `kill` the child first and remove the lock second.

### What this invocation did not do

No code, no CSS, no `redefined.ts`, no QUEUE line. D2.03 stays `[ ]` and
belongs to PID 25244, which is ten minutes older and mid-item; overwriting its
half-written files would produce exactly the ambiguously-authored commit
CLAUDE.md forbids. Only this journal entry was written, and it was committed
path-limited (`git commit docs/autonomy/ds/STATE.md`) so that the orphan's
in-flight working tree was left untouched.

**For the human, in order:** (1) let PID 25244 finish and commit D2.03, or kill
it and let the next iteration redo the item from clean — either is fine, but
not both at once; (2) fix the two bugs above in `scripts/run-loop.sh`; (3)
delete this halt marker; (4) restart. `web/src/shell/adoption.test.tsx` is
still untracked and is still the D2.03 specification — whichever invocation
completes the item commits it with the implementation.

**Next:** D2.03, unchanged.

RESOLVED HALT: a second, orphaned loop worker (PID 25244) owns this checkout and is mid-D2.03; the previous halt marker was a markdown heading, so run-loop.sh's anchored `^LOOP HALT` never matched it and the wrapper restarted anyway.

## D2.03 — shell

Seven stylesheets off the ratchet in one commit: `AgentActionCard`,
`AgentResultCard`, `ComingSoon`, `FiltersSection`, `SearchOverlay`,
`SettingsModal`, `SharingSection`. `ds/redefined.ts` is down from 29 files to
22, and the whole `shell/` block is gone from it. Net −259 lines across 21
files, which is what a migration should look like.

**The tree was clean when this started.** `git status` after the pull showed
one untracked file and no modifications, so the orphan described in the two
halt entries above (PID 25244, still listed in the process table, last write
recorded at 00:53) had left nothing behind: its D2.03 was discarded by the
wrapper exactly as the headless-discipline section warns. Every modified file
below was written by this invocation, and nothing appeared in the tree between
reads at any point during it.

Adopted:

- **`SettingsModal`** → `ds/Modal` (`wide`, `tall="page"`), `ds/IconButton`,
  `ds/Toggle`, `ds/Field` + `ds/Input`, `ds/Button`. This was the largest
  single deletion of the item — 186 lines of the stylesheet, most of it the
  overlay, the panel, the head, the footer row and a second hand-built switch.
  What it gained is the behaviour: **no key handler existed anywhere in the
  file**, so the only way out of Settings was the mouse, and Tab walked
  straight out of the panel onto the mailbox behind it.
- **`FiltersSection`** → `ds/Select` ×3, `ds/Input` ×2, `ds/Checkbox` ×4,
  `ds/IconButton` ×2, `ds/Button`. It also stopped importing
  `admin/admin.module.css` for two classes — a cross-module stylesheet import
  for `.input` and `.error`, which is the duplication problem wearing a
  different hat. `.error` is now local, byte-for-byte what admin drew.
- **`SharingSection`** → `ds/Input`, `ds/Select` ×2, `ds/Checkbox` ×3,
  `ds/Button` ×3, `ds/IconButton` ×2.
- **`AgentActionCard`** and **`AgentResultCard`** → `ds/Card`
  (`pad="sm" flat`) and `ds/Button`. The result card draws that surface in ten
  places, so it is named once as a local `ResultCard` rather than repeated ten
  times.

Renamed, with the argument in the file:

- `SearchOverlay.input` → `.query`. Deliberately **not** a `ds/Input`: the
  field there is the whole row — magnifier, text and close button sharing one
  border — and a bordered 40px control dropped into the middle of it draws a
  second field inside the first. Same argument as `mail/RecipientInput.entry`
  at D2.02 and `authoring/ParagraphBlock.textArea` at D2.01. It is also the
  only text entry in the product set at `--text-lg`, because a command
  palette's query is the thing you are looking at.
- `ComingSoon.badge` → `.plate`. A badge states a fact in words and is read;
  this is a 56px decoration above a heading that already says what the screen
  is, so it is now `aria-hidden`. `ModuleSwitchedOff` shares the stylesheet and
  moved with it.
- `AgentActionCard.field` → `.fact` (a label beside a value that cannot be
  edited, not a labelled control), `.buttons` → `.decide`, `.card` → `.stack`.

**One widening, stated in `ds/Modal.tsx` and in its stylesheet.** `tall` gained
a `"page"` value: a dialog that is a *place* rather than a question — its own
navigation, its own sections. It takes `min(720px, 86vh)` instead of its
content's height, so moving from General to Filters does not resize the panel
under the pointer, and its body is flush, because a two-pane layout draws its
own edges and padding there would push the nav column off the panel's side.
Settings was the only one of the sixteen `.modal` declarations that was a
place; `tall`'s existing 620px boolean is unchanged and its two authoring
callers are untouched.

**Visible changes, recorded rather than softened.** Settings is 60px narrower
(780 → `wide`'s 720) and its height is capped at 720px rather than 86vh of a
tall display — one width instead of a seventeenth one. Its header glyph loses
the 32px accent plate and becomes the shared accent mark. Close and Cancel, in
Settings and in the rule editor, become outlined `ghost` buttons where they
were borderless text — the same change mail took at D2.02. The out-of-office
subject box gains a visible label and loses its placeholder. Every select in
the shell moves from 34px to the shared 40px and gains a real focus ring and a
readable disabled state, where sharing's dimmed the whole control to
`opacity: 0.55` and took its text under the contrast floor. Filter checkboxes
get `accent-color`, so the one place in the product drawing Chrome's blue tick
now draws ours.

Verified: 17 behaviour tests in `shell/adoption.test.tsx`, each naming what the
hand-built version did instead — Escape closing Settings, the focus trap
returning Tab from the last control to the first, the out-of-office switch
announced as a switch with its hint described, a rule's box saying which rule
it belongs to (and an unnamed rule named by what it does), each condition's
three controls numbered, the folder picker out of the checkbox's label, both
sharing buttons naming the colleague, the folder button's `aria-expanded`,
`ds/Button`'s `type="button"` default not silently breaking the Add form, the
placeholder plate hidden from assistive technology, and a running agent action
refusing both decisions. `npx tsc --noEmit`, `npx eslint src/shell src/ds
src/i18n --max-warnings 0`, `npm run build` and `npx vitest run src` (79 files,
733 tests) all green. A dangling-class check both ways across the seven files
found no `styles.x` pointing at a deleted rule and no rule left behind with no
caller. The 4 unhandled rejections from `sites/Theme.test.tsx` are still there:
pre-existing, another track's area, non-failing.

**The test file arrived as inherited work**, the third time in three items:
`shell/adoption.test.tsx` was sitting untracked at the start, written as a
D2.03 specification by an earlier attempt whose implementation was discarded.
It was treated as the spec and the implementation written to meet it; all 17
pass. Several of its assertions name strings that did not exist yet, so eight
keys were added to all three catalogs (`UNTRANSLATED` is empty and must stay
that way): `filterConditionField/Op/Value`, `filterRemoveConditionAt`,
`filterRuleEnabled`, `filterFolderLabel`, `delegateRemoveFor`,
`delegateFoldersFor`. `filterRemoveCondition` is now unused; it stays, because
en/fr/nl are shared files that only take additive lines.

**Cut: no screenshot, the same cut as D2.01 and D2.02.** There is still no
browser and no screenshot tool in this environment — no Playwright or Puppeteer
in `web/`, checked again this iteration — so the queue's "look at it" step
cannot be done as written. What replaced it: the 17 DOM-level assertions above,
the two-way dangling-class check, and a production build.

**For D3.01, a reproducible CSS number.** `ls dist/assets/*.css` after
`npm run build` totals **932,239 bytes across 27 files**. D2.02 recorded
"909.8 KB across four files", which cannot be the same measurement — four files
is not what this build emits — and the queue's 252 KB matches nothing measured
here. D3.01 should re-derive the baseline with a stated method rather than
compare these three numbers to each other.

**Still flagged for D3.01, carried forward from D2.01 and D2.02:** the design
system has no multi-line text control. Settings' out-of-office message is now a
`<textarea>` inside a `ds/Field` — labelled and described correctly, but drawn
by a local rule, which is the honest shape of the gap.

**The halt marker above is deliberately left in place.** D2.03 is done and
committed, but neither cause of the race is fixed: PID 25244 is still in the
process table, and both `run-loop.sh` bugs (the `EXIT` trap that frees the lock
without killing its worker, and stale-lock takeover that does not look for a
surviving one) are untouched. Removing the marker would restart a wrapper
beside a process it cannot see, which is exactly what the last two entries
document. For the human: fix those two, confirm 25244 is gone, then delete the
marker and restart.

**Next:** D2.04, migrate drive (3 stylesheets — chip, dialog, input).

## Halts cleared, 2026-08-13 04:40 — by the operator's session, not the loop

Both markers above are resolved and rewritten as `RESOLVED HALT:` so the
wrapper can start.

**The orphan was mine.** Stopping the loop at 00:45 killed the *wrapper* and
left its *worker* alive, so the restart put two workers on one checkout. That
is the correct thing for the loop to have halted on, and the lesson is that
stopping a loop means killing the worker as well as the wrapper — a `pkill -f
run-loop.sh` does not reach the child.

**The invisible marker was mine too.** The anchored `^LOOP HALT` I added
yesterday stopped the wrapper reading a *prose* mention as a real marker, and in
doing so stopped it reading a marker somebody had written as a heading. Both
wrappers now accept an optional heading prefix, which the journal's own
quoting style keeps unambiguous. Verified against this file: two real markers
seen, four prose mentions ignored.

**Also carried forward for D3.01:** the shipped CSS figure in the queue (252 KB)
does not match what the build actually emits (909.8 KB across four files). The
review item should re-derive it rather than compare against a number this
session wrote down wrongly.
