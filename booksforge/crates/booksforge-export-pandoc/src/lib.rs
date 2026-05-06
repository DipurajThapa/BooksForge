//! Pandoc sidecar runner for DOCX and PDF export (Layer 4 — infrastructure).
//!
//! Pandoc is bundled as a sidecar binary in the Tauri app bundle.  This crate
//! locates it via the Tauri resource resolver, validates it is executable,
//! then invokes it with the appropriate arguments for DOCX or PDF output.
//!
//! Pandoc is NOT used for EPUB — the canonical EPUB pipeline lives in
//! `booksforge-export-epub`.
//!
//! Full implementation in M5.  See EXPORT_EPUB_SPEC.md.

#![forbid(unsafe_code)]

use booksforge_export::{ExportError, ExportOutcome, ExportProfile};

/// Input to the Pandoc export runner.
#[derive(Debug, Clone)]
pub struct PandocInput {
    /// Absolute path to the Pandoc binary (resolved by the Tauri sidecar API).
    pub pandoc_binary: String,
    /// Canonical Markdown source for the full manuscript.
    pub markdown_source: String,
    /// Reference `.docx` template path (for DOCX output).
    pub docx_template: Option<String>,
    /// Target profile — must be `TradePdf5x8`, `TradePdf6x9`, or `Docx`.
    pub profile: ExportProfile,
    /// Absolute path where the output file should be written.
    pub output_path: String,
}

/// Invoke Pandoc to produce DOCX or PDF output.
///
/// Returns an `ExportOutcome` with the output path and blake3 hash.
/// This is a stub; full implementation in M5.
pub async fn run_pandoc(_input: PandocInput) -> Result<ExportOutcome, ExportError> {
    // M5 implementation: spawn Pandoc as a Tokio child process, capture
    // stderr for diagnostics, verify the output file exists, compute blake3.
    todo!("Pandoc runner — M5")
}

/// Resolve the path to the bundled Pandoc sidecar binary.
///
/// On macOS the sidecar is at `<app>.app/Contents/MacOS/pandoc-3.5`.
/// On Windows it is at `<app dir>/pandoc-3.5.exe`.
pub fn sidecar_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "pandoc-3.5.exe"
    } else {
        "pandoc-3.5"
    }
}
