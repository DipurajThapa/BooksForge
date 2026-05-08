//! Tauri commands for the MZ-05/MZ-07 agent layer.
//!
//! - `agent_run_outline`     — runs the outline-architect, persists the
//!                             proposal as an `agent_outputs` row.
//! - `agent_apply_outline`   — accepts a previously-stored proposal and
//!                             materialises it as a document tree, taking
//!                             the mandatory `pre_agent_edit` snapshot first.

use std::sync::Arc;

use booksforge_domain::{
    pm_doc_to_text, PeerReviewConcern, PeerReviewResult, ProjectBrief, ProposalValidation,
    ValidationAxis, ValidationCheck, ValidationOutcome, ValidationVerdict, VerificationReport,
};
use booksforge_fs::{BundleFilesystem, OsFilesystem};
use booksforge_ipc::{
    AgentCancelInput, AgentRunCompletedEvent, AgentRunProgressEvent, AgentRunResultDto, AgentRunStartedEvent,
    ApplyContinuityInput, ApplyContinuityResultDto, ApplyCopyeditInput, ApplyCopyeditResult,
    ApplyHumanizationInput, ApplyOutlineInput, ApplyOutlineResult, BooksForgeError,
    ContinuityScenePassDto, EntityBibleApplyInput, EntityBibleApplyResult, OriginalityScanInput,
    OriginalityScanResult, OutlineRunResult, OverlapHitDto, PeerReviewConcernDto,
    PeerReviewResultDto, ProposalValidationDto, RunChapterDrafterInput, RunContinuityInput,
    RunCopyeditInput, RunDevEditorInput, RunDevelopmentalReviewInput, RunDevelopmentalReviewResult,
    RunHumanizationInput, RunIntakeAndOutlineInput, RunIntakeAndOutlineResult, RunIntakeInput,
    RunMemoryCuratorInput, RunOutlineInput, RunProposalValidatorInput, RunVocabDictionaryInput,
    ValidationCheckDto, VerificationReportDto, VocabApplyInput, VocabApplyResult,
};
use booksforge_ollama::{types::CancelToken, HttpOllamaClient};
use booksforge_orchestrator::{Orchestrator, OrchestratorConfig};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::StorageRepository;
use tauri::{AppHandle, Emitter as _, State};
use ulid::Ulid;

use crate::state::AppState;

// ── DTO helpers ──────────────────────────────────────────────────────────────

fn axis_str(a: ValidationAxis) -> &'static str {
    match a {
        ValidationAxis::Schema           => "schema",
        ValidationAxis::Contract         => "contract",
        ValidationAxis::Range            => "range",
        ValidationAxis::Redaction        => "redaction",
        ValidationAxis::Length           => "length",
        ValidationAxis::EntitySanity     => "entity_sanity",
        ValidationAxis::MemoryScope      => "memory_scope",
        ValidationAxis::Idempotent       => "idempotent",
        ValidationAxis::Originality      => "originality",
        ValidationAxis::Faithfulness     => "faithfulness",
        ValidationAxis::Style            => "style",
        ValidationAxis::Coherence        => "coherence",
        ValidationAxis::SelfConsistency  => "self_consistency",
    }
}

fn outcome_str(o: ValidationOutcome) -> &'static str {
    match o {
        ValidationOutcome::Pass => "pass",
        ValidationOutcome::Warn => "warn",
        ValidationOutcome::Fail => "fail",
    }
}

fn verdict_str(v: ValidationVerdict) -> &'static str {
    match v {
        ValidationVerdict::Pass  => "pass",
        ValidationVerdict::Warn  => "warn",
        ValidationVerdict::Block => "block",
    }
}

fn check_to_dto(c: ValidationCheck) -> ValidationCheckDto {
    ValidationCheckDto {
        axis:        axis_str(c.axis).to_owned(),
        outcome:     outcome_str(c.outcome).to_owned(),
        evidence:    c.evidence,
        remediation: c.remediation,
    }
}

fn validation_to_dto(v: ProposalValidation) -> ProposalValidationDto {
    ProposalValidationDto {
        verdict:    verdict_str(v.verdict).to_owned(),
        checks:     v.checks.into_iter().map(check_to_dto).collect(),
        summary:    v.summary,
        tier_2_ran: v.tier_2_ran,
    }
}

fn concern_to_dto(c: PeerReviewConcern) -> PeerReviewConcernDto {
    use booksforge_domain::PeerConcernSeverity;
    let sev = match c.severity {
        PeerConcernSeverity::Info    => "info",
        PeerConcernSeverity::Warning => "warning",
        PeerConcernSeverity::Error   => "error",
    };
    PeerReviewConcernDto {
        severity: sev.to_owned(), quote: c.quote, reason: c.reason, evidence: c.evidence,
    }
}

fn peer_to_dto(p: PeerReviewResult) -> PeerReviewResultDto {
    use booksforge_domain::PeerReviewFocus;
    let focus = match p.focus {
        PeerReviewFocus::FactFidelity        => "fact_fidelity",
        PeerReviewFocus::VoicePreservation   => "voice_preservation",
        PeerReviewFocus::AiTellResidue       => "ai_tell_residue",
        PeerReviewFocus::NamePovPreservation => "name_pov_preservation",
        PeerReviewFocus::StructuralPurpose   => "structural_purpose",
        PeerReviewFocus::MemoryConsistency   => "memory_consistency",
        PeerReviewFocus::EmotionalClarity    => "emotional_clarity",
    };
    PeerReviewResultDto {
        reviewer_agent_id: p.reviewer_agent_id,
        primary_task_id:   p.primary_task_id,
        focus:             focus.to_owned(),
        verdict:           verdict_str(p.verdict).to_owned(),
        concerns:          p.concerns.into_iter().map(concern_to_dto).collect(),
        recommendation:    p.recommendation,
    }
}

fn report_to_dto(r: VerificationReport) -> VerificationReportDto {
    VerificationReportDto {
        primary_agent_id: r.primary_agent_id,
        primary_task_id:  r.primary_task_id,
        tier_1:           validation_to_dto(r.tier_1),
        tier_2:           r.tier_2.map(validation_to_dto),
        peer_reviews:     r.peer_reviews.into_iter().map(peer_to_dto).collect(),
        final_verdict:    verdict_str(r.final_verdict).to_owned(),
    }
}

// `pm_doc_to_text` moved to `booksforge_domain::pm_doc` (re-imported above).

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build an orchestrator with snapshot service attached against the
/// currently-open project.  Reused by both run and apply commands so the
/// MZ-06 invariant ("every applied edit follows a `pre_agent_edit` snapshot")
/// holds end-to-end.
async fn open_orchestrator(
    state: &State<'_, AppState>,
) -> Result<Orchestrator, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let storage_arc = Arc::clone(&project.storage);
    let storage_trait: Arc<dyn StorageRepository> = storage_arc.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    let snapshot = Arc::new(SnapshotService::new(
        storage_trait,
        fs,
        project.bundle.clone(),
    ));

    let ollama: Arc<dyn booksforge_ollama::client::OllamaClient> =
        Arc::new(HttpOllamaClient::new());

    Ok(Orchestrator::new(ollama, storage_arc, OrchestratorConfig::default())
        .with_snapshot(snapshot))
}

// ── agent_run_outline ─────────────────────────────────────────────────────────

/// Run the outline-architect agent against the currently open project.
#[tauri::command]
pub async fn agent_run_outline(
    input: RunOutlineInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<OutlineRunResult, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "outline-architect").await;
    let res: Result<OutlineRunResult, BooksForgeError> = async {
        let brief: ProjectBrief = serde_json::from_str(&input.brief_json)
            .map_err(|e| BooksForgeError::validation(format!("invalid brief JSON: {e}")))?;
        brief
            .validate()
            .map_err(|e| BooksForgeError::validation(e.to_string()))?;

        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id ULID".to_owned()))?;

        let orchestrator = open_orchestrator(&state).await?;

        let result = orchestrator
            .run_outline(
                project_id,
                &brief,
                input.target_chapter_count,
                input.genre_overlay.as_deref(),
                &input.model,
                cancel.clone(),
            )
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        Ok(OutlineRunResult {
            run_id:       result.run_id,
            task_id:      result.task_id,
            status:       result.status,
            proposal_json: result.proposal.as_ref()
                .and_then(|p| serde_json::to_string(p).ok()),
            error:        result.error,
            raw_output:   result.raw_output,
        })
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "outline-architect", run_id, &cancel, err).await;
    res
}

// ── agent_apply_outline ───────────────────────────────────────────────────────

/// Accept a previously-stored outline proposal and create the document tree.
///
/// MZ-07 flow:
///   1. Refuse if the proposal has already been applied (`AlreadyApplied`).
///   2. Take a `pre_agent_edit` snapshot.
///   3. Build the `NodeTreeDelta` and insert all nodes atomically.
///   4. Record one `agent_applied_edits` row per node.
#[tauri::command]
pub async fn agent_apply_outline(
    input: ApplyOutlineInput,
    state: State<'_, AppState>,
) -> Result<ApplyOutlineResult, BooksForgeError> {
    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id ULID".to_owned()))?;
    let task_id = Ulid::from_string(&input.task_id)
        .map_err(|_| BooksForgeError::validation("invalid task_id ULID".to_owned()))?;

    let orchestrator = open_orchestrator(&state).await?;

    let result = orchestrator
        .apply_outline(project_id, task_id, &input.project_title)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ApplyOutlineResult {
        task_id:            result.task_id,
        pre_snapshot_id:    result.pre_snapshot_id,
        project_root_id:    result.project_root_id,
        created_node_count: result.created_node_count,
        applied_edit_count: result.applied_edit_count,
    })
}

// ── agent_apply_copyedit ──────────────────────────────────────────────────────

/// Accept one entry from a previously-stored `CopyeditProposals` and apply
/// it to the live scene.  See BACKLOG §E0d.5 for the full flow.
#[tauri::command]
pub async fn agent_apply_copyedit(
    input: ApplyCopyeditInput,
    state: State<'_, AppState>,
) -> Result<ApplyCopyeditResult, BooksForgeError> {
    let task_id = Ulid::from_string(&input.task_id)
        .map_err(|_| BooksForgeError::validation("invalid task_id ULID".to_owned()))?;
    let scene_id = Ulid::from_string(&input.scene_id)
        .map_err(|_| BooksForgeError::validation("invalid scene_id ULID".to_owned()))?;

    let orchestrator = open_orchestrator(&state).await?;
    let r = orchestrator
        .apply_copyedit_edit(task_id, scene_id, input.edit_index)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ApplyCopyeditResult {
        task_id:              r.task_id,
        edit_index:           r.edit_index,
        scene_id:             r.scene_id,
        pre_snapshot_id:      r.pre_snapshot_id,
        applied_edit_id:      r.applied_edit_id,
        used_fallback_search: r.used_fallback_search,
    })
}

// ── agent_apply_continuity ────────────────────────────────────────────────────

/// Accept one finding from a stored `ContinuityReport` and apply its
/// `proposed_fix` (rename across scope, or annotate via memory upsert).
/// See BACKLOG §E0d.7 for the full flow.
#[tauri::command]
pub async fn agent_apply_continuity(
    input: ApplyContinuityInput,
    state: State<'_, AppState>,
) -> Result<ApplyContinuityResultDto, BooksForgeError> {
    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id ULID".to_owned()))?;
    let task_id = Ulid::from_string(&input.task_id)
        .map_err(|_| BooksForgeError::validation("invalid task_id ULID".to_owned()))?;

    let orchestrator = open_orchestrator(&state).await?;
    let r = orchestrator
        .apply_continuity_fix(project_id, task_id, input.finding_index)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ApplyContinuityResultDto {
        task_id:          r.task_id,
        finding_index:    r.finding_index,
        kind:             r.kind,
        pre_snapshot_id:  r.pre_snapshot_id,
        applied_edit_ids: r.applied_edit_ids,
        scenes_touched:   r.scenes_touched,
        from_term:        r.from_term,
        to_term:          r.to_term,
    })
}

// ── agent_apply_humanization ──────────────────────────────────────────────────

/// Accept one entry from a stored `HumanizationProposals` (BACKLOG §E0d.6).
/// Returns the same `ApplyCopyeditResult` shape — the apply ledger
/// reuses `AppliedEditKind::TextReplace` for both, distinguished in
/// `edit_payload_json.agent`.
#[tauri::command]
pub async fn agent_apply_humanization(
    input: ApplyHumanizationInput,
    state: State<'_, AppState>,
) -> Result<ApplyCopyeditResult, BooksForgeError> {
    let task_id = Ulid::from_string(&input.task_id)
        .map_err(|_| BooksForgeError::validation("invalid task_id ULID".to_owned()))?;
    let scene_id = Ulid::from_string(&input.scene_id)
        .map_err(|_| BooksForgeError::validation("invalid scene_id ULID".to_owned()))?;

    let orchestrator = open_orchestrator(&state).await?;
    let r = orchestrator
        .apply_humanization_edit(task_id, scene_id, input.edit_index)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ApplyCopyeditResult {
        task_id:              r.task_id,
        edit_index:           r.edit_index,
        scene_id:             r.scene_id,
        pre_snapshot_id:      r.pre_snapshot_id,
        applied_edit_id:      r.applied_edit_id,
        used_fallback_search: r.used_fallback_search,
    })
}

// ── agent_run_copyedit ────────────────────────────────────────────────────────

/// Run the Copyeditor agent against a single scene.  Returns a typed
/// `CopyeditProposals` (as JSON for portability) plus the full
/// VerificationReport (Tier-1 cross-cutting + final verdict).
#[tauri::command]
pub async fn agent_run_copyedit(
    input: RunCopyeditInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "copyeditor").await;
    let result = run_copyedit_inner(&state, input, cancel.clone()).await;
    let err = result.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "copyeditor", run_id, &cancel, err).await;
    result
}

async fn run_copyedit_inner(
    state:  &State<'_, AppState>,
    input:  RunCopyeditInput,
    cancel: CancelToken,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
    let node_id = Ulid::from_string(&input.node_id)
        .map_err(|_| BooksForgeError::validation("invalid node_id".to_owned()))?;
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let scene = project.storage.load_scene(node_id).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
        .ok_or_else(|| BooksForgeError::validation("scene not found".to_owned()))?;
    let scene_text = pm_doc_to_text(&scene.pm_doc);

    let nodes = project.storage.list_nodes().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let title = nodes.iter().find(|n| n.id == node_id)
        .map(|n| n.title.clone())
        .unwrap_or_else(|| "Untitled scene".to_owned());

    let style_book = project.storage.load_style_book().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let style_book_json = serde_json::to_value(&style_book)
        .unwrap_or_else(|_| serde_json::json!({}));

    let context = load_run_context(&project).await?;
    let context_for_council = context.clone();
    let scene_text_for_council = scene_text.clone();
    let high_conf = input.high_confidence_mode;

    let orchestrator = open_orchestrator(state).await?;
    let mut result = orchestrator.run_copyedit_scene(
        project_id, scene_text, title, style_book_json, context,
        input.model.clone(), cancel.clone(),
    ).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    if let Some(out) = result.output.as_ref() {
        let primary_output_json = serde_json::to_value(out).unwrap_or(serde_json::json!({}));
        let task_id = result.task_id.to_string();
        let peers = orchestrator.dispatch_peer_reviews(
            project_id, "copyeditor", task_id, primary_output_json,
            scene_text_for_council.clone(), &context_for_council, high_conf,
            input.model.clone(), cancel.clone(),
        ).await;
        orchestrator.fold_peer_reviews_into_result("copyeditor", &mut result, peers);
    }
    let _ = orchestrator.maybe_dispatch_tier2(
        project_id, "copyeditor", &mut result, &context_for_council,
        scene_text_for_council, input.model, cancel,
    ).await;

    let proposal_json = result.output.as_ref()
        .and_then(|p| serde_json::to_string(p).ok());
    let status = match result.status {
        booksforge_domain::AgentTaskStatus::Completed => "completed",
        booksforge_domain::AgentTaskStatus::Cancelled => "cancelled",
        booksforge_domain::AgentTaskStatus::Error     => "error",
        _ => "invalid",
    };
    Ok(AgentRunResultDto {
        run_id:        result.run_id.to_string(),
        task_id:       result.task_id.to_string(),
        status:        status.to_owned(),
        agent_id:      "copyeditor".to_owned(),
        proposal_json,
        verification:  report_to_dto(result.verification),
        error:         result.error,
        raw_output:    result.raw_output,
    })
}

// ── agent_run_continuity ──────────────────────────────────────────────────────

/// Run the Continuity workflow against a single scene.  Runs the
/// deterministic linter first; only ambiguous findings go to the LLM
/// adjudicator.  High-confidence findings bypass the LLM entirely.
#[tauri::command]
pub async fn agent_run_continuity(
    input: RunContinuityInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "continuity").await;
    let result = run_continuity_inner(&state, input, cancel.clone()).await;
    let err = result.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "continuity", run_id, &cancel, err).await;
    result
}

async fn run_continuity_inner(
    state:  &State<'_, AppState>,
    input:  RunContinuityInput,
    cancel: CancelToken,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
    let node_id = Ulid::from_string(&input.node_id)
        .map_err(|_| BooksForgeError::validation("invalid node_id".to_owned()))?;
    let _ = input.high_confidence_mode;

    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let scene = project.storage.load_scene(node_id).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
        .ok_or_else(|| BooksForgeError::validation("scene not found".to_owned()))?;
    let scene_text = pm_doc_to_text(&scene.pm_doc);
    let context = load_run_context(&project).await?;

    let linter_findings = booksforge_validator::lint_scene(
        &node_id.to_string(),
        &scene_text,
        input.project_pov.as_deref(),
        &context.entity_bible,
    );
    let ambiguous: Vec<_> = linter_findings.iter().filter(|f| f.ambiguous).collect();

    let scene_excerpts = serde_json::json!(
        ambiguous.iter().map(|f| serde_json::json!({
            "node_id":     f.evidence[0].node_id,
            "range_from":  f.evidence[0].range_from,
            "range_to":    f.evidence[0].range_to,
            "excerpt":     f.evidence[0].excerpt,
        })).collect::<Vec<_>>()
    );
    let known_entities = serde_json::json!(
        context.entity_bible.iter().map(|e| serde_json::json!({
            "name":    e.name,
            "kind":    serde_json::to_value(e.kind).unwrap_or_else(|_| serde_json::json!("custom")),
            "aliases": e.aliases,
        })).collect::<Vec<_>>()
    );
    let ambiguous_json = serde_json::to_value(&ambiguous)
        .unwrap_or_else(|_| serde_json::json!([]));

    let orchestrator = open_orchestrator(state).await?;
    let result = orchestrator.run_continuity_adjudication(
        project_id, ambiguous_json, known_entities, scene_excerpts,
        input.project_pov, input.prior_summary, context,
        input.model, cancel,
    ).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let proposal_json = result.output.as_ref()
        .and_then(|p| serde_json::to_string(p).ok());
    let status = match result.status {
        booksforge_domain::AgentTaskStatus::Completed => "completed",
        booksforge_domain::AgentTaskStatus::Cancelled => "cancelled",
        booksforge_domain::AgentTaskStatus::Error     => "error",
        _ => "invalid",
    };
    Ok(AgentRunResultDto {
        run_id:        result.run_id.to_string(),
        task_id:       result.task_id.to_string(),
        status:        status.to_owned(),
        agent_id:      "continuity".to_owned(),
        proposal_json,
        verification:  report_to_dto(result.verification),
        error:         result.error,
        raw_output:    result.raw_output,
    })
}

// ── shared helpers for the remaining 7 commands ──────────────────────────────

fn status_str(s: booksforge_domain::AgentTaskStatus) -> &'static str {
    match s {
        booksforge_domain::AgentTaskStatus::Completed => "completed",
        booksforge_domain::AgentTaskStatus::Cancelled => "cancelled",
        booksforge_domain::AgentTaskStatus::Error     => "error",
        _ => "invalid",
    }
}

fn run_result_to_dto<T: serde::Serialize>(
    result:   booksforge_orchestrator::runner::AgentRunResult<T>,
    agent_id: &str,
) -> AgentRunResultDto {
    let proposal_json = result.output.as_ref().and_then(|p| serde_json::to_string(p).ok());
    AgentRunResultDto {
        run_id:       result.run_id.to_string(),
        task_id:      result.task_id.to_string(),
        status:       status_str(result.status).to_owned(),
        agent_id:     agent_id.to_owned(),
        proposal_json,
        verification: report_to_dto(result.verification),
        error:        result.error,
        raw_output:   result.raw_output,
    }
}

async fn require_open_project(
    state: &State<'_, AppState>,
) -> Result<Arc<crate::state::OpenProject>, BooksForgeError> {
    let guard = state.open_project.lock().await;
    guard.as_ref().cloned()
        .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))
}

/// Live-run dispatch begin (BACKLOG §E4) — call at the top of every
/// agent command.  Generates a frontend-visible `run_id`, registers a
/// fresh `CancelToken` in the app's `jobs` registry, and emits the
/// `agent-run-started` event so the overlay shows up.  Pair with
/// `end_agent_run` so the registry doesn't leak.
async fn begin_agent_run(
    state:    &State<'_, AppState>,
    app:      &AppHandle,
    agent_id: &str,
) -> (String, CancelToken) {
    let run_id = Ulid::new().to_string();
    let cancel = state.register_job(&run_id).await;
    let _ = app.emit("agent-run-started", AgentRunStartedEvent {
        run_id:     run_id.clone(),
        agent_id:   agent_id.to_owned(),
        started_at: chrono::Utc::now().to_rfc3339(),
    });
    (run_id, cancel)
}

/// Live-run dispatch end — call after the body resolves, regardless
/// of outcome.  Drops the registry entry and emits
/// `agent-run-completed` with the right status (cancelled / error /
/// completed) so the overlay clears.
async fn end_agent_run(
    state:    &State<'_, AppState>,
    app:      &AppHandle,
    agent_id: &str,
    run_id:   String,
    cancel:   &CancelToken,
    error:    Option<String>,
) {
    state.drop_job(&run_id).await;
    let status: &str = if cancel.is_cancelled() {
        "cancelled"
    } else if error.is_some() {
        "error"
    } else {
        "completed"
    };
    let _ = app.emit("agent-run-completed", AgentRunCompletedEvent {
        run_id,
        agent_id: agent_id.to_owned(),
        status:   status.to_owned(),
        error,
        finished_at: chrono::Utc::now().to_rfc3339(),
    });
}

/// Token-streaming progress helper (BACKLOG §E4 follow-up).  Returns
/// an `(on_token, stop)` pair the caller can hand to the runner:
///
///   - `on_token`: the closure to plug into `RunInput.on_token`.
///     Increments an atomic counter — cheap, lock-free.
///   - `stop`: a callback that ends the periodic emitter task.  Call
///     it after the run resolves so we don't leak the timer.
///
/// While the run is alive a tokio task wakes every 250 ms, reads the
/// counter, and emits an `agent-run-progress` event with cumulative
/// tokens + elapsed ms.  Frontend converts to tokens/sec.
fn start_token_progress_emitter(
    app:     AppHandle,
    run_id:  String,
) -> (
    std::sync::Arc<dyn Fn(&str) + Send + Sync>,
    Box<dyn FnOnce() + Send>,
) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let counter = Arc::new(AtomicU64::new(0));
    let counter_for_sink = counter.clone();
    let on_token: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |_t: &str| {
        counter_for_sink.fetch_add(1, Ordering::Relaxed);
    });

    let stop_flag = Arc::new(AtomicU64::new(0));
    let stop_flag_for_task = stop_flag.clone();
    let counter_for_task   = counter;
    let app_for_task       = app;
    let run_id_for_task    = run_id;
    let started            = Instant::now();

    // Periodic emitter — 4 Hz.  Runs until `stop` flips the flag.
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        // Skip the immediate first tick; the run hasn't produced
        // anything yet.
        interval.tick().await;
        loop {
            interval.tick().await;
            if stop_flag_for_task.load(Ordering::Relaxed) > 0 { break; }
            let tokens = counter_for_task.load(Ordering::Relaxed) as u32;
            let elapsed_ms = started.elapsed().as_millis().min(u32::MAX as u128) as u32;
            let _ = app_for_task.emit("agent-run-progress", AgentRunProgressEvent {
                run_id:     run_id_for_task.clone(),
                tokens,
                elapsed_ms,
            });
        }
    });

    let stop: Box<dyn FnOnce() + Send> = Box::new(move || {
        stop_flag.store(1, Ordering::Relaxed);
    });
    (on_token, stop)
}

/// Cancel an in-flight agent run by its frontend `run_id`.  Idempotent
/// — unknown ids are silent no-ops, so the overlay can fire it from a
/// closing dialog without checking whether the run is still alive.
#[tauri::command]
pub async fn agent_cancel(
    input: AgentCancelInput,
    state: State<'_, AppState>,
) -> Result<(), BooksForgeError> {
    state.cancel_job(&input.run_id).await;
    Ok(())
}

/// Load the cross-cutting `RunContext` (entity bible + active vocab
/// avoid-rules + voice fingerprint) for the currently-open project.
/// One round-trip per agent dispatch — the context fits comfortably in
/// memory (entity bible is bounded; vocab is filtered by layer).
async fn load_run_context(
    project: &Arc<crate::state::OpenProject>,
) -> Result<booksforge_orchestrator::runner::RunContext, BooksForgeError> {
    let entity_bible = project.storage.list_entities().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let layers = ["project", "ai_tells"];
    let active_avoid_rules = project.storage.vocab_list_by_layers(&layers).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let voice_fingerprint =
        booksforge_orchestrator::voice_pipeline::load_or_default(&project.storage).await;
    Ok(booksforge_orchestrator::runner::RunContext {
        entity_bible, active_avoid_rules, voice_fingerprint,
    })
}

// ── agent_run_intake ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_run_intake(
    input: RunIntakeInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "intake").await;
    let r: Result<AgentRunResultDto, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let orchestrator = open_orchestrator(&state).await?;
        let result = orchestrator.run_intake(
            project_id, input.idea_text, input.preferred_mode, input.model, cancel.clone(),
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        Ok(run_result_to_dto(result, "intake"))
    }.await;
    let err = r.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "intake", run_id, &cancel, err).await;
    r
}

// ── agent_run_intake_and_outline ─────────────────────────────────────────────

/// Chained intake → outline-architect workflow (BACKLOG §E1).  One
/// dispatch, two agent calls, structured result that includes both
/// halves so the UI can show the brief above the outline.
#[tauri::command]
pub async fn agent_run_intake_and_outline(
    input: RunIntakeAndOutlineInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<RunIntakeAndOutlineResult, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "intake-and-outline").await;
    let res: Result<RunIntakeAndOutlineResult, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let orchestrator = open_orchestrator(&state).await?;
        let r = orchestrator.run_intake_and_outline(
            project_id,
            input.idea_text,
            input.preferred_mode,
            input.target_chapter_count,
            input.genre_overlay,
            input.model,
            cancel.clone(),
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        Ok(RunIntakeAndOutlineResult {
            intake_run_id:    r.intake_run_id,
            intake_task_id:   r.intake_task_id,
            brief_json:       r.brief.as_ref().and_then(|b| serde_json::to_string(b).ok()),
            intake_error:     r.intake_error,
            intake_raw:       r.intake_raw,
            outline_run_id:   r.outline_run_id,
            outline_task_id:  r.outline_task_id,
            outline_status:   r.outline_status,
            outline_json:     r.outline.as_ref().and_then(|o| serde_json::to_string(o).ok()),
            outline_error:    r.outline_error,
            outline_raw:      r.outline_raw,
        })
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "intake-and-outline", run_id, &cancel, err).await;
    res
}

// ── agent_run_memory_curator ─────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_run_memory_curator(
    input: RunMemoryCuratorInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "memory-curator").await;
    let res: Result<AgentRunResultDto, BooksForgeError> = async {
        use booksforge_domain::MemoryScope;
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let scope_enum = match input.scope.as_str() {
            "book"    => MemoryScope::Book,
            "chapter" => MemoryScope::Chapter,
            "entity"  => MemoryScope::Entity,
            s => return Err(BooksForgeError::validation(format!("invalid memory scope: {s}"))),
        };
        let project = require_open_project(&state).await?;

        let chapter_text = if let Some(ref nid) = input.node_id {
            let node_id = Ulid::from_string(nid)
                .map_err(|_| BooksForgeError::validation("invalid node_id".to_owned()))?;
            match project.storage.load_scene(node_id).await
                .map_err(|e| BooksForgeError::internal(e.to_string()))?
            {
                Some(scene) => pm_doc_to_text(&scene.pm_doc),
                None        => String::new(),
            }
        } else {
            String::new()
        };

        let current_memory = project.storage.memory_list_by_scope(scope_enum).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let current_memory_json = serde_json::to_value(&current_memory)
            .unwrap_or_else(|_| serde_json::json!([]));

        let context = load_run_context(&project).await?;
        let existing_entities_json = serde_json::to_value(&context.entity_bible)
            .unwrap_or_else(|_| serde_json::json!([]));

        let orchestrator = open_orchestrator(&state).await?;
        let scope_label = input.scope.clone();
        let result = orchestrator.run_memory_curator(
            project_id, input.scope, input.node_id, chapter_text,
            current_memory_json, existing_entities_json, context,
            input.model, cancel.clone(),
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        // BACKLOG §E0d.8 — chapter-finalise voice-fingerprint refresh.
        if scope_label == "chapter"
            && matches!(result.status, booksforge_domain::AgentTaskStatus::Completed)
        {
            match booksforge_orchestrator::voice_pipeline::refresh_from_corpus(
                &project.storage, "memory-curator",
            ).await {
                Ok(fp) => tracing::info!(
                    project_id = %project_id,
                    corpus_tokens = fp.corpus_tokens,
                    "voice fingerprint refreshed after chapter checkpoint"
                ),
                Err(e) => tracing::warn!(
                    project_id = %project_id, error = %e,
                    "voice fingerprint refresh failed — keeping previous fingerprint"
                ),
            }
        }

        Ok(run_result_to_dto(result, "memory-curator"))
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "memory-curator", run_id, &cancel, err).await;
    res
}

// ── agent_run_vocab_dictionary ───────────────────────────────────────────────

#[tauri::command]
pub async fn agent_run_vocab_dictionary(
    input: RunVocabDictionaryInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "vocab-dictionary").await;
    let res: Result<AgentRunResultDto, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let project = require_open_project(&state).await?;

        let layers = ["project"];
        let current_vocab = project.storage.vocab_list_by_layers(&layers).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let current_vocab_json = serde_json::to_value(&current_vocab)
            .unwrap_or_else(|_| serde_json::json!([]));

        // BACKLOG §E0d.4 — feed real edit history from the apply ledger.
        let limit = input.lookback.unwrap_or(200).min(1_000);
        let recent = project.storage.recent_applied_edits_for_project(
            project_id, booksforge_domain::AppliedEditKind::TextReplace, limit,
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        let mut accepted = Vec::with_capacity(recent.len());
        let mut rejected = Vec::with_capacity(recent.len() / 4);
        for row in &recent {
            let payload: serde_json::Value = serde_json::from_str(&row.edit_payload_json)
                .unwrap_or(serde_json::Value::Null);
            let entry = serde_json::json!({
                "before":         payload.get("before").cloned().unwrap_or(serde_json::Value::Null),
                "after":          payload.get("after").cloned().unwrap_or(serde_json::Value::Null),
                "category":       payload.get("category").cloned().unwrap_or(serde_json::Value::Null),
                "rationale":      payload.get("rationale").cloned().unwrap_or(serde_json::Value::Null),
                "triggered_rule": payload.get("triggered_rule").cloned().unwrap_or(serde_json::Value::Null),
                "agent":          payload.get("agent").cloned().unwrap_or(serde_json::Value::Null),
                "applied_at":     row.applied_at,
            });
            if row.reverted_at.is_some() { rejected.push(entry); } else { accepted.push(entry); }
        }
        let accepted_json = serde_json::Value::Array(accepted);
        let rejected_json = serde_json::Value::Array(rejected);

        let orchestrator = open_orchestrator(&state).await?;
        let result = orchestrator.run_vocab_dictionary(
            project_id, accepted_json, rejected_json, current_vocab_json,
            input.model, cancel.clone(),
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        Ok(run_result_to_dto(result, "vocab-dictionary"))
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "vocab-dictionary", run_id, &cancel, err).await;
    res
}

// ── agent_run_chapter_drafter ────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_run_chapter_drafter(
    input: RunChapterDrafterInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "chapter-drafter").await;
    let res: Result<AgentRunResultDto, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let _node_id = Ulid::from_string(&input.node_id)
            .map_err(|_| BooksForgeError::validation("invalid node_id".to_owned()))?;
        let project = require_open_project(&state).await?;

        let context = load_run_context(&project).await?;
        let known_entities_json = serde_json::to_value(&context.entity_bible)
            .unwrap_or_else(|_| serde_json::json!([]));
        let voice_fingerprint_json = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));

        let chapter_memory = project.storage
            .memory_list_by_scope(booksforge_domain::MemoryScope::Chapter)
            .await.map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let prior_summary = chapter_memory.iter()
            .filter(|m| m.key.ends_with(":summary"))
            .max_by_key(|m| m.updated_at)
            .map(|m| m.value_json.as_str().unwrap_or("").to_owned())
            .unwrap_or_default();

        let context_for_council = context.clone();
        let synopsis_for_council = input.scene_synopsis.clone();
        let high_conf = input.high_confidence_mode.unwrap_or(false);

        let orchestrator = open_orchestrator(&state).await?;
        let (on_token, stop_emitter) = start_token_progress_emitter(app.clone(), run_id.clone());
        let mut stop_emitter = Some(stop_emitter);
        let stop = |s: &mut Option<Box<dyn FnOnce() + Send>>| {
            if let Some(f) = s.take() { f(); }
        };
        let chapter_drafter_result = orchestrator.run_chapter_drafter(
            project_id, input.scene_synopsis, input.chapter_purpose, input.project_pov,
            input.target_words, known_entities_json, prior_summary, voice_fingerprint_json,
            input.genre, input.tone, context, input.model.clone(), cancel.clone(),
            Some(on_token),
        ).await;
        // Stop the periodic emitter — primary call done, no more tokens.
        stop(&mut stop_emitter);
        let mut result = chapter_drafter_result
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        if let Some(out) = result.output.as_ref() {
            let primary_output_json = serde_json::to_value(out).unwrap_or(serde_json::json!({}));
            let peers = orchestrator.dispatch_peer_reviews(
                project_id, "chapter-drafter", result.task_id.to_string(), primary_output_json,
                synopsis_for_council.clone(), &context_for_council, high_conf,
                input.model.clone(), cancel.clone(),
            ).await;
            orchestrator.fold_peer_reviews_into_result("chapter-drafter", &mut result, peers);
        }
        let _ = orchestrator.maybe_dispatch_tier2(
            project_id, "chapter-drafter", &mut result, &context_for_council,
            synopsis_for_council, input.model, cancel.clone(),
        ).await;
        Ok(run_result_to_dto(result, "chapter-drafter"))
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "chapter-drafter", run_id, &cancel, err).await;
    res
}

// ── agent_run_dev_editor ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_run_dev_editor(
    input: RunDevEditorInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "dev-editor").await;
    let res: Result<AgentRunResultDto, BooksForgeError> = async {
    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
    let chapter_id_ulid = Ulid::from_string(&input.chapter_id)
        .map_err(|_| BooksForgeError::validation("invalid chapter_id".to_owned()))?;
    let project = require_open_project(&state).await?;

    // Concatenate every scene under this chapter into the chapter_text input.
    let nodes = project.storage.list_nodes().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let mut chapter_text = String::new();
    for n in &nodes {
        if n.parent_id == Some(chapter_id_ulid)
            && matches!(n.kind, booksforge_domain::NodeKind::Scene)
        {
            if let Some(sc) = project.storage.load_scene(n.id).await
                .map_err(|e| BooksForgeError::internal(e.to_string()))?
            {
                chapter_text.push_str(&pm_doc_to_text(&sc.pm_doc));
                chapter_text.push_str("\n\n");
            }
        }
    }

    // Project brief — fetched from book-scope memory if available; else empty.
    let book_memory = project.storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
        .await.map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let project_brief = book_memory.iter()
        .find(|m| m.key == "brief")
        .map(|m| m.value_json.clone())
        .unwrap_or_else(|| serde_json::json!({}));

    // Prior chapter summaries from chapter-scope memory.
    let chapter_memory = project.storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Chapter)
        .await.map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let prior_summaries = serde_json::json!(
        chapter_memory.iter()
            .filter(|m| m.key.ends_with(":summary"))
            .map(|m| serde_json::json!({
                "chapter_id": m.key.trim_end_matches(":summary"),
                "summary":    m.value_json,
            }))
            .collect::<Vec<_>>()
    );

    let context = load_run_context(&project).await?;
    let known_entities_json = serde_json::to_value(&context.entity_bible)
        .unwrap_or_else(|_| serde_json::json!([]));
    let context_for_council = context.clone();
    let chapter_text_for_council = chapter_text.clone();
    let high_conf = input.high_confidence_mode.unwrap_or(false);

    let orchestrator = open_orchestrator(&state).await?;
    let (dev_on_token, dev_stop) = start_token_progress_emitter(app.clone(), run_id.clone());
    let mut dev_stop_box: Option<Box<dyn FnOnce() + Send>> = Some(dev_stop);
    let dev_result = orchestrator.run_dev_editor(
        project_id, input.chapter_id, chapter_text, project_brief,
        prior_summaries, known_entities_json, context,
        input.model.clone(), cancel.clone(),
        Some(dev_on_token),
    ).await;
    if let Some(f) = dev_stop_box.take() { f(); }
    let mut result = dev_result
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    if let Some(out) = result.output.as_ref() {
        let primary_output_json = serde_json::to_value(out).unwrap_or(serde_json::json!({}));
        let peers = orchestrator.dispatch_peer_reviews(
            project_id, "dev-editor", result.task_id.to_string(), primary_output_json,
            chapter_text_for_council.clone(), &context_for_council, high_conf,
            input.model.clone(), cancel.clone(),
        ).await;
        orchestrator.fold_peer_reviews_into_result("dev-editor", &mut result, peers);
    }
    let _ = orchestrator.maybe_dispatch_tier2(
        project_id, "dev-editor", &mut result, &context_for_council,
        chapter_text_for_council, input.model, cancel.clone(),
    ).await;
    Ok(run_result_to_dto(result, "dev-editor"))
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "dev-editor", run_id, &cancel, err).await;
    res
}

// ── agent_run_humanization ───────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_run_humanization(
    input: RunHumanizationInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "humanization").await;
    let res: Result<AgentRunResultDto, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let node_id = Ulid::from_string(&input.node_id)
            .map_err(|_| BooksForgeError::validation("invalid node_id".to_owned()))?;
        let project = require_open_project(&state).await?;

        let scene = project.storage.load_scene(node_id).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?
            .ok_or_else(|| BooksForgeError::validation("scene not found".to_owned()))?;
        let scene_text = pm_doc_to_text(&scene.pm_doc);

        let nodes = project.storage.list_nodes().await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let title = nodes.iter().find(|n| n.id == node_id)
            .map(|n| n.title.clone())
            .unwrap_or_else(|| "Untitled scene".to_owned());

        let context = load_run_context(&project).await?;
        let active_avoid_rules_json = serde_json::to_value(&context.active_avoid_rules)
            .unwrap_or_else(|_| serde_json::json!([]));
        let voice_fingerprint_json = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));
        let context_for_council = context.clone();
        let scene_text_for_council = scene_text.clone();
        let high_conf = input.high_confidence_mode.unwrap_or(false);

        let orchestrator = open_orchestrator(&state).await?;
        let mut result = orchestrator.run_humanization(
            project_id, scene_text, title, active_avoid_rules_json, voice_fingerprint_json,
            context, input.model.clone(), cancel.clone(),
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        if let Some(out) = result.output.as_ref() {
            let primary_output_json = serde_json::to_value(out).unwrap_or(serde_json::json!({}));
            let peers = orchestrator.dispatch_peer_reviews(
                project_id, "humanization", result.task_id.to_string(), primary_output_json,
                scene_text_for_council.clone(), &context_for_council, high_conf,
                input.model.clone(), cancel.clone(),
            ).await;
            orchestrator.fold_peer_reviews_into_result("humanization", &mut result, peers);
        }
        let _ = orchestrator.maybe_dispatch_tier2(
            project_id, "humanization", &mut result, &context_for_council,
            scene_text_for_council, input.model, cancel.clone(),
        ).await;
        Ok(run_result_to_dto(result, "humanization"))
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "humanization", run_id, &cancel, err).await;
    res
}

// ── agent_run_proposal_validator (Tier-2) ────────────────────────────────────

#[tauri::command]
pub async fn agent_run_proposal_validator(
    input: RunProposalValidatorInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "proposal-validator").await;
    let res: Result<AgentRunResultDto, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let primary_output: serde_json::Value = serde_json::from_str(&input.primary_output_json)
            .map_err(|e| BooksForgeError::validation(format!("primary_output_json invalid: {e}")))?;
        let tier_1_findings: serde_json::Value = serde_json::from_str(&input.tier_1_findings_json)
            .map_err(|e| BooksForgeError::validation(format!("tier_1_findings_json invalid: {e}")))?;

        let project = require_open_project(&state).await?;
        let context = load_run_context(&project).await?;
        let active_avoid_rules_json = serde_json::to_value(&context.active_avoid_rules)
            .unwrap_or_else(|_| serde_json::json!([]));
        let voice_fingerprint_json = serde_json::to_value(&context.voice_fingerprint)
            .unwrap_or_else(|_| serde_json::json!({}));

        let orchestrator = open_orchestrator(&state).await?;
        let result = orchestrator.run_proposal_validator_tier2(
            project_id, input.primary_agent_id, primary_output, input.context_excerpt,
            tier_1_findings, voice_fingerprint_json, active_avoid_rules_json, input.model, cancel.clone(),
        ).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        Ok(run_result_to_dto(result, "proposal-validator"))
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "proposal-validator", run_id, &cancel, err).await;
    res
}

// ── vocab_apply_proposals ────────────────────────────────────────────────────

/// Promote selected entries from a vocab-dictionary run into the project
/// layer (BACKLOG §E0d.10).  Loads the persisted `VocabUpdateProposals`
/// from `agent_outputs`, then applies the user-accepted indices.
///
/// Modifications target the **project** layer specifically — agents
/// can't modify shipped starter dictionaries or `ai_tells` (those are
/// curated upstream).  Modifications whose target term doesn't exist in
/// the project layer are skipped (count surfaced in the result).
#[tauri::command]
pub async fn vocab_apply_proposals(
    input: VocabApplyInput,
    state: State<'_, AppState>,
) -> Result<VocabApplyResult, BooksForgeError> {
    use booksforge_domain::{EntryKind, EntrySource, VocabEntry};

    let task_id = Ulid::from_string(&input.task_id)
        .map_err(|_| BooksForgeError::validation("invalid task_id ULID".to_owned()))?;
    let project = require_open_project(&state).await?;

    // Load the persisted proposals.
    let output = project.storage.agent_output_load(task_id).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
        .ok_or_else(|| BooksForgeError::validation(format!(
            "no agent_outputs row for task {task_id}"
        )))?;
    let raw = output.content_inline.ok_or_else(|| BooksForgeError::validation(
        format!("agent_outputs[{task_id}] has no inline content"),
    ))?;
    let proposals: booksforge_domain::VocabUpdateProposals =
        serde_json::from_str(&raw).map_err(|e| BooksForgeError::internal(format!(
            "could not deserialise stored VocabUpdateProposals: {e}"
        )))?;

    // Snapshot of the current project layer for modification look-up.
    let layers = ["project"];
    let current = project.storage.vocab_list_by_layers(&layers).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let mut additions_applied     = 0u32;
    let mut additions_skipped     = 0u32;
    let mut modifications_applied = 0u32;
    let mut modifications_skipped = 0u32;

    // ── Apply accepted additions ──
    let accepted_adds: std::collections::HashSet<u32> =
        input.accepted_addition_indices.into_iter().collect();
    for (i, a) in proposals.additions.iter().enumerate() {
        if !accepted_adds.contains(&(i as u32)) { continue; }
        let kind = match EntryKind::from_str(&a.kind) {
            Some(k) => k,
            None    => { additions_skipped += 1; continue; }
        };
        let mut entry = VocabEntry::new(
            "project", a.term.clone(), kind, EntrySource::Agent,
        );
        if let Some(r) = &a.replacement { entry = entry.with_replacement(r.clone()); }
        if !a.rationale.trim().is_empty() {
            entry = entry.with_rationale(a.rationale.clone());
        }
        match project.storage.vocab_upsert(&entry).await {
            Ok(_)  => additions_applied += 1,
            Err(e) => {
                tracing::warn!(error = %e, term = a.term, "vocab addition skipped");
                additions_skipped += 1;
            }
        }
    }

    // ── Apply accepted modifications ──
    // We re-upsert the existing entry with the modified field swapped in.
    // Only project-layer entries can be modified by an agent run.
    let accepted_mods: std::collections::HashSet<u32> =
        input.accepted_modification_indices.into_iter().collect();
    for (i, m) in proposals.modifications.iter().enumerate() {
        if !accepted_mods.contains(&(i as u32)) { continue; }
        let term_lower = m.term.to_lowercase();
        let target = current.iter()
            .find(|e| e.term == term_lower && e.layer == "project");
        let target = match target {
            Some(t) => t.clone(),
            None    => { modifications_skipped += 1; continue; }
        };
        let mut updated = target;
        match m.field.as_str() {
            "kind" => {
                let new_kind = m.new_value.as_str()
                    .and_then(EntryKind::from_str);
                match new_kind {
                    Some(k) => updated.kind = k,
                    None    => { modifications_skipped += 1; continue; }
                }
            }
            "replacement" => {
                updated.replacement = m.new_value.as_str().map(|s| s.to_owned());
            }
            "rationale" => {
                updated.rationale = m.new_value.as_str().map(|s| s.to_owned());
            }
            _ => { modifications_skipped += 1; continue; }
        }
        updated.updated_at = chrono::Utc::now();
        match project.storage.vocab_upsert(&updated).await {
            Ok(_)  => modifications_applied += 1,
            Err(e) => {
                tracing::warn!(error = %e, term = m.term, "vocab modification skipped");
                modifications_skipped += 1;
            }
        }
    }

    Ok(VocabApplyResult {
        task_id: task_id.to_string(),
        additions_applied,
        additions_skipped,
        modifications_applied,
        modifications_skipped,
    })
}

// ── originality_scan_chapter ─────────────────────────────────────────────────

/// Run the local plagiarism / verbatim-overlap detector across every
/// scene under `chapter_id`, comparing each scene's text against every
/// other scene's text in the project.  Pure local n-gram match; no
/// network call, nothing leaves the device.  See BACKLOG §E0d.11 for the
/// opt-in online plagiarism API integration roadmap.
#[tauri::command]
pub async fn originality_scan_chapter(
    input: OriginalityScanInput,
    state: State<'_, AppState>,
) -> Result<OriginalityScanResult, BooksForgeError> {
    let chapter_id = Ulid::from_string(&input.chapter_id)
        .map_err(|_| BooksForgeError::validation("invalid chapter_id ULID".to_owned()))?;
    let min_words = input.min_words.unwrap_or(
        booksforge_validator::originality::DEFAULT_MIN_WORDS as u32
    ) as usize;

    let project = require_open_project(&state).await?;
    let nodes = project.storage.list_nodes().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // Scenes in this chapter (the corpus we scan).
    let chapter_scenes: Vec<_> = nodes.iter()
        .filter(|n| n.parent_id == Some(chapter_id)
            && matches!(n.kind, booksforge_domain::NodeKind::Scene))
        .collect();
    // Every scene in the project (the haystack to match against — used
    // to detect cross-chapter self-plagiarism).
    let all_scenes: Vec<_> = nodes.iter()
        .filter(|n| matches!(n.kind, booksforge_domain::NodeKind::Scene))
        .collect();

    let mut hits = Vec::new();
    for scene in &chapter_scenes {
        let scene_obj = match project.storage.load_scene(scene.id).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?
        { Some(s) => s, None => continue };
        let scene_text = pm_doc_to_text(&scene_obj.pm_doc);
        if scene_text.trim().is_empty() { continue; }

        for other in &all_scenes {
            if other.id == scene.id { continue; }
            let other_obj = match project.storage.load_scene(other.id).await
                .map_err(|e| BooksForgeError::internal(e.to_string()))?
            { Some(s) => s, None => continue };
            let other_text = pm_doc_to_text(&other_obj.pm_doc);
            if other_text.trim().is_empty() { continue; }

            let scene_hits = booksforge_validator::detect_self_plagiarism(
                &scene_text, &other_text, min_words,
            );
            for h in scene_hits {
                hits.push(OverlapHitDto {
                    kind:                "prior_scene".to_owned(),
                    scene_id:            scene.id.to_string(),
                    scene_title:         scene.title.clone(),
                    output_from:         h.output_from,
                    output_to:           h.output_to,
                    words:               h.words,
                    quote:               h.quote,
                    matched_scene_id:    other.id.to_string(),
                    matched_scene_title: other.title.clone(),
                });
            }
        }
    }

    Ok(OriginalityScanResult {
        chapter_id:     input.chapter_id,
        scenes_scanned: chapter_scenes.len() as u32,
        min_words:      min_words as u32,
        hits,
    })
}

// ── originality consent — load / save / clear ───────────────────────────────

/// Read the project's persisted originality-provider consent record.
/// Always returns a value — no row → `LocalOnly` default.
#[tauri::command]
pub async fn originality_consent_load(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let c = booksforge_orchestrator::originality_provider::load_consent(
        &project.storage,
    ).await;
    Ok(serde_json::to_value(&c).unwrap_or_else(|_| serde_json::json!({})))
}

/// Persist a new consent record.  In MVP, only the `LocalOnly` provider
/// is wired end-to-end — passing any other id stores the consent (so the
/// UI can show "Copyleaks consent on file") but does not yet send any
/// content off-device because no remote provider is registered.
#[tauri::command]
pub async fn originality_consent_set(
    consent_json: String,
    state:        State<'_, AppState>,
) -> Result<(), BooksForgeError> {
    let project = require_open_project(&state).await?;
    let consent: booksforge_domain::OriginalityConsent =
        serde_json::from_str(&consent_json)
            .map_err(|e| BooksForgeError::validation(format!(
                "invalid OriginalityConsent JSON: {e}"
            )))?;
    booksforge_orchestrator::originality_provider::save_consent(
        &project.storage, &consent, "user",
    ).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(())
}

/// Reset to the default `LocalOnly` consent — equivalent to revoking
/// any opt-in to a remote provider.
#[tauri::command]
pub async fn originality_consent_clear(
    state: State<'_, AppState>,
) -> Result<(), BooksForgeError> {
    let project = require_open_project(&state).await?;
    booksforge_orchestrator::originality_provider::clear_consent(
        &project.storage, "user",
    ).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(())
}

// ── voice_fingerprint_refresh ────────────────────────────────────────────────

/// Recompute the project's voice fingerprint from every accepted scene's
/// text and persist it under `MemoryScope::Style` key `voice_fingerprint`.
///
/// Returns the freshly-computed fingerprint as JSON.  Called manually from
/// the UI (settings → "Recalculate voice fingerprint") and from tests; the
/// auto-refresh hook on Memory Curator chapter-finalise is tracked under
/// BACKLOG §E0d.8.
#[tauri::command]
pub async fn voice_fingerprint_refresh(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let fp = booksforge_orchestrator::voice_pipeline::refresh_from_corpus(
        &project.storage, "manual-refresh",
    ).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(serde_json::to_value(&fp).unwrap_or_else(|_| serde_json::json!({})))
}

/// Load the persisted voice fingerprint, or default if no row exists.
/// Returned as JSON for direct UI consumption.
#[tauri::command]
pub async fn voice_fingerprint_load(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let fp = booksforge_orchestrator::voice_pipeline::load_or_default(
        &project.storage,
    ).await;
    Ok(serde_json::to_value(&fp).unwrap_or_else(|_| serde_json::json!({})))
}

// ── agent_run_developmental_review (BACKLOG §F2) ─────────────────────────────

/// Chained chapter-level review: 1 LLM call (dev_editor) + per-scene
/// deterministic continuity linter.  See
/// `booksforge_orchestrator::run::run_developmental_review`.
#[tauri::command]
pub async fn agent_run_developmental_review(
    input: RunDevelopmentalReviewInput,
    state: State<'_, AppState>,
    app:   AppHandle,
) -> Result<RunDevelopmentalReviewResult, BooksForgeError> {
    let (run_id, cancel) = begin_agent_run(&state, &app, "developmental-review").await;
    let res: Result<RunDevelopmentalReviewResult, BooksForgeError> = async {
        let project_id = Ulid::from_string(&input.project_id)
            .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
        let chapter_id = Ulid::from_string(&input.chapter_id)
            .map_err(|_| BooksForgeError::validation("invalid chapter_id".to_owned()))?;
        let _ = input.high_confidence_mode;
        let project = require_open_project(&state).await?;

        // Concatenate every scene under this chapter for dev_editor.
        let nodes = project.storage.list_nodes().await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let mut chapter_text   = String::new();
        let mut per_scene_text = Vec::new();
        for n in &nodes {
            if n.parent_id == Some(chapter_id)
                && matches!(n.kind, booksforge_domain::NodeKind::Scene)
            {
                if let Some(sc) = project.storage.load_scene(n.id).await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?
                {
                    let text = pm_doc_to_text(&sc.pm_doc);
                    chapter_text.push_str(&text);
                    chapter_text.push_str("\n\n");
                    per_scene_text.push((n.id, n.title.clone(), text));
                }
            }
        }
        if per_scene_text.is_empty() {
            return Err(BooksForgeError::validation(
                "chapter has no scenes — write some content before requesting a developmental review".to_owned(),
            ));
        }

        // Pull project brief + prior summaries (same shape as
        // agent_run_dev_editor).
        let book_memory = project.storage
            .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
            .await.map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let project_brief = book_memory.iter()
            .find(|m| m.key == "brief")
            .map(|m| m.value_json.clone())
            .unwrap_or_else(|| serde_json::json!({}));

        let chapter_memory = project.storage
            .memory_list_by_scope(booksforge_domain::MemoryScope::Chapter)
            .await.map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let prior_summaries = serde_json::json!(
            chapter_memory.iter()
                .filter(|m| m.key.ends_with(":summary"))
                .map(|m| serde_json::json!({
                    "chapter_id": m.key.trim_end_matches(":summary"),
                    "summary":    m.value_json,
                }))
                .collect::<Vec<_>>()
        );

        let context = load_run_context(&project).await?;
        let known_entities_json = serde_json::to_value(&context.entity_bible)
            .unwrap_or_else(|_| serde_json::json!([]));

        let orchestrator = open_orchestrator(&state).await?;
        let (dr_on_token, dr_stop) = start_token_progress_emitter(app.clone(), run_id.clone());
        let mut dr_stop_box: Option<Box<dyn FnOnce() + Send>> = Some(dr_stop);
        let r_inner = orchestrator.run_developmental_review(
            project_id,
            input.chapter_id,
            chapter_text,
            per_scene_text,
            project_brief,
            prior_summaries,
            known_entities_json,
            input.project_pov,
            context,
            input.model,
            cancel.clone(),
            Some(dr_on_token),
        ).await;
        if let Some(f) = dr_stop_box.take() { f(); }
        let r = r_inner
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        let continuity_passes: Vec<ContinuityScenePassDto> = r.continuity_findings
            .into_iter()
            .map(|p| ContinuityScenePassDto {
                scene_id:    p.scene_id,
                scene_title: p.scene_title,
                finding_count: p.findings.len() as u32,
                findings_json: serde_json::to_string(&p.findings).unwrap_or_else(|_| "[]".into()),
            })
            .collect();

        Ok(RunDevelopmentalReviewResult {
            chapter_id:      r.chapter_id,
            dev_run_id:      r.dev_run_id,
            dev_task_id:     r.dev_task_id,
            dev_status:      r.dev_status,
            dev_notes_json:  r.dev_notes.as_ref().and_then(|n| serde_json::to_string(n).ok()),
            dev_error:       r.dev_error,
            dev_raw:         r.dev_raw,
            continuity_passes,
            scenes_scanned:  r.scenes_scanned,
        })
    }.await;
    let err = res.as_ref().err().map(|e: &BooksForgeError| format!("{e:?}"));
    end_agent_run(&state, &app, "developmental-review", run_id, &cancel, err).await;
    res
}

// ── entity_bible_apply_proposals (BACKLOG §F4) ───────────────────────────────

/// Promote selected `EntityStub`s from a memory-curator run into the
/// project's entity bible (real `Entity` rows).  Loads the persisted
/// `MemoryRefreshProposals`, parses each accepted stub's `kind` to an
/// `EntityKind`, and inserts via `storage.insert_entity`.  Stubs whose
/// kind doesn't map to a known enum variant are skipped — count
/// surfaces in the result so the UI can show "3 added, 1 skipped".
#[tauri::command]
pub async fn entity_bible_apply_proposals(
    input: EntityBibleApplyInput,
    state: State<'_, AppState>,
) -> Result<EntityBibleApplyResult, BooksForgeError> {
    use booksforge_domain::{Entity, EntityKind};

    let task_id = Ulid::from_string(&input.task_id)
        .map_err(|_| BooksForgeError::validation("invalid task_id ULID".to_owned()))?;
    let project = require_open_project(&state).await?;

    // Load the persisted memory-refresh proposals.
    let output = project.storage.agent_output_load(task_id).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
        .ok_or_else(|| BooksForgeError::validation(format!(
            "no agent_outputs row for task {task_id}"
        )))?;
    let raw = output.content_inline.ok_or_else(|| BooksForgeError::validation(
        format!("agent_outputs[{task_id}] has no inline content"),
    ))?;
    let proposals: booksforge_domain::MemoryRefreshProposals =
        serde_json::from_str(&raw).map_err(|e| BooksForgeError::internal(format!(
            "could not deserialise stored MemoryRefreshProposals: {e}"
        )))?;

    let accepted: std::collections::HashSet<u32> =
        input.accepted_indices.into_iter().collect();
    let mut inserted = 0u32;
    let mut skipped  = 0u32;

    for (i, stub) in proposals.new_entities.iter().enumerate() {
        if !accepted.contains(&(i as u32)) { continue; }
        let kind = match stub.kind.to_lowercase().as_str() {
            "character" | "person" | "people"     => EntityKind::Character,
            "location"  | "place"                 => EntityKind::Location,
            "item"      | "object" | "artifact"   => EntityKind::Item,
            "organisation" | "organization" | "org" | "group" | "faction" => EntityKind::Organisation,
            "theme"                                => EntityKind::Theme,
            "custom" | "other"                    => EntityKind::Custom,
            _ => { skipped += 1; continue; }
        };
        if stub.name.trim().is_empty() {
            skipped += 1;
            continue;
        }
        let now = chrono::Utc::now();
        let entity = Entity {
            id:          Ulid::new(),
            kind,
            name:        stub.name.clone(),
            aliases:     stub.aliases.clone(),
            fields_json: stub.fields.clone(),
            notes:       String::new(),
            created_at:  now,
            updated_at:  now,
            deleted_at:  None,
        };
        match project.storage.insert_entity(&entity).await {
            Ok(_)  => inserted += 1,
            Err(e) => {
                tracing::warn!(error = %e, name = stub.name, "entity insert failed — skipping");
                skipped += 1;
            }
        }
    }

    Ok(EntityBibleApplyResult {
        task_id: task_id.to_string(),
        inserted,
        skipped,
    })
}
