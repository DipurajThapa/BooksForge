#!/usr/bin/env bash
# Hygiene check (not tied to a specific audit item) — counts TODO /
# FIXME / XXX / HACK / BACKLOG markers in production source and
# reports the trend.  Always exits 0 (informational); CI captures the
# count so the team can track whether the debt is growing or shrinking.
#
# Production source = booksforge/crates/*/src + booksforge/apps/*/src.
# Excludes: tests/, examples/, benches/, *.test.{ts,tsx}.

set -euo pipefail
cd "$(dirname "$0")/../.."

PATTERNS="TODO|FIXME|XXX|HACK|BACKLOG"

count_in() {
  local dir="$1"
  if [[ ! -d "${dir}" ]]; then
    echo 0
    return
  fi
  grep -rEn "${PATTERNS}" "${dir}" \
    --include='*.rs' --include='*.ts' --include='*.tsx' \
    --exclude-dir=tests \
    --exclude-dir=examples \
    --exclude-dir=benches \
    --exclude='*.test.ts' \
    --exclude='*.test.tsx' \
    2>/dev/null | wc -l | tr -d ' '
}

CRATES_COUNT=$(count_in "booksforge/crates")
APPS_COUNT=$(count_in "booksforge/apps")
TOTAL=$(( CRATES_COUNT + APPS_COUNT ))

echo "ℹ️  TODO/FIXME/XXX/HACK/BACKLOG markers in production source:"
echo "    booksforge/crates/    ${CRATES_COUNT}"
echo "    booksforge/apps/      ${APPS_COUNT}"
echo "    TOTAL                 ${TOTAL}"
echo
echo "(informational only — does not fail CI.  Use this to track debt"
echo "trend across PRs.)"

# Optional drill-down on demand.
if [[ "${1:-}" == "--list" ]]; then
  echo
  echo "Top 25 occurrences:"
  grep -rEn "${PATTERNS}" booksforge/crates booksforge/apps \
    --include='*.rs' --include='*.ts' --include='*.tsx' \
    --exclude-dir=tests --exclude-dir=examples --exclude-dir=benches \
    --exclude='*.test.ts' --exclude='*.test.tsx' \
    2>/dev/null | head -25
fi

exit 0
