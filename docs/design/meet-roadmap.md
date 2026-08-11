# alo Meet — the gap to Meet, Zoom and Teams

What exists, what is missing, and the order to close it. Written after the
first slice shipped (`docs/design/meet.md`), because "make it as good as Google
Meet" is not a task until it is a list.

**How this list is ordered, and why that matters more than its length.** Each
stage is usable on its own, and the earliest ones remove the reasons somebody
would refuse to use alo Meet at all. That ordering is the whole value of the
document: a demo that fails in a real customer meeting fails on a Stage 1
defect, never on a missing Stage 4 feature.

Nothing here is a judgement about what can be reached. State a competitor as a
weakness and a move, never as a size: *Teams keeps the meeting, the file and
the conversation in three places, so we put them in one* — not *Teams is large*.
The reason to close Stage 1 before Stage 3 is that a broken join ruins the
meeting where you would have shown them the minutes, not that the gap is wide.

## What alo Meet has today

Meeting records with tenancy, admission decided before a token exists, LiveKit
tokens the engine verifies, join from a chat room, from a calendar invitation,
or from the Meet page, attendance recorded, and leave. Camera off and
microphone on by default.

## Stage 1 — the reasons people would refuse to use it

Nothing here is a feature; each is a defect a first meeting would expose.

| Gap | Why it blocks adoption |
|---|---|
| **The room does not look like ours** | LiveKit's stock conference. Known layout blocker: its control bar sizes to the viewport, so a header above it pushes the bar below the fold. Inspect the rendered DOM for real class names and `--lk-*` variables before writing more CSS |
| **No device pre-flight** | Every competitor asks "which camera, which microphone, do you look right" *before* joining. Without it people join broken and leave |
| **No join-time mute state shown to others** | You cannot tell who is muted before they speak |
| **No participant list** | Who is here, and who is talking |
| **No active-speaker view** | A grid of eight faces with no indication of who is speaking is unusable past three people |
| **Nothing happens on connection loss** | No reconnection notice, no rejoin. Networks drop; a call that dies silently is a call people distrust |

## Stage 2 — what a working meeting needs

| Gap | Notes |
|---|---|
| **Do not show somebody their own screen share** | *First item.* We render the sharer's own screen back to them, which produces the infinite hall-of-mirrors when they share the window holding the call. Meet substitutes "You are presenting". Filter `Track.Source.ScreenShare` where `participant.isLocal` and render a card instead — it removes the recursion for the only person who sees it, and saves decoding your own stream |
| **Leave room for the browser's sharing bar** | *Second item.* Chrome's "…is sharing a window — Stop sharing" strip is browser-owned, unmovable, and currently covers our control bar. Every browser product has it; they reserve space and we do not. Pad the controls while `isScreenShareEnabled` |
| **Screen sharing that is ours, around the parts that cannot be** | **The picker itself can never be ours.** `getDisplayMedia()` always shows the browser's own chooser and a page can neither style nor replace it — if a site could draw that dialog it could show you a slide deck while capturing your bank. Meet, Zoom and Teams all show the same unstyled dialog for the same reason. What *is* ours: the trigger, a pre-share warning that everyone will see this, the persistent "you are sharing" indicator, our own stop control, and the presenter layout others see. **In the Tauri desktop app a native picker in our own colours IS possible** — the same native capability ADR 0039 needs, so the two share a foundation |
| **Raise hand, reactions** | Data channel; cheap and disproportionately missed |
| **In-meeting chat that is alo chat** | Their chat is a separate island. Ours should post into the room the meeting belongs to, so the conversation survives the call |
| **Lobby / knock** | `[L]` in features. Anyone with a link joining a board meeting unannounced is a security incident |
| **Background blur** | Every competitor has it; people work from kitchens |
| **Speaker and layout choice** | Grid, speaker, sidebar |
| **A meeting on every invitation, by default** | Creating an event adds an alo Meet link without being asked — that is what Google and Microsoft do, and an invitation without a way to join is the commonest complaint about every other calendar. **With the escape hatch**: a field for a Zoom, Teams or Whereby URL instead, because a customer who insists on their tool will otherwise put it in the description where nothing can read it. Storing it as a field rather than prose is what lets the agenda say "join" at all |
| **Guests without an alo account** | The largest design question in Meet, and it needs its own ADR before code. An external participant is invited to *one* meeting and must join without signing up — but the current admission model is "you are a member of this workspace", and a meeting token is minted only after the store has said so. Guests need a second path with its own rules: a per-invitee link that identifies the guest rather than the meeting, revocable, expiring with the event, never a bare room name. And a lobby, so a link that leaks does not mean a stranger in a board meeting — which is why lobby is listed above and not below this |

## Stage 3 — the differentiators, and the reason to choose alo

| Gap | Notes |
|---|---|
| **Recording to Drive with consent indicators** | LiveKit Egress. Consent is not a checkbox: it is a visible, persistent state for everyone in the call |
| **Transcript** | Speech-to-text, EU-hosted per the doctrine. Not OpenAI's US endpoint |
| **★ AI minutes** | Summary, decisions and action items posted to the meeting's chat thread. The features list calls this "included, not a €30/user add-on" — it is the commercial argument for the whole module |
| **Live captions** | Accessibility, and a legal requirement for some customers |
| **★ Live translated captions** | A Flemish/Walloon/German meeting where everyone reads their own language. The most European feature in the product |

## Stage 3b — the ones only alo can build

Everything in Stage 3 above is a feature Zoom could ship next quarter. These
cannot be copied by anybody who does not already hold the calendar, the CRM,
the tasks, the files and the mail — which is the whole argument for one
product. Each is small on its own; together they are the reason to switch.

| | |
|---|---|
| **The agenda is in the call** | The invitation already has a description and a duration. Show it, tick items off, and the meeting knows it is a 30-minute meeting with four things to cover. Nobody else has the invitation |
| **Who you are talking to** | On a customer call, the participant panel shows the deal, the last invoice, and what was agreed last time — from CRM and Billing, beside their face. This is the single most valuable thing on this list for a business, and Teams cannot do it because it does not know who your customers are |
| **Decisions become tasks, with owners** | Not "action items in a summary" — real Tasks in alo, assigned, due, appearing on somebody's board before they have left the call. The summary is the by-product; the task is the point |
| **The follow-up writes itself** | A draft email to the attendees, in Mail, with what was decided. Proposed and approved like everything else — never sent by a machine |
| **Notes sent to everyone who was there** | A setting, because a team that meets weekly should not approve the same email fifty times. Attendance is already recorded when somebody takes a join token, so the recipient list is a fact rather than a guess. **The design tension is real and is settled below** |
| **The room's files are at hand** | A meeting started from a chat room can offer what was shared in that room, so nobody hunts through Drive while eleven people watch |
| **"What have I missed?"** | Joining late, ask the agent. It has the transcript so far and the room's history. Every other product makes you interrupt |
| **Consent as a record, not a checkbox** | Recording in the EU is a legal act. Who consented, when, and to what, stored as an artefact somebody can produce later. Zoom shows a banner; a European product should keep evidence |
| **Notes in a real document** | Shared notes during the call are an alo Doc from the start, so they survive in Drive rather than in a panel nobody opens again |
| **Meeting templates** | A "customer demo" that always brings the deck, the CRM card and the agenda. Templates work here because the meeting can reach the things it needs |

### Sending the notes automatically — where the line is

Everything else in alo that reaches outside waits for a person: an agent
drafts an email and somebody presses send, because sending is irreversible and
a mistake arrives in somebody else's inbox. Automatically mailing minutes to
everyone who attended is exactly that act, and a setting that switches it on is
a setting that switches off the rule.

The resolution is not to refuse it — a team meeting weekly should not approve
the same email fifty times — but to make the boundary the one that already
matters everywhere else in the product:

- **Inside the workspace, automatic is fine.** Colleagues were in the meeting,
  they can already read the room, and the notes tell them nothing they were not
  present for. Default on.
- **Outside the workspace, it is a draft.** The moment a recipient is not a
  member — a customer, a candidate, a supplier — the send waits for a person.
  Not because the content differs, but because nobody can un-send a summary
  containing a sentence somebody wishes they had not said. Default off, and
  switching it on is a deliberate act with the consequence written on it.
- **Guests get what guests heard.** If a guest joined for twenty minutes of a
  ninety-minute meeting, minutes covering the other seventy are a disclosure.
  Either send them the part they attended or send them nothing.
- **Who it came from is a person, not the system.** The mail is from the
  organiser, so a reply reaches somebody rather than a mailbox nobody reads.

This depends on Stage 3's transcript and minutes; it is listed here because
the setting must be designed with them rather than bolted on afterwards, when
the easy answer is one switch that mails everybody everything.

Two more, honestly labelled as table stakes rather than differentiators, and
both genuinely expected in 2026: **noise suppression**, and **speaking-time
balance** shown privately to the host — the second costs almost nothing and
changes how meetings run.

## Stage 4 — scale and the long tail

Webinar mode, breakout rooms, polls, whiteboard, dial-in by phone, hardware
room systems, recording retention policy, per-tenant quality tuning, mobile
apps.

## What we should not chase

- **Beauty filters, avatars, virtual backgrounds beyond blur.** Consumer
  features that cost real engineering and win no business customer.
- **Our own SFU.** The doctrine is explicit: video internals are integrated,
  not built. Revisit only with paying customers and a specific reason.
- **Matching every Zoom setting.** Their settings screen is an artefact of
  fifteen years of enterprise requests. Copying it copies the debt.

## Order, and why

1. **Stage 1 first, entirely.** Every item is a reason to walk away, and
   walking away happens in the first meeting.
2. **Then Stage 3's recording and minutes**, ahead of most of Stage 2. They are
   why somebody picks alo over Meet, and Stage 2 is largely why they do not
   complain. Differentiators before polish, once the defects are gone.
3. **Stage 2 as it is needed**, guest access first — without it alo Meet cannot
   host a customer call, which is most of the meetings a business has.
4. **Stage 4 when customers ask.**

## How to build it without the failure that produced this document

The first UI rewrite changed five things at once, passed typecheck, eslint, 398
tests and a production build, and rendered a blank page. The layout fix after it
was attempted three times against class names that were guessed rather than
read.

So: one change, one screenshot, every time. The tests cannot see this module —
its whole surface is other people's video in a layout we do not control — and
that makes looking the only verification that counts.
