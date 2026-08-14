# ADR 0048 — An agent as a DM counterpart

**Status:** accepted — extends [ADR 0034](0034-per-product-agents.md)
(per-product agents) and [ADR 0038](0038-chat-built-natively.md) (chat built
natively)
**Date:** 2026-08-14
**Context:** `platform/alo-store/src/chat.rs`,
`platform/alo-store/src/chat_agents.rs`, migrations `0132_chat_channels.sql`
and `0141_chat_agents.sql`

## The decision in one line

A one-to-one conversation with an agent is a **third channel kind**
(`agent_dm`) — one human, one agent, no `@` needed — because `dm_key` is a pair
of **user** ids and cannot express it.

## What was actually wrong

ADR 0034 says an agent is a first-class chat participant. Chat has two room
shapes, and neither of them holds one:

- A DM is identified by `dm_key`, documented in `0132_chat_channels.sql` as
  "both member ids sorted and joined", enforced by
  `CONSTRAINT chat_channels_shape` (a `dm` must have one) and by a unique index
  on `(tenant_id, dm_key)`. `dm_key("alice", "bob") == "alice:bob"` — two user
  ids, and the sortedness is the whole point of it.
- Membership is `chat_members.user_id`. Agents are not members; they live in a
  separate `chat_agent_members` table.
- `0141_chat_agents.sql` **deliberately** refuses to make an agent a row in
  `users`: that "would make an agent mailable, assignable, addressable in Spaces
  and countable as a seat, and would put a non-human in a table whose every
  consumer assumes a person." That refusal is right, and it is what closes the
  easy door here.

So the only way to reach an agent today is to already be in a room that has it
and type its handle. The shape every chat user reaches for first — open a
conversation with the thing you want to talk to — does not exist, and cannot be
faked without putting an agent id in a column that every reader treats as a
person.

## What we build instead

A third value of `kind`, `agent_dm`, with exactly one human and exactly one
agent:

- **Identity.** A new nullable `agent_id` column on `chat_channels`, with a
  partial unique index over `(tenant_id, agent_id, created_by)` where
  `kind = 'agent_dm'` — one room per person per agent, per tenant. `dm_key`
  stays `NULL` here and keeps meaning what it says: a pair of humans. The shape
  CHECK gains a third arm; the migration is expand-only, and the new constraint
  is strictly more permissive than the one it replaces, so no existing row is
  touched.
- **Membership.** One `chat_members` row (the human) and one
  `chat_agent_members` row (the agent). `add_member` and `remove_member` refuse
  an `agent_dm` the same way they already refuse a `dm` — "a direct message has
  exactly two people" — so the room cannot become a channel by accretion.
- **The trigger.** In an `agent_dm`, **every message from the human is the
  trigger**; no handle needed, because there is nobody else it could be
  addressed to. An agent's own message (`author_kind = 'agent'`) never triggers
  a turn, so two agents cannot be arranged into a loop and an agent cannot
  answer itself.
- **Visibility.** Private, listed only to its own human. The existing "browse
  channels" query is already `WHERE c.kind = 'channel'`, so an agent DM cannot
  surface in discovery, and the caller's own room list is a join on
  `chat_members` and therefore includes it with no change.
- **Opened on demand, idempotently.** Opening the same one twice returns the
  same room, exactly as a human DM does. A room is created on first open — a
  tenant with a dozen agents does not get a dozen empty rooms in its sidebar.
- **A retired agent** keeps its room readable and takes no new turns, which is
  the rule `add_agent_to_channel` already applies.
- **Proposals are unchanged.** A write proposed in an agent DM waits for a tap
  exactly as in a channel, and `asked_by` is the only human in the room —
  ADR 0047 applies here with nothing special about it.

**Scope: store and API only.** Rendering an agent DM in the room list, its
avatar and its badge are chat-UI work and wait for the chat rebuild; this ADR
does not decide them.

## Consequences

- An agent becomes something you can talk to rather than something you can
  mention, which is the difference between a participant and a feature.
- Three room kinds instead of two: every query that switches on `kind` gains a
  case, and the tenant-isolation tests gain a third surface to prove — a wrong
  tenant and a wrong user must both reach nothing through an agent DM.
- The private-room reasoning stays intact: an agent in a one-to-one still runs
  through its human's account door, so the room is not a way to see anything
  that human could not already see.
- `dm_key` keeps a single meaning. Nothing downstream that parses it as a pair
  of user ids has to learn a second shape.

## Rejected

- **An agent as a row in `users`.** Already rejected in `0141_chat_agents.sql`,
  and for the same reasons: it makes an agent mailable, assignable, a Spaces
  member and a billable seat, and it hands a non-human an identity in the table
  that authentication reads.
- **A synthetic user id inside `dm_key`** (`agent:mail:u_123`). It puts a
  non-user id in a column every reader treats as a user id, breaks the
  sorted-pair invariant that makes the key idempotent, and would silently
  produce a "DM" whose second member no membership query can find.
- **A named channel per agent** (`#mail-agent`). A channel implies membership,
  discovery and a shared history; people would ask an agent private questions in
  a room their colleagues can join. A one-to-one has to actually be one-to-one.
- **Reusing the existing `dm` kind with a nullable agent column.** It leaves
  `dm` meaning two different things, so every existing query over DMs becomes
  wrong-by-default until it is audited. A new kind is refused by old code paths
  instead of misread by them.
