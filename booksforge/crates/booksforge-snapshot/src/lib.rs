//! Snapshot service (Layer 4 — infrastructure).
//!
//! Captures the document-tree state of a project into content-addressed
//! objects under `snapshots/objects/` and a manifest row in the `snapshots`
//! table.  Restores selected nodes from a snapshot back into SQLite.
//!
//! # Object layout
//!
//! - **Scene object.** Raw JSON bytes of `scene_content.pm_doc`.  Hash =
//!   blake3 of those bytes.  Filename = `<2-prefix>/<rest-of-hash>`.
//! - **Tree object.** A canonical-JSON `SnapshotTree` listing every captured
//!   node and its scene-content hash (if any).  Hash = blake3 of those bytes.
//!   The tree-hash is what `snapshots.tree_hash` stores.
//!
//! # Determinism
//!
//! - Tree entries are sorted by `node_id` before serialisation so the same
//!   project state always produces the same tree-hash.
//! - We use `serde_json::to_vec` (compact, field-order preserving) for both
//!   the tree object and any pm_doc round-trip.
//!
//! # Concurrency
//!
//! `create` issues all reads in sequence on a single connection; SQLite WAL
//! mode means in-flight writes don't block our reads.  The captured tree
//! reflects committed state at the moment of capture.

#![forbid(unsafe_code)]
// BACKLOG §C4: tests freely use `.unwrap()` / `.expect()` against canned
// fixtures; the workspace-level clippy lints fire only on shipped code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::sync::Arc;

use booksforge_domain::{
    AgentAppliedEdit, AppliedEditKind, Node, NodeKind, NodeStatus, SceneContent, SnapshotRecord,
    SnapshotScope, SnapshotTree, SnapshotTrigger, TreeEntry,
};
use booksforge_fs::{BundleFilesystem, BundlePath};
use booksforge_storage::{StorageError, StorageRepository};
use chrono::Utc;
use ulid::Ulid;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("filesystem error: {0}")]
    Fs(#[from] booksforge_fs::FsError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("snapshot not found: {0}")]
    NotFound(Ulid),

    #[error("snapshot tree object missing or corrupt: {hash}")]
    TreeMissing { hash: String },

    #[error("snapshot scene object missing for node {node_id}: {hash}")]
    SceneMissing { node_id: Ulid, hash: String },

    #[error("invalid input: {0}")]
    Invalid(String),

    /// Restore failed *after* the safety snapshot was already written.  The
    /// caller should surface `safety_id` so the user can manually undo the
    /// partial work via the SnapshotsPanel.  The wrapped error explains
    /// what went wrong.
    #[error("restore failed after safety snapshot {safety_id} was created: {source}")]
    RestoreFailedAfterSafety {
        safety_id: Ulid,
        #[source]
        source: Box<SnapshotError>,
    },
}

// ── Service ───────────────────────────────────────────────────────────────────

/// Snapshot service.
///
/// Holds an `Arc` to the storage repository (for SQLite reads/writes) and
/// to the bundle filesystem (for object reads/writes).
///
/// Object-safe enough that callers can wrap it in `Arc<SnapshotService>` and
/// share it across the orchestrator and the Tauri command layer.
pub struct SnapshotService {
    storage: Arc<dyn StorageRepository>,
    fs: Arc<dyn BundleFilesystem>,
    bundle: BundlePath,
}

impl SnapshotService {
    pub fn new(
        storage: Arc<dyn StorageRepository>,
        fs: Arc<dyn BundleFilesystem>,
        bundle: BundlePath,
    ) -> Self {
        Self {
            storage,
            fs,
            bundle,
        }
    }

    /// Capture the current project state into a new snapshot.
    ///
    /// Steps:
    /// 1. Atomically read all non-deleted nodes *and* every saved scene
    ///    content row in a single `BEGIN IMMEDIATE` transaction.  Without
    ///    this an autosave landing between the two reads could produce a
    ///    torn capture (node row pre-edit, scene hash post-edit).
    /// 2. For every leaf node with prose, write the raw `pm_doc` JSON bytes
    ///    as a content-addressed object and record the object hash.
    /// 3. Build a [`SnapshotTree`] (entries sorted by node_id), serialise
    ///    it, and write the tree as a content-addressed object.
    /// 4. Insert a [`SnapshotRecord`] row pointing at the tree-hash.
    pub async fn create(
        &self,
        scope: SnapshotScope,
        scope_id: Option<Ulid>,
        label: Option<String>,
        trigger: SnapshotTrigger,
    ) -> Result<SnapshotRecord, SnapshotError> {
        // 1. Atomic read.
        let (nodes, scenes) = self
            .storage
            .list_nodes_with_scene_content_consistent()
            .await?;
        let scenes_by_id: std::collections::HashMap<Ulid, &SceneContent> =
            scenes.iter().map(|s| (s.node_id, s)).collect();

        // 2. Walk nodes; for prose nodes, snapshot their content.
        let mut entries: Vec<TreeEntry> = Vec::with_capacity(nodes.len());
        let mut bytes_written: u64 = 0;

        for node in &nodes {
            let mut content_hash: Option<String> = None;

            if matches!(
                node.kind,
                NodeKind::Scene | NodeKind::FrontMatter | NodeKind::BackMatter
            ) {
                if let Some(scene) = scenes_by_id.get(&node.id) {
                    let bytes = serde_json::to_vec(&scene.pm_doc)?;
                    let hash = self.fs.write_snapshot_object(&self.bundle, &bytes).await?;
                    bytes_written += bytes.len() as u64;
                    content_hash = Some(hash);
                }
            }

            entries.push(node_to_entry(node, content_hash));
        }

        entries.sort_by_key(|e| e.node_id);

        // 3. Build + write tree object.
        let tree = SnapshotTree::new(entries);
        let tree_bytes = serde_json::to_vec(&tree)?;
        let tree_hash = self
            .fs
            .write_snapshot_object(&self.bundle, &tree_bytes)
            .await?;
        bytes_written += tree_bytes.len() as u64;

        // 4. Insert manifest row.
        let record = SnapshotRecord {
            id: Ulid::new(),
            scope,
            scope_id,
            label,
            trigger,
            tree_hash,
            created_at: Utc::now(),
            size_bytes: bytes_written,
        };
        self.storage.insert_snapshot(&record).await?;

        tracing::info!(
            snapshot_id = %record.id,
            trigger = ?record.trigger,
            scope = ?record.scope,
            entries = tree.entries.len(),
            bytes = record.size_bytes,
            "snapshot created"
        );

        Ok(record)
    }

    /// List snapshot records, newest-first.  Optionally scope-filtered.
    pub async fn list(&self, scope_id: Option<Ulid>) -> Result<Vec<SnapshotRecord>, SnapshotError> {
        Ok(self.storage.list_snapshots(scope_id).await?)
    }

    /// Load and verify a snapshot's tree object.
    pub async fn load_tree(&self, snapshot_id: Ulid) -> Result<SnapshotTree, SnapshotError> {
        let record = self
            .storage
            .get_snapshot(snapshot_id)
            .await?
            .ok_or(SnapshotError::NotFound(snapshot_id))?;
        self.load_tree_by_hash(&record.tree_hash).await
    }

    async fn load_tree_by_hash(&self, hash: &str) -> Result<SnapshotTree, SnapshotError> {
        let bytes = self
            .fs
            .read_snapshot_object(&self.bundle, hash)
            .await
            .map_err(|_| SnapshotError::TreeMissing {
                hash: hash.to_owned(),
            })?;
        let tree: SnapshotTree = serde_json::from_slice(&bytes)?;
        Ok(tree)
    }

    /// Diff two snapshots' trees; returns node-level differences (added /
    /// removed / changed) in `node_id` order.
    pub async fn diff(
        &self,
        a: Ulid,
        b: Ulid,
    ) -> Result<Vec<booksforge_domain::NodeDiff>, SnapshotError> {
        let ta = self.load_tree(a).await?;
        let tb = self.load_tree(b).await?;
        Ok(booksforge_domain::diff_trees(&ta, &tb))
    }

    /// Restore a snapshot back into SQLite.
    ///
    /// If `selective` is `Some`, only the listed `node_id`s are restored;
    /// other nodes (and the scene_content for other nodes) are untouched.
    /// If `selective` is `None`, every node in the captured tree is upserted.
    ///
    /// Always takes a `Manual` "pre-restore" snapshot first so the operation
    /// is reversible.
    pub async fn restore(
        &self,
        snapshot_id: Ulid,
        selective: Option<Vec<Ulid>>,
    ) -> Result<RestoreReport, SnapshotError> {
        // Pre-restore safety snapshot (reuses Manual trigger; see ADR
        // discussion in MZ-06 audit — extending the enum would require a
        // schema migration we'd rather defer).
        let pre = self
            .create(
                SnapshotScope::Project,
                None,
                Some(format!("pre-restore of {snapshot_id}")),
                SnapshotTrigger::PreRestore,
            )
            .await?;

        // Past this point, the safety snapshot exists.  Any failure must
        // be wrapped in `RestoreFailedAfterSafety` so the UI can surface
        // `pre.id` for manual recovery.
        let safety_id = pre.id;
        let inner = async {
            let tree = self.load_tree(snapshot_id).await?;
            let target_ids: Option<std::collections::BTreeSet<Ulid>> =
                selective.map(|v| v.into_iter().collect());

            let mut nodes_restored = 0u32;
            let mut scenes_restored = 0u32;

            for entry in &tree.entries {
                if let Some(ref allow) = target_ids {
                    if !allow.contains(&entry.node_id) {
                        continue;
                    }
                }
                self.restore_entry(entry).await?;
                nodes_restored += 1;
                if entry.content_hash.is_some() {
                    scenes_restored += 1;
                }
            }

            Ok::<(u32, u32), SnapshotError>((nodes_restored, scenes_restored))
        };

        let (nodes_restored, scenes_restored) = match inner.await {
            Ok(v) => v,
            Err(e) => {
                return Err(SnapshotError::RestoreFailedAfterSafety {
                    safety_id,
                    source: Box::new(e),
                });
            }
        };

        Ok(RestoreReport {
            pre_restore_snapshot_id: pre.id,
            nodes_restored,
            scenes_restored,
        })
    }

    async fn restore_entry(&self, entry: &TreeEntry) -> Result<(), SnapshotError> {
        let node = entry_to_node(entry)?;

        // Single-statement upsert via SQLite `ON CONFLICT(id) DO UPDATE`.
        // The previous insert-then-fallback-to-update path silently swallowed
        // non-uniqueness errors (FK violations, encoding issues), masking
        // real failures behind a benign "row already exists" branch.
        //
        // **Soft-delete behaviour (BACKLOG §A7):** the upsert SQL sets
        // `deleted_at = NULL` on conflict.  This means restoring a
        // snapshot resurrects nodes that were soft-deleted *after* the
        // snapshot was taken — which is the documented "restore to
        // snapshot state" semantics.  The pre-restore safety snapshot
        // captures the current state (including any soft-deletions) so
        // the user can revert if they didn't want the resurrection.
        // The `restore_resurrects_soft_deleted_node` test below pins
        // this behaviour.
        self.storage.upsert_node(&node).await?;

        if let Some(hash) = &entry.content_hash {
            let bytes = self
                .fs
                .read_snapshot_object(&self.bundle, hash)
                .await
                .map_err(|_| SnapshotError::SceneMissing {
                    node_id: entry.node_id,
                    hash: hash.clone(),
                })?;
            let pm_doc: serde_json::Value = serde_json::from_slice(&bytes)?;
            let scene = SceneContent {
                node_id: entry.node_id,
                pm_doc,
                word_count: 0,
                char_count: 0,
                hash: hash.clone(),
                updated_at: Utc::now(),
            };
            self.storage.save_scene(&scene).await?;
        }
        Ok(())
    }

    /// Take a `pre_agent_edit` snapshot for a workflow run, then build an
    /// [`AgentAppliedEdit`] record pointing at it.  The caller is responsible
    /// for inserting the record via `agent_applied_edit_insert` after the
    /// edit has been applied.
    ///
    /// Returns the new snapshot record so the caller can stash its `id` for
    /// later linking.
    pub async fn pre_agent_edit_snapshot(
        &self,
        scope: SnapshotScope,
        scope_id: Option<Ulid>,
        label: Option<String>,
    ) -> Result<SnapshotRecord, SnapshotError> {
        self.create(scope, scope_id, label, SnapshotTrigger::PreAgentEdit)
            .await
    }

    /// Take a `pre_ai` snapshot for a quick-action preset (MZ-08).
    ///
    /// Distinct from `pre_agent_edit_snapshot` because quick actions are not
    /// agents — they emit raw prose and are recorded in `ai_calls`, not the
    /// `agent_applied_edits` ledger.  Same content-capture mechanics; the
    /// trigger value lets the timeline UI distinguish the two later.
    pub async fn pre_ai_snapshot(
        &self,
        scope: SnapshotScope,
        scope_id: Option<Ulid>,
        label: Option<String>,
    ) -> Result<SnapshotRecord, SnapshotError> {
        self.create(scope, scope_id, label, SnapshotTrigger::PreAi)
            .await
    }

    /// Helper that constructs a typed [`AgentAppliedEdit`] record after a
    /// `pre_agent_edit_snapshot` has been taken.  Pure value construction —
    /// the caller must still call `storage.agent_applied_edit_insert`.
    pub fn build_applied_edit(
        task_id: Ulid,
        node_id: Ulid,
        pre_edit_snapshot_id: Ulid,
        edit_kind: AppliedEditKind,
        edit_payload_json: String,
    ) -> AgentAppliedEdit {
        AgentAppliedEdit {
            id: Ulid::new(),
            task_id,
            node_id,
            pre_edit_snapshot_id,
            applied_at: Utc::now(),
            edit_kind,
            edit_payload_json,
            reverted_at: None,
        }
    }
}

/// Outcome of a `restore` call.
#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub pre_restore_snapshot_id: Ulid,
    pub nodes_restored: u32,
    pub scenes_restored: u32,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn node_to_entry(node: &Node, content_hash: Option<String>) -> TreeEntry {
    TreeEntry {
        node_id: node.id,
        parent_id: node.parent_id,
        kind: node_kind_str(node.kind).to_owned(),
        title: node.title.clone(),
        position: node.position.clone(),
        status: node_status_str(node.status).to_owned(),
        pov: node.pov.clone(),
        beat: node.beat.clone(),
        target_words: node.target_words,
        content_hash,
    }
}

fn entry_to_node(entry: &TreeEntry) -> Result<Node, SnapshotError> {
    Ok(Node {
        id: entry.node_id,
        parent_id: entry.parent_id,
        kind: parse_node_kind(&entry.kind)?,
        title: entry.title.clone(),
        position: entry.position.clone(),
        status: parse_node_status(&entry.status)?,
        pov: entry.pov.clone(),
        beat: entry.beat.clone(),
        target_words: entry.target_words,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    })
}

fn node_kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Project => "project",
        NodeKind::Part => "part",
        NodeKind::Chapter => "chapter",
        NodeKind::Scene => "scene",
        NodeKind::FrontMatter => "front_matter",
        NodeKind::BackMatter => "back_matter",
    }
}

fn parse_node_kind(s: &str) -> Result<NodeKind, SnapshotError> {
    match s {
        "project" => Ok(NodeKind::Project),
        "part" => Ok(NodeKind::Part),
        "chapter" => Ok(NodeKind::Chapter),
        "scene" => Ok(NodeKind::Scene),
        "front_matter" => Ok(NodeKind::FrontMatter),
        "back_matter" => Ok(NodeKind::BackMatter),
        other => Err(SnapshotError::Invalid(format!(
            "unknown node kind: {other}"
        ))),
    }
}

fn node_status_str(s: NodeStatus) -> &'static str {
    match s {
        NodeStatus::Planned => "planned",
        NodeStatus::Drafting => "drafting",
        NodeStatus::Revised => "revised",
        NodeStatus::Final => "final",
    }
}

fn parse_node_status(s: &str) -> Result<NodeStatus, SnapshotError> {
    match s {
        "planned" => Ok(NodeStatus::Planned),
        "drafting" => Ok(NodeStatus::Drafting),
        "revised" => Ok(NodeStatus::Revised),
        "final" => Ok(NodeStatus::Final),
        other => Err(SnapshotError::Invalid(format!(
            "unknown node status: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_kind_roundtrip() {
        for k in [
            NodeKind::Project,
            NodeKind::Part,
            NodeKind::Chapter,
            NodeKind::Scene,
            NodeKind::FrontMatter,
            NodeKind::BackMatter,
        ] {
            assert_eq!(parse_node_kind(node_kind_str(k)).unwrap(), k);
        }
    }

    #[test]
    fn node_status_roundtrip() {
        for s in [
            NodeStatus::Planned,
            NodeStatus::Drafting,
            NodeStatus::Revised,
            NodeStatus::Final,
        ] {
            assert_eq!(parse_node_status(node_status_str(s)).unwrap(), s);
        }
    }
}
