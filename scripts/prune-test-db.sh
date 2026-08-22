#!/usr/bin/env bash
# Prune the local test database of tenants left behind by test runs.
#
# Why this exists: 92 of the 128 test files that create a tenant never delete
# it, and the shared harness does not either. That is harmless in CI, which
# gets a fresh database per run, and harmless for a developer running a suite
# once. It is not harmless for a build loop running suites continuously: six
# days of running left 74 671 tenants, 228 046 postings and a 583 MB database,
# and every tenant-scoped query in every test scanned all of it.
#
# The effect is severe and easy to misread, because it grows silently: the same
# `cargo nextest run -p alo-store` took 18 seconds against a clean database and
# over 90 minutes against the bloated one. It also makes any timing measurement
# expire — a gate benchmarked on Monday no longer describes Tuesday.
#
# Tests are not wrong to create tenants; the tenant is the isolation boundary
# and a test that shares one proves less. The environment is what needs the
# sweeping, so the sweeping lives here rather than in 92 test files.
#
# Usage:  bash scripts/prune-test-db.sh [max-age-hours]   # default 2
#
# Deletes every tenant older than the cutoff EXCEPT the bootstrap accounts the
# local backend signs in as. Deleting a tenant cascades to its rows, so this is
# the only statement needed. Batched, so a suite running alongside it waits
# briefly rather than blocking on one long lock.
set -euo pipefail

HOURS="${1:-2}"
CONTAINER="${ALO_PG_CONTAINER:-alo-pg}"
KEEP_EMAILS="'disan@alomails.com','admin@alomails.com'"
BATCH=5000

# The database to prune, taken from DATABASE_URL's last path segment.
#
# This used to be a `DB_URL` variable that psql_q ignored: the query function
# passed `-d alo` literally, so setting DATABASE_URL changed nothing and a
# checkout whose suites filled `alo_loop` pruned the wrong database and
# reported success. It was flagged four times in docs/autonomy before anyone
# was in a position to fix it, and it hid a second failure behind it — `alo`
# sat at migration version 154 while the suites needed 405, so the runs that
# did reach it died with VersionMismatch(154), which reads like a broken
# migration rather than a wrong database.
DB="${DATABASE_URL##*/}"
DB="${DB%%\?*}"
DB="${DB:-alo_scratch}"

# `alo` is the database the product runs on (CLAUDE.md, one-database rule),
# and everything below this line is `DELETE FROM tenants`. Pruning it would
# delete somebody's real tenants — every one of them but two, silently, in
# batches of five thousand. There is no argument for doing that from a
# maintenance script, so it is refused rather than confirmed.
if [ "$DB" = "alo" ]; then
  echo "refusing to prune \`alo\`: it is the database the product runs on," >&2
  echo "not a test database (CLAUDE.md, one-database rule). Point" >&2
  echo "DATABASE_URL at the scratch database your suites fill." >&2
  exit 2
fi

echo "pruning database: $DB"

# Errors are NOT swallowed. The first version of this script sent stderr to
# /dev/null, so when the delete failed it printed an empty string, the caller
# read that as "nothing to prune", and the script reported success while doing
# nothing for hours. A maintenance script that cannot fail loudly is worse than
# no maintenance script.
psql_q() { docker exec "$CONTAINER" psql -U alo -d "$DB" -t -v ON_ERROR_STOP=1 -c "$1" | tr -d ' '; }

# The prunable set: old enough that no running suite can still be using it, and
# not one of the bootstrap accounts the local backend signs in as.
PRUNABLE="created_at < now() - make_interval(hours => $HOURS)
          AND id NOT IN (SELECT tenant_id FROM users WHERE email IN ($KEEP_EMAILS))"

before=$(psql_q "select count(*) from tenants;")
echo "tenants before: ${before:-unknown}"

# `bank_matches` holds ON DELETE RESTRICT foreign keys to `billing_payments`
# and `fin_entries`, so deleting a tenant that ever reconciled a bank line
# fails on the constraint. That is a real defect in the erasure path — queued
# as B7.02, because `delete_tenant` is what a GDPR request runs — and until it
# is fixed the matches must be cleared first or nothing here can proceed.
# Remove this block when B7.02 lands; the delete below should stand alone.
cleared=$(psql_q "DELETE FROM bank_matches WHERE tenant_id IN
                    (SELECT id FROM tenants WHERE $PRUNABLE);" | grep -oE '[0-9]+$' || echo 0)
[ "${cleared:-0}" -gt 0 ] && echo "  cleared $cleared bank_matches (B7.02 workaround)"

while :; do
  deleted=$(psql_q "DELETE FROM tenants WHERE id IN (
      SELECT id FROM tenants WHERE $PRUNABLE LIMIT $BATCH);" | grep -oE '[0-9]+$' || echo 0)
  [ "${deleted:-0}" -eq 0 ] && break
  echo "  pruned $deleted"
done

after=$(psql_q "select count(*) from tenants;")
echo "tenants after:  ${after:-unknown}"
echo "database size:  $(psql_q "select pg_size_pretty(pg_database_size('$DB'));")"
