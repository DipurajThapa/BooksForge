//! IPC types for the MZ-06 snapshot commands.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input to `snapshot_create`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapshotCreateInput {
    /// One of: "project" | "part" | "chapter" | "scene".
    pub scope: String,
    /// ULID of the scoped node (omit for project scope).
    pub scope_id: Option<String>,
    pub label: Option<String>,
    /// One of: "manual" | "auto" | "pre_ai" | "pre_export" | "pre_migration"
    /// | "pre_agent_edit" | "crash_recovery".  The UI Snapshot button always
    /// sends "manual".
    pub trigger: String,
}

/// Input to `snapshot_list`.  An empty struct lists all snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapshotListInput {
    pub scope_id: Option<String>,
}

/// Input to `snapshot_diff`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapshotDiffInput {
    pub a: String,
    pub b: String,
}

/// Input to `snapshot_restore`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapshotRestoreInput {
    pub snapshot_id: String,
    /// If `Some`, only these node ULIDs are restored.  Omit for whole-tree.
    pub selective: Option<Vec<String>>,
}

/// Frontend-shaped snapshot record returned by `snapshot_create` / `_list`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapshotDto {
    pub id: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub label: Option<String>,
    pub trigger: String,
    pub tree_hash: String,
    /// ISO-8601 UTC.
    pub created_at: String,
    pub size_bytes: u64,
}

/// One node-level diff entry returned by `snapshot_diff`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeDiffDto {
    pub node_id: String,
    /// "added" | "removed" | "changed"
    pub kind: String,
    pub title: String,
}

/// Result of a `snapshot_restore` call.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapshotRestoreResult {
    pub pre_restore_snapshot_id: String,
    pub nodes_restored: u32,
    pub scenes_restored: u32,
}
