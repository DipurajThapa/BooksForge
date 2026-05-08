//! Export ledger records (`exports` table).
//!
//! Domain-only types — the `booksforge-export` crate produces the bytes,
//! the storage layer persists this row, and the IPC layer surfaces it
//! to the UI for the export-history list.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Export-format profile.  M0 ships only `Markdown`; the others arrive in
/// M5.  Mirrors the `exports.profile` CHECK constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportProfile {
    Markdown,
    Docx,
    GenericEpub,
    KdpEbook,
    TradePdf5x8,
    TradePdf6x9,
}

impl ExportProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown    => "markdown",
            Self::Docx        => "docx",
            Self::GenericEpub => "generic_epub",
            Self::KdpEbook    => "kdp_ebook",
            Self::TradePdf5x8 => "trade_pdf_5x8",
            Self::TradePdf6x9 => "trade_pdf_6x9",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "markdown"      => Some(Self::Markdown),
            "docx"          => Some(Self::Docx),
            "generic_epub"  => Some(Self::GenericEpub),
            "kdp_ebook"     => Some(Self::KdpEbook),
            "trade_pdf_5x8" => Some(Self::TradePdf5x8),
            "trade_pdf_6x9" => Some(Self::TradePdf6x9),
            _ => None,
        }
    }
}

/// One row in the `exports` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecord {
    pub id:          Ulid,
    pub profile:     ExportProfile,
    pub output_path: String,
    /// blake3 hex of the rendered bytes (drives reproducibility checks).
    pub hash:        String,
    pub created_at:  DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_roundtrip() {
        for p in [
            ExportProfile::Markdown, ExportProfile::Docx,
            ExportProfile::GenericEpub, ExportProfile::KdpEbook,
            ExportProfile::TradePdf5x8, ExportProfile::TradePdf6x9,
        ] {
            assert_eq!(ExportProfile::from_str(p.as_str()), Some(p));
        }
    }
}
