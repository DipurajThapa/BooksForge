#!/usr/bin/env bash
# Run every audit check and print a summary table.
#
# Exit code:
#   0 — every check passed
#   1 — one or more checks failed (informational checks ignored)
#
# Usage:
#   ./scripts/audit/run-all.sh
#   ./scripts/audit/run-all.sh --quiet   # only summary, no per-check output

set -uo pipefail

QUIET="${1:-}"

cd "$(dirname "$0")/../.."

# Each entry is "<audit-item>:<script-path>".  Order: lowest item
# number first, informational last.
CHECKS=(
  "8:scripts/audit/check-no-manuscript-over-wire.sh"
  "12:scripts/audit/check-unsafe-blocks-documented.sh"
  "15:scripts/audit/check-csp-no-unsafe-inline.sh"
  "18:scripts/audit/check-let-underscore-budget.sh"
  "19:scripts/audit/check-no-handwritten-ipc-types.sh"
  "44:scripts/audit/check-tauri-conf-metadata.sh"
)

INFORMATIONAL=(
  "scripts/audit/check-no-todo-tracker.sh"
)

PASS=()
FAIL=()
FAILED_DETAIL=""

for entry in "${CHECKS[@]}"; do
  item="${entry%%:*}"
  script="${entry#*:}"

  if [[ "${QUIET}" != "--quiet" ]]; then
    echo "── #${item} (${script}) ────────────────────────────────────────"
  fi

  if output=$( "${script}" 2>&1 ); then
    PASS+=("${item}")
    if [[ "${QUIET}" != "--quiet" ]]; then
      echo "${output}" | head -1
    fi
  else
    FAIL+=("${item}")
    FAILED_DETAIL+="${output}\n"
    if [[ "${QUIET}" != "--quiet" ]]; then
      echo "${output}"
    fi
  fi
done

if [[ "${QUIET}" != "--quiet" ]]; then
  echo
  echo "── Informational checks ────────────────────────────────────────"
  for script in "${INFORMATIONAL[@]}"; do
    "${script}" | sed 's/^/    /'
  done
fi

echo
echo "════════════════════════════════════════════════════════════════"
echo "  AUDIT SUMMARY"
echo "════════════════════════════════════════════════════════════════"
echo "  Passed  : ${#PASS[@]} (${PASS[*]:-})"
echo "  Failed  : ${#FAIL[@]} (${FAIL[*]:-})"
echo "════════════════════════════════════════════════════════════════"

if [[ "${#FAIL[@]}" -gt 0 ]]; then
  echo
  echo "Open audit items: see EXTERNAL_AUDIT_BACKLOG.md for resolution."
  exit 1
fi

exit 0
