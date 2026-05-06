//! EPUB-3 packager (Layer 4 — infrastructure).
//!
//! Converts the canonical HTML produced by the `booksforge-export` pipeline
//! into a valid EPUB-3 archive targeting KDP and generic stores.
//!
//! The packager writes a deterministic ZIP so that builds with identical
//! content produce byte-identical output (useful for reproducibility tests).
//!
//! Full implementation in M5.  See EXPORT_EPUB_SPEC.md.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use booksforge_export::{ExportOutcome, ExportProfile};

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
}

/// One chapter's canonical HTML for inclusion in the EPUB manifest.
#[derive(Debug, Clone)]
pub struct HtmlChapter {
    pub node_id:   String,
    pub title:     String,
    pub html_body: String,
}

/// EPUB metadata block derived from `ProjectMeta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubMetadata {
    pub title:       String,
    pub authors:     Vec<String>,
    pub language:    String,
    pub publisher:   Option<String>,
    pub description: Option<String>,
    pub isbn:        Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EpubError {
    #[error("unsupported profile for EPUB export: {profile:?}")]
    UnsupportedProfile { profile: ExportProfile },

    #[error("no chapters provided — cannot build an empty EPUB")]
    NoChapters,

    #[error("I/O error writing EPUB to {path}: {source}")]
    Io { path: String, source: std::io::Error },

    #[error("EPUBCheck validation failed — see logs for details")]
    ValidationFailed,
}

/// Build an EPUB-3 archive from the given input.
///
/// Returns an `ExportOutcome` with the output path and blake3 hash.
/// This is a stub; full implementation in M5.
pub async fn build_epub(_input: EpubPackageInput) -> Result<ExportOutcome, EpubError> {
    // M5 implementation: assemble OPF, NCX, HTML chapters, CSS, and META-INF
    // into a deterministic ZIP using the `zip` crate.
    todo!("EPUB packager — M5")
}
