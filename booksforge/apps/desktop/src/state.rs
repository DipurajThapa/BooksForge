//! Shared app state injected into Tauri command handlers.

use booksforge_ollama::HttpOllamaClient;

/// App-wide state managed by Tauri.
pub struct AppState {
    pub ollama: HttpOllamaClient,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ollama: HttpOllamaClient::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
