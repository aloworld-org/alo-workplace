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
$logDir = Join-Path $repo ".localdev\logs"

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
    foreach ($port in 5173, 8080) {
        $process = Get-Owner $port
        if ($null -eq $process) { Write-Host "[dev] $port stopped"; continue }
        Write-Host "[dev] $port PID $($process.ProcessId): $($process.CommandLine)"
    }
    $ready = Wait-Http "http://localhost:8080/health/ready" 5
    Write-Host "[dev] backend $($ready.status), revision $($ready.revision), schema $($ready.databaseSchema)"
}

Set-Location $repo
if ($Action -eq "Stop") {
    Stop-ProjectListener 5173
    Stop-ProjectListener 8080
    Write-Host "[dev] stopped alo frontend and backend; database left running and untouched."
    exit 0
}
if ($Action -eq "Check") { Show-Status; exit 0 }

Assert-GitRevision
Assert-Database
foreach ($port in 5173, 8080) {
    $owner = Get-Owner $port
    if ($null -ne $owner -and -not (Test-ProjectProcess $owner)) {
        throw "Port $port belongs to PID $($owner.ProcessId) ($($owner.CommandLine)); use the correct checkout or stop it explicitly."
    }
}

Stop-ProjectListener 5173
Stop-ProjectListener 8080
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$revision = git rev-parse HEAD
git diff --quiet --ignore-submodules HEAD --
if ($LASTEXITCODE -ne 0) { $revision = "$revision-dirty" }
$env:ALO_BUILD_REVISION = $revision
$env:SQLX_OFFLINE = "true"
cargo build -p alo-jmap --bin alo-jmap
if ($LASTEXITCODE -ne 0) { throw "alo-jmap build failed." }

$env:ALO_BLOB_DIR = if ($env:ALO_BLOB_DIR) { $env:ALO_BLOB_DIR } else { Join-Path $repo ".localdev\blobs" }
$env:ALO_IDENTITY_ISSUER = "http://localhost:5173"
$env:ALO_JMAP_ADDR = "127.0.0.1:8080"
$env:VITE_DEV_API = "http://localhost:8080"

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
