# alo Meet

The record is ours; the media is the engine's. LiveKit runs as a sealed,
version-pinned container and is contacted only through its public contract —
SDK in the browser, JWT for admission (`docs/alo-product-description.md`, what
we build vs. integrate: video/WebRTC internals are explicitly not ours).

## What the engine is told, and what it is not

The engine sees an opaque room name and a signed token. It never sees a tenant,
a title, or an address.

- **The room name is generated, never derived from the title.** A room called
  `q3-budget-acme-renewal` would put a customer's name in a third party's logs,
  and a meeting title is exactly the kind of thing that names a customer. Two
  meetings with the same title get different rooms.
- **A participant is a workspace user id**, meaningless outside alo, with the
  local part of an address as a display name. A participant list in someone
  else's logs must not be a list of who works at a customer.
- Both rules are asserted by tests that scan the room name and the minted
  claims for tenant words, not merely intended.

## Admission

The store decides, then a token is minted from its answer — never from a
request. `join_meeting` answers first; `mint` checks nothing, and that ordering
is the security property.

A token grants join on exactly one room, lasts five minutes, and is minted per
join. The secret never leaves the server: a browser receives a token, never a
key, and cannot mint another.

## Who may see a meeting

| Attached to | Visible to |
|---|---|
| A chat room | Whoever can read that room |
| A calendar event | Whoever can see that event |
| Nothing | Whoever started it |

There is no fourth case. A meeting nobody can place is a meeting nobody should
find, so a guessed id opens no call.

## Rules that came from thinking about failure

- **A finished meeting cannot be rejoined.** The engine would happily make a
  new room of the same name; the record refuses.
- **Joining twice is one attendance**, and does not move when the meeting
  began.
- **An event has at most one meeting.** Two links on one invitation puts half
  the attendees in the wrong call, so the button changes to Join rather than
  offering to make another. Ended meetings are ignored — a weekly that finished
  last Tuesday must not hand out last Tuesday's room.
- **Attendance is written by us**, when a token is taken, rather than read back
  from the engine later. Engines are swappable; attendance is evidence.
- **Microphone on, camera off.** Somebody joining expects to be heard and to
  choose to be seen; the reverse surprises people in a way that cannot be
  undone once it has happened.
- **Our own Leave button sits above the engine's controls.** Leaving must not
  depend on a third party's UI continuing to render.
- **No engine configured is its own answer** — 503 naming the deployment fact.
  The meeting is real and attendance is recorded; there is simply nowhere to
  hold it. A 500 would send an administrator hunting a bug instead of a
  setting.

## Surface

| Route | |
|---|---|
| `POST /meet` | Start one, optionally on a channel or an event |
| `GET /meet/{id}` | One meeting, if it is yours to see |
| `POST /meet/{id}/join` | Record attendance, mint a token |
| `POST /meet/{id}/end` | Declare it over; idempotent |
| `GET /meet/{id}/participants` | Who has been in it |
| `GET /meet/channels/{id}` | What is running in a room |
| `GET /meet/events/{id}` | The meeting on an invitation, or `null` |

**`/meet/*` must be named in the dev proxy and in every Caddy backend matcher.**
Both forward a list of prefixes; a route they have not heard of reaches the SPA
and answers 404. This has now cost two afternoons — once for chat, once here.

## Configuration

`ALO_MEET_URL`, `ALO_MEET_API_KEY`, `ALO_MEET_API_SECRET`. All three or none: a
half-configured engine mints tokens that are refused, which is harder to
diagnose than an absent one.

## Verified

Against LiveKit 1.13.5 locally: a token alo minted passes the engine's own
`/rtc/validate`, a browser join creates the room under our opaque name, and the
conference UI mounts. **Not yet verified:** two participants seeing each other's
video.

## Not built

Recording to Drive with consent indicators, AI minutes, live captions and
translated captions — all in `docs/features.md` § Meet. Recording is the next
one and needs LiveKit Egress. Remote control is ADR 0039 and deliberately
separate: it is not a meeting feature, it is a different threat model.
