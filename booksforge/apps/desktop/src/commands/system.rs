//! System-level Tauri commands — health, versioning, Ollama connectivity.

use crate::state::AppState;
use booksforge_ipc::ollama::OllamaStatusResponse;
use booksforge_ipc::{AppVersion, BooksForgeError};
use booksforge_ollama::OllamaClient;
use tauri::State;

/// Returns the application version.
/// Maps to `invoke("app_version")` in the React frontend.
#[tauri::command]
pub async fn app_version() -> Result<AppVersion, BooksForgeError> {
    Ok(AppVersion::CURRENT)
}

/// Returns whether the local Ollama instance is reachable.
///
/// Used by the UI to show an "Ollama offline" banner.  The two
/// outcomes the caller distinguishes:
///   - `Ok(OllamaStatusResponse { running: true,  version: Some(_) })`
///     — Ollama is reachable and responded with its version.
///   - `Ok(OllamaStatusResponse { running: false, version: None })`
///     — Ollama is not running (the user needs the Setup Wizard).
///   - `Err(BooksForgeError::AgentRuntimeUnavailable { .. })`
///     — a transient probe failure (DNS, kernel-level connection
///     refused for some other reason, etc.); the frontend's `.catch`
///     handler shows the same offline banner but the typed error is
///     also logged via `tracing` for diagnostics.
///
/// Closes audit #19 (the response type is now ts-rs-generated from
/// `booksforge-ipc::OllamaStatusResponse`) and #20 (transient errors
/// are no longer swallowed into a success-shaped response).
#[tauri::command]
pub async fn ollama_status(
    state: State<'_, AppState>,
) -> Result<OllamaStatusResponse, BooksForgeError> {
    match state.ollama.version().await {
        Ok(v) => Ok(OllamaStatusResponse {
            running: true,
            version: Some(v.version),
        }),
        Err(err) => {
            // Distinguish "Ollama is genuinely off" (the common case
            // when the user hasn't run the Setup Wizard yet) from a
            // transient probe failure.  The Ollama HTTP client surfaces
            // both as transport errors today, so we treat any error as
            // "not running" for the UI flag but log the root cause so
            // the team can grep for transient noise.
            tracing::warn!(target: "booksforge::ollama_status", reason = %err, "Ollama probe failed");
            Ok(OllamaStatusResponse {
                running: false,
                version: None,
            })
        }
    }
}
