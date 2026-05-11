# Book Formatting Taxonomy — Research-Backed

**Date:** 2026-05-09
**Sources:** Web research conducted today (KDP/IngramSpark/Apple/Google official 2026 guides + W3C EPUB-3.3 spec + Shunn Standard Manuscript Format).

This document is the canonical taxonomy of every formatting task BooksForge needs to handle to ship a book to any major channel. It maps every dimension to the BooksForge code that now implements it (or to the file you'd touch to implement it next).

---

## 1. The two orthogonal axes (locked)

| Axis | Type | Where in code | What it controls |
|---|---|---|---|
| **`FormatProfile`** | Genre × sub-genre typography | `crates/booksforge-domain/src/format_profile.rs` (1,263 lines, 30+ profiles) | Trim size, body/heading fonts, drop-cap, scene-break ornament, paragraph indent, line-height, drop-cap policy |
| **`PublishingTarget`** | Platform / storefront compliance | `crates/booksforge-domain/src/publishing_target.rs` (NEW, 10 targets) | Required artifact format (PDF/X-1a vs EPUB-3 vs DOCX), ISBN scheme, ToC depth cap, image DPI minimum, cover dims, font-embedding requirement, EPUBCheck pass requirement, accessibility metadata requirement |

The two compose: a user picks `RomanceContemporary` (typography) × `KdpKindle` (compliance) and the export pipeline composes both rules into one EPUB-3.

---

## 2. Publishing targets implemented

All ten supported targets, with the spec each one enforces:

| Target | Output | Trim sizes | ISBN | ToC ≤ | DPI ≥ | Cover ≥ | Fonts embedded? | PDF/X-1a? | EPUBCheck? | a11y? |
|---|---|---|---|---|---|---|---|---|---|---|
| **KDP Paperback** | PDF/X-1a | 13 sizes (5×8 → 8.5×11) | preferred | 3 | 300 | 1600×2560 | ✅ | ✅ | – | – |
| **KDP Hardcover** | PDF/X-1a | 4 sizes (5.5×8.5, 6×9, 6.14×9.21, 7×10) | preferred | 3 | 300 | 1600×2560 | ✅ | ✅ | – | – |
| **KDP Kindle** | EPUB-3 | – | (none) | 3 | 300 | 1600×2560 (1.6:1) | – | – | ✅ | – |
| **IngramSpark Print** | PDF/X-1a | 6 sizes | **required** | 3 | 300 | 1600×2560 | ✅ | ✅ | – | – |
| **IngramSpark eBook** | EPUB-3 (or 2) | – | **required** | 3 | 300 | 1600×2400 | – | – | ✅ | ✅ |
| **Apple Books** | EPUB-3 | – | **required (paid)** | 3 | 300 | 1400×2100 | – | – | ✅ | ✅ |
| **Google Play Books** | EPUB-3 (or PDF) | – | preferred (GGKEY otherwise) | 3 | 300 | 1400×2100 | – | – | ✅ | ✅ |
| **Kobo Writing Life** | EPUB-3 | – | preferred | 3 | 300 | 1400×2100 | – | – | ✅ | ✅ |
| **Shunn Manuscript** | DOCX | US Letter (8.5×11) | – | 0 (no ToC) | – | – | – | – | – | – |
| **Generic** | any | – | – | 6 | – | – | – | – | – | – |

Source-snapshots:
- KDP Paperback: [Amazon KDP Paperback Submission Guidelines](https://kdp.amazon.com/en_US/help/topic/G201857950) (gutter table, bleed 0.125", PDF/X-1a preferred).
- KDP Kindle: [Amazon KDP eBook Format Help](https://kdp.amazon.com/en_US/help/topic/G200634390) (MOBI rejected since 2025-03; EPUB-3 required; cover 1600×2560/1.6:1; from 2026-01-20 readers can download DRM-free Kindle books as EPUB/PDF).
- IngramSpark: [IngramSpark File Requirements](https://www.ingramspark.com/blog/file-requirements-for-ebooks) (separate ISBN per format; metadata-cover match).
- Google Play Books: [Google Play self-publish guide 2026](https://ghostwritingllc.com/blog/how-to-self-publish-a-book-on-google-play-books/) (ISBN preferred; GGKEY assigned otherwise).
- EPUB-3 a11y: [W3C EPUB 3.3 spec](https://www.w3.org/TR/epub-33/) and [EPUB 3 + WCAG accessibility guide](https://enabled.in/epub-3-and-wcag-building-truly-accessible-digital-books/).
- Shunn: [William Shunn — Standard Manuscript Format](https://www.shunn.net/format/classic/) and [Shunn — scene breaks](https://www.shunn.net/format/scene_breaks/).
- DOCX manuscript: [Wikipedia — Standard manuscript format](https://en.wikipedia.org/wiki/Standard_manuscript_format).

---

## 3. KDP Paperback gutter math (implemented)

Inside (gutter) margins scale with interior page count. The bands are now codified as a Rust function in `publishing_target.rs`:

```rust
pub fn kdp_paperback_gutter_inches(pages: u32) -> f32 {
    match pages {
        0..=150   => 0.375,
        151..=300 => 0.500,
        301..=500 => 0.625,
        501..=700 => 0.750,
        _         => 0.875,
    }
}
```

Outside margin: ≥ 0.25". Bleed: 0.125" all sides (when "with bleed" selected). Page size for bleed: trim + 0.25" height + 0.125" width.

Test coverage: `kdp_paperback_gutter_bands` exercises every band including the boundary values (150, 151, 300, 301, 500, 501, 700, 701).

---

## 4. Per-target formatting tasks (the full list)

This is the comprehensive list a working production export pipeline must perform. Status reflects the BooksForge codebase as of today.

### 4.1 Print PDF (KDP Paperback / Hardcover / IngramSpark)

| Task | Status | Notes |
|---|---|---|
| Trim-size selection from allowlist | ✅ codified in `TargetSpec.allowed_trims` | UI picker shows the list per-target |
| Page geometry (gutter + outer + top/bottom + bleed) | 🟡 `kdp_paperback_gutter_inches()` ready, not yet wired into Pandoc geometry args | Ticket: pass `gutter_inches` from selected target to `booksforge-export-pandoc::PandocInput` |
| Embed all fonts (subset, no-substitute) | 🟡 Pandoc + xelatex embeds by default; opt-out path exists | Verified for the seven Generic profiles; ticket to confirm for all 30 |
| Image min DPI (300) | ❌ no validator yet | Ticket: `R-EXP-IMG-DPI` validator in `booksforge-validator` |
| Headers / running titles | 🟡 partial via Pandoc template | Ticket: KDP-style "surname / title / page" header for Shunn target |
| Page numbers (suppress on chapter starts) | 🟡 partial | Ticket: ensure chapter pages start recto with no page number |
| ICC profile (US Web Coated v2 SWOP) | ❌ not embedded; PDF/X-1a conversion is manual | Acrobat Print Production handles this in MVP |
| PDF/X-1a:2001 final conversion | ❌ manual via Acrobat | Out-of-scope for V1; documented in UX proposal §H |
| Front matter (title / copyright / dedication / ToC / acks) | ✅ `FrontMatterPage` enum drives the order | Per-profile customization via `format_profile::spec().front_matter` |
| Drop caps | ✅ in CSS for EPUB and LaTeX for Pandoc | Per-profile `drop_cap: bool` flag |
| Scene breaks (ornament + Unicode glyph) | ✅ inline SVG per sub-genre + Unicode fallback | 30+ ornaments hand-curated in `format_profile.rs` |

### 4.2 EPUB-3 (Kindle / IngramSpark eBook / Apple / Google / Kobo)

| Task | Status | Notes |
|---|---|---|
| EPUB-3 packaging (mimetype + container.xml + OPF + nav.xhtml + toc.ncx) | ✅ `booksforge-export-epub` produces all of these deterministically | Pure Rust; no Pandoc dependency |
| Per-chapter XHTML files | ✅ via `manuscript_to_html_chapters` | Tested for byte-determinism |
| Stylesheet from genre × sub-genre typography | ✅ CSS factory in `booksforge-export-epub` | Tied to `FormatProfile` |
| `<dc:identifier>` scheme (urn:isbn vs urn:bf:project) | 🟡 Currently always `urn:bf:project:<ULID>`; ticket to honor `TargetSpec.identifier_scheme` | New ticket: thread `PublishingTarget` through the EPUB metadata builder |
| `dcterms:modified` | ✅ generated from manifest | |
| `rendition:layout=reflowable` | ✅ default | |
| ToC depth cap (≤3 for KDP/Apple/Google/Kobo) | ❌ no enforcement yet | Ticket: validator `R-EPB-TOC-DEPTH` reading `TargetSpec.toc_depth_max` |
| Page-list `<nav epub:type="page-list">` (when print edition exists) | ❌ not emitted | Ticket: opt-in for IngramSpark/Apple where print pagination matters |
| Landmarks `<nav epub:type="landmarks">` | ❌ not emitted | Ticket: required for Apple Books; recommended elsewhere |
| Accessibility metadata (`schema:accessMode`, `schema:accessibilityFeature`, `schema:accessibilityHazard`, `schema:accessibilitySummary`) | ❌ not emitted | Ticket: required for Apple/Google/IngramSpark; large but mechanical |
| `<meta property="schema:typicalAgeRange">` | ❌ not emitted | Optional everywhere |
| Cover image (`<item id="cover" properties="cover-image">`) | 🟡 ingestion path exists; cover validation pending | Ticket: `R-EPB-COV-DIMS` validator (per-target min dims + aspect) |
| Image alt text required | ❌ no enforcement | Ticket: validator that flags `<img>` without `alt` |
| Figure caption required (Google Books) | ❌ not enforced | Ticket: `R-EPB-IMG-CAP` for Google target |
| EPUB-safe CSS (no `position:fixed`, no JS, no `<iframe>`) | 🟡 stylesheet factory generates safe CSS by construction | Ticket: validator that scans manuscript-injected CSS |
| Embedded fonts with valid Adobe Embed Permission Bit | 🟡 Google Fonts bundle ships only OFL fonts (always embeddable) | Validator can short-circuit when the bundle is the source |
| EPUBCheck validation | 🟡 sidecar wiring in place; needs Java | UI banner now warns when Java is missing |

### 4.3 DOCX

| Task | Status | Notes |
|---|---|---|
| Pandoc-rendered DOCX with reference-doc styles | ✅ `export_via_pandoc` routes here | Per-profile reference-docx auto-generated |
| Auto-generated reference.docx from `FormatProfile` | ✅ deterministic per profile | Bytes-stable for caching |
| Heading styles (H1 chapter, H2 part, H3 section) | ✅ via reference doc | |
| Italic / bold / blockquote preserved | ✅ from ProseMirror via `pm_doc_to_markdown` → Pandoc | |
| Em-dashes / smart quotes per project StyleBook | ✅ enforced by copyeditor agent + StyleBook | Already ships |
| **Shunn-style manuscript DOCX** (TNR/Courier 12pt, double-spaced, half-inch indent, page header) | ❌ no profile yet | Ticket: new reference doc + special path; `PublishingTarget::ShunnManuscript` enum value already in place |

### 4.4 Markdown (no platform compliance)

Always works. No external deps. The lowest-common-denominator format and the universal handoff for further conversion. ✅ shipped.

---

## 5. What's now live in the UI (what a user sees)

After today's changes, opening the **Export** panel shows:

1. **Top banner** (only when needed): "Some export formats need extra software" — lists missing Pandoc / Java / EPUBCheck JAR with one-line install hints.
2. **Choose profile** (radio cards): markdown · generic_epub · kdp_ebook · docx · trade_pdf_5x8 · trade_pdf_6x9.
3. **Publishing target** (dropdown + briefing): 10 targets (KDP Paperback, KDP Kindle, KDP Hardcover, IngramSpark Print, IngramSpark eBook, Apple Books, Google Play Books, Kobo, Shunn Manuscript, Generic). Each pick shows:
   - The user-facing briefing copy explaining what the platform's spec requires.
   - Tags showing accepted formats, ToC depth cap, image DPI minimum, cover dims, ISBN scheme.
   - Allowed trim sizes for that target.
4. **Genre / typography** (genre × sub-genre cascade): unchanged from before — 30+ profiles.
5. **Pre-flight validators** (existing): block on errors, warn-and-continue otherwise.
6. **Export button** + saved-history list.

The Ollama wizard now **auto-launches on first start** when Ollama isn't running — closing the audit's "biggest blocker" for non-developers.

---

## 6. Status of every gap from prior audits

### From the readiness audit (the 8-ticket cutting-room list)

| # | Gap | Status | Where |
|---|---|---|---|
| 1 | Auto-launch Ollama Wizard on first start | ✅ **DONE** | `App.tsx:49` (`sessionStorage` flag, fires once per session) |
| 2 | Wire `agent_run_chapter_drafter` Tauri command | 🟡 deferred — falls through to GenericAgentForm | Audit estimated ~50 lines; out of scope for this turn |
| 3 | Build `ChapterDrafterPanel` component | 🟡 deferred — uses GenericAgentForm | Same |
| 4 | Add export-dependencies banner | ✅ **DONE** | `ExportPanel.tsx:153` (calls `exportCheckDependencies` on mount; renders banner when any required bin missing) |
| 5 | Wire export-check-dependencies to UI | ✅ **DONE** | (covered by #4) |
| 6 | Add Settings → Models page | 🟡 audit lists this; existing `OllamaWizard` covers model swap. Discrete page is a UX polish item |
| 7 | Add refinement-loop orchestration | 🟡 deferred — Python pipeline already chains drafter → polish → humanize → FRE; UI orchestration ticket open |
| 8 | Template preview descriptions | 🟡 deferred — UX polish item |

### From the user-workflow report (3 known gaps)

| Gap | Status |
|---|---|
| EPUBCheck validation skipped (Java not installed) | 🟡 banner now tells the user; install command provided |
| PDF export needs xelatex | 🟡 banner now tells the user when Pandoc is missing; xelatex remains a separate install |
| 27B FRE memory pressure on 32 GB Mac | 🟡 documented; user-controlled (use 27B FRE instead of 36B for tighter memory) |

### From the local-LLM pipeline findings

| Gap | Status |
|---|---|
| `chapter-drafter-nf` non-fiction template | ✅ **DONE** earlier |
| `final-polish-merge` template | ✅ **DONE** earlier |
| Per-call `think` flag in `booksforge-ollama` | ✅ **DONE** earlier |
| `DefaultThinking` per-agent binding | ✅ **DONE** earlier |
| Hardened `extract_json` w/ balance-prefix repair | ✅ **DONE** earlier |
| Attempt-3 budget escalation (5× + json_mode-off) | ✅ **DONE today** | `booksforge_full_pipeline.py` |
| Outline chapter-count compliance retry | 🟡 open — orchestrator state machine |
| Coverage-recovery re-roll | 🟡 open — orchestrator state machine |
| `chapter-drafter` vs `-nf` mode dispatch | 🟡 open — small follow-up in the agent-binding code path |

### From the formatting research (NEW gaps surfaced today)

| Gap | Effort | File |
|---|---|---|
| Thread `PublishingTarget` into export pipeline (use `identifier_scheme`, `toc_depth_max`, `cover_min_px` per target) | ~150 lines | `apps/desktop/src/commands/export.rs` + `crates/booksforge-export-epub/src/lib.rs` |
| `R-EPB-TOC-DEPTH` validator per `TargetSpec.toc_depth_max` | ~40 lines | `crates/booksforge-validator/src/validators.rs` |
| `R-EPB-COV-DIMS` validator (cover ≥ min dims + aspect) | ~50 lines | same |
| `R-EXP-IMG-DPI` validator (image ≥ 300 DPI for print targets) | ~80 lines | same; needs image-decoding crate (`image` is small) |
| EPUB-3 accessibility metadata block (`schema:accessMode`, `accessibilityFeature`, `accessibilityHazard`, `accessibilitySummary`) | ~100 lines | `crates/booksforge-export-epub/src/metadata.rs` |
| EPUB-3 landmarks `<nav epub:type="landmarks">` | ~60 lines | same crate |
| EPUB-3 page-list (when print edition exists) | ~80 lines | same crate |
| Image alt-text validator (`<img>` without `alt`) | ~30 lines | validator crate |
| Shunn-style DOCX reference-docx + path | ~100 lines | new `reference-shunn.docx` + branch in `export_via_pandoc` |
| Auto-pre-flight by target (run only the validators that target requires) | ~60 lines | `apps/desktop/src/commands/validators.rs` |

---

## 7. The four levels of compliance

Useful framing for what the user can ship today:

| Level | What works today | What you ship |
|---|---|---|
| **Level 0 — Local file** | Markdown, raw EPUB-3 (Pandoc), raw DOCX (Pandoc) | An EPUB you can sideload, a DOCX you can email |
| **Level 1 — Storefront-acceptable** | KDP Kindle (Pandoc EPUB-3 with reasonable defaults) | Will likely pass KDP upload; may flag warnings |
| **Level 2 — Storefront-compliant** | Most validators present in `booksforge-validator`; pre-flight gate on errors | Passes upload cleanly with no warnings |
| **Level 3 — Spec-perfect** | All accessibility metadata + landmarks + page-list + ICC profiles + PDF/X-1a auto-conversion | Apple Books / EU Accessibility Act compliant |

**Today BooksForge is between Level 1 and Level 2.** The publishing-target picker now exposes *what* Level 2/3 require, but the validator + EPUB-builder work to *enforce* every requirement is the open ticket list above.

---

## Sources

- [Amazon KDP Paperback Submission Guidelines](https://kdp.amazon.com/en_US/help/topic/G201857950)
- [Amazon KDP eBook Format Help](https://kdp.amazon.com/en_US/help/topic/G200634390)
- [Inkfluence AI: KDP Paperback Complete Formatting Guide 2026](https://www.inkfluenceai.com/learn/paperback-publishing-kdp-print)
- [BookBeam: KDP Formatting Requirements 2026](https://bookbeam.io/blog/kdp-formatting-requirements/)
- [BookClad: Amazon KDP Book Cover Requirements 2026](https://bookclad.com/blog/amazon-kdp-cover-requirements-2026)
- [iLayoutBooks: Amazon KDP eBook Cover Recommended Dimensions 2026](https://ilayoutbooks.com/amazon-kdp-ebook-cover-recommended-dimensions-2026-guide/)
- [Holograph PressWorks: How to Format a Book for Amazon KDP 2026](https://www.holographpressworks.com/blog/how-to-format-book-amazon-kdp-2026)
- [IngramSpark: File Requirements to Publish and Distribute eBooks](https://www.ingramspark.com/blog/file-requirements-for-ebooks)
- [IngramSpark File Creation Guide](https://www.ingramspark.com/hubfs/downloads/file-creation-guide.pdf)
- [Apple iTunes Producer Asset Guide — Navigation Document](https://help.apple.com/itc/booksassetguide/en.lproj/itc0f175a5b9.html)
- [Ghostwriting LLC: How to Self-Publish on Google Play Books 2026](https://ghostwritingllc.com/blog/how-to-self-publish-a-book-on-google-play-books/)
- [W3C: EPUB 3.3](https://www.w3.org/TR/epub-33/)
- [W3C: EPUB Accessibility Techniques 1.1](https://www.w3.org/TR/epub-a11y-tech-11/)
- [DAISY KB: Landmarks](https://kb.daisy.org/publishing/docs/navigation/landmarks.html)
- [Enabled.in: EPUB 3 and WCAG — Building Truly Accessible Digital Books](https://enabled.in/epub-3-and-wcag-building-truly-accessible-digital-books/)
- [W3C: EPUBCheck](https://www.w3.org/publishing/epubcheck/)
- [GitHub: w3c/epubcheck](https://github.com/w3c/epubcheck)
- [Wikipedia: Standard manuscript format](https://en.wikipedia.org/wiki/Standard_manuscript_format)
- [William Shunn: Classic Manuscript Format](https://www.shunn.net/format/classic/)
- [William Shunn: Scene Breaks](https://www.shunn.net/format/scene_breaks/)
- [Scribophile: Manuscript Format How to Format a Novel 2026](https://www.scribophile.com/academy/manuscript-format-how-to-format-a-novel-with-examples)
- [MasterClass: How to Format a Book Manuscript 2026](https://www.masterclass.com/articles/how-to-format-a-book-manuscript)
