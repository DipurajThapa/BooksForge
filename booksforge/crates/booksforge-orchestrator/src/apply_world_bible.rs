//! Per-world-bible acceptance flow (BACKLOG §A13 / Phase 1).
//!
//! `Orchestrator::apply_world_bible` accepts a previously-stored
//! [`WorldBibleProposal`] and writes:
//!   - one `entity` memory entry per location  (`location:<name>`)
//!   - one `book` memory entry per top-level field (`world:social_rules`,
//!     `world:history`, `world:sensory_palette`, `world:conflict_sources`,
//!     `world:symbolic_motifs`, `world:continuity_constraints`)
//!
//! Mirrors the `apply_character_bible` flow: idempotency, mandatory
//! `pre_agent_edit` snapshot at project scope, scope-authorisation check,
//! one `agent_applied_edits` row per memory write.

use std::sync::Arc;

use booksforge_domain::{
    authorise_write, AgentAppliedEdit, AppliedEditKind, MemoryEntry, MemoryScope, SnapshotScope,
    SnapshotTrigger, WorldBibleProposal,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

/// Outcome of `apply_world_bible`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyWorldBibleResult {
    pub task_id: String,
    pub pre_snapshot_id: String,
    pub applied_edit_ids: Vec<String>,
    /// Names of locations written to entity scope.
    pub location_names: Vec<String>,
    /// Memory keys of book-scope rows written (e.g. `world:social_rules`).
    pub book_scope_keys: Vec<String>,
}

impl Orchestrator {
    /// Accept the `WorldBibleProposal` stored against `task_id`,
    /// persisting locations as entity memory and the rest as book memory.
    pub async fn apply_world_bible(
        &self,
        task_id: Ulid,
    ) -> Result<ApplyWorldBibleResult, OrchestratorError> {
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
        let proposal: WorldBibleProposal = serde_json::from_str(&raw).map_err(|e| {
            OrchestratorError::Storage(format!(
                "could not deserialise stored WorldBibleProposal: {e}"
            ))
        })?;

        // 3. Authorise both scopes.
        authorise_write("world-bible", MemoryScope::Entity)
            .map_err(|e| OrchestratorError::Storage(format!("scope authorisation entity: {e}")))?;
        authorise_write("world-bible", MemoryScope::Book)
            .map_err(|e| OrchestratorError::Storage(format!("scope authorisation book: {e}")))?;

        // 4. Pre-edit snapshot (project scope).
        let snap = snapshot
            .create(
                SnapshotScope::Project,
                None,
                Some(format!("Pre world-bible apply for task {task_id}")),
                SnapshotTrigger::PreAgentEdit,
            )
            .await
            .map_err(|e| OrchestratorError::Storage(format!("snapshot create: {e}")))?;
        let pre_snapshot_id = snap.id;

        let mut applied_edit_ids: Vec<String> = Vec::new();
        let mut location_names: Vec<String> = Vec::new();
        let mut book_scope_keys: Vec<String> = Vec::new();
        let now = Utc::now();

        // 5a. Per-location → entity scope.
        for loc in &proposal.main_locations {
            let key = format!("location:{}", loc.name);
            let value_json = serde_json::to_value(loc).map_err(|e| {
                OrchestratorError::Storage(format!("serialise location '{}': {e}", loc.name))
            })?;
            let entry = MemoryEntry {
                id: Ulid::new(),
                scope: MemoryScope::Entity,
                key: key.clone(),
                value_json,
                agent_id: "world-bible".to_owned(),
                created_at: now,
                updated_at: now,
            };
            storage
                .memory_upsert(&entry)
                .await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            applied_edit_ids.push(
                write_ledger_row(
                    &storage,
                    task_id,
                    pre_snapshot_id,
                    now,
                    &key,
                    "entity",
                    Some(&loc.name),
                )
                .await?,
            );
            location_names.push(loc.name.clone());
        }

        // 5b. Top-level world fields → book scope.
        let book_fields: [(&str, serde_json::Value); 6] = [
            (
                "world:social_rules",
                serde_json::json!(proposal.social_rules),
            ),
            ("world:history", serde_json::json!(proposal.history)),
            (
                "world:sensory_palette",
                serde_json::json!(proposal.sensory_palette),
            ),
            (
                "world:conflict_sources",
                serde_json::json!(proposal.conflict_sources),
            ),
            (
                "world:symbolic_motifs",
                serde_json::json!(proposal.symbolic_motifs),
            ),
            (
                "world:continuity_constraints",
                serde_json::json!(proposal.continuity_constraints),
            ),
        ];

        for (key, value_json) in book_fields {
            let entry = MemoryEntry {
                id: Ulid::new(),
                scope: MemoryScope::Book,
                key: key.to_owned(),
                value_json,
                agent_id: "world-bible".to_owned(),
                created_at: now,
                updated_at: now,
            };
            storage
                .memory_upsert(&entry)
                .await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            applied_edit_ids.push(
                write_ledger_row(&storage, task_id, pre_snapshot_id, now, key, "book", None)
                    .await?,
            );
            book_scope_keys.push(key.to_owned());
        }

        tracing::info!(
            agent = "world-bible",
            task_id = %task_id,
            pre_snapshot_id = %pre_snapshot_id,
            locations_written = proposal.main_locations.len(),
            book_fields_written = 6,
            "world bible applied",
        );

        Ok(ApplyWorldBibleResult {
            task_id: task_id.to_string(),
            pre_snapshot_id: pre_snapshot_id.to_string(),
            applied_edit_ids,
            location_names,
            book_scope_keys,
        })
    }
}

async fn write_ledger_row(
    storage: &Arc<SqliteStorage>,
    task_id: Ulid,
    pre_snapshot_id: Ulid,
    applied_at: chrono::DateTime<Utc>,
    memory_key: &str,
    memory_scope: &str,
    location_name: Option<&str>,
) -> Result<String, OrchestratorError> {
    let edit_id = Ulid::new();
    let mut payload = serde_json::json!({
        "agent": "world-bible",
        "memory_scope": memory_scope,
        "memory_key": memory_key,
        "pre_snapshot_id": pre_snapshot_id.to_string(),
    });
    if let Some(name) = location_name {
        payload["location_name"] = serde_json::Value::String(name.to_owned());
    }
    let payload_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
    let edit = AgentAppliedEdit {
        id: edit_id,
        task_id,
        node_id: pre_snapshot_id, // project-scope sentinel
        pre_edit_snapshot_id: pre_snapshot_id,
        applied_at,
        edit_kind: AppliedEditKind::NoteAdd,
        edit_payload_json: payload_str,
        reverted_at: None,
    };
    storage
        .agent_applied_edit_insert(&edit)
        .await
        .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
    Ok(edit_id.to_string())
}
