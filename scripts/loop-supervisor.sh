#!/usr/bin/env bash
# loop-supervisor.sh — keeps a build loop alive on macOS/Linux until its
# journal says the queue is finished (docs/autonomy/LOOP.md).
#
# One checkout, one or more tracks run one after the other:
#   bash scripts/loop-supervisor.sh <repo-path> <track> [<track> ...]
#
# Why a supervisor: the runner's watchdog kills a silent worker (correctly),
# and on 2026-08-26 the runner died with it, so a loop silently stopped with
# twelve items open. The runner is restarted here until the journal carries a
# fresh `LOOP COMPLETE` or `LOOP HALT` — the only clean exits. Between runners
# the checkout's own build output is removed (dependencies kept), because a
# build tree that filled the disk mid-link has cost iterations before.
#
# Limits: 45 min of transcript silence, 5 h per iteration. Release builds and
# database suites are slow on a laptop, and an idle kill on the edge of the
# worker's own build-polling pattern has killed honest work.
set -u

REPO="${1:?repo path}"; shift
[ $# -ge 1 ] || { echo "usage: $0 <repo-path> <track> [<track> ...]"; exit 2; }
export DATABASE_URL="${DATABASE_URL:-postgres://alo:alo-dev-only@127.0.0.1:5432/alo_scratch}"
export SQLX_OFFLINE=true
export IDLE_KILL_MIN="${IDLE_KILL_MIN:-45}"
export ITERATION_CEILING_MIN="${ITERATION_CEILING_MIN:-300}"

cd "$REPO" || exit 2

clean_checkout() {
  # cargo clean of this workspace's own crates only — the same trade
  # scripts/dev.ps1 -Action Clean makes on Windows.
  local packages
  packages="$(cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c 'import json,sys; print(" ".join("-p "+p["name"] for p in json.load(sys.stdin)["packages"]))')"
  [ -n "$packages" ] && cargo clean $packages
  echo "[supervisor] cleaned this checkout's crates; free: $(df -h . | awk 'NR==2 {print $4}')"
}

for track in "$@"; do
  state="docs/autonomy/STATE.md"
  [ "$track" != "business" ] && state="docs/autonomy/$track/STATE.md"
  while true; do
    echo "[supervisor] starting runner for '$track' $(date '+%Y-%m-%d %H:%M')"
    clean_checkout
    bash scripts/run-loop.sh "$REPO" "$track"
    if grep -qE '^#{0,6} *LOOP (COMPLETE|HALT)' "$state" 2>/dev/null; then
      echo "[supervisor] '$track' journal says finished — moving on $(date '+%H:%M')"
      break
    fi
    echo "[supervisor] runner exited without a finish marker — restarting in 60s"
    sleep 60
  done
done
echo "[supervisor] every track finished $(date '+%Y-%m-%d %H:%M')"
