# ADR 0059 — alo OS exists, and what that means for this repository

**Status:** accepted — **revisits the device-management non-goal** of
`docs/alo-product-description.md` deliberately
**Date:** 2026-09-02
**Context:** the `alo-os` repository and its ADRs 0001–0005;
`platform/alo-ai/src/lib.rs` (`AiConfig`); `web/src/ds/tokens.css`;
[ADR 0002](0002-agpl-dual-license.md), [ADR 0019](0019-platform-and-product-repos.md),
[ADR 0025](0025-sso-token-introspection.md)

## The decision in one line

alo OS is a **separate product in its own repository**, whose decisions are
recorded there and not here; what this repository owes it is a **shared layer it
can legally and practically consume** — permissively licensed, versioned, and
with design tokens that a native Rust shell can read.

## Why this ADR is short

An earlier draft of 0059 settled alo OS's own architecture from inside this
repository: the shell, the agent daemon, the image, the engine. That was wrong
in method before it was wrong in content, and it went out of date within a day —
by the time it was reviewed it claimed the engine was rented and the shell was a
webview, and both had been decided the other way.

Decisions about alo OS belong where the people building alo OS will look for
them. They now live in `alo-os/docs/decisions/`:

| | |
|---|---|
| **0001** | The capability model — what an agent may reach, under whose authority |
| **0002** | The shell is native; the workspace is an application on it |
| **0003** | Being on the same network is not authority |
| **0004** | The organisation's machine, and what the person is told about it |
| **0005** | Applications are sandboxed, and ask for what they need |

This ADR records only what falls on **this** repository.

## What falls on this repository

**1. Three repositories, not one.** `alo-workplace` keeps the workspace,
`alo-os` the system, `alo-engine` the renderer when it starts. The monorepo
doctrine is narrowed rather than abandoned: **what shares a release and a licence
shares a repository; what needs its own community gets its own.** An engine
nobody can contribute to without cloning a business workspace would not get
contributors.

**2. The shared layer must be permissively licensed.** The intent framework, the
agent runtime and the design tokens are consumed by all three. [ADR
0002](0002-agpl-dual-license.md) keeps AGPL-3.0 for the workspace and it stays —
but AGPL crates consumed by a GPL-3.0 operating system make the operating
system's licence a consequence of a dependency graph rather than a decision. So
the shared crates are Apache-2.0 or MPL-2.0, and the boundary is drawn
deliberately rather than discovered by a lawyer.

**3. The shared crates are versioned, not copied.** They stay here at first and
are published with semver, pinned by consumers. They move to their own
repository when the workspace's needs and the system's needs actually diverge —
which is a trigger, not a schedule, so it happens as a decision rather than a
mess.

**4. Design tokens must leave CSS.** [`web/src/ds/tokens.css`](../../web/src/ds/tokens.css)
is the single source of visual truth and a native Rust shell cannot read a
stylesheet. The tokens become a language-neutral source generating both the CSS
this repository uses and the constants the shell uses. Without it the two drift
apart within months and nobody notices until they are side by side.

**5. Local inference needs no new code here.** `AiConfig.base_url` already
documents an OpenAI-compatible endpoint at `localhost:11434`, so pointing every
agent at a model running on the customer's own hardware is configuration against
code that shipped a year ago. What is missing is *management* — catalogue,
download, lifecycle — and that is `alo-os`'s work, not ours.

**6. Device management is no longer a blanket non-goal.** The product doc
excluded MDM. Fleet enrollment, policy and signed updates are now in scope **for
alo OS machines only** — never third-party devices, never a general MDM product.
Recorded here because a non-goal must be revised on purpose, never by drift.

## Consequences

- Cross-repository changes cost more: a new agent verb is two pull requests in
  two repositories, in order. The additive-contract rule in `CLAUDE.md` stops
  being aspirational and starts being enforced by reality.
- Something must test the three together — an image built against the current
  workspace, booted, signed into, one agent turn. Three green repositories can
  still produce a broken system, and on an operating system that is found by
  whoever installs it.
- alo OS inherits everything unfinished here, the migration suite above all.
  Building it neither pauses that work nor excuses it.

## Alternatives rejected

**Keep everything in this monorepo.** Rejected: different licences, different
release cadences, and an engine that needs outside contributors. The cost of the
split is real and is paid deliberately.

**Settle alo OS's architecture here, since this is where the team works.**
Rejected — it is what the first draft did. Decisions belong where the people
affected will look for them, and a decision recorded in the wrong repository is
one nobody reads and everybody re-litigates.
