#!/usr/bin/env bash
# Fetch the BooksForge Google Font bundle (BACKLOG §H8.2 follow-up).
#
# Downloads the curated 9-family book typography bundle from the
# `google/fonts` GitHub mirror (Apache 2.0 / SIL OFL — repo licences
# are checked in via `LICENSES.txt` after this script runs).
#
# We pull variable-weight TTFs where available (one file covers
# 100–900 weights + roman + italic for many families), and the
# static `Regular` + `Italic` files where the family is static-only.
#
# Re-running is idempotent: existing files are skipped unless their
# size is zero (failed mid-download).
#
# Output layout:
#   apps/desktop/resources/fonts/<Family>/<File>.ttf
#
# This directory is wired into `tauri.conf.json` as a bundled
# resource so the Tauri builder ships the fonts with the app.  The
# fonts are also embedded into EPUB exports by
# `booksforge-export-epub` and referenced by `xelatex` via the
# Pandoc `mainfont` variable.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/apps/desktop/resources/fonts"
BASE="https://raw.githubusercontent.com/google/fonts/main"

mkdir -p "$DIR"

# Format: "Family|relative/path/in/google-fonts-repo"
# %5B / %5D are the URL encoding for [ ].
FILES=(
  # Variable-weight families — one file covers the full 100–900 range.
  "EB_Garamond|ofl/ebgaramond/EBGaramond%5Bwght%5D.ttf"
  "EB_Garamond|ofl/ebgaramond/EBGaramond-Italic%5Bwght%5D.ttf"

  "Crimson_Pro|ofl/crimsonpro/CrimsonPro%5Bwght%5D.ttf"
  "Crimson_Pro|ofl/crimsonpro/CrimsonPro-Italic%5Bwght%5D.ttf"

  "Lora|ofl/lora/Lora%5Bwght%5D.ttf"
  "Lora|ofl/lora/Lora-Italic%5Bwght%5D.ttf"

  "Source_Serif_4|ofl/sourceserif4/SourceSerif4%5Bopsz,wght%5D.ttf"
  "Source_Serif_4|ofl/sourceserif4/SourceSerif4-Italic%5Bopsz,wght%5D.ttf"

  "Vollkorn|ofl/vollkorn/Vollkorn%5Bwght%5D.ttf"
  "Vollkorn|ofl/vollkorn/Vollkorn-Italic%5Bwght%5D.ttf"

  "Playfair_Display|ofl/playfairdisplay/PlayfairDisplay%5Bwght%5D.ttf"
  "Playfair_Display|ofl/playfairdisplay/PlayfairDisplay-Italic%5Bwght%5D.ttf"

  "Inter|ofl/inter/Inter%5Bopsz,wght%5D.ttf"
  "Inter|ofl/inter/Inter-Italic%5Bopsz,wght%5D.ttf"

  "Source_Sans_3|ofl/sourcesans3/SourceSans3%5Bwght%5D.ttf"
  "Source_Sans_3|ofl/sourcesans3/SourceSans3-Italic%5Bwght%5D.ttf"

  "Cormorant_Garamond|ofl/cormorantgaramond/CormorantGaramond%5Bwght%5D.ttf"
  "Cormorant_Garamond|ofl/cormorantgaramond/CormorantGaramond-Italic%5Bwght%5D.ttf"
)

# Per-family LICENSE files (SIL OFL or Apache 2.0).  Fetched once per
# family so the bundle ships with proper attribution.
LICENSES=(
  "EB_Garamond|ofl/ebgaramond/OFL.txt"
  "Crimson_Pro|ofl/crimsonpro/OFL.txt"
  "Lora|ofl/lora/OFL.txt"
  "Source_Serif_4|ofl/sourceserif4/OFL.txt"
  "Vollkorn|ofl/vollkorn/OFL.txt"
  "Playfair_Display|ofl/playfairdisplay/OFL.txt"
  "Inter|ofl/inter/OFL.txt"
  "Source_Sans_3|ofl/sourcesans3/OFL.txt"
  "Cormorant_Garamond|ofl/cormorantgaramond/OFL.txt"
)

fetch() {
  local family="$1"
  local path="$2"
  local outfile
  outfile="$DIR/$family/$(basename "${path//%5B/[}" | sed 's/%5D/]/g; s/%2C/,/g')"
  mkdir -p "$DIR/$family"
  if [ -s "$outfile" ]; then
    echo "  ✓ $family/$(basename "$outfile") (cached)"
    return
  fi
  echo "  ⤓ $family/$(basename "$outfile")"
  curl -fsSL "$BASE/$path" -o "$outfile"
}

echo "Fetching BooksForge font bundle into $DIR"
echo "(re-runs skip files already present)"

for entry in "${FILES[@]}"; do
  IFS='|' read -r family path <<< "$entry"
  fetch "$family" "$path"
done

echo ""
echo "Fetching font licences"
for entry in "${LICENSES[@]}"; do
  IFS='|' read -r family path <<< "$entry"
  outfile="$DIR/$family/LICENSE.txt"
  if [ -s "$outfile" ]; then continue; fi
  curl -fsSL "$BASE/$path" -o "$outfile"
done

echo ""
echo "Done.  Total bundle size:"
du -sh "$DIR" | awk '{ printf "  %s\n", $1 }'
echo ""
echo "Bundle is ready.  The EPUB packager + Pandoc PDF runner will"
echo "auto-detect these and embed/reference them on next export."
