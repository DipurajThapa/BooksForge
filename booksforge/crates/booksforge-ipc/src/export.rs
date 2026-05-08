//! IPC types for the manuscript export pipeline.
//!
//! M0 ships the Markdown profile only — the full Pandoc/EPUB pipeline arrives
//! in M5.  Once it does, the same `export_run` command will accept a richer
//! profile arg; for now we expose a dedicated `export_markdown` to keep the
//! UI affordance simple.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input to `export_markdown`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportMarkdownInput {
    /// Absolute path the file will be written to.  The user picks this with
    /// the OS save-file dialog (handled in the frontend).
    pub output_path: String,
}

/// Result of `export_markdown` — counters useful for the success toast and
/// sanity checks.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportMarkdownResult {
    pub export_id:     String,
    pub output_path:   String,
    pub bytes:         u64,
    pub part_count:    u32,
    pub chapter_count: u32,
    pub scene_count:   u32,
    pub word_count:    u32,
    /// blake3 hex of the rendered bytes (matches `exports.hash`).
    pub hash:          String,
}

/// Input to the unified `export_run` command (Phase 6).
///
/// `profile` is one of: `"markdown"`, `"docx"`, `"generic_epub"`,
/// `"kdp_ebook"`, `"trade_pdf_5x8"`, `"trade_pdf_6x9"`.  The wire form
/// is a string so the UI can drive it from a `<select>` without a
/// per-profile typed dispatch.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportRunInput {
    pub profile:     String,
    pub output_path: String,
    /// Genre-aware typography profile.  String form so the UI can drive
    /// it from a `<select>`.  One of: `"fiction_trade_mass"`,
    /// `"fiction_trade_standard"` (default), `"fiction_literary"`,
    /// `"fiction_young_adult"`, `"non_fiction_practical"`,
    /// `"non_fiction_memoir"`, `"academic"`.  Empty / unknown values
    /// fall back to `fiction_trade_standard`.
    #[serde(default)]
    pub format_profile: Option<String>,
}

/// Result of `export_run` — single shape across all profiles.  EPUB
/// runs additionally include validation summary fields if EPUBCheck
/// was available; DOCX/PDF runs return them empty.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportRunResult {
    pub export_id:     String,
    pub profile:       String,
    pub output_path:   String,
    pub bytes:         u64,
    pub hash:          String,
    /// True if EPUBCheck ran and reported zero ERROR/FATAL issues.
    /// Always `true` for non-EPUB profiles.  False if EPUBCheck is
    /// unavailable on this machine — `validation_message` carries the
    /// reason so the UI can show "Local-only — install Java + EPUBCheck
    /// to validate".
    pub validation_ok:      bool,
    pub validation_message: Option<String>,
    pub error_count:        u32,
    pub warning_count:      u32,
}

/// Status of an external binary the export pipeline depends on.
/// Surfaced to the UI so the user can see "Pandoc 3.5.0" vs
/// "Pandoc not found — install to enable DOCX/PDF" without dispatching
/// an export to discover it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportDependencyStatus {
    /// Identifier: `"pandoc"` | `"java"` | `"epubcheck"`.
    pub id:        String,
    /// Display name (e.g. "Pandoc").
    pub name:      String,
    /// True if the binary / JAR was found.
    pub found:     bool,
    /// Resolved path (binary) when found.  Empty when missing.
    pub path:      String,
    /// Version string when probable; empty when not.
    pub version:   String,
    /// Profiles unlocked by this dependency.  UI uses to grey out
    /// disabled cards.
    pub unlocks:   Vec<String>,
    /// One-line hint for the user when the dependency is missing
    /// (where to download / how to install).
    pub install_hint: String,
}

/// Aggregate response of `export_check_dependencies`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportDependencyReport {
    pub items: Vec<ExportDependencyStatus>,
}

/// One row from the `exports` ledger, IPC-safe.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportHistoryEntry {
    pub id:          String,
    pub profile:     String,
    pub output_path: String,
    pub hash:        String,
    /// ISO-8601 created_at.
    pub created_at:  String,
}
