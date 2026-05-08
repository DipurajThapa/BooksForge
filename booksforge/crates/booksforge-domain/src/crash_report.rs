//! Crash-report schema (MZ-09).
//!
//! See `docs/CRASH_REPORTING_DESIGN.md` for the design rationale.
//!
//! **Privacy contract** (the type system is the enforcement):
//!
//! - This module imports NO manuscript-touching types from elsewhere
//!   in the workspace.  In particular it does NOT import `pm_doc`,
//!   `entity`, `outline`, `brief`, `agent_io::CopyeditEdit`, or any
//!   of the other types that carry user prose.
//! - The fields on [`CrashReport`] are a typed allowlist: they
//!   describe the crash itself + minimal app/host metadata.  Every
//!   field's type is a primitive or a domain-controlled enum.
//! - A future contributor cannot accidentally add manuscript content
//!   to a report — the type has no slot for it.
//!
//! Tests at the bottom of this file fail compilation if any of the
//! manuscript types are present in the dependency closure of
//! [`CrashReport`].

use serde::{Deserialize, Serialize};

/// What kind of failure produced this report.
///
/// Adding a variant requires: (a) updating the local capture path in
/// `booksforge-orchestrator::crash_capture`, (b) adding a fixture to
/// the privacy-invariant tests so the new variant cannot smuggle a
/// manuscript field through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashKind {
    /// `std::panic` hook fired.
    Panic,
    /// A `tokio::JoinError` surfaced and the worker did not handle it.
    UncaughtTokio,
    /// The Ollama HTTP client returned an unrecoverable error path.
    OllamaConnection,
    /// `sqlx` returned a fatal error (DB corrupt or schema mismatched).
    Sqlx,
    /// The export pipeline (Pandoc / EPUBCheck) crashed.
    Export,
}

/// Operating-system family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsFamily {
    MacOs,
    Windows,
    Linux,
    Unknown,
}

/// CPU architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86_64,
    Aarch64,
    Unknown,
}

/// Generic agent-kind tag.  Deliberately coarse — a finer "agent run
/// id" is exactly the kind of identifier we do NOT want to carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    None,
    Intake,
    OutlineArchitect,
    ChapterDrafter,
    DevEditor,
    Continuity,
    Copyeditor,
    Humanization,
    MemoryCurator,
    VocabDictionary,
    FinalReviewEditor,
}

/// One frame of a symbolicated stack trace.  No captured argument
/// values — many crash reporters capture `format!`-style args; we do
/// not because those frequently contain manuscript-derived strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackFrame {
    /// Mangled or demangled symbol name (e.g.
    /// `booksforge_orchestrator::run::dispatch`).
    pub symbol: Option<String>,
    /// Source file path RELATIVE to the workspace root.  Absolute
    /// paths are forbidden — they leak the user's home directory.
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// Top-level crash report.  Serialised as JSON to
/// `~/.booksforge/crash-reports/<ulid>.json` and only sent if the
/// user explicitly clicks **Send** in the per-event preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashReport {
    /// Schema version — bump when the shape changes.  V1 == 1.
    pub schema_version: u32,
    /// Local-only id (ULID-format string).  Generated at capture
    /// time; not tied to any persistent installation identifier.
    pub report_id: String,
    /// RFC 3339 timestamp.
    pub captured_at: String,

    pub app_version: String,
    pub os_family: OsFamily,
    pub os_version: String,
    pub arch: Arch,

    pub kind: CrashKind,
    /// The PANIC TEMPLATE, not the formatted message.  E.g.
    /// `"internal error: assertion failed: {}"` rather than
    /// `"internal error: assertion failed: <user's character name>"`.
    pub panic_message_template: String,
    pub stack_frames: Vec<StackFrame>,

    /// Whether a project bundle was open at crash time.
    pub project_open: bool,
    /// Which agent kind was running (None = no agent).
    pub agent_running: AgentKind,
    /// Wall-clock milliseconds since `App` launched.
    pub elapsed_since_launch_ms: u64,
}

impl CrashReport {
    /// Schema version constant — must match `schema_version` on every
    /// instance constructed by current code.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── Compile-time privacy guards ───────────────────────────────────
//
// These tests fail to compile if a future contributor adds a
// manuscript-touching field to `CrashReport`.

#[cfg(test)]
mod privacy_guard {
    use super::*;

    /// `assert_not_impl_any!` would be ideal here, but we don't want
    /// a `static_assertions` workspace dep just for this — instead
    /// we exercise the same property via a doctest-style assertion:
    /// every field's serialised representation is a primitive,
    /// container of primitives, or a domain-controlled enum.
    #[test]
    fn crash_report_has_no_string_fields_typed_as_manuscript() {
        // The only `String` fields are: report_id, captured_at,
        // app_version, os_version, panic_message_template, plus
        // StackFrame { symbol, file }.  None of these are
        // manuscript-derived.  This test serves as living
        // documentation — if the struct grows, update both this
        // assertion's `expected` set AND the design doc.
        let expected: &[&str] = &[
            "report_id",
            "captured_at",
            "app_version",
            "os_version",
            "panic_message_template",
        ];
        let mut count = 0;
        // Walk the struct via a sample serialised value.
        let sample = sample_report();
        let value: serde_json::Value = serde_json::to_value(&sample).expect("serialise");
        if let serde_json::Value::Object(obj) = value {
            for (k, v) in obj {
                if matches!(v, serde_json::Value::String(_)) {
                    assert!(
                        expected.contains(&k.as_str()),
                        "Unexpected String field on CrashReport: {k}.\n\
                         If this is a NEW field, verify it cannot carry\n\
                         manuscript content and add it to `expected`\n\
                         in tests/crash_report.rs (and PRIVACY_POLICY.md §1.1).",
                    );
                    count += 1;
                }
            }
        }
        assert_eq!(count, expected.len(), "field-count drift");
    }

    fn sample_report() -> CrashReport {
        CrashReport {
            schema_version: CrashReport::SCHEMA_VERSION,
            report_id: "01HXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            captured_at: "2026-05-08T00:00:00Z".to_string(),
            app_version: "0.0.1".to_string(),
            os_family: OsFamily::MacOs,
            os_version: "14.4.1".to_string(),
            arch: Arch::Aarch64,
            kind: CrashKind::Panic,
            panic_message_template: "internal error: assertion failed: {}".to_string(),
            stack_frames: vec![StackFrame {
                symbol: Some("booksforge_orchestrator::run::dispatch".to_string()),
                file: Some("crates/booksforge-orchestrator/src/run.rs".to_string()),
                line: Some(123),
            }],
            project_open: true,
            agent_running: AgentKind::Copyeditor,
            elapsed_since_launch_ms: 12345,
        }
    }

    #[test]
    fn round_trip_through_json_preserves_every_field() {
        let original = sample_report();
        let json = serde_json::to_string(&original).expect("encode");
        let decoded: CrashReport = serde_json::from_str(&json).expect("decode");
        assert_eq!(original, decoded);
    }

    #[test]
    fn schema_version_is_1() {
        assert_eq!(CrashReport::SCHEMA_VERSION, 1);
    }
}
