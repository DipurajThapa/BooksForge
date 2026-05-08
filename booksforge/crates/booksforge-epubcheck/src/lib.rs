//! EPUBCheck subprocess runner (Layer 4 — infrastructure).
//!
//! EPUBCheck 5.x is bundled as a JAR sidecar and invoked via
//! `java -jar epubcheck.jar --json <report> <epub>`.  This crate is the
//! Rust wrapper that:
//!
//!   1. Verifies `java` is on PATH (or at the supplied path).
//!   2. Verifies the JAR exists.
//!   3. Spawns the process, captures stdout / stderr.
//!   4. Parses EPUBCheck's JSON report into a typed structure.
//!   5. Lets the caller decide whether warnings should fail the build —
//!      `is_valid()` is the strict (no errors / fatals) gate.
//!
//! When EPUBCheck is not installed we surface a clear error rather
//! than silently skipping validation — a user who doesn't have Java
//! gets a "checked locally" badge in the export UI; export still
//! succeeds via `build_epub` because the packager itself is reliable.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Severity of an EPUBCheck issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IssueSeverity {
    Suppressed,
    Usage,
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
    /// Path inside the EPUB archive (e.g. `OEBPS/text/chapter-001.xhtml`).
    pub path:      Option<String>,
    pub line:      Option<u32>,
    pub column:    Option<u32>,
}

/// The parsed EPUBCheck report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubCheckReport {
    pub checker_version: String,
    pub epub_path:       String,
    pub issues:          Vec<EpubCheckIssue>,
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

    /// Count of WARNING-level issues.
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Warning))
            .count()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EpubCheckError {
    #[error("Java not found at: {path} (set --java-binary or install a JRE on PATH)")]
    JavaNotFound { path: String },

    #[error("EPUBCheck JAR not found at: {path}")]
    JarNotFound { path: String },

    #[error("EPUBCheck process failed with exit code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("failed to parse EPUBCheck report: {reason}")]
    ReportParseError { reason: String },

    #[error("EPUBCheck found {error_count} error(s)")]
    ValidationErrors { error_count: usize },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ── EPUBCheck JSON wire shape (a subset — we only read what we need) ──

#[derive(Debug, Deserialize)]
struct WireReport {
    #[serde(rename = "customMessageFileName")]
    _custom_message_file_name: Option<String>,
    #[serde(rename = "checker")]
    checker: WireChecker,
    #[serde(rename = "messages", default)]
    messages: Vec<WireMessage>,
}
#[derive(Debug, Deserialize)]
struct WireChecker {
    #[serde(rename = "checkerVersion")]
    checker_version: String,
    #[serde(rename = "filename")]
    filename: String,
}
#[derive(Debug, Deserialize)]
struct WireMessage {
    #[serde(rename = "severity")]
    severity: String,
    #[serde(rename = "message", default)]
    message: String,
    #[serde(rename = "locations", default)]
    locations: Vec<WireLocation>,
}
#[derive(Debug, Deserialize)]
struct WireLocation {
    #[serde(rename = "fileName")]
    file_name: Option<String>,
    #[serde(default)] line: Option<u32>,
    #[serde(default)] column: Option<u32>,
}

/// Locate `java` from PATH.  Useful when the caller hasn't pinned a
/// specific JRE (developer workflow).
pub fn java_on_path() -> Option<String> { which("java") }

fn which(binary: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() { return Some(candidate.to_string_lossy().into_owned()); }
        let exe = candidate.with_extension("exe");
        if exe.is_file() { return Some(exe.to_string_lossy().into_owned()); }
    }
    None
}

/// Run EPUBCheck on the given EPUB file path.  `java_binary` is the
/// Java runtime to use; `jar_path` is the EPUBCheck JAR's location.
///
/// Returns the parsed report.  Callers should check `report.is_valid()`
/// before treating an export as shippable.
pub async fn run_epubcheck(
    epub_path:   &str,
    jar_path:    &str,
    java_binary: &str,
) -> Result<EpubCheckReport, EpubCheckError> {
    if !Path::new(java_binary).is_file() {
        return Err(EpubCheckError::JavaNotFound { path: java_binary.to_owned() });
    }
    if !Path::new(jar_path).is_file() {
        return Err(EpubCheckError::JarNotFound { path: jar_path.to_owned() });
    }
    if !Path::new(epub_path).is_file() {
        return Err(EpubCheckError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("EPUB file not found: {epub_path}"),
        )));
    }

    // EPUBCheck supports `--json -` to write the report to stdout.
    tracing::info!(epub = %epub_path, "running EPUBCheck");
    let out = Command::new(java_binary)
        .args([
            "-jar", jar_path,
            "--json", "-",      // JSON to stdout
            "--quiet",          // suppress text output
            epub_path,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    // EPUBCheck exits non-zero when it finds errors, but the JSON report
    // is still written to stdout.  Parse first, then decide.
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    let report = parse_report(&stdout, epub_path).map_err(|reason| {
        EpubCheckError::ReportParseError {
            reason: format!("{reason}\nstderr={stderr}"),
        }
    })?;

    // If the parsed report is empty AND the process failed, treat as a
    // process-level failure (Java crash / unparseable input).
    if report.issues.is_empty() && !out.status.success() {
        return Err(EpubCheckError::ProcessFailed {
            code:   out.status.code().unwrap_or(-1),
            stderr: stderr.lines().take(20).collect::<Vec<_>>().join(" | "),
        });
    }
    Ok(report)
}

/// Parse a JSON report blob into the typed shape.  Public for tests +
/// for callers that have already captured stdout via another mechanism
/// (e.g. file output).
pub fn parse_report(json: &str, epub_path: &str) -> Result<EpubCheckReport, String> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Err("empty EPUBCheck output".to_owned());
    }
    let wire: WireReport = serde_json::from_str(trimmed)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    let mut issues = Vec::with_capacity(wire.messages.len());
    for m in wire.messages {
        let severity = parse_severity(&m.severity)
            .ok_or_else(|| format!("unknown severity: {}", m.severity))?;
        let location = m.locations.into_iter().next();
        issues.push(EpubCheckIssue {
            severity,
            message: m.message,
            path:    location.as_ref().and_then(|l| l.file_name.clone()),
            line:    location.as_ref().and_then(|l| l.line),
            column:  location.as_ref().and_then(|l| l.column),
        });
    }

    Ok(EpubCheckReport {
        checker_version: wire.checker.checker_version,
        epub_path:       if wire.checker.filename.is_empty() {
            epub_path.to_owned()
        } else {
            wire.checker.filename
        },
        issues,
    })
}

fn parse_severity(s: &str) -> Option<IssueSeverity> {
    match s.to_ascii_uppercase().as_str() {
        "SUPPRESSED" => Some(IssueSeverity::Suppressed),
        "USAGE"      => Some(IssueSeverity::Usage),
        "INFO"       => Some(IssueSeverity::Info),
        "WARNING"    => Some(IssueSeverity::Warning),
        "ERROR"      => Some(IssueSeverity::Error),
        "FATAL"      => Some(IssueSeverity::Fatal),
        _            => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_REPORT: &str = r#"{
        "customMessageFileName": null,
        "checker": {
            "path": "/x/book.epub",
            "filename": "/x/book.epub",
            "checkerVersion": "5.1.0",
            "checkDate": "2026-05-07T12:00:00Z",
            "elapsedTime": 12,
            "nFatal": 0, "nError": 1, "nWarning": 2, "nUsage": 0
        },
        "messages": [
            { "severity": "ERROR",   "message": "broken link",
              "locations": [{ "fileName": "OEBPS/text/c1.xhtml", "line": 12 }] },
            { "severity": "WARNING", "message": "missing alt",
              "locations": [{ "fileName": "OEBPS/text/c1.xhtml", "line": 5, "column": 7 }] },
            { "severity": "WARNING", "message": "stale toc entry",
              "locations": [] },
            { "severity": "INFO",    "message": "fyi", "locations": [] }
        ]
    }"#;

    #[test]
    fn parse_report_extracts_messages_and_locations() {
        let r = parse_report(SAMPLE_REPORT, "/x/book.epub").unwrap();
        assert_eq!(r.checker_version, "5.1.0");
        assert_eq!(r.issues.len(), 4);
        assert_eq!(r.issues[0].severity, IssueSeverity::Error);
        assert_eq!(r.issues[0].path.as_deref(), Some("OEBPS/text/c1.xhtml"));
        assert_eq!(r.issues[0].line, Some(12));
        assert_eq!(r.issues[1].severity, IssueSeverity::Warning);
        assert_eq!(r.issues[1].column, Some(7));
        assert!(r.issues[2].path.is_none());
    }

    #[test]
    fn parse_report_rejects_empty() {
        assert!(parse_report("", "/x.epub").is_err());
        assert!(parse_report("   ", "/x.epub").is_err());
    }

    #[test]
    fn parse_report_rejects_invalid_json() {
        assert!(parse_report("{not json", "/x.epub").is_err());
    }

    #[test]
    fn report_validity_gates_on_errors_only() {
        let r = parse_report(SAMPLE_REPORT, "/x.epub").unwrap();
        assert!(!r.is_valid(), "ERROR-severity issue must invalidate");
        assert_eq!(r.error_count(), 1);
        assert_eq!(r.warning_count(), 2);
    }

    #[test]
    fn report_with_only_warnings_is_valid() {
        let report_json = r#"{
            "customMessageFileName": null,
            "checker": {
                "path": "/y.epub", "filename": "/y.epub",
                "checkerVersion": "5.1.0", "checkDate": "x", "elapsedTime": 1,
                "nFatal": 0, "nError": 0, "nWarning": 1, "nUsage": 0
            },
            "messages": [
                { "severity": "WARNING", "message": "x", "locations": [] }
            ]
        }"#;
        let r = parse_report(report_json, "/y.epub").unwrap();
        assert!(r.is_valid());
        assert_eq!(r.error_count(), 0);
        assert_eq!(r.warning_count(), 1);
    }

    #[tokio::test]
    async fn run_rejects_missing_java() {
        let r = run_epubcheck(
            "/tmp/x.epub",
            "/tmp/x.jar",
            "/nonexistent/java",
        ).await;
        assert!(matches!(r, Err(EpubCheckError::JavaNotFound { .. })));
    }

    #[tokio::test]
    async fn run_rejects_missing_jar() {
        // /usr/bin/true exists everywhere we run tests.
        let java = if std::path::Path::new("/usr/bin/true").exists() { "/usr/bin/true" } else { "/bin/true" };
        let r = run_epubcheck(
            "/tmp/x.epub",
            "/nonexistent/epubcheck.jar",
            java,
        ).await;
        assert!(matches!(r, Err(EpubCheckError::JarNotFound { .. })));
    }
}
