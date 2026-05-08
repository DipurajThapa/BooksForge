//! Workflow trigger types, run handle, and the core orchestrator execution
//! engine for MZ-05.
//!
//! `Orchestrator` drives the `outline-from-brief` workflow, which runs the
//! `outline-architect` agent against a pre-formed `ProjectBrief`.

use std::sync::Arc;
use std::time::Instant;

use booksforge_domain::{AgentOutput, AgentRun, AgentTask, AgentTaskStatus, OutlineProposal, ProjectBrief, SnapshotRecord, SnapshotScope};
use booksforge_ollama::{
    types::{CancelToken, ChatMessage, ChatRequest, GenerateOptions},
    TokenSink,
};
use booksforge_prompt::{render, TemplateVars};
use booksforge_snapshot::{SnapshotError, SnapshotService};
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{OrchestratorConfig, OrchestratorError};

// ── Public types ──────────────────────────────────────────────────────────────

/// What triggers a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "trigger", rename_all = "snake_case")]
pub enum WorkflowTrigger {
    OutlineFromBrief {
        project_id:          String,
        brief_json:          String,
        target_chapter_count: u32,
        genre_overlay:        Option<String>,
        model:                String,
    },
}

/// An opaque handle to a running or completed workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHandle {
    pub run_id: String,
}

impl RunHandle {
    pub fn new() -> Self {
        Self { run_id: Ulid::new().to_string() }
    }
}

impl Default for RunHandle {
    fn default() -> Self { Self::new() }
}

/// The outcome of a completed outline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineRunResult {
    pub run_id:     String,
    pub task_id:    String,
    pub status:     String,
    pub proposal:   Option<OutlineProposal>,
    pub error:      Option<String>,
    pub raw_output: Option<String>,
}

/// Combined outcome of `run_developmental_review` (BACKLOG §F2).  One
/// LLM call (dev-editor on the chapter) + deterministic continuity-linter
/// passes over each scene under the chapter.  The deterministic linter
/// is free (no LLM budget consumed) so we can run it on every scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalReviewResult {
    pub chapter_id:        String,
    pub dev_run_id:        String,
    pub dev_task_id:       String,
    /// "completed" | "invalid" | "error" | "cancelled"
    pub dev_status:        String,
    pub dev_notes:         Option<booksforge_domain::DevelopmentalNotes>,
    pub dev_error:         Option<String>,
    pub dev_raw:           Option<String>,
    /// Deterministic continuity-linter findings, grouped per scene.
    /// Empty when the chapter has no scenes or no findings.
    pub continuity_findings: Vec<ContinuityScenePass>,
    /// Total scenes scanned by the deterministic linter.
    pub scenes_scanned: u32,
}

/// One scene's deterministic continuity-linter pass output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityScenePass {
    pub scene_id:    String,
    pub scene_title: String,
    pub findings:    Vec<booksforge_domain::ContinuityFinding>,
}

/// Combined outcome of `run_intake_and_outline` (BACKLOG §E1).
/// Both halves of the chained run surface here so the UI can render
/// the brief (for confirmation) and the outline (for application)
/// from a single Tauri call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeAndOutlineResult {
    pub intake_run_id:   String,
    pub intake_task_id:  String,
    pub brief:           Option<ProjectBrief>,
    pub intake_error:    Option<String>,
    pub intake_raw:      Option<String>,
    pub outline_run_id:  Option<String>,
    pub outline_task_id: Option<String>,
    /// "completed" | "invalid" | "error" | "cancelled" | "skipped"
    pub outline_status:  String,
    pub outline:         Option<OutlineProposal>,
    pub outline_error:   Option<String>,
    pub outline_raw:     Option<String>,
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Workflow orchestrator (Layer 4 infrastructure).
///
/// Holds the Ollama client and storage handle.  Run cap enforcement is
/// stateless — checked against elapsed time and call counts on each step.
pub struct Orchestrator {
    ollama:   Arc<dyn booksforge_ollama::client::OllamaClient>,
    storage:  Arc<SqliteStorage>,
    /// Optional snapshot service.  Required for any flow that *applies* edits
    /// (per MZ-06: every applied edit must be preceded by a `pre_agent_edit`
    /// snapshot).  Outline-architect's proposal-only flow does not need it.
    snapshot: Option<Arc<SnapshotService>>,
    config:   OrchestratorConfig,
}

impl Orchestrator {
    pub fn new(
        ollama:  Arc<dyn booksforge_ollama::client::OllamaClient>,
        storage: Arc<SqliteStorage>,
        config:  OrchestratorConfig,
    ) -> Self {
        Self { ollama, storage, snapshot: None, config }
    }

    /// Builder method: attach a [`SnapshotService`].  Once attached, callers
    /// can use [`Orchestrator::take_pre_agent_edit_snapshot`] before
    /// mutating the project.
    pub fn with_snapshot(mut self, service: Arc<SnapshotService>) -> Self {
        self.snapshot = Some(service);
        self
    }

    /// Internal accessor — used by `apply.rs` (MZ-07) to reach the snapshot
    /// service after the orchestrator has been built.
    pub(crate) fn snapshot(&self) -> Option<Arc<SnapshotService>> {
        self.snapshot.clone()
    }

    /// Internal accessor for the storage handle.
    pub(crate) fn storage_arc(&self) -> Arc<SqliteStorage> {
        self.storage.clone()
    }

    /// Internal accessor for the Ollama client.
    pub(crate) fn ollama_clone(&self) -> Arc<dyn booksforge_ollama::client::OllamaClient> {
        self.ollama.clone()
    }

    /// Run the Copyeditor agent against a single scene.  Uses the generic
    /// runner; surfaces a `CopyeditProposals` plus a full `VerificationReport`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_copyedit_scene(
        &self,
        project_id:  Ulid,
        scene_text:  String,
        scene_title: String,
        style_book:  serde_json::Value,
        context: crate::runner::RunContext,
        model:       String,
        cancel:      CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::CopyeditProposals>, OrchestratorError> {
        let spec = booksforge_agents::copyeditor::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_text".into(),  serde_json::json!(scene_text));
        vars.insert("scene_title".into(), serde_json::json!(scene_title));
        vars.insert("style_book".into(),  style_book);

        let source_text_for_parse = scene_text.clone();
        let parse = move |raw: &str| {
            booksforge_agents::copyeditor::parse_and_validate(raw, &source_text_for_parse)
        };

        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "copyedit-scene",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: Some(&scene_text),
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Run the Continuity agent (LLM adjudicator half).  Caller is
    /// responsible for running the deterministic linter first and passing
    /// only the ambiguous findings here via `vars`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_continuity_adjudication(
        &self,
        project_id:         Ulid,
        ambiguous_findings: serde_json::Value,
        known_entities:     serde_json::Value,
        scene_excerpts:     serde_json::Value,
        project_pov:        Option<String>,
        prior_summary:      Option<String>,
        context: crate::runner::RunContext,
        model:              String,
        cancel:             CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::ContinuityReport>, OrchestratorError> {
        let spec = booksforge_agents::continuity::spec();
        let mut vars = TemplateVars::new();
        vars.insert("ambiguous_findings".into(), ambiguous_findings);
        vars.insert("known_entities".into(),     known_entities);
        vars.insert("scene_excerpts".into(),     scene_excerpts);
        if let Some(p) = project_pov   { vars.insert("project_pov".into(),   serde_json::json!(p)); }
        if let Some(s) = prior_summary { vars.insert("prior_summary".into(), serde_json::json!(s)); }

        let parse = |raw: &str| booksforge_agents::continuity::parse_and_validate(raw);

        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "continuity-check",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: None,
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Run the Intake agent: free-text idea → typed `ProjectBrief`.
    pub async fn run_intake(
        &self,
        project_id: Ulid,
        idea_text:  String,
        preferred_mode: Option<String>,
        model:      String,
        cancel:     CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::ProjectBrief>, OrchestratorError> {
        let spec = booksforge_agents::intake::spec();
        let mut vars = TemplateVars::new();
        vars.insert("idea_text".into(), serde_json::json!(idea_text));
        if let Some(m) = preferred_mode { vars.insert("preferred_mode".into(), serde_json::json!(m)); }
        let parse = |raw: &str| booksforge_agents::intake::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "intake", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &crate::runner::RunContext::empty(), proposed_memory_scopes: &[],
            peer_reviews: Vec::new(), tier_2: None,
            source_text: None, prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Combined intake → outline-architect workflow (BACKLOG §E1).
    ///
    /// Takes a free-text idea + target chapter count, runs intake to
    /// produce a `ProjectBrief`, then feeds that brief to the outline
    /// architect.  Returns both halves so the UI can show the brief
    /// (for confirmation) above the outline.
    ///
    /// Caps: counts as 2 of the workflow's ≤8 calls per run; the
    /// orchestrator's `OrchestratorConfig.max_agent_calls` is consulted
    /// before the second dispatch.  Aborts cleanly if the intake fails
    /// or if the user cancels between calls.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_intake_and_outline(
        &self,
        project_id:           Ulid,
        idea_text:            String,
        preferred_mode:       Option<String>,
        target_chapter_count: u32,
        genre_overlay:        Option<String>,
        model:                String,
        cancel:               CancelToken,
    ) -> Result<IntakeAndOutlineResult, OrchestratorError> {
        // ── Step 1: intake ──
        let intake_result = self.run_intake(
            project_id, idea_text, preferred_mode,
            model.clone(), cancel.clone(),
        ).await?;

        let brief = match (&intake_result.status, &intake_result.output) {
            (booksforge_domain::AgentTaskStatus::Completed, Some(b)) => b.clone(),
            _ => {
                // Surface the intake failure verbatim — outline doesn't
                // run without a typed brief.
                return Ok(IntakeAndOutlineResult {
                    intake_run_id:  intake_result.run_id.to_string(),
                    intake_task_id: intake_result.task_id.to_string(),
                    brief:          None,
                    intake_error:   intake_result.error.clone(),
                    intake_raw:     intake_result.raw_output.clone(),
                    outline_run_id: None,
                    outline_task_id: None,
                    outline_status: "skipped".into(),
                    outline:        None,
                    outline_error:  Some("intake did not produce a valid brief".into()),
                    outline_raw:    None,
                });
            }
        };

        // Cancellation check between calls.
        if cancel.is_cancelled() {
            return Err(OrchestratorError::Cancelled);
        }

        // Budget check: ≤8 agent calls per workflow run.  Intake +
        // outline = 2; we enforce against the configured ceiling so
        // a tightened config can refuse the chain.
        if self.config.max_agent_calls < 2 {
            return Err(OrchestratorError::AgentCallLimitExceeded {
                limit: self.config.max_agent_calls,
            });
        }

        // ── Step 2: outline-architect ──
        let outline_result = self.run_outline(
            project_id, &brief, target_chapter_count,
            genre_overlay.as_deref(), &model, cancel,
        ).await?;

        Ok(IntakeAndOutlineResult {
            intake_run_id:   intake_result.run_id.to_string(),
            intake_task_id:  intake_result.task_id.to_string(),
            brief:           Some(brief),
            intake_error:    None,
            intake_raw:      intake_result.raw_output,
            outline_run_id:  Some(outline_result.run_id),
            outline_task_id: Some(outline_result.task_id),
            outline_status:  outline_result.status,
            outline:         outline_result.proposal,
            outline_error:   outline_result.error,
            outline_raw:     outline_result.raw_output,
        })
    }

    /// Run the Memory Curator agent: chapter text + current memory →
    /// proposed upserts + optional new entity stubs.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_memory_curator(
        &self,
        project_id:     Ulid,
        scope:          String,                   // "book" | "chapter" | "entity"
        node_id:        Option<String>,
        chapter_text:   String,
        current_memory: serde_json::Value,
        existing_entities: serde_json::Value,
        context: crate::runner::RunContext,
        model:          String,
        cancel:         CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::MemoryRefreshProposals>, OrchestratorError> {
        let spec = booksforge_agents::memory_curator::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scope".into(),             serde_json::json!(scope));
        vars.insert("chapter_text".into(),      serde_json::json!(chapter_text));
        vars.insert("current_memory".into(),    current_memory);
        vars.insert("existing_entities".into(), existing_entities);
        if let Some(n) = node_id { vars.insert("node_id".into(), serde_json::json!(n)); }

        // Memory-curator's writes are scope-checked. The MemoryScope cross-cutting
        // validator needs the proposed scopes; extract them from a forward parse.
        let proposed_scopes: Vec<String> = vec![scope.clone()];

        let parse = |raw: &str| booksforge_agents::memory_curator::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "memory-refresh", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &context,
            proposed_memory_scopes: &proposed_scopes,
            peer_reviews: Vec::new(), tier_2: None,
            source_text: None, prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Run the Vocabulary Dictionary agent.
    pub async fn run_vocab_dictionary(
        &self,
        project_id:            Ulid,
        recent_accepted_edits: serde_json::Value,
        recent_rejected_edits: serde_json::Value,
        current_project_vocab: serde_json::Value,
        model:                 String,
        cancel:                CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::VocabUpdateProposals>, OrchestratorError> {
        let spec = booksforge_agents::vocab_dictionary::spec();
        let mut vars = TemplateVars::new();
        vars.insert("recent_accepted_edits".into(), recent_accepted_edits);
        vars.insert("recent_rejected_edits".into(), recent_rejected_edits);
        vars.insert("current_project_vocab".into(), current_project_vocab);
        let parse = |raw: &str| booksforge_agents::vocab_dictionary::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "vocab-refresh", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &crate::runner::RunContext::empty(), proposed_memory_scopes: &[],
            peer_reviews: Vec::new(), tier_2: None,
            source_text: None, prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Run the Chapter Drafter agent: synopsis + context → SceneDraftProposal.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub async fn run_chapter_drafter(
        &self,
        project_id:        Ulid,
        scene_synopsis:    String,
        chapter_purpose:   String,
        project_pov:       String,
        target_words:      u32,
        known_entities:    serde_json::Value,
        prior_summary:     String,
        voice_fingerprint: serde_json::Value,
        genre:             Option<String>,
        tone:              Option<String>,
        context: crate::runner::RunContext,
        model:             String,
        cancel:            CancelToken,
        on_token:          Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::SceneDraftProposal>, OrchestratorError> {
        let spec = booksforge_agents::chapter_drafter::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_synopsis".into(),    serde_json::json!(scene_synopsis));
        vars.insert("chapter_purpose".into(),   serde_json::json!(chapter_purpose));
        vars.insert("project_pov".into(),       serde_json::json!(project_pov));
        vars.insert("target_words".into(),      serde_json::json!(target_words));
        vars.insert("known_entities".into(),    known_entities);
        vars.insert("prior_summary".into(),     serde_json::json!(prior_summary));
        vars.insert("voice_fingerprint".into(), voice_fingerprint);
        if let Some(g) = genre { vars.insert("genre".into(), serde_json::json!(g)); }
        if let Some(t) = tone  { vars.insert("tone".into(),  serde_json::json!(t)); }
        let parse = |raw: &str| booksforge_agents::chapter_drafter::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "draft-scene", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &context, proposed_memory_scopes: &[],
            peer_reviews: Vec::new(), tier_2: None,
            source_text: Some(&scene_synopsis),
            prior_scene_corpus: None,
            on_token,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Run the Developmental Editor agent on one chapter.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_dev_editor(
        &self,
        project_id:    Ulid,
        chapter_id:    String,
        chapter_text:  String,
        project_brief: serde_json::Value,
        prior_chapter_summaries: serde_json::Value,
        known_entities: serde_json::Value,
        context: crate::runner::RunContext,
        model:         String,
        cancel:        CancelToken,
        on_token:      Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::DevelopmentalNotes>, OrchestratorError> {
        let spec = booksforge_agents::dev_editor::spec();
        let mut vars = TemplateVars::new();
        vars.insert("chapter_id".into(),               serde_json::json!(chapter_id));
        vars.insert("chapter_text".into(),             serde_json::json!(chapter_text));
        vars.insert("project_brief".into(),            project_brief);
        vars.insert("prior_chapter_summaries".into(),  prior_chapter_summaries);
        vars.insert("known_entities".into(),           known_entities);
        let parse = |raw: &str| booksforge_agents::dev_editor::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "dev-review", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &context, proposed_memory_scopes: &[],
            peer_reviews: Vec::new(), tier_2: None,
            source_text: None, prior_scene_corpus: None,
            on_token,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Combined developmental review chain (BACKLOG §F2).
    ///
    /// Runs one LLM call (`dev_editor` over the concatenated chapter
    /// text) plus a per-scene deterministic continuity-linter pass
    /// (free — no LLM budget consumed).  The deterministic linter is
    /// the same `booksforge_validator::lint_scene` the standalone
    /// continuity command uses for its first pass; here we surface the
    /// raw findings directly because chapter-level review focuses on
    /// "is anything obviously broken" rather than ambiguous-finding
    /// adjudication.
    ///
    /// One agent call total — much cheaper than naive
    /// "dev_editor + continuity LLM per scene".
    #[allow(clippy::too_many_arguments)]
    pub async fn run_developmental_review(
        &self,
        project_id:              Ulid,
        chapter_id:              String,
        chapter_text:            String,
        per_scene_text:          Vec<(Ulid, String, String)>, // (scene_id, title, text)
        project_brief:           serde_json::Value,
        prior_chapter_summaries: serde_json::Value,
        known_entities:          serde_json::Value,
        project_pov:             Option<String>,
        context:                 crate::runner::RunContext,
        model:                   String,
        cancel:                  CancelToken,
        on_token:                Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<DevelopmentalReviewResult, OrchestratorError> {
        // ── 1. dev_editor (1 LLM call) ──
        let dev = self.run_dev_editor(
            project_id, chapter_id.clone(), chapter_text,
            project_brief, prior_chapter_summaries, known_entities,
            context.clone(), model, cancel, on_token,
        ).await?;

        // ── 2. Per-scene deterministic continuity (free) ──
        // Use the existing `booksforge_validator::lint_scene` helper.
        // No LLM, no budget, no network.  The full linter runs every
        // detector — name drift, POV drift, tense drift, timeline.
        let mut continuity_findings = Vec::with_capacity(per_scene_text.len());
        for (scene_id, scene_title, scene_text) in &per_scene_text {
            let findings = booksforge_validator::lint_scene(
                &scene_id.to_string(),
                scene_text,
                project_pov.as_deref(),
                &context.entity_bible,
            );
            if !findings.is_empty() {
                continuity_findings.push(ContinuityScenePass {
                    scene_id:    scene_id.to_string(),
                    scene_title: scene_title.clone(),
                    findings,
                });
            }
        }

        let dev_status = match dev.status {
            booksforge_domain::AgentTaskStatus::Completed => "completed",
            booksforge_domain::AgentTaskStatus::Cancelled => "cancelled",
            booksforge_domain::AgentTaskStatus::Error     => "error",
            _                                              => "invalid",
        };

        Ok(DevelopmentalReviewResult {
            chapter_id,
            dev_run_id:   dev.run_id.to_string(),
            dev_task_id:  dev.task_id.to_string(),
            dev_status:   dev_status.to_owned(),
            dev_notes:    dev.output,
            dev_error:    dev.error,
            dev_raw:      dev.raw_output,
            continuity_findings,
            scenes_scanned: per_scene_text.len() as u32,
        })
    }

    /// Run the Humanization agent: scene text + voice fingerprint + active
    /// avoid-rules → concrete `before/after` edits with `triggered_rule`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_humanization(
        &self,
        project_id:        Ulid,
        scene_text:        String,
        scene_title:       String,
        active_avoid_rules: serde_json::Value,
        voice_fingerprint: serde_json::Value,
        context: crate::runner::RunContext,
        model:             String,
        cancel:            CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::HumanizationProposals>, OrchestratorError> {
        let spec = booksforge_agents::humanization::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_text".into(),         serde_json::json!(scene_text));
        vars.insert("scene_title".into(),        serde_json::json!(scene_title));
        vars.insert("active_avoid_rules".into(), active_avoid_rules);
        vars.insert("voice_fingerprint".into(),  voice_fingerprint);

        let source_for_parse = scene_text.clone();
        let parse = move |raw: &str| {
            booksforge_agents::humanization::parse_and_validate(raw, &source_for_parse)
        };
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "humanization", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &context, proposed_memory_scopes: &[],
            peer_reviews: Vec::new(), tier_2: None,
            source_text: Some(&scene_text),
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Run the Tier-2 Proposal Validator on another agent's output.
    /// Returns a `ProposalValidation` carrying the four context-fitness
    /// axes (faithfulness, style, coherence, self_consistency).
    #[allow(clippy::too_many_arguments)]
    pub async fn run_proposal_validator_tier2(
        &self,
        project_id:        Ulid,
        primary_agent_id:  String,
        primary_output:    serde_json::Value,
        context_excerpt:   String,
        tier_1_findings:   serde_json::Value,
        voice_fingerprint: serde_json::Value,
        active_avoid_rules: serde_json::Value,
        model:             String,
        cancel:            CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::ProposalValidation>, OrchestratorError> {
        let spec = booksforge_agents::proposal_validator::spec();
        let mut vars = TemplateVars::new();
        vars.insert("primary_agent_id".into(),   serde_json::json!(primary_agent_id));
        vars.insert("primary_output".into(),     primary_output);
        vars.insert("context_excerpt".into(),    serde_json::json!(context_excerpt));
        vars.insert("tier_1_findings".into(),    tier_1_findings);
        vars.insert("voice_fingerprint".into(),  voice_fingerprint);
        vars.insert("active_avoid_rules".into(), active_avoid_rules);
        let parse = |raw: &str| booksforge_agents::proposal_validator::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec, workflow_id: "proposal-validator-tier2", project_id, vars, model: &model, cancel,
            parse: Box::new(parse), context: &crate::runner::RunContext::empty(), proposed_memory_scopes: &[],
            peer_reviews: Vec::new(), tier_2: None,
            source_text: None, prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await
    }

    /// Dispatch all default-on peer reviewers for `primary_agent_id`,
    /// running each as an independent agent invocation against the shared
    /// `peer-review/v1.toml` template.  Returns the collected
    /// `PeerReviewResult`s; failures are dropped (best-effort) rather than
    /// failing the whole run.  `high_confidence_mode` flips on the
    /// non-default-on pairings (see AGENTS.md §6.5).
    ///
    /// Honours the workflow's ≤8-call cap by capping at
    /// `max_agent_calls - 1` reviewers (one slot reserved for the primary).
    #[allow(clippy::too_many_arguments)]
    pub async fn dispatch_peer_reviews(
        &self,
        project_id:           Ulid,
        primary_agent_id:     &str,
        primary_task_id:      String,
        primary_output:       serde_json::Value,
        context_excerpt:      String,
        context:              &crate::runner::RunContext,
        high_confidence_mode: bool,
        model:                String,
        cancel:               CancelToken,
    ) -> Vec<booksforge_domain::PeerReviewResult> {
        let pairings = crate::council::select_pairings(primary_agent_id, high_confidence_mode);
        if pairings.is_empty() { return Vec::new(); }

        let cap = (self.config.max_agent_calls.saturating_sub(1)) as usize;
        let take = pairings.len().min(cap.max(1));

        let known_entities = serde_json::to_value(&context.entity_bible)
            .unwrap_or_else(|_| serde_json::json!([]));
        let voice_fp = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));
        let avoid_rules = serde_json::to_value(&context.active_avoid_rules)
            .unwrap_or_else(|_| serde_json::json!([]));

        let mut out = Vec::with_capacity(take);
        for pairing in pairings.iter().take(take) {
            let focus_str = peer_focus_str(pairing.focus);
            let spec = booksforge_agents::peer_review::spec();
            let mut vars = TemplateVars::new();
            vars.insert("reviewer_agent_id".into(), serde_json::json!(pairing.reviewer_agent_id));
            vars.insert("primary_agent_id".into(),  serde_json::json!(primary_agent_id));
            vars.insert("primary_task_id".into(),   serde_json::json!(primary_task_id));
            vars.insert("primary_output".into(),    primary_output.clone());
            vars.insert("focus".into(),             serde_json::json!(focus_str));
            vars.insert("context_excerpt".into(),   serde_json::json!(context_excerpt.clone()));
            vars.insert("known_entities".into(),    known_entities.clone());
            vars.insert("voice_fingerprint".into(), voice_fp.clone());
            vars.insert("active_avoid_rules".into(), avoid_rules.clone());

            let parse = |raw: &str| booksforge_agents::peer_review::parse_and_validate(raw);
            let input = crate::runner::RunInput {
                spec: &spec,
                workflow_id: "peer-review",
                project_id,
                vars,
                model: &model,
                cancel: cancel.clone(),
                parse: Box::new(parse),
                context,
                proposed_memory_scopes: &[],
                peer_reviews: Vec::new(),
                tier_2: None,
                source_text: None,
                prior_scene_corpus: None,
                on_token: None,
            };
            match crate::runner::run(self.storage.clone(), self.ollama.clone(), &self.config, input).await {
                Ok(result) => {
                    if let Some(r) = result.output {
                        out.push(r);
                    } else {
                        tracing::warn!(
                            primary = primary_agent_id,
                            reviewer = pairing.reviewer_agent_id,
                            focus = ?pairing.focus,
                            "peer reviewer returned no output — dropping"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        primary = primary_agent_id,
                        reviewer = pairing.reviewer_agent_id,
                        error = %e,
                        "peer reviewer dispatch failed — dropping"
                    );
                }
            }
        }
        out
    }

    /// Re-assemble a primary's `VerificationReport` to fold in a fresh set
    /// of peer-review results.  Use after `dispatch_peer_reviews`.
    pub fn fold_peer_reviews_into_result<T>(
        &self,
        primary_agent_id: &str,
        result:           &mut crate::runner::AgentRunResult<T>,
        peer_reviews:     Vec<booksforge_domain::PeerReviewResult>,
    ) {
        if peer_reviews.is_empty() { return; }
        let new_report = crate::council::assemble_report(
            primary_agent_id,
            &result.task_id.to_string(),
            result.verification.tier_1.clone(),
            result.verification.tier_2.clone(),
            peer_reviews,
        );
        result.verification = new_report;
    }

    /// Optionally dispatch the Tier-2 (LLM-backed) ProposalValidator on a
    /// completed primary run.  No-op when:
    ///   - `config.tier2_enabled` is `false`,
    ///   - the primary run produced no typed output (`result.output.is_none()`),
    ///   - or Tier-2 has already been attached.
    ///
    /// On success, replaces `result.verification` with a freshly-assembled
    /// `VerificationReport` that folds in the Tier-2 verdict.  Tier-2
    /// failures are non-fatal: we log and leave the report unchanged so the
    /// primary's existing verdict still controls the user gate.
    pub async fn maybe_dispatch_tier2<T: serde::Serialize + Send + 'static>(
        &self,
        project_id:       Ulid,
        primary_agent_id: &str,
        result:           &mut crate::runner::AgentRunResult<T>,
        context:          &crate::runner::RunContext,
        context_excerpt:  String,
        model:            String,
        cancel:           CancelToken,
    ) -> Result<(), OrchestratorError> {
        if !self.config.tier2_enabled { return Ok(()); }
        if result.output.is_none()    { return Ok(()); }
        if result.verification.tier_2.is_some() { return Ok(()); }

        let primary_output_json = result.output.as_ref()
            .and_then(|o| serde_json::to_value(o).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let tier_1_findings = serde_json::to_value(&result.verification.tier_1)
            .unwrap_or_else(|_| serde_json::json!({}));
        let voice_fp_json = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));
        let avoid_json = serde_json::to_value(&context.active_avoid_rules)
            .unwrap_or_else(|_| serde_json::json!([]));

        let tier2_run = self.run_proposal_validator_tier2(
            project_id,
            primary_agent_id.to_owned(),
            primary_output_json,
            context_excerpt,
            tier_1_findings,
            voice_fp_json,
            avoid_json,
            model,
            cancel,
        ).await;

        match tier2_run {
            Ok(r) if r.output.is_some() => {
                // Safe — guarded by `r.output.is_some()` in the match arm.
                let Some(mut tier_2) = r.output else { unreachable!() };
                tier_2.tier_2_ran = true;
                let new_report = crate::council::assemble_report(
                    primary_agent_id,
                    &result.task_id.to_string(),
                    result.verification.tier_1.clone(),
                    Some(tier_2),
                    result.verification.peer_reviews.clone(),
                );
                result.verification = new_report;
            }
            Ok(_)  => tracing::warn!(primary = primary_agent_id, "tier-2 returned no output — keeping tier-1 verdict"),
            Err(e) => tracing::warn!(primary = primary_agent_id, error = %e, "tier-2 dispatch failed — keeping tier-1 verdict"),
        }
        Ok(())
    }

    /// Take the mandatory pre-agent-edit snapshot for an upcoming mutation.
    ///
    /// Returns the new [`SnapshotRecord`] whose `id` becomes the
    /// `pre_edit_snapshot_id` foreign key on the resulting
    /// `agent_applied_edits` row.  Errors if no snapshot service is attached.
    pub async fn take_pre_agent_edit_snapshot(
        &self,
        scope:    SnapshotScope,
        scope_id: Option<ulid::Ulid>,
        label:    Option<String>,
    ) -> Result<SnapshotRecord, OrchestratorError> {
        let svc = self.snapshot.as_ref().ok_or_else(|| OrchestratorError::Storage(
            "snapshot service not attached — cannot take pre_agent_edit snapshot".to_owned(),
        ))?;
        svc.pre_agent_edit_snapshot(scope, scope_id, label).await
            .map_err(|e: SnapshotError| OrchestratorError::Storage(e.to_string()))
    }

    /// Run the `outline-from-brief` workflow.
    ///
    /// Persists `agent_runs`, `agent_tasks`, `agent_outputs` rows.
    /// Enforces time and retry caps from `OrchestratorConfig`.
    pub async fn run_outline(
        &self,
        project_id:           Ulid,
        brief:                &ProjectBrief,
        target_chapter_count: u32,
        genre_overlay:        Option<&str>,
        model:                &str,
        cancel:               CancelToken,
    ) -> Result<OutlineRunResult, OrchestratorError> {
        let run_id = Ulid::new();
        let now    = Utc::now();

        // Insert the run row (status = running).
        self.storage.agent_run_insert(&AgentRun {
            id:            run_id,
            workflow_id:   "outline-from-brief".to_owned(),
            project_id,
            status:        AgentTaskStatus::Running,
            started_at:    now,
            completed_at:  None,
            total_tokens:  None,
            error_message: None,
            ollama_version: None,
            user_initiated: true,
        }).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        let result = self
            .run_outline_inner(run_id, project_id, brief, target_chapter_count, genre_overlay, model, cancel)
            .await;

        // Finalise the run row.
        let (final_status, err_msg) = match &result {
            Ok(r) if r.proposal.is_some() => (AgentTaskStatus::Completed, None),
            Ok(_)                          => (AgentTaskStatus::Invalid,   None),
            Err(OrchestratorError::Cancelled) => (AgentTaskStatus::Cancelled, Some("cancelled by user".to_owned())),
            Err(e)                         => (AgentTaskStatus::Error, Some(e.to_string())),
        };
        let _ = self.storage.agent_run_update(
            run_id, final_status, None, err_msg.as_deref(),
        ).await;

        result
    }

    async fn run_outline_inner(
        &self,
        run_id:               Ulid,
        project_id:           Ulid,
        brief:                &ProjectBrief,
        target_chapter_count: u32,
        genre_overlay:        Option<&str>,
        model:                &str,
        cancel:               CancelToken,
    ) -> Result<OutlineRunResult, OrchestratorError> {
        let _ = project_id; // used in run row; not needed in inner logic yet
        let spec        = booksforge_agents::outline_architect::spec();
        let template_id = spec.prompt_template.clone();
        let wall_start  = Instant::now();
        let task_id     = Ulid::new();

        // ── Build template vars ───────────────────────────────────────────
        let brief_json = serde_json::to_value(brief)
            .map_err(|e| OrchestratorError::AgentFailed {
                agent_id: "outline-architect".into(),
                retries:  0,
                reason:   format!("failed to serialize brief: {e}"),
            })?;

        let mut vars = TemplateVars::new();
        vars.insert("brief".to_owned(), brief_json);
        vars.insert("target_chapter_count".to_owned(), serde_json::json!(target_chapter_count));
        if let Some(overlay) = genre_overlay {
            vars.insert("genre_overlay".to_owned(), serde_json::json!(overlay));
        }

        // ── Render prompt ─────────────────────────────────────────────────
        let rendered = render(&template_id, &vars)
            .map_err(|e| OrchestratorError::AgentFailed {
                agent_id: "outline-architect".into(),
                retries:  0,
                reason:   format!("prompt render failed: {e}"),
            })?;

        let template_hash_hex = rendered.template_hash.to_hex().to_string();
        let input_blob        = serde_json::to_string(&vars).unwrap_or_default();
        let input_hash        = blake3::hash(input_blob.as_bytes()).to_hex().to_string();

        // ── Insert task row ───────────────────────────────────────────────
        let task_now = Utc::now();
        self.storage.agent_task_insert(&AgentTask {
            id:                   task_id,
            run_id,
            step_index:           0,
            agent_id:             "outline-architect".to_owned(),
            prompt_template_id:   format!("{}.{}", template_id.id, template_id.version),
            prompt_template_hash: template_hash_hex,
            model:                model.to_owned(),
            model_digest:         None,
            input_hash,
            output_hash:          None,
            context_tokens:       None,
            output_tokens:        None,
            duration_ms:          None,
            retries:              0,
            status:               AgentTaskStatus::Running,
            error_category:       None,
            error_message:        None,
            created_at:           task_now,
            updated_at:           task_now,
        }).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // ── Retry loop ────────────────────────────────────────────────────
        // `last_raw` / `last_error` are populated on every iteration before
        // the post-loop block reads them; the initial `None` assignments are
        // dead but required to satisfy definite-initialisation.
        #[allow(unused_assignments)]
        let mut attempt:    u32           = 0;
        #[allow(unused_assignments)]
        let mut last_raw:   Option<String> = None;
        #[allow(unused_assignments)]
        let mut last_error: Option<String> = None;

        loop {
            if cancel.is_cancelled() {
                self.storage.agent_task_update(
                    task_id, AgentTaskStatus::Cancelled, None, None, None,
                    Some(wall_start.elapsed().as_millis() as u64),
                    attempt, None, Some("cancelled by user"),
                ).await.ok();
                return Err(OrchestratorError::Cancelled);
            }
            if wall_start.elapsed().as_secs() >= self.config.max_duration_secs {
                let msg = format!(
                    "time limit {}s exceeded after attempt {attempt}",
                    self.config.max_duration_secs
                );
                self.storage.agent_task_update(
                    task_id, AgentTaskStatus::Error, None, None, None,
                    Some(wall_start.elapsed().as_millis() as u64),
                    attempt, Some("timeout"), Some(&msg),
                ).await.ok();
                return Err(OrchestratorError::TimeLimitExceeded {
                    limit_secs: self.config.max_duration_secs,
                });
            }

            // Build chat request.
            let messages = vec![
                ChatMessage { role: "system".to_owned(), content: rendered.system.clone() },
                ChatMessage { role: "user".to_owned(),   content: rendered.user.clone() },
            ];
            let req = ChatRequest {
                model:   model.to_owned(),
                messages,
                stream:  true,
                options: Some(GenerateOptions {
                    temperature: Some(0.3),
                    top_p:       None,
                    num_ctx:     Some(spec.context_budget.total()),
                    num_predict: Some(spec.context_budget.max_output_tokens),
                }),
            };

            // Stream tokens into a buffer.
            let buf = Arc::new(std::sync::Mutex::new(String::new()));
            let buf2 = buf.clone();
            let sink: TokenSink = Box::new(move |tok: &str| {
                if let Ok(mut b) = buf2.lock() { b.push_str(tok); }
            });

            let step_start  = Instant::now();
            let chat_result = self.ollama.chat(req, sink, cancel.clone()).await;
            let step_ms     = step_start.elapsed().as_millis() as u64;

            let raw_text = match chat_result {
                Ok(ref outcome) if !outcome.message.content.is_empty() => {
                    outcome.message.content.clone()
                }
                _ => buf.lock().map(|b| b.clone()).unwrap_or_default(),
            };
            last_raw = Some(raw_text.clone());

            // Try parse + validate.
            match extract_and_parse::<OutlineProposal>(&raw_text) {
                Ok(proposal) => {
                    let sem_errors = booksforge_agents::outline_architect::validate_semantic(
                        &proposal, target_chapter_count, brief.target_word_count,
                    );
                    if sem_errors.is_empty() {
                        // ── Success ───────────────────────────────────────
                        let out_json  = serde_json::to_string(&proposal).unwrap_or_default();
                        let out_hash  = blake3::hash(out_json.as_bytes()).to_hex().to_string();
                        let validated = Utc::now();

                        self.storage.agent_task_update(
                            task_id, AgentTaskStatus::Completed,
                            Some(&out_hash), None, None, Some(step_ms),
                            attempt, None, None,
                        ).await.ok();

                        self.storage.agent_output_insert(&AgentOutput {
                            task_id,
                            schema_id:      "OutlineProposal".to_owned(),
                            schema_version: 1,
                            content_inline: Some(out_json),
                            content_path:   None,
                            hash:           out_hash,
                            validated_at:   validated,
                        }).await.ok();

                        return Ok(OutlineRunResult {
                            run_id:     run_id.to_string(),
                            task_id:    task_id.to_string(),
                            status:     "completed".to_owned(),
                            proposal:   Some(proposal),
                            error:      None,
                            raw_output: last_raw,
                        });
                    }
                    last_error = Some(format!(
                        "semantic validation failed: {}", sem_errors.join("; ")
                    ));
                }
                Err(e) => {
                    last_error = Some(format!("parse failed: {e}"));
                }
            }

            attempt += 1;
            if attempt > self.config.max_retries_per_step {
                break;
            }
            tracing::warn!(
                %run_id, attempt,
                error = ?last_error,
                "outline-architect failed — retrying"
            );
        }

        // All retries exhausted.
        self.storage.agent_task_update(
            task_id, AgentTaskStatus::Invalid, None, None, None,
            Some(wall_start.elapsed().as_millis() as u64),
            attempt.saturating_sub(1), Some("schema_or_semantic"), last_error.as_deref(),
        ).await.ok();

        Ok(OutlineRunResult {
            run_id:     run_id.to_string(),
            task_id:    task_id.to_string(),
            status:     "invalid".to_owned(),
            proposal:   None,
            error:      last_error,
            raw_output: last_raw,
        })
    }
}

fn peer_focus_str(f: booksforge_domain::PeerReviewFocus) -> &'static str {
    use booksforge_domain::PeerReviewFocus::*;
    match f {
        FactFidelity        => "fact_fidelity",
        VoicePreservation   => "voice_preservation",
        AiTellResidue       => "ai_tell_residue",
        NamePovPreservation => "name_pov_preservation",
        StructuralPurpose   => "structural_purpose",
        MemoryConsistency   => "memory_consistency",
        EmotionalClarity    => "emotional_clarity",
    }
}

// ── JSON extraction helper ────────────────────────────────────────────────────

fn extract_and_parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    serde_json::from_str::<T>(crate::runner::strip_code_fences(text).trim())
        .map_err(|e| format!("JSON parse error: {e}"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_handle_new_is_non_empty() {
        assert!(!RunHandle::new().run_id.is_empty());
    }
}
