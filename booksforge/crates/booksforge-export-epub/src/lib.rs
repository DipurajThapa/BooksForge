//! EPUB-3 packager (Layer 4 — infrastructure).
//!
//! Builds a valid EPUB-3.2 archive from a book's chapter HTML + metadata.
//! No external sidecar — the archive is produced entirely in Rust via the
//! `zip` crate.  EPUBCheck (a separate sidecar) validates the result.
//!
//! ## Determinism
//!
//! All file entries are written in a fixed order, with a fixed
//! modification time (1980-01-01, the lowest the ZIP format permits) and
//! no extended attributes.  Builds with identical input produce
//! byte-identical output, which makes reproducibility tests trivial
//! (BACKLOG §H5) and means hash-based dedup actually works.
//!
//! ## Archive layout
//!
//! ```text
//! mimetype                              # STORE-only, must be first
//! META-INF/container.xml                # rootfile pointer
//! OEBPS/content.opf                     # package document (manifest + spine)
//! OEBPS/nav.xhtml                       # navigation document (TOC)
//! OEBPS/styles/book.css                 # stylesheet
//! OEBPS/text/chapter-001.xhtml          # one per chapter, zero-padded
//! OEBPS/text/chapter-NNN.xhtml
//! ```

#![forbid(unsafe_code)]
// BACKLOG §C4: tests freely use `.unwrap()` / `.expect()` against canned
// fixtures; the workspace-level clippy lints fire only on shipped code.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::print_stderr,
        clippy::print_stdout,
    )
)]

use std::io::{Cursor, Write};

pub mod kdp_checks;
pub use kdp_checks::{run_kdp_checks, KdpFinding, KdpSeverity};

use booksforge_domain::FormatProfile;
use booksforge_export::{ExportOutcome, ExportProfile};
use serde::{Deserialize, Serialize};
use zip::{write::SimpleFileOptions, CompressionMethod, DateTime, ZipWriter};

/// Input to the EPUB packager.
#[derive(Debug, Clone)]
pub struct EpubPackageInput {
    /// Canonical HTML chapters in reading order.
    pub chapters: Vec<HtmlChapter>,
    /// Book-level metadata.
    pub metadata: EpubMetadata,
    /// Target profile — must be `KdpEbook` or `GenericEpub`.
    pub profile: ExportProfile,
    /// Absolute path where the output `.epub` file should be written.
    pub output_path: String,
    /// Genre-aware format profile that drives the CSS + auto-emitted
    /// front-matter.  Defaults to `FormatProfile::FictionTradeStandard`
    /// when omitted (BACKLOG §H8.1).
    pub format_profile: FormatProfile,
    /// Optional path to the bundled Google Font directory (BACKLOG
    /// §H8.2 follow-up — bundled fonts).  Layout under this directory:
    ///
    /// ```text
    /// <font_bundle_dir>/<Family_Name>/<Family>[wght].ttf
    /// <font_bundle_dir>/<Family_Name>/<Family>-Italic[wght].ttf
    /// ```
    ///
    /// When supplied, the packager copies the body + heading fonts
    /// for the `format_profile` into `OEBPS/fonts/` and emits
    /// `@font-face` rules pointing at them — readers see the book's
    /// intended typography even when offline.  When `None`, the CSS
    /// falls back to `@import url("https://fonts.googleapis.com/...")`.
    pub font_bundle_dir: Option<String>,
}

/// One chapter's canonical HTML for inclusion in the EPUB manifest.
#[derive(Debug, Clone)]
pub struct HtmlChapter {
    pub node_id: String,
    pub title: String,
    /// HTML *body* — the packager wraps it in the XHTML envelope (head,
    /// stylesheet link, etc).  Must be well-formed XHTML.
    pub html_body: String,
}

/// EPUB metadata block derived from `ProjectMeta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubMetadata {
    pub title: String,
    pub authors: Vec<String>,
    /// BCP-47 language code (e.g. `"en"`).  Defaults to `"en"` when blank.
    pub language: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    /// ISBN — copied into `<dc:identifier>` if provided; otherwise a
    /// `urn:uuid:` is generated from `book_id`.
    pub isbn: Option<String>,
    /// Stable book id used as the package's `unique-identifier`.  Caller
    /// supplies a project-stable ULID so subsequent exports of the same
    /// project share an id.
    pub book_id: String,
    /// Optional dedication line ("For my parents") for the
    /// auto-generated dedication page.  Only emitted when the
    /// `format_profile` includes `FrontMatterPage::Dedication`.
    #[serde(default)]
    pub dedication: Option<String>,
    /// Optional epigraph quote + attribution.  `(text, source)`.
    /// Only emitted when the profile includes
    /// `FrontMatterPage::Epigraph`.
    #[serde(default)]
    pub epigraph: Option<(String, String)>,
    /// Copyright notice line.  When `None`, an auto-generated default
    /// is composed from the year + first author.
    #[serde(default)]
    pub copyright_notice: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EpubError {
    #[error("unsupported profile for EPUB export: {profile:?}")]
    UnsupportedProfile { profile: ExportProfile },

    #[error("no chapters provided — cannot build an empty EPUB")]
    NoChapters,

    #[error("invalid metadata: {message}")]
    InvalidMetadata { message: String },

    #[error("I/O error writing EPUB to {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },

    #[error("ZIP construction failed: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("I/O error during EPUB build: {0}")]
    BuildIo(#[from] std::io::Error),
}

/// Build an EPUB-3 archive in memory and return its bytes.  Pure
/// computation — useful for golden tests and for the async writer below.
pub fn build_epub_bytes(input: &EpubPackageInput) -> Result<Vec<u8>, EpubError> {
    if input.chapters.is_empty() {
        return Err(EpubError::NoChapters);
    }
    if !matches!(
        input.profile,
        ExportProfile::KdpEbook | ExportProfile::GenericEpub
    ) {
        return Err(EpubError::UnsupportedProfile {
            profile: input.profile,
        });
    }
    if input.metadata.title.trim().is_empty() {
        return Err(EpubError::InvalidMetadata {
            message: "title cannot be empty".to_owned(),
        });
    }
    if input.metadata.authors.is_empty()
        || input.metadata.authors.iter().all(|a| a.trim().is_empty())
    {
        return Err(EpubError::InvalidMetadata {
            message: "at least one author required".to_owned(),
        });
    }

    let language = if input.metadata.language.trim().is_empty() {
        "en".to_owned()
    } else {
        input.metadata.language.clone()
    };

    // Fixed timestamp = ZIP epoch (1980-01-01 00:00:00).
    let zip_dt = DateTime::default();

    let mut buf = Cursor::new(Vec::with_capacity(64 * 1024));
    {
        let mut zip = ZipWriter::new(&mut buf);

        // ── 1. mimetype — first, STORE-compressed
        let mimetype_opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(zip_dt);
        zip.start_file("mimetype", mimetype_opts)?;
        zip.write_all(b"application/epub+zip")?;

        let entry_opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(zip_dt);

        // ── 2. META-INF/container.xml
        zip.start_file("META-INF/container.xml", entry_opts)?;
        zip.write_all(CONTAINER_XML.as_bytes())?;

        // ── 3a. OEBPS/fonts/* — bundled Google Fonts (BACKLOG §H8.2)
        // When `font_bundle_dir` is set, copy the body + heading font
        // files for the chosen `format_profile` into the EPUB so the
        // book renders with its intended typography even on offline
        // / sandboxed readers (most modern EPUB readers refuse network
        // requests anyway).  When `None`, the CSS falls back to the
        // Google Fonts CDN @import.
        let bundled_fonts: Vec<BundledFont> = match input.font_bundle_dir.as_deref() {
            Some(dir) => collect_bundled_fonts(dir, input.format_profile),
            None => Vec::new(),
        };
        for f in &bundled_fonts {
            zip.start_file(format!("OEBPS/fonts/{}", f.epub_name), entry_opts)?;
            zip.write_all(&f.bytes)?;
        }

        // ── 3b. OEBPS/styles/book.css — generated from the format profile
        zip.start_file("OEBPS/styles/book.css", entry_opts)?;
        let css = render_book_css(input.format_profile, &bundled_fonts);
        zip.write_all(css.as_bytes())?;

        // ── 4. Auto-generated front-matter (BACKLOG §H8.1)
        // Profile-driven: each `FormatProfile` declares which front-
        // matter pages to inject (title page / copyright / dedication
        // / epigraph / TOC).  Skipped entirely if the caller's first
        // chapter already looks like a title page — authors who
        // supply their own front-matter take precedence.
        let supplied_first_title = input
            .chapters
            .first()
            .map(|c| c.title.to_lowercase())
            .unwrap_or_default();
        let author_provided_frontmatter = matches!(
            supplied_first_title.as_str(),
            "title page" | "title" | "front matter" | "frontmatter",
        );

        let mut chapter_filenames: Vec<String> = Vec::with_capacity(input.chapters.len() + 4);
        let mut spine_chapters: Vec<HtmlChapter> = Vec::with_capacity(input.chapters.len() + 4);

        if !author_provided_frontmatter {
            for page in input.format_profile.front_matter_pages() {
                use booksforge_domain::FrontMatterPage;
                let (fname, title, body) = match page {
                    FrontMatterPage::TitlePage => (
                        "title-page.xhtml",
                        input.metadata.title.clone(),
                        render_title_page_body(&input.metadata),
                    ),
                    FrontMatterPage::Copyright => (
                        "copyright.xhtml",
                        "Copyright".to_owned(),
                        render_copyright_body(&input.metadata),
                    ),
                    FrontMatterPage::Dedication => {
                        // Skip if the author hasn't supplied one — better
                        // than a stub "For ___" placeholder.
                        let Some(text) = input
                            .metadata
                            .dedication
                            .as_ref()
                            .filter(|s| !s.trim().is_empty())
                        else {
                            continue;
                        };
                        (
                            "dedication.xhtml",
                            "Dedication".to_owned(),
                            render_dedication_body(text),
                        )
                    }
                    FrontMatterPage::Epigraph => {
                        let Some((text, source)) = input
                            .metadata
                            .epigraph
                            .as_ref()
                            .filter(|(t, _)| !t.trim().is_empty())
                        else {
                            continue;
                        };
                        (
                            "epigraph.xhtml",
                            "Epigraph".to_owned(),
                            render_epigraph_body(text, source),
                        )
                    }
                    FrontMatterPage::TableOfContents => {
                        // The EPUB nav.xhtml IS the table of contents in
                        // EPUB-3 — we don't need a separate page in
                        // OEBPS/text.  The reader app renders it from
                        // nav.xhtml automatically.
                        continue;
                    }
                };
                let xhtml = render_chapter_xhtml_with_role(&title, &body, "frontmatter");
                zip.start_file(format!("OEBPS/text/{fname}"), entry_opts)?;
                zip.write_all(xhtml.as_bytes())?;
                chapter_filenames.push(fname.to_owned());
                spine_chapters.push(HtmlChapter {
                    node_id: fname.trim_end_matches(".xhtml").to_owned(),
                    title,
                    html_body: body,
                });
            }
        }

        // ── 5. OEBPS/text/chapter-NNN.xhtml — one per body chapter
        for (i, ch) in input.chapters.iter().enumerate() {
            let fname = format!("chapter-{:03}.xhtml", i + 1);
            let path = format!("OEBPS/text/{fname}");
            let xhtml = render_chapter_xhtml(&ch.title, &ch.html_body);
            zip.start_file(&path, entry_opts)?;
            zip.write_all(xhtml.as_bytes())?;
            chapter_filenames.push(fname);
            spine_chapters.push(ch.clone());
        }

        // ── 6. OEBPS/nav.xhtml
        zip.start_file("OEBPS/nav.xhtml", entry_opts)?;
        let nav = render_nav_xhtml(&input.metadata.title, &spine_chapters, &chapter_filenames);
        zip.write_all(nav.as_bytes())?;

        // ── 7. OEBPS/content.opf
        zip.start_file("OEBPS/content.opf", entry_opts)?;
        let opf = render_content_opf(
            &input.metadata,
            &language,
            input.profile,
            &chapter_filenames,
            &spine_chapters,
            &bundled_fonts,
        );
        zip.write_all(opf.as_bytes())?;

        zip.finish()?;
    }
    Ok(buf.into_inner())
}

/// Build an EPUB-3 archive and write it to `input.output_path`, returning
/// an `ExportOutcome` (path + blake3 hash).  Hops to a blocking task to
/// avoid stalling the async runtime on large books.
pub async fn build_epub(input: EpubPackageInput) -> Result<ExportOutcome, EpubError> {
    let bytes = build_epub_bytes(&input)?;
    let path = input.output_path.clone();
    let hash = blake3::hash(&bytes).to_hex().to_string();

    let path_blocking = path.clone();
    let bytes_for_write = bytes;
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        if let Some(parent) = std::path::Path::new(&path_blocking).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let tmp = format!("{path_blocking}.tmp");
        std::fs::write(&tmp, &bytes_for_write)?;
        std::fs::rename(&tmp, &path_blocking)?;
        Ok(())
    })
    .await
    .map_err(|e| EpubError::Io {
        path: path.clone(),
        source: std::io::Error::other(e),
    })?
    .map_err(|e| EpubError::Io {
        path: path.clone(),
        source: e,
    })?;

    Ok(ExportOutcome {
        profile: input.profile,
        output_path: path,
        hash,
    })
}

// ── XML / XHTML / OPF rendering ─────────────────────────────────────────────

const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;

/// One font file copied into the EPUB at `OEBPS/fonts/<epub_name>`.
/// Used both by the CSS factory (to emit `@font-face` rules) and by
/// the OPF renderer (to add the right manifest items).
#[derive(Debug, Clone)]
pub struct BundledFont {
    /// Family name as it appears in `FormatProfile::google_body_family`
    /// (e.g. `"EB Garamond"`).
    pub family: String,
    /// File name inside the EPUB (e.g. `"EBGaramond[wght].ttf"`).
    pub epub_name: String,
    /// CSS `font-style` — `"normal"` or `"italic"`.
    pub style: &'static str,
    /// Raw font bytes.
    pub bytes: Vec<u8>,
}

impl BundledFont {
    fn media_type(&self) -> &'static str {
        // OpenType / TrueType — `application/font-sfnt` is the EPUB-3
        // recommended type for both .otf and .ttf files.
        "application/font-sfnt"
    }
    fn opf_id(&self, idx: usize) -> String {
        format!("font-{idx}")
    }
}

/// Walk `dir` (the bundled-font directory laid down by
/// `scripts/fetch-fonts.sh`) and return every font file the
/// packager should embed for the chosen profile.  Looks for both
/// the body font and the heading font (skipping duplicates when a
/// profile uses the same family for both).
///
/// Layout expected:
///
/// ```text
/// <dir>/<Family_With_Underscores>/<Family>[wght].ttf            # roman
/// <dir>/<Family_With_Underscores>/<Family>-Italic[wght].ttf     # italic
/// ```
///
/// I/O failures (missing dir / missing file) are *not* errors — the
/// CSS factory simply emits the Google Fonts CDN @import as a
/// fallback for any family it doesn't get bytes for.
fn collect_bundled_fonts(dir: &str, profile: FormatProfile) -> Vec<BundledFont> {
    use std::path::Path;
    let mut out: Vec<BundledFont> = Vec::with_capacity(4);
    let mut wanted: Vec<&str> = vec![profile.google_body_family()];
    if profile.google_heading_family() != profile.google_body_family() {
        wanted.push(profile.google_heading_family());
    }
    for family in wanted {
        let dir_name = family.replace(' ', "_"); // "EB Garamond" → "EB_Garamond"
        let stem = family.replace(' ', ""); // "EB Garamond" → "EBGaramond"
        let family_dir = Path::new(dir).join(&dir_name);

        // Variable-weight families ship as `<Stem>[wght].ttf` /
        // `<Stem>-Italic[wght].ttf`; some (Source Serif 4 / Inter)
        // also expose the `opsz` axis.  Try the candidates in order.
        let candidates: [(&str, Vec<String>); 2] = [
            (
                "normal",
                vec![
                    format!("{stem}[wght].ttf"),
                    format!("{stem}[opsz,wght].ttf"),
                ],
            ),
            (
                "italic",
                vec![
                    format!("{stem}-Italic[wght].ttf"),
                    format!("{stem}-Italic[opsz,wght].ttf"),
                ],
            ),
        ];
        for (style, names) in candidates.iter() {
            for name in names {
                let path = family_dir.join(name);
                if let Ok(bytes) = std::fs::read(&path) {
                    out.push(BundledFont {
                        family: family.to_owned(),
                        epub_name: name.clone(),
                        style,
                        bytes,
                    });
                    break;
                }
            }
        }
    }
    out
}

/// Build the CSS for an EPUB targeted at a particular `FormatProfile`.
/// Pulls font families, type sizes, drop-cap policy, scene-break
/// glyph, and paragraph-indent from the profile so different genres
/// get genre-appropriate typography from a single packager.
///
/// When `bundled_fonts` is non-empty, the CSS emits `@font-face`
/// rules pointing at `fonts/<file>.ttf` (offline-friendly).  When
/// empty, it falls back to `@import url(googleapis.com/...)`.
///
/// Static fallback (the original `BOOK_CSS` const) lives below as a
/// reference but is no longer used directly.
fn render_book_css(profile: FormatProfile, bundled_fonts: &[BundledFont]) -> String {
    let body_family = profile.body_font_family();
    let heading_family = profile.heading_font_family();
    let body_size = profile.body_em();
    let line_height = profile.line_height();
    let scene_glyph = profile.scene_break_glyph();
    let ornament_svg = profile.ornament_svg();
    let para_indent = profile.paragraph_indent_em();
    let google_body = profile.google_body_family();
    let google_heading = profile.google_heading_family();

    // Font import block — two paths:
    //
    //   (a) **Bundled fonts present** (BACKLOG §H8.2 follow-up).
    //       Emit a `@font-face` rule per file pointing at the
    //       OPF-manifested EPUB asset under `OEBPS/fonts/`.  The
    //       reader needs no network access; the book renders
    //       identically across devices.
    //   (b) **No bundled fonts** (fallback).  `@import` Google's CDN
    //       so online readers still see the intended typography,
    //       falling back to the system stack offline.
    let font_imports = if !bundled_fonts.is_empty() {
        let mut s = String::with_capacity(256);
        for f in bundled_fonts {
            s.push_str(&format!(
                "@font-face {{\n  font-family: \"{family}\";\n  font-style: {style};\n  font-weight: 100 900;\n  src: url(\"../fonts/{file}\") format(\"truetype\");\n}}\n",
                family = f.family,
                style  = f.style,
                file   = f.epub_name,
            ));
        }
        s
    } else {
        let google_url = |family: &str| -> String {
            let slug: String = family
                .chars()
                .map(|c| if c == ' ' { '+' } else { c })
                .collect();
            format!(
                "https://fonts.googleapis.com/css2?family={slug}:ital,wght@0,400;0,700;1,400;1,700&display=swap"
            )
        };
        if google_body == google_heading {
            format!("@import url(\"{}\");", google_url(google_body))
        } else {
            format!(
                "@import url(\"{}\");\n@import url(\"{}\");",
                google_url(google_body),
                google_url(google_heading),
            )
        }
    };

    let drop_cap_block = if profile.drop_cap() {
        r#"
/* Optional drop cap on first paragraph after a chapter title.
 * Authors opt in by giving the paragraph class "drop". */
p.drop::first-letter {
  float: left;
  font-size: 3.4em;
  line-height: 0.9;
  padding: 0.05em 0.08em 0 0;
  font-weight: 600;
}"#
    } else {
        ""
    };

    // For non-fiction practical we want block paragraphs (no indent +
    // gap between paragraphs).  For everything else, indented paras.
    let block_paragraph_extras = if matches!(profile, FormatProfile::NonFictionPractical) {
        "p { margin-bottom: 0.6em; }"
    } else {
        ""
    };

    // Scene-break rendering — three-tier fallback.
    //
    // 1. If an SVG ornament is defined for the profile, render the hr
    //    as a centred image via a data: URI that references the
    //    inline SVG.  This is the BACKLOG §H8.2 "hand-curated
    //    ornament library" path — every sub-genre gets its own glyph
    //    (regency flourish, gothic cross, cookbook plates, …).
    // 2. If only a Unicode glyph is defined, render via `::before`
    //    `content: "<glyph>"` (the H8.1 path).
    // 3. If neither is defined (Academic), suppress the hr entirely.
    let scene_break_block = if !ornament_svg.is_empty() {
        // URL-encode the inline SVG.  We percent-encode only the
        // characters that have meaning in CSS url() / data: URIs;
        // the SVG body itself is otherwise plain ASCII.
        let svg_for_url = ornament_svg
            .replace('"', "%22")
            .replace('#', "%23")
            .replace('<', "%3C")
            .replace('>', "%3E");
        format!(
            "hr, .scene-break {{
  border: none;
  height: 1.2em;
  margin: 1.5em auto;
  background-image: url(\"data:image/svg+xml;utf8,{svg_for_url}\");
  background-repeat: no-repeat;
  background-position: center center;
  background-size: contain;
  color: inherit;
}}",
        )
    } else if scene_glyph.is_empty() {
        "hr, .scene-break { display: none; }".to_owned()
    } else {
        format!(
            "hr, .scene-break {{
  border: none;
  margin: 1.5em auto;
  text-align: center;
  letter-spacing: 0.4em;
}}
hr::before {{ content: \"{scene_glyph}\"; }}",
        )
    };

    // Practical non-fiction gets a callout-box utility class.  YA gets
    // a softer paragraph rhythm.  Memoir gets footnote rule styling.
    let profile_extras = match profile {
        FormatProfile::NonFictionPractical => {
            r#"
.callout {
  margin: 1em 0;
  padding: 0.8em 1em;
  border-left: 3px solid #888;
  background: #f7f7f5;
  font-size: 0.95em;
}
.callout > h4 {
  margin: 0 0 0.4em 0;
  font-size: 1em;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}"#
        }
        FormatProfile::NonFictionMemoir => {
            r#"
.footnote {
  font-size: 0.85em;
  margin: 0.4em 0 0.4em 1em;
  border-top: 1px solid #ddd;
  padding-top: 0.3em;
  opacity: 0.85;
}
.photo-plate {
  text-align: center;
  margin: 1.5em 0;
}
.photo-plate img { max-width: 100%; }
.photo-plate .caption {
  font-size: 0.85em;
  font-style: italic;
  margin-top: 0.4em;
  opacity: 0.85;
}"#
        }
        FormatProfile::Academic => {
            r#"
.footnote-ref { vertical-align: super; font-size: 0.75em; }
table {
  border-collapse: collapse;
  margin: 1em auto;
  font-size: 0.9em;
}
table th, table td {
  border: 1px solid #999;
  padding: 0.3em 0.6em;
}
.bibliography p { text-indent: -1.2em; padding-left: 1.2em; }"#
        }
        FormatProfile::FictionYoungAdult => {
            r#"
/* Softer chapter heading rhythm — YA conventionally has more air. */
h1 { margin-top: 3em; }
h2 { margin-top: 2em; }"#
        }
        _ => "",
    };

    format!(
        r#"@charset "utf-8";
/* ── BooksForge EPUB stylesheet — profile: {profile_str} ─────────────
 *
 * Generated per `FormatProfile`.  See
 * `crates/booksforge-domain/src/format_profile.rs` for the full
 * matrix of typography decisions per profile.
 * ────────────────────────────────────────────────────────────────── */

{font_imports}

html, body {{ margin: 0; padding: 0; }}

body {{
  font-family: {body_family};
  font-size:  {body_size};
  line-height: {line_height};
  margin: 0 1em;
  -webkit-hyphens: auto;
  -epub-hyphens: auto;
  hyphens: auto;
}}

h1, h2, h3, h4, h5, h6 {{
  font-family: {heading_family};
  line-height: 1.2;
  font-weight: 600;
  -webkit-hyphens: none;
  -epub-hyphens: none;
  hyphens: none;
  page-break-after: avoid;
}}

h1 {{
  font-size: 1.7em;
  margin: 2.5em 0 1.25em;
  text-align: center;
  page-break-before: always;
}}
h2 {{ font-size: 1.3em;  margin: 1.4em 0 0.5em; }}
h3 {{ font-size: 1.1em;  margin: 1.2em 0 0.4em; }}

p {{
  text-indent: {para_indent};
  margin: 0;
  orphans: 2;
  widows:  2;
  text-align: justify;
}}
p:first-of-type,
h1 + p, h2 + p, h3 + p, h4 + p,
blockquote p,
hr + p,
.scene-break + p,
p.first {{ text-indent: 0; }}
{block_paragraph_extras}
{drop_cap_block}

blockquote {{
  margin: 1em 1.6em;
  font-style: italic;
}}

{scene_break_block}

ul, ol {{ margin: 0.6em 0 0.6em 1.4em; padding: 0; }}
li {{ margin: 0.2em 0; }}

code, pre {{
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.92em;
}}
pre {{
  white-space: pre-wrap;
  margin: 1em 0;
  padding: 0.6em 0.8em;
  background: #f5f5f3;
  border-radius: 4px;
}}

section.frontmatter h1 {{ margin-top: 4em; }}
section.frontmatter   {{ text-align: center; }}
section.copyright    {{ font-size: 0.85em; opacity: 0.8; text-align: center; }}
section.dedication   {{ text-align: center; font-style: italic; margin-top: 4em; }}
section.epigraph     {{ text-align: center; margin-top: 4em; }}
section.epigraph .quote     {{ font-style: italic; }}
section.epigraph .source    {{ font-size: 0.85em; opacity: 0.85; margin-top: 0.6em; }}

a {{
  color: inherit;
  text-decoration: underline;
  text-decoration-thickness: 0.05em;
  text-underline-offset: 0.15em;
}}
{profile_extras}
"#,
        profile_str = profile.as_str(),
        body_family = body_family,
        heading_family = heading_family,
        body_size = body_size,
        line_height = line_height,
        para_indent = para_indent,
        block_paragraph_extras = block_paragraph_extras,
        drop_cap_block = drop_cap_block,
        scene_break_block = scene_break_block,
        profile_extras = profile_extras,
    )
}

#[allow(dead_code)]
const BOOK_CSS: &str = r#"@charset "utf-8";
/* ── BooksForge default EPUB stylesheet ─────────────────────────────
 *
 * Targets fiction trade-paperback typography on Apple Books, Kindle,
 * Kobo, Calibre, and Thorium.  Emphasis on:
 *   - Indented paragraphs in the body, no-indent after a heading or
 *     scene break (the trade-paperback convention).
 *   - Centred chapter titles with breathing room above and below.
 *   - Scene breaks rendered as a centred row of three asterisks.
 *   - A subtle drop-cap on the first paragraph of each chapter
 *     (opt-in via `<p class="first">`; the default fallback is fine).
 *   - Blockquotes set in italic with margin indents.
 *
 * Readers that don't honour CSS3 (older Kindle MOBI) degrade
 * gracefully — text-indent and font-style work everywhere.
 * ────────────────────────────────────────────────────────────────── */

html, body {
  margin: 0;
  padding: 0;
}

body {
  font-family: Georgia, "Iowan Old Style", "Times New Roman", serif;
  line-height: 1.55;
  margin: 0 1em;
  /* Hyphenate where the renderer supports it; harmless elsewhere. */
  -webkit-hyphens: auto;
  -epub-hyphens: auto;
  hyphens: auto;
}

/* Headings — sans-serif counterpoint to the body's serif. */
h1, h2, h3, h4, h5, h6 {
  font-family: "Helvetica Neue", Helvetica, Arial, sans-serif;
  line-height: 1.2;
  font-weight: 600;
  -webkit-hyphens: none;
  -epub-hyphens: none;
  hyphens: none;
  page-break-after: avoid;
}

/* Chapter title.  Add space above so it doesn't crowd the page top
 * on devices that respect page-break-before. */
h1 {
  font-size: 1.7em;
  margin: 2.5em 0 1.25em;
  text-align: center;
  page-break-before: always;
}

h2 { font-size: 1.3em;  margin: 1.4em 0 0.5em; }
h3 { font-size: 1.1em;  margin: 1.2em 0 0.4em; }

/* Body paragraphs — indented per trade-paperback convention. */
p {
  text-indent: 1.2em;
  margin: 0;
  orphans: 2;
  widows:  2;
  text-align: justify;
}

/* No indent immediately after a heading or scene break, or for the
 * first paragraph of any block. */
p:first-of-type,
h1 + p, h2 + p, h3 + p, h4 + p,
blockquote p,
hr + p,
.scene-break + p,
p.first { text-indent: 0; }

/* Optional drop cap on first paragraph after a chapter title.
 * Authors opt in by giving the paragraph class "drop". */
p.drop::first-letter {
  float: left;
  font-size: 3.4em;
  line-height: 0.9;
  padding: 0.05em 0.08em 0 0;
  font-weight: 600;
  font-family: "Helvetica Neue", Helvetica, Arial, sans-serif;
}

/* Block quotes set in italic with comfortable indents. */
blockquote {
  margin: 1em 1.6em;
  font-style: italic;
}

/* Scene break — three centred asterisks separated by spaces.  Authors
 * insert with `<p class="scene-break">* * *</p>` or via the
 * markdown horizontal-rule which exports as <hr/>. */
hr,
.scene-break {
  border: none;
  margin: 1.5em auto;
  text-align: center;
  font-size: 1em;
  letter-spacing: 0.5em;
}
hr::before { content: "* * *"; }

/* Lists — unindented relative to body so they don't double-indent. */
ul, ol { margin: 0.6em 0 0.6em 1.4em; padding: 0; }
li { margin: 0.2em 0; }

/* Inline code (rare in fiction; common in non-fiction) — make it
 * obvious without dominating the page. */
code, pre {
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.92em;
}
pre {
  white-space: pre-wrap;
  margin: 1em 0;
  padding: 0.6em 0.8em;
  background: #f5f5f3;
  border-radius: 4px;
}

/* Front-matter conventions: title page centred, copyright muted. */
section.frontmatter h1 { margin-top: 4em; }
section.frontmatter   { text-align: center; }
section.copyright    { font-size: 0.85em; opacity: 0.8; text-align: center; }

/* Internal cross-references — same colour as body text, underlined
 * subtly so they read as text, not as web links. */
a {
  color: inherit;
  text-decoration: underline;
  text-decoration-thickness: 0.05em;
  text-underline-offset: 0.15em;
}
"#;

fn render_chapter_xhtml(title: &str, body: &str) -> String {
    render_chapter_xhtml_with_role(title, body, "bodymatter")
}

/// Same as `render_chapter_xhtml` but lets the caller choose the
/// `epub:type` role.  Used for front-matter (title page, copyright)
/// and back-matter where the body chapters get `bodymatter`.
fn render_chapter_xhtml_with_role(title: &str, body: &str, role: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="en" lang="en">
<head>
<meta charset="utf-8"/>
<title>{title}</title>
<link rel="stylesheet" type="text/css" href="../styles/book.css"/>
</head>
<body>
<section epub:type="{role}">
{body}
</section>
</body>
</html>
"#,
        title = xml_escape(title),
        role = xml_escape(role),
        body = body,
    )
}

/// Build the auto-generated title page XHTML (BACKLOG H1 formatting).
/// EPUB readers display the first spine entry as the cover when no
/// real cover image is present; a clean title page is the next best
/// thing.  Authors who supply their own front-matter chapter take
/// precedence — this only fires when there's no chapter with title
/// "Title Page" or similar already in the input.
fn render_title_page_body(md: &EpubMetadata) -> String {
    let authors = md
        .authors
        .iter()
        .filter(|a| !a.trim().is_empty())
        .map(|a| xml_escape(a))
        .collect::<Vec<_>>()
        .join(" &amp; ");
    let publisher_line = md
        .publisher
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|p| format!("<p class=\"copyright\">{}</p>\n", xml_escape(p)))
        .unwrap_or_default();
    format!(
        "<h1>{title}</h1>\n<p style=\"text-align:center;font-style:italic\">by {authors}</p>\n{publisher}",
        title = xml_escape(&md.title),
        authors = if authors.is_empty() { "—".to_owned() } else { authors },
        publisher = publisher_line,
    )
}

/// Copyright page body.  Composes a default "© <year> <author>" line
/// when `metadata.copyright_notice` is None; otherwise renders the
/// caller's verbatim text.  Wrapped in `<section class="copyright">`
/// so the CSS picks up the muted styling.
fn render_copyright_body(md: &EpubMetadata) -> String {
    use chrono::Datelike;
    let notice = md.copyright_notice.clone().unwrap_or_else(|| {
        let year = chrono::Utc::now().year();
        let author = md
            .authors
            .first()
            .cloned()
            .unwrap_or_else(|| "the author".to_owned());
        format!("© {year} {author}.  All rights reserved.")
    });
    let publisher_line = md
        .publisher
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|p| format!("<p>{}</p>", xml_escape(p)))
        .unwrap_or_default();
    let isbn_line = md
        .isbn
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|i| format!("<p>ISBN {}</p>", xml_escape(i)))
        .unwrap_or_default();
    format!(
        "<section class=\"copyright\"><p>{notice}</p>{publisher}{isbn}</section>",
        notice = xml_escape(&notice),
        publisher = publisher_line,
        isbn = isbn_line,
    )
}

/// Dedication page — single italic line, centred via the
/// `section.dedication` CSS rule.
fn render_dedication_body(text: &str) -> String {
    format!(
        "<section class=\"dedication\"><p>{}</p></section>",
        xml_escape(text.trim()),
    )
}

/// Epigraph page — quote + source.  Source is rendered in muted
/// 0.85em via the `section.epigraph .source` rule.
fn render_epigraph_body(text: &str, source: &str) -> String {
    let source_line = if source.trim().is_empty() {
        String::new()
    } else {
        format!("<p class=\"source\">— {}</p>", xml_escape(source.trim()))
    };
    format!(
        "<section class=\"epigraph\"><p class=\"quote\">“{}”</p>{source}</section>",
        xml_escape(text.trim()),
        source = source_line,
    )
}

fn render_nav_xhtml(book_title: &str, chapters: &[HtmlChapter], fnames: &[String]) -> String {
    let mut items = String::new();
    for (i, ch) in chapters.iter().enumerate() {
        items.push_str(&format!(
            "      <li><a href=\"text/{fname}\">{title}</a></li>\n",
            fname = fnames[i],
            title = xml_escape(&ch.title),
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="en" lang="en">
<head>
<meta charset="utf-8"/>
<title>{title}</title>
</head>
<body>
<nav epub:type="toc" id="toc">
  <h1>Contents</h1>
  <ol>
{items}  </ol>
</nav>
</body>
</html>
"#,
        title = xml_escape(book_title),
        items = items,
    )
}

fn render_content_opf(
    md: &EpubMetadata,
    language: &str,
    profile: ExportProfile,
    fnames: &[String],
    chapters: &[HtmlChapter],
    fonts: &[BundledFont],
) -> String {
    let identifier = md.isbn.as_deref().unwrap_or("");
    let identifier = if identifier.is_empty() {
        format!("urn:uuid:{}", md.book_id)
    } else {
        identifier.to_owned()
    };

    // Stable per-profile timestamp.  GenericEpub uses Unix epoch for byte
    // determinism; KdpEbook uses a stable post-1970 timestamp because KDP
    // rejects 1970 as "untrusted" metadata.
    // Each arm explicitly documents the per-profile choice; the
    // GenericEpub and `_` wildcard happen to match but for *different*
    // reasons (one is policy, one is a guard for unsupported profiles).
    #[allow(clippy::match_same_arms)]
    let dcterms_modified = match profile {
        ExportProfile::GenericEpub => "1970-01-01T00:00:00Z",
        ExportProfile::KdpEbook => "2026-01-01T00:00:00Z",
        _ => "1970-01-01T00:00:00Z",
    };

    let mut creators = String::new();
    for (i, a) in md.authors.iter().enumerate() {
        creators.push_str(&format!(
            "    <dc:creator id=\"creator-{i}\">{name}</dc:creator>\n",
            name = xml_escape(a),
        ));
    }

    let publisher_line = md
        .publisher
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|p| format!("    <dc:publisher>{}</dc:publisher>\n", xml_escape(p)))
        .unwrap_or_default();

    let description_line = md
        .description
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|d| format!("    <dc:description>{}</dc:description>\n", xml_escape(d)))
        .unwrap_or_default();

    let mut manifest = String::new();
    manifest.push_str("    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");
    manifest.push_str("    <item id=\"css\" href=\"styles/book.css\" media-type=\"text/css\"/>\n");
    for (idx, f) in fonts.iter().enumerate() {
        manifest.push_str(&format!(
            "    <item id=\"{id}\" href=\"fonts/{name}\" media-type=\"{mt}\"/>\n",
            id = f.opf_id(idx),
            name = xml_escape(&f.epub_name),
            mt = f.media_type(),
        ));
    }
    for (i, fname) in fnames.iter().enumerate() {
        manifest.push_str(&format!(
            "    <item id=\"ch-{idx}\" href=\"text/{fname}\" media-type=\"application/xhtml+xml\"/>\n",
            idx = i + 1,
        ));
    }

    let mut spine = String::new();
    for i in 0..chapters.len() {
        spine.push_str(&format!("    <itemref idref=\"ch-{idx}\"/>\n", idx = i + 1));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf"
         version="3.0"
         xml:lang="{language}"
         unique-identifier="pub-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="pub-id">{identifier}</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:language>{language}</dc:language>
{creators}{publisher}{description}    <meta property="dcterms:modified">{modified}</meta>
  </metadata>
  <manifest>
{manifest}  </manifest>
  <spine>
{spine}  </spine>
</package>
"#,
        language = xml_escape(language),
        identifier = xml_escape(&identifier),
        title = xml_escape(&md.title),
        creators = creators,
        publisher = publisher_line,
        description = description_line,
        modified = dcterms_modified,
        manifest = manifest,
        spine = spine,
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> EpubPackageInput {
        EpubPackageInput {
            chapters: vec![
                HtmlChapter {
                    node_id: "01HX1".into(),
                    title: "Chapter 1".into(),
                    html_body: "<h1>Chapter 1</h1><p>Once upon a time.</p>".into(),
                },
                HtmlChapter {
                    node_id: "01HX2".into(),
                    title: "Chapter 2".into(),
                    html_body: "<h1>Chapter 2</h1><p>The next morning.</p>".into(),
                },
            ],
            metadata: EpubMetadata {
                title: "Test Book".into(),
                authors: vec!["Jane Doe".into()],
                language: "en".into(),
                publisher: Some("BooksForge".into()),
                description: None,
                isbn: None,
                book_id: "01H0000000000000000000000A".into(),
                dedication: None,
                epigraph: None,
                copyright_notice: None,
            },
            profile: ExportProfile::GenericEpub,
            output_path: "/tmp/ignored.epub".into(),
            format_profile: FormatProfile::FictionTradeStandard,
            font_bundle_dir: None,
        }
    }

    #[test]
    fn build_produces_a_zip() {
        let bytes = build_epub_bytes(&sample_input()).unwrap();
        assert_eq!(&bytes[0..4], b"PK\x03\x04");
    }

    #[test]
    fn mimetype_is_first_and_stored_uncompressed() {
        let bytes = build_epub_bytes(&sample_input()).unwrap();
        // First local file header at offset 0.
        let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let name = std::str::from_utf8(&bytes[30..30 + name_len]).unwrap();
        assert_eq!(name, "mimetype");
        // Compression method at offset 8..10 — 0 = STORE.
        let method = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(method, 0, "mimetype must be STORE-compressed");
    }

    #[test]
    fn bundled_fonts_get_embedded_under_oebps_fonts() {
        // Walk up from the crate dir to the workspace, then into
        // `apps/desktop/resources/fonts`.  Skip the test silently when
        // the bundle isn't on disk (e.g. before `scripts/fetch-fonts.sh`
        // has run) so CI without the bundle still passes.
        let bundle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .map(|root| {
                root.join("apps")
                    .join("desktop")
                    .join("resources")
                    .join("fonts")
            });
        let bundle = match bundle {
            Some(p) if p.is_dir() => p,
            _ => {
                eprintln!("font bundle not present — skipping");
                return;
            }
        };

        let mut input = sample_input();
        // FictionTradeStandard uses EB Garamond (body) + Inter (heading).
        input.font_bundle_dir = Some(bundle.to_string_lossy().into_owned());
        let bytes = build_epub_bytes(&input).expect("build");

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes[..])).expect("re-open zip");
        let mut found_body = false;
        let mut found_heading = false;
        for i in 0..zip.len() {
            let entry = zip.by_index(i).expect("entry");
            let name = entry.name();
            if name.starts_with("OEBPS/fonts/EBGaramond") {
                found_body = true;
            }
            if name.starts_with("OEBPS/fonts/Inter") {
                found_heading = true;
            }
        }
        assert!(found_body, "expected EBGaramond entry under OEBPS/fonts/");
        assert!(found_heading, "expected Inter entry under OEBPS/fonts/");
    }

    #[test]
    fn identical_inputs_produce_byte_identical_output() {
        let a = build_epub_bytes(&sample_input()).unwrap();
        let b = build_epub_bytes(&sample_input()).unwrap();
        assert_eq!(a.len(), b.len());
        assert_eq!(a, b, "deterministic bytes — drives reproducibility tests");
    }

    #[test]
    fn rejects_no_chapters() {
        let mut input = sample_input();
        input.chapters.clear();
        assert!(matches!(
            build_epub_bytes(&input),
            Err(EpubError::NoChapters)
        ));
    }

    #[test]
    fn rejects_unsupported_profile() {
        let mut input = sample_input();
        input.profile = ExportProfile::Docx;
        assert!(matches!(
            build_epub_bytes(&input),
            Err(EpubError::UnsupportedProfile { .. })
        ));
    }

    #[test]
    fn rejects_blank_title() {
        let mut input = sample_input();
        input.metadata.title = "   ".into();
        assert!(matches!(
            build_epub_bytes(&input),
            Err(EpubError::InvalidMetadata { .. })
        ));
    }

    #[test]
    fn rejects_no_authors() {
        let mut input = sample_input();
        input.metadata.authors = vec![];
        assert!(matches!(
            build_epub_bytes(&input),
            Err(EpubError::InvalidMetadata { .. })
        ));
    }

    #[test]
    fn xml_escape_handles_special_chars() {
        assert_eq!(
            xml_escape("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
    }

    #[tokio::test]
    async fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.epub");
        let mut input = sample_input();
        input.output_path = path.to_string_lossy().to_string();
        let outcome = build_epub(input).await.unwrap();
        assert!(std::path::Path::new(&outcome.output_path).exists());
        assert_eq!(outcome.hash.len(), 64);
        let on_disk = std::fs::read(&outcome.output_path).unwrap();
        assert_eq!(&on_disk[0..4], b"PK\x03\x04");
    }
}
