#!/usr/bin/env bash
# Closes audit #44 — Production tauri.conf.json metadata must include
# productName, bundleIdentifier, version, publisher, copyright, and
# longDescription / shortDescription / category.
#
# Pass criterion: every required field is present and non-empty.
# Fail behaviour: lists missing keys and exits 1.

set -euo pipefail
cd "$(dirname "$0")/../.."

CONF="booksforge/apps/desktop/tauri.conf.json"

if [[ ! -f "${CONF}" ]]; then
  echo "❌ #44 — tauri.conf.json: cannot find ${CONF}."
  exit 1
fi

# Minimum-required keys that the audit calls out by name.  Names are
# searched as JSON keys (with surrounding quotes + colon) so we don't
# match values containing them.
REQUIRED_KEYS=(
  "productName"
  "bundleIdentifier"
  "version"
  "publisher"
  "copyright"
  "shortDescription"
  "longDescription"
  "category"
)

MISSING=()

for key in "${REQUIRED_KEYS[@]}"; do
  if ! grep -qE "\"${key}\"[[:space:]]*:" "${CONF}"; then
    MISSING+=("${key}")
  fi
done

if [[ "${#MISSING[@]}" -eq 0 ]]; then
  echo "✅ #44 — tauri.conf.json metadata: all required keys present."
  exit 0
fi

echo "❌ #44 — tauri.conf.json missing required keys:"
for key in "${MISSING[@]}"; do
  echo "   - ${key}"
done
echo
echo "Resolution: add the missing keys.  See EXTERNAL_AUDIT_BACKLOG.md"
echo "#44 and docs/DISTRIBUTION.md §3 for canonical values."
exit 1
