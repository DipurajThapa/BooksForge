#!/usr/bin/env bash
# run_pipeline.sh — end-to-end manuscript pipeline with consistency gates.
#
# Stages (each gated by consistency_check.py before moving on):
#
#   [1] Ingest         canonical Rust manuscript.md → per-chapter files
#                      (mojibake + sentence-spacing auto-fixed here)
#   [2] Auto-fix       mojibake, spacing, model-commentary leakage
#                      (idempotent — safe to re-run)
#   [3] Surgery        rewrite scenes that fail the content checks
#                      (POV bleed, length bloat, repetition, gaps)
#                      Uses qwen3.6:latest. Skipped if all-clear.
#   [4] Gate           consistency_check --gate must pass before export
#   [5] Compose        front + body + back matter → FULL_MANUSCRIPT.md
#   [6] Export         pandoc + chromium → PDF / EPUB-3 / DOCX
#   [7] Final verify   render-time check on output files
#
# Usage:
#   ./run_pipeline.sh [book-dir]              # default: my-confused-life
#   ./run_pipeline.sh [book-dir] --no-surgery # skip stage 3 (assume scenes are clean)
#   ./run_pipeline.sh [book-dir] --force      # skip cleanliness check, force surgery

set -euo pipefail

DIR="${1:-my-confused-life}"
shift || true
NO_SURGERY=0
FORCE_SURGERY=0
for arg in "$@"; do
  case "$arg" in
    --no-surgery) NO_SURGERY=1 ;;
    --force)      FORCE_SURGERY=1 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC_DIR="$ROOT/$DIR"
PY="$ROOT/.venv/bin/python"
[[ -x "$PY" ]] || PY="python3"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "FATAL: $SRC_DIR not found." >&2
  exit 1
fi

# ── Stage 1: Ingest (only if a fresh canonical run is sitting in
# multi-chapter-runs/ and the chapters dir is empty or older) ─────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[1/7] Ingest canonical run"
echo "════════════════════════════════════════════════════════════════"
if [[ ! -d "$SRC_DIR/canonical/chapters" ]] || [[ -z "$(ls -A "$SRC_DIR/canonical/chapters" 2>/dev/null)" ]]; then
  "$PY" "$ROOT/ingest_canonical.py" "$DIR"
else
  echo "chapters already present at $SRC_DIR/canonical/chapters — skipping ingest"
  echo "(re-ingest by deleting that directory or passing --force)"
fi

# ── Stage 2: Auto-fix safe defects in place ───────────────────────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[2/7] Auto-fix mojibake / sentence-spacing / model commentary"
echo "════════════════════════════════════════════════════════════════"
"$PY" "$ROOT/consistency_check.py" "$DIR" --auto-fix || true

# ── Stage 3: Decide whether surgery is needed ─────────────────────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[3/7] Consistency triage — should we run surgery?"
echo "════════════════════════════════════════════════════════════════"
REPORT="$SRC_DIR/canonical/consistency-report.json"
"$PY" "$ROOT/consistency_check.py" "$DIR" --json "$REPORT" || true
CRITICAL_COUNT="$("$PY" -c "import json,sys; d=json.load(open(sys.argv[1])); print(len(d.get('critical', [])))" "$REPORT")"
echo "critical issues: $CRITICAL_COUNT"

if [[ "$NO_SURGERY" -eq 1 ]]; then
  echo "--no-surgery set; skipping content surgery."
elif [[ "$FORCE_SURGERY" -eq 1 ]] || [[ "$CRITICAL_COUNT" -gt 0 ]]; then
  echo
  echo "[3a] Running scene surgery via qwen3.6:latest..."
  echo "     (this rewrites every scene with full book context;"
  echo "      ~50s per scene, ~15 min for a 6-chapter / 18-scene book)"
  "$PY" "$ROOT/scene_surgery.py" "$DIR" --all
  echo
  echo "[3b] Re-running auto-fix on the rewritten scenes..."
  "$PY" "$ROOT/consistency_check.py" "$DIR" --auto-fix || true
else
  echo "no critical issues; skipping content surgery."
fi

# ── Stage 4: Hard gate — every critical check must pass ───────────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[4/7] Hard gate — consistency_check --gate"
echo "════════════════════════════════════════════════════════════════"
if ! "$PY" "$ROOT/consistency_check.py" "$DIR" --gate; then
  echo
  echo "FATAL: critical content issues remain after surgery." >&2
  echo "Inspect $SRC_DIR/canonical/consistency-report.json and either:" >&2
  echo "  - re-run with --force (another surgery pass)" >&2
  echo "  - hand-edit the offending scenes in $SRC_DIR/canonical/chapters/" >&2
  exit 1
fi
echo "gate passed."

# ── Stage 5: Compose ──────────────────────────────────────────────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[5/7] Compose front + body + back matter"
echo "════════════════════════════════════════════════════════════════"
"$PY" "$ROOT/compose_book.py" "$DIR"

# ── Stage 6: Export to PDF / EPUB / DOCX ──────────────────────────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[6/7] Export PDF / EPUB-3 / DOCX"
echo "════════════════════════════════════════════════════════════════"
"$ROOT/export_book.sh" "$DIR"

# ── Stage 7: Final render-time verification ───────────────────────────────
echo
echo "════════════════════════════════════════════════════════════════"
echo "[7/7] Final verification"
echo "════════════════════════════════════════════════════════════════"
EXPORT_DIR="$SRC_DIR/export"
TITLE="$("$PY" -c "import json,sys; d=json.load(open(sys.argv[1])); print((d.get('title_suggestions') or ['Book'])[0])" "$SRC_DIR/01-brief.json")"
SAFE_TITLE="$(echo "$TITLE" | tr -c '[:alnum:]-' '_' | sed -E 's/_+/_/g; s/^_|_$//g')"

# Quick existence + size check on each output artifact. Anything below
# the size floor is suspicious (empty PDF, broken EPUB, etc.).
for ext in pdf epub docx; do
  p="$EXPORT_DIR/${SAFE_TITLE}.${ext}"
  if [[ ! -f "$p" ]]; then
    echo "FATAL: expected output missing: $p" >&2
    exit 1
  fi
  size=$(stat -f%z "$p" 2>/dev/null || stat -c%s "$p" 2>/dev/null)
  case "$ext" in
    pdf)  floor=100000  ;;
    epub) floor=20000   ;;
    docx) floor=20000   ;;
  esac
  if [[ "$size" -lt "$floor" ]]; then
    echo "FATAL: $p is suspiciously small (${size} bytes < ${floor})" >&2
    exit 1
  fi
  printf "  OK  %-40s  %s bytes\n" "$(basename "$p")" "$size"
done

# EPUB XHTML validity — pandoc sometimes leaves crossed tags; xmllint
# catches them before a reader chokes.
if command -v xmllint >/dev/null 2>&1; then
  TMP="$(mktemp -d)"
  unzip -q "$EXPORT_DIR/${SAFE_TITLE}.epub" -d "$TMP"
  XHTML_ERRORS=0
  for f in "$TMP"/*.xhtml "$TMP"/EPUB/*.xhtml "$TMP"/OEBPS/*.xhtml; do
    [[ -f "$f" ]] || continue
    if ! xmllint --noout "$f" 2>/dev/null; then
      echo "FATAL: invalid XHTML in $f" >&2
      XHTML_ERRORS=$((XHTML_ERRORS + 1))
    fi
  done
  rm -rf "$TMP"
  if [[ "$XHTML_ERRORS" -gt 0 ]]; then
    exit 1
  fi
  echo "  OK  EPUB XHTML is well-formed"
fi

echo
echo "─────────────────────────────────────────────────────────────────"
echo "Pipeline complete. Final outputs in:"
echo "  $EXPORT_DIR"
echo "─────────────────────────────────────────────────────────────────"
