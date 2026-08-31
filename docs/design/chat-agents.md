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

- `chat_agents` — tenant_id, id, handle (`alo`), name, description, **product**,
  created_at, disabled_at. The identity an agent posts as. Tenant-scoped, so a
  tenant can name its own.

  `product` (migration 0401) is what the agent is the agent **of**: one of the
  rail's own module ids (`mail`, `agenda`, `tasks`, `chat`, `drive`, `billing`,
  `crm`, `projects`, `finance`, `inventory`, `hr`, `insights`, `meet`, `sites`),
  or `workspace` for "Ask alo". It is the value everything else scopes by, and
  it is read in two places that must not be confused:

  - the **prompt** offers an agent its own product's tools and describes it as
    that product's agent (`alo-ai`'s `agent_product`);
  - the **execution boundary** refuses every other product's tools, whatever the
    model returned, reading the product off the agent's own row rather than
    taking it from whoever built the run.

  Only the second is a permission. A prompt that asks nicely is not one: the
  model is the untrusted party, and an injected turn will name a tool it was
  never offered. A refused tool leaves an audit row with `ok = false`, and a
  refused *lookup* is handed back to the model as its result, so the turn ends
  with the agent naming the agent that owns the question rather than putting a
  button on a lookup (which ADR 0047 removed).

  Sharing the vocabulary with `tenant_user_module_denials` (migration 0208) is
  deliberate: it is what lets an agent be gated on the module access an admin
  has already decided, instead of a second permission system that can disagree
  with the first. `mail` and `workspace` have no module — mail cannot be denied,
  and `workspace` is not a module.

  **That gate is now applied** (A1.5). Every agent read joins the denial table
  on the agent's product, so a person who cannot open Inventory has no
  `@inventory`: not in the list, not by id, not in a room they share with a
  colleague who does have it, and not as the counterpart of a one-to-one they
  opened before the switch was thrown (its history stays readable; nobody
  answers in it).

  Two products' words are **not** their module's, and the join translates both:
  `sheets` (A2.2) and `docs` (A2.3) are real products with agents in ADR 0034
  and no rail app of their own — a spreadsheet and a document are both Drive
  nodes, opened from Drive — so `AgentProduct::module` answers Drive for each
  and the SQL says `CASE a.product WHEN 'sheets' THEN 'drive' WHEN 'docs' THEN
  'drive'`. Left untranslated either would compare a word against a column that
  can never hold it, and somebody denied Drive would keep `@sheets` and
  `@docs`: agents that read the very files they were denied. A unit test reads
  `module()` and holds the CASE to it, so a later product that borrows a module
  fails there rather than in production. Defining one is refused with a 422 rather than made and
  hidden, because an agent its author cannot then see would be a 200 followed by
  a 404. `NOT u.is_admin` is in the predicate: an administrator is never denied,
  which is `AccessFacts::may_open`'s own rule and exists so an admin who
  switched an app off for themselves can still reach the console.
- `chat_agent_seeds` (migration 0403) — the ledger recording that a tenant has
  been **given its default agents**: one per product, on the first read of
  `GET /chat/agents`, with nobody registering a handle by hand. Handles are the
  product words (`@mail`, `@sites`, …) except `workspace`, which is `@alo`.

  The names and descriptions come from the API edge's language tables
  (`chat_agent_names.rs`, `?lang=`), never from the store: an agent called
  `Websites` in a French tenant is a hardcoded English string in a European
  product. Each name is the **rail's own word for the module** — Sales, People,
  Websites — so the agent and the app a person clicks are recognisably the same
  thing.

  A ledger rather than "are there any agents yet", for the reason `inv_seeds`
  gives: once has to survive what it wrote, so a tenant that retires an agent is
  not handed it back the next morning. Each insert is `ON CONFLICT DO NOTHING`
  besides, so a tenant that had already registered its own `@mail` keeps theirs,
  name and all, and is given the rest.

  That same "once" would leave a tenant seeded **before** a product existed
  permanently without its agent, which is the other half of A1.5's promise. So a
  product built later is offered once more, under **its own ledger key**
  (`LATER_AGENT_PRODUCTS`, today `sheets` → `default-agents:sheets` and `docs`
  → `default-agents:docs`): a tenant
  that never saw it gets it, a tenant that threw it away keeps it thrown away,
  and a tenant seeded from scratch today already has it, finds the handle taken,
  and simply records the key.
- `chat_channels.kind` gains **`agent_dm`** and `chat_channels.agent_id`
  (migration 0402, ADR 0048) — a one-to-one between one person and one agent.
  A DM could not hold one: `dm_key` is "both member ids sorted and joined", two
  **user** ids, and 0141 deliberately refuses to make an agent a user. So it is
  a third kind rather than a second meaning for `dm`, and old code that switches
  on `kind` refuses it instead of misreading it as two humans.

  One `chat_members` row (the human) and one `chat_agent_members` row (the
  agent). `dm_key` stays `NULL`; a partial unique index over
  `(tenant_id, agent_id, created_by)` gives one room per person per agent, so
  opening it twice is the same conversation and two colleagues asking the same
  agent get two separate ones. It is created on first open — a tenant with a
  dozen agents does not get a dozen empty rooms — and it is private to its own
  human, which the existing `kind = 'channel'` filter on discovery already
  enforces.

  **In an `agent_dm` every message from the human is the trigger**: there is
  nobody else it could be addressed to, so no handle is typed. The room is asked
  which agent it is with; the words are not parsed. An agent's own message is
  posted through `post_as_agent`, which is not the path that triggers a turn, so
  an agent cannot answer itself and two agents cannot be arranged into a loop.
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
| `GET /chat/agents[?lang=nl]` | The tenant's agents that **this caller** may see — for the composer's `@` list and the member sheet. **Seeds the default set on the tenant's first read**, named in `lang` (that language is the tenant's from then on; the seed runs once and nothing retranslates an agent a human may have renamed). Each carries its `product` and its **record**: answers given and actions approved, counted **only over rooms the caller can see**, so two people can legitimately be shown different numbers for the same agent. An aggregate leaks too, just more slowly — a tally that included private rooms would answer "is that agent busy somewhere I cannot see?" |
| `POST /chat/agents` `{handle, name, description?, product}` | Define an agent. `product` is **required** and has no default: the only sensible default would be `workspace`, which is every tool in the workspace, and the widest agent must not be the one you get by forgetting. An unknown word is a 422 naming the accepted set, and so is a product this caller cannot open |
| `POST /chat/agents/{id}/dm` | Open the caller's one-to-one with an agent (ADR 0048), creating it once and returning the same room every time after. A route of its own rather than a third shape of `POST /chat/channels`, whose DM branch takes `{with}` — a **user** id; naming an agent there is the same confusion the schema refused. 404 for an agent this tenant does not have, 422 for a retired one |
| `GET /chat/agents/directory[?lang=nl]` | **The agent directory** (A3.3) — the same roster as `GET /chat/agents`, seeded the same way and gated the same way, with two things added to each entry: `gatedOn`, the rail module whose switch decides whether this person has the agent at all (`null` for mail and Ask alo, `"drive"` for the two Drive-node products), and `tools`, every tool of its product as `{name, effect}` read straight from the registry the **execution boundary** asks, so the directory cannot describe a reach that would be refused. What an agent is *for* is its own `description` — tenant data in the tenant's language — and never the English `headline` its system prompt opens with |
| `GET /chat/agents/{id}/directory` | One directory entry, plus `recent`: the last twenty runs behind its tallies, `{id, tool, effect, ok, channel, at}` and deliberately **no `args`** — what ran and whether it worked is the record; what it was asked *about* is a message body, a person's name, a document's text. The runs are the **caller's own** (a run is an act through one person's access, so a colleague must not read which diaries were opened for somebody else), which is why this is a route rather than a `recent` field on every entry of the roster. 404 for an agent this tenant does not have **and** for one whose module this caller may not open — the same answer, so the refusal is no oracle |
| `POST /chat/channels/{id}/agents` · `DELETE …/agents/{agent}` | Add or remove an agent from a room (owner only, like any member) |
| `GET /chat/channels/{id}/turns` | Agent turns running in this room right now, so a room does not look idle while a model thinks |
| `POST /chat/channels/{id}/turns/{turn}/stop` | Stop a running turn — **only the person who asked**, for the same reason only they may approve what it proposes. Answers 204 even when nothing was found: the turn may have just finished, and what the caller wanted is true either way |
| `POST /chat/proposals/{id}` `{approve}` | Decide a pending proposal. **403 for anyone but the asker**, with the reason said plainly. Approving **runs the action in the same request**, through the one executor the command palette already uses — recording a decision the client must then follow up on would let the record and the effect drift, which is what this table exists to prevent |

An agent turn is not a route. Saying something to an agent in an ordinary
`POST …/messages` triggers it, because the trigger *is* the message — a
separate "ask the agent" endpoint would let the two disagree about what was
said. In a channel that means naming it; in a one-to-one with it, every message
already is.

## Errors

| Case | Answer |
|---|---|
| Approving someone else's proposal | **403** naming the rule — the room can see it, so there is no secret to keep, only a permission |
| Approving a proposal for another product's tool | **403** naming the tool and the agent's product, and **nothing runs**. Approval widens *who* may run a tool, never *which product's* tools an agent has. The attempt is audited with `ok = false` |
| A lookup belonging to another product | Refused at the boundary and reported back to the model as the tool's result, so the agent answers by saying which agent owns the question. Never a proposal: a button on a lookup is the bug ADR 0047 removed |
| Approving one already decided | 422 saying what it became |
| Mentioning an agent that is not in the room | Nothing. Plain text, exactly as an unresolved `@person` is today |
| Mentioning an agent of a module you cannot open | The same nothing, and **no model call**. To that person the room has no such member to name, so there is no turn to refuse partway through — while the colleague beside them who still has the module is answered in the very same room |
| No AI provider configured | The agent posts nothing and the asker is told once. Matching `/ai/agent`'s soft answer, not the 503 other AI routes give — an unconfigured model must not make chat look broken |
| The model is unreachable | Same: said once, in the room, as the agent |

## Tenancy

`chat_agents` carries `tenant_id`; an agent is added to a room through the same
membership path a person is, so a room in another tenant does not resolve.
Every retrieval and every execution runs through the **asker's** `AccountStore`,
so tenancy is enforced exactly once, in the place it is already enforced, and
an agent has no account door of its own to widen.

## Out of scope, deliberately

> **Superseded in part by [ADR 0057](../decisions/0057-one-agent-per-app-complete-over-its-api.md)
> (2026-08-28):** agent-to-agent *delegation inside one run* and *standing
> instructions* are now in scope — see `complete-agents.md`. Free agent-to-agent
> conversation and unsolicited monitoring stay out, for the reasons below.


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

## The citation, and the footnote behind it

An agent may answer only from a numbered list of sources it was handed for that
turn, and must cite each claim by its number — "Ben owns the rollout [2]". That
rule is what makes "never invent files, people, or facts" checkable rather than
promised, and it is why an agent with nothing to cite says it could not find the
answer instead of writing something plausible.

Until 2026-08-31 the room was shown the citation and never the list. The sources
existed inside the turn, were numbered, were cited, and were then thrown away, so
a reader met `[2]` with nothing to resolve it against. That is worse than no
citation at all: it looks like a broken link, and it invites trust ("it cited
something") while withholding the one thing a citation exists to allow — the
check.

**Surface.** `chat_messages.sources` (JSONB, nullable, migration `0912`) holds
`[{n, kind, title}]` in citation order. `post_as_agent_cited` writes it;
`post_as_agent` stays exactly as it was and writes none, because most of what an
agent says — a plan, a refusal, the sentence describing a proposal — cites
nothing and the answer path is the only one holding the list. Every chat message
in the API carries a `sources` array, empty where there are none, so no client
has to guard a sometimes-absent field. The room shows a collapsed
"Answered from N sources" under the answer, opening to the numbered list.

**What travels, and what does not.** The number, the kind and the title. Not
`detail` — that is body text a source was summarised from, and the room is
showing a footnote, not republishing the record. Nothing new is readable either:
the list is the asker's own grounding, already theirs to see, stored on their own
message in their own room.

**Out of scope, deliberately.** A source does not link to its record yet — the
grounding carries `(kind, id, title)` internally, but wiring each kind to the
screen that opens it is per-module work. A proposal's sentence cites nothing, so
it has no list. Messages written before the column have none and are not
backfilled: there is nothing to backfill from.

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
