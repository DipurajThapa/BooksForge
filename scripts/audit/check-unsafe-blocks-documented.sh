#!/usr/bin/env bash
# Closes audit #12 — every `unsafe { ... }` block in production Rust
# must carry a `// SAFETY:` comment within the 8 lines preceding it.
#
# Pass criterion: every `unsafe {` (excluding tests) has a SAFETY
# comment in its preceding context.
# Fail behaviour: prints undocumented unsafe sites and exits 1.

set -euo pipefail
cd "$(dirname "$0")/../.."

# Search booksforge/crates/booksforge-fs and booksforge/crates/booksforge-ollama
# (the two crates that opt out of forbid(unsafe_code) per the audit).
SEARCH_PATHS=(
  "booksforge/crates/booksforge-fs/src"
  "booksforge/crates/booksforge-ollama/src"
)

UNDOCUMENTED=()

for path in "${SEARCH_PATHS[@]}"; do
  if [[ ! -d "${path}" ]]; then
    continue
  fi

  while IFS= read -r -d '' file; do
    # Skip files containing only test code.
    if grep -qE "^\s*#\[cfg\(test\)\]" "${file}" && ! grep -qE "unsafe[[:space:]]*\{" "${file}"; then
      continue
    fi

    # Find lines containing `unsafe {` (function bodies, not item-level
    # `unsafe fn`).  For each, look at the 8 lines above it for a
    # // SAFETY: comment.
    grep -nE "unsafe[[:space:]]*\{" "${file}" | while IFS=: read -r lineno _; do
      # Check the 8 lines before lineno (or start of file).
      start=$(( lineno > 8 ? lineno - 8 : 1 ))
      window=$(sed -n "${start},$(( lineno - 1 ))p" "${file}" || true)
      if ! echo "${window}" | grep -qE "//[[:space:]]*SAFETY"; then
        echo "${file}:${lineno}"
      fi
    done
  done < <(find "${path}" -name '*.rs' -print0)
done > /tmp/booksforge-audit-12.tmp || true

if [[ ! -s /tmp/booksforge-audit-12.tmp ]]; then
  echo "✅ #12 — every unsafe block in booksforge-fs / booksforge-ollama has a SAFETY comment."
  rm -f /tmp/booksforge-audit-12.tmp
  exit 0
fi

echo "❌ #12 — unsafe blocks without a // SAFETY: comment in the preceding 8 lines:"
echo
cat /tmp/booksforge-audit-12.tmp
echo
echo "Resolution: add // SAFETY: <invariant> immediately above each"
echo "unsafe { ... } block, stating *why* the call is sound (libc::kill"
echo "semantics, GlobalMemoryStatusEx layout stability, etc.).  Promote"
echo "  #![deny(clippy::undocumented_unsafe_blocks)]"
echo "in the crate's lib.rs to make this a compile-time gate."
echo "See EXTERNAL_AUDIT_BACKLOG.md #12."
rm -f /tmp/booksforge-audit-12.tmp
exit 1
