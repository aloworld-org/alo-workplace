# One safe door into the local alo stack.
[CmdletBinding()]
param(
    [ValidateSet("Start", "Check", "Stop")]
    [string]$Action = "Start",
    [string]$RepoPath = ""
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($RepoPath)) {
    $RepoPath = Split-Path -Parent $PSScriptRoot
}
$repo = (Resolve-Path -LiteralPath $RepoPath).Path
$backend = Join-Path $repo "target\debug\alo-jmap.exe"
$mailer = Join-Path $repo "target\debug\alo-smtp.exe"
$logDir = Join-Path $repo ".localdev\logs"

# Every port this stack owns. 2525 is the MX and 2526 the trusted internal
# submission listener that alo-jmap hands composed mail to; without the latter
# running, EmailSubmission/set has nowhere to send and the app can read mail
# but not send any.
$devPorts = 5173, 8080, 2525, 2526

function Get-Listener([int]$Port) {
    Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue |
        Select-Object -First 1
}

function Get-Owner([int]$Port) {
    $listener = Get-Listener $Port
    if ($null -eq $listener) { return $null }
    Get-CimInstance Win32_Process -Filter "ProcessId=$($listener.OwningProcess)"
}

function Test-ProjectProcess($Process) {
    if ($null -eq $Process) { return $false }
    $path = [string]$Process.ExecutablePath
    $command = [string]$Process.CommandLine
    return $path.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase) -or
        $command.IndexOf($repo, [StringComparison]::OrdinalIgnoreCase) -ge 0
}

function Stop-ProjectListener([int]$Port) {
    $process = Get-Owner $Port
    if ($null -eq $process) { return }
    if (-not (Test-ProjectProcess $process)) {
        throw "Port $Port belongs to PID $($process.ProcessId) ($($process.CommandLine)); refusing to stop a foreign process."
    }
    Stop-Process -Id $process.ProcessId -Force
}

function Wait-Http([string]$Uri, [int]$Seconds = 30) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    do {
        try { return Invoke-RestMethod -Uri $Uri -TimeoutSec 3 }
        catch { Start-Sleep -Milliseconds 500 }
    } while ((Get-Date) -lt $deadline)
    throw "$Uri did not become ready within $Seconds seconds."
}

# The mail server has no HTTP surface, so readiness is the listening socket.
function Wait-Port([int]$Port, [string]$What, [int]$Seconds = 60) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    do {
        if ($null -ne (Get-Listener $Port)) { return }
        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)
    throw "$What did not start listening on $Port within $Seconds seconds. See $logDir\smtp.err.log."
}

function Assert-Database {
    if ([string]::IsNullOrWhiteSpace($env:DATABASE_URL)) {
        throw "DATABASE_URL is required. Point it at the one local development database named 'alo'."
    }
    $database = ([Uri]$env:DATABASE_URL).AbsolutePath.Trim("/")
    if ($database -ne "alo") {
        throw "DATABASE_URL names '$database'; local development must use the preserved database named 'alo'."
    }
    docker version --format "{{.Server.Version}}" | Out-Null
    if ((docker inspect -f "{{.State.Running}}" alo-pg 2>$null) -ne "true") {
        docker start alo-pg | Out-Null
    }
    $sourceSchema = Get-ChildItem (Join-Path $repo "platform\alo-store\migrations") -Filter "*.sql" |
        ForEach-Object { if ($_.Name -match "^(\d+)_") { [int64]$Matches[1] } } |
        Measure-Object -Maximum |
        Select-Object -ExpandProperty Maximum
    $databaseSchema = [int64](docker exec alo-pg psql -U alo -d alo -Atc "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = TRUE;")
    if ($databaseSchema -gt $sourceSchema) {
        throw "Database schema $databaseSchema is newer than checkout schema $sourceSchema. Pull main and rebuild; the database was not changed."
    }
}

function Assert-GitRevision {
    git -C $repo fetch origin | Out-Null
    $branch = git -C $repo branch --show-current
    if ($branch -ne "main") { throw "Expected branch main, found '$branch'." }
    $behind = [int](git -C $repo rev-list --count HEAD..origin/main)
    if ($behind -ne 0) { throw "This checkout is $behind commit(s) behind origin/main. Rebase before starting." }
}

function Show-Status {
    foreach ($port in $devPorts) {
        $process = Get-Owner $port
        if ($null -eq $process) { Write-Host "[dev] $port stopped"; continue }
        Write-Host "[dev] $port PID $($process.ProcessId): $($process.CommandLine)"
    }
    $ready = Wait-Http "http://localhost:8080/health/ready" 5
    Write-Host "[dev] backend $($ready.status), revision $($ready.revision), schema $($ready.databaseSchema)"
}

Set-Location $repo
if ($Action -eq "Stop") {
    foreach ($port in $devPorts) { Stop-ProjectListener $port }
    Write-Host "[dev] stopped alo frontend, backend and mail server; database left running and untouched."
    exit 0
}
if ($Action -eq "Check") { Show-Status; exit 0 }

Assert-GitRevision
Assert-Database
foreach ($port in $devPorts) {
    $owner = Get-Owner $port
    if ($null -ne $owner -and -not (Test-ProjectProcess $owner)) {
        throw "Port $port belongs to PID $($owner.ProcessId) ($($owner.CommandLine)); use the correct checkout or stop it explicitly."
    }
}

foreach ($port in $devPorts) { Stop-ProjectListener $port }
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$revision = git rev-parse HEAD
git diff --quiet --ignore-submodules HEAD --
if ($LASTEXITCODE -ne 0) { $revision = "$revision-dirty" }
$env:ALO_BUILD_REVISION = $revision
$env:SQLX_OFFLINE = "true"
cargo build -p alo-jmap --bin alo-jmap -p alo-smtp --bin alo-smtp
if ($LASTEXITCODE -ne 0) { throw "build failed." }

# The blob store is the other half of the one local database, so it lives the
# same way: outside every checkout, shared by all of them.
#
# It used to sit in whichever checkout launched the server. With one 'alo'
# database and several checkouts, that meant a message row written from one
# tree and its bytes written into that tree's folder — so the message would not
# open from another tree, and cleaning a tree destroyed bodies whose rows lived
# on. What that looks like is a message list that loads and a message that will
# not open: a broken product, not a stranded file. A per-user path (never in a
# checkout, never inside OneDrive) makes the two halves of the store agree by
# construction.
#
# An explicit ALO_BLOB_DIR still wins, for a case that genuinely wants its own.
$env:ALO_BLOB_DIR = if ($env:ALO_BLOB_DIR) {
    $env:ALO_BLOB_DIR
} else {
    Join-Path $env:LOCALAPPDATA "alo\dev-blobs"
}
New-Item -ItemType Directory -Force -Path $env:ALO_BLOB_DIR | Out-Null
$env:ALO_IDENTITY_ISSUER = "http://localhost:5173"
$env:ALO_JMAP_ADDR = "127.0.0.1:8080"
$env:VITE_DEV_API = "http://localhost:8080"

# ---- the mail server -------------------------------------------------------
#
# Without this, the stack could read mail but not send any: EmailSubmission/set
# hands a composed message to a submission listener, and when there is none it
# refuses. Reading needs only Postgres and the blob store, which is why the gap
# stayed invisible until somebody pressed Send.
#
# NOTHING LEAVES THIS MACHINE — and the way that is arranged matters, because
# the obvious way does not work.
#
# Submission does not deliver: it spools, and the queue runner drains the spool.
# The queue runner only exists when outbound is enabled. So simply turning
# outbound off makes the stack look like it sends — Sent fills, no error — while
# every message sits in the spool forever, including one addressed to yourself.
# That is a worse lie than the honest refusal it replaces.
#
# So outbound is ON, and every message is routed to a smarthost that is our own
# MX on 127.0.0.1. A recipient at a local domain is delivered into the store,
# which is exactly what happens in production once MX lookup lands back on our
# own server. A recipient anywhere else is refused by that same MX, because a
# domain we do not host is precisely what its anti-open-relay guard rejects, and
# the sender gets a bounce. The loop is what keeps the box sealed: there is no
# route out of it that does not pass through a listener bound to 127.0.0.1.
$env:ALO_SMTP_ADDR = "127.0.0.1:2525"
$env:ALO_SMTP_INTERNAL_SUBMISSION_ADDR = "127.0.0.1:2526"
$env:ALO_SMTP_HOSTNAME = "localhost"
$env:ALO_SMTP_OUTBOUND_ENABLED = "true"
$env:ALO_SMTP_SMARTHOST = "127.0.0.1:2525"
$env:ALO_SMTP_ALLOW_SELF_SIGNED = "true"
# Deliver promptly: the production default paces a real queue, and waiting a
# minute to see your own test message is how you conclude sending is broken.
$env:ALO_SMTP_QUEUE_INTERVAL_SECS = "2"
# Which domains count as ours, and so get delivered rather than spooled. It is
# also the anti-open-relay guard, which is why it is never empty.
if (-not $env:ALO_SMTP_LOCAL_DOMAINS) { $env:ALO_SMTP_LOCAL_DOMAINS = "alomails.com" }
# The two services must read and write ONE blob store. Separate directories
# would put a delivered message's row in the shared database and its bytes
# somewhere the API cannot reach — a message that arrives and will not open.
$env:ALO_SMTP_BLOB_DIR = $env:ALO_BLOB_DIR
$env:ALO_SMTP_SPOOL_DIR = Join-Path (Split-Path -Parent $env:ALO_BLOB_DIR) "dev-spool"
New-Item -ItemType Directory -Force -Path $env:ALO_SMTP_SPOOL_DIR | Out-Null
# What alo-jmap hands composed mail to. Set before the backend starts, because
# it is read once at startup.
$env:ALO_JMAP_SUBMISSION_ADDR = $env:ALO_SMTP_INTERNAL_SUBMISSION_ADDR

Start-Process -FilePath $mailer -WorkingDirectory $repo -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $logDir "smtp.out.log") `
    -RedirectStandardError (Join-Path $logDir "smtp.err.log")
Wait-Port 2526 "alo-smtp internal submission" 60

Start-Process -FilePath $backend -WorkingDirectory $repo -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $logDir "backend.out.log") `
    -RedirectStandardError (Join-Path $logDir "backend.err.log")
Start-Process -FilePath "npm.cmd" -ArgumentList "run", "dev", "--", "--host", "127.0.0.1", "--port", "5173" `
    -WorkingDirectory (Join-Path $repo "web") -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $logDir "frontend.out.log") `
    -RedirectStandardError (Join-Path $logDir "frontend.err.log")

$ready = Wait-Http "http://localhost:8080/health/ready" 60
if ($ready.status -ne "ready" -or $ready.revision -ne $revision) {
    throw "Backend readiness disagrees with this checkout: expected $revision, received $($ready.revision)."
}
$discovery = Wait-Http "http://localhost:5173/.well-known/openid-configuration" 60
if ($discovery.issuer -ne "http://localhost:5173") {
    throw "OIDC issuer mismatch: $($discovery.issuer)"
}
$frontend = Invoke-WebRequest -Uri "http://localhost:5173/login" -UseBasicParsing
if ($frontend.StatusCode -ne 200) { throw "Frontend login page returned $($frontend.StatusCode)." }
Show-Status
