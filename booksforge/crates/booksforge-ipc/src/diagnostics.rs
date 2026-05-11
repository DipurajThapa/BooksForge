//! IPC types for the diagnostic bundle command (BACKLOG §B3).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input to `save_diagnostic_bundle`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SaveDiagnosticBundleInput {
    /// Absolute path the bundle should be written to.  User picks via
    /// the OS save-file dialog (extension `.zip`).
    pub output_path: String,
}

/// Result of `save_diagnostic_bundle`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SaveDiagnosticBundleResult {
    pub output_path: String,
    pub bytes: u64,
    pub log_files_included: u32,
    pub redaction_applied: bool,
}
