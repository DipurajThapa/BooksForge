//! IPC types for project lifecycle commands.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Inputs ───────────────────────────────────────────────────────────────────

/// Input for `project_create`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectInput {
    /// Absolute path where the `*.booksforge/` bundle should be created.
    pub bundle_path: String,
    pub title: String,
    pub author: String,
    /// Optional `genre` string (free-form tag for UI grouping).
    pub genre: Option<String>,
}

/// Input for `project_open`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenProjectInput {
    /// Absolute path to an existing `*.booksforge/` bundle.
    pub bundle_path: String,
}

// ── Outputs ──────────────────────────────────────────────────────────────────

/// Returned by `project_create` and `project_open` on success.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenProjectResult {
    pub project_id: String,
    pub title: String,
    pub author: String,
    pub bundle_path: String,
}

/// One row in the recent-projects list (returned by `project_recent`).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecentProjectEntry {
    pub id: String,
    pub path: String,
    pub name: String,
    /// ISO-8601 timestamp string.
    pub last_opened: String,
    /// `true` when the path no longer exists on disk.
    pub missing: bool,
}
