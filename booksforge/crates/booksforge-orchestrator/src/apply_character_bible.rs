//! Per-character-bible acceptance flow (BACKLOG §A13 / Phase 1).
//!
//! `Orchestrator::apply_character_bible` accepts a previously-stored
//! [`CharacterBibleProposal`] (persisted by a character-bible run as an
//! `agent_outputs.content_inline` row keyed by `task_id`) and writes one
//! `MemoryEntry` per character into the `entity` scope. The entries are
//! then read by `run_scene_drafter_fic` (Phase 1) and the per-character
//! voice-dictionary work in Phase 3.
//!
//! Flow per AGENTS.md §3 + the broader apply-path pattern in
//! `apply_chapter_drafter.rs`:
//!   1. Idempotency: refuse if `task_id` already has rows in
//!      `agent_applied_edits`.
//!   2. Take the mandatory `pre_agent_edit` snapshot (scope = Project —
//!      bibles are project-wide, not scene-scoped).
//!   3. Authorise: the `character-bible` agent may write to the `entity`
//!      scope per `domain::memory::allowed_write_scopes`.
//!   4. Walk the proposal's characters and `memory_upsert` one entry per
//!      character with key `character:<name>` — atomic per-character.
//!   5. Insert one `agent_applied_edits` ledger row per character (kind =
//!      `NoteAdd`, payload includes the character name + role + the
//!      pre-edit snapshot id for revertibility).
//!
//! On any failure after step 2, the snapshot is preserved so the user
//! can revert via `snapshot.restore`.

use std::sync::Arc;

use booksforge_domain::{
    authorise_write, AgentAppliedEdit, AppliedEditKind, CharacterBibleProposal, MemoryEntry,
    MemoryScope, SnapshotScope, SnapshotTrigger,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

/// Outcome of `apply_character_bible`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCharacterBibleResult {
    pub task_id: String,
    pub pre_snapshot_id: String,
    /// One `agent_applied_edits` row id per character written.
    pub applied_edit_ids: Vec<String>,
    /// Names of the characters written (for UI surfacing).
    pub character_names: Vec<String>,
    /// Memory keys of the entries written (e.g. `character:Ada`). UI can
    /// link these to memory-list rows.
    pub memory_keys: Vec<String>,
}

impl Orchestrator {
    /// Accept the `CharacterBibleProposal` stored against `task_id`,
    /// persisting one memory entry per character.
    pub async fn apply_character_bible(
        &self,
        task_id: Ulid,
    ) -> Result<ApplyCharacterBibleResult, OrchestratorError> {
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
        let proposal: CharacterBibleProposal = serde_json::from_str(&raw).map_err(|e| {
            OrchestratorError::Storage(format!(
                "could not deserialise stored CharacterBibleProposal: {e}"
            ))
        })?;

        // 3. Authorise — the agent's allowed-write-scope is enforced even
        //    though we already know the agent id, so a future scope-policy
        //    change is caught at the boundary.
        authorise_write("character-bible", MemoryScope::Entity)
            .map_err(|e| OrchestratorError::Storage(format!("scope authorisation: {e}")))?;

        // 4. Pre-edit snapshot (project scope — bibles are project-wide).
        let snap = snapshot
            .create(
                SnapshotScope::Project,
                None,
                Some(format!("Pre character-bible apply for task {task_id}")),
                SnapshotTrigger::PreAgentEdit,
            )
            .await
            .map_err(|e| OrchestratorError::Storage(format!("snapshot create: {e}")))?;
        let pre_snapshot_id = snap.id;

        // 5. Per-character write + ledger row.
        let mut applied_edit_ids: Vec<String> = Vec::with_capacity(proposal.characters.len());
        let mut character_names: Vec<String> = Vec::with_capacity(proposal.characters.len());
        let mut memory_keys: Vec<String> = Vec::with_capacity(proposal.characters.len());
        let now = Utc::now();

        for card in &proposal.characters {
            let key = format!("character:{}", card.name);
            let value_json = serde_json::to_value(card).map_err(|e| {
                OrchestratorError::Storage(format!("serialise card '{}': {e}", card.name))
            })?;

            let entry = MemoryEntry {
                id: Ulid::new(),
                scope: MemoryScope::Entity,
                key: key.clone(),
                value_json,
                agent_id: "character-bible".to_owned(),
                created_at: now,
                updated_at: now,
            };
            storage
                .memory_upsert(&entry)
                .await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

            let edit_id = Ulid::new();
            // We don't have a node_id — use the snapshot id as a stand-in
            // sentinel (project-scope edits don't bind to a node). The
            // payload tells the UI which memory entry was created.
            let payload = serde_json::json!({
                "agent": "character-bible",
                "memory_scope": "entity",
                "memory_key": key,
                "character_name": card.name,
                "character_role": card.role,
                "pre_snapshot_id": pre_snapshot_id.to_string(),
            });
            let payload_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
            let edit = AgentAppliedEdit {
                id: edit_id,
                task_id,
                node_id: pre_snapshot_id, // project-scope sentinel
                pre_edit_snapshot_id: pre_snapshot_id,
                applied_at: now,
                edit_kind: AppliedEditKind::NoteAdd,
                edit_payload_json: payload_str,
                reverted_at: None,
            };
            storage
                .agent_applied_edit_insert(&edit)
                .await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

            applied_edit_ids.push(edit_id.to_string());
            character_names.push(card.name.clone());
            memory_keys.push(key);
        }

        tracing::info!(
            agent = "character-bible",
            task_id = %task_id,
            pre_snapshot_id = %pre_snapshot_id,
            characters_written = proposal.characters.len(),
            "character bible applied — one entity-memory row per character",
        );

        Ok(ApplyCharacterBibleResult {
            task_id: task_id.to_string(),
            pre_snapshot_id: pre_snapshot_id.to_string(),
            applied_edit_ids,
            character_names,
            memory_keys,
        })
    }
}
