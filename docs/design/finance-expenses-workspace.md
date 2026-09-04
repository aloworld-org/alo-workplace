# Finance expenses workspace

## Surface

The Expenses view presents one grouped date range, a status filter, and the
existing claims table or empty state. When the list is empty, its only New
claim action sits directly below the onboarding explanation; once claims
exist, the action moves to the page header. It uses the shared branded calendar
and dropdown card and retains all existing expense dialog actions.

## Errors

Loading and mutation failures continue through the existing Finance error
banner. Applying a date range or status refreshes the server-backed list
immediately.

## Tenancy

The view never accepts a user identifier. Reads and writes continue through the
existing tenant-scoped Finance API, and the server remains the authority for
which claims are editable.

## Out of scope

This change does not alter Finance navigation, expense APIs, approval rules, or
accounting behavior.

The rejected alternative was adding more decorative cards around every control;
that would increase visual noise without improving the task hierarchy.
