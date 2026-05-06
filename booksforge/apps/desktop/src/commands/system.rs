//! System-level Tauri commands — health, versioning, Ollama connectivity.

use booksforge_ipc::{AppVersion, BooksForgeError};
use booksforge_ollama::OllamaClient;
use tauri::State;
use crate::state::AppState;

/// Returns the application version.
/// Maps to `invoke("app_version")` in the React frontend.
#[tauri::command]
pub async fn app_version() -> Result<AppVersion, BooksForgeError> {
    Ok(AppVersion::CURRENT)
}

/// Returns whether the local Ollama instance is reachable.
/// Used by the UI to show an "Ollama offline" banner.
#[tauri::command]
pub async fn ollama_status(state: State<'_, AppState>) -> Result<OllamaStatusResponse, BooksForgeError> {
    match state.ollama.version().await {
        Ok(v) => Ok(OllamaStatusResponse {
            running: true,
            version: Some(v.version),
        }),
        Err(_) => Ok(OllamaStatusResponse {
            running: false,
            version: None,
        }),
    }
}

/// Returned by `ollama_status`.
#[derive(Debug, serde::Serialize)]
pub struct OllamaStatusResponse {
    pub running: bool,
    pub version: Option<String>,
}
