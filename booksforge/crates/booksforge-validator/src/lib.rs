//! Manuscript validator engine (Layer 3 — pure logic).
//!
//! Validators are deterministic functions: `fn(&Manuscript) -> Vec<ValidatorIssue>`.
//! They never call LLM agents. The ≥15 MVP validators are implemented in M4.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Severity level for a validator finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    /// Errors block export until resolved.
    Error,
}

/// A single finding from a validator run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorIssue {
    pub validator_id: String,
    pub severity:     Severity,
    pub message:      String,
    /// ULID of the node (Chapter/Scene) the issue is attached to, if applicable.
    pub node_id:      Option<String>,
    /// Character offset within the node's ProseMirror doc, if applicable.
    pub offset:       Option<u32>,
    /// Whether a one-click deterministic fix is available.
    pub auto_fixable: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("validator '{id}' timed out after {limit_ms}ms")]
    Timeout { id: String, limit_ms: u64 },
    #[error("validator '{id}' internal error: {message}")]
    Internal { id: String, message: String },
}
