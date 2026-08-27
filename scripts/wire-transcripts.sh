#!/usr/bin/env bash
# Regenerates the scripted wire transcripts (mail M6.1) and splices them into
# docs/interop.md between the wire-transcripts markers.
#
# The transcripts are captured by the `transcripts` integration-test binaries
# in alo-imap, alo-smtp and alo-jmap: each drives the real protocol over a
# real local socket, asserts the exchange, and writes the recorded dialog when
# ALO_WIRE_TRANSCRIPTS names a directory. Needs the test Postgres reachable
# (the suites create and drop their own database via alo-test-db).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$root/target/wire-transcripts"
doc="$root/docs/interop.md"

rm -rf "$out"
mkdir -p "$out"

(
  cd "$root"
  ALO_WIRE_TRANSCRIPTS="$out" SQLX_OFFLINE=true \
    cargo nextest run -p alo-imap -p alo-smtp -p alo-jmap -E 'binary(transcripts)'
)

# Assemble the section: each transcript file's first line is its title.
section="$out/section.md"
{
  echo
  echo "Generated $(date -u +%Y-%m-%d) by \`bash scripts/wire-transcripts.sh\`."
  echo "TLS transcripts show the decrypted stream of a real rustls session; the"
  echo "DAV exchanges are the literal HTTP/1.1 bytes (the production proxy"
  echo "terminates TLS in front of them). Credentials and bearer blobs are"
  echo "redacted, ids and addresses normalised; \`(…)\` lines are annotations,"
  echo "not wire bytes."
  echo
  for t in imap imap-xoauth2 pop3 smtp-submission smtp-xoauth2 carddav-sync caldav; do
    f="$out/$t.txt"
    if [ ! -f "$f" ]; then
      echo "missing transcript: $t (did its test fail or skip?)" >&2
      exit 1
    fi
    echo "### $(head -n 1 "$f")"
    echo
    echo '```text'
    tail -n +2 "$f"
    echo '```'
    echo
  done
} >"$section"

grep -q '<!-- wire-transcripts:begin -->' "$doc" || {
  echo "docs/interop.md has no wire-transcripts markers" >&2
  exit 1
}

awk -v f="$section" '
  /<!-- wire-transcripts:begin -->/ {
    print
    while ((getline line < f) > 0) print line
    close(f)
    skip = 1
    next
  }
  /<!-- wire-transcripts:end -->/ { skip = 0 }
  !skip { print }
' "$doc" >"$doc.tmp"
mv "$doc.tmp" "$doc"

echo "wire transcripts regenerated; docs/interop.md updated."
