# Export & ePUB Pipeline — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for the export pipeline.** Companion to `EXPORT_EPUB_QA.md` (test plan).

This document specifies the export-pipeline architecture (DOCX, PDF, EPUB-3) and the canonical-HTML invariant that keeps the in-app preview byte-identical to the downloaded EPUB.

---

## 1. The pipeline rule (`[DECISION-017]`)

**The editor preview HTML is the export source for EPUB-3.** The same canonical HTML — produced by `booksforge-export` from the document tree — is rendered in:

1. The editor's **preview** view (a WebView pane sharing the canonical CSS).
2. The packaged EPUB-3 (`booksforge-export-epub` zips the canonical HTML with the canonical CSS into a valid EPUB-3 container).

Pandoc handles **DOCX and PDF only**, where its strengths (paragraph styling, footnote rendering, page layout) outweigh the drift risk.

The canonical-HTML pipeline structurally eliminates preview-vs-export drift: anything that looks wrong in the EPUB also looks wrong in the preview, and the user fixes it once.

### 1.1 Invariants this rule enforces

- The preview pane and the EPUB content render **byte-identical HTML** with **byte-identical CSS**. Visual regression in CI catches drift.
- The packager (`booksforge-export-epub`) zips the canonical bytes; it does not re-render or transform.
- Pandoc never touches EPUB.
- The CSS uses only the subset that EPUB-3 readers reliably support (specifics in `EXPORT_EPUB_QA.md §9`).
- Custom Node or Python EPUB libraries are not used (the Rust-native packager keeps the architecture single-runtime; no external script runtimes inside the export path).

## 2. Pipeline architecture

```
Document tree (SQLite + Markdown mirror)
        │
        ▼
booksforge-export (Layer 3, pure)
   ├── tree → Canonical Document
   ├── Canonical Document → Canonical HTML
   └── Canonical Document → Pandoc-AST  (only for DOCX/PDF)
        │
        ├──► booksforge-export-epub (Layer 4)  ── packages → EPUB-3
        │                                              │
        │                                              ▼
        │                                       booksforge-epubcheck ── validates
        │
        ├──► booksforge-export-pandoc (Layer 4) ─► Pandoc sidecar ──► DOCX
        │                                                        ──► PDF
        │
        └──► Editor preview (WebView in apps/desktop/src-ui)
                renders the same Canonical HTML with the same Canonical CSS
```

**Key invariant.** The Canonical HTML and Canonical CSS that the preview renders are **byte-identical** to the HTML/CSS inside the EPUB's content directory. The only difference between the preview and the EPUB is the EPUB packaging (META-INF, container.xml, OPF manifest, NCX/nav).

## 3. Canonical Document

`booksforge-export` defines a **Canonical Document** — a typed, intermediate representation of a book ready for rendering. It is a pure Rust struct, language-agnostic.

```rust
pub struct CanonicalDocument {
    pub metadata: BookMetadata,         // title, authors, language, identifier, ...
    pub front_matter: Vec<Block>,       // title page, copyright, dedication, ToC placeholder
    pub body: Vec<ChapterDoc>,          // ordered chapters
    pub back_matter: Vec<Block>,        // about-the-author, also-by, etc.
    pub assets: Vec<AssetRef>,          // images, fonts; content-addressed
    pub stylesheet: Stylesheet,         // canonical CSS (resolved from template + project overrides)
}

pub struct ChapterDoc {
    pub id: NodeId,
    pub title: String,
    pub blocks: Vec<Block>,             // canonical HTML-ish block model
    pub footnotes: Vec<Footnote>,
}
```

The `Block` enum mirrors the editor's TipTap node set 1:1. There is no information loss between the editor and the Canonical Document.

## 4. Canonical HTML

`booksforge-export::render_html(&CanonicalDocument)` produces deterministic HTML that:

- Has stable element IDs (using ULID-derived ids on every block) for cross-references.
- Uses semantic HTML5 tags (`<article>`, `<section>`, `<aside>` for footnotes, `<figure>`, `<figcaption>`).
- Includes `epub:type` attributes on key elements (chapter, frontmatter, bodymatter, backmatter, footnote, footnote-ref) for EPUB-3 semantics.
- Uses **Unicode characters** for typography (em-dash, en-dash, smart quotes, ellipsis) — never HTML entities for these.
- Has **no inline styles** — all styling is in the stylesheet.
- Has **no scripts** — EPUB readers vary in script support; we don't rely on them.
- Has stable image references via content-addressed asset paths (`assets/<aa>/<rest>.<ext>`).

The HTML is **deterministic**: the same Canonical Document produces byte-identical HTML on every invocation. Tested in CI.

## 5. Canonical CSS

A single stylesheet `style.css` is composed from:

1. The template's base stylesheet (from `templates/<id>/style.css`).
2. Project overrides from `style_book` and `manifest.toml` (margins, font choices).
3. A small reset to neutralise reader defaults where they cause visual drift.

The CSS is the same in the preview and in the EPUB. It is generated by `booksforge-export::render_css(&CanonicalDocument)` — also deterministic.

**Constraint.** The CSS uses only the subset that EPUB-3 readers reliably support: standard typography properties, Flexbox/Grid only where progressive-fallback works, no CSS-Houdini. Specifics in `EXPORT_EPUB_QA.md §9`.

## 6. Preview pane

The preview pane is a WebView in `apps/desktop/src-ui`. It mounts the Canonical HTML and Canonical CSS. There is **no React rendering for the preview** — it is a pure HTML view, identical to what an EPUB reader would render.

**Why a separate WebView?** The TipTap editor in the centre pane has its own selection / cursor / decoration overlays. The preview is a clean view, the same HTML/CSS the EPUB will contain.

The preview supports:

- Live updating on save (with a debounce).
- Per-chapter navigation matching the editor binder.
- A "Compare with last export" overlay (V1.0) that diffs the canonical HTML with the last exported EPUB's content; differences highlighted.

## 7. EPUB-3 packaging

`booksforge-export-epub` (Rust crate) packages the EPUB-3 container. It uses the `epub-builder` crate (or equivalent) for the mechanical zipping; the content (HTML, CSS, images) is the canonical output above.

**Output structure** (standard EPUB-3):

```
MyBook.epub (zip)
├── mimetype                              ('application/epub+zip', stored uncompressed, first entry)
├── META-INF/
│   └── container.xml                     (points at OEBPS/content.opf)
└── OEBPS/
    ├── content.opf                        (package manifest)
    ├── nav.xhtml                          (EPUB-3 navigation document)
    ├── style.css                          (canonical CSS)
    ├── chapters/
    │   ├── 0001-front-matter-title.xhtml
    │   ├── 0002-front-matter-copyright.xhtml
    │   ├── 0003-front-matter-dedication.xhtml
    │   ├── 0004-front-matter-toc.xhtml    (optional — nav.xhtml is the source of truth)
    │   ├── 1001-chapter-one.xhtml
    │   ├── ...
    │   └── 9001-back-matter-about.xhtml
    └── images/
        ├── ab/cd1234ef.png
        └── ...
```

### 7.1 Files we generate ourselves (no Pandoc, no Calibre)

- **`mimetype`** — fixed, stored uncompressed, first entry. Required for valid EPUB.
- **`META-INF/container.xml`** — fixed XML pointer to `OEBPS/content.opf`.
- **`OEBPS/content.opf`** — package manifest with metadata, item list, spine. Generated from `BookMetadata` and the chapter file list.
- **`OEBPS/nav.xhtml`** — EPUB-3 navigation document, generated from the document tree (parts → chapters → optional sub-headings).
- **`OEBPS/chapters/*.xhtml`** — wrapped canonical HTML for each chapter (XHTML-strict, with `epub:type` attributes).
- **`OEBPS/style.css`** — the canonical CSS.
- **`OEBPS/images/`** — content-addressed copies of the project's assets.

### 7.2 What we do not do

- We do not run Pandoc on the EPUB. There is no Pandoc step.
- We do not regenerate the Table of Contents from headings — `nav.xhtml` is built from the document tree, which is the source of truth.
- We do not modify image bytes; assets are copied verbatim into `OEBPS/images/`.

## 8. Profiles

Three EPUB-3 profiles in MVP:

### 8.1 `epub-3.generic`

Plain EPUB-3, no store-specific tweaks. Used for sending to beta readers, archiving, or uploading to platforms that accept any valid EPUB-3.

### 8.2 `epub-3.kdp-ebook`

Amazon KDP requirements:

- Cover image as the first item in the spine, with `properties="cover-image"` in the manifest.
- TOC depth ≤ 3.
- No external font references (fonts must be embedded if used).
- Fixed-layout EPUBs disabled in MVP (reflowable only).
- Specific metadata: ISBN, KDP-recommended dc:identifier scheme.
- Cover validation: at least 1600×2560 px, RGB, JPEG or PNG; under 50 MB total file.
- File size cap: 650 MB (KDP limit; we soft-cap at 50 MB for the MVP profile).

### 8.3 `epub-3.kdp-print` (placeholder)

Reserved; not generated as EPUB. KDP-Print uses PDF; this is a **PDF profile**, not an EPUB profile, and is part of the Pandoc PDF pipeline. Listed here to avoid confusion in the profile picker.

V1.0 adds: Apple Books EPUB, Kobo EPUB, Google Play Books EPUB, IngramSpark print-EPUB.

## 9. DOCX and PDF

These go through Pandoc. The pipeline:

1. Render the Canonical Document → Pandoc-AST (a separate `booksforge-export` function from the HTML renderer).
2. Pandoc reads the AST via stdin JSON and writes the target format.
3. Post-processors handle font embedding (PDF) and cleanup.

Profiles:

- **`docx.manuscript`** — industry-standard manuscript (12pt Times New Roman, double-spaced, 1-inch margins, headers with author / title / page #).
- **`pdf.trade-5x8`** — Trade Paperback 5×8.
- **`pdf.trade-6x9`** — Trade Paperback 6×9.

V1.0 adds: IngramSpark trim sizes, KDP-Print PDF, LaTeX export (academic).

## 10. Reproducibility

**Invariant.** Same Canonical Document + same template version + same engine version = byte-identical EPUB / DOCX / PDF.

We achieve this by:

- Sorting all collections deterministically (asset list by hash, manifest items by file path).
- Using fixed timestamps in EPUB ZIP entries (Pandoc has its own; we override).
- Pinning Pandoc version per release.
- Pinning `epub-builder` and `epubcheck` versions per release.
- Fixing the locale used in any text-formatting (e.g., date formatting in copyright pages — frozen to `en-US` unless overridden).

CI runs the reproducibility test on a 30-chapter fixture twice and asserts byte equality. Any drift fails the PR.

## 11. Asset pipeline

Assets live under `assets/<aa>/<rest>.<ext>` in the project bundle (content-addressed). For export:

- The exporter lists every asset referenced by the document tree.
- Copies them verbatim into `OEBPS/images/<aa>/<rest>.<ext>` (EPUB) or into Pandoc's media directory (DOCX/PDF).
- Updates references in the canonical HTML to point at the EPUB-internal paths (still content-addressed, just rooted differently).
- For PDF / print profiles: applies image-fitting rules per template (e.g., max page-image height = 70% of page height).

Image formats supported: PNG, JPEG, SVG, GIF (still). WebP is **post-MVP** for EPUB (reader support is uneven for older devices); converted to PNG on the fly during EPUB packaging if WebP is in the source.

## 12. Validation

**EPUBCheck** (the W3C-blessed validator) runs against every EPUB export.

- Errors block: the export is not written until the user resolves them.
- Warnings prompt: the user sees the warning list and can proceed.

In MVP we ship epubcheck 5.x as a sidecar with a small JRE bundle (jlink-stripped). The sidecar runs in a tmpdir and emits structured JSON; `booksforge-epubcheck` parses it.

For DOCX / PDF, we run lightweight format checks (PDF/A optional V1.0; DOCX schema check) but we do not block on them in MVP.

## 13. Pre-export gate

Per `UI_UX_SPEC.md §12` (Export dialog):

1. The user picks a format and profile.
2. "Run pre-export check" runs:
   - Manuscript validators (heading hierarchy, broken refs, missing alt text, etc.).
   - Format-specific validators (KDP file-size limits, ToC depth, ISBN format).
   - For EPUB: a dry-run of the EPUB packager (without writing to the user's chosen folder) followed by EPUBCheck.
3. The gate dialog shows summary; errors block (override allowed for advanced users with a confirmation); warnings prompt; info silent.
4. On accept, the export writes to the user's chosen folder and an `exports` row is created.

## 14. Performance budgets

| Target | Budget | Measured |
|--------|--------|----------|
| EPUB-3 export of 100k-word fixture with 20 images | ≤ 30 s | CI bench |
| DOCX export of 100k-word fixture | ≤ 15 s | CI bench |
| PDF export of 100k-word fixture (Trade 6×9) | ≤ 45 s | CI bench |
| Preview update on save (live preview) | ≤ 800 ms p95 | dev-tools probe |
| Reproducibility hash test | passes | CI |

## 15. Failure modes and recovery

| Failure | Detection | Recovery |
|---------|-----------|----------|
| EPUBCheck reports errors | parsed from epubcheck stdout | Surfaced in the gate; errors block by default; user can override warnings |
| Asset missing on disk (referenced but not in `assets/`) | pre-export validator | Surfaces with file path; user can re-import the asset |
| Pandoc sidecar crashes | exit code != 0 | Captured stderr surfaced in a typed error; user can retry; CI tracks Pandoc-version regressions |
| Reproducibility regression | CI hash diff | PR blocked unless the description includes a baseline-update commit and reason |
| Preview drift from export | Visual regression test (see `EXPORT_EPUB_QA.md`) | PR blocked; the canonical HTML/CSS pipeline is the contract |
| Disk full mid-export | I/O error | Partial files cleaned up (atomic rename pattern); typed error |
| User cancels export mid-run | cancellation token | Partial files cleaned; no `exports` row written; UI confirms |

## 16. Acceptance criteria for the export subsystem

The subsystem is acceptable when:

1. A 100k-word fixture project exports to all four MVP profiles (DOCX manuscript, PDF Trade 5×8, PDF Trade 6×9, EPUB-3 KDP-eBook) in <60 seconds total on the reference hardware.
2. The reproducibility test (CI) reports byte-identical output across two consecutive runs.
3. The exported EPUB-3 KDP-eBook profile passes EPUBCheck with zero errors and zero warnings on the fixture.
4. The visual regression test (preview rendering vs. EPUB content rendering) reports zero pixel diffs above the documented tolerance on the fixture.
5. Killing the app mid-export leaves no half-files in the user's export directory.
6. The Markdown / Pandoc-AST / Canonical HTML pipelines are fully deterministic — the prompt-template-hash style mechanism does not apply to export but the equivalent (engine version + template version + canonical-document hash) is logged on every export and matches across reruns.

## 17. Out of scope (V1.0+)

- Apple Books, Kobo, Google Play Books store profiles — V1.0.
- IngramSpark profiles (print + EPUB) — V1.0.
- LaTeX export — V1.0 (academic mode).
- Fixed-layout EPUBs (children's / illustrated) — V1.5.
- Audiobook export — V2.0.
- WebP image support in EPUB — when reader support stabilises (likely V1.5).
- A "Custom export profile editor" — V1.5.

## 18. What this is not

- It is not a Pandoc replacement. Pandoc is a great tool; we use it where its strengths apply (DOCX/PDF).
- It is not a WYSIWYG editor for EPUB. The user edits in TipTap; the export pipeline is rule-based.
- It is not magic. The preview-vs-export consistency comes from using the same source — not from clever conversion logic.
