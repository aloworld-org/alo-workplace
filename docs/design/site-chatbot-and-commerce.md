# The site chatbot, and selling through it

**Status:** direction, not a decision. Nothing here is queued. The load-bearing
questions are marked, and each needs settling in an ADR before code — putting
any of this straight into `docs/autonomy/sites/QUEUE.md` would start a loop
building a half-formed idea.

## Why this belongs to alo and not to Wix

A website chatbot is a commodity. Every builder has one, and it answers
questions from the page you were already looking at.

What none of them can do is *act*, because none of them owns the calendar, the
CRM, the catalog or the books. alo does. So the interesting product is not a
bot that answers — it is a bot that finishes the job:

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

## The open questions — each an ADR, not a queue item

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

Stated as intent, with the honest gap.

Chat is genuinely competitive today: threads, reactions, mentions, search,
Drive attachments as pointers, formatting with code and maths shared with Docs
and Mail, and agents as participants with identity separated from authority —
which Teams does not have and, given its architecture, cannot easily add.

Meet is not there yet, and `meet-roadmap.md` says exactly what is missing and
in what order. The differentiators that matter are recording with consent,
transcripts, and AI minutes posted back into the room the meeting belongs to —
"included, not a 30 euro per user add-on".

The claim to make is not "better than Teams at everything". It is: **the
meeting, the room it belongs to, the file discussed in it and the invoice it
produced are one product** — and neither Microsoft nor Odoo can say that.
