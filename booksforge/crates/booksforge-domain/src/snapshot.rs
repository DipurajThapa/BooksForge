//! Snapshot domain types.
//!
//! `SnapshotRecord` is the manifest row in the `snapshots` table.  The actual
//! captured content is stored content-addressed in `snapshots/objects/`:
//!
//! - One **scene object** per scene (zstd-compressed `pm_doc` bytes).
//! - One **tree object** per snapshot — a canonical-JSON `SnapshotTree`
//!   listing every captured node and its content hash.
//!
//! All IDs are ULIDs.  All hashes are blake3 hex.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

// ── Snapshot manifest ─────────────────────────────────────────────────────────

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
    /// Safety snapshot taken automatically by `SnapshotService::restore`
    /// before any restore-driven mutation.  Distinct from `Manual` so the
    /// timeline UI can filter user-initiated snapshots.
    PreRestore,
    /// Snapshot created when the crash-recovery flow merges a recovery log.
    CrashRecovery,
}

/// A snapshot manifest row (`snapshots` table).
///
/// The actual content is stored content-addressed in `snapshots/objects/`
/// within the bundle.  This record tracks the metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: Ulid,
    pub scope: SnapshotScope,
    /// `None` when `scope == Project`.
    pub scope_id: Option<Ulid>,
    pub label: Option<String>,
    pub trigger: SnapshotTrigger,
    /// Root content-address (blake3 hex) into `snapshots/objects/`.
    pub tree_hash: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
}

// ── Tree object (the snapshot index) ──────────────────────────────────────────

/// One entry in a snapshot tree object — points at the captured state of a
/// single node.  `content_hash` is `Some` only for `scene` kind (the only
/// node type with a separately-hashed content blob in MVP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    pub node_id: Ulid,
    pub parent_id: Option<Ulid>,
    pub kind: String,
    pub title: String,
    pub position: String,
    pub status: String,
    pub pov: Option<String>,
    pub beat: Option<String>,
    pub target_words: Option<u32>,
    /// blake3 hex of the scene-content object, or `None` for non-scene nodes.
    pub content_hash: Option<String>,
}

/// The root content-addressed object of a snapshot.
///
/// Serialised as canonical JSON (sorted keys, no whitespace) so the tree-hash
/// is reproducible across machines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotTree {
    /// Schema version of the tree object format.  Bump when the layout changes.
    pub schema_version: u32,
    /// Captured node entries, ordered by `position` for determinism.
    pub entries: Vec<TreeEntry>,
}

impl SnapshotTree {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    pub fn new(entries: Vec<TreeEntry>) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            entries,
        }
    }

    /// Find an entry by node id.
    pub fn find(&self, node_id: Ulid) -> Option<&TreeEntry> {
        self.entries.iter().find(|e| e.node_id == node_id)
    }
}

// ── Agent applied edits ───────────────────────────────────────────────────────

/// The kind of edit an agent applied to the manuscript.  Matches the CHECK
/// constraint on `agent_applied_edits.edit_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliedEditKind {
    /// A text replacement inside a scene.
    TextReplace,
    /// Renaming an entity (and propagating it through prose).
    RenameEntity,
    /// Reordering siblings within a parent (positions changed).
    Reorder,
    /// Adding a new note attached to a node.
    NoteAdd,
    /// Creating a node as part of an agent-driven tree creation
    /// (e.g. the Outline Architect's accept-then-apply flow).
    TreeCreate,
}

impl AppliedEditKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextReplace => "text_replace",
            Self::RenameEntity => "rename_entity",
            Self::Reorder => "reorder",
            Self::NoteAdd => "note_add",
            Self::TreeCreate => "tree_create",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text_replace" => Some(Self::TextReplace),
            "rename_entity" => Some(Self::RenameEntity),
            "reorder" => Some(Self::Reorder),
            "note_add" => Some(Self::NoteAdd),
            "tree_create" => Some(Self::TreeCreate),
            _ => None,
        }
    }
}

/// One row in `agent_applied_edits` — the audit ledger entry that proves an
/// agent edit was preceded by a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAppliedEdit {
    pub id: Ulid,
    pub task_id: Ulid,
    pub node_id: Ulid,
    pub pre_edit_snapshot_id: Ulid,
    pub applied_at: DateTime<Utc>,
    pub edit_kind: AppliedEditKind,
    /// Free-form payload describing what changed (interpreted per `edit_kind`).
    pub edit_payload_json: String,
    pub reverted_at: Option<DateTime<Utc>>,
}

// ── Diff types (pure logic — no I/O) ──────────────────────────────────────────

/// What kind of difference exists between two snapshot trees for a single
/// node id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum NodeDiffKind {
    /// Node exists in `b` but not `a`.
    Added,
    /// Node exists in `a` but not `b`.
    Removed,
    /// Node exists in both but `content_hash` or metadata differs.
    Changed,
}

/// One entry in a snapshot-vs-snapshot diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDiff {
    pub node_id: Ulid,
    pub kind: NodeDiffKind,
    pub title: String,
}

/// Diff two snapshot trees and return a deterministic list of node-level
/// differences, ordered by `node_id`.  Pure function — no I/O.
pub fn diff_trees(a: &SnapshotTree, b: &SnapshotTree) -> Vec<NodeDiff> {
    use std::collections::BTreeMap;

    let mut by_id: BTreeMap<Ulid, (Option<&TreeEntry>, Option<&TreeEntry>)> = BTreeMap::new();
    for e in &a.entries {
        by_id.entry(e.node_id).or_insert((None, None)).0 = Some(e);
    }
    for e in &b.entries {
        by_id.entry(e.node_id).or_insert((None, None)).1 = Some(e);
    }

    let mut out = Vec::with_capacity(by_id.len());
    for (node_id, (ea, eb)) in by_id {
        match (ea, eb) {
            (None, Some(e)) => out.push(NodeDiff {
                node_id,
                kind: NodeDiffKind::Added,
                title: e.title.clone(),
            }),
            (Some(e), None) => out.push(NodeDiff {
                node_id,
                kind: NodeDiffKind::Removed,
                title: e.title.clone(),
            }),
            (Some(ea), Some(eb)) if ea != eb => out.push(NodeDiff {
                node_id,
                kind: NodeDiffKind::Changed,
                title: eb.title.clone(),
            }),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: Ulid, title: &str, content: Option<&str>) -> TreeEntry {
        TreeEntry {
            node_id: id,
            parent_id: None,
            kind: "scene".to_owned(),
            title: title.to_owned(),
            position: "0|hzzzzz:".to_owned(),
            status: "drafting".to_owned(),
            pov: None,
            beat: None,
            target_words: None,
            content_hash: content.map(str::to_owned),
        }
    }

    #[test]
    fn diff_detects_added_removed_changed() {
        let id1 = Ulid::new();
        let id2 = Ulid::new();
        let id3 = Ulid::new();

        let a = SnapshotTree::new(vec![
            entry(id1, "Scene 1", Some("aaa")),
            entry(id2, "Scene 2", Some("bbb")),
        ]);
        let b = SnapshotTree::new(vec![
            entry(id1, "Scene 1", Some("aaa")),           // unchanged
            entry(id2, "Scene 2 (revised)", Some("ccc")), // changed
            entry(id3, "Scene 3", Some("ddd")),           // added
        ]);

        let d = diff_trees(&a, &b);
        assert_eq!(d.len(), 2);
        assert!(d
            .iter()
            .any(|n| n.node_id == id2 && n.kind == NodeDiffKind::Changed));
        assert!(d
            .iter()
            .any(|n| n.node_id == id3 && n.kind == NodeDiffKind::Added));
    }

    #[test]
    fn applied_edit_kind_roundtrip() {
        for k in [
            AppliedEditKind::TextReplace,
            AppliedEditKind::RenameEntity,
            AppliedEditKind::Reorder,
            AppliedEditKind::NoteAdd,
        ] {
            assert_eq!(AppliedEditKind::from_str(k.as_str()), Some(k));
        }
    }
}
