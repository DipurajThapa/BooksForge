#!/usr/bin/env bash
# export_book.sh — assemble + format a publication-ready fiction interior.
#
# Pipeline:
#   1. Run compose_book.py to wrap chapters in front/back matter HTML
#   2. Pandoc → HTML5 standalone with print CSS for the chromium engine
#   3. Headless Chrome → PDF (6×9 trim, mirrored margins, drop caps)
#   4. Pandoc → EPUB-3 (uses a stripped CSS variant for reflowable display)
#   5. Pandoc → DOCX (basic export; final typesetting belongs in InDesign)
#
# Usage:
#   ./export_book.sh <book-output-dir>
#
# Example:
#   ./export_book.sh my-confused-life

set -euo pipefail

DIR="${1:-my-confused-life}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC_DIR="$ROOT/$DIR"
OUT_DIR="$SRC_DIR/export"
PY="$ROOT/.venv/bin/python"
[[ -x "$PY" ]] || PY="python3"

if [[ ! -d "$SRC_DIR/chapters" ]]; then
  echo "FATAL: $SRC_DIR/chapters does not exist." >&2
  echo "Run booksforge_fiction_pipeline.py first." >&2
  exit 1
fi

# ── 0. Consistency gate ────────────────────────────────────────────────────
# If a canonical-flow consistency_check.py is available and the
# canonical chapters dir exists, gate the export on it. This blocks
# the production of broken PDFs/EPUBs from manuscripts that still have
# critical content defects (POV bleed, missing scenes, etc.).
# Skip with EXPORT_SKIP_GATE=1 for hand-edited cases where the user
# wants to inspect the output anyway.
if [[ -d "$SRC_DIR/canonical/chapters" ]] && [[ -f "$ROOT/consistency_check.py" ]] && [[ "${EXPORT_SKIP_GATE:-0}" != "1" ]]; then
  echo "[0/4] Consistency gate (set EXPORT_SKIP_GATE=1 to bypass)"
  if ! "$PY" "$ROOT/consistency_check.py" "$DIR" --gate; then
    echo "FATAL: consistency gate failed. Fix scenes or re-run scene_repair.py." >&2
    exit 1
  fi
fi

# ── 1. Compose front/back matter into FULL_MANUSCRIPT.md ───────────────────
echo "[1/4] Compose front + body + back matter"
"$PY" "$ROOT/compose_book.py" "$DIR"
SRC="$SRC_DIR/FULL_MANUSCRIPT.md"

mkdir -p "$OUT_DIR"

# Title is the first entry of `01-brief.json::title_suggestions`. The
# composed manuscript no longer starts with a markdown `# Title` line
# (that was triggering pandoc's EPUB section-splitter and breaking the
# XHTML); the title now lives only in the brief and the composed half-
# title div.
TITLE="$("$PY" -c "import json,sys; d=json.load(open(sys.argv[1])); print((d.get('title_suggestions') or ['Book'])[0])" "$SRC_DIR/01-brief.json")"
[[ -z "$TITLE" ]] && TITLE="$DIR"
SAFE_TITLE="$(echo "$TITLE" | tr -c '[:alnum:]-' '_' | sed -E 's/_+/_/g; s/^_|_$//g')"

DOCX="$OUT_DIR/${SAFE_TITLE}.docx"
EPUB="$OUT_DIR/${SAFE_TITLE}.epub"
HTML="$OUT_DIR/${SAFE_TITLE}.html"
PDF="$OUT_DIR/${SAFE_TITLE}.pdf"
CSS_SRC="$ROOT/fiction_print.css"
CSS_TARGET="$OUT_DIR/fiction_print.css"
cp "$CSS_SRC" "$CSS_TARGET"

echo "Title:  $TITLE"
echo "Out:    $OUT_DIR"
echo

# ── 2. Pandoc → HTML5 (paged-media CSS) ────────────────────────────────────
echo "[2/4] HTML (paged-media)"
pandoc "$SRC" \
  --from=gfm+raw_html \
  --to=html5 \
  --standalone \
  --metadata=title:"$TITLE" \
  --metadata=lang:en \
  --css="fiction_print.css" \
  --output="$HTML"

# Web fonts (EB Garamond + Cormorant Garamond) are loaded via @import
# inside fiction_print.css — no sed gymnastics needed. Pandoc's
# `<header id="title-block-header">` is hidden by a CSS rule in the
# same file so the composed half-title page stands alone.

# ── 3. Headless Chrome → PDF (book trim) ───────────────────────────────────
echo "[3/4] PDF via headless Chrome"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
"$CHROME" \
  --headless=new \
  --disable-gpu \
  --hide-scrollbars \
  --no-pdf-header-footer \
  --no-margins \
  --print-to-pdf="$PDF" \
  --virtual-time-budget=10000 \
  "file://$HTML" 2>/dev/null

ls -la "$PDF" 2>/dev/null | awk '{print "       " $5, $NF}'

# ── 4. EPUB-3 + DOCX (pandoc handles section breaks via H1/H2) ─────────────
echo "[4/4] EPUB-3 and DOCX"

# Strip the @page / running-head rules from the EPUB CSS — those only
# apply to paginated print media and confuse reflowable EPUB readers.
# Also append a rule that hides pandoc's auto-injected `<h1>` above
# the body content (derived from `--metadata=title`). The composed
# half-title page already prints the title; the duplicate H1 from
# pandoc would otherwise show twice in EPUB readers.
EPUB_CSS="$OUT_DIR/fiction_epub.css"
# shellcheck disable=SC2016
awk '
  BEGIN { skip = 0 }
  /@page/      { skip = 1; depth = 0 }
  skip == 1 {
    n = gsub(/\{/, "{");  depth += n
    m = gsub(/\}/, "}");  depth -= m
    if (depth <= 0 && index($0, "}") > 0) { skip = 0 }
    next
  }
  { print }
' "$CSS_SRC" > "$EPUB_CSS"
cat >> "$EPUB_CSS" <<'EOF'

/* EPUB-only — hide pandoc's auto-injected duplicate of the title. */
body > section.level1.unnumbered > h1.unnumbered:first-child { display: none; }
EOF

# EPUB: pandoc 3.9 `--split-level=N` splits at heading levels >= N,
# inclusive. With N=1 (default), only H1 boundaries split — and we
# only have one H1 (the auto-injected title-page header), so the
# entire body lands in a single content xhtml. Chapter H2s still
# populate the nav for reader navigation. This is the value that
# produces clean XHTML without splitting through my `<div>` wrappers.
# (Earlier we tried N=6 thinking that would mean "split only at H6";
# the docs make clear that's wrong — higher N means MORE splits.)
pandoc "$SRC" \
  --from=gfm+raw_html \
  --to=epub3 \
  --metadata=title:"$TITLE" \
  --metadata=lang:en \
  --metadata=creator:"[Author Name]" \
  --css="$EPUB_CSS" \
  --split-level=1 \
  --toc --toc-depth=2 \
  --output="$EPUB"
ls -la "$EPUB" | awk '{print "       " $5, $NF}'

# DOCX: pandoc silently drops `<div style="page-break-before">` in
# its OOXML writer (page-break CSS is not honoured). The portable
# fix is a raw OOXML page-break paragraph via the `raw_attribute`
# extension. We pre-process the manuscript to substitute the
# composer's HTML page-break markers with raw OOXML before the
# DOCX pass; HTML/PDF/EPUB stay untouched.
DOCX_SRC="$OUT_DIR/_docx_source.md"
"$PY" - "$SRC" "$DOCX_SRC" <<'PYEOF'
import sys, re
src, dst = sys.argv[1], sys.argv[2]
text = open(src).read()
# Substitute every page-break div with a raw OOXML paragraph that
# carries a single page-break run. Pandoc's `raw_attribute` extension
# preserves the literal OOXML in the docx output.
ooxml = "`<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>`{=openxml}"
text = re.sub(
    r'<div style="page-break-before:\s*always"></div>',
    ooxml,
    text,
)
open(dst, "w").write(text)
PYEOF
pandoc "$DOCX_SRC" \
  --from=gfm+raw_html+raw_attribute \
  --to=docx \
  --metadata=title:"$TITLE" \
  --metadata=author:"[Author Name]" \
  --toc --toc-depth=2 \
  --output="$DOCX"
ls -la "$DOCX" | awk '{print "       " $5, $NF}'

echo
echo "Done. Open the export folder:"
echo "  open \"$OUT_DIR\""
