//! Export DOM and canonical-HTML pipeline (Layer 3 — pure logic).
//!
//! The canonical HTML is the single export source for EPUB-3. The editor
//! preview renders the same HTML. Any drift between them is a CI failure.
//!
//! Pandoc handles DOCX and PDF only; this crate handles the canonical-HTML
//! representation that feeds both the preview and the EPUB packager.
//!
//! Full implementation in M5. See EXPORT_EPUB_SPEC.md.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// The export profile determines template, page size, and target store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportProfile {
    /// KDP-compatible EPUB-3 (primary MVP target).
    KdpEbook,
    /// Generic EPUB-3 (no store-specific constraints).
    GenericEpub,
    /// Trade paperback 5×8 PDF.
    TradePdf5x8,
    /// Trade paperback 6×9 PDF.
    TradePdf6x9,
    /// DOCX for manuscript submission / external editing.
    Docx,
}

/// Result of an export job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOutcome {
    pub profile:   ExportProfile,
    /// Absolute path to the output file inside the bundle's `exports/` dir.
    pub output_path: String,
    /// blake3 hash of the output file (for reproducibility assertions).
    pub hash:      String,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("sidecar '{binary}' not found or not executable")]
    SidecarMissing { binary: String },
    #[error("EPUBCheck failed with {error_count} errors")]
    EpubCheckFailed { error_count: usize },
    #[error("export failed: {message}")]
    Failed { message: String },
}
