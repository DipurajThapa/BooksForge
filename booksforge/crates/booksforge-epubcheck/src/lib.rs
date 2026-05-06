//! EPUBCheck sidecar runner (Layer 4 — infrastructure).
//!
//! EPUBCheck 5.1.0 is bundled as a JAR sidecar and invoked via `java -jar`.
//! This crate parses the JSON report output and fails the export job if any
//! ERROR-severity issues are found.
//!
//! Full implementation in M5.  See EXPORT_EPUB_SPEC.md.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Severity of an EPUBCheck issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

/// A single issue from the EPUBCheck JSON report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubCheckIssue {
    pub severity:  IssueSeverity,
    pub message:   String,
    pub path:      Option<String>,
    pub line:      Option<u32>,
    pub column:    Option<u32>,
}

/// The parsed EPUBCheck report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubCheckReport {
    pub checker_version: String,
    pub epub_path:        String,
    pub issues:           Vec<EpubCheckIssue>,
}

impl EpubCheckReport {
    /// Returns `true` when there are no ERROR or FATAL issues.
    pub fn is_valid(&self) -> bool {
        !self.issues.iter().any(|i| {
            matches!(i.severity, IssueSeverity::Error | IssueSeverity::Fatal)
        })
    }

    /// Count of issues at ERROR or FATAL severity.
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Error | IssueSeverity::Fatal))
            .count()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EpubCheckError {
    #[error("Java not found — EPUBCheck requires a JRE on PATH")]
    JavaNotFound,

    #[error("EPUBCheck JAR not found at: {path}")]
    JarNotFound { path: String },

    #[error("EPUBCheck process failed with exit code {code}")]
    ProcessFailed { code: i32 },

    #[error("failed to parse EPUBCheck report: {reason}")]
    ReportParseError { reason: String },

    #[error("EPUBCheck found {error_count} error(s)")]
    ValidationErrors { error_count: usize },
}

/// Run EPUBCheck on the given EPUB file path.
///
/// Returns the parsed report.  Callers should check `report.is_valid()`.
/// This is a stub; full implementation in M5.
pub async fn run_epubcheck(_epub_path: &str, _jar_path: &str) -> Result<EpubCheckReport, EpubCheckError> {
    // M5 implementation: spawn `java -jar <jar_path> --json <epub_path>`,
    // capture stdout, parse JSON into EpubCheckReport.
    todo!("EPUBCheck runner — M5")
}
