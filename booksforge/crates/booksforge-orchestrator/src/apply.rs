//! Outline-acceptance flow (MZ-07).
//!
//! `Orchestrator::apply_outline` consumes a previously-stored outline proposal
//! (persisted by the outline-architect run as an `agent_outputs.content_inline`
//! row keyed by `task_id`) and materialises it as a document tree.
//!
//! Flow per AGENTS.md §4.2 + IMPLEMENTATION_PLAN MZ-07:
//!   1. Refuse if the proposal has already been applied (idempotency).
//!   2. Take the mandatory `pre_agent_edit` snapshot.
//!   3. Translate the proposal to a `NodeTreeDelta` via the pure
//!      `outline_to_tree`.
//!   4. Insert all nodes atomically (`insert_nodes_batch`).
//!   5. Record one `agent_applied_edits` row per node, all linked to the
//!      same pre-edit snapshot.
//!
//! On any failure after step 2, the snapshot is preserved so the user can
//! revert to pre-state via `snapshot.restore`.

use std::sync::Arc;

use booksforge_domain::{
    empty_subtree_ids, outline_to_tree, AppliedEditKind, NodeKind, OutlineApplyError,
    OutlineProposal, SnapshotScope,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

/// Outcome of `apply_outline`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyOutlineResult {
    pub run_id: String,
    pub task_id: String,
    pub pre_snapshot_id: String,
    pub project_root_id: String,
    pub created_node_count: u32,
    pub applied_edit_count: u32,
    /// Number of empty placeholder subtree nodes soft-deleted before
    /// the new outline was mounted. Surfaced so the UI can show
    /// "Cleaned up N placeholder scenes from the template." 0 means
    /// there was nothing to clean.
    #[serde(default)]
    pub pre_cleanup_node_count: u32,
}

impl Orchestrator {
    /// Materialise the outline proposal stored against `task_id` into the
    /// document tree of `project_id`'s open project.
    pub async fn apply_outline(
        &self,
        project_id: Ulid,
        task_id: Ulid,
        project_title: &str,
    ) -> Result<ApplyOutlineResult, OrchestratorError> {
        // Snapshot service is required for any flow that mutates the tree.
        let snapshot: Arc<SnapshotService> = self.snapshot().ok_or_else(|| {
            OrchestratorError::Storage("snapshot service not attached".to_owned())
        })?;

        let storage: Arc<SqliteStorage> = self.storage_arc();

        // 1. Idempotency guard.
        let already = storage
            .count_applied_edits_for_task(task_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        if already > 0 {
            return Err(OrchestratorError::AlreadyApplied { task_id });
        }

        // 2. Load the persisted proposal.
        let output = storage
            .agent_output_load(task_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| {
                OrchestratorError::Storage(format!("no agent_outputs row for task {task_id}"))
            })?;
        let raw = output.content_inline.ok_or_else(|| {
            OrchestratorError::Storage(format!("agent_outputs[{task_id}] has no inline content"))
        })?;
        let proposal: OutlineProposal = serde_json::from_str(&raw).map_err(|e| {
            OrchestratorError::Storage(format!("could not deserialise stored proposal: {e}"))
        })?;

        // 3. Pre-edit snapshot — mandatory before any mutation.
        let pre = snapshot
            .pre_agent_edit_snapshot(
                SnapshotScope::Project,
                Some(project_id),
                Some(format!("pre-outline-apply for task {task_id}")),
            )
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 3a. Find the project's existing root node — `project_create`
        // always inserts one, so a desktop-launched project should have
        // exactly one. We pass that ID into outline_to_tree so the
        // outline mounts UNDER it instead of producing a 2nd root.
        //
        // Bug history: until 2026-05-11, outline_to_tree always emitted
        // a fresh project root, leaving the binder with two trees and
        // the book pipeline doubling its scene count. See
        // book-output/design/UX_REDESIGN_2026-05.md (apply_outline RCA).
        //
        // If we find ZERO project roots — that's CLI-test land where
        // no project_create ran (multi_chapter_run uses the API
        // directly against a temp bundle). Pass None so outline_to_tree
        // creates a fresh root, preserving that path.
        //
        // If we find MORE THAN ONE — the bundle is already in the
        // broken-tree state from a prior run. Refuse rather than
        // adding to the mess; the user must clean up first.
        let existing_nodes = storage
            .list_nodes()
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let existing_roots: Vec<_> = existing_nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Project && n.deleted_at.is_none())
            .collect();
        let existing_root_id: Option<Ulid> = match existing_roots.len() {
            0 => None,
            1 => Some(existing_roots[0].id),
            n => {
                return Err(OrchestratorError::Storage(format!(
                    "project has {n} project-root nodes — likely a leftover from the \
                     pre-2026-05-11 apply_outline duplicate-root bug. Clean up the \
                     extra roots before re-applying an outline."
                )));
            }
        };
        let title_for_root = if existing_root_id.is_some() {
            // Title is unused when reusing an existing root, but the
            // function still requires a string. Pass the project_title
            // unchanged for log-diff symmetry.
            project_title
        } else {
            project_title
        };

        // 3b. Pre-cleanup: soft-delete empty placeholder subtrees under
        // the existing root before mounting the new outline.
        //
        // Why: project_create + template application can seed a Generic
        // Novel scaffold (10 chapters × 3 placeholder scenes = 30
        // empty scenes). When apply_outline runs afterwards, the new
        // outline's 15 chapters land alongside the 30 placeholders —
        // result: 45 scenes total, 30 of them empty. The drafter
        // would then waste hours generating prose for placeholders
        // the writer doesn't want.
        //
        // `empty_subtree_ids` walks the existing tree bottom-up and
        // returns every node whose subtree contains zero scenes with
        // word_count > 0. We pre-edit-snapshot first (step 2 above),
        // so the soft-delete is reversible via Snapshots.
        //
        // Only runs when an existing root is present (CLI / temp-bundle
        // path with no root has nothing to clean up).
        let mut cleanup_count: u32 = 0;
        if existing_root_id.is_some() {
            let scene_contents = storage
                .list_all_scene_content()
                .await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            let has_prose = |id: Ulid| {
                scene_contents
                    .iter()
                    .any(|sc| sc.node_id == id && sc.word_count > 0)
            };
            let to_delete = empty_subtree_ids(&existing_nodes, has_prose);
            for node_id in &to_delete {
                storage
                    .delete_node(*node_id)
                    .await
                    .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            }
            cleanup_count = to_delete.len() as u32;
            if cleanup_count > 0 {
                tracing::info!(
                    project_id = %project_id,
                    soft_deleted = cleanup_count,
                    "apply_outline pre-cleanup: removed empty placeholder subtrees",
                );
            }
        }

        // 4. Build delta (pure).
        let now = Utc::now();
        let mut counter: u128 = Ulid::new().0;
        let mut id_factory = move || {
            let id = Ulid(counter);
            counter = counter.wrapping_add(1);
            id
        };
        let delta = outline_to_tree(
            &proposal,
            title_for_root,
            &mut id_factory,
            now,
            existing_root_id,
        )
        .map_err(|e| OrchestratorError::OutlineApply(e.to_string()))?;

        // 4a. The "project root id" for the result is either the one
        // we mounted UNDER (existing) or the one outline_to_tree
        // freshly emitted (no existing root case).
        let project_root_id = match existing_root_id {
            Some(id) => id,
            None => delta
                .creates
                .iter()
                .find(|n| n.kind == NodeKind::Project)
                .map(|n| n.id)
                .ok_or_else(|| {
                    OrchestratorError::Storage(
                        "outline_to_tree produced no project-root node and none was supplied"
                            .to_owned(),
                    )
                })?,
        };

        // 5. Atomic batch insert.
        storage
            .insert_nodes_batch(&delta.creates)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 6. One applied-edit row per node, linked to the pre-edit snapshot.
        let mut applied_edit_count = 0u32;
        for node in &delta.creates {
            let payload = serde_json::json!({
                "kind": node_kind_str(node.kind),
                "title": node.title,
                "parent_id": node.parent_id.map(|u| u.to_string()),
            })
            .to_string();
            let edit = SnapshotService::build_applied_edit(
                task_id,
                node.id,
                pre.id,
                AppliedEditKind::TreeCreate,
                payload,
            );
            storage
                .agent_applied_edit_insert(&edit)
                .await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            applied_edit_count += 1;
        }

        tracing::info!(
            %task_id, %project_id,
            pre_snapshot = %pre.id,
            nodes = delta.creates.len(),
            "outline applied"
        );

        Ok(ApplyOutlineResult {
            run_id: "".to_owned(), // set by caller if needed
            task_id: task_id.to_string(),
            pre_snapshot_id: pre.id.to_string(),
            project_root_id: project_root_id.to_string(),
            created_node_count: delta.creates.len() as u32,
            applied_edit_count,
            pre_cleanup_node_count: cleanup_count,
        })
    }
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

// ── OutlineApplyError → OrchestratorError mapping ────────────────────────────

impl From<OutlineApplyError> for OrchestratorError {
    fn from(e: OutlineApplyError) -> Self {
        OrchestratorError::OutlineApply(e.to_string())
    }
}
