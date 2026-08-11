# The site chatbot, and selling through it

**Status:** the load-bearing questions are now settled — **ADR 0040** (what the
bot may read, what it may do, who pays), **ADR 0041** (the shop as a surface
over one catalog) and **ADR 0042** (direct-manipulation editing). This file
stays as the argument behind them; the ADRs are what binds.

What remains before code is sequencing, not thinking: the items go into
`docs/autonomy/sites/QUEUE.md` in the order ADR 0041 sets out, and no sites
item may edit a file the Billing or CRM track owns.

## Why this belongs to alo and not to Wix

**Their weakness:** a website chatbot is a commodity, and every builder's one
answers questions from the page you were already looking at. It cannot book
anything, quote anything or remember you, because Wix and Squarespace do not
own a calendar, a CRM, a catalog or a set of books. Their AI builds a site from
a *prompt* for the same reason — a prompt is all they have.

**Our move:** a bot that finishes the job rather than describing it.

> A visitor asks whether you do this kind of work. The bot answers from the
> site, offers twenty minutes on Thursday, books it against real availability,
> and the deal is in CRM before the salesperson has read the message.

That sentence is impossible for Wix, Squarespace and Webflow, and it is the
demo that explains why alo being one product matters.

## Three levels, in the order they should be built

**1. It answers from what is already public.** Grounded only in the published
site. Leaks nothing by construction, and still useful — most visitor questions
are answered somewhere on the site, just not where anyone looks. Every answer
cites the page it came from: verifiable, and it drives traffic inward.

**2. It answers from what the tenant adds.** A price list, a services
document, a project's public status. This is where people will want it and
where it becomes dangerous — see the open questions.

**3. It acts.** Books a meeting in Agenda. Creates the contact and the deal in
CRM. Raises a quote in Billing. Sells a ticket.

## Selling through it — events first

An event is already three things alo has: a calendar entry, a catalog product,
and an invoice. So ticketing is the narrowest possible commerce wave — one
product type, one fulfilment path — on top of a catalog, invoicing, calendar
and CRM that already exist.

The loop: ask, answer, "two tickets, 90 euro?", hosted payment page, paid,
ticket by email **and in the buyer's calendar**, contact in CRM, money in
Billing.

Payments stay integrated, never built (product doctrine, non-goals). Card
details never touch alo: the order is created here and the payment happens on
the provider's hosted page. **Mollie or Adyen ahead of Stripe** — both Dutch,
which matters more than usual in a product sold on sovereignty.

Three parts that must be right, in the order they bite:

- **Capacity.** Overselling is the classic ticketing failure. Check-then-create
  is a race two buyers will win simultaneously. A hold with an expiry is taken
  *before* payment and released if they do not finish. This one has to be right
  on day one, not hardened later.
- **VAT.** Event tickets follow place-of-supply rules that differ from goods —
  for a physical event it is usually where the event happens, not where the
  buyer lives. Wrong here means wrong numbers in Finance. Worth a tax
  professional's ten minutes before any code.
- **Never a price the model invented.** Read the catalog or say you do not
  know. A hallucinated price on a public site is one a customer will hold you
  to.

## The questions, and where they were answered

1. **What may an anonymous visitor's bot read?** Whatever it can read, the
   internet can read. If a tenant can point it at a Drive folder, the interface
   must say *"anyone on the internet will be able to read this"* — not "select
   sources". This is the entire design.
2. **Who pays for the model?** An anonymous endpoint calling an LLM is a bill
   any stranger can run up. Per-site rate limits and a monthly ceiling are day
   one, not hardening.
3. **What may it do without a human?** Booking a meeting is reversible. Taking
   money is not. The propose-then-approve model that governs agents inside the
   workspace has no obvious equivalent for a visitor who is not a member of it.
4. **It crosses tracks.** The queue says the sites track must not touch billing
   or CRM. The best version of this feature is precisely that crossing, so it
   cannot be one queue item — it needs sequencing across both.

## Editing: direct manipulation, not a canvas

Related, and settled enough to state: people asking for "a canvas like Figma"
want **direct manipulation** — click the thing, change the thing, see it — not
absolute positioning.

A true canvas would cost the three things that make alo Sites different:
reviewable AI edits (a diff of a typed section is readable; a diff of moved
pixels is not), a static renderer with a finite golden-tested surface, and
semantic HTML that gives SEO and accessibility for free. ADR 0036 rejected the
canvas for exactly these reasons and that rejection stands.

What can be built on the section model without touching any of it: editing text
in place on the page rather than in a sidebar form, dragging sections to
reorder with the page reflowing live, resizing within a section's own
constraints, and a palette to drag new sections from. That is the Squarespace
and Webflow *editor* experience — and combined with "ask for a change and
review the diff", it is something neither of them has.

## alo Chat and alo Meet against Microsoft

**Their weakness:** Teams is three products in a trench coat. A file shared in
a chat lands in a SharePoint folder nobody finds again; the meeting about that
file is a fourth place; the recording is a fifth. Every seam is somewhere work
goes missing, and the seams exist because those parts were separate products
before they were one tab.

**Our move:** remove the seams rather than decorate them. A file in a room is a
Drive pointer, not a copy. A meeting belongs to the room it was started from.
The transcript posts back into that room. Nothing is anywhere else.

The second move is the one Teams cannot answer at all: agents as participants,
with identity separated from authority — an agent posts as itself and acts on
behalf of the person who asked, and only that person may approve what it
proposes.

Chat is genuinely competitive today: threads, reactions, mentions, search,
Drive attachments as pointers, formatting with code and maths shared with Docs
and Mail.

Meet is not there yet, and `meet-roadmap.md` says exactly what is missing and
in what order. The differentiators that matter are recording with consent,
transcripts, and AI minutes posted back into the room the meeting belongs to —
"included, not a 30 euro per user add-on".

The claim to make is not "better than Teams at everything". It is: **the
meeting, the room it belongs to, the file discussed in it and the invoice it
produced are one product** — and neither Microsoft nor Odoo can say that.

## alo Commerce — the shop as a surface, not a system

**Status:** direction. Not queued, and further out than everything above.

Odoo is the right comparison and the honest one. Their e-commerce is strong
for a reason that has nothing to do with their storefront: the shop sells the
*same* catalog that Inventory counts, Sales quotes and Accounting posts. Add a
product once and it exists everywhere; an order reserves stock and lands in the
books with no integration to maintain.

**Shopify's weakness** is the mirror image: a beautiful shop with no books
behind it. Every merchant of any size ends up paying somebody — a bookkeeper, an
integration, a monthly SaaS bill — to reconcile Shopify against their
accounting, their stock and their CRM. That reconciliation is the tax on a good
storefront, and it never ends.

**Our move:** there is nothing to reconcile, because there is one catalog and
one set of books.

alo is already in Odoo's position rather than Shopify's: the catalog landed in
Billing, Inventory tracks suppliers, locations and moves, Finance posts, CRM
holds the customer. **The shop is a surface over things that already exist —
not a system to integrate.**

### So where is "better than Odoo"?

State it as a weakness and a move, never as a comparison of size.

**Their weakness:** every setting is a form somebody has to fill in. Product
templates versus variants, tax positions, fiscal mappings, delivery carriers,
pricelists. The software already knows most of those answers — a bookshop in
Belgium selling paperbacks has one plausible VAT treatment, not a blank field —
and it demands them anyway. That is why an entire consulting industry exists to
type them in, and why a shop is a project with a partner rather than a decision
somebody makes.

**Our move:** propose the whole configuration from a sentence about the
business, and let somebody approve it. Not fewer settings — the *same* settings,
already answered, shown for confirmation. Every wrong guess is one correction
instead of one lesson in accounting.

That is a harder engineering problem than a settings screen, which is the point.
Nobody has done it because it requires the software to understand the business,
and alo is the only one holding the invoices, the stock and the customers needed
to try.

The wedge is that alo can *propose* the configuration instead of demanding it:

> "I run workshops in Antwerp and sell two books."
>
> A draft catalog with the workshops as dated products and the books as stock
> items. VAT proposed for each — Belgian rate on the workshops, the reduced
> rate on books, flagged as needing confirmation. A shop page built from the
> sections that already exist. Shipping for the books, nothing for the
> workshops. **Reviewed and approved, not configured.**

That is the same propose-then-approve pattern used everywhere else in alo, and
it is worth more here than anywhere: this is the screen where Odoo loses
customers who cannot afford a consultant.

### Order of building, narrowest first

1. **Tickets and dated products** — one product type, no shipping, no
   variants, no stock. The calendar is the inventory. Covered above.
2. **Simple stock items** — one price, one tax, a shipping rate. Enough for
   the books, the merchandise, the parts.
3. **Variants** — size, colour, and the combinatorial explosion behind them.
   This is where every existing commerce system becomes complicated, which
   makes it the most interesting problem here rather than the one to avoid:
   the goal is variants somebody can set up in a minute without learning what
   a product template is.
4. **The rest** — subscriptions, bundles, B2B pricelists, multi-warehouse.
   Sequenced by what customers actually ask for, not by what a competitor's
   feature list contains.

### What must be right, and what must not be built

- **Stock is one number.** The shop must not hold its own count. The whole
  argument for building rather than integrating collapses the moment two
  systems disagree about how many are left.
- **Payments are never ours.** Hosted page, Mollie or Adyen, card details
  never touching alo (product doctrine, non-goals).
- **Tax is a rule, not a field.** Cross-border EU selling means OSS and IOSS
  thresholds, and a shop that quietly gets this wrong hands its owner a
  compliance problem years later. This deserves a professional's review before
  code, exactly like the event VAT question above.
- **Never a price or a stock figure the model invented.** The catalog answers,
  or nobody answers.

### The sentence the whole thing is for

Odoo can say the shop and the books are one system. Shopify can say the shop is
lovely. **Neither can say the customer who bought, the invoice they paid, the
email thread about their delivery, the meeting where you agreed the discount and
the project you delivered are the same product.** That is the only claim worth
making, and it is only true if the shop stays a surface.
