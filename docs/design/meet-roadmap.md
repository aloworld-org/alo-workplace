# alo Meet — the gap to Meet, Zoom and Teams

What exists, what is missing, and the order to close it. Written after the
first slice shipped (`docs/design/meet.md`), because "make it as good as Google
Meet" is not a task until it is a list.

**How this list is ordered, and why that matters more than its length.** Each
stage is usable on its own, and the earliest ones remove the reasons somebody
would refuse to use alo Meet at all. That ordering is the whole value of the
document: a demo that fails in a real customer meeting fails on a Stage 1
defect, never on a missing Stage 4 feature.

Nothing here is a judgement about what can be reached. Incumbency is not a
moat — it is a list of decisions made years ago that nobody can revisit. The
reason to close Stage 1 before Stage 3 is that a broken join ruins the meeting
where you would have shown them the minutes, not that the gap is wide.

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
| **Meeting from a calendar invitation, for guests** | External participants without an alo account. This is the largest single design question in Meet and needs its own ADR: identity, admission, and abuse |

## Stage 3 — the differentiators, and the reason to choose alo

| Gap | Notes |
|---|---|
| **Recording to Drive with consent indicators** | LiveKit Egress. Consent is not a checkbox: it is a visible, persistent state for everyone in the call |
| **Transcript** | Speech-to-text, EU-hosted per the doctrine. Not OpenAI's US endpoint |
| **★ AI minutes** | Summary, decisions and action items posted to the meeting's chat thread. The features list calls this "included, not a €30/user add-on" — it is the commercial argument for the whole module |
| **Live captions** | Accessibility, and a legal requirement for some customers |
| **★ Live translated captions** | A Flemish/Walloon/German meeting where everyone reads their own language. The most European feature in the product |

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
