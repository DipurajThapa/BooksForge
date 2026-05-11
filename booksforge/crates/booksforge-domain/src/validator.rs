//! Manuscript validator types (Layer 3 — pure logic).
//!
//! Validators are deterministic, pure functions: `(&Manuscript, &Context)
//! -> Vec<ValidatorIssue>`.  They never call agents, never hit the network,
//! and never write to storage — all I/O is the orchestrator's job.
//!
//! The implementations themselves live in `booksforge-validator`; this
//! module owns the value-object types so they flow cleanly through IPC.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Severity ordering: `Info < Warning < Error`.  Used to bubble the worst
/// finding up to the export gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    /// Errors block export until resolved.
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    /// True iff this severity should block its consuming quality gate.
    /// Used by the Phase C critic types (`CharacterCriticProposal`,
    /// `StructureCriticProposal`).
    pub fn blocks_gate(self) -> bool {
        matches!(self, Self::Error)
    }
}

impl Default for Severity {
    fn default() -> Self {
        Self::Warning
    }
}

/// Tolerant deserialiser: unknown / null / wrong-typed values fall
/// back to `Severity::Warning` rather than erroring the whole parent
/// parse. Use as `#[serde(default, deserialize_with =
/// "deserialize_severity_tolerant")]` on fields where a misbehaving
/// model's typo'd severity must not poison the rest of the payload.
pub fn deserialize_severity_tolerant<'de, D>(deserializer: D) -> Result<Severity, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    Ok(s.as_deref().and_then(Severity::from_str).unwrap_or_default())
}

/// A single finding emitted by a validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorIssue {
    /// Stable kebab-case id of the validator that produced this issue —
    /// `"heading-hierarchy"`, `"double-spaces"`, …
    pub validator_id: String,
    /// Stable per-issue code so the UI can dedupe / show docs links.
    pub code: String,
    pub severity: Severity,
    pub message: String,
    /// Node the issue is attached to, when applicable.
    pub node_id: Option<Ulid>,
    /// Character offsets within the node's plain text — for the editor to
    /// highlight the offending span.
    pub offset_from: Option<u32>,
    pub offset_to: Option<u32>,
    /// Whether a deterministic one-click fix is available.  Used by the UI
    /// to surface an "Apply all" affordance.
    pub auto_fixable: bool,
}

/// Status of a single validator run.  Mirrors the
/// `validator_runs.status` CHECK constraint in migration 0001.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorRunStatus {
    Ok,
    Warnings,
    Errors,
    Crashed,
}

impl ValidatorRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warnings => "warnings",
            Self::Errors => "errors",
            Self::Crashed => "crashed",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ok" => Some(Self::Ok),
            "warnings" => Some(Self::Warnings),
            "errors" => Some(Self::Errors),
            "crashed" => Some(Self::Crashed),
            _ => None,
        }
    }

    /// Compute the run-level status from a list of issues.
    pub fn from_issues(issues: &[ValidatorIssue]) -> Self {
        if issues.iter().any(|i| i.severity == Severity::Error) {
            return Self::Errors;
        }
        if issues.iter().any(|i| i.severity == Severity::Warning) {
            return Self::Warnings;
        }
        Self::Ok
    }
}

/// Persistable record for `validator_runs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRun {
    pub id: Ulid,
    /// `"batch:all"` for whole-project runs, or a single validator id.
    pub validator_id: String,
    pub ran_at: DateTime<Utc>,
    pub status: ValidatorRunStatus,
    pub duration_ms: u64,
    /// blake3 of the manuscript scope so re-runs over unchanged input
    /// short-circuit (future optimisation).
    pub scope_hash: String,
}

/// The whole-batch report — what `run_all_validators` returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub run: ValidatorRun,
    pub issues: Vec<ValidatorIssue>,
}

impl ValidationReport {
    pub fn count(&self, severity: Severity) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .count()
    }

    pub fn worst_severity(&self) -> Option<Severity> {
        self.issues.iter().map(|i| i.severity).max()
    }
}

/// Outcome of evaluating a [`ValidationReport`] against the export gate
/// policy: errors block, warnings prompt, info is silent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum GateOutcome {
    /// No issues, or only info-level findings.
    Pass,
    /// Warnings present — the UI must surface them and ask the user to
    /// confirm before proceeding.
    Warn { warnings: Vec<ValidatorIssue> },
    /// Error-level issues — export refused.
    Block {
        errors: Vec<ValidatorIssue>,
        warnings: Vec<ValidatorIssue>,
    },
}

/// Apply the export-gate policy to a report.
pub fn pre_export_gate(report: &ValidationReport) -> GateOutcome {
    let errors: Vec<ValidatorIssue> = report
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .cloned()
        .collect();
    let warnings: Vec<ValidatorIssue> = report
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .cloned()
        .collect();

    if !errors.is_empty() {
        GateOutcome::Block { errors, warnings }
    } else if !warnings.is_empty() {
        GateOutcome::Warn { warnings }
    } else {
        GateOutcome::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(severity: Severity) -> ValidatorIssue {
        ValidatorIssue {
            validator_id: "test".into(),
            code: "T001".into(),
            severity,
            message: "x".into(),
            node_id: None,
            offset_from: None,
            offset_to: None,
            auto_fixable: false,
        }
    }

    #[test]
    fn severity_orders_info_lt_warning_lt_error() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn run_status_from_issues_picks_worst() {
        assert_eq!(ValidatorRunStatus::from_issues(&[]), ValidatorRunStatus::Ok);
        assert_eq!(
            ValidatorRunStatus::from_issues(&[issue(Severity::Info)]),
            ValidatorRunStatus::Ok
        );
        assert_eq!(
            ValidatorRunStatus::from_issues(&[issue(Severity::Warning)]),
            ValidatorRunStatus::Warnings
        );
        assert_eq!(
            ValidatorRunStatus::from_issues(&[issue(Severity::Warning), issue(Severity::Error)]),
            ValidatorRunStatus::Errors
        );
    }

    #[test]
    fn gate_pass_on_only_info() {
        let report = ValidationReport {
            run: dummy_run(ValidatorRunStatus::Ok),
            issues: vec![issue(Severity::Info)],
        };
        assert!(matches!(pre_export_gate(&report), GateOutcome::Pass));
    }

    #[test]
    fn gate_warn_on_warnings_only() {
        let report = ValidationReport {
            run: dummy_run(ValidatorRunStatus::Warnings),
            issues: vec![issue(Severity::Warning), issue(Severity::Info)],
        };
        let out = pre_export_gate(&report);
        match out {
            GateOutcome::Warn { warnings } => assert_eq!(warnings.len(), 1),
            other => panic!("expected Warn, got {other:?}"),
        }
    }

    #[test]
    fn gate_block_when_any_error() {
        let report = ValidationReport {
            run: dummy_run(ValidatorRunStatus::Errors),
            issues: vec![issue(Severity::Warning), issue(Severity::Error)],
        };
        match pre_export_gate(&report) {
            GateOutcome::Block { errors, warnings } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(warnings.len(), 1);
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    fn dummy_run(status: ValidatorRunStatus) -> ValidatorRun {
        ValidatorRun {
            id: Ulid::new(),
            validator_id: "batch:all".into(),
            ran_at: Utc::now(),
            status,
            duration_ms: 0,
            scope_hash: String::new(),
        }
    }
}
