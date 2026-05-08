use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input to the `agent_run_outline` Tauri command.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunOutlineInput {
    /// ULID of the open project.
    pub project_id:          String,
    /// JSON-serialised `ProjectBrief` (validated client-side before sending).
    pub brief_json:          String,
    /// Desired chapter count (6–60).
    pub target_chapter_count: u32,
    /// Optional genre overlay string.
    pub genre_overlay:        Option<String>,
    /// Ollama model tag to use (e.g. "qwen2.5:7b-instruct-q4_K_M").
    pub model:                String,
}

/// Result returned by `agent_run_outline`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OutlineRunResult {
    pub run_id:     String,
    pub task_id:    String,
    /// "completed" | "invalid" | "error" | "cancelled"
    pub status:     String,
    /// JSON of `OutlineProposal` on success; null otherwise.
    pub proposal_json: Option<String>,
    pub error:      Option<String>,
    /// Raw model text — always returned so the UI can show it on failure.
    pub raw_output: Option<String>,
}

/// Input to the `agent_apply_outline` Tauri command (MZ-07).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyOutlineInput {
    /// ULID of the open project (matches the bundle's project id).
    pub project_id:    String,
    /// `agent_tasks.id` of the previously-completed outline-architect run.
    /// The orchestrator looks up the persisted proposal via this id.
    pub task_id:       String,
    /// Title to use for the project-root node.  Usually the manifest title.
    pub project_title: String,
}

/// Result of `agent_apply_outline`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyOutlineResult {
    pub task_id:            String,
    pub pre_snapshot_id:    String,
    pub project_root_id:    String,
    pub created_node_count: u32,
    pub applied_edit_count: u32,
}

/// Input to the `agent_apply_copyedit` Tauri command (BACKLOG §E0d.5).
///
/// Accepts one entry from a stored `CopyeditProposals` and applies it to
/// the live scene's `pm_doc`.  The orchestrator takes the mandatory
/// `pre_agent_edit` snapshot, mutates the scene, and inserts an
/// `agent_applied_edits` ledger row with `edit_kind = TextReplace`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyCopyeditInput {
    /// `agent_tasks.id` of the copyeditor run that produced the proposal.
    pub task_id:    String,
    /// ULID of the scene to mutate.  The UI knows which scene the proposal
    /// applies to (it dispatched the run).
    pub scene_id:   String,
    /// Index into `CopyeditProposals.edits` to accept.
    pub edit_index: u32,
}

/// Result of `agent_apply_copyedit`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyCopyeditResult {
    pub task_id:         String,
    pub edit_index:      u32,
    pub scene_id:        String,
    pub pre_snapshot_id: String,
    pub applied_edit_id: String,
    /// `true` if the original char range no longer matched and a unique
    /// `before`-substring search was used instead.  UI surfaces a hint so
    /// the user can re-verify after concurrent edits.
    pub used_fallback_search: bool,
}

/// Input to the `vocab_apply_proposals` Tauri command (BACKLOG §E0d.10).
/// Accepts the indices the user picked from the vocab-dictionary's
/// `VocabUpdateProposals` and writes them to the project layer.  Index
/// arrays default to "all" when omitted at the call site (the UI passes
/// explicit lists once the user has reviewed each row).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VocabApplyInput {
    /// `agent_tasks.id` of the vocab-dictionary run that produced the
    /// proposals.  The orchestrator looks up the persisted proposal
    /// via this id (same pattern as `agent_apply_copyedit`).
    pub task_id:                 String,
    /// Indices into `VocabUpdateProposals.additions` that the user
    /// accepted.  Empty array = none accepted.
    pub accepted_addition_indices:    Vec<u32>,
    /// Indices into `VocabUpdateProposals.modifications` that the user
    /// accepted.  Empty array = none accepted.
    pub accepted_modification_indices: Vec<u32>,
}

/// Result of `vocab_apply_proposals`.  The lists name the rows actually
/// written — the UI shows a confirmation toast with the counts.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VocabApplyResult {
    pub task_id:               String,
    pub additions_applied:     u32,
    pub modifications_applied: u32,
    pub additions_skipped:     u32,
    pub modifications_skipped: u32,
}

/// Input to the `originality_scan_chapter` command — runs the local
/// plagiarism detector (n-gram match against the project's own corpus)
/// over every scene under `chapter_id`.  Local-only; nothing leaves the
/// device.  Online plagiarism API integration is opt-in and gated on a
/// separate consent flow (BACKLOG §E0d.11).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OriginalityScanInput {
    pub chapter_id: String,
    /// Minimum n-gram length (in words) before an overlap counts.
    /// Defaults to 12 (≈ one full clause).
    pub min_words:  Option<u32>,
}

/// One detected verbatim overlap, mirrored from
/// `booksforge_validator::OverlapHit` for IPC.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OverlapHitDto {
    /// "source" | "prior_scene"
    pub kind:        String,
    pub scene_id:    String,
    pub scene_title: String,
    pub output_from: u32,
    pub output_to:   u32,
    pub words:       u32,
    pub quote:       String,
    /// Which other scene (within the project) the span matches.  Empty
    /// when the scan was against an external source.
    pub matched_scene_id:    String,
    pub matched_scene_title: String,
}

/// Result returned by `originality_scan_chapter`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OriginalityScanResult {
    pub chapter_id:      String,
    pub scenes_scanned:  u32,
    pub min_words:       u32,
    pub hits:            Vec<OverlapHitDto>,
}

/// Input to `agent_run_intake_and_outline` (BACKLOG §E1).
///
/// Chained workflow: free-text idea → intake agent → typed
/// `ProjectBrief` → outline-architect agent → `OutlineProposal`.
/// Counts as 2 of the workflow's ≤8 calls per run.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunIntakeAndOutlineInput {
    pub project_id:           String,
    pub idea_text:            String,
    pub preferred_mode:       Option<String>,
    pub target_chapter_count: u32,
    pub genre_overlay:        Option<String>,
    pub model:                String,
}

/// Result of the chained run.  Both halves surface — the UI can show
/// the brief above the outline and let the user re-run if the brief
/// looks off.  All fields are best-effort: `brief = None` when the
/// intake call failed; `outline = None` when the brief was rejected
/// or the outline call failed.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunIntakeAndOutlineResult {
    pub intake_run_id:    String,
    pub intake_task_id:   String,
    /// JSON of `ProjectBrief` on intake success, null otherwise.
    pub brief_json:       Option<String>,
    pub intake_error:     Option<String>,
    pub intake_raw:       Option<String>,
    pub outline_run_id:   Option<String>,
    pub outline_task_id:  Option<String>,
    /// "completed" | "invalid" | "error" | "cancelled" | "skipped"
    pub outline_status:   String,
    /// JSON of `OutlineProposal` on outline success.
    pub outline_json:     Option<String>,
    pub outline_error:    Option<String>,
    pub outline_raw:      Option<String>,
}

/// Input to `agent_run_developmental_review` (BACKLOG §F2).  Chained
/// chapter-level review: 1 LLM call (dev_editor) + per-scene
/// deterministic continuity linter (free, no LLM).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunDevelopmentalReviewInput {
    pub project_id: String,
    pub chapter_id: String,
    pub model:      String,
    /// Project POV string (e.g. "third-limited") used by the
    /// deterministic POV-drift detector.  Empty / null skips POV checks.
    pub project_pov: Option<String>,
    pub high_confidence_mode: Option<bool>,
}

/// One scene's deterministic continuity findings, as surfaced to the UI.
/// Mirrors `booksforge_orchestrator::run::ContinuityScenePass` but
/// re-shapes the inner findings as JSON strings for IPC stability.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ContinuityScenePassDto {
    pub scene_id:    String,
    pub scene_title: String,
    /// JSON-serialised array of `ContinuityFinding`.  The UI parses on
    /// demand — keeps this DTO simple and avoids a per-finding ts-rs
    /// derivation.
    pub findings_json: String,
    pub finding_count: u32,
}

/// Result of `agent_run_developmental_review`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunDevelopmentalReviewResult {
    pub chapter_id:    String,
    pub dev_run_id:    String,
    pub dev_task_id:   String,
    pub dev_status:    String,
    /// JSON-serialised `DevelopmentalNotes` on success, null otherwise.
    pub dev_notes_json: Option<String>,
    pub dev_error:     Option<String>,
    pub dev_raw:       Option<String>,
    pub continuity_passes: Vec<ContinuityScenePassDto>,
    pub scenes_scanned: u32,
}

/// Input to `entity_bible_apply_proposals` (BACKLOG §F4).  Promotes
/// memory-curator's auto-extracted `EntityStub`s into real `Entity`
/// rows in the project's bible.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntityBibleApplyInput {
    /// `agent_tasks.id` of the memory-curator run that produced the
    /// stubs (looked up via `agent_outputs`).
    pub task_id: String,
    /// Indices into `MemoryRefreshProposals.new_entities` to accept.
    pub accepted_indices: Vec<u32>,
}

/// Result of `entity_bible_apply_proposals`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntityBibleApplyResult {
    pub task_id:  String,
    pub inserted: u32,
    pub skipped:  u32,
}

/// Input to the `agent_apply_continuity` Tauri command (BACKLOG §E0d.7).
/// Accepts one finding from a stored `ContinuityReport` and applies its
/// `proposed_fix` (rename across scope, or annotate via memory upsert).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyContinuityInput {
    pub project_id:    String,
    pub task_id:       String,
    pub finding_index: u32,
}

/// Result of `agent_apply_continuity`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyContinuityResultDto {
    pub task_id:           String,
    pub finding_index:     u32,
    /// "rename" | "annotate"
    pub kind:              String,
    pub pre_snapshot_id:   String,
    pub applied_edit_ids:  Vec<String>,
    /// Number of scenes whose text was rewritten (rename only; 0 for annotate).
    pub scenes_touched:    u32,
    pub from_term:         Option<String>,
    pub to_term:           Option<String>,
}

/// Input to the `agent_apply_humanization` Tauri command (BACKLOG §E0d.6).
/// Same shape as `ApplyCopyeditInput` — a separate struct keeps the
/// command surface explicit so the UI doesn't muddle the two flows.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApplyHumanizationInput {
    pub task_id:    String,
    pub scene_id:   String,
    pub edit_index: u32,
}
