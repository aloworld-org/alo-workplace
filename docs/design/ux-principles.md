# alo UX principles — the interface laws

Owner mandate (2026-08-08): alo's interfaces must feel like the best UX the
user has ever met, grounded in interaction psychology — and **no one should
ever need a menu, manual, or tour to know how an alo app works.** These laws
bind every UI built by any hand (loops, Codex, humans). Each law names its
psychological basis and a verification a wave review can actually run.

## The prime law: zero-manual

**If a screen needs explaining, the screen is wrong — and the user must
NEVER need a menu to use an alo tool.** Everything required to do the job
lives on the surface, in sight, in reach; menus exist for depth and rarities,
never for discovery of the core flow. A first-time user must achieve the
screen's core task in under a minute with no help, no menu-hunting, no tour.
Tooltips and docs may _deepen_ understanding, never _enable_ it.
_The menu test:_ remove every menu from the screen in your head — can the
core task still be completed with what remains visible? If not, redesign.
_Verify:_ the wave review walks each new screen as a stranger; any step that
required knowing-in-advance is filed as a defect, like S1.30b/c were.

## Ten laws that implement it

1. **Recognition over recall** _(Nielsen heuristic)_ — every available action
   is visible where it is used; nothing important lives only in a context
   menu, keyboard shortcut, or memory. Menus may duplicate, never gatekeep.
   _Verify:_ core tasks completable with visible controls alone.
2. **Meet expectations users already own** _(Jakob's law)_ — people arrive
   trained by the best-known tools of EACH domain, and every alo module must
   match the reflexes of ITS OWN world: Mail/Agenda → Outlook & Gmail; Sheets
   → Excel; Docs → Word & Google Docs; **Sites → Wix/Squarespace-class
   builders; CRM pipelines → Trello-style boards; Billing/ERP → the flows an
   accountant knows (minus SAP's cruelty); Insights → **Power BI / Tableau-class visualization quality — their output,
   never their learning curve**; Chat → Slack/WhatsApp**. Before designing a screen, name
   its domain references and match their reflexes — then beat them on clarity.
   Innovate in what the product does, not in how controls behave.
   _Verify:_ each new screen's design/journal names its domain references; no
   control behaves unlike its lookalike in that domain's mainstream tools.
3. **Few choices per moment** _(Hick's law + progressive disclosure)_ — a
   screen presents one obvious next step; advanced options unfold only when
   summoned. Default over decision: settings the user never met must already
   be right (the Insights pre-built overview is the canon example).
   _Verify:_ count the decisions a new user must make before value — target
   ≤1 per screen.
4. **The primary action is unmissable** _(Fitts's law)_ — biggest, closest,
   highest-contrast thing in reach; destructive actions are farther and
   quieter. Every call to action uses the shared `Button` primitive's
   **brand-orange primary treatment**; neutral, outline, and text treatments
   are reserved for secondary actions and must never make the next step look
   optional. Screens do not invent local CTA colours. Touch targets ≥40px.
   Secondary action buttons use the shared quiet neutral treatment: token
   `bg-raised`, `text-primary`, and no underline, with `bg-default` on hover.
   Their icons inherit the same primary text colour. They must not use brand
   orange, an accent-tinted fill, or arbitrary local colours; orange is
   reserved for the one primary call to action and selected state. A secondary
   action placed beside a primary remains visibly quieter while retaining the
   same control height, protected inset, radius role, focus visibility, and
   minimum touch target.
   _Verify:_ squint test — the blurred screen still shows what to press.
   Button labels and icons must never touch or visually crowd a button border.
   Measure from the outermost text or icon edge to the inside of the border:
   compact buttons keep at least `space-5` (20px) horizontally and `space-2`
   (8px) vertically; standard calls to action keep at least `space-6` (24px)
   horizontally and 10px vertically. These are minimums, not targets that a
   screen may compress. CSS resets, table cells, and layout constraints must
   never reduce the shared `Button` primitive's computed padding or place an
   icon outside it. Longer labels expand the control; they
   never touch its border, clip, or force a sibling action out of shape.
   Flex rows, tables, and action clusters must preserve the button's intrinsic
   width instead of compressing it; surrounding content reflows or scrolls
   before a call-to-action loses its protected inset.
   _Verify:_ every button remains legible with its longest translated label;
   the shared Button test mechanically guards the minimum inset and target.
5. **Empty states are the onboarding** — a new module's first screen is not
   blank; it teaches the ONE next step and often does it for the user
   (auto-created Home page; pre-built dashboard). Every empty list explains
   itself in one sentence + one button.
   _Verify:_ every empty state has action + explanation; none says just
   "No items".
6. **Immediate, honest feedback** _(Doherty threshold, <400ms)_ — every click
   visibly responds instantly: optimistic updates, skeletons over spinners,
   progress with meaning. Silence after a click is a defect.
   _Verify:_ no interaction leaves the screen unchanged >400ms without a
   working indicator.
7. **Undo over confirmation** _(error tolerance; peak-end)_ — routine actions
   execute immediately with a visible undo window; confirmations are reserved
   for the genuinely irreversible/outward (send, issue, publish, delete-forever
   — where alo's propose-then-approve pattern rules). Never punish the 99%
   flow to guard the 1% mistake.
   _Verify:_ count confirm dialogs per module; each must justify itself
   against this law.
8. **Errors speak human and help** — say what happened, why, and the way out,
   in the user's words; surface the server's precise reason verbatim rather
   than a generic veil (the S1.30b lesson, now law). No codes, no blame.
   _Verify:_ trigger each error path; every message names a next step.
9. **Calm surfaces, one voice** _(aesthetic-usability; Miller's law)_ — the
   token system is the single source of color/type/spacing; one accent per
   surface; information chunked in scannable groups; motion subtle and
   purposeful (reduced-motion respected). Beauty here is trust, not
   decoration. Sibling module navigation, project view navigation, and every
   other tab row are one shared visual role and use one state system
   everywhere: inactive tabs are neutral secondary text on a transparent
   surface; only the selected tab receives the soft brand-orange background,
   orange text/icon, semibold weight, and orange bottom indicator. Nested tabs
   do not invent a quieter or stronger active colour; hierarchy comes from
   placement and labels, not a different state treatment.
   Global link color must never leak into inactive navigation items.
   Navigation labels must never crowd their icons, neighbouring tabs, or the
   control boundary: use at least 8px between sibling tabs, at least 8px
   between an icon and its label, at least 16px horizontal padding inside each
   tab, and a 44px minimum hit height. Increase those values for a calmer
   desktop layout; do not reduce them to fit more items—allow horizontal
   scrolling on narrow screens instead.
   _Verify:_ no hardcoded colors/spacings outside tokens; screens pass a
   5-second "what is this page about" glance test. Compare sibling module tabs
   side by side and confirm their active and inactive states are identical.
10. **End on a high** _(peak-end rule)_ — completing something meaningful
    (site published, invoice issued, week approved) earns a small, fast
    moment of acknowledgment with the result's identity (the live URL, the
    number assigned) — never a modal essay.
    _Verify:_ each module's "done" moments show the outcome, not just close.

11. **Smoother than the tool they came from** _(flow: the goal-gradient
    effect, and the reason to switch)_ — familiarity is the floor; the flow
    is where alo wins. Three measurable rules:
    (a) **Step budget:** every core journey takes FEWER clicks/screens than
    the same journey in its domain-reference tool — count them both in the
    design journal, and beat the reference or justify why not.
    (b) **Never ask what alo already knows:** data entered once flows
    everywhere — a customer typed in CRM autofills the invoice, the site
    form, the email; a user should never re-type or copy-paste anything
    between alo modules, ever.
    (c) **No seams:** the flows competitors need exports/integrations for
    are ONE motion here (won deal → invoice; site form → CRM lead; hours →
    invoice line; question → chart on the board). alo owns the whole house —
    walking between rooms must feel like one floor, not doorways with locks.
    _Verify:_ wave reviews map each core journey's step count vs. the
    reference tool, hunt for any re-typed field, and walk every cross-module
    handoff end-to-end.

12. **The one-click law** — if an action can be done safely in one click,
    making it two is a defect; every click must earn its place. No ceremony:
    no intermediate "landing" screens before common actions, no navigating
    away to do something that can happen in place, no dialog asking for
    information a default could supply (laws 3, 7, 11b). The most-used action
    of every screen is exactly one click away from where the user already is.
    When extra clicks exist to feel "safe" or "organized", they are neither —
    they are friction wearing a costume.
    _Verify:_ for each screen, list its three most frequent actions and count
    the clicks from screen-open to done; any count that could be lower with
    a safe design is filed as a defect.

13. **Brilliant with the AI off** _(economics + philosophy)_ — AI calls cost
    money per use; navigation, defaults, and design cost nothing per use.
    Therefore the excellence of every alo flow comes FIRST from deterministic
    UX — galleries, pre-built defaults, direct manipulation, one-click paths,
    zero learning curve — and AI is acceleration and delight on top, never
    the only door to a capability and never a crutch propping up a confusing
    screen. If a flow is only usable "because you can ask the AI", the flow
    is wrong. Every AI feature has a first-class manual sibling (template
    gallery beside generate-my-site; tile gallery beside ask-to-chart), and
    the AI-off configuration is a complete, pleasant product — not a degraded
    mode.
    _Verify:_ the wave review walks every module with NO AI key configured;
    every core task must be achievable and feel finished. Count model calls
    in routine flows — a routine action that silently spends an AI call is a
    defect.

## The visual scale — how every size gets chosen

Nothing on an alo screen has an arbitrary dimension. Icon sizes, card
paddings, corner radii, shadows, gaps — every value is picked **by role from
the token scale** (`web/src/ds/tokens.css`), which is the single place a
value may be born. The principles that govern the scale:

- **One rhythm.** All spacing and sizing sits on the token grid (the 4px
  family). Two adjacent elements never differ by amounts the grid can't
  express — that sub-pixel-of-chaos feeling in lesser apps comes exactly
  from breaking this.
- **Size encodes importance.** Bigger means more important, always —
  a screen's visual hierarchy must survive the squint test (law 4). Icons
  come in exactly three roles: inline-with-text, in-controls, and
  rail/hero — chosen by role, never by taste per screen.
- **Radius encodes surface class.** Small radius for controls (inputs,
  buttons), medium for cards and panels, large only for floating overlays —
  one class, one radius, never mixed on the same surface kind. Corner
  radius is how users subconsciously read "what kind of thing is this".
- **Elevation is a ladder.** Shadows come only from the token set, one step
  between adjacent layers; a surface never jumps two levels. Depth is
  information, not decoration.
- **Room to breathe.** Cards and panels keep generous minimum padding from
  the space scale; reading text keeps a book-like measure; density is a
  deliberate mode (as in Drive's density control), never an accident.
- **Controls protect their content.** Inputs, selects, combobox triggers, and
  menu options keep at least `space-4` (16px) between visible text or icons and
  every bordered edge. Large selectors use `space-5` (20px). Leading icons,
  labels, values, and trailing affordances use an explicit gap from the token
  scale; chevrons keep the same trailing inset as the value's leading inset.
  Trigger text and its menu options align to the same left edge. Content never
  touches a border, focus ring, or adjacent affordance, and a layout may not
  compress this protected inset. Long and translated values truncate or wrap
  before the inset is reduced.
  Selection cards implemented with native buttons must protect the same inset
  from the global zero-padding button reset: use the token-equivalent important
  padding utility where required and verify the computed padding, not merely
  the class name. Checkboxes, radio marks, previews, and labels must sit wholly
  inside the card with at least `space-5` (20px) to every bordered edge.
  _Verify:_ inspect the longest translated value in every control and its open
  menu; the nearest glyph edge remains at least 16px from the control border.
- **Touch respects fingers.** Interactive targets stay ≥40px regardless of
  how small the glyph inside them is.
- **Navigation has deliberate separation.** Sibling navigation actions keep at
  least `space-2` (8px) between their interactive bounds, use a minimum 44px
  target height, and keep `space-2` between icon and label. The active item uses
  the soft brand surface, Terracotta text, and a brand indicator; inactive items
  remain neutral and gain only a quiet neutral hover surface. Navigation labels
  are never underlined. On narrow screens, the row scrolls horizontally instead
  of shrinking labels or removing their protected padding.
- **New values are legislation, not improvisation.** A dimension that the
  scale lacks is added to tokens.css in its own reasoned commit — never
  inlined into a component.

_Verify (mechanical):_ grep new UI for hardcoded px/color values outside the
ds — any literal that isn't a token reference is a defect. _Verify (human):_
the wave review compares sibling screens side by side — same roles must look
identical in size, radius, and rhythm across modules.

## Standing constraints

- **Uploaded files never go into `localStorage`.** Images, logos, documents,
  and other binary content use the workspace file service or IndexedDB for a
  local-only draft. `localStorage` is reserved for small preferences and IDs;
  never persist base64/data URLs there. Every upload save is guarded so quota,
  permission, or serialization failures produce an inline recovery message
  instead of crashing or unmounting the screen.
  _Verify:_ upload representative and near-limit files, reload the page, and
  simulate a storage rejection; the file remains available when saved and the
  surrounding workspace remains usable when saving fails.
- **Full-page workspaces consume the available viewport through their final
  section.** A module with a fixed application shell must have exactly one
  deliberate vertical scroll owner beneath its header. That scroll region is
  `min-h-0 flex-1 overflow-y-auto`; its primary document or workspace is at
  least `min-h-full` and grows with content. Never let a fixed-height ancestor
  clip the lower half of a page while unused application background appears
  beneath it, and never add arbitrary footer padding or viewport calculations
  to disguise a broken height chain.
  _Verify:_ test short, viewport-height, and long records at common desktop and
  mobile sizes. The workspace reaches the bottom edge when content is short,
  the final section remains reachable when content is long, and there is no
  detached blank footer or nested page scrollbar.
  A viewport-filling document must not reserve bottom page padding or opt out
  of flex growth with `shrink-0`; its final surface continues to the workspace
  edge. Bottom breathing room belongs inside the document's last section, not
  as an exposed band of application canvas beneath the document.
- **The authenticated application shell owns the dynamic viewport height.**
  Its outer frame uses `100dvh` (with a `100vh` fallback) rather than relying
  only on percentage heights inherited through route guards and wrappers.
  Internal modules consume that frame with flex/grid sizing; they do not set
  their own viewport height. This keeps the navigation rail, workspace ground,
  and final content edge aligned even when browser chrome, zoom, extensions,
  or mobile viewport controls change the usable height.
  _Verify:_ the shell rail and main background reach the same bottom edge at
  every tested viewport; resizing DevTools or browser chrome never exposes a
  blank band beneath the application.
- **Every application colour comes from the approved alo palette.** Product
  chrome uses only the semantic design tokens for Terracotta accents,
  ivory/cream surfaces, warm-stone neutrals, warm-charcoal structure, and ink
  text. Components must not introduce generic Tailwind palette colours such as
  `blue-*`, `slate-*`, `gray-*`, arbitrary hex/RGB values, or browser/system
  colours. Success, warning, and danger tokens are reserved for states that
  genuinely communicate those meanings; charts and user-authored content are
  the only intentional broader-colour exceptions. Change the shared semantic
  token when the palette evolves rather than patching individual screens.
  User-authored document branding is the narrow exception: validated colours
  chosen in a colour picker may flow through scoped CSS custom properties to
  the document preview and export only. They must never recolour alo's own
  navigation, controls, focus, validation, or status states.
  _Verify:_ audit all resting, hover, focus, selected, disabled, error, overlay,
  and autofill states; every computed application colour resolves to an alo
  semantic token or a documented status/data exception.
- **Application components use Tailwind utilities only.** Do not create CSS
  modules, component stylesheets, inline style objects, or local `<style>`
  blocks. When touched code still depends on a component-specific `.css` or
  `.module.css` file, migrate those rules to token-backed Tailwind utilities
  and delete the obsolete stylesheet. The only CSS files allowed are the
  design-system foundations Tailwind consumes: the Tailwind entrypoint,
  shared tokens, themes, resets, fonts, and genuinely global browser rules.
  Those files define the language; components speak it through utilities.
  _Verify:_ new or changed component files import no component stylesheet and
  introduce no `style={{...}}`; searches for new `.module.css` files are empty.
- **Browser defaults never become product design.** No native browser or
  operating-system blue, purple, bevel, focus glow, selected-row colour,
  autofill colour, or default spacing may appear in alo application chrome.
  Every visible state—resting, hover, focus, open, selected, checked,
  disabled, invalid, and autofilled—uses design-system tokens and brand
  colours. When a native popup cannot be themed consistently across supported
  browsers, use the accessible design-system primitive for that interaction
  instead of shipping an unbranded platform menu.
  _Verify:_ exercise every control state in Chromium, Firefox, and WebKit;
  no state falls back to an unthemed browser colour.
  Browser-managed details are included in this rule: text selection, search
  decorations, autofill, file-picker triggers, checkbox/radio accents, date
  affordances, validation, and disabled states all need an Alo treatment.
  Use `ChoicePicker` for visible dropdown menus, `DatePicker` for dates, the
  shared checkbox/toggle primitives for choices, and an Alo button backed by a
  visually hidden file input for uploads. A raw native control is only an
  accessibility fallback; it is never the finished visible interaction. These
  treatments live in Tailwind-based components, not feature or fallback CSS.
- **Development must fail closed when its API is unavailable.** A frontend
  configured for a loopback API may not start until that API reports ready.
  Never leave a convincing login screen in front of a dead proxy: it turns an
  infrastructure failure into a false password failure and sends debugging in
  the wrong direction. Use the repository's full-stack launcher, which checks
  the database revision, API readiness, OAuth issuer, and frontend together.
  If the API later becomes unavailable, authentication must describe a server
  connection problem rather than blaming credentials.
  _Verify:_ point `VITE_DEV_API` at a stopped loopback port and confirm the dev
  command exits with the recovery command; a real HTTP 401 remains the only
  outcome presented as incorrect credentials.
- **Focus indicators never use browser blue or cool blue-gray.** Text fields,
  textareas, selects, buttons, links, listboxes, and composite controls use a
  Terracotta outline or ring and a Terracotta focused border. Strong neutral
  borders use the warm-stone token; they are not a substitute focus colour.
  Mouse, keyboard, autofill, validation, and programmatic focus must not reveal
  the browser's default blue glow, outline, or selected-control treatment.
  Shared form primitives and the global fallback own this rule so feature code
  does not invent a local focus colour.
  _Verify:_ tab through and click every interactive control in Chromium,
  Firefox, and WebKit; inspect focused borders and outlines and confirm every
  visible focus treatment resolves to `--accent`, never a browser/system blue.
- **Form controls show one border, never a double ring.** Inputs, textareas,
  selects, combobox triggers, date fields, and editable cells use a single
  Terracotta focus perimeter aligned to the control edge. Do not combine a
  focused border with an outward ring or offset outline. The shared primitive
  and global fallback own this treatment so it remains identical throughout
  the application and does not shift layout or get clipped by a dialog.
  _Verify:_ focus every field with mouse and keyboard at normal and high zoom;
  exactly one continuous Terracotta edge is visible around the control.
- **Every dropdown option has an unmistakable branded hover state.** Menu,
  listbox, combobox, command-palette, suggestion, and picker rows use the soft
  Terracotta surface with Terracotta text on pointer hover. The hover applies
  across the row's full protected target, not only its label or icon, and never
  depends on a browser's native option highlight. Selected and keyboard-active
  rows use the same brand family and remain distinguishable through weight,
  checkmark, or `aria-selected`; disabled rows do not react to hover. Shared
  dropdown primitives own this behavior so product screens inherit it rather
  than restating it.
  _Verify:_ open every dropdown and move the pointer across every enabled row;
  the complete row changes to `bg-accent-soft text-accent`, with no native blue
  or visually inert option.
- **The interface never uses underlines as decoration or interaction feedback.**
  Links, buttons, tabs, navigation, table actions, and their hover, focus,
  active, and visited states use colour, weight, surface, and a visible focus
  ring instead. Underlining remains available only inside user-authored rich
  text where it is content, not application chrome.
  _Verify:_ interactive interface text has `text-decoration: none` in every
  state; keyboard focus remains obvious without an underline.
- **Preserve every working behavior outside the requested change.** A visual
  refinement must not change data flow, persistence, navigation, validation,
  permissions, or adjacent interactions unless the user explicitly asks for
  that change. Identify those invariants before editing and exercise the
  touched core flow afterward (create, save, reopen, list, and navigate as
  applicable). If an adjacent contract truly must change, make the impact
  explicit instead of silently broadening scope.
- All copy through the i18n catalog, in the user's language, jargon-free
  (write "web address", not "subdomain", wherever a normal person will read).
- Keyboard reachability and visible focus on all interactive elements;
  contrast per WCAG AA. Accessibility is part of law 1, not a tier-3 item.
- **`docs/design/` holds laws and standing design decisions ONLY.** Audit
  findings, defect lists, and review outcomes are work items — they live in
  `docs/reviews/` (dated files) or the track queues, never beside the rules.
  A law is permanent; a defect is temporary; mixing them erodes both.
- These laws extend CLAUDE.md quality gates: a UI slice is not done when it
  compiles — it is done when a stranger can use it. Wave reviews test the
  laws explicitly and file violations as queue items.
