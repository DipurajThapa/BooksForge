//! Shared app state injected into Tauri command handlers.

use std::sync::Arc;
use tokio::sync::Mutex;

use booksforge_fs::{BundleLock, BundlePath};
use booksforge_ollama::HttpOllamaClient;
use booksforge_storage::SqliteStorage;

/// App-wide state managed by Tauri.
pub struct AppState {
    pub ollama: HttpOllamaClient,
    /// The currently-open project, if any.
    /// Wrapped in `Arc` so commands can clone the pointer without holding
    /// the `Mutex` across await points.
    pub open_project: Mutex<Option<Arc<OpenProject>>>,
}

/// State held while a project is open.
pub struct OpenProject {
    pub bundle: BundlePath,
    pub storage: Arc<SqliteStorage>,
    /// Advisory lock released when the project is closed (drops with the Arc).
    pub _lock: BundleLock,
    pub project_id: String,
    pub title: String,
    pub author: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ollama: HttpOllamaClient::new(),
            open_project: Mutex::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
