#!/usr/bin/env bash
# Closes audit #19 — IPC TS types must be ts-rs-generated.  No
# `interface` / `type` definitions outside the generated bindings/
# directory in @booksforge/shared-types.
#
# Pass criterion: shared-types/src/ contains only re-exports of
# bindings/* — no hand-rolled `interface FooResponse { ... }` /
# `type Foo = { ... }` definitions.
# Fail behaviour: prints any hand-written IPC types and exits 1.

set -euo pipefail
cd "$(dirname "$0")/../.."

ROOT="booksforge/packages/shared-types/src"

if [[ ! -d "${ROOT}" ]]; then
  echo "❌ #19 — IPC types: cannot find ${ROOT}."
  exit 1
fi

# Search every .ts/.tsx file under shared-types/src EXCEPT bindings/
# for `interface ` or `type Foo = {` definitions that look like they
# describe an IPC payload (capitalised name, `{` body).
OFFENDERS=$(
  find "${ROOT}" -name '*.ts' -not -path "*/bindings/*" -print0 \
    | xargs -0 grep -nE "^(export[[:space:]]+)?(interface|type)[[:space:]]+[A-Z][A-Za-z0-9_]*([[:space:]]*=[[:space:]]*\{|[[:space:]]*\{)" \
        || true
)

if [[ -z "${OFFENDERS}" ]]; then
  echo "✅ #19 — IPC types: only ts-rs-generated bindings in shared-types/src/."
  exit 0
fi

echo "❌ #19 — Hand-written IPC types detected (must come from booksforge-ipc + ts-rs):"
echo
echo "${OFFENDERS}"
echo
echo "Resolution: move the type to crates/booksforge-ipc/src/, decorate"
echo "with #[derive(...)] and #[ts(export, export_to = \"...\")], run"
echo "  cargo test -p booksforge-ipc"
echo "and replace the hand-written declaration with"
echo "  export type { Foo } from \"./bindings/Foo\";"
echo "See EXTERNAL_AUDIT_BACKLOG.md #19."
exit 1
