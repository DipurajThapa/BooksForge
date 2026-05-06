# Phase 06 — Non-fiction & academic

## Goal

Bring V1.0-tier capability to Persona-B (Trade-author Theo) and Persona-C (Academic Aisha): tracked-changes round-trip with DOCX, native footnote/endnote rendering and numbering, citation engine with CSL styles + BibTeX import, math via KaTeX (editor) and LaTeX (export), tables with header rows and merged cells, cross-reference resolution, index generation, LaTeX export profile.

Add the IngramSpark-Print, Apple-Books-EPUB-3, and Academic-DOCX profiles. Add the genre/series-consistency validator that depends on the entity bible.

## Pre-conditions

MVP merged and beta-tested. Bug bar from beta closed.

## Inputs

1. `../_deep/02-FSD-functional-specifications.md` — sections 2 (FR-EDIT-008 tracked changes, 010–016), 6 (entity bible), 9 (export profiles).
2. `../_deep/04-data-model-and-project-format.md` — `references`, `bibliography`, `comments`, `tracked_changes`.
3. `../_deep/09-export-pipeline.md` — sections 4 (profiles), 5.3 (DOCX tracked changes), 5.4 (LaTeX).
4. `../_deep/12-risk-register.md` — R-04 specifically; this phase closes that risk.

## Deliverables

### 1. Citation engine

`booksforge-citation` (new sub-crate or module under `booksforge-domain`): CSL processor with at least the popular styles (Chicago author-date, Chicago notes-bibliography, APA 7, MLA 9, IEEE, Vancouver). BibTeX import; CSL-JSON import. Citations resolved at AST-build time per profile.

### 2. Footnote / endnote engine

Numbering policies (per page, per chapter, per book). Renumber on insert/delete. Editor renders footnotes inline with click-to-flyout; export renders per profile. Cross-references update.

### 3. Tracked changes round-trip

The hardest single piece of this phase. DOCX import preserves `<w:ins>`, `<w:del>`, `<w:moveFrom>`, `<w:moveTo>`, with author and date. Editor displays as marks. Accept/reject one / all / by author. Export re-injects into DOCX.

Round-trip test: load a real publisher-template DOCX with tracked changes, save, export, compare structurally with `pandoc-diff`-style comparator. Lossless on tested fixtures.

### 4. Math

KaTeX in the editor for inline `$...$` and display blocks. LaTeX raw passthrough on export (Pandoc handles). MathML on DOCX export.

### 5. Tables

TipTap table extension with header rows, alignment, merged cells. Pandoc round-trip. DOCX-table-style alignment.

### 6. Cross-reference engine

Cross-refs survive renumbering. Engine in `booksforge-domain`; renderer in `booksforge-export`.

### 7. Index generation

Manual index entries (mark a term as an index term with sub-entries). Index page generated at export time. LaTeX `\printindex` for academic profiles.

### 8. New profiles

- IngramSpark-Print PDF (PDF/X-1a:2001 via Pandoc → LaTeX → ghostscript or via specialised tool; ICC profile US Web Coated SWOP v2; bleed; embedded fonts).
- Apple-Books-EPUB-3 (Apple metadata schema; accessibility metadata mandatory).
- Academic-DOCX (Chicago author-date, Cambria 11pt, footnote style).
- LaTeX-Monograph (memoir or book class; per-template `.cls` selection).

### 9. Entity bible (FSD §6)

Implement entities table operations, auto-suggestion of entities from manuscript text (deterministic regex + token analysis; not AI), entity card UI, scene-appearance linking. Series-consistency validator: name-spelling drift, location-spelling drift, POV discipline. Genre-specific validators for the three templates (Romance Black Moment, Sci-Fi worldbuilding consistency).

### 10. Beta-reader and series-consistency AI features

`Beta-reader` preset: long-form critique, multi-pass, structured Markdown report. `Series-consistency` (hybrid deterministic + AI adjudication).

### 11. Tests

- Tracked-changes round-trip on ≥ 5 real publisher fixtures (Penguin Random House manuscript template, generic Big-Five spec).
- Citation tests for each CSL style.
- Index-generation correctness fixture.
- Academic monograph fixture round-trips DOCX losslessly (footnotes, citations, bibliography, headings).
- Performance: 80k-word academic export to LaTeX in ≤ 30 s on reference Mac.

## Guard-rails specific to this phase

**[GUARD-P6-1]** Tracked-changes round-trip is asserted with `pandoc-diff` structural equality, not just no-crash.

**[GUARD-P6-2]** Citation IDs are stable across renumbering and content edits.

**[GUARD-P6-3]** Index generation is deterministic.

**[GUARD-P6-4]** Math rendering is consistent across editor preview and export (KaTeX ↔ LaTeX semantic fidelity).

## Acceptance criteria

1. The R-04 risk closes: tracked-changes round-trip lossless on the fixture suite.
2. A 600-footnote academic monograph exports to LaTeX, compiles externally, produces correct PDF.
3. IngramSpark validator passes a print profile export.
4. Series-consistency validator flags injected drift in a fixture and lets the user accept the fix.

## Out of scope

- Plugin runtime (Phase 07).
- Linux installer signing (Phase 08).
- Encryption (Phase 09).

## When you finish

PR title `Phase 06: Non-fiction and academic`. Update `STATUS.md`. Phase 07 unblocked.
