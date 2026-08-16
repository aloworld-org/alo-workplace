# ADR 0051 — alo owns the chart in a sheet

**Status:** accepted (2026-08-16)
**Context:** `web/src/drive/SheetEditor.tsx`, `platform/alo-ai/src/sheet_grid.rs`,
`web/src/insights/chart/`, ADR 0033 (native editors on embedded engines),
ADR 0037 (Insights), ADR 0034 (per-product agents)
**Supersedes** the assumption in `features.md` that charts in alo Sheets were a
matter of wiring another Univer plugin.

## The decision in one line

A chart in an alo Sheet is **alo's own record** — kind, title, and the ranges it
reads — stored beside the workbook and drawn with the chart engine this product
already ships, not a structure owned by the grid engine's commercial plugin.

## What was actually wrong

The agent queue tried to build chart-from-intent and found there was nothing to
build it on. The investigation was correct and worth keeping:

- `SheetEditor` registers eleven Univer presets and **none of them is a chart**.
- The only implementation in that ecosystem is **`@univerjs-pro/sheets-chart`**,
  a Univer **Pro** package with no `license` field of its own, present in
  `node_modules` only as a transitive dependency and imported nowhere.
- `importOffice.ts` reads `.xlsx` into `cellData` and **drops charts by
  construction**; the export path writes none either.

Create, import and export are all chartless, so no alo workbook can hold a
chart and no fixture of one can exist. The conclusion drawn at the time was that
this needed a commercial dependency or a native charting effort, and that the
whole thing was therefore expensive.

**That was wrong on the facts.** alo already ships a chart engine:
`echarts@6.1.0`, **Apache-2.0**, bundled with no CDN and no runtime network,
lazily loaded, drawing in alo's own design tokens — and already behind a neutral
model of ours (`insights/chart/model.ts`), with `EChart.tsx` documented as *the
only file in alo that imports a chart library* precisely so the engine stays a
dependency rather than an architecture.

The expensive part of "build charts natively" — a licensed, themed, bundled
renderer — was done and paid for a wave ago. What was missing was never
rendering. It was **where a chart lives**.

## Where a chart lives

An alo Sheet is a Drive node whose blob is, in the words of the editor's own
comment, *"a Univer workbook snapshot — an opaque JSON object we persist
verbatim."* The editor asks Univer for a snapshot and stores what it gets, so
**anything alo adds to that object is lost on the next save**: the snapshot is
regenerated from Univer's state, not merged into.

That is the real reason charts looked to need a Univer plugin. A plugin is how
you get a foreign structure to survive Univer's round-trip.

The alternative is not to put it through the round-trip at all. **The stored
blob becomes an envelope:**

```json
{ "schemaVersion": 1, "workbook": { …the Univer snapshot, still verbatim… },
  "charts": [ … alo's own records … ] }
```

Univer keeps the grid and never sees the charts; alo keeps the charts and never
parses the grid engine's plugin structures. A workbook whose blob has no
`workbook` key but does have `sheets` is the old bare snapshot and is read as
one, so every sheet that exists today opens unchanged and gains an envelope the
first time it is saved.

**A chart therefore survives an engine swap**, which is the property ADR 0033
asks for and which a plugin-owned structure would have destroyed — the reason
this is worth doing even if the Pro plugin were free.

## What a chart is

alo's vocabulary, borrowing neither Univer's nor ECharts':

- an **id** and a **title**;
- a **kind** — bar, line or pie, the three `EChart.tsx` already draws;
- the **tab** it reads from;
- a **categories** range in A1 notation, and one or more **series**, each a name
  and a range.

A chart is a **view of the grid, never a copy of it**. It stores ranges, not
values, so a chart cannot disagree with the cells it came from — the same rule
that makes the Sheet agent cite an address instead of restating a number.

## Consequences

- **Chart-from-intent becomes an ordinary agent tool.** It proposes a chart
  record over a range `sheet_read` already handed the model, and the approved
  write is a record in the envelope. The agent queue was right to demand a
  reader before a writer; this ADR supplies the thing to read.
- **`.xlsx` interop is explicitly out of scope and stays honest.** Excel's chart
  parts are a different structure entirely, import drops them today, and export
  will not write one. A chart that works in alo and does not survive a
  round-trip to Excel is still worth having; claiming otherwise is how the old
  features line came to be wrong. `features.md` states the limit.
- **`Workbook::read` learns one unwrapping step**, in one place, and every
  existing caller keeps working.
- **No commercial dependency**, so no argument about a proprietary package
  inside a sovereignty product — an argument this ADR would rather not win than
  have.

## Rejected

- **`@univerjs-pro/sheets-chart`.** A commercial licence in a product sold on
  sovereignty, for a capability whose engine we already own outright, and it
  would put the chart inside the grid engine where an engine change loses it.
- **A Univer resource registered through `IResourceManagerService`.** It would
  survive the round-trip, and it is the standard way to make a plugin structure
  persist — but it couples alo's document to a Univer internal for no gain over
  an envelope, and it keeps the chart hostage to the engine.
- **A second blob beside the workbook.** Two Drive versions to keep in step,
  and a sheet whose chart list is one save behind its grid is worse than either.
- **Storing computed values in the chart.** Faster to draw and permanently
  capable of contradicting the sheet.
