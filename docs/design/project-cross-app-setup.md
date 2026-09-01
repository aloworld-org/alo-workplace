# Project cross-app setup

## Purpose

A won Sales opportunity creates only its linked delivery project. Alo does not
silently create supporting records. In the Project workspace, **Set up project**
opens a review dialog where the user explicitly selects the resources the team
needs: a shared Drive Space, a tenant-visible Chat room, starter tasks, and an
optional one-hour kickoff meeting.

Opening or cancelling the dialog writes nothing. Confirmation creates only the
selected resources and returns their canonical IDs. The Project workspace then
links directly to those records, so the project remains the operational hub
without copying or guessing data between apps.

## API and storage contract

- `GET /projects/{id}/setup` returns the setup record or `null`.
- `POST /projects/{id}/setup` accepts `createFilesSpace`, `createChatRoom`, an
  optional RFC 3339 `kickoff`, and up to 20 `starterTasks`.
- An empty confirmation is rejected with `422` and creates no setup record.
- `project_setup` stores the canonical Drive, Chat, Agenda, and Task IDs against
  the project. A repeated identical request is a no-op: it neither duplicates
  resources nor changes `updatedAt`.
- A tenant/project-scoped transaction lock serializes simultaneous confirmations,
  so a double click or duplicated network request cannot create parallel resources.
- Starter tasks carry `source_kind = project_setup` and the project ID, allowing
  an interrupted attempt to recover their IDs on retry.

## Trust and visibility

The Project visibility check is the access door for both reads and writes. A
project outside the authenticated account's tenant or personal scope returns
`404`, disclosing neither the project nor its setup. Team projects currently
have tenant-wide visibility, so their generated Chat room uses the same scope.
The kickoff is created in the confirming user's personal calendar; Alo does not
invent attendees while Projects has no explicit membership model.

The confirmed mutation resolves to the `projects.project.setup` audit action.
Real-database and real-router tests cover tenancy, empty confirmation, creation,
read-back, and retry behavior.
