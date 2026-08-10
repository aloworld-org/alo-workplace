# alo Chat — agents as participants (ADR 0034 §chat, ADR 0038)

ADR 0034 already decided this: every product gets a thin agent over one shared
framework, and "Chat agent(s) — first-class chat participants, @mentionable,
reply/react" is a listed line item. ADR 0038 restates it: agents are *ordinary
participants*, not bridge bots, and E2EE was rejected precisely so an agent can
read a channel.

What neither ADR settles is the question this note exists to answer:

> An agent posts **as itself**, but must read and act **as the person who
> asked it**. Whose permissions apply, and who may approve what it proposes?

Everything below follows from that one answer.

## The answer: identity and authority are different things

**Identity** is whose name is on the message. **Authority** is whose eyes and
hands were used to produce it. For a human they are always the same, which is
why no system here has needed to separate them before. For an agent they are
never the same.

- The agent has an **identity**: a row in `chat_agents`, a member of the rooms
  it belongs to, `@alo`-mentionable, with a name and an avatar. Its messages
  say they are its messages.
- Every agent turn has an **asker**: the person whose message triggered it.
  Retrieval, and any action the turn proposes, are computed through **that
  person's `AccountStore`** — the same account door everything else uses. An
  agent therefore cannot see or do one thing more than the human who summoned
  it, which is ADR 0034's "an agent cannot widen access" made structural
  rather than promised.
- Each agent message records `on_behalf_of` — the asker's user id. The room
  sees the agent; the audit trail sees whose reach produced it. Both are true
  and neither is hidden.

### Only the asker may approve

A proposal is computed under one person's access. If a second person could
approve it, the action would run with the *asker's* reach on the *approver's*
say-so — a privilege confusion with a friendly face. So:

**A proposed action may be approved only by the person whose message caused it.**

Everyone in the room sees the proposal, because a room where actions happen
invisibly is worse. But the buttons are live for one person, and inert, with
the reason stated, for everybody else.

This is the strictest rule that is still useful, and it is the one this note
adds to ADR 0034. Loosening it later (a room owner co-signing, say) is an
additive change; tightening it later would break behaviour people relied on.

## What an agent may do without being asked

From ADR 0034, unchanged: **autonomous replies and reactions are allowed;
autonomous actions are not.** In this module that means an agent may:

- answer when `@`-mentioned, citing what it used;
- react to a message;

and may never, without a tap:

- create, send, move, delete or modify anything.

An agent that is not mentioned says nothing. There is no ambient agent reading
along and volunteering — that is the "silent autonomous agent" ADR 0034
explicitly rejected, and a room full of unsolicited commentary is a worse
product besides.

## Data model

- `chat_agents` — tenant_id, id, handle (`alo`), name, description,
  created_at, disabled_at. The identity an agent posts as. Tenant-scoped, so a
  tenant can name its own.
- `chat_messages.author_kind` — `user` (default) | `agent`. `author_id` already
  has no foreign key to `users`, so an agent id lives there without schema
  violence. A reader tells them apart by `author_kind` and never by parsing an
  id.
- `chat_messages.on_behalf_of` — the asker's user id on an agent message,
  `NULL` otherwise.
- `chat_proposals` — tenant_id, id, channel_id, message_id, asked_by, tool,
  args (jsonb), state (`pending`|`approved`|`discarded`|`expired`),
  decided_by, decided_at, created_at.

  This table is new because the existing approval flow keeps a proposal **in
  React state only**. That is sufficient for a command palette used by one
  person for four seconds. It is not sufficient here: a chat proposal is
  visible to a room, must survive a reload, must be refusable, and must leave
  a record of who decided. A proposal nobody can audit is exactly the thing
  ADR 0023 exists to prevent.

## Routes

| Route | Does |
|---|---|
| `GET /chat/agents` | The tenant's agents — for the composer's `@` list and the member sheet. Each carries its **record**: answers given and actions approved, counted **only over rooms the caller can see**, so two people can legitimately be shown different numbers for the same agent. An aggregate leaks too, just more slowly — a tally that included private rooms would answer "is that agent busy somewhere I cannot see?" |
| `POST /chat/channels/{id}/agents` · `DELETE …/agents/{agent}` | Add or remove an agent from a room (owner only, like any member) |
| `GET /chat/channels/{id}/turns` | Agent turns running in this room right now, so a room does not look idle while a model thinks |
| `POST /chat/channels/{id}/turns/{turn}/stop` | Stop a running turn — **only the person who asked**, for the same reason only they may approve what it proposes. Answers 204 even when nothing was found: the turn may have just finished, and what the caller wanted is true either way |
| `POST /chat/proposals/{id}` `{approve}` | Decide a pending proposal. **403 for anyone but the asker**, with the reason said plainly. Approving **runs the action in the same request**, through the one executor the command palette already uses — recording a decision the client must then follow up on would let the record and the effect drift, which is what this table exists to prevent |

An agent turn is not a route. Mentioning an agent in an ordinary
`POST …/messages` triggers it, because the trigger *is* the message — a
separate "ask the agent" endpoint would let the two disagree about what was
said.

## Errors

| Case | Answer |
|---|---|
| Approving someone else's proposal | **403** naming the rule — the room can see it, so there is no secret to keep, only a permission |
| Approving one already decided | 422 saying what it became |
| Mentioning an agent that is not in the room | Nothing. Plain text, exactly as an unresolved `@person` is today |
| No AI provider configured | The agent posts nothing and the asker is told once. Matching `/ai/agent`'s soft answer, not the 503 other AI routes give — an unconfigured model must not make chat look broken |
| The model is unreachable | Same: said once, in the room, as the agent |

## Tenancy

`chat_agents` carries `tenant_id`; an agent is added to a room through the same
membership path a person is, so a room in another tenant does not resolve.
Every retrieval and every execution runs through the **asker's** `AccountStore`,
so tenancy is enforced exactly once, in the place it is already enforced, and
an agent has no account door of its own to widen.

## Out of scope, deliberately

- **Agent-to-agent conversation.** Two agents replying to each other is a loop
  with a bill attached.
- **Autonomous monitoring** ("tell me when X"). That is a scheduled job wearing
  an agent's coat, and belongs with the schedulers.
- **Per-agent custom tool sets.** The chat agent gets the workspace tool set
  ADR 0034 already defines. A tool that only exists in chat can come later.
- **Aborting the model call itself.** Stop is now built (below), but it
  declines to post a result rather than cancelling the request in flight —
  which is what someone pressing it wants, and costs no new dependency. A true
  abort belongs with the multi-step turns that do not exist yet.
- **Stop across processes.** The registry is in memory, so a Stop only reaches
  turns on the process that receives it. Acceptable while a turn is a single
  call of a few seconds; the thing to revisit before turns run long.

## Alternatives rejected

- **An agent as an ordinary `users` row.** Tempting — everything would just
  work. Rejected: it would make the agent mailable, assignable, addressable in
  Spaces and countable as a seat, all of which are wrong, and it would put a
  non-human in a table whose every consumer assumes a person.
- **The agent acting with its own permissions.** Simpler to reason about and
  catastrophic in practice: an agent with standing access becomes the widest
  credential in the tenant, and the first prompt injection in a channel spends
  it. Acting only as the asker means a hostile message can never reach further
  than its author already could.
- **Approval by any room member.** Friendlier, and wrong for the reason given
  above: it executes one person's reach on another's decision.
