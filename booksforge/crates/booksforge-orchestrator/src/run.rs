//! Workflow trigger types, run handle, and the core orchestrator execution
//! engine for MZ-05.
//!
//! `Orchestrator` drives the `outline-from-brief` workflow, which runs the
//! `outline-architect` agent against a pre-formed `ProjectBrief`.

use std::sync::Arc;
use std::time::Instant;

use booksforge_domain::{
    AgentOutput, AgentRun, AgentTask, AgentTaskStatus, OutlineProposal, ProjectBrief,
    SnapshotRecord, SnapshotScope,
};
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
        project_id: String,
        brief_json: String,
        target_chapter_count: u32,
        genre_overlay: Option<String>,
        model: String,
    },
}

/// An opaque handle to a running or completed workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHandle {
    pub run_id: String,
}

impl RunHandle {
    pub fn new() -> Self {
        Self {
            run_id: Ulid::new().to_string(),
        }
    }
}

impl Default for RunHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// The outcome of a completed outline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineRunResult {
    pub run_id: String,
    pub task_id: String,
    pub status: String,
    pub proposal: Option<OutlineProposal>,
    pub error: Option<String>,
    pub raw_output: Option<String>,
}

/// Combined outcome of `run_developmental_review` (BACKLOG §F2).  One
/// LLM call (dev-editor on the chapter) + deterministic continuity-linter
/// passes over each scene under the chapter.  The deterministic linter
/// is free (no LLM budget consumed) so we can run it on every scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalReviewResult {
    pub chapter_id: String,
    pub dev_run_id: String,
    pub dev_task_id: String,
    /// "completed" | "invalid" | "error" | "cancelled"
    pub dev_status: String,
    pub dev_notes: Option<booksforge_domain::DevelopmentalNotes>,
    pub dev_error: Option<String>,
    pub dev_raw: Option<String>,
    /// Deterministic continuity-linter findings, grouped per scene.
    /// Empty when the chapter has no scenes or no findings.
    pub continuity_findings: Vec<ContinuityScenePass>,
    /// Total scenes scanned by the deterministic linter.
    pub scenes_scanned: u32,
}

/// One scene's deterministic continuity-linter pass output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityScenePass {
    pub scene_id: String,
    pub scene_title: String,
    pub findings: Vec<booksforge_domain::ContinuityFinding>,
}

/// Combined outcome of `run_intake_and_outline` (BACKLOG §E1).
/// Both halves of the chained run surface here so the UI can render
/// the brief (for confirmation) and the outline (for application)
/// from a single Tauri call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeAndOutlineResult {
    pub intake_run_id: String,
    pub intake_task_id: String,
    pub brief: Option<ProjectBrief>,
    pub intake_error: Option<String>,
    pub intake_raw: Option<String>,
    pub outline_run_id: Option<String>,
    pub outline_task_id: Option<String>,
    /// "completed" | "invalid" | "error" | "cancelled" | "skipped"
    pub outline_status: String,
    pub outline: Option<OutlineProposal>,
    pub outline_error: Option<String>,
    pub outline_raw: Option<String>,
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Workflow orchestrator (Layer 4 infrastructure).
///
/// Holds the Ollama client and storage handle.  Run cap enforcement is
/// stateless — checked against elapsed time and call counts on each step.
pub struct Orchestrator {
    ollama: Arc<dyn booksforge_ollama::client::OllamaClient>,
    storage: Arc<SqliteStorage>,
    /// Optional snapshot service.  Required for any flow that *applies* edits
    /// (per MZ-06: every applied edit must be preceded by a `pre_agent_edit`
    /// snapshot).  Outline-architect's proposal-only flow does not need it.
    snapshot: Option<Arc<SnapshotService>>,
    config: OrchestratorConfig,
}

impl Orchestrator {
    pub fn new(
        ollama: Arc<dyn booksforge_ollama::client::OllamaClient>,
        storage: Arc<SqliteStorage>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            ollama,
            storage,
            snapshot: None,
            config,
        }
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

    /// Run a "stateless" agent — one that uses an empty `RunContext`,
    /// no peer reviews, no Tier-2 validator, no source-text or
    /// prior-scene corpus, and no per-token sink. The vast majority
    /// of agents fit this shape; this helper saves ~30 lines of
    /// `RunInput` boilerplate per call site.
    ///
    /// Agents that need any of the non-default fields (e.g. polish
    /// stages that take `source_text`, scene-critic with `tier_2`)
    /// must build the `RunInput` directly.
    pub(crate) async fn run_simple_agent<'a, T, P>(
        &self,
        spec: &'a booksforge_agents::AgentSpec,
        workflow_id: &'a str,
        project_id: Ulid,
        vars: TemplateVars,
        model: &'a str,
        cancel: CancelToken,
        parse: P,
        proposed_memory_scopes: &'a [String],
    ) -> Result<crate::runner::AgentRunResult<T>, OrchestratorError>
    where
        T: Serialize + Send + 'static,
        P: Fn(&str) -> Result<T, String> + Send + Sync + 'a,
    {
        let input = crate::runner::RunInput {
            spec,
            workflow_id,
            project_id,
            vars,
            model,
            cancel,
            parse: Box::new(parse),
            context: &crate::runner::RunContext::empty(),
            proposed_memory_scopes,
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: None,
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Copyeditor agent against a single scene.  Uses the generic
    /// runner; surfaces a `CopyeditProposals` plus a full `VerificationReport`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_copyedit_scene(
        &self,
        project_id: Ulid,
        scene_text: String,
        scene_title: String,
        style_book: serde_json::Value,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::CopyeditProposals>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::copyeditor::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_text".into(), serde_json::json!(scene_text));
        vars.insert("scene_title".into(), serde_json::json!(scene_title));
        vars.insert("style_book".into(), style_book);

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
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Continuity agent (LLM adjudicator half).  Caller is
    /// responsible for running the deterministic linter first and passing
    /// only the ambiguous findings here via `vars`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_continuity_adjudication(
        &self,
        project_id: Ulid,
        ambiguous_findings: serde_json::Value,
        known_entities: serde_json::Value,
        scene_excerpts: serde_json::Value,
        project_pov: Option<String>,
        prior_summary: Option<String>,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::ContinuityReport>, OrchestratorError>
    {
        let spec = booksforge_agents::continuity::spec();
        let mut vars = TemplateVars::new();
        vars.insert("ambiguous_findings".into(), ambiguous_findings);
        vars.insert("known_entities".into(), known_entities);
        vars.insert("scene_excerpts".into(), scene_excerpts);
        if let Some(p) = project_pov {
            vars.insert("project_pov".into(), serde_json::json!(p));
        }
        if let Some(s) = prior_summary {
            vars.insert("prior_summary".into(), serde_json::json!(s));
        }

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
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Intake agent: free-text idea → typed `ProjectBrief`.
    pub async fn run_intake(
        &self,
        project_id: Ulid,
        idea_text: String,
        preferred_mode: Option<String>,
        model: String,
        cancel: CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::ProjectBrief>, OrchestratorError>
    {
        let spec = booksforge_agents::intake::spec();
        let mut vars = TemplateVars::new();
        vars.insert("idea_text".into(), serde_json::json!(idea_text));
        if let Some(m) = preferred_mode {
            vars.insert("preferred_mode".into(), serde_json::json!(m));
        }
        self.run_simple_agent(
            &spec,
            "intake",
            project_id,
            vars,
            &model,
            cancel,
            booksforge_agents::intake::parse_and_validate,
            &[],
        )
        .await
    }

    /// Run the `concept_scorer` agent against a saved `ProjectBrief`.
    /// Returns a `ConceptScoreProposal` with per-axis scores (0-10),
    /// composite, weakest axis, and 0-5 targeted revision suggestions.
    ///
    /// Used as Stage 1's quality gate (Phase C). The proposal passes
    /// the gate when composite ≥ 8.5 AND every axis ≥ 7.0. Sub-8.5
    /// composites surface to the writer as a panel with the weakest
    /// axis named + one-click "apply this edit" buttons.
    pub async fn run_concept_scorer(
        &self,
        project_id: Ulid,
        brief: &booksforge_domain::ProjectBrief,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::ConceptScoreProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::concept_scorer::spec();
        let brief_json =
            serde_json::to_value(brief).map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let mut vars = TemplateVars::new();
        vars.insert("brief".into(), brief_json);
        // Concept scorer doesn't write memory — its output flows
        // straight back to the UI for the writer to act on.
        self.run_simple_agent(
            &spec,
            "concept-scorer",
            project_id,
            vars,
            &model,
            cancel,
            booksforge_agents::concept_scorer::parse_and_validate,
            &[],
        )
        .await
    }

    /// Run the `audience-mapper` agent against a saved `ProjectBrief`.
    /// Returns a `ReaderExpectationMap` (genre expectations + emotional
    /// promises + recommended themes/tropes + tropes to avoid +
    /// pacing expectation). Used as Stage 2's output.
    pub async fn run_audience_mapper(
        &self,
        project_id: Ulid,
        brief: &booksforge_domain::ProjectBrief,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::ReaderExpectationMap>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::audience_mapper::spec();
        let brief_json =
            serde_json::to_value(brief).map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let mut vars = TemplateVars::new();
        vars.insert("brief".into(), brief_json);
        let scopes = [String::from("book")]; // writes book:audience_map
        self.run_simple_agent(
            &spec,
            "audience-mapper",
            project_id,
            vars,
            &model,
            cancel,
            booksforge_agents::audience_mapper::parse_and_validate,
            &scopes,
        )
        .await
    }

    /// Run the `character-critic` agent against the saved character
    /// bible. Returns a `CharacterCriticProposal` (per-card 5-axis
    /// scores, cross-card structural findings, per-card edits).
    ///
    /// Pulls the bible from entity-scope memory (`character:*` keys)
    /// the same way `apply_character_bible` writes it.
    pub async fn run_character_critic(
        &self,
        project_id: Ulid,
        brief: &booksforge_domain::ProjectBrief,
        bible: &booksforge_domain::CharacterBibleProposal,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::CharacterCriticProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::character_critic::spec();
        let brief_json =
            serde_json::to_value(brief).map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let bible_json =
            serde_json::to_value(bible).map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let mut vars = TemplateVars::new();
        vars.insert("brief".into(), brief_json);
        vars.insert("characters".into(), bible_json);
        self.run_simple_agent(
            &spec,
            "character-critic",
            project_id,
            vars,
            &model,
            cancel,
            booksforge_agents::character_critic::parse_and_validate,
            &[],
        )
        .await
    }

    /// Run the `structure-critic` agent against a saved outline +
    /// brief. Returns a `StructureCriticProposal` (4-axis scores,
    /// structural findings, per-location edits).
    ///
    /// The caller is responsible for loading the outline from
    /// `agent_outputs` (or wherever the project persists it) and
    /// passing it as the `outline` parameter.
    pub async fn run_structure_critic(
        &self,
        project_id: Ulid,
        brief: &booksforge_domain::ProjectBrief,
        outline: &booksforge_domain::OutlineProposal,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::StructureCriticProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::structure_critic::spec();
        let brief_json =
            serde_json::to_value(brief).map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let outline_json =
            serde_json::to_value(outline).map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let mut vars = TemplateVars::new();
        vars.insert("brief".into(), brief_json);
        vars.insert("outline".into(), outline_json);
        self.run_simple_agent(
            &spec,
            "structure-critic",
            project_id,
            vars,
            &model,
            cancel,
            booksforge_agents::structure_critic::parse_and_validate,
            &[],
        )
        .await
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
        project_id: Ulid,
        idea_text: String,
        preferred_mode: Option<String>,
        target_chapter_count: u32,
        genre_overlay: Option<String>,
        model: String,
        cancel: CancelToken,
    ) -> Result<IntakeAndOutlineResult, OrchestratorError> {
        // ── Step 1: intake ──
        let intake_result = self
            .run_intake(
                project_id,
                idea_text,
                preferred_mode,
                model.clone(),
                cancel.clone(),
            )
            .await?;

        let brief = match (&intake_result.status, &intake_result.output) {
            (booksforge_domain::AgentTaskStatus::Completed, Some(b)) => b.clone(),
            _ => {
                // Surface the intake failure verbatim — outline doesn't
                // run without a typed brief.
                return Ok(IntakeAndOutlineResult {
                    intake_run_id: intake_result.run_id.to_string(),
                    intake_task_id: intake_result.task_id.to_string(),
                    brief: None,
                    intake_error: intake_result.error.clone(),
                    intake_raw: intake_result.raw_output.clone(),
                    outline_run_id: None,
                    outline_task_id: None,
                    outline_status: "skipped".into(),
                    outline: None,
                    outline_error: Some("intake did not produce a valid brief".into()),
                    outline_raw: None,
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
        let outline_result = self
            .run_outline(
                project_id,
                &brief,
                target_chapter_count,
                genre_overlay.as_deref(),
                &model,
                cancel,
            )
            .await?;

        Ok(IntakeAndOutlineResult {
            intake_run_id: intake_result.run_id.to_string(),
            intake_task_id: intake_result.task_id.to_string(),
            brief: Some(brief),
            intake_error: None,
            intake_raw: intake_result.raw_output,
            outline_run_id: Some(outline_result.run_id),
            outline_task_id: Some(outline_result.task_id),
            outline_status: outline_result.status,
            outline: outline_result.proposal,
            outline_error: outline_result.error,
            outline_raw: outline_result.raw_output,
        })
    }

    /// Run the Memory Curator agent: chapter text + current memory →
    /// proposed upserts + optional new entity stubs.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_memory_curator(
        &self,
        project_id: Ulid,
        scope: String, // "book" | "chapter" | "entity"
        node_id: Option<String>,
        chapter_text: String,
        current_memory: serde_json::Value,
        existing_entities: serde_json::Value,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::MemoryRefreshProposals>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::memory_curator::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scope".into(), serde_json::json!(scope));
        vars.insert("chapter_text".into(), serde_json::json!(chapter_text));
        vars.insert("current_memory".into(), current_memory);
        vars.insert("existing_entities".into(), existing_entities);
        if let Some(n) = node_id {
            vars.insert("node_id".into(), serde_json::json!(n));
        }

        // Memory-curator's writes are scope-checked. The MemoryScope cross-cutting
        // validator needs the proposed scopes; extract them from a forward parse.
        let proposed_scopes: Vec<String> = vec![scope.clone()];

        let parse = |raw: &str| booksforge_agents::memory_curator::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "memory-refresh",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &proposed_scopes,
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: None,
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Vocabulary Dictionary agent.
    pub async fn run_vocab_dictionary(
        &self,
        project_id: Ulid,
        recent_accepted_edits: serde_json::Value,
        recent_rejected_edits: serde_json::Value,
        current_project_vocab: serde_json::Value,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::VocabUpdateProposals>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::vocab_dictionary::spec();
        let mut vars = TemplateVars::new();
        vars.insert("recent_accepted_edits".into(), recent_accepted_edits);
        vars.insert("recent_rejected_edits".into(), recent_rejected_edits);
        vars.insert("current_project_vocab".into(), current_project_vocab);
        self.run_simple_agent(
            &spec,
            "vocab-refresh",
            project_id,
            vars,
            &model,
            cancel,
            booksforge_agents::vocab_dictionary::parse_and_validate,
            &[],
        )
        .await
    }

    // ── Specialist polish stack (BACKLOG §A15 / Phase 2) ─────────────────────

    /// Run a single polish stage on a chapter's `pm_doc`. Polymorphic over
    /// the four specialist stages — the caller selects which by passing
    /// the right `agent_id` from `booksforge_agents::*_polish::spec()`.
    /// Each stage produces a `PolishProposal` (revised pm_doc) which the
    /// caller persists via `apply_polish`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_polish_stage(
        &self,
        project_id: Ulid,
        stage: booksforge_domain::PolishStageId,
        chapter_text: String,
        genre_label: String,
        voice_constraints: String,
        pov_character: String,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::PolishProposal>, OrchestratorError>
    {
        use booksforge_domain::PolishStageId;

        let spec = match stage {
            PolishStageId::Dialogue => booksforge_agents::dialogue_polish::spec(),
            PolishStageId::Metaphor => booksforge_agents::metaphor_polish::spec(),
            PolishStageId::Voice => booksforge_agents::voice_polish::spec(),
            PolishStageId::SceneTension => booksforge_agents::scene_tension_polish::spec(),
        };
        let workflow_id = match stage {
            PolishStageId::Dialogue => "polish-dialogue",
            PolishStageId::Metaphor => "polish-metaphor",
            PolishStageId::Voice => "polish-voice",
            PolishStageId::SceneTension => "polish-scene-tension",
        };
        let mut vars = TemplateVars::new();
        vars.insert("chapter_text".into(), serde_json::json!(chapter_text));
        vars.insert("genre_label".into(), serde_json::json!(genre_label));
        vars.insert(
            "voice_constraints".into(),
            serde_json::json!(voice_constraints),
        );
        vars.insert("pov_character".into(), serde_json::json!(pov_character));
        let parse: Box<
            dyn Fn(&str) -> Result<booksforge_domain::PolishProposal, String> + Send + Sync,
        > = match stage {
            PolishStageId::Dialogue => {
                Box::new(booksforge_agents::dialogue_polish::parse_and_validate)
            }
            PolishStageId::Metaphor => {
                Box::new(booksforge_agents::metaphor_polish::parse_and_validate)
            }
            PolishStageId::Voice => Box::new(booksforge_agents::voice_polish::parse_and_validate),
            PolishStageId::SceneTension => {
                Box::new(booksforge_agents::scene_tension_polish::parse_and_validate)
            }
        };
        // The Originality cross-cutting validator wants the source text
        // the agent operated on — the unrevised chapter — so it can flag
        // long verbatim spans that survived unchanged in regions the
        // stage shouldn't have touched. The stages each have their own
        // narrow remit, so high overlap is expected; the validator's
        // tolerance is set accordingly.
        let source_text_owned = chapter_text.clone();
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id,
            project_id,
            vars,
            model: &model,
            cancel,
            parse,
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: Some(&source_text_owned),
            prior_scene_corpus: None,
            on_token,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the per-scene critic — produces `SceneCritiqueProposal`. Drives
    /// the per-scene critique-revise loop in the fiction polish stack
    /// (BACKLOG §A15 / RCA §L1.2). The reviser then runs
    /// `run_scene_drafter_fic` again with the critic's `specific_edits`
    /// folded into the scene_reveal block.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_scene_critic(
        &self,
        project_id: Ulid,
        scene_text: String,
        scene_goal: String,
        scene_conflict: String,
        scene_reveal: String,
        critic_axes: Vec<String>,
        genre_label: String,
        voice_constraints: String,
        prior_summary: String,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::SceneCritiqueProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::scene_critic::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_text".into(), serde_json::json!(scene_text));
        vars.insert("scene_goal".into(), serde_json::json!(scene_goal));
        vars.insert("scene_conflict".into(), serde_json::json!(scene_conflict));
        vars.insert("scene_reveal".into(), serde_json::json!(scene_reveal));
        vars.insert("critic_axes".into(), serde_json::json!(critic_axes));
        vars.insert("genre_label".into(), serde_json::json!(genre_label));
        vars.insert(
            "voice_constraints".into(),
            serde_json::json!(voice_constraints),
        );
        vars.insert("prior_summary".into(), serde_json::json!(prior_summary));
        let parse = |raw: &str| booksforge_agents::scene_critic::parse_and_validate(raw);
        let source_text_owned = scene_text.clone();
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "scene-critique",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: Some(&source_text_owned),
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// **Item 4 of FEATURE_HARDENING_PLAN.** Run the adaptive
    /// polish planner. Takes the structured `voice_score` (the
    /// result of `VoiceTarget::score` against the fresh draft) and
    /// the `tells_report` (output of `tells_per_1000_words`), plus
    /// the scene card and a short prose excerpt. Returns a
    /// [`booksforge_domain::PolishPlan`] — an ordered list of
    /// polish stages to invoke with TARGETED instructions.
    ///
    /// The orchestrator's polish loop reads the plan and dispatches
    /// each entry to its corresponding `run_polish_stage` call, with
    /// the per-entry instruction folded into `voice_constraints` (so
    /// the polish stage's prompt carries the targeted directive
    /// alongside the bible's voice contract).
    ///
    /// An empty `entries` plan is the correct output when the draft
    /// passes every voice dimension and has no tells — the polish
    /// loop is then skipped entirely.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_scene_planner(
        &self,
        project_id: Ulid,
        voice_score: serde_json::Value,
        tells_report: serde_json::Value,
        scene_card: serde_json::Value,
        genre_label: String,
        prose_excerpt: String,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<crate::runner::AgentRunResult<booksforge_domain::PolishPlan>, OrchestratorError>
    {
        let spec = booksforge_agents::scene_planner::spec();
        let mut vars = TemplateVars::new();
        vars.insert("voice_score".into(), voice_score);
        vars.insert("tells_report".into(), tells_report);
        vars.insert("scene_card".into(), scene_card);
        vars.insert("genre_label".into(), serde_json::json!(genre_label));
        vars.insert("prose_excerpt".into(), serde_json::json!(prose_excerpt));
        let parse = |raw: &str| booksforge_agents::scene_planner::parse_and_validate(raw);
        let source_text_owned = prose_excerpt.clone();
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "scene-planner",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: Some(&source_text_owned),
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    // ── Fiction agents (BACKLOG §A13 / Phase 1) ──────────────────────────────

    /// Run the Character Bible agent — produces `CharacterBibleProposal`.
    ///
    /// Inputs:
    /// - `project_brief`     — the typed brief produced by intake.
    /// - `chapter_count`     — number of chapters in the outline (each
    ///                         `CharacterCard.chapter_arc` must have one
    ///                         entry per chapter).
    /// - `accepted_prose`    — optional list of accepted prose paragraphs;
    ///                         the bible derives measurable voice traits
    ///                         from these when present.
    /// - `prior_bible`       — optional previously-accepted bible to extend
    ///                         (rather than redoing work) on a re-run.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_character_bible(
        &self,
        project_id: Ulid,
        project_brief: serde_json::Value,
        chapter_count: u32,
        accepted_prose: serde_json::Value,
        prior_bible: serde_json::Value,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::CharacterBibleProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::character_bible::spec();
        let mut vars = TemplateVars::new();
        vars.insert("project_brief".into(), project_brief);
        vars.insert("chapter_count".into(), serde_json::json!(chapter_count));
        vars.insert("accepted_prose_samples".into(), accepted_prose);
        vars.insert("prior_bible".into(), prior_bible);
        let cc = chapter_count as usize;
        let parse =
            move |raw: &str| booksforge_agents::character_bible::parse_and_validate(raw, cc);
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "character-bible",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &["entity".to_owned()],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: None,
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Round 7 RCA fix — produce a `CharacterBibleProposal` by
    /// generating each character one at a time and stitching the
    /// results.
    ///
    /// The monolithic `run_character_bible` asks the model for 4-6
    /// nested objects with cross-coupled constraints (chapter_arc
    /// length matching, no-duplicate-name, relationships
    /// self-referencing, exactly 3-6 voice traits per character) in a
    /// single response. Local models smaller than ~30B chronically
    /// fail this in one shot — runner cycles through max retries and
    /// returns an empty bible. Per-character chunking solves all four
    /// failure modes:
    ///
    ///   1. **Output budget**: ~250-400 tokens per call vs.
    ///      1000-1600 for the monolithic prompt — fits 9B competence.
    ///   2. **Failure isolation**: a bad character N triggers retry
    ///      of just N, not the whole bible.
    ///   3. **Wall-clock**: predictable N × ~30-60s on 9B vs.
    ///      unbounded retry-ladder on the monolithic prompt.
    ///   4. **Cross-references**: feeding `prior_characters` to each
    ///      call lets the model deterministically avoid name
    ///      collisions and reference real prior names in
    ///      relationships.
    ///
    /// Roles are assigned deterministically per the bible's hard
    /// rules (1 protagonist, 1 antagonist, then supporting roles up
    /// to `desired_count`). The full multi-character `validate()`
    /// runs once after all cards are assembled.
    ///
    /// Returns the assembled bible OR an error listing per-character
    /// validation failures. Per-card retries are bounded by the
    /// orchestrator's `max_retries_per_step`.
    pub async fn run_character_bible_chunked(
        &self,
        project_id: Ulid,
        project_brief: serde_json::Value,
        chapter_count: u32,
        desired_count: u32,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<booksforge_domain::CharacterBibleProposal, OrchestratorError> {
        // Roles to fill, in order. Mirrors the bible's hard rule:
        // 1 protagonist, 1 antagonist, then supporting / mentor /
        // foil for the remainder. `desired_count` is clamped to 2-6.
        let n = desired_count.clamp(2, 6) as usize;
        let mut roles: Vec<&'static str> = vec!["protagonist", "antagonist"];
        let extras = ["mentor", "foil", "supporting", "supporting"];
        for r in extras.iter().take(n.saturating_sub(2)) {
            roles.push(r);
        }

        let spec = booksforge_agents::character_bible_card::spec();
        let cc = chapter_count as usize;

        let mut characters: Vec<booksforge_domain::CharacterCard> = Vec::with_capacity(n);
        let mut failed_cards: Vec<(usize, &'static str)> = Vec::new();
        for (i, role) in roles.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(OrchestratorError::Cancelled);
            }
            let prior_names: Vec<String> = characters.iter().map(|c| c.name.clone()).collect();
            let prior_names_for_parse = prior_names.clone();
            let mut vars = TemplateVars::new();
            vars.insert("project_brief".into(), project_brief.clone());
            vars.insert("chapter_count".into(), serde_json::json!(chapter_count));
            vars.insert("role".into(), serde_json::json!(role));
            vars.insert("character_index".into(), serde_json::json!(i as u32 + 1));
            vars.insert("prior_characters".into(), serde_json::json!(characters));
            let parse = move |raw: &str| {
                booksforge_agents::character_bible_card::parse_and_validate(
                    raw,
                    cc,
                    &prior_names_for_parse,
                )
            };
            let input = crate::runner::RunInput {
                spec: &spec,
                workflow_id: "character-bible-card",
                project_id,
                vars,
                model: &model,
                cancel: cancel.clone(),
                parse: Box::new(parse),
                context: &context,
                proposed_memory_scopes: &["entity".to_owned()],
                peer_reviews: Vec::new(),
                tier_2: None,
                source_text: None,
                prior_scene_corpus: None,
                on_token: None,
            };
            let result = crate::runner::run(
                self.storage.clone(),
                self.ollama.clone(),
                &self.config,
                input,
            )
            .await?;
            // LENIENT failure handling — when a single card fails its
            // retry ladder, log it and continue to the next role
            // instead of aborting the whole bible. The final cross-
            // character `validate()` below decides whether the
            // assembled set is publishable.
            //
            // Per-card outcome goes to `tracing` (not `eprintln`) so
            // CLI runners can surface it via the standard subscriber
            // (`live_book_run.rs` installs a `fmt`+`env_filter` writer)
            // and the clippy `print_stderr` gate stays clean.
            match result.output {
                Some(card) => {
                    tracing::info!(
                        agent      = "character-bible-card",
                        card_index = i + 1,
                        role       = role,
                        name       = ?card.name,
                        "card OK",
                    );
                    characters.push(card);
                }
                None => {
                    let raw_preview: String = result
                        .raw_output
                        .as_deref()
                        .map(|s| {
                            let head: String = s.chars().take(500).collect();
                            head.replace('\n', " ⏎ ")
                        })
                        .unwrap_or_else(|| "(no raw output captured)".to_owned());
                    tracing::warn!(
                        agent       = "character-bible-card",
                        card_index  = i + 1,
                        role        = role,
                        error       = ?result.error,
                        raw_preview = %raw_preview,
                        "card exhausted retries; continuing to next role",
                    );
                    failed_cards.push((i + 1, role));
                }
            }
        }

        // Final cross-character validate — runs the same checks the
        // monolithic agent runs, on the assembled set. With the
        // lenient per-card policy above, this is the gate that
        // decides whether enough cards survived to call the bible
        // usable. CharacterBibleProposal::validate requires ≥2
        // characters AND a protagonist — so a run where every card
        // except the protagonist failed will still error out here.
        if !failed_cards.is_empty() {
            tracing::warn!(
                agent = "character-bible-card",
                failed = ?failed_cards,
                produced = characters.len(),
                "chunked bibles produced partial set; running final validate",
            );
        }
        // Chunked builds don't carry a voice target — that's bible-author-time
        // metadata (or a genre-pack default), set later via the bible editor
        // or genre-pack import path.
        let proposal = booksforge_domain::CharacterBibleProposal::new(characters);
        let final_errs = proposal.validate(cc);
        if !final_errs.is_empty() {
            return Err(OrchestratorError::AgentFailed {
                agent_id: "character-bible-card".into(),
                retries: 0,
                reason: format!(
                    "chunked bible final validate failed (failed_cards={failed_cards:?}): {}",
                    final_errs.join("; ")
                ),
            });
        }
        Ok(proposal)
    }

    /// Run the World Bible agent — produces `WorldBibleProposal`.
    pub async fn run_world_bible(
        &self,
        project_id: Ulid,
        project_brief: serde_json::Value,
        prior_bible: serde_json::Value,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::WorldBibleProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::world_bible::spec();
        let mut vars = TemplateVars::new();
        vars.insert("project_brief".into(), project_brief);
        vars.insert("prior_bible".into(), prior_bible);
        let parse = |raw: &str| booksforge_agents::world_bible::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "world-bible",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &["book".to_owned(), "entity".to_owned()],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: None,
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Scene Drafter (Fiction) agent — produces `SceneDraftProposal`.
    ///
    /// Fiction-shaped sibling of `run_chapter_drafter`. Loads character +
    /// world bibles from the per-call inputs (callers typically pull them
    /// from the project's accepted memory entries). Voice constraints are
    /// passed in as a string (rendered by `booksforge-voice` once Phase 3
    /// lands; for Phase 1 callers can pass an empty string).
    #[allow(clippy::too_many_arguments)]
    pub async fn run_scene_drafter_fic(
        &self,
        project_id: Ulid,
        scene_goal: String,
        scene_conflict: String,
        scene_reveal: String,
        target_words: u32,
        chapter_pov: String,
        genre_lens: String,
        character_bible: serde_json::Value,
        world_bible: serde_json::Value,
        voice_constraints: String,
        prior_summary: String,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::SceneDraftProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::scene_drafter_fic::spec();
        // FEATURE_HARDENING_PLAN.md §1.6 — when the bible carries a numeric
        // voice contract (CharacterBibleProposal.voice_target), render its
        // directive_block here and pass into the prompt template via
        // `voice_target_directive`. Falls back to empty string when the
        // bible was authored before §1.6 or has no target — the drafter
        // then relies on per-character voice_traits strings only.
        let voice_target_directive: String = serde_json::from_value::<
            booksforge_domain::CharacterBibleProposal,
        >(character_bible.clone())
        .ok()
        .map(|b| b.voice_target_directive())
        .unwrap_or_default();

        // Item 5 of FEATURE_HARDENING_PLAN — load top-K exemplars for
        // this agent from `agent_exemplars` and inject them into the
        // prompt as in-context examples. Cross-project lookup
        // (project_id = None) so the drafter learns house style from
        // the user's prior accepted prose anywhere in their workspace.
        // Cold-start (no exemplars yet) returns the empty string and
        // the prompt template renders nothing for the slot.
        const EXEMPLAR_LIMIT: i64 = 3;
        let exemplars_block: String = match self
            .storage
            .fetch_top_exemplars("scene-drafter-fic", None, EXEMPLAR_LIMIT)
            .await
        {
            Ok(exemplars) => booksforge_storage::render_exemplars_block(&exemplars),
            Err(e) => {
                // Exemplar lookup failure is NOT fatal — the drafter can run
                // without exemplars (cold-start mode). Log and continue.
                tracing::warn!(
                    agent = "scene-drafter-fic",
                    error = ?e,
                    "exemplar fetch failed; running without exemplars",
                );
                String::new()
            }
        };
        let mut vars = TemplateVars::new();
        vars.insert("scene_goal".into(), serde_json::json!(scene_goal));
        vars.insert("scene_conflict".into(), serde_json::json!(scene_conflict));
        vars.insert("scene_reveal".into(), serde_json::json!(scene_reveal));
        vars.insert("target_words".into(), serde_json::json!(target_words));
        vars.insert("chapter_pov".into(), serde_json::json!(chapter_pov));
        vars.insert("genre_lens".into(), serde_json::json!(genre_lens));
        vars.insert("character_bible".into(), character_bible);
        vars.insert("world_bible".into(), world_bible);
        vars.insert(
            "voice_constraints".into(),
            serde_json::json!(voice_constraints),
        );
        vars.insert(
            "voice_target_directive".into(),
            serde_json::json!(voice_target_directive),
        );
        vars.insert("exemplars_block".into(), serde_json::json!(exemplars_block));
        vars.insert("prior_summary".into(), serde_json::json!(prior_summary));
        let parse = |raw: &str| booksforge_agents::scene_drafter_fic::parse_and_validate(raw);
        // The drafter's source_text is the scene goal — drives the
        // Originality check (catches goal-string echo from the prompt
        // appearing verbatim in the prose).
        let source_text_owned = scene_goal.clone();
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "draft-scene-fic",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: Some(&source_text_owned),
            prior_scene_corpus: None,
            on_token,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// **Run #14 Fix-1.** Long-scene drafter via beat decomposition.
    ///
    /// Mirrors the chunked-bible pattern: instead of asking the
    /// drafter to produce a 1500-word scene in one call (which it
    /// judges complete at ~500 and stops, undershooting the target
    /// — Run #14 hit 33% of target), break the scene into 3–5
    /// micro-beats of 400–600 words each. Each beat is a
    /// complete-feeling sub-scene with its own goal / conflict /
    /// reveal that the drafter can finish without padding.
    /// Concatenate the resulting `pm_doc.content` paragraphs into
    /// one combined `SceneDraftProposal`.
    ///
    /// Each beat call inherits the running prior-summary from the
    /// previous beats' notes, so the model knows where in the arc
    /// it is and writes accordingly. Failure of a single beat is
    /// **lenient**: the chunked run continues to the next beat,
    /// matching the chunked-bible policy. Final pm_doc must contain
    /// at least one paragraph or the whole call returns an error.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_scene_drafter_fic_chunked(
        &self,
        project_id: Ulid,
        beats: Vec<booksforge_domain::SceneBeat>,
        chapter_pov: String,
        genre_lens: String,
        character_bible: serde_json::Value,
        world_bible: serde_json::Value,
        voice_constraints: String,
        initial_prior_summary: String,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<booksforge_domain::SceneDraftProposal, OrchestratorError> {
        if beats.is_empty() {
            return Err(OrchestratorError::AgentFailed {
                agent_id: "scene-drafter-fic-chunked".into(),
                retries: 0,
                reason: "beats list is empty — nothing to draft".into(),
            });
        }
        let mut beat_errors: Vec<String> = Vec::new();
        for b in &beats {
            beat_errors.extend(b.validate());
        }
        if !beat_errors.is_empty() {
            return Err(OrchestratorError::AgentFailed {
                agent_id: "scene-drafter-fic-chunked".into(),
                retries: 0,
                reason: format!("beat validation failed: {}", beat_errors.join("; ")),
            });
        }

        let mut all_paragraphs: Vec<serde_json::Value> = Vec::new();
        let mut total_word_count: u32 = 0;
        let mut all_notes: Vec<String> = Vec::new();
        let mut failed_beats: Vec<String> = Vec::new();
        let mut running_prior_summary: String = initial_prior_summary;

        for (i, beat) in beats.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(OrchestratorError::Cancelled);
            }
            tracing::info!(
                agent      = "scene-drafter-fic-chunked",
                beat_index = i + 1,
                beat_id    = %beat.beat_id,
                target     = beat.target_words,
                "drafting beat",
            );

            let result = self
                .run_scene_drafter_fic(
                    project_id,
                    beat.goal.clone(),
                    beat.conflict.clone(),
                    beat.reveal.clone(),
                    beat.target_words,
                    chapter_pov.clone(),
                    genre_lens.clone(),
                    character_bible.clone(),
                    world_bible.clone(),
                    voice_constraints.clone(),
                    running_prior_summary.clone(),
                    context.clone(),
                    model.clone(),
                    cancel.clone(),
                    on_token.clone(),
                )
                .await?;

            match result.output {
                Some(proposal) => {
                    tracing::info!(
                        agent      = "scene-drafter-fic-chunked",
                        beat_index = i + 1,
                        beat_id    = %beat.beat_id,
                        word_count = proposal.word_count,
                        "beat OK",
                    );
                    // Append paragraphs from this beat into the combined doc.
                    if let Some(content) = proposal.pm_doc.get("content").and_then(|c| c.as_array())
                    {
                        all_paragraphs.extend(content.iter().cloned());
                    }
                    total_word_count = total_word_count.saturating_add(proposal.word_count);
                    all_notes.push(format!("[{}] {}", beat.beat_id, proposal.notes));
                    // Build the prior_summary for the NEXT beat — uses this
                    // beat's notes so the drafter knows what just happened
                    // and writes the next beat with continuity.
                    running_prior_summary = format!(
                        "{}\n\nJust completed [{}]: {}",
                        running_prior_summary.trim_end(),
                        beat.beat_id,
                        proposal.notes,
                    );
                }
                None => {
                    tracing::warn!(
                        agent      = "scene-drafter-fic-chunked",
                        beat_index = i + 1,
                        beat_id    = %beat.beat_id,
                        error      = ?result.error,
                        "beat exhausted retries; continuing to next beat (lenient policy)",
                    );
                    failed_beats.push(beat.beat_id.clone());
                }
            }
        }

        if all_paragraphs.is_empty() {
            return Err(OrchestratorError::AgentFailed {
                agent_id: "scene-drafter-fic-chunked".into(),
                retries: 0,
                reason: format!(
                    "every beat failed (failed_beats={failed_beats:?}); no prose produced"
                ),
            });
        }
        if !failed_beats.is_empty() {
            tracing::warn!(
                agent  = "scene-drafter-fic-chunked",
                failed = ?failed_beats,
                produced_words = total_word_count,
                "chunked draft produced partial scene; some beats failed",
            );
        }

        let combined_notes = if all_notes.is_empty() {
            String::new()
        } else {
            // Cap combined notes at ~120 words to satisfy SceneDraftProposal::validate.
            let joined = all_notes.join(" / ");
            joined
                .split_whitespace()
                .take(120)
                .collect::<Vec<_>>()
                .join(" ")
        };
        Ok(booksforge_domain::SceneDraftProposal {
            pm_doc: serde_json::json!({
                "type":    "doc",
                "content": all_paragraphs,
            }),
            word_count: total_word_count,
            notes: combined_notes,
        })
    }

    /// Run the Chapter Drafter agent: synopsis + context → SceneDraftProposal.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_chapter_drafter(
        &self,
        project_id: Ulid,
        scene_synopsis: String,
        chapter_purpose: String,
        project_pov: String,
        target_words: u32,
        known_entities: serde_json::Value,
        prior_summary: String,
        voice_fingerprint: serde_json::Value,
        genre: Option<String>,
        tone: Option<String>,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::SceneDraftProposal>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::chapter_drafter::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_synopsis".into(), serde_json::json!(scene_synopsis));
        vars.insert("chapter_purpose".into(), serde_json::json!(chapter_purpose));
        vars.insert("project_pov".into(), serde_json::json!(project_pov));
        vars.insert("target_words".into(), serde_json::json!(target_words));
        vars.insert("known_entities".into(), known_entities);
        vars.insert("prior_summary".into(), serde_json::json!(prior_summary));
        vars.insert("voice_fingerprint".into(), voice_fingerprint);
        if let Some(g) = genre {
            vars.insert("genre".into(), serde_json::json!(g));
        }
        if let Some(t) = tone {
            vars.insert("tone".into(), serde_json::json!(t));
        }
        let parse = |raw: &str| booksforge_agents::chapter_drafter::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "draft-scene",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &context,
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: Some(&scene_synopsis),
            prior_scene_corpus: None,
            on_token,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Developmental Editor agent on one chapter.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_dev_editor(
        &self,
        project_id: Ulid,
        chapter_id: String,
        chapter_text: String,
        project_brief: serde_json::Value,
        prior_chapter_summaries: serde_json::Value,
        known_entities: serde_json::Value,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::DevelopmentalNotes>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::dev_editor::spec();
        let mut vars = TemplateVars::new();
        vars.insert("chapter_id".into(), serde_json::json!(chapter_id));
        vars.insert("chapter_text".into(), serde_json::json!(chapter_text));
        vars.insert("project_brief".into(), project_brief);
        vars.insert("prior_chapter_summaries".into(), prior_chapter_summaries);
        vars.insert("known_entities".into(), known_entities);
        let parse = |raw: &str| booksforge_agents::dev_editor::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "dev-review",
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
            on_token,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
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
        project_id: Ulid,
        chapter_id: String,
        chapter_text: String,
        per_scene_text: Vec<(Ulid, String, String)>, // (scene_id, title, text)
        project_brief: serde_json::Value,
        prior_chapter_summaries: serde_json::Value,
        known_entities: serde_json::Value,
        project_pov: Option<String>,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<DevelopmentalReviewResult, OrchestratorError> {
        // ── 1. dev_editor (1 LLM call) ──
        let dev = self
            .run_dev_editor(
                project_id,
                chapter_id.clone(),
                chapter_text,
                project_brief,
                prior_chapter_summaries,
                known_entities,
                context.clone(),
                model,
                cancel,
                on_token,
            )
            .await?;

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
                    scene_id: scene_id.to_string(),
                    scene_title: scene_title.clone(),
                    findings,
                });
            }
        }

        let dev_status = match dev.status {
            booksforge_domain::AgentTaskStatus::Completed => "completed",
            booksforge_domain::AgentTaskStatus::Cancelled => "cancelled",
            booksforge_domain::AgentTaskStatus::Error => "error",
            _ => "invalid",
        };

        Ok(DevelopmentalReviewResult {
            chapter_id,
            dev_run_id: dev.run_id.to_string(),
            dev_task_id: dev.task_id.to_string(),
            dev_status: dev_status.to_owned(),
            dev_notes: dev.output,
            dev_error: dev.error,
            dev_raw: dev.raw_output,
            continuity_findings,
            scenes_scanned: per_scene_text.len() as u32,
        })
    }

    /// Run the Humanization agent: scene text + voice fingerprint + active
    /// avoid-rules → concrete `before/after` edits with `triggered_rule`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_humanization(
        &self,
        project_id: Ulid,
        scene_text: String,
        scene_title: String,
        active_avoid_rules: serde_json::Value,
        voice_fingerprint: serde_json::Value,
        context: crate::runner::RunContext,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::HumanizationProposals>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::humanization::spec();
        let mut vars = TemplateVars::new();
        vars.insert("scene_text".into(), serde_json::json!(scene_text));
        vars.insert("scene_title".into(), serde_json::json!(scene_title));
        vars.insert("active_avoid_rules".into(), active_avoid_rules);
        vars.insert("voice_fingerprint".into(), voice_fingerprint);

        let source_for_parse = scene_text.clone();
        let parse = move |raw: &str| {
            booksforge_agents::humanization::parse_and_validate(raw, &source_for_parse)
        };
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "humanization",
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
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
    }

    /// Run the Tier-2 Proposal Validator on another agent's output.
    /// Returns a `ProposalValidation` carrying the four context-fitness
    /// axes (faithfulness, style, coherence, self_consistency).
    #[allow(clippy::too_many_arguments)]
    pub async fn run_proposal_validator_tier2(
        &self,
        project_id: Ulid,
        primary_agent_id: String,
        primary_output: serde_json::Value,
        context_excerpt: String,
        tier_1_findings: serde_json::Value,
        voice_fingerprint: serde_json::Value,
        active_avoid_rules: serde_json::Value,
        model: String,
        cancel: CancelToken,
    ) -> Result<
        crate::runner::AgentRunResult<booksforge_domain::ProposalValidation>,
        OrchestratorError,
    > {
        let spec = booksforge_agents::proposal_validator::spec();
        let mut vars = TemplateVars::new();
        vars.insert(
            "primary_agent_id".into(),
            serde_json::json!(primary_agent_id),
        );
        vars.insert("primary_output".into(), primary_output);
        vars.insert("context_excerpt".into(), serde_json::json!(context_excerpt));
        vars.insert("tier_1_findings".into(), tier_1_findings);
        vars.insert("voice_fingerprint".into(), voice_fingerprint);
        vars.insert("active_avoid_rules".into(), active_avoid_rules);
        let parse = |raw: &str| booksforge_agents::proposal_validator::parse_and_validate(raw);
        let input = crate::runner::RunInput {
            spec: &spec,
            workflow_id: "proposal-validator-tier2",
            project_id,
            vars,
            model: &model,
            cancel,
            parse: Box::new(parse),
            context: &crate::runner::RunContext::empty(),
            proposed_memory_scopes: &[],
            peer_reviews: Vec::new(),
            tier_2: None,
            source_text: None,
            prior_scene_corpus: None,
            on_token: None,
        };
        crate::runner::run(
            self.storage.clone(),
            self.ollama.clone(),
            &self.config,
            input,
        )
        .await
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
        project_id: Ulid,
        primary_agent_id: &str,
        primary_task_id: String,
        primary_output: serde_json::Value,
        context_excerpt: String,
        context: &crate::runner::RunContext,
        high_confidence_mode: bool,
        model: String,
        cancel: CancelToken,
    ) -> Vec<booksforge_domain::PeerReviewResult> {
        let pairings = crate::council::select_pairings(primary_agent_id, high_confidence_mode);
        if pairings.is_empty() {
            return Vec::new();
        }

        let cap = (self.config.max_agent_calls.saturating_sub(1)) as usize;
        let take = pairings.len().min(cap.max(1));

        let known_entities =
            serde_json::to_value(&context.entity_bible).unwrap_or_else(|_| serde_json::json!([]));
        let voice_fp = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));
        let avoid_rules = serde_json::to_value(&context.active_avoid_rules)
            .unwrap_or_else(|_| serde_json::json!([]));

        let mut out = Vec::with_capacity(take);
        for pairing in pairings.iter().take(take) {
            let focus_str = peer_focus_str(pairing.focus);
            let spec = booksforge_agents::peer_review::spec();
            let mut vars = TemplateVars::new();
            vars.insert(
                "reviewer_agent_id".into(),
                serde_json::json!(pairing.reviewer_agent_id),
            );
            vars.insert(
                "primary_agent_id".into(),
                serde_json::json!(primary_agent_id),
            );
            vars.insert("primary_task_id".into(), serde_json::json!(primary_task_id));
            vars.insert("primary_output".into(), primary_output.clone());
            vars.insert("focus".into(), serde_json::json!(focus_str));
            vars.insert(
                "context_excerpt".into(),
                serde_json::json!(context_excerpt.clone()),
            );
            vars.insert("known_entities".into(), known_entities.clone());
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
            match crate::runner::run(
                self.storage.clone(),
                self.ollama.clone(),
                &self.config,
                input,
            )
            .await
            {
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
        result: &mut crate::runner::AgentRunResult<T>,
        peer_reviews: Vec<booksforge_domain::PeerReviewResult>,
    ) {
        if peer_reviews.is_empty() {
            return;
        }
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
        project_id: Ulid,
        primary_agent_id: &str,
        result: &mut crate::runner::AgentRunResult<T>,
        context: &crate::runner::RunContext,
        context_excerpt: String,
        model: String,
        cancel: CancelToken,
    ) -> Result<(), OrchestratorError> {
        if !self.config.tier2_enabled {
            return Ok(());
        }
        if result.output.is_none() {
            return Ok(());
        }
        if result.verification.tier_2.is_some() {
            return Ok(());
        }

        let primary_output_json = result
            .output
            .as_ref()
            .and_then(|o| serde_json::to_value(o).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let tier_1_findings = serde_json::to_value(&result.verification.tier_1)
            .unwrap_or_else(|_| serde_json::json!({}));
        let voice_fp_json = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));
        let avoid_json = serde_json::to_value(&context.active_avoid_rules)
            .unwrap_or_else(|_| serde_json::json!([]));

        let tier2_run = self
            .run_proposal_validator_tier2(
                project_id,
                primary_agent_id.to_owned(),
                primary_output_json,
                context_excerpt,
                tier_1_findings,
                voice_fp_json,
                avoid_json,
                model,
                cancel,
            )
            .await;

        match tier2_run {
            Ok(r) if r.output.is_some() => {
                // Safe — guarded by `r.output.is_some()` in the match arm.
                let Some(mut tier_2) = r.output else {
                    unreachable!()
                };
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
            Ok(_) => tracing::warn!(
                primary = primary_agent_id,
                "tier-2 returned no output — keeping tier-1 verdict"
            ),
            Err(e) => {
                tracing::warn!(primary = primary_agent_id, error = %e, "tier-2 dispatch failed — keeping tier-1 verdict")
            }
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
        scope: SnapshotScope,
        scope_id: Option<ulid::Ulid>,
        label: Option<String>,
    ) -> Result<SnapshotRecord, OrchestratorError> {
        let svc = self.snapshot.as_ref().ok_or_else(|| {
            OrchestratorError::Storage(
                "snapshot service not attached — cannot take pre_agent_edit snapshot".to_owned(),
            )
        })?;
        svc.pre_agent_edit_snapshot(scope, scope_id, label)
            .await
            .map_err(|e: SnapshotError| OrchestratorError::Storage(e.to_string()))
    }

    /// Run the `outline-from-brief` workflow.
    ///
    /// Persists `agent_runs`, `agent_tasks`, `agent_outputs` rows.
    /// Enforces time and retry caps from `OrchestratorConfig`.
    pub async fn run_outline(
        &self,
        project_id: Ulid,
        brief: &ProjectBrief,
        target_chapter_count: u32,
        genre_overlay: Option<&str>,
        model: &str,
        cancel: CancelToken,
    ) -> Result<OutlineRunResult, OrchestratorError> {
        self.run_outline_with_progress(
            project_id,
            brief,
            target_chapter_count,
            genre_overlay,
            model,
            cancel,
            None,
        )
        .await
    }

    /// Same as [`run_outline`], but the caller can hand in an `on_token`
    /// callback that fires once per streamed token. Used by the desktop
    /// IPC layer to emit `agent-run-progress` events so the wizard's
    /// "Generating outline…" dialog can show a live token-rate / elapsed
    /// counter rather than appearing frozen.
    pub async fn run_outline_with_progress(
        &self,
        project_id: Ulid,
        brief: &ProjectBrief,
        target_chapter_count: u32,
        genre_overlay: Option<&str>,
        model: &str,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<OutlineRunResult, OrchestratorError> {
        let run_id = Ulid::new();
        let now = Utc::now();

        // Insert the run row (status = running).
        self.storage
            .agent_run_insert(&AgentRun {
                id: run_id,
                workflow_id: "outline-from-brief".to_owned(),
                project_id,
                status: AgentTaskStatus::Running,
                started_at: now,
                completed_at: None,
                total_tokens: None,
                error_message: None,
                ollama_version: None,
                user_initiated: true,
            })
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        let result = self
            .run_outline_inner(
                run_id,
                project_id,
                brief,
                target_chapter_count,
                genre_overlay,
                model,
                cancel,
                on_token,
            )
            .await;

        // Finalise the run row.
        let (final_status, err_msg) = match &result {
            Ok(r) if r.proposal.is_some() => (AgentTaskStatus::Completed, None),
            Ok(_) => (AgentTaskStatus::Invalid, None),
            Err(OrchestratorError::Cancelled) => (
                AgentTaskStatus::Cancelled,
                Some("cancelled by user".to_owned()),
            ),
            Err(e) => (AgentTaskStatus::Error, Some(e.to_string())),
        };
        let _ = self
            .storage
            .agent_run_update(run_id, final_status, None, err_msg.as_deref())
            .await;

        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_outline_inner(
        &self,
        run_id: Ulid,
        project_id: Ulid,
        brief: &ProjectBrief,
        target_chapter_count: u32,
        genre_overlay: Option<&str>,
        model: &str,
        cancel: CancelToken,
        on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<OutlineRunResult, OrchestratorError> {
        let _ = project_id; // used in run row; not needed in inner logic yet
        let spec = booksforge_agents::outline_architect::spec();
        let template_id = spec.prompt_template.clone();
        let wall_start = Instant::now();
        let task_id = Ulid::new();

        // ── Build template vars ───────────────────────────────────────────
        let brief_json =
            serde_json::to_value(brief).map_err(|e| OrchestratorError::AgentFailed {
                agent_id: "outline-architect".into(),
                retries: 0,
                reason: format!("failed to serialize brief: {e}"),
            })?;

        let mut vars = TemplateVars::new();
        vars.insert("brief".to_owned(), brief_json);
        vars.insert(
            "target_chapter_count".to_owned(),
            serde_json::json!(target_chapter_count),
        );
        if let Some(overlay) = genre_overlay {
            vars.insert("genre_overlay".to_owned(), serde_json::json!(overlay));
        }

        // ── Render prompt ─────────────────────────────────────────────────
        let rendered = render(&template_id, &vars).map_err(|e| OrchestratorError::AgentFailed {
            agent_id: "outline-architect".into(),
            retries: 0,
            reason: format!("prompt render failed: {e}"),
        })?;

        let template_hash_hex = rendered.template_hash.to_hex().to_string();
        let input_blob = serde_json::to_string(&vars).unwrap_or_default();
        let input_hash = blake3::hash(input_blob.as_bytes()).to_hex().to_string();

        // ── Insert task row ───────────────────────────────────────────────
        let task_now = Utc::now();
        self.storage
            .agent_task_insert(&AgentTask {
                id: task_id,
                run_id,
                step_index: 0,
                agent_id: "outline-architect".to_owned(),
                prompt_template_id: format!("{}.{}", template_id.id, template_id.version),
                prompt_template_hash: template_hash_hex,
                model: model.to_owned(),
                model_digest: None,
                input_hash,
                output_hash: None,
                context_tokens: None,
                output_tokens: None,
                duration_ms: None,
                retries: 0,
                status: AgentTaskStatus::Running,
                error_category: None,
                error_message: None,
                created_at: task_now,
                updated_at: task_now,
            })
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // ── Retry loop ────────────────────────────────────────────────────
        // `last_raw` / `last_error` are populated on every iteration before
        // the post-loop block reads them; the initial `None` assignments are
        // dead but required to satisfy definite-initialisation.
        #[allow(unused_assignments)]
        let mut attempt: u32 = 0;
        #[allow(unused_assignments)]
        let mut last_raw: Option<String> = None;
        #[allow(unused_assignments)]
        let mut last_error: Option<String> = None;

        loop {
            if cancel.is_cancelled() {
                self.storage
                    .agent_task_update(
                        task_id,
                        AgentTaskStatus::Cancelled,
                        None,
                        None,
                        None,
                        Some(wall_start.elapsed().as_millis() as u64),
                        attempt,
                        None,
                        Some("cancelled by user"),
                    )
                    .await
                    .ok();
                return Err(OrchestratorError::Cancelled);
            }
            if wall_start.elapsed().as_secs() >= self.config.max_duration_secs {
                let msg = format!(
                    "time limit {}s exceeded after attempt {attempt}",
                    self.config.max_duration_secs
                );
                self.storage
                    .agent_task_update(
                        task_id,
                        AgentTaskStatus::Error,
                        None,
                        None,
                        None,
                        Some(wall_start.elapsed().as_millis() as u64),
                        attempt,
                        Some("timeout"),
                        Some(&msg),
                    )
                    .await
                    .ok();
                return Err(OrchestratorError::TimeLimitExceeded {
                    limit_secs: self.config.max_duration_secs,
                });
            }

            // Build chat request.
            let messages = vec![
                ChatMessage {
                    role: "system".to_owned(),
                    content: rendered.system.clone(),
                },
                ChatMessage {
                    role: "user".to_owned(),
                    content: rendered.user.clone(),
                },
            ];
            // Translate the agent's declared DefaultThinking into the wire
            // `think` field. See runner.rs for the rationale (Qwen-3.x footgun).
            let think = match spec.default_thinking {
                booksforge_agents::DefaultThinking::Disabled => Some(false),
                booksforge_agents::DefaultThinking::Enabled => Some(true),
                booksforge_agents::DefaultThinking::ModelDefault => None,
            };
            let req = ChatRequest {
                model: model.to_owned(),
                messages,
                stream: true,
                think,
                // Run #13 fix — force JSON-mode at decoder level. See
                // runner.rs for the full rationale.
                format: Some("json".to_owned()),
                options: Some(GenerateOptions {
                    temperature: Some(0.3),
                    top_p: None,
                    // RCA_RUN15_THRASH.md Fix 1 — pipeline num_ctx pin.
                    num_ctx: Some(
                        self.config
                            .pipeline_num_ctx_override
                            .unwrap_or_else(|| spec.context_budget.total()),
                    ),
                    num_predict: Some(spec.context_budget.max_output_tokens),
                    // This code path is the outline-architect-specific
                    // loop. The scene-drafter hardening (see runner.rs)
                    // doesn't apply here — outline-architect emits a
                    // bounded chapter list that doesn't trip the
                    // explainer-loop failure mode.
                    repeat_penalty: None,
                    stop: None,
                }),
            };

            // Stream tokens into a buffer. Also fan out to the caller's
            // optional on_token sink so the desktop IPC layer can emit
            // `agent-run-progress` events for the wizard's spinner.
            let buf = Arc::new(std::sync::Mutex::new(String::new()));
            let buf2 = buf.clone();
            let on_token_for_sink = on_token.clone();
            let sink: TokenSink = Box::new(move |tok: &str| {
                if let Ok(mut b) = buf2.lock() {
                    b.push_str(tok);
                }
                if let Some(ref cb) = on_token_for_sink {
                    cb(tok);
                }
            });

            let step_start = Instant::now();
            let chat_result = self.ollama.chat(req, sink, cancel.clone()).await;
            let step_ms = step_start.elapsed().as_millis() as u64;

            let raw_text = match chat_result {
                Ok(ref outcome) if !outcome.message.content.is_empty() => {
                    outcome.message.content.clone()
                }
                _ => buf.lock().map(|b| b.clone()).unwrap_or_default(),
            };
            last_raw = Some(raw_text.clone());

            // Try parse + validate.
            match extract_and_parse::<OutlineProposal>(&raw_text) {
                Ok(mut proposal) => {
                    // Auto-rescale per-scene `target_word_count` to fit the
                    // brief budget. Local 9B models routinely propose
                    // outlines whose totals are 1.5×–3× the brief — the
                    // structural shape is fine, only the arithmetic needs
                    // fixing. See `OutlineProposal::rescale_to_brief_target`.
                    let pre_total = proposal.total_target_words();
                    proposal.rescale_to_brief_target(brief.target_word_count);
                    let post_total = proposal.total_target_words();
                    if pre_total != post_total {
                        tracing::info!(
                            agent = "outline-architect",
                            pre_total,
                            post_total,
                            brief_target = brief.target_word_count,
                            "rescaled per-scene target_word_count to fit brief",
                        );
                    }

                    let sem_errors = booksforge_agents::outline_architect::validate_semantic(
                        &proposal,
                        target_chapter_count,
                        brief.target_word_count,
                    );
                    if sem_errors.is_empty() {
                        // ── Success ───────────────────────────────────────
                        let out_json = serde_json::to_string(&proposal).unwrap_or_default();
                        let out_hash = blake3::hash(out_json.as_bytes()).to_hex().to_string();
                        let validated = Utc::now();

                        self.storage
                            .agent_task_update(
                                task_id,
                                AgentTaskStatus::Completed,
                                Some(&out_hash),
                                None,
                                None,
                                Some(step_ms),
                                attempt,
                                None,
                                None,
                            )
                            .await
                            .ok();

                        self.storage
                            .agent_output_insert(&AgentOutput {
                                task_id,
                                schema_id: "OutlineProposal".to_owned(),
                                schema_version: 1,
                                content_inline: Some(out_json),
                                content_path: None,
                                hash: out_hash,
                                validated_at: validated,
                            })
                            .await
                            .ok();

                        return Ok(OutlineRunResult {
                            run_id: run_id.to_string(),
                            task_id: task_id.to_string(),
                            status: "completed".to_owned(),
                            proposal: Some(proposal),
                            error: None,
                            raw_output: last_raw,
                        });
                    }
                    last_error = Some(format!(
                        "semantic validation failed: {}",
                        sem_errors.join("; ")
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
        self.storage
            .agent_task_update(
                task_id,
                AgentTaskStatus::Invalid,
                None,
                None,
                None,
                Some(wall_start.elapsed().as_millis() as u64),
                attempt.saturating_sub(1),
                Some("schema_or_semantic"),
                last_error.as_deref(),
            )
            .await
            .ok();

        Ok(OutlineRunResult {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            status: "invalid".to_owned(),
            proposal: None,
            error: last_error,
            raw_output: last_raw,
        })
    }
}

fn peer_focus_str(f: booksforge_domain::PeerReviewFocus) -> &'static str {
    use booksforge_domain::PeerReviewFocus::*;
    match f {
        FactFidelity => "fact_fidelity",
        VoicePreservation => "voice_preservation",
        AiTellResidue => "ai_tell_residue",
        NamePovPreservation => "name_pov_preservation",
        StructuralPurpose => "structural_purpose",
        MemoryConsistency => "memory_consistency",
        EmotionalClarity => "emotional_clarity",
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
