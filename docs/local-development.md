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
- port 5173 or 8080 belongs to another checkout or application;
- the database schema is newer than the checked-out migrations;
- the built backend revision or OIDC issuer disagrees with this checkout.

It rebuilds `alo-jmap`, starts the backend on 8080 and Vite on 5173, waits for
`/health/ready`, checks OIDC discovery through the Vite proxy, and checks the
login page. Logs are written beneath ignored `.localdev/logs/`.

```powershell
powershell -ExecutionPolicy Bypass -File scripts\dev.ps1 -Action Check
powershell -ExecutionPolicy Bypass -File scripts\dev.ps1 -Action Stop
```

`Stop` terminates only listeners whose executable or command line belongs to
this checkout. PostgreSQL and all data remain running and untouched.

## Readiness

`GET /health/ready` returns the backend build revision and both the compiled
and applied migration versions. It returns `503` when those schema versions do
not match. The response contains no tenant, account, or credential data.

Vite uses strict port binding. An occupied 5173 is an error; it never silently
moves the frontend to another port.
