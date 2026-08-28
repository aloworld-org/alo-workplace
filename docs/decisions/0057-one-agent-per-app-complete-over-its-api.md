# ADR 0057 — One agent per app, complete over its API

**Status:** accepted — extends [ADR 0034](0034-per-product-agents.md) and
[ADR 0047](0047-reads-answer-writes-propose.md); **revisits two exclusions**
of `docs/design/chat-agents.md` (agent-to-agent conversation, autonomous
monitoring) on purpose and with new facts
**Date:** 2026-08-28
**Context:** `platform/alo-ai/src/agent_product.rs`,
`products/mail/alo-jmap/src/agent.rs`, `products/mail/alo-jmap/src/server.rs`,
`docs/design/complete-agents.md`, the evaluation run of 2026-08-28 recorded in
`docs/autonomy/agents/STATE.md`

## The decision in one line

Each app has **one agent that can do everything the app can do — and nothing
else — when asked**; agents may **hand work to each other** under the asker's
authority; and an agent **remembers what a channel shared with it**, the
channel being the boundary of what it may remember.

## What was actually wrong

On 2026-08-28 every one of the seventeen agents was asked one question in a
real room, against the tenant's own model. All seventeen answered, in two to
five seconds, and none invented anything. Six of them — Billing, Sales,
Finance, Projects, Drive, Docs — answered *"I could not find it"* to questions
the record answers plainly: nineteen quotes exist and the Billing agent said it
could not find any open ones.

No tool ran. The Billing agent's tools are `create_invoice_draft`,
`quote_to_invoice` and `draft_payment_reminder`: three writes, no read. Sales
and Finance are the same shape. The Billing module itself exposes forty-four
routes; the agent reaches three of them. Across the workspace the modules
expose some five hundred routes and the agents reach about seventy, each one
hand-written, hand-documented and hand-scoped.

So the failure is not the model and not the framework. It is **coverage**: an
agent is a short list of things somebody thought to write, over an app that
can do far more. "Knows its app in and out" was a name, and the evaluation
showed it.

ADR 0034 also drew two lines that this evaluation puts in a new light. *Agent
to agent conversation* was excluded as "a loop with a bill attached", and
*autonomous monitoring* as "a scheduled job wearing an agent's coat". Both
exclusions were right about the failure mode and wrong about the remedy: the
answer to a loop is a budget, not a ban, and a scheduled job is exactly what a
standing instruction to a colleague is.

## What we build instead

1. **Coverage by construction.** Every module publishes a **capability
   manifest** — one file, one responsibility — describing each route the
   module's own web client uses: name, purpose, arguments, and effect
   (`Read`/`Write`), in the module's own words. The agent's tool set is
   *derived* from the manifest, not written beside it. A route that is not in
   the manifest is listed there as **excluded, with the reason**. The definition
   of done for an agent is a test: **every route of its module is either
   reachable or explicitly excluded.** A new route is a new capability the day
   it ships, or a failing test.
2. **Only when asked; then anything.** An agent speaks and acts only when a
   person asks — by mention, in a DM, or in a **standing instruction** ("every
   Monday, post the open quotes here"), which is a person asking once, in
   advance, for a bounded thing, visible in the room and cancellable by its
   author. There are no unsolicited offers and no ambient commentary. Within
   what it was asked, the agent may use any capability of its app.
3. **Delegation, not conversation.** An agent may hand a sub-question or a
   sub-task to another agent — the Billing agent asking Sales for the deal
   behind a quote — **inside one run, under the asker's authority, to a depth
   of two, within one budget, and visibly**: the room sees who asked whom for
   what. Two agents never talk to each other outside a run.
4. **The channel is the memory boundary.** If something was shared in a
   channel, the agents in that channel may remember it, whoever wrote it:
   membership is the consent. What an agent learned in a channel is usable in
   that channel and nowhere else; a DM with an agent feeds only that person's
   memory; a memory dies with the message or the channel it came from. An
   explicit "remember that …" works everywhere. What an agent remembers is a
   page anyone in the channel can read and the owner can forget from.
5. **What does not change.** Every turn runs through the asker's account door;
   reads answer and writes propose (a standing instruction is its author's
   pre-approval, bounded to what it names); only the asker approves; every run
   is audited; a tool the agent was not offered is refused at the execution
   boundary, whatever the model said.

## Consequences

- The agent framework stops being a hand-written tool list and becomes a
  reader of manifests. The first manifest is Billing's, because that is where
  the evaluation failed loudest and where a customer will ask first.
- `docs/design/chat-agents.md`'s "out of scope" for agent-to-agent conversation
  and autonomous monitoring is **superseded** by points 3 and 2 of this ADR;
  the rest of that note stands.
- The seventeen-question evaluation of 2026-08-28 becomes the standing
  acceptance test of the agents track, run against a real model, its answers
  recorded verbatim — a green suite proves a tool runs, not that an agent
  answered.
- Memory introduces a table (`agent_memories`), a per-channel switch and a
  "what I remember" page; the design note carries the shape.
- Delegation and standing instructions introduce budgets — steps, depth, model
  calls, and a wall clock — enforced in code, never in the prompt, because the
  model is the untrusted party.

## Rejected

- **Hand-writing forty tools per module.** Months of work that drifts every
  time a route changes, and a coverage claim nobody can test.
- **Giving the model the raw route table.** Routes are for clients; their doc
  comments are not written for a model and carry no effect bit. The manifest
  is the place where a module says, in words meant for an agent, what each
  capability is for — and where it says what is deliberately withheld.
- **Free agent-to-agent conversation.** The loop with a bill attached is
  still a loop. Delegation inside a bounded run keeps the benefit and drops
  the loop.
- **Tenant-wide memory.** An agent that repeats in one room what it learned in
  another has widened access with a friendly face. The channel boundary is the
  only scope under which "remember everything shared here" is safe.
- **Proactive offers ("I noticed this quote expires Friday").** Wanted by
  everyone in the abstract, and the fastest route to an agent people mute.
  Standing instructions give the same value on the person's terms.
