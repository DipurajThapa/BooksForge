# Export & ePUB QA — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Test plan for the export pipeline.** Companion to `EXPORT_EPUB_SPEC.md` (architecture) and `TESTING_STRATEGY.md` (overall test posture).

This document is the QA contract for the export pipeline. Every check below has at least one CI job. The scaffolding lives in `crates/booksforge-test-fixtures/exports/` and `apps/desktop/src-ui/__tests__/preview/`.

---

## 1. The QA promise

A user clicks "Export to EPUB-3 (KDP-eBook)" on a fixture project and the resulting file:

1. **Validates** under EPUBCheck with zero errors and zero warnings.
2. **Looks the same** as the in-app preview (visual regression under documented tolerances).
3. **Reads correctly** in real EPUB readers (Apple Books, calibre, Adobe Digital Editions, KDP Previewer) — verified manually pre-release and via known fixture comparisons in CI.
4. **Reproduces byte-identically** across consecutive CI runs.
5. **Round-trips** — re-importing the exported EPUB into BooksForge restores the structure (chapters, metadata, images) without loss within documented tolerance.

If any of those fails, the PR fails.

## 2. Test fixtures

`crates/booksforge-test-fixtures/exports/` contains:

- **`tiny/`** — a 5,000-word, 3-chapter fixture. Used for fast unit tests.
- **`medium/`** — a 30,000-word, 12-chapter fixture with 6 images, 2 footnotes, 1 inline equation, a styled chapter epigraph. Used for golden-file regression.
- **`large/`** — a 100,000-word, 30-chapter fixture with 20 images, 40 footnotes, complex structure. Used for performance benches and end-to-end EPUBCheck.
- **`unicode/`** — a 5,000-word fixture in Spanish, Japanese (vertical text), Arabic (RTL), and English mixed. Tests Unicode handling.
- **`edge-cases/`** — a 1,000-word fixture with: very long paragraphs, deeply nested lists, an image at chapter end, a footnote that references another footnote, a wide table, an SVG figure, smart quotes inside `<code>`. Surfaces uncommon failures.

## 3. Required ePUB QA checks

The KDP-eBook profile must produce an EPUB whose content satisfies all of the following. Each check runs in CI.

### 3.1 Structural

| # | Check | Method |
|---|-------|--------|
| S1 | Valid `mimetype` (first entry, stored uncompressed) | `unzip -l` + byte inspection |
| S2 | Valid `META-INF/container.xml` pointing at `OEBPS/content.opf` | Parse + schema validate |
| S3 | Valid `content.opf` with required metadata (title, language, identifier, modified) | Parse + EPUBCheck |
| S4 | Valid `nav.xhtml` with hierarchical TOC | Parse + EPUBCheck |
| S5 | All spine items exist in manifest | `content.opf` cross-check |
| S6 | All manifest items exist in the ZIP | ZIP enumeration |
| S7 | No orphan files (every ZIP entry referenced or required by spec) | ZIP enumeration vs. manifest |
| S8 | Reading order matches document tree | `nav.xhtml` parsed and compared to fixture's expected order |

### 3.2 Content

| # | Check | Method |
|---|-------|--------|
| C1 | Title page renders with title, subtitle, authors | XHTML parse for known elements |
| C2 | Copyright page contains copyright statement and identifier | Regex on rendered XHTML |
| C3 | Dedication renders only when present | Conditional fixture comparison |
| C4 | Table of contents reflects parts → chapters → sub-headings | `nav.xhtml` parse |
| C5 | Chapter headings match the document tree titles | XHTML parse |
| C6 | Scene breaks render as the fixture's chosen separator (e.g., `* * *`) | XHTML parse |
| C7 | Paragraph spacing as specified by the template (em-based or px) | CSS-computed style probe |
| C8 | Indents render correctly per template (first-line vs. block) | CSS-computed style probe |
| C9 | Smart quotes, em-dashes, ellipses use Unicode characters (not entities) | Source text inspection |
| C10 | Footnotes/endnotes link bidirectionally via EPUB-3 `epub:type="noteref"` and `epub:type="footnote"` | XHTML parse + link verification |
| C11 | Internal links (cross-references) resolve to real anchors | Link traversal |
| C12 | Cover image present, correctly tagged with `properties="cover-image"` | Manifest inspection |
| C13 | Reading order in `spine` matches the document tree | Parse + compare |

### 3.3 Typography & layout

| # | Check | Method |
|---|-------|--------|
| T1 | Default font: serif specified by template; falls back to a stack including system serif | CSS-computed style probe |
| T2 | Headings use the template-specified hierarchy (h1 chapter, h2 section, etc.) | XHTML parse |
| T3 | Margins and line-height per template | CSS-computed style probe |
| T4 | No inline `style` attributes in rendered XHTML | XHTML parse |
| T5 | No `<script>` tags in rendered XHTML | XHTML parse |
| T6 | Image scaling: max-width 100%, height auto | CSS-computed style probe |
| T7 | Tables, where present, use simple structures with header rows | XHTML parse |

### 3.4 Metadata

| # | Check | Method |
|---|-------|--------|
| M1 | `dc:title`, `dc:creator`, `dc:language`, `dc:identifier` present | OPF parse |
| M2 | `dc:identifier` matches the project's manifest id | OPF parse + compare |
| M3 | `meta property="dcterms:modified"` present and ISO-8601 | OPF parse |
| M4 | KDP-recommended identifier scheme (e.g., uuid:…) | OPF parse |
| M5 | `dc:rights` populated when the project has a rights statement | OPF parse |
| M6 | Cover metadata `meta name="cover" content="cover-image"` | OPF parse (legacy compatibility) |

### 3.5 Accessibility (basic — full set is V1.0)

| # | Check | Method |
|---|-------|--------|
| A1 | `xml:lang` set on `<html>` of every chapter | XHTML parse |
| A2 | Every `<img>` has `alt` (empty string for purely decorative is ok with explicit `role="presentation"`) | XHTML parse |
| A3 | Heading hierarchy has no skips (h1 → h3 missing h2) within a chapter | XHTML parse |
| A4 | EPUB-3 semantics: `epub:type="frontmatter"`, `bodymatter`, `backmatter` on top-level sections | XHTML parse |

### 3.6 Compatibility

| # | Check | Method |
|---|-------|--------|
| K1 | EPUBCheck 5.x: zero errors, zero warnings on fixtures | Sidecar run, JSON parse |
| K2 | KDP Previewer dry-run (V1.0; manual pre-release in MVP) | Manual checklist |
| K3 | Apple Books opens fixture without "Could not open" errors | Manual pre-release |
| K4 | Adobe Digital Editions opens fixture | Manual pre-release |
| K5 | calibre `ebook-viewer` opens fixture | Manual pre-release |

In MVP, K2–K5 are part of a manual pre-release checklist (per `TESTING_STRATEGY.md §14`). In V1.0 we add headless KDP Previewer if Amazon offers it.

## 4. Preview-vs-export visual regression

The "downloaded EPUB doesn't match preview" pain is fixed structurally (canonical-HTML pipeline), but we still verify mechanically.

### 4.1 Setup

A Playwright test fixture mounts:

1. **Render A**: the in-app preview WebView showing the canonical HTML + canonical CSS for a fixture chapter.
2. **Render B**: the same WebView showing the **unzipped EPUB's** chapter XHTML + the EPUB's `style.css`.

Both renders use the same WebView engine (the OS's WebView2 / WebKit). Same viewport, same zoom, same fonts.

### 4.2 Comparison

For each fixture chapter:

1. Take a screenshot of Render A and Render B.
2. Pixel-diff using `pixelmatch` with a configured tolerance.
3. Tolerances:
   - Tiny / unicode fixture: ≤0.1% pixel difference. (Should be 0 in practice.)
   - Medium fixture: ≤0.2%.
   - Large fixture: ≤0.5%.
   - Edge-cases fixture: ≤1.0% (anti-aliasing on complex tables).
4. Any diff above tolerance fails the PR.

### 4.3 Rationale for non-zero tolerance

Even with byte-identical HTML/CSS, anti-aliasing and font hinting can produce trivial pixel differences. Tolerance is non-zero **only** to absorb that. **No structural diff** is acceptable: if a paragraph or image is positioned differently, the diff spikes and the test fails.

### 4.4 What the test catches

- Stylesheet drift between preview and export.
- Different HTML emitted (e.g., a recent change broke the canonical renderer).
- Asset path mismatches (image references break in one but not the other).
- Font-loading differences.

## 5. Golden-file regression

For the medium and large fixtures, CI runs:

1. Export → produces `out.epub`.
2. Hash the EPUB content using `blake3`. Hash the canonical HTML, the canonical CSS, the OPF, and the nav separately.
3. Compare against the committed golden hashes in `crates/booksforge-test-fixtures/exports/<fixture>/golden.toml`.
4. Mismatch fails the PR unless the PR includes a baseline-update commit and a written reason.

The point: a developer change cannot silently alter export output.

## 6. Round-trip test

A weak round-trip test runs in CI:

1. Export the medium fixture to EPUB.
2. Re-import the EPUB into a fresh BooksForge project.
3. Compare the imported document tree to the original fixture's document tree.

Tolerance: chapter titles, paragraph counts, image counts, footnote counts, and metadata must match. We do not require ProseMirror-perfect round-trip — EPUB import to ProseMirror is a lossy transformation by design.

## 7. EPUBCheck integration

`booksforge-epubcheck` (Layer 4) runs the bundled EPUBCheck JAR via the bundled jlink-stripped JRE. It:

1. Receives an EPUB file path.
2. Spawns the JRE process with `java -jar epubcheck.jar -j <out.json> <file.epub>`.
3. Parses the resulting JSON report.
4. Returns a typed `EpubCheckReport { errors: Vec<Issue>, warnings: Vec<Issue>, info: Vec<Issue> }`.
5. Errors block the export; warnings prompt; info is silent.

The JRE is bundled (~30 MB jlink-stripped). Fallback: if the user has Java installed and the bundled JRE fails for any reason, we try `java` from `PATH` as a recovery action.

## 8. Performance benches

| Bench | Fixture | Budget | Measured |
|-------|---------|--------|----------|
| EPUB export | medium | ≤ 5 s | CI |
| EPUB export | large | ≤ 30 s | CI |
| DOCX export | medium | ≤ 3 s | CI |
| DOCX export | large | ≤ 15 s | CI |
| PDF export (Trade 6×9) | medium | ≤ 8 s | CI |
| PDF export (Trade 6×9) | large | ≤ 45 s | CI |
| Preview update on save | medium | ≤ 800 ms p95 | dev-tools probe |
| EPUBCheck on medium | medium | ≤ 4 s | CI |

A regression > 10% on any bench fails the PR without a justification block.

## 9. CSS subset (the safe set)

The canonical CSS uses only these properties to maximise EPUB-reader compatibility:

**Allowed**

- `font-family`, `font-size`, `font-weight`, `font-style`, `text-align`, `line-height`, `letter-spacing`, `word-spacing`.
- `margin`, `padding`, `border`, `text-indent`, `text-decoration`.
- `color`, `background-color` (sparing).
- `display: block | inline | inline-block | flex | grid` (with progressive fallback).
- `width`, `height`, `max-width`, `max-height` (relative units).
- `hyphens`, `widows`, `orphans` (where supported).
- `page-break-before`, `page-break-after`, `break-before`, `break-after` for chapter starts.

**Disallowed in MVP**

- CSS animations, transitions.
- CSS variables (we resolve them at render time before emitting CSS).
- `@font-face` from external URLs (only embedded fonts).
- `position: absolute | fixed`.
- Floats (use Flexbox).
- Pseudo-elements beyond `::first-letter` and `::first-line`.

## 10. Failure-mode tests

Beyond happy paths, we systematically inject failures:

- Asset missing on disk → surfaced in pre-export gate; export refuses.
- Image dimensions exceed KDP cover requirements → surfaced as warning; user can fix.
- File size approaching KDP cap (50 MB MVP soft cap) → warning.
- EPUBCheck reports an error → export refused; report shown.
- Pandoc sidecar crashes → typed error; partial files cleaned up.
- Disk full mid-export → typed error; no partial files visible.
- User cancels mid-export → cancellation token; partial files cleaned; no `exports` row written.

Each has a CI test.

## 11. Manual pre-release checklist

Before tagging a release, a tester:

1. Builds the signed binaries on macOS and Windows.
2. Installs Ollama + a 7B model on a clean machine.
3. Creates a new project from each of the three MVP templates.
4. Drafts a 5,000-word fixture chapter.
5. Runs each MVP agent end-to-end; accepts/rejects proposals.
6. Exports to all four MVP profiles.
7. Opens each exported EPUB in Apple Books, calibre, and Adobe Digital Editions; verifies appearance matches preview.
8. Runs EPUBCheck on the exports; confirms zero errors / zero warnings.
9. Confirms no telemetry leakage with telemetry off.
10. Signs the release checklist.

A failed item blocks the release.

## 12. What this is not

- This is not the full V1.0 export QA. Apple Books / Kobo / Google Play / IngramSpark profiles are V1.0 with their own checklists.
- This is not a substitute for human reading. Authors should still read their EPUB on a real device before publishing.
- This is not a guarantee that every reader app renders identically — only that the canonical HTML and EPUB content match, and EPUBCheck passes. Reader-specific quirks remain reader-specific.

## 13. Acceptance criteria for this QA contract

The contract is acceptable when:

1. CI runs all checks in §3 on the medium fixture and they pass.
2. Visual regression (§4) passes on tiny, medium, and unicode fixtures.
3. Golden-file regression (§5) passes on medium and large fixtures.
4. Round-trip (§6) passes on medium fixture.
5. EPUBCheck (§7) reports zero errors / warnings on the KDP-eBook profile of every MVP fixture.
6. Performance budgets (§8) all met.
7. The manual pre-release checklist (§11) completes for the most recent release tag.
