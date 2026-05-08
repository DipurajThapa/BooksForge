//! Shared app state injected into Tauri command handlers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use booksforge_fs::{BundleLock, BundlePath};
use booksforge_ollama::{types::CancelToken, HttpOllamaClient};
use booksforge_storage::SqliteStorage;

/// App-wide state managed by Tauri.
pub struct AppState {
    pub ollama: HttpOllamaClient,
    /// The currently-open project, if any.
    /// Wrapped in `Arc` so commands can clone the pointer without holding
    /// the `Mutex` across await points.
    pub open_project: Mutex<Option<Arc<OpenProject>>>,
    /// In-flight long-running jobs keyed by `job_id`.  Used by the
    /// `ai_suggest` / `ai_cancel` IPC pair (MZ-08) so any streaming call can
    /// be aborted from the frontend.  Cleared automatically when a job
    /// completes; survivors at app exit are dropped harmlessly.
    pub jobs: Mutex<HashMap<String, CancelToken>>,
    /// Unix-seconds timestamp of the most recent manuscript-touching IPC
    /// (currently `scene_save`).  Read by the scheduled-snapshot task to
    /// decide whether the user has been editing since the last hourly auto
    /// snapshot.  Defaults to 0 — meaning "no activity recorded yet".
    pub last_change_at:    Arc<AtomicI64>,
    /// Unix-seconds timestamp of the last successful `Auto`-triggered
    /// snapshot.  Only incremented by the scheduler.
    pub last_auto_snap_at: Arc<AtomicI64>,
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
            ollama:            HttpOllamaClient::new(),
            open_project:      Mutex::new(None),
            jobs:              Mutex::new(HashMap::new()),
            last_change_at:    Arc::new(AtomicI64::new(0)),
            last_auto_snap_at: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Record a manuscript-touching event.  Cheap atomic store — safe to
    /// call from every `scene_save` IPC.
    pub fn touch(&self) {
        let now = chrono::Utc::now().timestamp();
        self.last_change_at.store(now, Ordering::Relaxed);
    }

    /// Whether there has been any activity since the last auto-snapshot.
    pub fn dirty_since_last_auto_snap(&self) -> bool {
        let changed = self.last_change_at.load(Ordering::Relaxed);
        let snapped = self.last_auto_snap_at.load(Ordering::Relaxed);
        changed > 0 && changed > snapped
    }

    /// Mark the auto-snapshot timestamp.  Called by the scheduler after a
    /// successful Auto-triggered capture.
    pub fn mark_auto_snap(&self) {
        let now = chrono::Utc::now().timestamp();
        self.last_auto_snap_at.store(now, Ordering::Relaxed);
    }

    /// Reserve a fresh CancelToken for `job_id`.  Callers should `drop_job`
    /// it once the job is done.
    pub async fn register_job(&self, job_id: &str) -> CancelToken {
        let token = CancelToken::new();
        self.jobs.lock().await.insert(job_id.to_owned(), token.clone());
        token
    }

    /// Cancel a job by id.  Idempotent — unknown ids are silent no-ops.
    pub async fn cancel_job(&self, job_id: &str) {
        if let Some(t) = self.jobs.lock().await.get(job_id) {
            t.cancel();
        }
    }

    /// Forget a job's CancelToken.  Called on completion regardless of
    /// outcome.
    pub async fn drop_job(&self, job_id: &str) {
        self.jobs.lock().await.remove(job_id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
