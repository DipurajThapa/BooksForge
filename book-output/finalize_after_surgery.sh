#!/usr/bin/env bash
# finalize_after_surgery.sh — once scene_surgery.py has rewritten every
# scene, run the publication-readiness checks and produce the final
# PDF/EPUB/DOCX.
#
# Sequence:
#   1. post_surgery_check.py --normalise     (mojibake + sentence-spacing)
#   2. post_surgery_check.py                 (gate — must report 0 issues)
#   3. compose_book.py                       (front + body + back matter)
#   4. export_book.sh                        (HTML → PDF, EPUB-3, DOCX)
#
# Usage: ./finalize_after_surgery.sh [book-dir]

set -euo pipefail

DIR="${1:-my-confused-life}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
PY="$ROOT/.venv/bin/python"
[[ -x "$PY" ]] || PY="python3"

echo "[1/4] Normalise mojibake + sentence-spacing in rewritten scenes"
"$PY" "$ROOT/post_surgery_check.py" "$DIR" --normalise || true

echo
echo "[2/4] Gate: verify all scenes pass content checks"
if ! "$PY" "$ROOT/post_surgery_check.py" "$DIR"; then
  echo "FATAL: content checks failed — fix the offending scenes before export." >&2
  exit 1
fi

echo
echo "[3/4] Compose front + body + back matter"
"$PY" "$ROOT/compose_book.py" "$DIR"

echo
echo "[4/4] Export PDF / EPUB / DOCX"
"$ROOT/export_book.sh" "$DIR"

echo
echo "Done. Open the export folder:"
echo "  open \"$ROOT/$DIR/export\""
