# How to view the BooksForge exports

The BF-E2E-LOCAL-LLM-FIRST-BOOK-001 test produced a complete book bundle here:

```
/Users/dipurajthapa/Work/AIProjects/BooksForge/book-output/booksforge-e2e-bf001/exports/
├── manuscript.pdf            240 KB    ← print-ready PDF (typst, US-letter)
├── manuscript.epub            48 KB    ← reflowable EPUB-3 (EPUBCheck PASS)
├── manuscript.docx            40 KB    ← editable Word document
├── manuscript.print.html      82 KB    ← 6×9 print-CSS HTML preview
├── manuscript.source.md       75 KB    ← raw markdown source
├── kindle_preview_emulation.html       ← in-browser Kindle-style preview
├── metadata.json + metadata.kdp.csv    ← KDP-ready metadata package
└── epubcheck.report.txt                ← validation report (0 errors)
```

## Quickest one-liner to open everything

```bash
open /Users/dipurajthapa/Work/AIProjects/BooksForge/book-output/booksforge-e2e-bf001/exports/
```

This pops a Finder window. Double-click any file.

## Per-format opener (terminal)

```bash
cd /Users/dipurajthapa/Work/AIProjects/BooksForge/book-output/booksforge-e2e-bf001/exports

# PDF — opens in Preview
open manuscript.pdf

# DOCX — opens in Word / Pages
open manuscript.docx

# EPUB — macOS opens in Apple Books by default
open manuscript.epub

# Print-CSS HTML preview — opens in your default browser
open manuscript.print.html

# Kindle-style preview emulation
open kindle_preview_emulation.html
```

## EPUB readers worth installing if Apple Books feels too thin

| Reader | Install | Why |
|---|---|---|
| **Calibre** | `brew install --cask calibre` | The desktop standard. Best for inspecting EPUB internals, converting, validating. |
| **Thorium Reader** | `brew install --cask thorium` | Modern, clean, accessible. Better typography than Apple Books. |
| **Kindle Previewer 3** | Download from [Amazon's KDP portal](https://kdp.amazon.com/en_US/help/topic/G202131170) | The ONLY authoritative way to see what the book will look like on Kindle hardware. The emulator HTML in the bundle is a local approximation; Kindle Previewer is the real thing — install it before any KDP submission. |

## "I want to see the prose, not the file"

The cleanest plain-text source is in the markdown:

```bash
less /Users/dipurajthapa/Work/AIProjects/BooksForge/book-output/booksforge-e2e-bf001/exports/manuscript.source.md
```

Or, for the per-chapter raw drafts as the pipeline produced them:

```bash
ls /Users/dipurajthapa/Work/AIProjects/BooksForge/artifacts/
# 14_final_manuscript.md is the rated/optimised version (15,551 words, 8 chapters)
# 12_polished_manuscript.md is after polish but before final-rating revision
# 08_draft_manuscript.md is the raw 9B drafter output
```

Diff any two stages to see what each polish round actually did:

```bash
diff /Users/dipurajthapa/Work/AIProjects/BooksForge/artifacts/12_polished_manuscript.md \
     /Users/dipurajthapa/Work/AIProjects/BooksForge/artifacts/14_final_manuscript.md \
  | less
```

## "Why isn't this in the BooksForge UI?"

The Tauri app *does* have an Export panel — that's the supported in-product path. The BF-E2E test ran the **Python driver**, not the UI, so the exports landed on disk directly instead of through the desktop app's export dialog. Both paths produce the same artefact format; the UI just hides the file picker behind a button.

To exercise the in-product export:
1. `cd booksforge && cargo tauri dev` (builds + runs the desktop app)
2. Open or create a project
3. Click **Export** in the toolbar
4. Choose formats; the dialog asks where to save

(The same EPUBCheck warning surfaces; install `brew install epubcheck` if you want validation in-loop.)
