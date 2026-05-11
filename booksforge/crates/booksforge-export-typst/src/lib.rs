//! Typst subprocess runner for PDF interior generation (Layer 4).
//!
//! BooksForge bundles `typst` 0.14+ as a Tauri sidecar; this crate is the
//! cross-platform Rust shim that:
//!
//!   1. Resolves the `typst` binary path (caller-supplied — Tauri sidecar
//!      resource resolver in production, system PATH for dev).
//!   2. Renders a manuscript Markdown source to PDF via a small fixed
//!      Typst template (KDP trim sizes 5×8 / 5.5×8.5 / 6×9, with
//!      configurable interior margins).
//!   3. Captures stderr for diagnostics; verifies the output file exists;
//!      computes blake3 (drives reproducibility checks).
//!
//! Replaces the previous reliance on `xelatex` for PDF generation. xelatex
//! is GPL-adjacent (LaTeX licensing is fine but the install footprint is
//! large), and is rarely present on macOS without a TeX distro install.
//! Typst is Apache-2.0, ships as a single ~30 MB binary, and is what the
//! BF-E2E test successfully used to render `manuscript.pdf`.
//!
//! ## Trim sizes
//!
//! KDP trade-paperback options the renderer supports:
//!
//! | profile         | width | height | inner margin | outer margin | top/bottom |
//! |-----------------|-------|--------|--------------|--------------|------------|
//! | `TradePdf5x8`   | 5in   | 8in    | 0.625in      | 0.50in       | 0.625in    |
//! | `TradePdf5_5x8_5` | 5.5in | 8.5in  | 0.625in      | 0.50in       | 0.625in    |
//! | `TradePdf6x9`   | 6in   | 9in    | 0.75in       | 0.50in       | 0.75in     |
//!
//! Bleed and full-cover wrap are out of scope here (interior only).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use booksforge_export::{ExportError, ExportOutcome};
use std::path::Path;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

/// Trim-size profile for the PDF interior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypstTrim {
    /// 5"×8" mass-market paperback.
    Trade5x8,
    /// 5.5"×8.5" trade paperback.
    Trade5_5x8_5,
    /// 6"×9" trade paperback (KDP default for fiction).
    Trade6x9,
    /// US Letter (8.5"×11") — for proofs and reading copies, not KDP.
    UsLetter,
}

impl TypstTrim {
    /// (width_in, height_in, inner_margin_in, outer_margin_in, vertical_margin_in)
    fn dimensions(self) -> (f32, f32, f32, f32, f32) {
        match self {
            Self::Trade5x8 => (5.0, 8.0, 0.625, 0.50, 0.625),
            Self::Trade5_5x8_5 => (5.5, 8.5, 0.625, 0.50, 0.625),
            Self::Trade6x9 => (6.0, 9.0, 0.75, 0.50, 0.75),
            Self::UsLetter => (8.5, 11.0, 1.0, 1.0, 1.0),
        }
    }
}

/// Input to the Typst export runner.
#[derive(Debug, Clone)]
pub struct TypstInput {
    /// Absolute path to the `typst` binary. Either resolved from the Tauri
    /// sidecar resource path or `typst_on_path()` for development.
    pub typst_binary: String,
    /// The manuscript markdown source. The runner converts it to a small
    /// Typst program before compilation; the user's prose appears between
    /// the page-setup preamble and the document close.
    pub markdown_source: String,
    /// Absolute path where the output PDF should be written.
    pub output_path: String,
    /// Trim profile.
    pub trim: TypstTrim,
    /// Title shown on the title page.
    pub title: String,
    /// Author byline.
    pub author: String,
}

/// Resolve `typst` on the system PATH. Used for development; the shipping
/// build uses the Tauri sidecar resource resolver instead.
pub fn typst_on_path() -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join("typst");
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        let exe = candidate.with_extension("exe");
        if exe.is_file() {
            return Some(exe.to_string_lossy().into_owned());
        }
    }
    None
}

/// Verify that the configured Typst binary is invokable. Runs
/// `typst --version` and returns the version string.
pub async fn probe_typst(binary: &str) -> Result<String, ExportError> {
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
        let status = out.status;
        return Err(ExportError::Failed {
            message: format!("{binary} --version exited {status:?}"),
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first_line = stdout.lines().next().unwrap_or("").trim();
    Ok(first_line.to_owned())
}

/// Render the manuscript to PDF.
///
/// Implementation: builds a Typst program in memory (page setup + title
/// page + per-paragraph body), writes it to a tempfile beside the output
/// path, invokes `typst compile <tmp.typ> <output.pdf>`, captures stderr,
/// and returns a blake3 of the PDF bytes.
pub async fn run_typst(input: TypstInput) -> Result<ExportOutcome, ExportError> {
    let (w, h, inner, outer, vert) = input.trim.dimensions();
    let typst_source = build_typst_source(&input, w, h, inner, outer, vert);

    // Write the Typst program to a sibling tempfile so user prose never
    // leaves the output directory's filesystem.
    let typ_path = format!("{}.typ", input.output_path);
    {
        let mut f = tokio::fs::File::create(&typ_path)
            .await
            .map_err(|e| ExportError::Failed {
                message: format!("create temp .typ: {e}"),
            })?;
        f.write_all(typst_source.as_bytes())
            .await
            .map_err(|e| ExportError::Failed {
                message: format!("write temp .typ: {e}"),
            })?;
        f.flush().await.map_err(|e| ExportError::Failed {
            message: format!("flush temp .typ: {e}"),
        })?;
    }

    let result = Command::new(&input.typst_binary)
        .arg("compile")
        .arg(&typ_path)
        .arg(&input.output_path)
        .output()
        .await
        .map_err(|e| ExportError::Failed {
            message: format!("typst compile launch: {e}"),
        })?;

    // Best-effort cleanup of the temp .typ file. We deliberately don't
    // propagate cleanup errors — the PDF (or its absence) is the result.
    let _ = tokio::fs::remove_file(&typ_path).await;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let status = result.status;
        let snippet = stderr.lines().take(8).collect::<Vec<_>>().join(" | ");
        return Err(ExportError::Failed {
            message: format!("typst compile exited {status:?}: {snippet}"),
        });
    }

    let bytes = tokio::fs::read(&input.output_path)
        .await
        .map_err(|e| ExportError::Failed {
            message: format!("read produced PDF: {e}"),
        })?;
    let hash = blake3::hash(&bytes).to_hex().to_string();

    tracing::info!(
        engine = "typst",
        output = %input.output_path,
        bytes = bytes.len(),
        hash = %hash,
        "typst PDF render complete",
    );

    Ok(ExportOutcome {
        profile: booksforge_export::ExportProfile::TradePdf6x9,
        output_path: input.output_path,
        hash,
    })
}

/// Build the Typst program. Pure logic — split out for unit tests.
fn build_typst_source(
    input: &TypstInput,
    width_in: f32,
    height_in: f32,
    inner_in: f32,
    outer_in: f32,
    vert_in: f32,
) -> String {
    // Escape backslashes and quote chars in user-supplied strings for
    // safe Typst-string interpolation. Typst strings use the same escape
    // rules as Rust string literals at this level.
    let title = escape_typst_string(&input.title);
    let author = escape_typst_string(&input.author);
    let body = markdown_to_typst_body(&input.markdown_source);

    format!(
        r#"// Generated by booksforge-export-typst.
#set document(title: "{title}", author: "{author}")
#set page(
  width: {width_in}in,
  height: {height_in}in,
  margin: (
    inside: {inner_in}in,
    outside: {outer_in}in,
    top: {vert_in}in,
    bottom: {vert_in}in,
  ),
  numbering: "1",
)
#set par(justify: true, leading: 0.65em)
#set text(font: "New Computer Modern", size: 11pt)

#align(center)[
  #v(2in)
  #text(size: 24pt, weight: "bold")[{title}]
  #v(0.5in)
  #text(size: 14pt, style: "italic")[{author}]
]

#pagebreak()

{body}
"#
    )
}

/// Convert manuscript markdown to a Typst body. We deliberately implement a
/// narrow subset rather than depend on a full markdown parser:
///
///   - `# H1` → `= H1` (chapter heading; forces page break before)
///   - `## H2` → `== H2`
///   - blank line → paragraph break
///   - `* * *` or `---` → centred section break
///   - everything else → paragraph text (whitespace preserved within paragraphs)
///
/// This matches what the BooksForge canonical-markdown export emits.
fn markdown_to_typst_body(md: &str) -> String {
    let mut out = String::new();
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current = String::new();

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
            continue;
        }
        // Block-level constructs
        if let Some(rest) = trimmed.strip_prefix("# ") {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
            paragraphs.push(format!("__CHAPTER__{rest}"));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
            paragraphs.push(format!("__H2__{rest}"));
            continue;
        }
        if trimmed == "* * *" || trimmed == "---" || trimmed == "***" {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
            paragraphs.push("__BREAK__".to_owned());
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(trimmed);
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }

    for p in paragraphs {
        if let Some(rest) = p.strip_prefix("__CHAPTER__") {
            out.push_str(&format!(
                "#pagebreak()\n#v(1in)\n= {}\n\n",
                escape_typst_inline(rest),
            ));
        } else if let Some(rest) = p.strip_prefix("__H2__") {
            out.push_str(&format!("== {}\n\n", escape_typst_inline(rest)));
        } else if p == "__BREAK__" {
            out.push_str("#align(center)[#v(0.5em) * * * #v(0.5em)]\n\n");
        } else {
            out.push_str(&format!("{}\n\n", escape_typst_inline(&p)));
        }
    }
    out
}

/// Escape Typst content-mode special chars so user text doesn't trigger
/// markup interpretation. Conservative: backslash-escapes the chars Typst
/// recognises in content-mode (`#`, `$`, `*`, `_`, `[`, `]`, `<`, `>`,
/// `\``, `\`, `~`).
fn escape_typst_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '#' | '$' | '*' | '_' | '[' | ']' | '<' | '>' | '`' | '~' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape a string destined to appear inside a Typst `"…"` literal.
fn escape_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_match_kdp_trade_options() {
        assert_eq!(TypstTrim::Trade6x9.dimensions().0, 6.0);
        assert_eq!(TypstTrim::Trade6x9.dimensions().1, 9.0);
        assert_eq!(TypstTrim::Trade5x8.dimensions().0, 5.0);
    }

    #[test]
    fn h1_becomes_typst_chapter_heading() {
        let body = markdown_to_typst_body("# Chapter One\n\nThe rain stopped.");
        assert!(body.contains("= Chapter One"));
        assert!(body.contains("#pagebreak()"));
        assert!(body.contains("The rain stopped."));
    }

    #[test]
    fn paragraph_break_separates_paragraphs() {
        let body = markdown_to_typst_body("First paragraph.\n\nSecond paragraph.");
        let paragraphs: Vec<_> = body.split("\n\n").filter(|s| !s.is_empty()).collect();
        assert!(paragraphs.iter().any(|p| p.contains("First paragraph")));
        assert!(paragraphs.iter().any(|p| p.contains("Second paragraph")));
    }

    #[test]
    fn section_break_renders() {
        let body = markdown_to_typst_body("Para one.\n\n* * *\n\nPara two.");
        assert!(body.contains("#align(center)"));
        assert!(body.contains("Para one."));
        assert!(body.contains("Para two."));
    }

    #[test]
    fn typst_specials_are_escaped() {
        let body = markdown_to_typst_body("She paid $100 for the *broken* clock.");
        // $ and * must be escaped so Typst doesn't interpret them as markup.
        assert!(body.contains("\\$"));
        assert!(body.contains("\\*"));
    }

    #[test]
    fn typst_string_escape_handles_quotes_and_backslashes() {
        assert_eq!(escape_typst_string(r#"a "b" c"#), r#"a \"b\" c"#);
        assert_eq!(escape_typst_string(r"a\b"), r"a\\b");
    }

    #[test]
    fn build_typst_source_includes_title_and_author() {
        let input = TypstInput {
            typst_binary: "typst".to_owned(),
            markdown_source: "# Chapter\n\nProse.".to_owned(),
            output_path: "/tmp/x.pdf".to_owned(),
            trim: TypstTrim::Trade6x9,
            title: "The Hour Between".to_owned(),
            author: "Ada Vain".to_owned(),
        };
        let src = build_typst_source(&input, 6.0, 9.0, 0.75, 0.50, 0.75);
        assert!(src.contains("title: \"The Hour Between\""));
        assert!(src.contains("author: \"Ada Vain\""));
        assert!(src.contains("width: 6in"));
        assert!(src.contains("height: 9in"));
        assert!(src.contains("inside: 0.75in"));
    }
}
