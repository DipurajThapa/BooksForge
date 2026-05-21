#!/usr/bin/env bash
# run_canonical.sh — run the locked canonical fiction pipeline end-to-end.
#
# REUSES (does not reinvent):
#   1. booksforge/crates/booksforge-orchestrator/examples/multi_chapter_run.rs
#      The proven Rust binary at ≥95% quality. Scenes were adapted
#      for "My Confused Life" via outline_to_scenes.py (one-off
#      data conversion, not pipeline rewriting).
#   2. book-output/ingest_canonical.py
#      Splits the Rust example's manuscript.md into per-chapter files.
#   3. book-output/final_review.py
#      Re-runs dev-editor on v2 + computes mechanical scorecard.
#   4. book-output/export_book.sh
#      Re-exports DOCX/EPUB/PDF with the publication-ready interior.
#
# Models (per multi_chapter_run.rs):
#   LIGHT = qwen3.5:9b     (intake, critic)
#   HEAVY = qwen3.6:latest (bibles, drafter, polish stack)
#
# Usage:
#   ./run_canonical.sh [book-dir]

set -euo pipefail

BOOK_DIR="${1:-my-confused-life}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
PY="$ROOT/.venv/bin/python"
[[ -x "$PY" ]] || PY="python3"

REPO="$(cd "$ROOT/.." && pwd)"

echo "============================================================================"
echo " BooksForge canonical pipeline (reuse of multi_chapter_run.rs)"
echo "   book:   $BOOK_DIR"
echo "============================================================================"

echo
echo "▶ Stage 1: cargo run --example multi_chapter_run -p booksforge-orchestrator --release"
echo "  (intake → character-bible → world-bible → per-scene drafter → critic → polish)"
cd "$REPO/booksforge"
cargo run --example multi_chapter_run -p booksforge-orchestrator --release

echo
echo "▶ Stage 2: ingest manuscript.md → $BOOK_DIR/canonical/chapters/"
cd "$ROOT"
"$PY" ingest_canonical.py "$BOOK_DIR"

echo
echo "▶ Stage 3: final_review.py (publication-readiness scorecard)"
"$PY" final_review.py "$BOOK_DIR"

echo
echo "▶ Stage 4: export_book.sh (DOCX/EPUB/PDF)"
"$ROOT/export_book.sh" "$BOOK_DIR"

echo
echo "============================================================================"
echo " DONE. Open the reports:"
echo "   $ROOT/$BOOK_DIR/canonical/score_card.json   (Rust pipeline scoring)"
echo "   $ROOT/$BOOK_DIR/PUBLICATION_READINESS.md    (rating scorecard)"
echo "   $ROOT/$BOOK_DIR/export/                     (DOCX/EPUB/PDF)"
echo "============================================================================"
