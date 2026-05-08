#!/usr/bin/env bash
# Closes audit #18 — `let _ = ...` discards in apps/desktop/src/ must
# carry a `// Best-effort:` comment immediately above OR within 2 lines.
#
# Pass criterion: every `let _ = ` is preceded by `// Best-effort:`
# (or similar justification keyword: Best-effort, BACKLOG, Intentional,
# Safe-to-ignore).
# Fail behaviour: prints unjustified discards and exits 1.

set -euo pipefail
cd "$(dirname "$0")/../.."

ROOT="booksforge/apps/desktop/src"

if [[ ! -d "${ROOT}" ]]; then
  echo "❌ #18 — let _ budget: cannot find ${ROOT}."
  exit 1
fi

UNJUSTIFIED=()

while IFS= read -r -d '' file; do
  grep -nE "^\s*let _ = " "${file}" | while IFS=: read -r lineno _; do
    start=$(( lineno > 3 ? lineno - 3 : 1 ))
    window=$(sed -n "${start},$(( lineno - 1 ))p" "${file}" || true)
    if ! echo "${window}" | grep -qE "//.*(Best-effort|BACKLOG|Intentional|Safe-to-ignore|SAFETY)"; then
      echo "${file}:${lineno}"
    fi
  done
done < <(find "${ROOT}" -name '*.rs' -print0) > /tmp/booksforge-audit-18.tmp || true

count=$(wc -l < /tmp/booksforge-audit-18.tmp 2>/dev/null | tr -d ' ' || echo 0)

if [[ "${count}" -eq 0 ]]; then
  echo "✅ #18 — every let _ = ... in commands/ carries a Best-effort comment."
  rm -f /tmp/booksforge-audit-18.tmp
  exit 0
fi

echo "❌ #18 — ${count} let _ = ... discards without Best-effort justification:"
echo
cat /tmp/booksforge-audit-18.tmp
echo
echo "Resolution: directly above each \`let _ = ...\` add a comment of"
echo "the form"
echo "  // Best-effort: <reason>; failure is logged at WARN."
echo "and add a tracing::warn!() inside the block, OR propagate the error."
echo "See EXTERNAL_AUDIT_BACKLOG.md #18."
rm -f /tmp/booksforge-audit-18.tmp
exit 1
