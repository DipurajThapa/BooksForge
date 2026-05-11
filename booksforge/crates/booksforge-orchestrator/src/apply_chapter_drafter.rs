//! Per-scene chapter-drafter acceptance flow (BACKLOG §A9).
//!
//! `Orchestrator::apply_chapter_drafter` accepts a previously-stored
//! [`SceneDraftProposal`] (persisted by a chapter-drafter run as an
//! `agent_outputs.content_inline` row keyed by `task_id`) and writes its
//! `pm_doc` into the live scene at `scene_id`.
//!
//! Closes the gap that the prior session's UI fix only papered over:
//! before A9, the React `GenericAgentForm` Apply-to-scene button called
//! `snapshotCreate` + `sceneSave` directly from the UI. That bypassed
//! the orchestrator (which is the ONLY mutator per `outputs/CLAUDE.md
//! §9`) and skipped the `agent_applied_edits` ledger row, so the
//! snapshot-invariant CI test couldn't cover the path. This command
//! restores the contract.
//!
//! Flow per AGENTS.md §4.3 + the broader apply-path pattern in
//! `apply.rs` / `apply_copyedit.rs`:
//!   1. Idempotency: refuse if `task_id` already has a row in
//!      `agent_applied_edits`.
//!   2. Take the mandatory `pre_agent_edit` snapshot (scope = Scene).
//!   3. Load the current `SceneContent` so the audit-payload preserves
//!      the prior hash for revertibility.
//!   4. Replace the scene's `pm_doc` with the proposal's, recompute
//!      blake3 + word/char counts.
//!   5. Save the scene and insert one `agent_applied_edits` row with
//!      `edit_kind = TextReplace` and the prior hash in the payload.
//!
//! On any failure after step 2, the snapshot is preserved so the user
//! can revert via `snapshot.restore`.

use std::sync::Arc;

use booksforge_domain::{
    pm_doc_to_text, AgentAppliedEdit, AppliedEditKind, SceneContent, SceneDraftProposal,
    SnapshotScope, SnapshotTrigger,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

/// Outcome of `apply_chapter_drafter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyChapterDrafterResult {
    pub task_id: String,
    pub scene_id: String,
    pub pre_snapshot_id: String,
    pub applied_edit_id: String,
    pub previous_hash: String,
    pub new_hash: String,
    pub new_word_count: u32,
    pub new_char_count: u32,
}

impl Orchestrator {
    /// Accept the `SceneDraftProposal` stored against `task_id`, applying
    /// it to scene `scene_id`.
    pub async fn apply_chapter_drafter(
        &self,
        task_id: Ulid,
        scene_id: Ulid,
    ) -> Result<ApplyChapterDrafterResult, OrchestratorError> {
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
        let proposal: SceneDraftProposal = serde_json::from_str(&raw).map_err(|e| {
            OrchestratorError::Storage(format!(
                "could not deserialise stored SceneDraftProposal: {e}"
            ))
        })?;

        let validation = proposal.validate();
        if !validation.is_empty() {
            return Err(OrchestratorError::OutlineApply(format!(
                "stored proposal failed semantic validation: {}",
                validation.join("; ")
            )));
        }

        // 3. Load current scene so we can preserve the prior hash in the
        //    audit payload (so a future "revert" path can target it
        //    precisely).
        let current = storage
            .load_scene(scene_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let previous_hash = current.as_ref().map(|c| c.hash.clone()).unwrap_or_default();

        // 4. Take the pre_agent_edit snapshot (scope = Scene).
        let snap = snapshot
            .create(
                SnapshotScope::Scene,
                Some(scene_id),
                Some(format!("Pre chapter-drafter apply for task {task_id}")),
                SnapshotTrigger::PreAgentEdit,
            )
            .await
            .map_err(|e| OrchestratorError::Storage(format!("snapshot create: {e}")))?;
        let pre_snapshot_id = snap.id;

        // 5. Compute new pm_doc bytes / hash / counts and save the scene.
        let pm_str = serde_json::to_string(&proposal.pm_doc)
            .map_err(|e| OrchestratorError::Storage(format!("serialize new pm_doc: {e}")))?;
        let new_hash = blake3::hash(pm_str.as_bytes()).to_hex().to_string();
        let new_text = pm_doc_to_text(&proposal.pm_doc);
        let new_word_count = u32::try_from(new_text.split_whitespace().count()).unwrap_or(u32::MAX);
        let new_char_count = u32::try_from(new_text.chars().count()).unwrap_or(u32::MAX);

        let now = Utc::now();
        let new_content = SceneContent {
            node_id: scene_id,
            pm_doc: proposal.pm_doc.clone(),
            word_count: new_word_count,
            char_count: new_char_count,
            hash: new_hash.clone(),
            updated_at: now,
        };
        storage
            .save_scene(&new_content)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 6. Audit-ledger row.
        let edit_id = Ulid::new();
        let payload = serde_json::json!({
            "agent": "chapter-drafter",
            "previous_hash": previous_hash,
            "new_hash": new_hash,
            "new_word_count": new_word_count,
            "new_char_count": new_char_count,
            "drafter_notes": proposal.notes,
        });
        let payload_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
        let edit = AgentAppliedEdit {
            id: edit_id,
            task_id,
            node_id: scene_id,
            pre_edit_snapshot_id: pre_snapshot_id,
            applied_at: now,
            edit_kind: AppliedEditKind::TextReplace,
            edit_payload_json: payload_str,
            reverted_at: None,
        };
        storage
            .agent_applied_edit_insert(&edit)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        tracing::info!(
            agent = "chapter-drafter",
            task_id = %task_id,
            scene_id = %scene_id,
            pre_snapshot_id = %pre_snapshot_id,
            new_hash = %new_hash,
            new_word_count,
            "chapter-drafter applied to scene",
        );

        Ok(ApplyChapterDrafterResult {
            task_id: task_id.to_string(),
            scene_id: scene_id.to_string(),
            pre_snapshot_id: pre_snapshot_id.to_string(),
            applied_edit_id: edit_id.to_string(),
            previous_hash,
            new_hash,
            new_word_count,
            new_char_count,
        })
    }
}
