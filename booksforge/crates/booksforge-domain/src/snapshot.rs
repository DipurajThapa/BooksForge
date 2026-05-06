use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// The scope of a snapshot — what portion of the project was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotScope {
    Project,
    Part,
    Chapter,
    Scene,
}

/// What triggered this snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotTrigger {
    Manual,
    Auto,
    PreAi,
    PreExport,
    PreMigration,
    /// Mandatory snapshot taken before any agent-applied edit.
    PreAgentEdit,
    /// Snapshot created when the crash-recovery flow merges a recovery log.
    CrashRecovery,
}

/// A snapshot manifest row (`snapshots` table).
///
/// The actual content is stored content-addressed in `snapshots/objects/`
/// within the bundle.  This record tracks the metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id:         Ulid,
    pub scope:      SnapshotScope,
    /// `None` when `scope == Project`.
    pub scope_id:   Option<Ulid>,
    pub label:      Option<String>,
    pub trigger:    SnapshotTrigger,
    /// Root content-address (blake3 hex) into `snapshots/objects/`.
    pub tree_hash:  String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
}
