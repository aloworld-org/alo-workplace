# alo OS

**The sovereign AI workstation. Your models, your hardware, your building.**

*Status: **proposal, not a settled decision.** This document is the written case
the Non-goals section of `alo-product-description.md` requires ("revisited only
with revenue on the table and a written case"). Nothing here is built or
scheduled until ADR 0059 is accepted and `features.md` carries the rows with a
tier. Written 2026-08-30.*

---

## 1. What alo OS is

alo OS is an operating system whose interface is an agent and whose first-class
workload is a language model you own.

It boots into alo workplace — mail, files, chat, meetings, documents and the
sixteen product agents — as the desktop itself rather than as an application
inside somebody else's desktop. The agents reach the whole machine: the real
filesystem, real applications, the window in front of you, the printer nobody
can configure. And the model those agents run on can live on the same machine,
so that for the organisations who need it, **nothing leaves the building at
all**.

It is not a Linux distribution with our web app on it. The distinguishing claim
is narrow and testable: *an action a person would take by hand can be proposed
by an agent, approved in one click, and afterwards explained — and the model
that proposed it ran on hardware the customer owns.*

## 2. Why it exists

Three facts arrived at the same time.

**Windows 10 support ended in October 2025.** A large share of European
business and public-sector machines cannot run Windows 11 because of its
TPM and CPU requirements. Those organisations are choosing right now between
new hardware, paid extended support, and leaving. That is a migration window,
and it closes.

**European public bodies are actively moving off Microsoft.**
Schleswig-Holstein, Denmark, and the French state's own suite are not
experiments any more. Procurement has language for sovereignty now, and budget
behind it.

**AI arrived where the data cannot go.** Hospitals, law firms, notaries,
defence suppliers, municipalities — organisations under GDPR or sector rules
that forbid sending records to a US inference provider. Their current options
are to do without, or to send it anyway and hope the contract holds. Neither is
a decision anybody wants to defend.

alo workplace already answers the first two. alo OS answers the third, and it
is the only one of the three where the answer must reach below the browser.

## 3. Who it is for

Two profiles of the same system, chosen at install:

**alo OS Desktop** — the machine an ordinary person works on. Runs on hardware
the customer already owns, including the Windows 10 fleet that Microsoft
stopped supporting. Agents work here, backed by inference on an alo server
(hosted, or the customer's own appliance). This is the volume product.

**alo OS AI** — the workstation with a GPU. Same system, same shell, but
inference and fine-tuning run locally: models are downloaded, served and
adapted on the customer's own hardware, and the agents point at them by
default. This is the product with the sovereignty story that cannot be bought
elsewhere, and it is the one to build first, because it is the one people
cannot get anywhere else.

The two are one codebase with a hardware profile, not two products.

## 4. Principles

Inherited from `CLAUDE.md` and not renegotiated here:

- **The tenant is sacred.** Every read and write is tenant-scoped. A shared
  workstation does not become a way around isolation.
- **Engines are configured, never patched.** We do not write a kernel, a
  graphics stack, a browser engine, or an inference kernel. We pin them,
  configure them, and own the layer above.
- **Rust below the waterline, TypeScript above.** An OS does not change this —
  see §6, and see §9 for the one honest tension it creates.
- **Done means the full path works.** An OS that boots but cannot print is not
  a released OS.

And one added by this product:

- **Nothing leaves silently.** Every network egress caused by an agent is
  visible at the moment it happens, and afterwards in a record. On a
  sovereignty product this indicator is a feature, not a diagnostic.

## 5. What a person actually sees

**Boot.** Firmware to the alo login in seconds, no distribution branding, no
desktop environment underneath to fall back into.

**Sign in** with an alo identity. The workspace is the desktop: modules where
another system would put applications, Drive where it would put a filesystem,
the agents' proposals where it would put notifications.

**One key, anywhere.** A single hotkey opens the agent from inside any
application, with the current context offered — the document in focus, the
selection, the window — and never harvested in the background.

**Propose, approve, explain.** The agent does not act. It proposes; a person
approves once; the action runs; the result is a record carrying its origin
(ADR 0023, ADR 0058). This is the same machinery the web product already uses,
which is why it can ship early rather than being rebuilt.

**The documents people are actually sent** open: `.docx`, `.xlsx`, `.pptx`
through Collabora, already in the stack.

**Printing works**, which is unglamorous and decides public-sector deals.

## 6. Architecture

| Layer | Ours? | Language | Notes |
|---|---|---|---|
| Kernel | rented | — | Linux, unmodified. Hardware support is why OS projects die; we do not fight that battle. |
| Drivers, graphics | rented | — | Mesa, libinput, NVIDIA/AMD stacks. We link C; we never write it. |
| Init, system & policy services | **ours** | **Rust** | Where sovereignty is enforced in code: egress policy, tenant scoping, disk encryption, update integrity. |
| Compositor / display | **ours** | **Rust** | Wayland via Smithay. Owning the display server without C is the fact that makes this plan viable. |
| `alo-agentd` — the OS agent runtime | **ours** | **Rust** | §7. The one genuinely new component. |
| Model runtime & serving | rented | — | llama.cpp / vLLM / Ollama — pinned, wrapped by our API, never forked. |
| Fine-tuning stack | rented | (Python) | PyTorch, `transformers`, `peft`. Shipped as a pinned engine — see §9. |
| Shell — what a person sees | **ours** | **TypeScript** | alo workplace itself, full-screen. A year of built product, reused rather than rewritten. |
| Web rendering for the shell | rented | — | An embedded engine today; Servo when it is ready. The swap touches one crate because rendering was never where our value lived. |

The shape to notice: **we own the experience and the policy, and rent every
commodity.** That is the same doctrine that governs Synapse, LiveKit, Collabora
and Garage in alo workplace, applied one layer down.

## 7. `alo-agentd` — the agent's reach into the machine

The agents exist. What does not exist is their reach. `alo-agentd` is a Rust
system service that exposes OS-level verbs to the intent layer already built:

| Verb class | What it means |
|---|---|
| **Files** | The real filesystem, not only Drive: file, rename, sort, find, archive. |
| **Applications** | Open, arrange, close — "put the tender next to the spreadsheet". |
| **Context** | The focused window, the selection — *offered on invocation*, never watched. |
| **System** | Printers, network, updates, storage — the administration nobody knows how to do. |
| **Invocation** | One hotkey, anywhere, in any application. |

Every verb is an intent: proposed, approved once, executed, recorded with an
origin. No new approval model, no second audit trail.

**This is also the largest attack surface in the product**, and it is being
added to a system sold on security. Its permission model — what an agent may
reach, under whose authority, with what sandboxing, and what a compromised
model can and cannot cause — is designed in ADR 0059 **before** the service is
written. A capability model retrofitted onto a running daemon is how this
product would fail publicly.

## 8. The AI stack, and why an OS is the right layer for it

Running models locally is not hard because of models. It is hard because of
environment: driver against CUDA toolkit against PyTorch against kernel, ROCm
on AMD, virtualenvs that rot, an upgrade that silently breaks a training run.
Teams lose days to this, repeatedly, and every team rebuilds the same fragile
stack.

That is precisely a problem an operating system solves, because the fix *is*
the base image: drivers, runtime and serving pinned together, versioned,
atomic, with rollback. NVIDIA ships DGX OS and Lambda ships Lambda Stack for
this reason. Nobody ships the European, sovereign, agent-integrated one.

What "easy" has to mean, concretely, or the claim is marketing:

- **The GPU works on first boot.** No driver installation, no CUDA archaeology.
- **A model runs in one command**, from a curated catalogue of open-weight
  models with their licences stated.
- **A fine-tune on your own documents is a guided flow** — LoRA/QLoRA over a
  folder or a tenant's records, with the dataset never leaving the machine.
- **The agents point at the local model by default**, so sovereignty is the
  default configuration rather than an option someone must find.
- **An upgrade cannot break a working stack**: atomic images, rollback, and the
  model runtime versioned with the drivers it needs.

## 9. The Python question, answered explicitly

Fine-tuning is PyTorch, `transformers` and `peft`. That ecosystem is Python and
will not be replaced by anything we write.

The constitution says a third language in our repos is a bug. It remains a bug.
The resolution is the doctrine already in force for Collabora and Synapse:
**Python enters as a pinned engine — shipped, configured, never patched, never
written in.** Our code stays Rust and TypeScript; the fine-tuning stack is a
dependency with a version number, exactly like a container image.

Written down here because, left unexamined, this is the decision by which "two
languages only" quietly becomes four.

## 10. Sovereignty, stated as a testable claim

- On **alo OS AI** with a local model, a working day produces **no inference
  egress**. Testable at the network boundary, and we should publish the test.
- On **alo OS Desktop**, inference goes to an alo server the customer chose —
  our EU cloud or their own appliance — and the egress indicator says so at the
  moment it happens.
- **Fine-tuning data never leaves.** The dataset, the adapter and the resulting
  weights stay on the customer's disk.
- **No telemetry.** Not "anonymised telemetry". None, with the egress policy in
  a Rust service rather than a settings checkbox.

## 11. What alo OS does not do

- **No kernel.** Linux, unmodified.
- **No browser engine.** Rented, swappable, and a separate decision (see the
  browser ADR, if it is written).
- **No inference kernels.** We do not compete with llama.cpp or vLLM.
- **No model training from scratch.** We serve and adapt open weights.
- **No general-purpose distribution.** No package manager for the world, no
  attempt to be Ubuntu. One system that does one job.
- **No phone or tablet.** Not in v1, possibly never.

**One existing non-goal must be revisited, not ignored:**
`alo-product-description.md` §14 excludes **device management (MDM)**. An OS
sold to a fifty-seat firm cannot require each machine to be configured by hand.
Fleet enrollment, policy and updates are therefore in scope for v1, and ADR
0059 must record that revision explicitly rather than letting it happen by
drift.

## 12. Hardware

**Certified first, compatible later.** One machine model, bought twice, works
completely — that is v0.01. A compatibility list grows from there; "supports
PCs" is not a claim anyone can honour.

- **alo OS AI:** one GPU workstation configuration, 24 GB VRAM or more.
- **alo OS Desktop:** one recent business-class model, then the Windows 10
  fleet by generation.

The temptation to support everything at once is how this project would spend a
year and ship nothing.

## 13. Release shape

**v0.01 — it boots and the agent acts.** One hardware target. Boots into alo,
alo identity login, `alo-agentd` with the file and application verbs, hotkey
invocation, a local model serving the agents. No fleet management, no
installer, no compatibility list. The point is to prove the sentence in §1 on
real hardware.

**v0.5 — a person can work on it all day.** Collabora, printing, browser,
suspend/resume, disk encryption, atomic updates with rollback, the guided
fine-tune.

**v1 — an organisation can buy it.** Fleet enrollment and policy, signed
updates, backup/restore, the security audit, the documented egress guarantees,
support and SLA definitions.

Each stage is releasable to someone. None of them is a rewrite of the one
before.

## 14. Risks, stated honestly

- **Hardware support is where OS projects die.** Mitigated only by refusing to
  support arbitrary hardware. If the certified-model discipline slips, this
  risk is unmitigated.
- **`alo-agentd` is privileged code on a security product.** A capability
  model designed late is the most likely route to a public failure.
- **It inherits everything unfinished in alo workplace** — the migration suite
  above all (`ROADMAP.md` Phase 4: 1 done, 9 open). A municipality adopting alo
  OS is also migrating its mail, calendar and files.
- **Scope.** `alo-product-description.md` §14 opens with "scope creep killed
  most of our predecessors". alo workplace has sixteen modules, sixteen agents
  and no paying customer with no prior relationship to the founder. An OS does
  not close that gap, and building it before the gap closes is the single
  largest risk in this document.
- **GPU supply and cost** for the AI SKU, which is a procurement problem before
  it is an engineering one.

## 15. Open decisions

These belong in ADR 0059 and are listed here so none is settled by accident:

1. **Which SKU is v1** — AI workstation, or Desktop.
2. **The `alo-agentd` capability and sandboxing model.**
3. **MDM**: the non-goal revision, and how far fleet management goes.
4. **Model serving**: which runtime is pinned, and the model catalogue's
   licence policy.
5. **Where the shell renders** — embedded engine now, Servo later, or a native
   Rust shell and no web engine at all.
6. **Image build tooling** — and which language it is allowed to introduce.
7. **Update and signing infrastructure**, which is a security boundary.
8. **Licence**: how AGPL-3.0 core plus commercial (ADR 0002) applies to an OS
   image containing rented engines.

## 16. Positioning in one line

**The operating system where the assistant does the work, on a model you own,
in a building you control.**
