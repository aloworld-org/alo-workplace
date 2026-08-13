#!/usr/bin/env bash
# run-loop.sh — the Business-track build loop engine (docs/autonomy/LOOP.md),
# macOS/Linux version. One Claude Code invocation per queue item, forever —
# until QUEUE.md is complete (STATE.md gains "LOOP COMPLETE") or the loop
# halts ("LOOP HALT"). Ctrl+C is always safe: every finished item was already
# committed and pushed by the iteration that built it.
#
# Usage:
#   bash scripts/run-loop.sh [repo-path]     # default: the repo containing this script
set -u

REPO="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
TRACK="${2:-business}"                    # business | sites | ds (LOOP.md Tracks table)
MAX_ITERATIONS="${MAX_ITERATIONS:-500}"   # hard backstop against runaway loops
PROMPT="Read docs/autonomy/LOOP.md and execute exactly ONE iteration of the loop for track '$TRACK', then exit."
# Every track but the default keeps its journal in a folder of its own. Named
# tracks were hardcoded here, so a new one silently read the *business*
# journal, found its "LOOP COMPLETE" and stopped on the first iteration
# reporting success — the failure that looks exactly like the work being done.
STATE_FILE="docs/autonomy/STATE.md"
[ "$TRACK" != "business" ] && STATE_FILE="docs/autonomy/$TRACK/STATE.md"
if [ ! -f "$STATE_FILE" ]; then
  echo "[loop] no journal at $STATE_FILE — is '$TRACK' a track in docs/autonomy/LOOP.md?"; exit 2
fi

cd "$REPO"

# SINGLE-WRAPPER LOCK: three concurrent-editor incidents traced to stopped
# wrappers surviving as detached processes and spawning rival workers. A
# wrapper now claims its track machine-wide and refuses to start if a live
# owner exists; a stale lock (dead PID) is taken over.
LOCK="$HOME/.alo-loop-$TRACK.lock"
if [ -f "$LOCK" ]; then
  oldpid="$(cat "$LOCK" 2>/dev/null)"
  if [ -n "$oldpid" ] && kill -0 "$oldpid" 2>/dev/null; then
    echo "[loop] another wrapper (PID $oldpid) owns track '$TRACK' on this machine — refusing to start."
    echo "[loop] if that wrapper is truly dead, remove $LOCK and retry."
    exit 3
  fi
  echo "[loop] stale lock from dead PID $oldpid — taking over."
fi
echo $$ > "$LOCK"
trap 'rm -f "$LOCK"' EXIT

for ((i = 1; i <= MAX_ITERATIONS; i++)); do
  echo "============================================================"
  echo "[loop] iteration $i  $(date '+%Y-%m-%d %H:%M')"

  git pull --rebase origin main >/dev/null 2>&1

  # Anchored to the start of a line, because the journal is prose and quotes
  # its own markers. The sites journal says "- **Next:** `LOOP COMPLETE` — every
  # Sites queue item is checked" in the middle of a 2591-line file, describing
  # the end of an earlier wave; an unanchored match found that and stopped the
  # loop on its first iteration with 58 items still open, reporting success.
  # The real marker is appended as its own line, so anchoring tells the record
  # of a decision apart from the decision.
  state="$(cat "$STATE_FILE" 2>/dev/null || true)"
  # `^` alone missed a marker somebody wrote as a heading — `## LOOP HALT: …` —
  # and the wrapper restarted over the top of it. Allowing an optional heading
  # prefix keeps prose out (the journal quotes these markers mid-sentence,
  # always behind a quote or a backtick) while seeing a real one whichever way
  # it was written.
  if grep -qE '^#{0,6} *LOOP COMPLETE' <<<"$state"; then
    echo "[loop] queue complete — stopping."; break
  fi
  if grep -qE '^#{0,6} *LOOP HALT' <<<"$state"; then
    echo "[loop] halted by the agent — fix the reason in STATE.md, remove the marker, restart."; break
  fi

  # One iteration, with an IDLE-based hang guard: a truly hung worker goes
  # silent (its session transcript stops growing) while an honest long item
  # keeps writing — a duration-only guard once executed 90 min of honest
  # work. Kill after IDLE_KILL_MIN of transcript silence; the ceiling is the
  # absolute backstop. The killed item is redone next iteration.
  # --dangerously-skip-permissions is required for unattended runs; the hard
  # safety rails live in LOOP.md and the repo's deny rules.
  proj_key="$(printf '%s' "$REPO" | sed 's#[/: ]#-#g')"
  transcripts="$HOME/.claude/projects/$proj_key"
  start_epoch=$(date +%s)

  claude -p "$PROMPT" --dangerously-skip-permissions &
  cpid=$!
  idle_kill=$(( ${IDLE_KILL_MIN:-20} * 60 ))
  ceiling=$(( ${ITERATION_CEILING_MIN:-240} * 60 ))
  code=""
  while kill -0 "$cpid" 2>/dev/null; do
    sleep 30
    now=$(date +%s)
    # Newest transcript write overall (macOS stat -f, GNU stat -c fallback).
    # The worker creates/writes its transcript within seconds of starting, so
    # newest-overall tracks the CURRENT session almost immediately.
    # GNU stat (-c) FIRST: on Git Bash, BSD-style `stat -f %m` prints
    # filesystem info instead of failing, poisoning the age check — the guard
    # then killed honestly-working workers every 20 minutes. macOS falls
    # through to -f when -c errors out. Numeric-only guard on the result.
    newest=$(find "$transcripts" -name '*.jsonl' -exec stat -c %Y {} \; 2>/dev/null | grep -E '^[0-9]+$' | sort -rn | head -1)
    if [ -z "$newest" ]; then
      newest=$(find "$transcripts" -name '*.jsonl' -exec stat -f %m {} \; 2>/dev/null | grep -E '^[0-9]+$' | sort -rn | head -1)
    fi
    if [ -z "$newest" ] || [ "$newest" -lt "$start_epoch" ] 2>/dev/null && [ $(( start_epoch + 120 )) -gt "$now" ]; then
      # First two minutes: give the worker time to open its transcript.
      newest=$start_epoch
    fi
    [ -z "$newest" ] && newest=$start_epoch
    idle=$(( now - newest ))
    reason=""
    if [ "$idle" -ge "$idle_kill" ]; then reason="silent for $((idle / 60)) min"; fi
    if [ $(( now - start_epoch )) -ge "$ceiling" ]; then reason="hit the $((ceiling / 60))-min ceiling"; fi
    if [ -n "$reason" ]; then
      echo "[loop] killing the worker — $reason."
      kill -TERM "$cpid" 2>/dev/null; sleep 10; kill -KILL "$cpid" 2>/dev/null
      # Drop half-done uncommitted state so the next iteration starts clean.
      git rebase --abort >/dev/null 2>&1
      git checkout -- . >/dev/null 2>&1
      code=124
      break
    fi
  done
  if [ -z "$code" ]; then
    wait "$cpid"; code=$?
  else
    wait "$cpid" 2>/dev/null || true
  fi

  if [ "$code" -eq 124 ]; then
    sleep 30                          # the hang already wasted time — go again
  elif [ "$code" -ne 0 ]; then
    # Rate limit / transient failure: back off instead of spinning.
    echo "[loop] iteration exited with code $code — waiting 15 minutes."
    sleep 900
  else
    sleep 10
  fi
done
echo "[loop] done."
