#!/usr/bin/env bash
# Closes audit #8 (static half) — no crate other than booksforge-ollama
# may mention both `reqwest::` and a manuscript-touching type
# (Manuscript, SceneContent, Outline, Brief, PmDoc, etc.) in the same
# file.  Adding such a combination opens a path for manuscript content
# to reach a non-loopback URL.
#
# Pass criterion: no Rust file outside booksforge-ollama matches both
# patterns.  (booksforge-ollama itself is allowlisted because the
# loopback enforcement is its job.)
# Fail behaviour: prints offending files and exits 1.

set -euo pipefail
cd "$(dirname "$0")/../.."

ROOT="booksforge/crates"

if [[ ! -d "${ROOT}" ]]; then
  echo "❌ #8 — manuscript-over-wire static guard: cannot find ${ROOT}."
  exit 1
fi

# Allowlisted crate (the only place reqwest is allowed to coexist with
# pretty much anything).
ALLOWED_CRATE="booksforge-ollama"

# Manuscript-touching type names.  We grep for *type identifiers*, so
# inevitably this is a heuristic — but the pattern is conservative
# enough that adding any of these alongside reqwest is intentional.
MANUSCRIPT_TYPES=(
  "Manuscript"
  "SceneContent"
  "PmDoc"
  "pm_doc"
  "Outline"
  "Brief"
  "ChapterDraft"
  "MemoryEntry"
  "VocabEntry"
)

OFFENDERS=()

while IFS= read -r -d '' file; do
  # Skip allowlisted crate.
  if [[ "${file}" == *"crates/${ALLOWED_CRATE}/"* ]]; then
    continue
  fi
  # Skip test files (they may legitimately mock both for assertions).
  if [[ "${file}" == *"/tests/"* ]] || [[ "${file}" == *"_test.rs" ]]; then
    continue
  fi

  if grep -qE "(reqwest::|reqwest\.)" "${file}"; then
    for ty in "${MANUSCRIPT_TYPES[@]}"; do
      # Use word boundary to reduce false positives.
      if grep -qE "\b${ty}\b" "${file}"; then
        OFFENDERS+=("${file}: contains both 'reqwest' and '${ty}'")
        break
      fi
    done
  fi
done < <(find "${ROOT}" -name '*.rs' -print0)

if [[ "${#OFFENDERS[@]}" -eq 0 ]]; then
  echo "✅ #8 — no Rust file outside booksforge-ollama mentions both reqwest and a manuscript type."
  exit 0
fi

echo "❌ #8 — manuscript-over-wire static guard:"
echo
for line in "${OFFENDERS[@]}"; do
  echo "   ${line}"
done
echo
echo "Resolution: split the file so the reqwest-using and manuscript-"
echo "consuming code live in separate modules; or move the network call"
echo "into booksforge-ollama; or add an explicit allowlist comment if"
echo "the combination is genuinely safe (and explain why)."
echo "See EXTERNAL_AUDIT_BACKLOG.md #8."
exit 1
