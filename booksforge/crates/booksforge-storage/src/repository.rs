//! `StorageRepository` trait — the Layer 3 ↔ Layer 4 boundary for SQLite.
//!
//! All methods are async and return typed errors.  The trait is object-safe
//! (via `async_trait`) so it can be stored as `Arc<dyn StorageRepository>`.

use async_trait::async_trait;
use booksforge_domain::{
    AgentAppliedEdit, AgentOutput, AgentRun, AgentTask, AgentTaskStatus, AiCall, AppliedEditKind,
    Entity, ExportRecord, MemoryEntry, MemoryScope, Node, SceneContent, SnapshotRecord, StyleBook,
    ValidatorIssue, ValidatorRun, VocabEntry,
};
use chrono::{DateTime, Utc};
use ulid::Ulid;

use crate::StorageError;

#[async_trait]
pub trait StorageRepository: Send + Sync {
    // ── Nodes ─────────────────────────────────────────────────────────────

    /// Return all non-deleted nodes ordered by position (LexoRank asc).
    async fn list_nodes(&self) -> Result<Vec<Node>, StorageError>;

    /// Insert a new node row.
    async fn insert_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Insert multiple node rows atomically.  All-or-nothing: any failure
    /// rolls back every row in the batch.  Used by `apply_outline` (MZ-07)
    /// to materialise an entire document tree in one transaction.
    async fn insert_nodes_batch(&self, nodes: &[Node]) -> Result<(), StorageError>;

    /// Upsert a single node row by primary key.  If a row with `node.id`
    /// exists, every mutable field is overwritten; otherwise a new row is
    /// inserted.  Used by snapshot restore so it does not silently swallow
    /// non-uniqueness errors (FK violations etc.) the way an
    /// insert-then-fallback-to-update pattern would.
    async fn upsert_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Update mutable fields of an existing node.
    async fn update_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Soft-delete a node (sets `deleted_at = now`).
    async fn delete_node(&self, id: Ulid) -> Result<(), StorageError>;

    // ── Scene content ──────────────────────────────────────────────────────

    /// Load scene content for a node.  Returns `None` if never saved.
    async fn load_scene(&self, node_id: Ulid) -> Result<Option<SceneContent>, StorageError>;

    /// Upsert scene content (INSERT OR REPLACE).
    async fn save_scene(&self, content: &SceneContent) -> Result<(), StorageError>;

    // ── Style book ─────────────────────────────────────────────────────────

    /// Read the singleton style-book row.  Returns `StyleBook::default()` if
    /// the row does not exist yet.
    async fn load_style_book(&self) -> Result<StyleBook, StorageError>;

    /// Upsert the singleton style-book row.
    async fn save_style_book(&self, style: &StyleBook) -> Result<(), StorageError>;

    // ── Entities ───────────────────────────────────────────────────────────

    /// List all non-deleted entities with aliases populated.
    async fn list_entities(&self) -> Result<Vec<Entity>, StorageError>;

    /// Insert a new entity and its aliases atomically.
    async fn insert_entity(&self, entity: &Entity) -> Result<(), StorageError>;

    /// Soft-delete an entity.
    async fn delete_entity(&self, id: Ulid) -> Result<(), StorageError>;

    // ── Snapshots ──────────────────────────────────────────────────────────

    /// Insert a snapshot manifest row.
    async fn insert_snapshot(&self, snap: &SnapshotRecord) -> Result<(), StorageError>;

    /// List snapshot records newest-first, optionally filtered by scope_id.
    async fn list_snapshots(
        &self,
        scope_id: Option<Ulid>,
    ) -> Result<Vec<SnapshotRecord>, StorageError>;

    /// Fetch a single snapshot record by id.
    async fn get_snapshot(&self, id: Ulid) -> Result<Option<SnapshotRecord>, StorageError>;

    /// Insert an `agent_applied_edits` row.
    ///
    /// Enforces the invariant `pre_edit_snapshot.created_at < applied_at`;
    /// returns `StorageError::ConstraintViolation` if violated, missing, or
    /// the snapshot does not exist.
    async fn agent_applied_edit_insert(&self, edit: &AgentAppliedEdit) -> Result<(), StorageError>;

    /// List applied-edit rows for all tasks in a given run, oldest first.
    async fn list_applied_edits_for_run(
        &self,
        run_id: Ulid,
    ) -> Result<Vec<AgentAppliedEdit>, StorageError>;

    /// Count applied-edit rows linked to a single task.  Used for the
    /// idempotency guard in `apply_outline` — re-applying the same proposal
    /// must be a no-op rather than producing duplicate rows.
    async fn count_applied_edits_for_task(&self, task_id: Ulid) -> Result<u32, StorageError>;

    /// List applied-edit rows linked to a single task, oldest first.
    /// Used by per-edit apply paths (Copyedit, Humanization) so the caller
    /// can decode `edit_payload_json` and enforce `(task_id, edit_index)`
    /// idempotency without an extra round-trip per check.
    async fn list_applied_edits_for_task(
        &self,
        task_id: Ulid,
    ) -> Result<Vec<AgentAppliedEdit>, StorageError>;

    /// List the most recent `agent_applied_edits` rows of `kind` for a
    /// project (joined through `agent_tasks` ⨝ `agent_runs`), newest first.
    /// Powers the Vocabulary Dictionary's edit-history input (BACKLOG
    /// §E0d.4) so its proposed avoid-rules are grounded in actual edits.
    async fn recent_applied_edits_for_project(
        &self,
        project_id: Ulid,
        kind: AppliedEditKind,
        limit: u32,
    ) -> Result<Vec<AgentAppliedEdit>, StorageError>;

    // ── Agent audit ledger ─────────────────────────────────────────────────

    /// Insert a new agent run row (status = running).
    async fn agent_run_insert(&self, run: &AgentRun) -> Result<(), StorageError>;

    /// Update the status and completion fields of an existing run.
    async fn agent_run_update(
        &self,
        id: Ulid,
        status: AgentTaskStatus,
        total_tokens: Option<u32>,
        error_message: Option<&str>,
    ) -> Result<(), StorageError>;

    /// Insert a new agent task row (status = running).
    async fn agent_task_insert(&self, task: &AgentTask) -> Result<(), StorageError>;

    /// Update the status, hashes, and metrics of a task row.
    async fn agent_task_update(
        &self,
        id: Ulid,
        status: AgentTaskStatus,
        output_hash: Option<&str>,
        context_tokens: Option<u32>,
        output_tokens: Option<u32>,
        duration_ms: Option<u64>,
        retries: u32,
        error_category: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), StorageError>;

    /// Insert the validated output for a completed task.
    async fn agent_output_insert(&self, output: &AgentOutput) -> Result<(), StorageError>;

    /// Load the output for a given task, if any.
    async fn agent_output_load(&self, task_id: Ulid) -> Result<Option<AgentOutput>, StorageError>;

    /// O3 of `docs/VSM_LLM_OPTIMIZATION.md` — find an existing
    /// `agent_outputs` row produced by an EARLIER task with the same
    /// `(prompt_template_hash, model, input_hash)` triple. Used by the
    /// runner to short-circuit duplicate LLM calls — when the writer
    /// re-runs an agent on identical inputs, return the cached output
    /// instead of paying for another inference.
    ///
    /// Determinism contract: two tasks with the same template hash,
    /// model, and input hash MUST produce semantically equivalent
    /// outputs (the prompt-guard / creative-profile blocks are
    /// already part of `vars` and therefore baked into `input_hash`).
    /// Returns the most recent matching output, or `None`.
    async fn agent_output_lookup_by_input(
        &self,
        prompt_template_hash: &str,
        model: &str,
        input_hash: &str,
    ) -> Result<Option<AgentOutput>, StorageError>;

    // ── Quick-action ledger (MZ-08) ────────────────────────────────────────

    /// Insert a new `ai_calls` row.  The orchestrator writes this on every
    /// quick-action call, including on cancellation and error.
    async fn ai_call_insert(&self, call: &AiCall) -> Result<(), StorageError>;

    /// Mark a quick-action call as applied: stamps `pre_edit_snapshot_id`
    /// and `applied_at`.  Called from `apply_quick_action` after the
    /// snapshot has been taken and the scene_content has been mutated.
    async fn ai_call_update_apply(
        &self,
        id: Ulid,
        pre_edit_snapshot_id: Ulid,
        applied_at: DateTime<Utc>,
    ) -> Result<(), StorageError>;

    /// Fetch a single `ai_calls` row by id.  Used by `apply_quick_action`
    /// to recover the original output text from the audit ledger.
    async fn ai_call_get(&self, id: Ulid) -> Result<Option<AiCall>, StorageError>;

    // ── Scene content bulk helpers (export pipeline) ──────────────────────

    /// Load every saved `scene_content` row.  Used by the export pipeline
    /// so the renderer can include the prose for every scene/front-/back-
    /// matter node in a single pass.  Order is unspecified; callers sort
    /// by traversing `list_nodes` in tree order.
    async fn list_all_scene_content(&self) -> Result<Vec<SceneContent>, StorageError>;

    /// Atomically load every non-deleted node *and* every saved scene
    /// content row in a single read transaction (`BEGIN IMMEDIATE`).
    ///
    /// Used by `SnapshotService::create` to guarantee a consistent capture
    /// — without this, an autosave landing between the `list_nodes` and
    /// per-scene loads could produce a torn snapshot where a node row is
    /// pre-edit and its scene hash is post-edit.
    async fn list_nodes_with_scene_content_consistent(
        &self,
    ) -> Result<(Vec<Node>, Vec<SceneContent>), StorageError>;

    // ── Export ledger ─────────────────────────────────────────────────────

    /// Insert an `exports` row.  Called by the export command after the
    /// output file has been atomically written to disk.
    async fn export_insert(&self, record: &ExportRecord) -> Result<(), StorageError>;

    /// List exports newest-first.  Drives the (future) export-history UI.
    async fn list_exports(&self) -> Result<Vec<ExportRecord>, StorageError>;

    // ── Memory ledger ────────────────────────────────────────────────────

    /// Upsert a memory entry by its `(scope, key)` natural key.  The
    /// orchestrator must call `booksforge_domain::authorise_write` first
    /// — the storage layer trusts the caller and only enforces the
    /// schema's CHECK constraints.
    async fn memory_upsert(&self, entry: &MemoryEntry) -> Result<(), StorageError>;

    /// Fetch a single memory entry by `(scope, key)`.
    async fn memory_get(
        &self,
        scope: MemoryScope,
        key: &str,
    ) -> Result<Option<MemoryEntry>, StorageError>;

    /// List all entries for a given scope, ordered by key.
    async fn memory_list_by_scope(
        &self,
        scope: MemoryScope,
    ) -> Result<Vec<MemoryEntry>, StorageError>;

    /// Delete a memory entry by `(scope, key)`.  Returns the number of
    /// rows affected (0 or 1).
    async fn memory_delete(&self, scope: MemoryScope, key: &str) -> Result<u32, StorageError>;

    // ── Vocabulary ledger ────────────────────────────────────────────────

    /// Upsert one vocab entry by `(layer, term, kind)`.  Used by the
    /// starter-seed loader, the user-curated UI, and the Vocabulary
    /// Dictionary Agent.
    async fn vocab_upsert(&self, entry: &VocabEntry) -> Result<(), StorageError>;

    /// Atomically replace every starter entry.  Wipes existing
    /// `source = 'starter'` rows and reinserts; user / agent rows are
    /// untouched.  Called once at project creation.
    async fn vocab_seed_starters(&self, entries: &[VocabEntry]) -> Result<(), StorageError>;

    /// List every entry whose layer is in `layers`.  Pure-logic
    /// `resolve_vocab` collapses the result into the merged active set.
    async fn vocab_list_by_layers(&self, layers: &[&str]) -> Result<Vec<VocabEntry>, StorageError>;

    /// Count rows for a single layer — useful for assertions in tests.
    async fn vocab_count_by_layer(&self, layer: &str) -> Result<u32, StorageError>;

    // ── Validator ledger (Phase 4) ───────────────────────────────────────

    /// Insert one `validator_runs` row plus every issue it produced in a
    /// single transaction.  Used by the `validators_run` Tauri command.
    async fn validator_run_persist(
        &self,
        run: &ValidatorRun,
        issues: &[ValidatorIssue],
    ) -> Result<(), StorageError>;

    /// Most recent run row, newest first — drives the "Last validated"
    /// indicator in the UI.
    async fn latest_validator_run(&self) -> Result<Option<ValidatorRun>, StorageError>;

    /// Issues for a given run id.
    async fn list_validator_issues_for_run(
        &self,
        run_id: Ulid,
    ) -> Result<Vec<ValidatorIssue>, StorageError>;
}
