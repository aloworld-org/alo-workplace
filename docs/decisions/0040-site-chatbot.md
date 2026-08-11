# ADR 0040 — The site chatbot: what it may read, what it may do, who pays

**Status:** accepted
**Date:** 2026-08-11
**Context:** `docs/design/site-chatbot-and-commerce.md`, ADR 0036 (typed
sections), ADR 0034 (one agent, tool sets per module)

## The decision in one line

A visitor's bot answers from what is **published**, acts only where the act is
**reversible**, and takes money only by handing the visitor to a page the
tenant's payment provider owns — under a spend ceiling the tenant sets.

## Why this needs deciding before any code

Every website builder ships a chatbot, and every one of them answers from the
page you were already reading. The reason alo's can be different is that it
sits on a calendar, a CRM, a catalog and a set of books — which is also
precisely what makes it dangerous. The same seam that lets it book a meeting
lets it read a price list nobody meant to publish.

The three questions below are not implementation details. Answer them wrong and
the feature is a liability; answer them late and the answer is whatever the
first handler happened to do.

## 1. What may an anonymous visitor's bot read?

**Decision: only what is already public, plus sources the tenant has
explicitly published to it — and the interface says so in those words.**

The grounding set is:

- the **published** version of the site (never a draft, never a scheduled
  publish that has not run);
- documents the tenant has added to a named **Public knowledge** collection,
  which is a deliberate act with its own screen;
- structured facts the tenant switches on one by one: opening hours, published
  prices from the catalog, published availability.

Never: a Drive folder chosen from a picker, a Space, mail, a CRM record, an
invoice, or anything belonging to a person rather than to the site.

The rule that makes this safe is a sentence, not a permission model:

> **Whatever the bot can read, the internet can read.** The screen that adds a
> source says exactly that, above the button, every time.

*Rejected: "select sources" with a permissions matrix.* A matrix invites the
belief that some sources are readable-but-not-quotable, which is not a thing a
language model can promise. One boundary, stated in one sentence, is the whole
design — and it is the sentence a customer will be held to after a leak.

**Citations are mandatory.** Every answer names the page it came from. That is
verifiable by the visitor, it drives traffic inward, and an answer that cannot
cite is an answer the bot does not give.

## 2. What may it do without a human?

**Decision: reversible acts alone, and never a number it invented.**

| The bot may, by itself | The bot may not, ever |
|---|---|
| Book a meeting in published availability | Take a payment itself |
| Create a contact and a lead in CRM | Issue or alter an invoice |
| Hold a ticket for a stated few minutes | Confirm an order as paid |
| Offer to email a published document | Promise a discount, a date or a price |

The line is reversibility. A meeting booked wrongly is cancelled with an
apology; money taken wrongly is a refund, a chargeback and a complaint.

**Money always leaves the conversation.** The bot creates an order and hands
over a link to the provider's hosted page (ADR: payments are integrated, never
built). Card details never touch alo, and the visitor's last action before
paying is on a page whose address they can see.

**Never a price the model invented.** Prices, stock and availability are read
from the catalog through a tool call, or the bot says it does not know and
offers a human. A hallucinated price on a public site is one a customer will
hold the tenant to, and in several EU jurisdictions may bind them.

*Rejected: propose-then-approve for the visitor.* Inside the workspace an agent
proposes and the asker approves. That pattern has no equivalent here: the
visitor is not a member of the workspace, and asking the tenant to approve
mid-conversation means the bot stops being useful at 6pm. So the boundary is
drawn at reversibility instead of at approval.

## 3. Who pays for the model?

**Decision: a per-site monthly ceiling the tenant sets, on by default, plus a
per-visitor rate limit — and a graceful refusal when either is hit.**

An anonymous endpoint that calls an LLM is a bill any stranger on the internet
can run up, and the tenant discovers it at the end of the month. So:

- every site has a **monthly ceiling**, defaulted rather than left blank, shown
  in the same screen that switches the bot on;
- a per-visitor and per-IP **rate limit** below that;
- at the ceiling the bot does not degrade quietly — it says it is unavailable
  and offers the contact form, and the tenant is told;
- the ceiling is spend, not tokens. Tokens are our unit, not a customer's.

*Rejected: unmetered, billed in arrears.* That is a support incident with a
customer who is already angry, and it makes the feature's cost unpredictable
for exactly the small businesses this product is for.

## 4. It crosses two tracks, and that is deliberate

The sites track must not edit Billing or CRM. The best version of this feature
reads availability from Agenda, writes a lead to CRM and an order to Billing —
which is the crossing, not an accident of design.

**The bot lives in the sites track and calls the other modules only through
their existing public seams.** Where a seam does not exist it is added by the
module that owns it, as its own queue item, before the bot's item is taken.
No sites item may edit a file under a Billing or CRM path.

## Consequences

- The grounding set is small enough to be auditable, and its rule fits in a
  sentence a customer can be shown after an incident.
- The bot is useful out of hours without anybody approving anything, because
  everything it does alone can be undone.
- The feature has a known worst-case cost per site per month.
- "The bot that books the meeting" — the strongest demo in
  `positioning.md` — is buildable exactly as written: it answers from the
  site, offers real availability, books, and the deal is in CRM before the
  salesperson has read the message.
- Three things stay impossible for Wix, Squarespace and Webflow, and they are
  impossible for the same reason: those products do not own a calendar, a CRM
  or a catalog to read.
