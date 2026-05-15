//! Tauri commands for the Ollama Setup Wizard and model management.

use std::time::Instant;

use booksforge_ipc::{
    BooksForgeError, ModelListEntry, OllamaProbeResult, PullProgressPayload, SmokeTestResult,
};
use booksforge_ollama::{
    client::OllamaClient as _,
    probe, registry,
    types::{CancelToken, GenerateOptions, GenerateRequest},
    ProgressSink,
};
use tauri::{AppHandle, Emitter as _, State};

use crate::state::AppState;

// ── ollama_probe ──────────────────────────────────────────────────────────────

/// Probe the local Ollama environment: is the API reachable, is a binary
/// installed, how much RAM does the machine have?
#[tauri::command]
pub async fn ollama_probe(
    state: State<'_, AppState>,
) -> Result<OllamaProbeResult, BooksForgeError> {
    let version_result = state.ollama.version().await;
    let (api_reachable, version) = match version_result {
        Ok(v) => (true, Some(v.version)),
        Err(_) => (false, None),
    };

    let binary_path = probe::find_binary();
    let binary_found = binary_path.is_some();
    let ram_gb = probe::available_ram_gb();

    Ok(OllamaProbeResult {
        api_reachable,
        version,
        binary_found,
        ram_gb,
    })
}

// ── ollama_launch ─────────────────────────────────────────────────────────────

/// Attempt to start the Ollama application or background service.
/// Returns immediately — the caller should poll `ollama_probe` to wait for
/// the API to become ready.
#[tauri::command]
pub async fn ollama_launch() -> Result<(), BooksForgeError> {
    probe::launch_ollama()
        .map_err(|e| BooksForgeError::internal(format!("could not launch Ollama: {e}")))
}

// ── ollama_list_models ────────────────────────────────────────────────────────

/// List all curated models merged with locally installed models.
/// The list is filtered to `official = true` entries only (no smoke-only
/// models are shown in the picker).
#[tauri::command]
pub async fn ollama_list_models(
    state: State<'_, AppState>,
) -> Result<Vec<ModelListEntry>, BooksForgeError> {
    // Get installed models from Ollama (empty list if offline).
    let installed = state.ollama.list_local_models().await.unwrap_or_default();

    let entries = registry::official_models()
        .map(|m| {
            let local = installed.iter().find(|lm| lm.name == m.id);
            ModelListEntry {
                id: m.id.clone(),
                display_name: m.display_name.clone(),
                family: m.family.clone(),
                size_bytes: m.size_bytes,
                ram_min_gb: m.ram_min_gb,
                context_window: m.context_window,
                recommended_for: m.recommended_for.clone(),
                strengths: m.strengths.clone(),
                notes: m.notes.clone(),
                default_for_modes: m.default_for_modes.clone(),
                official: m.official,
                is_installed: local.is_some(),
                digest: local.map(|lm| lm.digest.clone()),
            }
        })
        .collect();

    Ok(entries)
}

// ── ollama_pull ───────────────────────────────────────────────────────────────

/// Pull a model from the Ollama registry.  Emits `"ollama:pull-progress"`
/// events to the frontend for each NDJSON line received.
///
/// The command blocks until the pull completes or fails.
#[tauri::command]
pub async fn ollama_pull(
    model: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), BooksForgeError> {
    let model_clone = model.clone();
    let app_clone = app_handle.clone();

    let sink: ProgressSink = Box::new(move |p| {
        let payload = PullProgressPayload {
            model: model_clone.clone(),
            status: p.status,
            completed: p.completed,
            total: p.total,
        };
        // Best-effort emit — ignore errors (window may have been closed).
        let _ = app_clone.emit("ollama:pull-progress", &payload);
    });

    state
        .ollama
        .pull(&model, sink)
        .await
        .map_err(|e| BooksForgeError::internal(format!("pull failed: {e}")))
}

// ── ollama_smoke_test ─────────────────────────────────────────────────────────

const SMOKE_PROMPT: &str = "Reply with exactly three words: I am working.";

/// Run a tiny generation against the chosen model to verify it is functional.
/// Uses a fixed prompt and a very low token budget.
#[tauri::command]
pub async fn ollama_smoke_test(
    model: String,
    state: State<'_, AppState>,
) -> Result<SmokeTestResult, BooksForgeError> {
    let start = Instant::now();

    let request = GenerateRequest {
        model: model.clone(),
        prompt: SMOKE_PROMPT.to_owned(),
        system: None,
        stream: true,
        think: None,
        // Smoke test wants a free-form short response — leave format unset.
        format: None,
        options: Some(GenerateOptions {
            temperature: Some(0.0),
            top_p: None,
            num_ctx: None,
            num_predict: Some(32), // hard cap — prevents runaway output
            // Smoke test — keep model defaults.
            repeat_penalty: None,
            stop: None,
        }),
    };

    // Collect streamed tokens via shared state.
    let tokens = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let tokens_clone = tokens.clone();
    let sink: booksforge_ollama::TokenSink = Box::new(move |token: &str| {
        if let Ok(mut buf) = tokens_clone.lock() {
            buf.push_str(token);
        }
    });

    let cancel = CancelToken::new();

    match state.ollama.generate(request, sink, cancel).await {
        Ok(outcome) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            // Prefer the final assembled response from the outcome.
            let response = outcome.response.trim().to_owned();
            let success = !response.is_empty();
            Ok(SmokeTestResult {
                success,
                response: Some(response).filter(|s| !s.is_empty()),
                duration_ms,
                error: None,
            })
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(SmokeTestResult {
                success: false,
                response: None,
                duration_ms,
                error: Some(e.to_string()),
            })
        }
    }
}
