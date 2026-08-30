# run-loop.ps1 — the Business-track build loop engine (docs/autonomy/LOOP.md).
#
# Runs Claude Code headless, one queue item per invocation, forever — until
# QUEUE.md is complete (STATE.md gains "LOOP COMPLETE") or the loop halts
# ("LOOP HALT"). Safe to stop any time with Ctrl+C: every finished item was
# already committed and pushed by the iteration that built it.
#
# Usage (PowerShell, on the build PC):
#   powershell -ExecutionPolicy Bypass -File scripts\run-loop.ps1 -RepoPath "C:\dev\Ficina"
param(
  [string]$RepoPath = "C:\dev\Ficina",
  [string]$Track = "business",       # business | sites | ds (LOOP.md Tracks table)
  [int]$MaxIterations = 500,         # hard backstop against runaway loops
  [int]$IdleKillMin = 20,            # kill a worker SILENT this long (true hang)
  [int]$IterationCeilingMin = 240    # absolute per-iteration backstop
)
$ErrorActionPreference = "Continue"
Set-Location $RepoPath

# Every track but the default keeps its journal in a folder of its own. Naming
# them one by one meant a new track read the business journal, found its
# "LOOP COMPLETE" and stopped on iteration one reporting success.
$StateFile = if ($Track -eq "business") { "docs/autonomy/STATE.md" } else { "docs/autonomy/$Track/STATE.md" }
if (-not (Test-Path $StateFile)) {
  Write-Host "[loop] no journal at $StateFile - is '$Track' a track in docs/autonomy/LOOP.md?"; exit 2
}
$prompt = "Read docs/autonomy/LOOP.md and execute exactly ONE iteration of the loop for track '$Track', then exit."

# Resolve the claude CLI: PATH first, else the newest VSCode-extension binary.
$claude = (Get-Command claude -ErrorAction SilentlyContinue).Source
if (-not $claude) {
  $claude = Get-ChildItem "$env:USERPROFILE\.vscode\extensions\anthropic.claude-code-*\resources\native-binary\claude.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $claude) { Write-Host "[loop] claude CLI not found - npm install -g @anthropic-ai/claude-code"; exit 1 }
Write-Host "[loop] using claude at $claude"

for ($i = 1; $i -le $MaxIterations; $i++) {
  Write-Host ("=" * 60)
  Write-Host "[loop] iteration $i  $(Get-Date -Format 'yyyy-MM-dd HH:mm')"

  git pull --rebase origin main 2>&1 | Out-Null

  $state = Get-Content $StateFile -Raw -ErrorAction SilentlyContinue
  # Case-sensitive (-cmatch): -match is not, and on 2026-08-28 the ds loop
  # stopped on the prose "LOOP halts on a broken environment" with four items
  # open. Anchored to the start of a line ((?m) makes ^ mean that), because the
  # journal is prose and quotes its own markers -- an unanchored match found
  # "- **Next:** `LOOP COMPLETE`" mid-file and stopped with 58 items open.
  #
  # Bold is the third spelling, and run-loop.sh learned it on 2026-08-29 after
  # sixteen no-op iterations over a finished agents-web queue whose marker read
  # "**LOOP COMPLETE** - every item ... is [x]". This wrapper did not learn it
  # with its twin, so the same journal would have spun here; the two patterns
  # are now the same pattern. Prose is still excluded by the anchor -- the
  # journal's own references sit behind a bullet or a backtick.
  if ($state -cmatch "(?m)^#{0,6} *\*{0,2}LOOP COMPLETE") { Write-Host "[loop] queue complete - stopping."; break }
  if ($state -cmatch "(?m)^#{0,6} *\*{0,2}LOOP HALT")     { Write-Host "[loop] halted by the agent - fix the reason in STATE.md, remove the marker, restart."; break }

  # One iteration, with an IDLE-based hang guard: a truly hung worker goes
  # silent (its session transcript stops growing), while an honest long item
  # keeps writing constantly — a duration-only guard once executed 90 minutes
  # of honest work. So: kill after $IdleKillMin of transcript silence, with
  # $IterationCeilingMin as the absolute backstop. The killed item is simply
  # redone next iteration. --dangerously-skip-permissions is required for
  # unattended runs; the hard safety rails live in LOOP.md + repo deny rules.
  if ($claude -like "*.ps1") {
    $file = "powershell"
    $cliArgs = @("-ExecutionPolicy","Bypass","-File",$claude,"-p","`"$prompt`"","--dangerously-skip-permissions")
  } else {
    $file = $claude
    $cliArgs = @("-p","`"$prompt`"","--dangerously-skip-permissions")
  }
  # Transcript dir for this repo (path with [:\ ] as '-'), where activity shows.
  #
  # Resolve-Path first, because the key is derived from the path AS TYPED. A
  # -RepoPath given with forward slashes yields "C-/dev/repo"; no such folder
  # exists, $newest below stays null, and $idleMin silently becomes total
  # elapsed time — so the idle kill turns into a hard timer that stops honest
  # work at exactly $IdleKillMin minutes however busy it is. Two iterations
  # died that way before anyone noticed the timer was measuring the wrong
  # thing, and the log said "silent for 45 min" about a worker that had been
  # writing files the whole time.
  $projKey = ((Resolve-Path $RepoPath).Path -replace "[:\\ ]", "-")
  $transcripts = Join-Path $env:USERPROFILE ".claude\projects\$projKey"
  if (-not (Test-Path $transcripts)) {
    # Refuse rather than degrade. A missing transcript folder means the idle
    # detector cannot see anything, and a watchdog that cannot observe its
    # subject must not be trusted to kill it.
    Write-Host "[loop] no transcript folder at $transcripts - the idle detector would be blind, so this would become a $IdleKillMin-min hard timer. Check -RepoPath."
    exit 2
  }
  $started = Get-Date

  $proc = Start-Process -FilePath $file -ArgumentList $cliArgs -NoNewWindow -PassThru
  $killed = $false
  while (-not $proc.WaitForExit(30 * 1000)) {
    $now = Get-Date
    $newest = Get-ChildItem "$transcripts\*.jsonl" -ErrorAction SilentlyContinue |
      Where-Object { $_.LastWriteTime -gt $started.AddSeconds(-60) } |
      Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $idleMin = if ($newest) { ($now - $newest.LastWriteTime).TotalMinutes }
               else { ($now - $started).TotalMinutes }
    $reason = $null
    if ($idleMin -ge $IdleKillMin) { $reason = "silent for $([int]$idleMin) min" }
    elseif (($now - $started).TotalMinutes -ge $IterationCeilingMin) { $reason = "hit the $IterationCeilingMin-min ceiling" }
    if ($reason) {
      Write-Host "[loop] killing the worker - $reason."
      taskkill /PID $proc.Id /T /F 2>$null | Out-Null
      # Drop half-done, uncommitted state so the next iteration starts clean
      # (local commits survive - only unpushed edits of the killed run are lost).
      git rebase --abort 2>$null | Out-Null
      git checkout -- . 2>$null | Out-Null
      $killed = $true
      break
    }
  }
  if ($killed) {
    $code = 124
  } else {
    # PS quirk: after a timed WaitForExit, ExitCode can read null until a
    # blocking WaitForExit() refreshes the handle; a null code then looked
    # like a failure and cost a 15-minute backoff per SUCCESSFUL item.
    $proc.WaitForExit()
    $code = if ($null -ne $proc.ExitCode) { $proc.ExitCode } else { 0 }
  }

  if ($code -eq 124) {
    Start-Sleep -Seconds 30           # the hang already wasted time - go again
  } elseif ($code -ne 0) {
    # Rate limit / transient failure: back off instead of spinning.
    Write-Host "[loop] iteration exited with code $code - waiting 15 minutes."
    Start-Sleep -Seconds 900
  } else {
    Start-Sleep -Seconds 10
  }
}
Write-Host "[loop] done after $i iterations."
