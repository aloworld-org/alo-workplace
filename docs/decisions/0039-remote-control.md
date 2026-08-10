# ADR 0039 — Remote control: taking over a colleague's screen

**Status:** proposed
**Date:** 2026-08-10
**Supersedes:** nothing. **Amends:** nothing.

## Context

alo Meet shares screens. Sharing is not controlling: a support engineer
watching a colleague's screen still has to say "click the blue button, no, the
other one". Remote control — moving that person's pointer and typing on their
keyboard — is what turns a support call into a fix, and it is the single
feature most often asked for by the IT departments alo is sold to.

It is also, by a wide margin, the most dangerous thing this product would
contain. Every other alo capability is bounded by a tenant and a permission
check. Remote control is bounded by nothing: a session that can move a mouse
can open a password manager, read mail, sign a payment, and disable the audit
that would have recorded it. The threat model is not "somebody sees data they
should not"; it is "somebody becomes an employee".

Two facts shape everything below:

- **A browser cannot do this.** Injecting input into an operating system needs
  a native process with accessibility or input-injection privileges. alo has
  exactly one such surface: the Tauri desktop app (ADR 0005).
- **The hard part is not the pixels.** LiveKit already carries a screen and has
  a data channel that can carry input events. The engineering is small; the
  consent, revocation and audit design is the whole job.

## Decision

Remote control is built, on these terms. Every one of them is a refusal that
somebody will later ask to relax, and each is recorded here so that
conversation starts from a written reason.

### 1. Only the desktop app can be controlled, and only when installed deliberately

Control requires the Tauri app with the input-injection capability compiled in
and the OS permission granted by the person at the keyboard. There is no
browser path and no silent install. A workspace that never installs the desktop
app cannot be remotely controlled by alo, and that must remain true.

### 2. The controlled person grants, and can always take it back

- Control is **requested**, never assumed. The request names who is asking.
- The grant is an explicit act by the person at the machine, in a native dialog
  the web page cannot draw or dismiss.
- **Any input from the local keyboard or mouse immediately suspends control.**
  Not a hotkey to remember — moving your own mouse is the escape. Somebody
  panicking must not have to recall a key combination.
- A hard stop (`Esc` held, or the tray item) **ends** the session outright.
- Control ends when the meeting ends, when the app loses focus of the shared
  surface, and after an idle period. It never persists across a restart.

### 3. It is a session, visible while it happens

While control is active the controlled machine shows a persistent, always-on-
top indicator naming the controller. It cannot be hidden, minimised or made
transparent by either party. A control session that can be concealed is a
backdoor with a product name.

### 4. Nobody is granted control by policy

An administrator cannot pre-authorise control of an employee's machine, and
there is no "unattended access" mode. This is the largest single difference
from AnyDesk and TeamViewer, and it is deliberate: unattended access is what
turns a support tool into surveillance infrastructure, and a European
workplace product that ships it is selling the thing it claims to oppose. If a
customer needs unattended device management, that is MDM, and alo refers MDM
to partners (product doctrine, non-goals).

### 5. Every session is audited, and the audit is not optional

Start, end, who asked, who granted, which machine, and every suspension. The
audit is written by the server, not the client, and a session whose audit write
fails does not start. Audit exists to answer "who was in my machine on the
14th?" and it is worthless if it can be turned off by whoever is inside.

### 6. Input flows over the meeting's own data channel

No second transport, no direct peer connection, no port to open. LiveKit's data
channel already exists between exactly the two parties and is already encrypted
in transit. A separate channel would be a separate thing to secure, and the
sovereignty argument for the engine covers this too: input events travel the
same EU-hosted path as the media.

### 7. What is deliberately absent in v1

File transfer, clipboard sync, unattended access, controlling a machine that is
not in a meeting, and controlling more than one machine at once. Each is a
reasonable request and each widens the blast radius; none is needed to fix a
colleague's spreadsheet.

## Consequences

- The desktop app becomes security-critical in a way it was not. Signing,
  update integrity and the input capability's scope now matter as much as
  anything in the server. The unsigned Windows build noted in the desktop
  memory must be resolved before this ships.
- Support staff will ask for unattended access within a month of launch. The
  answer is in §4 and requires a new ADR to change, not a settings toggle.
- The indicator in §3 will be described as intrusive. That is what it is for.
- Penetration testing is a precondition, not a follow-up. This is the first alo
  feature where a defect is a compromise of the customer's whole estate rather
  than of their alo data.

## Alternatives considered

**Browser-only "guided" control** — the controller draws on the shared screen
and the other person clicks. Safe, and genuinely useful, but it does not do the
thing being asked for. Worth building anyway, and it does not need this ADR.

**Integrating an existing remote-control engine** (RustDesk is Rust, AGPL, and
self-hostable). Tempting under the build-vs-integrate rule, and rejected for
now because the consent and audit model above is the actual product, and it
would have to be enforced around an engine designed for unattended access —
the wrong default to be fighting. Revisit if the input layer proves harder than
expected; the seam is the data channel either way.

**Not building it.** The honest option, and the one this ADR would have
recommended on security grounds alone. Overruled deliberately: it is a stated
requirement, the demand is real, and a support tool that cannot fix anything
loses the customer to one that can. The terms above are what make it
defensible.
