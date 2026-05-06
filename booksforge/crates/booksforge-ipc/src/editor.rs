//! IPC types for the single-scene editor: node CRUD and scene save/load.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Node types ────────────────────────────────────────────────────────────────

/// One row from the `nodes` table, safe to send to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeInfo {
    pub id:           String,
    pub parent_id:    Option<String>,
    /// `"project"` | `"part"` | `"chapter"` | `"scene"` | `"front_matter"` | `"back_matter"`
    pub kind:         String,
    pub title:        String,
    pub position:     String,
    /// `"planned"` | `"drafting"` | `"revised"` | `"final"`
    pub status:       String,
    pub pov:          Option<String>,
    pub beat:         Option<String>,
    pub target_words: Option<u32>,
    pub word_count:   u32,
    pub created_at:   String,
    pub updated_at:   String,
}

/// Input for `node_create`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeCreateInput {
    pub parent_id:    Option<String>,
    pub kind:         String,
    pub title:        String,
    /// LexoRank position string.
    pub position:     String,
    pub status:       String,
    pub target_words: Option<u32>,
}

/// Input for `node_update` (partial — only mutable fields).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeUpdateInput {
    pub id:           String,
    pub title:        Option<String>,
    pub position:     Option<String>,
    pub status:       Option<String>,
    pub pov:          Option<String>,
    pub beat:         Option<String>,
    pub target_words: Option<u32>,
}

// ── Scene content types ───────────────────────────────────────────────────────

/// Input for `scene_save`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneSaveInput {
    pub node_id:    String,
    /// ProseMirror JSON document (matches `SceneContent.pm_doc`).
    pub pm_doc:     serde_json::Value,
    pub word_count: u32,
    pub char_count: u32,
}

/// Returned by `scene_load`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneLoadResult {
    pub node_id:    String,
    pub pm_doc:     serde_json::Value,
    pub word_count: u32,
    pub char_count: u32,
    pub updated_at: String,
}

// ── Recovery types ────────────────────────────────────────────────────────────

/// Returned by `recovery_check`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecoveryStatus {
    /// Whether there are uncommitted pending saves in the recovery log.
    pub has_pending: bool,
    /// The node_id of the most recent uncommitted save (if any).
    pub node_id:     Option<String>,
    /// ISO-8601 timestamp of the pending save.
    pub pending_at:  Option<String>,
}
