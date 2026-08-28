# Complete agents — one agent per app, delegation, channel memory, standing instructions

*Design note for [ADR 0057](../decisions/0057-one-agent-per-app-complete-over-its-api.md).
Extends `chat-agents.md`, which stays authoritative for identity, authority,
approval and the room mechanics. Nothing here is built yet; the agents queue
(`docs/autonomy/agents/QUEUE.md`, waves A4–A8) executes it.*

## What this is for

Sila (silahq.com) set the bar for *where* agents live: in the team's own
channels and DMs, `@`-mentionable, acting across tools, aware of one another.
We already have that shape. What we do not have is the substance under it:
each agent reaches a handful of hand-written tools over an app that can do far
more. This note is how an agent becomes complete over its app, how agents hand
work to each other, what an agent may remember, and how a person asks once for
something recurring.

## Surface

| Who | Sees |
|---|---|
| A person in a channel | `@billing which quotes are open?` answered from the record; `@billing invoice the Northstar quote` proposed with one tap; a visible line when Billing hands part of the work to Sales; a **standing instruction** card the author can cancel; a **What I remember** page per agent per channel. |
| A person in a DM with an agent | The same, with memory scoped to that one person. |
| A module owner | A **capability manifest** to keep true; a coverage test that fails when a route is neither reachable nor excluded. |
| An admin | Which agents exist, their manifests, their run record — the directory that exists today, now complete. |

## 1. Coverage by construction — the capability manifest

Each module ships one manifest, one file, one responsibility:
`products/mail/alo-jmap/src/<module>_capabilities.rs` (billing, crm, finance,
projects, inventory, hr, drive, docs, sheets, sites, tasks, calendar, chat,
meet, insights, mail).

```rust
Capability {
    name: "open_quotes",                       // the tool name
    purpose: "The offers that are still open, newest first, with what each is worth.",
    route: Route::Get("/billing/quotes?status=sent"),
    args: &[Arg::opt("customer", "Limit to one customer, by name or id")],
    effect: Effect::Read,
    answers: &["which quotes are open", "what did we quote X", "what is X worth"],
}
Excluded { route: Route::Delete("/billing/quotes/{id}"), why: "A draft is deleted by a person, in the app, on purpose." }
```

Rules:

- **Derived, never duplicated.** The agent's tool set, the prompt's tool
  lines, the execution boundary's allow-list and the directory's `tools`
  entry are all renderings of the manifest. There is no second list.
- **Every route is accounted for.** The coverage test walks the module's
  router and fails on a route that is neither a capability nor an exclusion.
  An exclusion is a sentence, not a shrug.
- **Execution goes through the route.** A capability runs the same handler
  the web client calls, with the asker's token — so an agent can never do
  more than the app, and validation, tenancy and audit are the route's own.
- **Effect is declared once**, on the capability, as ADR 0047 requires.
- **Examples ground the router.** `answers` are the questions the capability
  exists for; Ask alo's planner and the agent's own tool choice read them.
  They are also the seed of the evaluation set.

The Billing manifest comes first (forty-four routes). Then Sales, Finance,
Projects, Drive, Docs — the six that failed the evaluation — then the rest.

### Multi-step turns

A complete agent needs more than one read per turn: "what did we quote
Northstar" is *find the customer → list their quotes → read the one*. The turn
loop allows up to **six** read executions and **two** model calls per turn
(ADR 0047 allowed three and two); a write still ends the turn with one
proposal. Budgets are constants in code, not prompt text.

## 2. Delegation between agents

Inside one run, an agent may **hand off** a sub-question or sub-task:

```
{ "delegate": { "to": "crm", "ask": "which deal is behind quote Q-2026-00031?" } }
```

- Runs as an ordinary turn of the named agent, **with the asker's authority
  and scope** — the delegate reaches nothing the asker could not.
- **Depth ≤ 2** (an agent may delegate; its delegate may delegate once more;
  no further). **At most four delegations per run.** The run's budget is one
  budget across all of them.
- **Visible**: the room gets a system line — *Billing asked Sales: which deal
  is behind quote Q-2026-00031?* — and the delegate's answer is folded into
  the asking agent's turn, cited.
- **Only to agents the asker can see** (module gating), exactly as Ask alo's
  planner already drops unknown handles.
- A write proposed by a delegate is proposed **to the asker**, in the asking
  agent's name, with the delegate credited. One approval surface.
- Never across a shared (cross-company) channel in v1.

Ask alo's planner becomes a special case of this: it is the agent whose whole
job is to delegate.

## 3. Channel memory

**The channel is the consent boundary.** What was shared in a channel may be
remembered by the agents in it, whoever wrote it.

| Scope | Learned from | Usable in |
|---|---|---|
| `channel` | messages in that channel | that channel |
| `person` | the person's DM with the agent | that person's DM with it |

- **What is remembered**: facts and decisions the agent extracted, each with
  the message it came from — *"Northstar is on net 30 (from Anna, 12 Aug)"* —
  never transcripts.
- **When**: at the end of a turn the agent took part in, over the messages it
  read for that turn; and on an explicit *"@billing remember that …"*, which
  works even where learning is off.
- **Switch**: per channel, in room settings, on by default; off means no
  learning and the existing memories for that channel are hidden (kept for
  30 days, then deleted). Workspace-level default in the admin console.
- **Visible**: *What I remember* — per agent, per channel; every member reads
  it; the owner forgets one item or all; the author of a memory's source
  message may forget it too.
- **Deletion follows the source**: deleting a message deletes what was learned
  from it; archiving a channel deletes its memories; removing an agent from a
  channel deletes what it learned there.
- **Retrieval**: a turn may read only memories whose scope the asker is inside
  — the channel it runs in, or the asker's own DM memory. Nothing cross-channel.
- **No cross-channel pooling in v1.** A company-wide fact is a "remember
  that …" in the channel everyone is in.

### Data model

```sql
CREATE TABLE agent_memories (
    tenant_id     TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    id            TEXT NOT NULL,
    agent_id      TEXT NOT NULL,                 -- chat_agents
    scope         TEXT NOT NULL,                 -- 'channel' | 'person'
    channel_id    TEXT,                          -- scope = channel
    user_id       TEXT,                          -- scope = person
    fact          TEXT NOT NULL,                 -- one sentence
    source_msg    TEXT,                          -- chat message id, NULL for "remember that"
    learned_from  TEXT NOT NULL,                 -- user id of the message's author
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, id)
);
-- channel settings gain: agent_memory BOOLEAN NOT NULL DEFAULT true
```

Memory is tenant data in the tenant's language; it is never sent to a model
except inside a turn in its own scope.

## 4. Standing instructions

A person asks once, in advance: *"@billing every Monday at 9, post the open
quotes here"*, *"@finance when an invoice is 14 days overdue, draft the
reminder"*.

- **A card in the channel**, with the instruction in the author's words, the
  schedule or trigger, the agent, and *Cancel* — live for the author and the
  room owner, inert for everyone else.
- **Runs as the author**: each firing is a turn with the author as asker.
  Reads post their answer; a write is proposed to the author (a Monday post is
  a read; drafting a reminder is a write and waits for the author's tap, as
  any write does). A standing instruction is the author's pre-approval of
  the *read*, never of a write.
- **Bounded**: at most one firing per instruction per hour; at most twenty
  instructions per channel; an instruction whose author leaves the channel is
  paused and shown as such.
- **Two kinds of trigger in v1**: a schedule (cron-like, in words the client
  turns into a schedule) and a module event the manifest names as a trigger
  (`invoice.overdue`, `quote.expiring`). Nothing else — "when X" over
  arbitrary conditions is a query the agent runs on a schedule, which the
  first kind already covers.
- Lives in `agent_instructions` (tenant, id, agent, channel, author, text,
  trigger, next_run, paused) and runs on the same background sweeper that
  sends scheduled mail.

## 5. Evaluation is the acceptance test

`docs/autonomy/agents/STATE.md` records the seventeen-question run of
2026-08-28. That run, extended per manifest with the capability `answers`, is
the acceptance test of every item in waves A4–A8: run against a real model,
answers quoted verbatim, with the tool runs behind them. A wave is done when
its agent answers its questions from the record, not when its tests are green.

## Errors

| Case | Answer |
|---|---|
| Capability the agent was not offered | `403`, refused at the boundary, run recorded (unchanged) |
| Delegation beyond depth or count | the step is dropped and the agent says which part it could not do |
| Budget exhausted mid-run | the turn ends with what it has, marked partial in the room |
| Memory learning off for the channel | turn runs, nothing is remembered, "remember that …" still works |
| Standing instruction's author gone | paused, card says so |

## Tenancy

Every capability runs through the asker's account door via the module's own
route. Memories carry `tenant_id` and a scope the retrieval joins on; the
wrong-tenant and wrong-channel tests are mandatory for `agent_memories` and
`agent_instructions`. A delegate turn is the asker's turn.

## Rollout

Per tenant, default off: `agents.complete` (manifest-derived tools),
`agents.delegation`, `agents.memory` (with the per-channel switch beneath it),
`agents.instructions`. Turning one on is a config change; off is the same
change. Watch: tool runs per agent per day, `ok` ratio, turns hitting a budget,
delegations per run, memories per channel, instruction firings.

## Out of scope for this note

Cross-channel or tenant-wide memory; proactive offers; agent-to-agent
conversation outside a run; delegation across shared channels; voice; bringing
outside agents (Claude Code, Cursor) into a room; a manifest for the admin
console (agents do not administer the workspace).
