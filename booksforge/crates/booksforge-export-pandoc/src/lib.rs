//! Pandoc subprocess runner for DOCX and PDF export (Layer 4).
//!
//! BooksForge bundles Pandoc as a Tauri sidecar; this crate is the
//! cross-platform Rust shim that:
//!
//!   1. Resolves the Pandoc binary path (caller-supplied — typically
//!      from the Tauri sidecar resource resolver, with system PATH as
//!      a developer fallback).
//!   2. Spawns it with arguments for DOCX or PDF output.
//!   3. Pipes the manuscript Markdown over stdin (no temp file).
//!   4. Captures stderr for diagnostics; verifies the output file
//!      exists; computes blake3.
//!
//! Pandoc is NOT used for EPUB — the canonical EPUB-3 pipeline lives in
//! `booksforge-export-epub` (pure Rust, no sidecar).
//!
//! ## Profile mapping
//!
//! | profile          | format args                                                       |
//! |------------------|-------------------------------------------------------------------|
//! | `Docx`           | `-f markdown -t docx [--reference-doc=<template>]`                |
//! | `TradePdf5x8`    | `-f markdown -o <out>.pdf -V geometry:paperwidth=5in,paperheight=8in,...` |
//! | `TradePdf6x9`    | `-f markdown -o <out>.pdf -V geometry:paperwidth=6in,paperheight=9in,...` |
//!
//! PDF generation requires a TeX engine (xelatex / lualatex) on the
//! user's system; the wrapper surfaces a helpful error if it isn't
//! found rather than silently producing a broken file.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod reference_docx;
pub use reference_docx::build_reference_docx;

use booksforge_domain::FormatProfile;
use booksforge_export::{ExportError, ExportOutcome, ExportProfile};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

/// Input to the Pandoc export runner.
#[derive(Debug, Clone)]
pub struct PandocInput {
    /// Absolute path to the Pandoc binary.  Either resolved from the
    /// Tauri sidecar resource path or the system PATH lookup
    /// (`pandoc_on_path()`).
    pub pandoc_binary: String,
    /// Canonical Markdown source for the full manuscript.  Streamed to
    /// Pandoc's stdin so we never write the manuscript to /tmp.
    pub markdown_source: String,
    /// Reference `.docx` template path (DOCX output only).  When
    /// `None`, Pandoc's default DOCX template is used.
    pub docx_template: Option<String>,
    /// Target profile — must be `Docx`, `TradePdf5x8`, or `TradePdf6x9`.
    pub profile: ExportProfile,
    /// Absolute path where the output file should be written.
    pub output_path: String,
    /// Genre-aware format profile (BACKLOG §H8.1).  Drives PDF
    /// typography (font, size, line-height, document class, TOC).
    /// Ignored for DOCX output (the reference template handles that).
    pub format_profile: FormatProfile,
    /// Optional path to the bundled Google Font directory (BACKLOG
    /// §H8.2 follow-up).  When set, the PDF runner passes
    /// `mainfontoptions=Path=<file>,...` to xelatex so fonts resolve
    /// from the bundle without depending on the writer's system
    /// install.  When `None`, xelatex falls back to its native font
    /// lookup (system-installed Google Fonts).  No effect on DOCX.
    pub font_bundle_dir: Option<String>,
}

/// Resolve `pandoc` from the system PATH.  Useful for development; the
/// shipping build uses the Tauri sidecar path instead.
pub fn pandoc_on_path() -> Option<String> {
    which("pandoc")
}

fn which(binary: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        // Windows .exe fallback
        let exe = candidate.with_extension("exe");
        if exe.is_file() {
            return Some(exe.to_string_lossy().into_owned());
        }
    }
    None
}

/// Verify that the configured Pandoc binary is invokable.  Runs
/// `pandoc --version` and checks the exit status.  Returns the version
/// string on success so callers can log it / show it in settings.
pub async fn probe_pandoc(binary: &str) -> Result<String, ExportError> {
    if !Path::new(binary).is_file() {
        return Err(ExportError::SidecarMissing {
            binary: binary.to_owned(),
        });
    }
    let out = Command::new(binary)
        .arg("--version")
        .output()
        .await
        .map_err(|e| ExportError::SidecarMissing {
            binary: format!("{binary}: {e}"),
        })?;
    if !out.status.success() {
        return Err(ExportError::Failed {
            message: format!("{binary} --version exited {:?}", out.status),
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first_line = stdout.lines().next().unwrap_or("").trim();
    Ok(first_line.to_owned())
}

/// Build the argv for a given profile.  Pure logic (no I/O); split out
/// for unit testability of the profile → CLI mapping.
pub fn args_for_profile(
    profile: ExportProfile,
    output_path: &str,
    docx_template: Option<&str>,
) -> Vec<String> {
    args_for_profile_with_format(
        profile,
        output_path,
        docx_template,
        FormatProfile::default(),
    )
}

/// Build argv for an export profile, with full genre awareness via
/// `FormatProfile` (BACKLOG §H8.1).  PDF output uses the format
/// profile's trim, font, body size, line-height, document class,
/// class options, and TOC policy.  DOCX output ignores the format
/// profile (the reference template handles styling).
pub fn args_for_profile_with_format(
    profile: ExportProfile,
    output_path: &str,
    docx_template: Option<&str>,
    format_profile: FormatProfile,
) -> Vec<String> {
    args_for_profile_full(profile, output_path, docx_template, format_profile, None)
}

/// Same as [`args_for_profile_with_format`] but additionally threads
/// in the bundled-font directory so PDF runs use bundled Google Fonts
/// (BACKLOG §H8.2 follow-up).
pub fn args_for_profile_full(
    profile: ExportProfile,
    output_path: &str,
    docx_template: Option<&str>,
    format_profile: FormatProfile,
    font_bundle_dir: Option<&str>,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-f".into(),
        "markdown".into(),
        "--standalone".into(),
        "-o".into(),
        output_path.to_owned(),
    ];
    match profile {
        ExportProfile::Docx => {
            args.push("-t".into());
            args.push("docx".into());
            if let Some(tmpl) = docx_template {
                args.push(format!("--reference-doc={tmpl}"));
            }
        }
        ExportProfile::TradePdf5x8 | ExportProfile::TradePdf6x9 => {
            args.push("-t".into());
            args.push("pdf".into());
            push_pdf_geometry_for_format_with_fonts(&mut args, format_profile, font_bundle_dir);
        }
        _ => { /* caller already gated; produce a permissive default */ }
    }
    args
}

/// Build the LaTeX header-includes string from a `FormatProfile`.
/// Applies microtype + ragged2e + per-profile paragraph indent +
/// orphan/widow penalties.  Pure logic — public for unit tests.
fn header_includes_for_format(fp: FormatProfile) -> String {
    use FormatProfile::*;
    let para_indent = match fp {
        // Practical non-fiction uses block paragraphs (no indent).
        NonFictionPractical => "0pt",
        Academic => "1.5em",
        _ => "1.2em",
    };
    let extra = match fp {
        // Literary fiction gets fancy chapter headings via memoir's
        // chapterstyle.  YA gets bigger heading air.  Academic adds
        // section numbering.
        FictionLiterary => r"\chapterstyle{veelo}",
        FictionYoungAdult => r"\linespread{1.1}",
        Academic => r"\setcounter{secnumdepth}{3}",
        _ => "",
    };
    format!(
        r"\usepackage{{microtype}}\usepackage{{ragged2e}}\setlength{{\parindent}}{{{para_indent}}}\widowpenalty=10000\clubpenalty=10000{extra}",
    )
}

/// Per-format-profile PDF geometry.  Replaces the old fixed
/// `push_pdf_geometry` — trim, font, body size, line-height, document
/// class, class options, and TOC policy all come from `FormatProfile`
/// so a switch from "Fiction Literary" to "Academic" produces
/// genuinely different page layouts.
// Kept as the no-bundle entry-point; some test paths still call it
// directly via `args_for_profile_with_format`.
#[allow(dead_code)]
fn push_pdf_geometry_for_format(args: &mut Vec<String>, fp: FormatProfile) {
    push_pdf_geometry_for_format_with_fonts(args, fp, None)
}

/// Same as [`push_pdf_geometry_for_format`] but also wires in the
/// bundled Google Font directory so xelatex resolves fonts from the
/// app bundle rather than the writer's system install.
fn push_pdf_geometry_for_format_with_fonts(
    args: &mut Vec<String>,
    fp: FormatProfile,
    bundle: Option<&str>,
) {
    let (w, h) = fp.trim_inches();
    args.push("-V".into());
    args.push(format!("geometry:paperwidth={w}"));
    args.push("-V".into());
    args.push(format!("geometry:paperheight={h}"));

    // Asymmetric gutter for chapters-on-recto layouts (KDP minimums
    // for novel-length books).  Academic / YA use simpler symmetric
    // margins to save paper.
    if fp.chapter_starts_recto() {
        args.push("-V".into());
        args.push("geometry:inner=0.75in".into());
        args.push("-V".into());
        args.push("geometry:outer=0.5in".into());
    } else {
        args.push("-V".into());
        args.push("geometry:inner=0.7in".into());
        args.push("-V".into());
        args.push("geometry:outer=0.5in".into());
    }
    args.push("-V".into());
    args.push("geometry:top=0.75in".into());
    args.push("-V".into());
    args.push("geometry:bottom=0.75in".into());

    args.push("-V".into());
    args.push(format!("documentclass={}", fp.pandoc_documentclass()));
    args.push("-V".into());
    args.push(format!("classoption={}", fp.pandoc_classoption()));

    // Body + heading fonts come straight from the Google Font bundle
    // (BACKLOG §H8.2).  xelatex resolves these by family name from
    // the system's installed fonts — the writer needs to install the
    // bundle locally (or wait for the bundled-font follow-up that
    // ships them with the desktop app under
    // `<resources>/fonts/<family>.ttf`).  When fonts aren't installed,
    // xelatex prints a clear "font not found" error rather than
    // silently substituting.
    let body_font_name = fp.google_body_family();
    let heading_font_name = fp.google_heading_family();
    args.push("-V".into());
    args.push(format!("mainfont={body_font_name}"));
    // When the bundled font directory is supplied, also emit the
    // fontspec `Path=` / `UprightFont=` / `ItalicFont=` options so
    // xelatex resolves the font from our bundle rather than the
    // writer's system install.  Variable-weight TTFs (`<Family>[wght].ttf`
    // and `<Family>-Italic[wght].ttf`) cover regular, bold, italic,
    // and bold-italic in one file each.
    if let Some(dir) = bundle {
        push_fontspec_options(args, "mainfontoptions", dir, body_font_name);
    }
    if heading_font_name != body_font_name {
        // For sans-serif heads, use `sansfont`; otherwise no override
        // (xelatex will use mainfont for the heading style too).
        let is_sans = matches!(heading_font_name, "Inter" | "Source Sans 3");
        if is_sans {
            args.push("-V".into());
            args.push(format!("sansfont={heading_font_name}"));
            if let Some(dir) = bundle {
                push_fontspec_options(args, "sansfontoptions", dir, heading_font_name);
            }
        }
    }

    args.push("-V".into());
    args.push(format!("fontsize={}", fp.body_size_pt()));
    args.push("-V".into());
    args.push(format!("linestretch={}", fp.line_height()));

    args.push("-V".into());
    args.push(format!(
        "header-includes={}",
        header_includes_for_format(fp)
    ));

    args.push("--top-level-division=chapter".into());
    if fp.pdf_toc() {
        args.push("--toc".into());
        args.push("--toc-depth=2".into());
    }
    args.push("--pdf-engine=xelatex".into());
}

/// Emit `-V <key>:Path=...` / `UprightFont=...` / `ItalicFont=...`
/// pairs that xelatex's `fontspec` package understands, pointing at
/// the bundled Google Font directory laid down by
/// `scripts/fetch-fonts.sh`.
///
/// Pandoc concatenates each `--variable mainfontoptions:<value>` into
/// a comma-separated option list passed to `\setmainfont`.  We use
/// the same convention for `sansfontoptions`.
///
/// Variable-weight TTFs (e.g. `EBGaramond[wght].ttf`) cover regular,
/// bold, italic, and bold-italic in one file each — fontspec routes
/// the right glyphs based on the requested weight.  For static-only
/// families (Cormorant Garamond) we point at the variable-weight
/// build that ships in google/fonts (also `<Family>[wght].ttf`).
fn push_fontspec_options(args: &mut Vec<String>, key: &str, bundle_dir: &str, family: &str) {
    let dir_name = family.replace(' ', "_");
    let stem = family.replace(' ', "");
    let dir_path = format!("{bundle_dir}/{dir_name}/");
    args.push("-V".into());
    args.push(format!("{key}:Path={dir_path}"));
    args.push("-V".into());
    args.push(format!("{key}:Extension=.ttf"));

    // Variable-weight `[wght]` files come first; some families also
    // expose `[opsz,wght]`.  fontspec tries the literal name on disk,
    // so we need to pick whichever variant is actually present in the
    // bundle.  The `_with_opsz` heuristic mirrors the EPUB
    // `collect_bundled_fonts` lookup.
    let has_opsz = matches!(family, "Source Serif 4" | "Inter");
    let upright_suffix = if has_opsz { "[opsz,wght]" } else { "[wght]" };
    let italic_suffix = if has_opsz {
        "-Italic[opsz,wght]"
    } else {
        "-Italic[wght]"
    };
    args.push("-V".into());
    args.push(format!("{key}:UprightFont={stem}{upright_suffix}"));
    args.push("-V".into());
    args.push(format!("{key}:ItalicFont={stem}{italic_suffix}"));
}

/// Pull the first font name out of a CSS-style font stack.
/// `"Adobe Garamond Pro", Garamond, Georgia, serif` → `Adobe Garamond Pro`.
/// Strips surrounding double-quotes if present.
///
/// Retained for the legacy `body_font_family()` parser path used by
/// older tests; the H8.2 PDF runner uses `google_body_family()`
/// directly.
#[allow(dead_code)]
fn first_font_in_stack(stack: &str) -> String {
    let first = stack.split(',').next().unwrap_or(stack).trim();
    let stripped = first.trim_matches('"');
    stripped.to_owned()
}

#[allow(dead_code)]
fn push_pdf_geometry(args: &mut Vec<String>, w: &str, h: &str) {
    // ── Trade paperback PDF defaults ──────────────────────────────────
    //
    // These match the most common KDP / IngramSpark trim sizes and the
    // typography you'd see in a published novel:
    //
    //   - **memoir** documentclass — designed for trade-book layouts;
    //     gives us proper running headers, drop folios, and chapter
    //     starts on the recto (right) page automatically.
    //   - **twoside** so the gutter and outer margins flip per spread
    //     (a binding that lays flat needs the extra space on the
    //     gutter side).
    //   - **inner=0.75in / outer=0.5in** — KDP's bleed-aware gutter
    //     minimum is 0.5in for books up to 150 pages, 0.625in
    //     151-300, 0.75in 301-500, 0.875in 501-700, 1.0in 701+.
    //     We pick 0.75in as a safe default for novel-length manuscripts.
    //   - **top=0.75in / bottom=0.75in** — comfortable for 5×8 / 6×9.
    //   - **mainfont=Georgia** — bundled on macOS / Windows by default;
    //     a user with custom fonts can override via a future template.
    //   - **\setlength{\parindent}{1.2em}** in `header-includes` —
    //     trade-paperback paragraph indent.
    //   - `--toc` adds a table of contents from the chapter headings.
    //   - `--top-level-division=chapter` makes H1 headings start on a
    //     fresh recto page (the trade-book convention).
    args.push("-V".into());
    args.push(format!("geometry:paperwidth={w}"));
    args.push("-V".into());
    args.push(format!("geometry:paperheight={h}"));
    args.push("-V".into());
    args.push("geometry:inner=0.75in".into());
    args.push("-V".into());
    args.push("geometry:outer=0.5in".into());
    args.push("-V".into());
    args.push("geometry:top=0.75in".into());
    args.push("-V".into());
    args.push("geometry:bottom=0.75in".into());
    args.push("-V".into());
    args.push("documentclass=memoir".into());
    args.push("-V".into());
    args.push("classoption=twoside,openright".into());
    args.push("-V".into());
    args.push("mainfont=Georgia".into());
    args.push("-V".into());
    args.push("fontsize=11pt".into());
    args.push("-V".into());
    args.push("linestretch=1.15".into());

    // Header includes for typography niceties — paragraph indent +
    // suppress orphans/widows where the renderer supports it.
    args.push("-V".into());
    args.push(r"header-includes=\usepackage{microtype}\usepackage{ragged2e}\setlength{\parindent}{1.2em}\widowpenalty=10000\clubpenalty=10000".into());

    args.push("--top-level-division=chapter".into());
    args.push("--toc".into());
    args.push("--toc-depth=2".into());
    args.push("--pdf-engine=xelatex".into());
}

/// Invoke Pandoc to produce DOCX or PDF output.
///
/// Pipes `markdown_source` over stdin; reads exit status; verifies
/// `output_path` exists and is non-empty; computes blake3.
pub async fn run_pandoc(input: PandocInput) -> Result<ExportOutcome, ExportError> {
    if !matches!(
        input.profile,
        ExportProfile::Docx | ExportProfile::TradePdf5x8 | ExportProfile::TradePdf6x9,
    ) {
        return Err(ExportError::Failed {
            message: format!("Pandoc cannot handle profile {:?}", input.profile),
        });
    }
    if !Path::new(&input.pandoc_binary).is_file() {
        return Err(ExportError::SidecarMissing {
            binary: input.pandoc_binary.clone(),
        });
    }

    // Make sure the output directory exists.
    if let Some(parent) = Path::new(&input.output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ExportError::Failed {
                    message: format!("could not create output dir: {e}"),
                })?;
        }
    }

    let args = args_for_profile_full(
        input.profile,
        &input.output_path,
        input.docx_template.as_deref(),
        input.format_profile,
        input.font_bundle_dir.as_deref(),
    );

    tracing::info!(
        binary = %input.pandoc_binary,
        out    = %input.output_path,
        profile = ?input.profile,
        "spawning Pandoc"
    );

    let mut child = Command::new(&input.pandoc_binary)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ExportError::SidecarMissing {
            binary: format!("{}: {e}", input.pandoc_binary),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.markdown_source.as_bytes())
            .await
            .map_err(|e| ExportError::Failed {
                message: format!("failed to pipe markdown to Pandoc stdin: {e}"),
            })?;
        // Drop closes the pipe.
    }

    let out = child
        .wait_with_output()
        .await
        .map_err(|e| ExportError::Failed {
            message: format!("Pandoc child failed: {e}"),
        })?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(ExportError::Failed {
            message: format!(
                "Pandoc exited with code {}: {}",
                out.status.code().unwrap_or(-1),
                stderr.lines().take(20).collect::<Vec<_>>().join(" | "),
            ),
        });
    }

    let bytes = tokio::fs::read(&input.output_path)
        .await
        .map_err(|e| ExportError::Failed {
            message: format!("Pandoc reported success but output file is missing: {e}"),
        })?;
    if bytes.is_empty() {
        return Err(ExportError::Failed {
            message: "Pandoc produced an empty file".to_owned(),
        });
    }
    let hash = blake3::hash(&bytes).to_hex().to_string();

    Ok(ExportOutcome {
        profile: input.profile,
        output_path: input.output_path,
        hash,
    })
}

/// Resolve the path to the bundled Pandoc sidecar binary.
pub fn sidecar_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "pandoc-3.5.exe"
    } else {
        "pandoc-3.5"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_for_docx() {
        let a = args_for_profile(ExportProfile::Docx, "/tmp/out.docx", None);
        assert!(a.contains(&"-f".to_owned()));
        assert!(a.contains(&"markdown".to_owned()));
        assert!(a.contains(&"docx".to_owned()));
        assert!(a.contains(&"/tmp/out.docx".to_owned()));
    }

    #[test]
    fn args_for_docx_with_reference_template() {
        let a = args_for_profile(ExportProfile::Docx, "/tmp/o.docx", Some("/x/t.docx"));
        assert!(a.iter().any(|s| s == "--reference-doc=/x/t.docx"));
    }

    #[test]
    fn args_for_pdf_5x8_includes_geometry() {
        // FormatProfile drives trim — pass the mass-market profile explicitly.
        let a = args_for_profile_with_format(
            ExportProfile::TradePdf5x8,
            "/tmp/o.pdf",
            None,
            FormatProfile::FictionTradeMass,
        );
        let joined = a.join(" ");
        assert!(joined.contains("geometry:paperwidth=5in"));
        assert!(joined.contains("geometry:paperheight=8in"));
        assert!(a.iter().any(|s| s == "--pdf-engine=xelatex"));
    }

    #[test]
    fn args_for_pdf_6x9_includes_geometry() {
        // Default FormatProfile is FictionTradeStandard (6×9).
        let a = args_for_profile(ExportProfile::TradePdf6x9, "/tmp/o.pdf", None);
        let joined = a.join(" ");
        assert!(joined.contains("paperwidth=6in"));
        assert!(joined.contains("paperheight=9in"));
    }

    #[tokio::test]
    async fn run_pandoc_rejects_missing_binary() {
        let r = run_pandoc(PandocInput {
            pandoc_binary: "/nonexistent/path/to/pandoc".into(),
            markdown_source: "# hi".into(),
            docx_template: None,
            profile: ExportProfile::Docx,
            output_path: "/tmp/should_not_exist.docx".into(),
            format_profile: FormatProfile::default(),
            font_bundle_dir: None,
        })
        .await;
        assert!(matches!(r, Err(ExportError::SidecarMissing { .. })));
    }

    #[tokio::test]
    async fn run_pandoc_rejects_unsupported_profile() {
        let r = run_pandoc(PandocInput {
            pandoc_binary: "/usr/bin/true".into(), // exists, just not pandoc
            markdown_source: "# hi".into(),
            docx_template: None,
            profile: ExportProfile::Markdown, // unsupported here
            output_path: "/tmp/.x".into(),
            format_profile: FormatProfile::default(),
            font_bundle_dir: None,
        })
        .await;
        assert!(matches!(r, Err(ExportError::Failed { .. })));
    }

    #[test]
    fn pdf_args_pick_trim_from_format_profile() {
        // YA profile uses 5.5×8.5; literary fiction uses 6×9.
        let ya = args_for_profile_with_format(
            ExportProfile::TradePdf6x9,
            "/tmp/o.pdf",
            None,
            FormatProfile::FictionYoungAdult,
        );
        let joined = ya.join(" ");
        assert!(joined.contains("paperwidth=5.5in"));
        assert!(joined.contains("paperheight=8.5in"));
        // Body size should be 12pt for YA.
        assert!(joined.contains("fontsize=12pt"));
        // YA uses `book` not `memoir`.
        assert!(joined.contains("documentclass=book"));
    }

    #[test]
    fn pdf_args_emit_toc_for_non_fiction_only() {
        let fiction = args_for_profile_with_format(
            ExportProfile::TradePdf6x9,
            "/tmp/o.pdf",
            None,
            FormatProfile::FictionTradeStandard,
        );
        let academic = args_for_profile_with_format(
            ExportProfile::TradePdf6x9,
            "/tmp/o.pdf",
            None,
            FormatProfile::Academic,
        );
        assert!(!fiction.iter().any(|a| a == "--toc"));
        assert!(academic.iter().any(|a| a == "--toc"));
    }

    #[test]
    fn first_font_in_stack_strips_quotes() {
        assert_eq!(
            first_font_in_stack(r#""Adobe Garamond Pro", Garamond, serif"#),
            "Adobe Garamond Pro"
        );
        assert_eq!(first_font_in_stack("Georgia, serif"), "Georgia");
    }

    #[test]
    fn pdf_args_emit_fontspec_path_when_bundle_supplied() {
        // RomanceHistorical → Cormorant Garamond body+heading.
        let a = args_for_profile_full(
            ExportProfile::TradePdf6x9,
            "/tmp/o.pdf",
            None,
            FormatProfile::RomanceHistorical,
            Some("/abs/font/bundle"),
        );
        let joined = a.join(" ");
        assert!(
            joined.contains("mainfontoptions:Path=/abs/font/bundle/Cormorant_Garamond/"),
            "expected mainfontoptions Path; got: {joined}"
        );
        assert!(
            joined.contains("mainfontoptions:UprightFont=CormorantGaramond[wght]"),
            "expected UprightFont; got: {joined}"
        );
        assert!(
            joined.contains("mainfontoptions:ItalicFont=CormorantGaramond-Italic[wght]"),
            "expected ItalicFont; got: {joined}"
        );
    }

    #[test]
    fn pdf_args_use_opsz_axis_for_inter_and_source_serif() {
        let a = args_for_profile_full(
            ExportProfile::TradePdf6x9,
            "/tmp/o.pdf",
            None,
            FormatProfile::NonFictionPractical, // Source Serif 4 + Inter
            Some("/x/y"),
        );
        let joined = a.join(" ");
        assert!(
            joined.contains("UprightFont=SourceSerif4[opsz,wght]"),
            "expected opsz,wght for Source Serif 4: {joined}"
        );
        // Non-fiction practical sets sansfont = Inter.
        assert!(
            joined.contains("sansfontoptions:UprightFont=Inter[opsz,wght]"),
            "expected opsz,wght for Inter: {joined}"
        );
    }

    #[test]
    fn pdf_args_omit_fontspec_when_no_bundle() {
        let a = args_for_profile_with_format(
            ExportProfile::TradePdf6x9,
            "/tmp/o.pdf",
            None,
            FormatProfile::FictionTradeStandard,
        );
        let joined = a.join(" ");
        assert!(
            !joined.contains("mainfontoptions:Path="),
            "fontspec opts should be omitted without bundle: {joined}"
        );
    }

    #[test]
    fn sidecar_binary_is_platform_specific() {
        let n = sidecar_binary_name();
        assert!(n == "pandoc-3.5" || n == "pandoc-3.5.exe");
    }
}
