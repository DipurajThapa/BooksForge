//! Per-stage polish acceptance flow (BACKLOG §A15 / Phase 2).
//!
//! `Orchestrator::apply_polish` accepts a previously-stored
//! [`PolishProposal`] (from any of the 4 specialist polish stages —
//! dialogue / metaphor / voice / scene-tension) and writes its
//! `revised_pm_doc` into the live scene at `scene_id`.
//!
//! Polymorphic over stage: the `agent_id` is recorded in the audit
//! payload so a future "revert this stage's changes" action can replay
//! per-stage.
//!
//! Mirrors the `apply_chapter_drafter` flow: idempotency, mandatory
//! `pre_agent_edit` snapshot, scope-authorisation check (read-only —
//! polish stages don't write to memory), `agent_applied_edits` row.

use std::sync::Arc;

use booksforge_domain::{
    pm_doc_to_text, AgentAppliedEdit, AppliedEditKind, PolishProposal, PolishStageId, SceneContent,
    SnapshotScope, SnapshotTrigger,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

/// Outcome of `apply_polish`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPolishResult {
    pub task_id: String,
    pub scene_id: String,
    pub stage: String,
    pub pre_snapshot_id: String,
    pub applied_edit_id: String,
    pub previous_hash: String,
    pub new_hash: String,
    pub new_word_count: u32,
    pub new_char_count: u32,
}

impl Orchestrator {
    /// Accept a stored `PolishProposal` and write its `revised_pm_doc`
    /// into `scene_id`.
    pub async fn apply_polish(
        &self,
        task_id: Ulid,
        scene_id: Ulid,
    ) -> Result<ApplyPolishResult, OrchestratorError> {
        let snapshot: Arc<SnapshotService> = self.snapshot().ok_or_else(|| {
            OrchestratorError::Storage("snapshot service not attached".to_owned())
        })?;
        let storage: Arc<SqliteStorage> = self.storage_arc();

        // 1. Idempotency.
        let already = storage
            .count_applied_edits_for_task(task_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        if already > 0 {
            return Err(OrchestratorError::AlreadyApplied { task_id });
        }

        // 2. Load proposal.
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
        let proposal: PolishProposal = serde_json::from_str(&raw).map_err(|e| {
            OrchestratorError::Storage(format!("could not deserialise stored PolishProposal: {e}"))
        })?;
        let validation = proposal.validate();
        if !validation.is_empty() {
            return Err(OrchestratorError::OutlineApply(format!(
                "stored polish proposal failed semantic validation: {}",
                validation.join("; ")
            )));
        }
        let stage_id = proposal.stage_id;
        let agent_id = match stage_id {
            PolishStageId::Dialogue => "dialogue-polish",
            PolishStageId::Metaphor => "metaphor-polish",
            PolishStageId::Voice => "voice-polish",
            PolishStageId::SceneTension => "scene-tension-polish",
        };

        // 3. Capture prior hash.
        let current = storage
            .load_scene(scene_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let previous_hash = current.as_ref().map(|c| c.hash.clone()).unwrap_or_default();

        // 4. Pre-edit snapshot at scene scope.
        let snap = snapshot
            .create(
                SnapshotScope::Scene,
                Some(scene_id),
                Some(format!("Pre {agent_id} apply for task {task_id}")),
                SnapshotTrigger::PreAgentEdit,
            )
            .await
            .map_err(|e| OrchestratorError::Storage(format!("snapshot create: {e}")))?;
        let pre_snapshot_id = snap.id;

        // 5. Compute new hash + counts; save scene.
        let pm_str = serde_json::to_string(&proposal.revised_pm_doc)
            .map_err(|e| OrchestratorError::Storage(format!("serialize new pm_doc: {e}")))?;
        let new_hash = blake3::hash(pm_str.as_bytes()).to_hex().to_string();
        let new_text = pm_doc_to_text(&proposal.revised_pm_doc);
        let new_word_count = u32::try_from(new_text.split_whitespace().count()).unwrap_or(u32::MAX);
        let new_char_count = u32::try_from(new_text.chars().count()).unwrap_or(u32::MAX);

        let now = Utc::now();
        let new_content = SceneContent {
            node_id: scene_id,
            pm_doc: proposal.revised_pm_doc.clone(),
            word_count: new_word_count,
            char_count: new_char_count,
            hash: new_hash.clone(),
            updated_at: now,
        };
        storage
            .save_scene(&new_content)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 6. Audit-ledger row — note `agent: <stage>-polish` and `stage` field.
        let edit_id = Ulid::new();
        let payload = serde_json::json!({
            "agent": agent_id,
            "stage": stage_id.as_str(),
            "previous_hash": previous_hash,
            "new_hash": new_hash,
            "new_word_count": new_word_count,
            "new_char_count": new_char_count,
            "edit_notes": proposal.edit_notes,
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
            agent = agent_id,
            stage = stage_id.as_str(),
            task_id = %task_id,
            scene_id = %scene_id,
            pre_snapshot_id = %pre_snapshot_id,
            new_hash = %new_hash,
            new_word_count,
            "polish stage applied to scene",
        );

        Ok(ApplyPolishResult {
            task_id: task_id.to_string(),
            scene_id: scene_id.to_string(),
            stage: stage_id.as_str().to_owned(),
            pre_snapshot_id: pre_snapshot_id.to_string(),
            applied_edit_id: edit_id.to_string(),
            previous_hash,
            new_hash,
            new_word_count,
            new_char_count,
        })
    }
}
