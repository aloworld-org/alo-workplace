# alo Campaigns — build journal

One entry per completed queue item: what was built, what the tenancy and
consent tests proved, and — for anything touching who may be mailed — **the
test that proves who may not be**, quoted rather than summarised.

Started 2026-08-17 against a build with no campaign code at all. This is a new
module, and the people it will reach already exist in three tenant-wide places:
`billing_customers`, `crm_deals.contact_email` and
`site_form_submissions.sender_email`.

**The rule this journal exists to hold.** Every other queue in this repository
records what was built. This one also records **who was excluded and why**,
because a campaign module's failures are not crashes — they are messages
arriving at people who did not agree to them, or who asked to stop, and those
land in somebody's inbox rather than in a log.

Three things are therefore worth more than a green suite here:

- **`contacts` is never a source.** It is a per-user address book. A company
  campaign drawn from it mails somebody's private contacts, and no test suite
  catches that as a bug — it looks like a feature working.
- **Suppression is enforced in SQL, not by the caller.** A rule the sender has
  to remember is not absolute, and the first import that forgets it makes the
  promise false for good.
- **Consent is provenance, not a boolean.** "Did they agree" and "how do we
  know" are different questions, and only the second survives a complaint.

**Nothing here sends.** The sending identity waits on a second IP, which is a
purchase; read-duration tracking waits on an ADR that decides whether a number
that unreliable belongs in a product sold on not tracking people. A loop
supplies neither, and an item that finds itself needing one has found the edge
of this queue rather than a problem to solve.
