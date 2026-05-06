# Export Pipeline — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

> **Status note (2026-05-06):** This document is **partially superseded** for ePUB by `../EXPORT_EPUB_SPEC.md` and `../EXPORT_EPUB_QA.md` per `[DECISION-017]`. EPUB-3 now uses a **canonical-HTML pipeline** where the editor preview HTML is the export source; Pandoc is no longer used for EPUB. This document remains **authoritative for DOCX and PDF** export profiles, font subsetting, asset handling, and the broader pipeline mechanics that apply across formats. Where this document and the canonical-HTML spec disagree on EPUB specifically, the canonical-HTML spec wins for MVP.

---

## 1. Goals

The export pipeline transforms the canonical project state into store-validated DOCX, PDF, EPUB-3, LaTeX, Markdown, and HTML outputs. It must be **reproducible** (same inputs → same bytes), **profile-driven** (one project, many target stores with different rules), **correct** under the formatting requirements of major stores (KDP, IngramSpark, Apple Books, academic presses), and **auditable** (every export records its inputs and validators).

## 2. Pipeline shape

```
Project State
    │
    ▼
[1] Canonical BooksForge-AST builder
    │  (resolves cross-refs, citations, footnote numbering, language tags,
    │   embeds asset references, applies tracked-change accept policy)
    ▼
[2] Profile resolution
    │  (template + target + post-processors)
    ▼
[3] Pre-flight validators (target-specific)
    │  → may abort on errors per user setting
    ▼
[4] BooksForge-AST → Pandoc-AST JSON transformer
    │
    ▼
[5] Pandoc sidecar (spawned process, GPL-isolated)
    │  pandoc --from=json --to=<format> ...
    ▼
[6] Format-specific post-processors
    │  EPUB: epubcheck + fix, font subsetting, cover insertion
    │  PDF:  bleed/trim, font embedding, ICC profile
    │  DOCX: re-attach tracked changes, comment IDs, custom XML
    ▼
[7] Final validation pass
    │
    ▼
[8] Write to exports/, record `exports` row
```

Each step is a typed function with strict inputs/outputs, runs in the Rust sidecar, and is independently testable.

## 3. Pandoc as a sidecar — [DECISION-005] reaffirmed

We invoke Pandoc as a separate OS process via stdin/stdout JSON. We do **not** statically link or dynamically link Pandoc into the BooksForge binary. This is the standard interpretation of GPL: spawning a GPL binary as a separate process is "mere aggregation" and does not impose the GPL on the host. The Pandoc binary is bundled in the installer alongside the BooksForge binary, with its license text shipped at `licenses/pandoc/COPYRIGHT`.

We pin the Pandoc version per app release. The export row records the Pandoc version so we can reproduce.

The transformer (step 4) builds a Pandoc-AST JSON document with embedded metadata (title, authors, language, identifiers), mainmatter, frontmatter, backmatter, footnotes, citations, raw blocks for things Pandoc can't represent (we fence those carefully), and the resource path is set to a temp directory with all referenced assets copied/symlinked in.

## 4. Export profiles

A *profile* is a named composition of (template, target format, post-processors, validator set). We ship a curated set; users can edit (V1.5, FR-EXP-007).

Built-in profiles include:

**Manuscript-DOCX (industry standard):** double-spaced, Times New Roman 12pt, 1-inch margins, page numbers, header with author/title, scene breaks marked with `#`, no fancy styling. The shape any agent or editor expects to receive.

**KDP-Print PDF:** trim size from project metadata (e.g., 6×9), 0.125-inch bleed on cover only (interior is no-bleed), embedded fonts with subset, ToC with hyperlinks, ISBN on copyright page, section breaks at chapter starts on right-hand page (recto).

**KDP-eBook EPUB-3:** EPUB-3 with reflowable layout, semantic markup (chapter `<section>` elements), embedded cover, embedded fonts (license-permitted), table of contents in nav doc, alt text on every image, KDP-specific cover dimensions (1600×2560).

**IngramSpark-Print PDF:** stricter than KDP — bleed must match exactly, ICC profile (US Web Coated SWOP v2), 1200 dpi rasterisation, ISBN matches assigned, no transparency in PDF/X-1a:2001 export.

**Apple-Books EPUB-3:** Apple's stricter EPUB-3 profile, fixed metadata schema, accessibility metadata mandatory.

**Kobo-EPUB-3, Google-Play-EPUB-3:** similar EPUB-3 with platform tweaks.

**Academic-DOCX (Chicago):** Chicago author-date or notes-bibliography (per template), Cambria 11pt, footnote style, bibliography section, Word native footnotes (not endnote XML), paragraph styles named to match a typical university press's manuscript spec.

**LaTeX-Monograph:** outputs a `.tex` file plus `.bib` (BibTeX) plus `figures/` directory. Class file driven by template (`memoir`, `book`, custom press class). User compiles externally or uses our optional integrated `latexmk` runner.

**Markdown-Bundle:** zip with `manuscript.md`, `assets/`, `bibliography.bib`. For users who want to live in a different toolchain.

## 5. Format specifics

### 5.1 EPUB-3

Pandoc produces a baseline EPUB-3. Post-processors:

The cover is inserted from a project asset with the right `epub:type="cover"` semantic. Fonts are subsetted by `pyftsubset` (or a Rust equivalent like `subset-font`) to embedded glyphs only — keeps file size down and avoids licensing issues with full-font shipping. `epubcheck` runs at the end; warnings logged, errors blocking. Accessibility metadata (`schema:accessibilityFeature`, etc.) added per project metadata. ToC depth limited per profile (KDP wants ≤ 4 levels). Reading order verified.

### 5.2 PDF

Pandoc → LaTeX (or Typst, if `--engine=typst` is selected — see DECISION below) → PDF.

[DECISION-007] PDF engine: **Default to Typst when available, fallback to LaTeX (xelatex)**. Typst is faster, has cleaner errors, and is becoming Pandoc-supported. LaTeX remains the fallback because some templates (academic presses) require .tex source. Users can force LaTeX per profile.

Post-processors: font embedding verified (no system-font references that would break on a different machine); ICC profile attached for print profiles; PDF/X compliance for IngramSpark; bleed and trim marks where required; PDF/A-2b for academic press optional.

### 5.3 DOCX

Pandoc produces a clean DOCX. Post-processors:

Tracked changes are re-attached: Pandoc's DOCX writer doesn't preserve our internal change set, so we run a small post-pass using `python-docx` or a Rust DOCX library to inject `<w:ins>` / `<w:del>` elements with author and date. Comments similarly. Custom XML parts hold BooksForge-specific metadata (project id, snapshot id) so we can later ingest the same DOCX losslessly.

### 5.4 LaTeX

Direct output via Pandoc's LaTeX writer with our class file selection. We bundle `latexmk` as an optional sidecar for one-click compile. References go to `bibliography.bib` (BibTeX) regardless of project's CSL — the LaTeX writer is BibTeX-native.

## 6. Asset handling

The transformer copies referenced assets from the bundle's content-addressed `assets/` directory to a temp work directory with stable names (e.g., `figure-001.png`). Image scaling, format conversion (e.g., HEIC → PNG), and DPI normalisation happen here:

- KDP-Print: minimum 300 DPI at print size, force PNG/JPEG, RGB → CMYK is *not* required (KDP converts).
- IngramSpark-Print: minimum 300 DPI at print size, CMYK with ICC profile.
- EPUB: maximum 1600 px on long edge to control file size, JPEG quality 85, alpha preserved as PNG.

Equation rendering: KaTeX in EPUB/HTML, native LaTeX in PDF/LaTeX, MathML in DOCX (Word renders MathML).

## 7. Fonts and licensing

BooksForge ships a curated set of license-permissive fonts (SIL Open Font License or similar): EB Garamond, Source Serif, Atkinson Hyperlegible, Inter, JetBrains Mono. Templates pick from this set by default, ensuring exports embed legally-clear fonts.

Users can use any system font for **on-screen** display. For **export embedding**, the engine warns if a font's licence is restrictive, and offers to substitute. This avoids the trap where a user's Word-style document references "Calibri" and the EPUB ends up with a system-font fallback on the reader's device.

## 8. Reproducibility

Same inputs (canonical AST hash + template version + Pandoc version + post-processor versions + profile) must produce **byte-identical** output. To achieve this:

We canonicalise the BooksForge-AST (sorted keys, no insertion timestamps, deterministic node IDs). We pin Pandoc and tool versions per release. We use `--metadata-file` with a fixed timestamp when reproducibility mode is on. Random seeds for any randomised step (e.g., font subsetter ordering) are fixed.

CI has a reproducibility test: build the same project twice, hash outputs, compare. This is the single most effective regression detector for the export pipeline.

## 9. Validators (export-time)

Profile-specific validators run pre-export and post-export:

**Pre-export.** ISBN format if assigned, copyright page presence, ToC depth ≤ profile limit, all images have alt text, no broken cross-refs, no orphan footnotes, language tag present, manuscript word count consistent with metadata.

**Post-export.** epubcheck for EPUB; PDF/X validation for IngramSpark; DOCX schema validation; file-size ≤ store limit; embedded fonts list inspection.

## 10. Performance

Targets (FR-EXP-004 reference hardware): EPUB-3 export of a 100k-word project with 20 images ≤ 30 s end-to-end. PDF export ≤ 60 s. DOCX export ≤ 20 s. Caching: the transformer step caches AST-by-scene; only changed scenes are re-transformed. Pandoc is invoked once per export, not per scene.

## 11. Failure handling

Each step emits typed errors. Pandoc non-zero exit captures stderr and surfaces a structured error linked to the step. On post-processor failure (e.g., epubcheck reports an unrecoverable issue) the pipeline aborts with a clear message including a "save partial output for inspection" affordance.

## 12. User-visible UX

The export dialog is a wizard: pick profile → review preflight (validators) → confirm → progress with steps. The progress UI is granular: user sees "Building canonical AST" → "Validating manuscript" → "Calling Pandoc" → "Subsetting fonts" → "Running epubcheck" → "Done". A "Reveal in Finder/Explorer" link on completion. An "Export log" panel for advanced inspection.

## 13. Custom export profiles (V1.5)

A profile editor lets advanced users compose: template + target + override style rules (per-element overrides) + validator set. Profiles can be exported/imported as TOML files, and shared via plugins.

## 14. Audiobook export (V2.0)

Speculative: route the canonical AST through a TTS engine (XTTS, OpenVoice, or cloud) and produce M4B chapters with chapter markers. Voice cloning is opt-in only with the user's recorded voice and stored locally. Out of V1.x scope; sketched here for architectural runway.

## 15. Open issues / backlog

epubcheck is Java — bundling a JRE is heavy. Two options being evaluated: bundling a small JRE (~50 MB), or porting the relevant epubcheck rules to Rust (months of work). Default for V1.0: bundle a small JRE.

PDF/X compliance for IngramSpark is fiddly across platforms; we will validate on each OS in CI before V1.0 GA.

DOCX tracked-changes round-trip needs an extensive fixture library (drawn from real publisher templates) — see Test Strategy §6.
