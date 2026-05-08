#!/usr/bin/env bash
# Closes audit #44 — Production tauri.conf.json metadata must include
# productName, identifier (the Tauri-2 name for what Tauri-1 called
# bundleIdentifier), version, plus the publisher / copyright /
# shortDescription / longDescription / category fields under bundle.
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

# Required keys, searched as JSON keys (quoted + colon) so we don't
# match values that happen to contain the key name.
#
# Tauri 2 schema:
#   - productName / version / identifier are TOP-LEVEL.
#   - publisher / copyright / shortDescription / longDescription /
#     category live under "bundle".
#
# We don't enforce nesting (a single grep over the file is enough),
# but we DO enforce non-empty values via the second pattern below.
REQUIRED_KEYS=(
  "productName"
  "identifier"
  "version"
  "publisher"
  "copyright"
  "shortDescription"
  "longDescription"
  "category"
)

MISSING=()
EMPTY=()

for key in "${REQUIRED_KEYS[@]}"; do
  if ! grep -qE "\"${key}\"[[:space:]]*:" "${CONF}"; then
    MISSING+=("${key}")
    continue
  fi
  # Reject empty-string values: "key": "" .
  if grep -qE "\"${key}\"[[:space:]]*:[[:space:]]*\"\"[[:space:]]*[,}]" "${CONF}"; then
    EMPTY+=("${key}")
  fi
done

if [[ "${#MISSING[@]}" -eq 0 && "${#EMPTY[@]}" -eq 0 ]]; then
  echo "✅ #44 — tauri.conf.json metadata: all required keys present and non-empty."
  exit 0
fi

if [[ "${#MISSING[@]}" -gt 0 ]]; then
  echo "❌ #44 — tauri.conf.json missing required keys:"
  for key in "${MISSING[@]}"; do
    echo "   - ${key}"
  done
fi

if [[ "${#EMPTY[@]}" -gt 0 ]]; then
  echo "❌ #44 — tauri.conf.json has empty-string values for:"
  for key in "${EMPTY[@]}"; do
    echo "   - ${key}"
  done
fi

echo
echo "Resolution: add / fill the missing keys.  In Tauri 2:"
echo "  - productName, version, identifier are TOP-LEVEL."
echo "  - publisher, copyright, shortDescription, longDescription,"
echo "    category live under \"bundle\"."
echo "See EXTERNAL_AUDIT_BACKLOG.md #44 and docs/DISTRIBUTION.md §3"
echo "for canonical values."
exit 1
