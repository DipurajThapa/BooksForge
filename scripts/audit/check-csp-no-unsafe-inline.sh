#!/usr/bin/env bash
# Closes audit #15 — CSP must not contain 'unsafe-inline'.
#
# Pass criterion: tauri.conf.json's CSP has neither 'unsafe-inline'
# nor 'unsafe-eval' anywhere.
# Fail behaviour: prints the offending CSP line and exits 1.

set -euo pipefail
cd "$(dirname "$0")/../.."

CONF="booksforge/apps/desktop/tauri.conf.json"

if [[ ! -f "${CONF}" ]]; then
  echo "❌ #15 — CSP: cannot find ${CONF}."
  exit 1
fi

OFFENDERS=$(grep -nE "'unsafe-inline'|'unsafe-eval'" "${CONF}" || true)

if [[ -z "${OFFENDERS}" ]]; then
  echo "✅ #15 — CSP: no 'unsafe-inline' / 'unsafe-eval' in ${CONF}."
  exit 0
fi

echo "❌ #15 — CSP still contains 'unsafe-inline' / 'unsafe-eval':"
echo
echo "${OFFENDERS}"
echo
echo "Resolution: migrate inline style=\"...\" to CSS modules / Vanilla"
echo "Extract; tighten CSP to style-src 'self'.  See"
echo "EXTERNAL_AUDIT_BACKLOG.md #15."
exit 1
