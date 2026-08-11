# Meet UX audit — 2026-08-11

Scope: the Meet destination in loading, failure, empty, and active states; the
pre-join device check; joining, unavailable, failed, and connected room states.
The reference interaction model is Google Meet and Zoom, interpreted through
`docs/design/ux-principles.md` and alo's shared tokens and controls.

## Findings and resolution

| Screen | Finding | UX law | Resolution |
|---|---|---|---|
| Destination loading | The page was blank while meetings loaded. | Immediate, honest feedback | Added a reduced-motion-aware skeleton. |
| Destination failure | A failed list request was presented as a successful empty list. | Errors speak human and help | Added an honest error state with a visible retry action; starting a meeting remains available. |
| Start meeting | A failed start produced no feedback. | Immediate feedback; errors help | Added an inline actionable error and immediate busy label. |
| Empty destination | The explanation and primary action were visible and clear. | Empty states onboard; primary action | Retained, while bringing its size and geometry onto tokens. |
| Active meeting list | The stretched utility rows had weak hierarchy, wasted wide screens, and made every meeting look identical. Ad-hoc typography, icon dimensions, and border values also bypassed the design scale. | Calm surfaces, one voice; recognition over recall | Replaced the list with a responsive live-meeting gallery: status, title, start time, and a full-width one-click Join action form one scannable card. All geometry uses semantic design tokens. |
| Destination first impression | The primary Start action floated in the page header with no meaningful composition or reason to prefer this screen over a generic call list. | Primary action; aesthetic usability; end-to-end flow | Added a dark alo hero that makes starting the unmistakable first action, explains the privacy-respecting device defaults, and visually separates creation from meetings already happening. |
| Pre-join | There was no visible way back. The hidden required LiveKit username also left Join permanently disabled. | Zero-manual; recognition over recall | Added a persistent Back to Meet control and supplied a non-visible internal value; alo's signed token remains the identity authority. |
| Pre-join | LiveKit's blue join action did not use alo's single accent. | Calm surfaces, one voice | The primary action now uses alo accent and focus tokens. |
| Joining | Plain text gave weak progress feedback. | Immediate, honest feedback | Added a labelled, reduced-motion-aware joining state. |
| Join failure | The only action was Close and the error offered no recovery. | Errors speak human and help | Added Try again beside Close. The deployment-configuration state remains close-only because retry cannot configure a server. |
| Connected room | Leave was below the 40px target, lacked explicit focus treatment, duplicated LiveKit's own Leave, and used hardcoded colors/geometry. | Fitts's law; keyboard reach; token scale; one voice | Rebuilt it with the shared danger button, 40px target and focus ring; removed the duplicate engine exit. |

## Journey and decision record

Starting an instant meeting remains one click from the Meet destination, versus
Google Meet's common two-step new-meeting menu. Pre-join adds no ceremony: it is
the expected device-safety check, and Join is one visible action. Back and Leave
remain visible without opening any menu.

Rejected alternative: replacing LiveKit's pre-join and conference controls with
custom media UI. That would duplicate engine behavior and contradict ADR 0003;
alo instead owns the framing, recovery paths, visual tokens, and guaranteed exit.

No API, persistence, tenancy, or admission contract changed in this audit.
