# Local development stack

Use one checkout, one database, and one launcher. The canonical Windows
checkout is outside file-sync folders at `C:\dev\Ficina`; the development
database is named `alo`. The launcher never creates, replaces, or deletes that
database.

## Start

Set `DATABASE_URL` in your shell or an ignored local environment file, then:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\dev.ps1
```

The launcher refuses to start when:

- this is not `main`, or `main` is behind `origin/main`;
- `DATABASE_URL` names anything other than `alo`;
- port 5173, 8080, 2525 or 2526 belongs to another checkout or application;
- the database schema is newer than the checked-out migrations;
- the built backend revision or OIDC issuer disagrees with this checkout.

It rebuilds `alo-jmap` and `alo-smtp`, starts the mail server on 2525/2526, the
backend on 8080 and Vite on 5173, waits for `/health/ready`, checks OIDC
discovery through the Vite proxy, and checks the login page. Logs are written
beneath ignored `.localdev/logs/`.

```powershell
powershell -ExecutionPolicy Bypass -File scripts\dev.ps1 -Action Check
powershell -ExecutionPolicy Bypass -File scripts\dev.ps1 -Action Stop
```

`Stop` terminates only listeners whose executable or command line belongs to
this checkout. PostgreSQL and all data remain running and untouched.

## Mail actually sends

`EmailSubmission/set` does not deliver anything itself: it hands the message to
alo-smtp's trusted internal submission listener, and refuses when there is
none. So a stack without a mail server can read mail perfectly and cannot send
at all — reading needs only PostgreSQL and the blob store.

The launcher therefore runs `alo-smtp` too: the MX on 2525, the internal
submission listener on 2526, and `ALO_JMAP_SUBMISSION_ADDR` pointing the
backend at the latter.

**Nothing leaves the machine**, and the arrangement that guarantees it is worth
understanding before changing it. Submission spools; the queue runner drains
the spool; and the queue runner only exists when outbound delivery is enabled.
Turning outbound *off* therefore does not give a safe sandbox — it gives a
stack that appears to send (the Sent folder fills, no error appears) while
every message, including one addressed to yourself, sits in the spool forever.

So outbound is on, and `ALO_SMTP_SMARTHOST` routes every message to our own MX
on `127.0.0.1:2525`:

- a recipient at a local domain (`ALO_SMTP_LOCAL_DOMAINS`, default
  `alomails.com`) is delivered into the store, exactly as it is in production
  once MX lookup resolves back to our own server;
- a recipient anywhere else is refused by that same MX — a domain we do not
  host is what its anti-open-relay guard exists to reject — and the sender
  receives a bounce.

Every listener in the loop is bound to `127.0.0.1`, so there is no route out.
Set `ALO_SMTP_LOCAL_DOMAINS` to test with a different hosted domain.

The blob store is shared with the backend (`ALO_SMTP_BLOB_DIR` is set to
`ALO_BLOB_DIR`). They must never diverge: separate directories put a delivered
message's row in the shared database and its bytes somewhere the API cannot
read, which presents as a message that arrives and will not open.

## Readiness

`GET /health/ready` returns the backend build revision and both the compiled
and applied migration versions. It returns `503` when those schema versions do
not match. The response contains no tenant, account, or credential data.

Vite uses strict port binding. An occupied 5173 is an error; it never silently
moves the frontend to another port.
