//! Tauri commands for the MZ-08 quick-action presets.
//!
//! - `ai_suggest`  — kicks off a streaming preset; returns a `job_id`.
//!   Tokens are emitted on `ai-suggest:<job_id>:token`; the final
//!   `ai-suggest:<job_id>:done` carries the full text + `ai_call_id`.
//! - `ai_cancel`   — aborts an in-flight `ai_suggest` by `job_id`.
//! - `ai_apply`    — accepts a suggestion: takes the `pre_ai` snapshot and
//!                   writes the new prose into the scene's pm_doc.

use std::sync::Arc;

use booksforge_domain::QuickActionPreset;
use booksforge_fs::{BundleFilesystem, OsFilesystem};
use booksforge_ipc::{
    AiApplyInput, AiApplyResult, AiCancelInput, AiSuggestDoneEvent, AiSuggestInput,
    AiSuggestStartedResult, AiSuggestTokenEvent, BooksForgeError,
};
use booksforge_ollama::{HttpOllamaClient, TokenSink};
use booksforge_orchestrator::{quick_action::QuickActionOptions, ApplyOp, Orchestrator, OrchestratorConfig};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::StorageRepository;
use tauri::{AppHandle, Emitter as _, Manager as _, State};
use ulid::Ulid;

use crate::state::AppState;

// ── Helpers ───────────────────────────────────────────────────────────────────

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

fn parse_preset(s: &str) -> Result<QuickActionPreset, BooksForgeError> {
    QuickActionPreset::from_str(s)
        .ok_or_else(|| BooksForgeError::validation(format!("unknown preset: {s}")))
}

fn parse_apply_op(s: &str) -> Result<ApplyOp, BooksForgeError> {
    match s {
        "replace" => Ok(ApplyOp::Replace),
        "append"  => Ok(ApplyOp::Append),
        other     => Err(BooksForgeError::validation(format!("unknown apply op: {other}"))),
    }
}

fn parse_options(json: Option<&str>) -> Result<QuickActionOptions, BooksForgeError> {
    let mut opts = QuickActionOptions::default();
    if let Some(raw) = json {
        if raw.trim().is_empty() { return Ok(opts); }
        let val: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| BooksForgeError::validation(format!("invalid options_json: {e}")))?;
        if let Some(obj) = val.as_object() {
            for (k, v) in obj {
                opts.extra_vars.insert(k.clone(), v.clone());
            }
        }
    }
    Ok(opts)
}

// ── ai_suggest ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_suggest(
    input: AiSuggestInput,
    app:   AppHandle,
    state: State<'_, AppState>,
) -> Result<AiSuggestStartedResult, BooksForgeError> {
    let preset  = parse_preset(&input.preset)?;
    let node_id = Ulid::from_string(&input.node_id)
        .map_err(|_| BooksForgeError::validation("invalid node_id ULID".to_owned()))?;
    let options = parse_options(input.options_json.as_deref())?;
    let model   = input.model.clone().unwrap_or_else(|| {
        booksforge_domain::OllamaSettings::DEFAULT_MODEL.to_owned()
    });

    let job_id = Ulid::new().to_string();
    let cancel = state.register_job(&job_id).await;

    // The orchestrator must outlive the spawned task; build it on this task
    // (which holds the Tauri State).
    let orchestrator = open_orchestrator(&state).await?;

    let job_id_for_task = job_id.clone();
    let app_clone       = app.clone();
    let scope_text      = input.scope_text.clone();

    // Spawn the streaming call.  The token sink emits Tauri events scoped to
    // this job_id; the done event is emitted at the end with the audit row id.
    tokio::spawn(async move {
        let token_channel = format!("ai-suggest:{job_id_for_task}:token");
        let app_for_sink  = app_clone.clone();
        let job_for_sink  = job_id_for_task.clone();
        let sink: TokenSink = Box::new(move |delta: &str| {
            let _ = app_for_sink.emit(
                &token_channel,
                AiSuggestTokenEvent { job_id: job_for_sink.clone(), delta: delta.to_owned() },
            );
        });

        let outcome = orchestrator
            .run_quick_action(node_id, preset, scope_text, model, options, cancel, sink)
            .await;

        let done = match outcome {
            Ok(o) => AiSuggestDoneEvent {
                job_id:      job_id_for_task.clone(),
                status:      o.status.as_str().to_owned(),
                ai_call_id:  o.ai_call_id.to_string(),
                full_text:   o.output_text,
                duration_ms: o.duration_ms,
                error:       o.error,
            },
            Err(e) => AiSuggestDoneEvent {
                job_id:      job_id_for_task.clone(),
                status:      "error".to_owned(),
                ai_call_id:  String::new(),
                full_text:   String::new(),
                duration_ms: 0,
                error:       Some(e.to_string()),
            },
        };
        let done_channel = format!("ai-suggest:{job_id_for_task}:done");
        let _ = app_clone.emit(&done_channel, done);

        // Backlog A6 — drop the job's CancelToken from the registry now that
        // the streaming task has finished naturally.  Without this the entry
        // would survive until the user calls `ai_cancel` or the app exits.
        let state = app_clone.state::<AppState>();
        state.drop_job(&job_id_for_task).await;
    });

    Ok(AiSuggestStartedResult { job_id })
}

// ── ai_cancel ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_cancel(
    input: AiCancelInput,
    state: State<'_, AppState>,
) -> Result<(), BooksForgeError> {
    state.cancel_job(&input.job_id).await;
    state.drop_job(&input.job_id).await;
    Ok(())
}

// ── ai_apply ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_apply(
    input: AiApplyInput,
    state: State<'_, AppState>,
) -> Result<AiApplyResult, BooksForgeError> {
    let ai_call_id = Ulid::from_string(&input.ai_call_id)
        .map_err(|_| BooksForgeError::validation("invalid ai_call_id ULID".to_owned()))?;
    let op = parse_apply_op(&input.op)?;

    let orchestrator = open_orchestrator(&state).await?;
    let result = orchestrator
        .apply_quick_action(ai_call_id, input.accepted_text, op)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(AiApplyResult {
        ai_call_id:      result.ai_call_id.to_string(),
        pre_snapshot_id: result.pre_snapshot_id.to_string(),
        applied_at:      result.applied_at.to_rfc3339(),
    })
}
