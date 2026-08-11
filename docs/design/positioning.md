# Positioning — who we are against, and what to say

Written from a working session's worth of argument so it is not re-derived
every time somebody writes a landing page or plans a demo. **Direction, not a
decision**; nothing here is queued.

## The rule for talking about a competitor

State a **weakness** and a **move**. Never a size.

> Not: *"you cannot beat Nokia, they have twenty years and a large team."*
>
> But: *"their buttons are ugly and fixed. Build a screen that becomes any
> app, because customers want the app more than the button."*

Incumbency is not a moat — it is a list of decisions made years ago that
nobody there can revisit. Nokia and BlackBerry had far more phone experience
than Apple in 2007, and that experience is what made their answer confidently
wrong. If a feature looks huge, sequence it; never decline it. Reasons to defer
are sequencing reasons, never capacity reasons.

## Who we are actually against

**Odoo is the business half. Microsoft and Google are the workplace half. alo
is both, and that is the entire product.**

| Against | Their weakness | Our move |
|---|---|---|
| **Odoo** | Every setting is a form somebody must fill in — product templates versus variants, fiscal positions, carriers, pricelists — when the software already knows most of the answers. A consulting industry exists to type them in | Propose the whole configuration from a sentence about the business and let somebody approve it. The same settings, already answered |
| **Odoo** | No mail server, no CalDAV, no file storage worth the name, no document editors, rudimentary chat, no real meetings. Their customers still pay Microsoft for everything they use all day | One product for the work *and* the business. The invoice knows about the email thread and the project |
| **Microsoft Teams** | Three products in a trench coat. A file shared in a chat lands in a SharePoint folder nobody finds again; the meeting about it is a fourth place; the recording a fifth. Every seam is where work goes missing | Remove the seams rather than decorate them. A file is a Drive pointer, a meeting belongs to its room, the transcript posts back into it |
| **Microsoft Teams** | No concept of an agent that acts on somebody's behalf while posting as itself | Agents as participants — identity separated from authority, and only the asker approves |
| **Google Workspace** | Brilliant productivity, zero business. You still buy an ERP and reconcile it for ever | The books, the deals and the stock are in the same product as the mail |
| **Shopify** | A lovely shop with no books behind it. Every merchant of any size pays somebody, for ever, to reconcile it against accounting, stock and CRM | Nothing to reconcile: one catalog, one set of books, the shop is a surface |
| **Wix / Squarespace** | Their AI builds a site from a *prompt*, because a prompt is all they have. They do not know what you sell or who bought it | A site drafted from real invoices, real deals and real delivered projects |

**One correction worth remembering: Odoo is Belgian.** European sovereignty is
decisive against Microsoft and Google. Against Odoo it is neutral, and claiming
it as an advantage there makes us look like we have not read their website.

## The demo — four moments, in the order they land

A promotional video should not tour features. It should show four things that
are impossible for anybody else, and let the audience work out why.

**1. "Build my site" — and it already knows the business.** Everybody demos
*prompt → website*; that is 2023 and nobody gasps. alo reads real invoices,
real deals, real completed projects, so the first draft names the services you
actually bill for with case studies you actually delivered. *"I didn't describe
my company. It already knew."*

**2. The bot that books the meeting.** A visitor asks whether you do this kind
of work. The bot answers from the site, offers twenty minutes against real
availability, books it, and the deal is in CRM before the salesperson has read
the message. Not a chatbot — a salesperson that never sleeps. **This is the
strongest one**; if only one is built for the video, build this.

**3. Editing by saying it — with a diff.** *"Make the hero shorter and mention
we're in Antwerp."* Before and after, side by side, **Approve**. The wow is the
approve button, not the AI: Wix physically cannot show a reviewable diff of an
AI edit, because their pages are not typed sections.

**4. The site keeps itself current.** A project finishes → *"add this as a case
study?"* A price changes in Billing → *"update the pricing page?"* Five
visitors ask the same unanswered question → *"nobody can find your delivery
times — add a page?"*

**The line that ties it together:** *your website is not a brochure you update.
It is the front door of the business you already run.*

## What we do not claim

- **Not "better than Teams at everything".** The claim is that the meeting, the
  room it belongs to, the file discussed in it and the invoice it produced are
  one product — which neither Microsoft nor Odoo can say.
- **Not feature parity with Odoo's configuration surface.** Deliberately: that
  surface is the problem, and matching it would mean inheriting it.
- **Not sovereignty as an edge over other European companies.** It is an edge
  over the American ones, which is where the customers are leaving from.

## Where the detail lives

- `meet-roadmap.md` — Meet's gap to Zoom and Teams, in the order it closes,
  including the features only alo can build
- `site-chatbot-and-commerce.md` — the site chatbot, ticketing and commerce,
  and why the editor is direct manipulation rather than a canvas
- `chat.md`, `chat-agents.md` — what shipped, and the rules behind it
- ADR 0039 — remote control, and the refusals that make it defensible
