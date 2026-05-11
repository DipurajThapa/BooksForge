//! Workflow orchestration commands (Phase 4E of PRODUCT_ROADMAP_E2E.md).
//!
//! These commands chain multiple agents together for the
//! one-click "full scene" experience the writer expects: pick a scene,
//! click Run, get a draft → critic notes → polished prose with the
//! genre-correct polish-stack order.
//!
//! The chained commands deliberately call orchestrator methods directly
//! (not the per-agent Tauri command wrappers). This keeps the
//! `begin_agent_run` / token-emitter ceremony out of the loop body and
//! lets the workflow surface ONE consolidated progress event per stage
//! to the UI.

use booksforge_anti_ai_tells::tells_per_1000_words;
use booksforge_domain::{BookKind, PolishStageId};
use booksforge_fs::manifest::BundleManifest;
use booksforge_genre_packs::pack_for;
use booksforge_ipc::{AgentRunResultDto, BooksForgeError};
use booksforge_ollama::{types::CancelToken, OllamaClient};
use booksforge_storage::StorageRepository as _;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as _, State};
use tokio::sync::Mutex;
use ts_rs::TS;
use ulid::Ulid;

use crate::commands::agents::{
    begin_agent_run, end_agent_run, load_run_context, open_orchestrator, require_open_project,
    run_result_to_dto,
};
use crate::state::AppState;

// ── Inputs / Results ─────────────────────────────────────────────────────────

/// Input to `agent_run_full_scene_pipeline`. Drafts a fiction scene
/// using the project's `book_kind` to drive genre-correct prompts +
/// polish-stack ordering, then runs the 4-stage polish stack, and
/// finishes with an AI-tells density measurement.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunFullScenePipelineInput {
    pub project_id: String,
    pub node_id: String,
    pub scene_goal: String,
    pub scene_conflict: String,
    pub scene_reveal: String,
    pub target_words: u32,
    pub chapter_pov: String,
    /// Default model for stages that don't specify their own. Acts as
    /// the fallback for both the heavy (drafter / polish) and the
    /// light (critic) tier when `model_critic` is unset. Most callers
    /// will pin this to a 27B-class model.
    pub model: String,
    /// O2 of `docs/VSM_LLM_OPTIMIZATION.md` — optional faster model
    /// used for the scene-critic pass. The critic's job is structural
    /// scoring + targeted edit suggestions, not prose generation, so
    /// a smaller faster model (e.g. `qwen3.5:9b`) shaves ~20–30% off
    /// total wall-clock without measurable quality loss. Falls back
    /// to `model` when unset.
    #[serde(default)]
    pub model_critic: Option<String>,
    /// O1 of `docs/VSM_LLM_OPTIMIZATION.md` — when true (the
    /// default), polish stages whose deterministic skip-detector
    /// reports "no work to do" are skipped instead of running the
    /// LLM. Set to `false` to force every stage to run regardless
    /// (useful for benchmarking). Saves ~30–40% wall-clock on a
    /// typical literary scene where 1–2 of the 4 polish stages are
    /// no-ops.
    #[serde(default = "default_true")]
    pub skip_empty_polish_stages: bool,
    /// When true, the workflow stops after the scene-critic pass and
    /// returns its findings — useful for "draft, then let me decide
    /// whether to polish." Default `false` (run the full pipeline).
    #[serde(default)]
    pub stop_after_critic: bool,
}

fn default_true() -> bool {
    true
}

/// One stage of the pipeline. The UI uses these to render per-stage
/// status badges as the workflow progresses.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PipelineStageResult {
    /// `"draft" | "critic" | "polish:dialogue" | "polish:metaphor" |
    /// "polish:voice" | "polish:scene_tension" | "tells_scan"`.
    pub stage: String,
    /// `"completed" | "skipped" | "failed"`.
    pub status: String,
    /// task_id for an LLM stage (so the UI can fetch the proposal_json
    /// and offer a per-stage Apply); empty for `tells_scan`.
    pub task_id: String,
    /// Free-text hint surfaced to the user. For the critic: the
    /// `weakest_axis` and `overall_one_liner`. For tells: the verdict +
    /// density.
    pub summary: String,
    /// Wall-clock seconds for this stage (rounded to 1 decimal).
    pub elapsed_s: f32,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunFullScenePipelineResult {
    pub project_id: String,
    pub node_id: String,
    /// The book_kind used to drive prompt + polish-stack ordering.
    pub book_kind: String,
    pub stages: Vec<PipelineStageResult>,
    /// Aggregate AI-tells density on the final polished prose, rendered
    /// as the verdict string ("PUBLISHABLE" / "NEEDS_REVISION" / "AI_SMELL_HIGH").
    pub final_tells_verdict: String,
    /// Total wall-clock seconds across every stage.
    pub total_elapsed_s: f32,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Read the open project's `book_kind` from the manifest. Falls back
/// to `LiteraryFiction` and emits a warning if the project predates
/// Phase 4 — the UI's onboarding overlay should have caught this
/// before getting here, but we don't refuse the workflow on its
/// account.
async fn resolve_book_kind(state: &State<'_, AppState>) -> Result<BookKind, BooksForgeError> {
    let project = require_open_project(state).await?;
    let manifest = BundleManifest::read_from_bundle(&project.bundle)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(manifest.project.book_kind.unwrap_or_else(|| {
        tracing::warn!(
            "no book_kind on project; defaulting to LiteraryFiction. \
             Surface the onboarding overlay so the user can choose."
        );
        BookKind::LiteraryFiction
    }))
}

// ── agent_run_full_scene_pipeline ──────────────────────────────────────────

/// Chained workflow: scene-drafter-fic → scene-critic → 4-stage
/// genre-ordered polish stack → AI-tells scan. Each stage emits a
/// `pipeline:progress` Tauri event so the UI can render per-stage
/// status badges in real time.
#[tauri::command]
pub async fn agent_run_full_scene_pipeline(
    input: RunFullScenePipelineInput,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RunFullScenePipelineResult, BooksForgeError> {
    let book_kind = resolve_book_kind(&state).await?;
    let pack = pack_for(book_kind);

    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;
    let scene_id = Ulid::from_string(&input.node_id)
        .map_err(|_| BooksForgeError::validation("invalid node_id".to_owned()))?;

    let total_start = std::time::Instant::now();
    let stages: Mutex<Vec<PipelineStageResult>> = Mutex::new(Vec::new());

    let emit_progress = |stage: &str, status: &str, summary: &str, elapsed_s: f32| {
        let _ = app.emit(
            "pipeline:progress",
            serde_json::json!({
                "stage": stage,
                "status": status,
                "summary": summary,
                "elapsed_s": elapsed_s,
            }),
        );
    };

    // ── Stage 1: scene-drafter-fic ──────────────────────────────────────
    let stage_start = std::time::Instant::now();
    emit_progress("draft", "running", "Drafting scene…", 0.0);
    let (run_id, cancel) = begin_agent_run(&state, &app, "scene-drafter-fic").await;
    let draft_dto = run_scene_drafter_fic_stage(
        &state,
        &app,
        &run_id,
        cancel.clone(),
        project_id,
        scene_id,
        &input,
        &pack,
    )
    .await;
    let draft_err = draft_dto.as_ref().err().map(|e| format!("{e:?}"));
    end_agent_run(
        &state,
        &app,
        "scene-drafter-fic",
        run_id,
        &cancel,
        draft_err,
    )
    .await;
    let draft_dto = draft_dto?;
    let draft_elapsed = stage_start.elapsed().as_secs_f32();
    emit_progress(
        "draft",
        "completed",
        &format!("Drafted in {draft_elapsed:.1}s"),
        draft_elapsed,
    );
    stages.lock().await.push(PipelineStageResult {
        stage: "draft".to_owned(),
        status: "completed".to_owned(),
        task_id: draft_dto.task_id.clone(),
        summary: format!("draft completed via {} pack", pack.kind.as_str()),
        elapsed_s: round1(draft_elapsed),
    });
    let draft_proposal_json = draft_dto.proposal_json.clone().unwrap_or_default();
    let scene_text = extract_pm_doc_text(&draft_proposal_json);

    // ── Stage 2: scene-critic ───────────────────────────────────────────
    let stage_start = std::time::Instant::now();
    emit_progress("critic", "running", "Critiquing draft…", 0.0);
    let (run_id, cancel) = begin_agent_run(&state, &app, "scene-critic").await;
    let critic_dto = run_scene_critic_stage(
        &state,
        &app,
        &run_id,
        cancel.clone(),
        project_id,
        scene_id,
        &input,
        &pack,
        &scene_text,
    )
    .await;
    let critic_err = critic_dto.as_ref().err().map(|e| format!("{e:?}"));
    end_agent_run(&state, &app, "scene-critic", run_id, &cancel, critic_err).await;
    let critic_dto = critic_dto?;
    let critic_elapsed = stage_start.elapsed().as_secs_f32();
    let critic_summary = extract_critic_summary(critic_dto.proposal_json.as_deref());
    emit_progress("critic", "completed", &critic_summary, critic_elapsed);
    stages.lock().await.push(PipelineStageResult {
        stage: "critic".to_owned(),
        status: "completed".to_owned(),
        task_id: critic_dto.task_id.clone(),
        summary: critic_summary.clone(),
        elapsed_s: round1(critic_elapsed),
    });

    if input.stop_after_critic {
        let final_tells = tells_per_1000_words(&scene_text);
        return Ok(RunFullScenePipelineResult {
            project_id: input.project_id,
            node_id: input.node_id,
            book_kind: book_kind.as_str().to_owned(),
            stages: stages.into_inner(),
            final_tells_verdict: final_tells.verdict,
            total_elapsed_s: round1(total_start.elapsed().as_secs_f32()),
        });
    }

    // ── Stages 3–6: genre-ordered polish stack ────────────────────────
    // The pack's `polish_stack_order` is a list of `PolishStageId`
    // strings (`"voice" | "metaphor" | "dialogue" | "scene_tension"`).
    // We run them in order on the most recent revised pm_doc.
    let mut current_text = scene_text.clone();
    for stage_str in &pack.polish_stack_order {
        let Some(stage_id) = PolishStageId::from_str(stage_str) else {
            // Unknown stage in pack — log + skip. (Pack data is curated;
            // this can only happen on schema drift.)
            stages.lock().await.push(PipelineStageResult {
                stage: format!("polish:{stage_str}"),
                status: "skipped".to_owned(),
                task_id: String::new(),
                summary: format!("unknown polish stage: {stage_str}"),
                elapsed_s: 0.0,
            });
            continue;
        };
        let stage_label = format!("polish:{stage_str}");

        // O1 — deterministic skip when this stage has no work to do.
        // Pure pattern scan, ~1ms; saves the 60–90s LLM call entirely.
        // The user can disable this per-run via `skip_empty_polish_stages: false`.
        if input.skip_empty_polish_stages {
            if let Some(reason) = booksforge_genre_packs::skip_reason(stage_id, &current_text) {
                let summary = format!("skipped: {reason}");
                emit_progress(&stage_label, "skipped", &summary, 0.0);
                stages.lock().await.push(PipelineStageResult {
                    stage: stage_label,
                    status: "skipped".to_owned(),
                    task_id: String::new(),
                    summary,
                    elapsed_s: 0.0,
                });
                continue;
            }
        }
        let agent_label = match stage_id {
            PolishStageId::Dialogue => "dialogue-polish",
            PolishStageId::Metaphor => "metaphor-polish",
            PolishStageId::Voice => "voice-polish",
            PolishStageId::SceneTension => "scene-tension-polish",
        };
        let stage_start = std::time::Instant::now();
        emit_progress(&stage_label, "running", "Polishing…", 0.0);
        let (run_id, cancel) = begin_agent_run(&state, &app, agent_label).await;
        let polish_res = run_polish_stage_inner(
            &state,
            project_id,
            stage_id,
            current_text.clone(),
            &pack,
            &input,
            cancel.clone(),
        )
        .await;
        let polish_err = polish_res.as_ref().err().map(|e| format!("{e:?}"));
        end_agent_run(&state, &app, agent_label, run_id, &cancel, polish_err).await;
        match polish_res {
            Ok(dto) => {
                let stage_elapsed = stage_start.elapsed().as_secs_f32();
                let revised = dto
                    .proposal_json
                    .as_deref()
                    .map(extract_polish_revised_text)
                    .unwrap_or_default();
                if !revised.is_empty() {
                    current_text = revised;
                }
                let summary = format!("{stage_str} polish complete in {stage_elapsed:.1}s");
                emit_progress(&stage_label, "completed", &summary, stage_elapsed);
                stages.lock().await.push(PipelineStageResult {
                    stage: stage_label,
                    status: "completed".to_owned(),
                    task_id: dto.task_id,
                    summary,
                    elapsed_s: round1(stage_elapsed),
                });
            }
            Err(e) => {
                let stage_elapsed = stage_start.elapsed().as_secs_f32();
                let summary = format!("{stage_str} polish FAILED: {e}");
                emit_progress(&stage_label, "failed", &summary, stage_elapsed);
                stages.lock().await.push(PipelineStageResult {
                    stage: stage_label,
                    status: "failed".to_owned(),
                    task_id: String::new(),
                    summary,
                    elapsed_s: round1(stage_elapsed),
                });
            }
        }
    }

    // ── Stage 7: AI-tells scan on final prose ──────────────────────────
    let tells_start = std::time::Instant::now();
    emit_progress("tells_scan", "running", "Measuring AI-tells density…", 0.0);
    let final_tells = tells_per_1000_words(&current_text);
    let tells_elapsed = tells_start.elapsed().as_secs_f32();
    let tells_summary = format!(
        "{} ({:.1} weighted/1000 words)",
        final_tells.verdict, final_tells.weighted_density_per_1000
    );
    emit_progress("tells_scan", "completed", &tells_summary, tells_elapsed);
    stages.lock().await.push(PipelineStageResult {
        stage: "tells_scan".to_owned(),
        status: "completed".to_owned(),
        task_id: String::new(),
        summary: tells_summary,
        elapsed_s: round1(tells_elapsed),
    });

    Ok(RunFullScenePipelineResult {
        project_id: input.project_id,
        node_id: input.node_id,
        book_kind: book_kind.as_str().to_owned(),
        stages: stages.into_inner(),
        final_tells_verdict: final_tells.verdict,
        total_elapsed_s: round1(total_start.elapsed().as_secs_f32()),
    })
}

// ── Internal stage helpers ───────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_scene_drafter_fic_stage(
    state: &State<'_, AppState>,
    _app: &AppHandle,
    _run_id: &str,
    cancel: CancelToken,
    project_id: Ulid,
    _scene_id: Ulid,
    input: &RunFullScenePipelineInput,
    pack: &booksforge_genre_packs::GenrePack,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let project = require_open_project(state).await?;
    let context = load_run_context(&project).await?;

    // Assemble bibles from project memory.
    let entity_mem = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Entity)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let characters: Vec<_> = entity_mem
        .iter()
        .filter(|m| m.key.starts_with("character:"))
        .map(|m| m.value_json.clone())
        .collect();
    let character_bible_json = serde_json::json!({ "characters": characters });
    let locations: Vec<_> = entity_mem
        .iter()
        .filter(|m| m.key.starts_with("location:"))
        .map(|m| m.value_json.clone())
        .collect();
    let book_mem = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let mut world_obj = serde_json::Map::new();
    world_obj.insert("main_locations".to_owned(), serde_json::json!(locations));
    for m in book_mem.iter().filter(|m| m.key.starts_with("world:")) {
        let field_name = m.key.strip_prefix("world:").unwrap_or(&m.key).to_owned();
        world_obj.insert(field_name, m.value_json.clone());
    }
    let world_bible_json = serde_json::Value::Object(world_obj);

    // Voice constraints from book-scope `voice:anchor` (if set).
    let voice_constraints: String = book_mem
        .iter()
        .find(|m| m.key == "voice:anchor")
        .and_then(|m| {
            m.value_json["constraints_block"]
                .as_str()
                .map(|s| s.to_owned())
        })
        .unwrap_or_default();

    // Prior summary from chapter-scope memory.
    let chapter_memory = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Chapter)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let prior_summary = chapter_memory
        .iter()
        .filter(|m| m.key.ends_with(":summary"))
        .max_by_key(|m| m.updated_at)
        .map(|m| m.value_json.as_str().unwrap_or("").to_owned())
        .unwrap_or_default();

    let orchestrator = open_orchestrator(state).await?;
    let result = orchestrator
        .run_scene_drafter_fic(
            project_id,
            input.scene_goal.clone(),
            input.scene_conflict.clone(),
            input.scene_reveal.clone(),
            input.target_words,
            input.chapter_pov.clone(),
            pack.genre_label.clone(),
            character_bible_json,
            world_bible_json,
            voice_constraints,
            prior_summary,
            context,
            input.model.clone(),
            cancel,
            None,
        )
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(run_result_to_dto(result, "scene-drafter-fic"))
}

#[allow(clippy::too_many_arguments)]
async fn run_scene_critic_stage(
    state: &State<'_, AppState>,
    _app: &AppHandle,
    _run_id: &str,
    cancel: CancelToken,
    project_id: Ulid,
    _scene_id: Ulid,
    input: &RunFullScenePipelineInput,
    pack: &booksforge_genre_packs::GenrePack,
    scene_text: &str,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let project = require_open_project(state).await?;
    let context = load_run_context(&project).await?;

    let book_mem = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let voice_constraints: String = book_mem
        .iter()
        .find(|m| m.key == "voice:anchor")
        .and_then(|m| {
            m.value_json["constraints_block"]
                .as_str()
                .map(|s| s.to_owned())
        })
        .unwrap_or_default();

    let chapter_memory = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Chapter)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let prior_summary = chapter_memory
        .iter()
        .filter(|m| m.key.ends_with(":summary"))
        .max_by_key(|m| m.updated_at)
        .map(|m| m.value_json.as_str().unwrap_or("").to_owned())
        .unwrap_or_default();

    let orchestrator = open_orchestrator(state).await?;
    // O2 — route the critic to a faster model when one is configured.
    // The critic produces structural scoring + targeted-edit
    // suggestions, not generative prose, so a smaller/faster model
    // shaves ~20–30% off total wall-clock without measurable quality
    // loss. Falls back to the heavy model when `model_critic` is None.
    let critic_model = input
        .model_critic
        .clone()
        .unwrap_or_else(|| input.model.clone());
    let result = orchestrator
        .run_scene_critic(
            project_id,
            scene_text.to_owned(),
            input.scene_goal.clone(),
            input.scene_conflict.clone(),
            input.scene_reveal.clone(),
            pack.critic_axes.clone(),
            pack.genre_label.clone(),
            voice_constraints,
            prior_summary,
            context,
            critic_model,
            cancel,
        )
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(run_result_to_dto(result, "scene-critic"))
}

async fn run_polish_stage_inner(
    state: &State<'_, AppState>,
    project_id: Ulid,
    stage_id: PolishStageId,
    chapter_text: String,
    pack: &booksforge_genre_packs::GenrePack,
    input: &RunFullScenePipelineInput,
    cancel: CancelToken,
) -> Result<AgentRunResultDto, BooksForgeError> {
    let project = require_open_project(state).await?;
    let context = load_run_context(&project).await?;
    let book_mem = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let voice_constraints: String = book_mem
        .iter()
        .find(|m| m.key == "voice:anchor")
        .and_then(|m| {
            m.value_json["constraints_block"]
                .as_str()
                .map(|s| s.to_owned())
        })
        .unwrap_or_default();

    let agent_label = match stage_id {
        PolishStageId::Dialogue => "dialogue-polish",
        PolishStageId::Metaphor => "metaphor-polish",
        PolishStageId::Voice => "voice-polish",
        PolishStageId::SceneTension => "scene-tension-polish",
    };

    let orchestrator = open_orchestrator(state).await?;
    let result = orchestrator
        .run_polish_stage(
            project_id,
            stage_id,
            chapter_text,
            pack.genre_label.clone(),
            voice_constraints,
            input.chapter_pov.clone(),
            context,
            input.model.clone(),
            cancel,
            None,
        )
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(run_result_to_dto(result, agent_label))
}

// ── Pure helpers ─────────────────────────────────────────────────────────────

fn round1(x: f32) -> f32 {
    (x * 10.0).round() / 10.0
}

/// Pull plain text out of a SceneDraftProposal's pm_doc JSON.
fn extract_pm_doc_text(proposal_json: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(proposal_json) else {
        return String::new();
    };
    booksforge_domain::pm_doc_to_text(&value["pm_doc"])
}

/// Pull plain text out of a PolishProposal's revised_pm_doc JSON.
fn extract_polish_revised_text(proposal_json: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(proposal_json) else {
        return String::new();
    };
    booksforge_domain::pm_doc_to_text(&value["revised_pm_doc"])
}

/// Render the per-scene critic's verdict + weakest axis as a one-liner
/// for the progress event.
fn extract_critic_summary(proposal_json: Option<&str>) -> String {
    let Some(raw) = proposal_json else {
        return "critic produced no output".to_owned();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return "critic produced unparseable output".to_owned();
    };
    let weakest = value["weakest_axis"].as_str().unwrap_or("?");
    let one_liner = value["overall_one_liner"].as_str().unwrap_or("");
    let n_edits = value["specific_edits"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    format!("weakest: {weakest} · {n_edits} targeted edits · {one_liner}")
}

// ── Book-level pipeline (character-bible → world-bible → for-each scene drafter) ─

/// Input to `agent_run_book_pipeline`. Drives the full intake →
/// bibles → per-scene drafter chain in a single Tauri call so the
/// writer doesn't have to manually trigger each agent from the
/// Agents panel after the outline lands.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunBookPipelineInput {
    pub project_id: String,
    /// When true (default), scenes that already have prose
    /// (`scene_content.word_count > 0`) are skipped — so the user can
    /// re-run the pipeline after editing one scene without losing
    /// hand-edits to others. Set `false` to overwrite every scene
    /// (a snapshot is taken before each apply, so it's reversible).
    #[serde(default = "default_true")]
    pub skip_already_drafted_scenes: bool,
    /// Hard cap on how many scenes to draft in this run. `None` =
    /// no cap (drafts every scene in the project's outline).
    /// Useful for "just generate Chapter 1" by passing 3-4.
    #[serde(default)]
    pub max_scenes: Option<u32>,
}

/// Per-stage result for the book pipeline. The UI uses this to render
/// a checklist of completed stages. The `scenes_drafted` array carries
/// one entry per scene that the drafter touched.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunBookPipelineResult {
    pub project_id: String,
    pub character_bible_status: String,
    pub world_bible_status: String,
    /// One entry per scene, in document order. `status` is one of
    /// `"completed"`, `"skipped"`, `"failed"`, or `"cancelled"`.
    pub scenes: Vec<BookSceneStageResult>,
    pub total_elapsed_s: f32,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BookSceneStageResult {
    pub scene_id: String,
    pub scene_title: String,
    pub status: String,
    pub word_count: u32,
    pub elapsed_s: f32,
    pub note: String,
}

/// Chained workflow: character-bible → world-bible → for-each scene
/// in document order, run scene-drafter-fic and persist to
/// `scene_content`. Each stage emits a `book-pipeline:progress` Tauri
/// event so the UI can show what's currently running.
///
/// Auto-resolves models per tier: bibles run on Medium (qwen3.5:27b
/// when installed), drafter on Heavy (qwen3.6:latest when installed).
/// The user does NOT pick a model; that's the orchestrator's job.
///
/// Cancellation: cancellable via `agent_cancel` with the run_id from
/// the `agent-run-started` event. The current in-flight stage will
/// be interrupted; partial progress is preserved (already-applied
/// stages stay; the cancelled stage rolls back).
#[tauri::command]
pub async fn agent_run_book_pipeline(
    input: RunBookPipelineInput,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RunBookPipelineResult, BooksForgeError> {
    let total_start = std::time::Instant::now();
    let project_id = Ulid::from_string(&input.project_id)
        .map_err(|_| BooksForgeError::validation("invalid project_id".to_owned()))?;

    // Resolve models for each tier from currently installed Ollama
    // tags. Done once upfront so the user sees the resolution decision
    // in the dispatch log instead of three separate decisions.
    let installed: Vec<String> = state
        .ollama
        .list_local_models()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.name)
        .collect();
    let bible_model = booksforge_ollama::registry::resolve_tier(
        booksforge_ollama::registry::ModelTier::Medium,
        &installed,
    )
    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let drafter_model = booksforge_ollama::registry::resolve_tier(
        booksforge_ollama::registry::ModelTier::Heavy,
        &installed,
    )
    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    tracing::info!(
        bible_model = %bible_model,
        drafter_model = %drafter_model,
        "agent_run_book_pipeline starting",
    );

    let project = require_open_project(&state).await?;
    let context = load_run_context(&project).await?;
    let book_kind = resolve_book_kind(&state).await?;
    let genre_lens = match book_kind {
        BookKind::LiteraryFiction | BookKind::Memoir => "literary_fiction",
        _ => "genre_fiction",
    }
    .to_owned();

    // Pull the project brief from book-scope memory. If the user
    // never ran the intake form (some flows skip it), fall back to a
    // minimal brief so the agents have something coherent to build
    // from rather than failing the structural validators.
    let brief_value = project
        .storage
        .memory_get(booksforge_domain::MemoryScope::Book, "project_brief")
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
        .map(|m| m.value_json)
        .unwrap_or_else(|| {
            serde_json::json!({
                "title_suggestions": [project.title.clone()],
                "mode": "fiction",
                "premise": "A story about characters facing meaningful choices.",
                "key_promises": ["Engaging characters", "Clear stakes"],
            })
        });

    let nodes = project
        .storage
        .list_nodes()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let chapter_count = nodes
        .iter()
        .filter(|n| n.kind == booksforge_domain::NodeKind::Chapter)
        .count() as u32;
    if chapter_count == 0 {
        return Err(BooksForgeError::validation(
            "project has no chapters — run the outline-architect first.".to_owned(),
        ));
    }
    // Scenes in document order: walk in the same order list_nodes
    // returns. The Binder displays this same order, so progress
    // events line up with what the user sees on the left.
    let scene_nodes: Vec<&booksforge_domain::Node> = nodes
        .iter()
        .filter(|n| n.kind == booksforge_domain::NodeKind::Scene && n.deleted_at.is_none())
        .collect();
    if scene_nodes.is_empty() {
        return Err(BooksForgeError::validation(
            "project has no scenes — run the outline-architect first.".to_owned(),
        ));
    }

    let emit_progress =
        |stage: &str, status: &str, summary: &str, current: u32, total: u32, elapsed_s: f32| {
            let _ = app.emit(
                "book-pipeline:progress",
                serde_json::json!({
                    "stage": stage,
                    "status": status,
                    "summary": summary,
                    "current": current,
                    "total": total,
                    "elapsed_s": elapsed_s,
                }),
            );
        };

    // Pre-flight scan: does the writer already have bibles in memory?
    // The `bibles_save` IPC command writes to the same scopes the AI
    // agents do (entity:character:*, entity:location:*, book:world:*),
    // so we just count entries to decide whether to skip the LLM.
    // Saves up to 5-10 minutes of pipeline time on a hand-authored
    // bible.
    let pre_entity = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Entity)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let pre_book = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let has_user_character_bible = pre_entity.iter().any(|m| m.key.starts_with("character:"));
    let has_user_world_bible = pre_entity.iter().any(|m| m.key.starts_with("location:"))
        || pre_book.iter().any(|m| m.key.starts_with("world:"));

    // ── Stage 1: character-bible (skipped if writer provided one) ──
    //
    // Switched 2026-05 from monolithic `run_character_bible` to
    // `run_character_bible_chunked`. The orchestrator's own RCA
    // comment for the monolithic path: "Local models smaller than ~30B
    // chronically fail this in one shot — runner cycles through max
    // retries and returns an empty bible." We were burning 5+ min on
    // qwen3.5:27b watching the retry-ladder. The chunked path
    // generates each character independently (~250-400 tokens
    // each), and the multi_chapter_run integration test uses it
    // successfully on qwen3.5:9b in ~2-4 min total.
    //
    // Tradeoff: chunked returns a `CharacterBibleProposal` directly
    // (not wrapped in AgentRunResult), so the standard apply path
    // doesn't apply — we persist each character to entity memory by
    // hand below, mirroring `bibles_save`.
    let stage_start = std::time::Instant::now();
    let orchestrator = open_orchestrator(&state).await?;
    // Light tier (qwen3.5:9b) is the right pick for chunked — it's
    // what the chunked path was designed around. Falls back to the
    // bible_model (Medium) if Light isn't installed.
    let chunked_cb_model = booksforge_ollama::registry::resolve_tier(
        booksforge_ollama::registry::ModelTier::Light,
        &installed,
    )
    .unwrap_or_else(|_| bible_model.clone());
    // 4 characters is the chunked default (1 protagonist, 1 antagonist,
    // 2 supporting). Matches what the multi_chapter_run example uses.
    const CHUNKED_CHARACTER_COUNT: u32 = 4;
    let (cb_status, cb_elapsed) = if has_user_character_bible {
        let elapsed = stage_start.elapsed().as_secs_f32();
        emit_progress(
            "character-bible",
            "skipped",
            "Character bible: writer-supplied — skipping AI stage",
            0,
            0,
            elapsed,
        );
        ("skipped (writer-supplied)".to_owned(), elapsed)
    } else {
        emit_progress(
            "character-bible",
            "running",
            &format!(
                "Building character bible — generating {CHUNKED_CHARACTER_COUNT} characters one at a time on {chunked_cb_model}…"
            ),
            0,
            CHUNKED_CHARACTER_COUNT,
            0.0,
        );
        let (cb_run_id, cb_cancel) = begin_agent_run(&state, &app, "character-bible").await;
        let cb_result = orchestrator
            .run_character_bible_chunked(
                project_id,
                brief_value.clone(),
                chapter_count,
                CHUNKED_CHARACTER_COUNT,
                context.clone(),
                chunked_cb_model.clone(),
                cb_cancel.clone(),
            )
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()));
        let cb_err = cb_result
            .as_ref()
            .err()
            .map(|e: &BooksForgeError| format!("{e:?}"));
        // Persist each character returned by the chunked path to
        // entity:character:<slug> memory. Mirrors the on-disk shape
        // of `bibles_save` and `apply_character_bible` so the scene
        // drafter sees it identically.
        let status = match &cb_result {
            Ok(bible) if !bible.characters.is_empty() => {
                let now = chrono::Utc::now();
                let mut saved = 0u32;
                for card in &bible.characters {
                    let slug: String = card
                        .name
                        .trim()
                        .to_lowercase()
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                        .trim_matches('_')
                        .to_owned();
                    let value = match serde_json::to_value(card) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let entry = booksforge_domain::MemoryEntry {
                        id: Ulid::new(),
                        scope: booksforge_domain::MemoryScope::Entity,
                        key: format!("character:{slug}"),
                        value_json: value,
                        agent_id: "character-bible".to_owned(),
                        created_at: now,
                        updated_at: now,
                    };
                    if project.storage.memory_upsert(&entry).await.is_ok() {
                        saved = saved.saturating_add(1);
                    }
                }
                if saved > 0 {
                    format!("completed ({saved} characters)")
                } else {
                    "no-output".to_owned()
                }
            }
            Ok(_) => "no-output (empty bible)".to_owned(),
            Err(e) => format!("failed: {e}"),
        };
        end_agent_run(
            &state,
            &app,
            "character-bible",
            cb_run_id,
            &cb_cancel,
            cb_err,
        )
        .await;
        let elapsed = stage_start.elapsed().as_secs_f32();
        emit_progress(
            "character-bible",
            if status.starts_with("completed") {
                "completed"
            } else {
                "failed"
            },
            &format!("Character bible: {status} ({elapsed:.1}s)"),
            CHUNKED_CHARACTER_COUNT,
            CHUNKED_CHARACTER_COUNT,
            elapsed,
        );
        (status, elapsed)
    };
    let _ = cb_elapsed; // kept for symmetry with the wb_elapsed below

    // ── Stage 2: world-bible (skipped if writer provided one) ───────
    let stage_start = std::time::Instant::now();
    let (wb_status, wb_elapsed) = if has_user_world_bible {
        let elapsed = stage_start.elapsed().as_secs_f32();
        emit_progress(
            "world-bible",
            "skipped",
            "World bible: writer-supplied — skipping AI stage",
            0,
            0,
            elapsed,
        );
        ("skipped (writer-supplied)".to_owned(), elapsed)
    } else {
        emit_progress("world-bible", "running", "Building world bible…", 0, 0, 0.0);
        let (wb_run_id, wb_cancel) = begin_agent_run(&state, &app, "world-bible").await;
        let wb_result = orchestrator
            .run_world_bible(
                project_id,
                brief_value.clone(),
                serde_json::json!({}),
                context.clone(),
                bible_model.clone(),
                wb_cancel.clone(),
            )
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()));
        let wb_err = wb_result
            .as_ref()
            .err()
            .map(|e: &BooksForgeError| format!("{e:?}"));
        let status = match &wb_result {
            Ok(r) if r.output.is_some() => {
                let _ = orchestrator
                    .apply_world_bible(r.task_id)
                    .await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
                "completed".to_owned()
            }
            Ok(_) => "no-output".to_owned(),
            Err(e) => format!("failed: {e}"),
        };
        end_agent_run(&state, &app, "world-bible", wb_run_id, &wb_cancel, wb_err).await;
        let elapsed = stage_start.elapsed().as_secs_f32();
        emit_progress(
            "world-bible",
            if status == "completed" {
                "completed"
            } else {
                "failed"
            },
            &format!("World bible: {status} ({elapsed:.1}s)"),
            0,
            0,
            elapsed,
        );
        (status, elapsed)
    };
    let _ = wb_elapsed;

    // Re-load context now that the bibles are in memory — the scene
    // drafter assembles them from the entity / book scopes inside its
    // own input pipeline.
    let context_after_bibles = load_run_context(&project).await?;

    // Build the bible payloads ONCE for the per-scene drafter calls
    // — assembled from the same memory scopes the per-agent Tauri
    // command uses, so the drafter sees the freshly-applied bibles.
    let entity_mem = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Entity)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let characters: Vec<_> = entity_mem
        .iter()
        .filter(|m| m.key.starts_with("character:"))
        .map(|m| m.value_json.clone())
        .collect();
    let character_bible_json = serde_json::json!({ "characters": characters });
    let locations: Vec<_> = entity_mem
        .iter()
        .filter(|m| m.key.starts_with("location:"))
        .map(|m| m.value_json.clone())
        .collect();
    let book_mem = project
        .storage
        .memory_list_by_scope(booksforge_domain::MemoryScope::Book)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let mut world_obj = serde_json::Map::new();
    world_obj.insert("main_locations".to_owned(), serde_json::json!(locations));
    for m in book_mem.into_iter().filter(|m| m.key.starts_with("world:")) {
        let field_name = m.key.strip_prefix("world:").unwrap_or(&m.key).to_owned();
        world_obj.insert(field_name, m.value_json);
    }
    let world_bible_json = serde_json::Value::Object(world_obj);

    // ── Stage 3: per-scene drafter ──────────────────────────────────
    let mut scene_results: Vec<BookSceneStageResult> = Vec::new();
    let total_scenes = scene_nodes.len() as u32;
    let cap = input.max_scenes.unwrap_or(total_scenes).min(total_scenes);

    // Derive a default POV from the first scene that has one set; falls
    // back to "third-limited" which is the safe literary default.
    let default_pov = scene_nodes
        .iter()
        .find_map(|n| n.pov.clone())
        .unwrap_or_else(|| "third-limited".to_owned());

    for (idx, scene) in scene_nodes.iter().enumerate().take(cap as usize) {
        let current = (idx + 1) as u32;
        let scene_start = std::time::Instant::now();

        // Skip scenes that already have prose, unless the user
        // explicitly opted in to overwrite.
        if input.skip_already_drafted_scenes {
            if let Ok(Some(existing)) = project.storage.load_scene(scene.id).await {
                if existing.word_count > 0 {
                    let elapsed = scene_start.elapsed().as_secs_f32();
                    emit_progress(
                        "scene-drafter-fic",
                        "skipped",
                        &format!("{} (already has prose)", scene.title),
                        current,
                        cap,
                        elapsed,
                    );
                    scene_results.push(BookSceneStageResult {
                        scene_id: scene.id.to_string(),
                        scene_title: scene.title.clone(),
                        status: "skipped".to_owned(),
                        word_count: existing.word_count,
                        elapsed_s: round1(elapsed),
                        note: "already drafted — passed --skip flag".to_owned(),
                    });
                    continue;
                }
            }
        }

        // Build a minimal scene_card from the scene's metadata. The
        // outline-apply step doesn't (yet) persist scene_goal /
        // scene_conflict / scene_reveal as separate fields — only
        // the synopsis, which we use as the goal. The drafter is
        // tolerant of thin cards but produces better prose with
        // richer ones; future work will plumb the outline-architect's
        // synopsis through to per-scene memory entries.
        let scene_goal = if scene.title.trim().is_empty() {
            format!("Scene {current} unfolds.")
        } else {
            scene.title.clone()
        };
        let scene_conflict =
            "The POV character must act despite resistance — internal or external.".to_owned();
        let scene_reveal =
            "Something becomes true at the end of this scene that wasn't true at the start."
                .to_owned();
        let target_words = scene.target_words.unwrap_or(1500);
        let pov = scene.pov.clone().unwrap_or_else(|| default_pov.clone());

        emit_progress(
            "scene-drafter-fic",
            "running",
            &format!("Drafting scene {current}/{cap}: {}", scene.title),
            current,
            cap,
            0.0,
        );

        let (sd_run_id, sd_cancel) = begin_agent_run(&state, &app, "scene-drafter-fic").await;
        let sd_result = orchestrator
            .run_scene_drafter_fic(
                project_id,
                scene_goal,
                scene_conflict,
                scene_reveal,
                target_words,
                pov,
                genre_lens.clone(),
                character_bible_json.clone(),
                world_bible_json.clone(),
                String::new(),
                String::new(),
                context_after_bibles.clone(),
                drafter_model.clone(),
                sd_cancel.clone(),
                None,
            )
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()));

        let sd_err = sd_result
            .as_ref()
            .err()
            .map(|e: &BooksForgeError| format!("{e:?}"));
        let elapsed = scene_start.elapsed().as_secs_f32();

        let stage = match sd_result {
            Ok(r) if r.output.is_some() => {
                let task_id = r.task_id;
                match orchestrator
                    .apply_scene_drafter_fic(task_id, scene.id)
                    .await
                {
                    Ok(applied) => {
                        emit_progress(
                            "scene-drafter-fic",
                            "completed",
                            &format!(
                                "{}: {} words ({elapsed:.1}s)",
                                scene.title, applied.new_word_count
                            ),
                            current,
                            cap,
                            elapsed,
                        );
                        BookSceneStageResult {
                            scene_id: scene.id.to_string(),
                            scene_title: scene.title.clone(),
                            status: "completed".to_owned(),
                            word_count: applied.new_word_count,
                            elapsed_s: round1(elapsed),
                            note: "drafted + applied".to_owned(),
                        }
                    }
                    Err(e) => {
                        emit_progress(
                            "scene-drafter-fic",
                            "failed",
                            &format!("{} apply failed: {e}", scene.title),
                            current,
                            cap,
                            elapsed,
                        );
                        BookSceneStageResult {
                            scene_id: scene.id.to_string(),
                            scene_title: scene.title.clone(),
                            status: "failed".to_owned(),
                            word_count: 0,
                            elapsed_s: round1(elapsed),
                            note: format!("apply failed: {e}"),
                        }
                    }
                }
            }
            Ok(_) => {
                emit_progress(
                    "scene-drafter-fic",
                    "failed",
                    &format!("{}: drafter produced no output", scene.title),
                    current,
                    cap,
                    elapsed,
                );
                BookSceneStageResult {
                    scene_id: scene.id.to_string(),
                    scene_title: scene.title.clone(),
                    status: "failed".to_owned(),
                    word_count: 0,
                    elapsed_s: round1(elapsed),
                    note: "drafter produced no output".to_owned(),
                }
            }
            Err(e) => {
                emit_progress(
                    "scene-drafter-fic",
                    "failed",
                    &format!("{}: {e}", scene.title),
                    current,
                    cap,
                    elapsed,
                );
                BookSceneStageResult {
                    scene_id: scene.id.to_string(),
                    scene_title: scene.title.clone(),
                    status: "failed".to_owned(),
                    word_count: 0,
                    elapsed_s: round1(elapsed),
                    note: format!("drafter run failed: {e}"),
                }
            }
        };
        end_agent_run(
            &state,
            &app,
            "scene-drafter-fic",
            sd_run_id,
            &sd_cancel,
            sd_err,
        )
        .await;

        let cancelled = sd_cancel.is_cancelled();
        scene_results.push(stage);
        if cancelled {
            // User clicked cancel mid-pipeline — stop after the current
            // scene rolls back. Preserve everything we've already
            // applied (bibles + earlier scenes).
            break;
        }
    }

    Ok(RunBookPipelineResult {
        project_id: input.project_id,
        character_bible_status: cb_status,
        world_bible_status: wb_status,
        scenes: scene_results,
        total_elapsed_s: round1(total_start.elapsed().as_secs_f32()),
    })
}
