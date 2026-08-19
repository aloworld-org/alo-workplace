# alo design system — migration queue

Forty-three stylesheets still declare a primitive that `ds/` should own, and
`web/src/ds/primitives.test.ts` names every one of them. This queue empties
that list.

**Read ADR 0045 and then ADR 0046 — in that order, because the second changes
half of the first.**

0045 diagnosed the failure: token discipline here is excellent — 7,422
`var(--token)` references against 108 hard-coded colours — so the values were
never the problem. What was missing is the layer above them, and CSS Modules
made the duplication invisible rather than caused it. **That diagnosis stands,
and it is why this queue exists**: a utility class string composes no better
than a stylesheet, so `ds/` still owns the primitives and
`primitives.test.ts` still enforces it.

0046 (2026-08-13, owner decision) changed the styling mechanism underneath:
**styles are now Tailwind utilities**, generated from these same tokens by
`web/scripts/gen-tailwind-theme.mjs`, so `--bg-surface` and `bg-surface` are
one definition with two spellings. 0045's paragraph rejecting Tailwind no
longer applies; its composition argument still does.

For this queue that means: **no new `.module.css` file is created**, the
primitives are restyled to Tailwind first (wave D1.5), and every module
migration after that adopts components which are already Tailwind — so a
migration deletes a stylesheet instead of trading one for another.

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

## Restyle the primitives to Tailwind first (wave D1.5)

ADR 0046 made Tailwind the styling mechanism. These come **before** the
remaining module migrations for one reason: every migration below adopts these
components, so converting them first means each migration deletes a stylesheet
rather than adopting a component that still carries one. Doing it the other way
round would migrate eleven areas onto CSS-Modules primitives and then restyle
underneath them, touching every area twice.

**What must not change:** the component's props, its behaviour, and its tests.
The accessibility work in these — focus traps, arrow-key movement, ARIA names,
the label bound to its control — was the expensive part and is untouched by a
styling change. A restyle that edits a test has changed behaviour and is wrong;
if a test genuinely must change, say why in the commit message.

**What changes:** the component's `.module.css` is deleted and its classes
become utilities from the generated theme. Use semantic utilities only
(`bg-surface`, `text-primary`, `border-subtle`) — the raw scale is deliberately
not exposed, and an arbitrary value (`bg-[#e76f51]`) is a defect, not a
shortcut. Where a component needs a value the theme lacks, add the token to
`ds/tokens.css` and re-run `node scripts/gen-tailwind-theme.mjs`; never inline
the literal.

**Verify by looking**, as everywhere else in this queue: one screenshot per
item, opened and read, plus `npx vitest run src` green and
`node scripts/gen-tailwind-theme.mjs --check` passing.

- [x] D1.51 Restyle the **form** primitives — `Input`, `Field`, `Select`,
  `Checkbox`, `Toggle`. The set a form is built from, and the one where a
  visual regression is most obvious. Done when: their `.module.css` files are
  gone, `Select.test.tsx` and `Toggle.test.tsx` are unedited and green, and a
  screenshot of a real form (Billing customer dialog) is read and reported.
- [x] D1.52 Restyle the **container** primitives — `Card`, `Modal`, `Dialog`.
  Overlay, elevation and focus trap are behaviour; only the surface, padding
  and radius are style. Done when: `Modal.test.tsx` is unedited and green and a
  screenshot of an open dialog shows the same elevation and scrim as before.
- [x] D1.53 Restyle the **data** primitives — `Table`, `Toolbar`. Zebra rows,
  header weight, alignment and the empty state; the keyboard movement in
  `Toolbar` is behaviour and stays. Done when: `Table.test.tsx` and
  `Toolbar.test.tsx` are unedited and green, and a dense screen (Inventory
  stock by location) is screenshotted and read.
- [x] D1.54 Restyle the **small** primitives — `Button`, `IconButton`, `Badge`,
  `Chip`, `Avatar`, `Spinner`, `Menu`, `DatePicker`, `ResizeHandle`. Several
  are a handful of rules each; keep them one commit so the shared button
  proportions are reconciled once. Done when: `Chip.test.tsx` is unedited and
  green, no `.module.css` remains under `ds/`, and a screenshot of the shell
  (rail, header, a menu open) is read.
- [x] D1.55 Wave check: `ds/` declares no `.module.css` at all; the generated
  theme is current (`--check`); `rg "\[#" web/src/ds` finds no arbitrary
  values; `npm run build` clean; and one screenshot each of a form, a dialog
  and a table sit in the journal with what was looked at written down.

## Migrate, area by area

**`web/src/sites/**` is not in this queue.** An autonomous loop is building
through the sites wave and cannot negotiate with you over a file. Those
stylesheets stay on the allow-list until that track migrates them itself, or
until it is idle and somebody says so — which means `redefined.ts` will not
reach zero from this queue alone, and D3.01 accounts for that.

- [x] D2.01 Migrate **authoring** (7 stylesheets — badge,chip,input,modal,table,toolbar) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [x] D2.02 Migrate **mail** (7 stylesheets — btn,card,chip,field,input,modal,select,toolbar) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [x] D2.03 Migrate **shell** (7 stylesheets — badge,button,card,field,input,modal,select,toggle) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [x] D2.04 Migrate **drive** (3 stylesheets — chip,dialog,input) to `ds/`: adopt the components, delete the local rules, remove the lines from `ds/redefined.ts`, and screenshot one screen from the area.
- [x] D2.05 Migrate **admin**, **agenda**, **auth** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [x] D2.06 Migrate **contacts** — chat was already done, and **billing** is split out as D2.06b below: it has no stylesheet to delete, so it is a different piece of work rather than half of this one.
- [x] D2.06b Migrate **billing** off `billingStyles.ts` — the one area that is already Tailwind and still reimplements the primitives: `input`, `select`, `chip`, `badge`, `table`, `toolbar` and `toggle` are all declared in that file and read by nineteen `.tsx`. `ds/redefined.ts` never listed it because it is not a stylesheet and `primitives.test.ts` cannot see it, which is the reason it survived the wave. Adopt the components, delete those keys, keep the ones that are billing's own layout (`page`, `editorHead`, `totals`…), and screenshot a ledger. Prerequisite, copied here rather than left in a guide: there is still no multi-line control in `ds/`, so `textarea` stays local until one exists.
- [x] D2.07 Migrate **crm** — cut down from "crm, finance, home" when crm alone turned out to be thirteen `.tsx` and a 678-line stylesheet, which is a whole item. Adopted, deleted, off `redefined.ts`.
- [x] D2.07b Migrate **finance** and **home** — the half of D2.07 that was cut, said as its own item. `finance/FinanceModule.module.css` declares `input`, `select`, `table`, `chip`, `toolbar`, `modal` and `field` across ~20 `.tsx`; **home has no stylesheet at all** and is not on `redefined.ts` — it is billing's problem in miniature, Tailwind recipes `primitives.test.ts` cannot see, so read `HomeModule.tsx` for hand-rolled cards and badges rather than trusting the list. Prerequisite, copied here rather than left in a guide: `home/HomeModule` writes `bg--soft` five times and that is not a class — it generates nothing (found in D2.06b); fix it while you are in the file.
- [x] D2.08 Migrate **importer** and **insights** — cut down from "hr, importer, insights" for the reason D2.07 was cut down from "crm, finance, home": `hr/hr.module.css` is **1,119 lines across thirteen `.tsx`**, declaring `card`, `chip`, `field`, `input`, `modal`, `table`, `toggle` and `toolbar`, which is a whole item and not a third of one. Adopted, deleted, off `redefined.ts`.
- [x] D2.08b Migrate **hr** — the half of D2.08 that was cut, said as its own item. Its 1,119-line stylesheet is the largest left in the queue and it is a module of five screens plus five dialogs (`HiringBoard`, `DirectoryView`, `LeaveView`, `AwayView`, `ApprovalsView`, `LetterTemplatesView`, and the applicant, hire, leave and opening dialogs). Prerequisites, copied here rather than left in a guide: **(a)** there is still no multi-line control in `ds/`, so `hr.module.css`'s `.textarea` stays local until one exists — it is now the sixth area waiting (billing ×3, crm, finance, hr); **(b)** `.card` and `crm`'s were byte-identical, hardcoded values included, and `ds/Card` says so in its own header — so the hiring board's cards are an adoption, not a judgement call; **(c)** `ApprovalsWidget.module.css` is a second stylesheet in the area and is *not* on `redefined.ts` — it declares no primitive — so read it rather than trusting the list, which is the lesson billing and home each taught once.
- [x] D2.09 Migrate **invite** and **meet** — cut down from "inventory, invite, meet" for the reason D2.07 and D2.08 were each cut down: `inventory/InventoryModule.module.css` is **626 lines across ten `.tsx`**, declaring `toolbar`, `select`, `toggle`, `table`, `chip`, `field` and `input`, which is a whole item and not a third of one. Adopted, deleted, off `redefined.ts`.
- [ ] D2.09b Migrate **inventory** — the half of D2.09 that was cut, said as its own item. Ten `.tsx` (the catalog, the stock list, the two order lists, the two order editors, the line grid, the consignments, the move-history dialog and the scanner) over a 626-line stylesheet. Prerequisites, copied here rather than left in a guide: **(a)** `ds/` still has no multi-line control, so `.textarea` stays local — inventory is the **seventh** area waiting, after billing ×3, crm, finance and hr ×2; **(b)** `parts.tsx` exports a module-local `Field` and a `StatusChip` whose five tones (`neutral`/`info`/`good`/`warn`/`muted`) are the return type of `format.ts`'s `poStatusTone` and `soStatusTone`, which map eleven order states onto them — `ds/Badge` has three, so the tone vocabulary stays the module's exactly as hr's `StateBadge` kept its, and only the drawing is adopted; **(c)** the module hand-rolls **two** dialogs (`.modalWide` for the movement history, `.scanModal` for the scanner) with no focus trap between them, which is the behaviour `ds/Modal` exists for and the reason this is a migration rather than a restyle; **(d)** `.search` puts no icon inside the field, unlike hr's, so it is a plain `ds/Input` and not an exemption.
- [ ] D2.10 Migrate **platform** — same loop: adopt, delete, remove the line, screenshot. **Projects was taken off `redefined.ts` by somebody outside the loops** (`c41851be refactor(projects): unify actions and migrate styles to Tailwind`, landed 2026-08-19 while D2.09 was being gated), which is the case LOOP describes: a commit in an area no journal claims. Read `web/src/projects/**` before assuming it is done to this queue's standard — an area that stops declaring a primitive has satisfied the ratchet, which is not the same as having adopted the components — and file what is missing as its own item rather than reopening theirs.
- [ ] D2.11 Migrate **tasks** — one commit, same loop: adopt, delete, remove the lines, screenshot.
- [ ] D3.01 Wave review: everything outside `web/src/sites/**` is off `ds/redefined.ts`; walk the product in a browser at desktop and phone width; check the CSS actually shipped has fallen from its 252 KB; and record in `CHANGELOG.md` what a user would notice.
