# ADR 0058 — Intents, records and events: one command layer for people and agents

**Status:** accepted — **replaces the mechanism** of
[ADR 0057](0057-one-agent-per-app-complete-over-its-api.md) §1 ("coverage by
construction" is kept; the capability manifest over routes is not built);
extends [ADR 0034](0034-per-product-agents.md), [ADR 0047](0047-reads-answer-writes-propose.md)
**Date:** 2026-08-28
**Context:** `platform/alo-ai/src/intent.rs`, `platform/alo-ai/src/agent_product.rs`,
`products/mail/alo-jmap/src/billing_intents.rs`, `docs/design/complete-agents.md`

## The decision in one line

Everything that happens in an alo app happens through an **intent** — a verb
defined once, with typed arguments, an effect, a preview and (where the domain
allows) an undo — and the web client, the HTTP API and the app's agent all
dispatch the same intents. Records carry **provenance**; every intent emits an
**event**. Being AI-native means the app and its agent share one vocabulary,
not that an agent has been handed the app's API.

## What was actually wrong

ADR 0057 correctly named the failure — an agent reaches a handful of hand-written
tools over an app that can do far more — and proposed a capability manifest per
module: a description of each route, from which the agent's tools are derived.

That fixes coverage. It does not make the product AI-native. A manifest is a
second vocabulary kept in step with the first by a test: routes on one side,
capabilities on the other, a translator between them, and an agent that will
forever be "using the app from outside". It is what a company builds when it
does not own the app. We own every app.

## What we build instead

1. **Intents.** Each module defines its verbs once — `quote.open`,
   `quote.send`, `invoice.raise_from_quote`, `invoice.issue`,
   `payment.record` — as an `IntentSpec`: name, purpose in the module's own
   words, typed arguments, effect (`Read`/`Write`), the questions it answers,
   and for a write a **preview** (what will change, in one sentence, before it
   runs) and an **undo** when the domain has an inverse (a draft can be
   discarded; an issued invoice cannot be un-issued and says so). The HTTP
   routes become adapters over intents; the web client's buttons dispatch
   intents; the agent plans over intents. **There is no other list.** The
   prompt lines, the execution boundary's allow-list and the agent directory
   are renderings of the intent registry.
2. **Coverage is structural.** A module's coverage test names every route of
   the module and requires each to be either the adapter of an intent or an
   exclusion with a reason (print/PDF/export routes serve files; settings are a
   person's; imports take a file). A route that is neither fails the build.
3. **Record views.** Each module exposes the record a person sees on its detail
   page — a quote with its lines, customer, status, history — as the same shape
   an agent grounds in. No separate retrieval summary of a record exists.
4. **Provenance on records.** A record carries where it came from — the
   thread, the meeting, the quote, the message — so an agent explains rather
   than asserts, and every answer links into the record.
5. **Events as perception.** Every intent execution emits a domain event on the
   tenant's stream (`quote.sent`, `invoice.overdue`, `stock.below_minimum`).
   Notifications, audit, standing instructions and memory extraction consume
   the one stream; nothing polls.
6. **Actions are objects.** A person's click and an agent's proposal produce
   the same action record with the same preview and the same audit, so an
   action can be handed to an agent, a task assigned to one like to a
   colleague, and an agent's action undone with the button that undoes a
   person's.
7. **Goals.** Multi-step work across agents is a goal object — plan, steps,
   progress, one approval surface, Stop — not a conversation between agents.
   Delegation (ADR 0057 §3) is an intent of another module called inside a
   run as the asker.

Unchanged and absolute: the asker's account door, reads answer / writes
propose, asker-only approval, audit of every run, EU inference (ADR 0011).

## Consequences

- Billing is the reference: its 44 routes become ~25 intents with the routes
  calling them; the Billing agent plans over the same intents; the six
  questions that failed on 2026-08-28 are answered from the record.
- `alo-ai` gains `intent.rs` — the spec type and its renderings — and the
  hand-written per-product tool modules are replaced one module at a time.
  `ToolSet` stops being three string constants and becomes a rendering of
  intents.
- A turn may run up to six reads (ADR 0047 allowed three): "what did we quote
  Northstar" is customer → quotes → quote.
- Record views, events and action objects are additive: a module that has not
  moved yet keeps working; its agent stays as thin as it is until its intents
  exist. Roll-out is per module, flagged per tenant.
- Forms generated from intent schemas, goals and the daily brief are surfaces
  over this layer and come after it (agents queue waves A5–A9).

## Rejected

- **The capability manifest over routes** (ADR 0057 §1). Correct on coverage,
  wrong on nativeness: two vocabularies kept in step by a test.
- **Giving the model the router.** Routes are addressed to clients; an intent
  is addressed to whoever wants the thing done, in the module's words.
- **A separate "agent API".** The fastest way to an agent that can do things
  the app cannot, or cannot do things the app can — both are bugs.
- **Free-form agent-to-agent conversation** — still a loop with a bill; goals
  and delegation inside a run give the coordination without the loop.
