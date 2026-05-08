//! IPC types for the manuscript-validator subsystem (Phase 4).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One validator finding shaped for the frontend.  Mirrors
/// `booksforge_domain::ValidatorIssue` but stringifies ULIDs and the
/// severity enum.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidatorIssueDto {
    pub validator_id: String,
    pub code:         String,
    /// "info" | "warning" | "error".
    pub severity:     String,
    pub message:      String,
    pub node_id:      Option<String>,
    pub offset_from:  Option<u32>,
    pub offset_to:    Option<u32>,
    pub auto_fixable: bool,
}

/// The whole-batch report returned by `validators_run`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidatorReportDto {
    pub run_id:       String,
    /// "ok" | "warnings" | "errors" | "crashed".
    pub status:       String,
    pub duration_ms:  u64,
    pub error_count:  u32,
    pub warning_count: u32,
    pub info_count:   u32,
    pub issues:       Vec<ValidatorIssueDto>,
}

/// Result of the export-gate evaluation that the UI can show before
/// allowing the export command.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportGateDto {
    /// "pass" | "warn" | "block".
    pub outcome:   String,
    pub errors:    Vec<ValidatorIssueDto>,
    pub warnings:  Vec<ValidatorIssueDto>,
}

/// Input to `validators_apply_fix` — a single auto-fix invocation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyFixInput {
    pub validator_id: String,
    pub node_id:      String,
}

/// Result of `validators_apply_fix`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyFixResult {
    pub validator_id: String,
    pub node_id:      String,
    /// Number of text nodes the fix mutated.  Zero is a successful no-op.
    pub fixes_applied: u32,
}
