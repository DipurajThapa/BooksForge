//! Background-task scheduler for the desktop app.
//!
//! Currently hosts a single task: the hourly auto-snapshot loop (D7).
//! Spun up from `lib.rs::run` once Tauri's setup callback fires so the
//! task has a live `AppHandle` for state access.
//!
//! Design notes:
//!   - Cheap atomics (`last_change_at`, `last_auto_snap_at`) decide
//!     whether to fire — no DB round-trip on idle ticks.
//!   - The interval is configurable for tests; production uses 1 hour.
//!   - A failure (no project open, snapshot service unavailable) is a
//!     no-op for that tick, never a crash.

use std::sync::Arc;
use std::time::Duration;

use booksforge_domain::{SnapshotScope, SnapshotTrigger};
use booksforge_fs::{BundleFilesystem, OsFilesystem};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::StorageRepository;
use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Default interval between auto-snapshot ticks: 1 hour.
pub const AUTO_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Spawn the hourly auto-snapshot loop.  Idempotent — calling more than
/// once produces independent loops, but `lib.rs::run` calls it exactly
/// once from the Tauri setup callback.
pub fn spawn_auto_snapshot_task(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        run_auto_snapshot_loop(app, AUTO_SNAPSHOT_INTERVAL).await;
    });
}

/// Drives the loop.  Public for direct invocation from tests with a
/// shorter interval — the tick handler itself is `tick_once` below.
pub async fn run_auto_snapshot_loop(app: AppHandle, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    // Skip the immediate-fire tick — we don't want to snapshot at
    // startup before the user has done anything.
    ticker.tick().await;
    loop {
        ticker.tick().await;
        if let Err(e) = tick_once(&app).await {
            tracing::warn!(error = %e, "auto-snapshot tick failed");
        }
    }
}

/// One scheduler iteration.  Skips silently when nothing should happen.
pub async fn tick_once(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    if !state.dirty_since_last_auto_snap() {
        return Ok(());
    }

    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    };
    let Some(project) = project else {
        return Ok(());
    };

    let storage_arc = Arc::clone(&project.storage);
    let storage_trait: Arc<dyn StorageRepository> = storage_arc.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    let svc = SnapshotService::new(storage_trait, fs, project.bundle.clone());

    let label = Some(format!(
        "hourly auto · {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M")
    ));
    svc.create(SnapshotScope::Project, None, label, SnapshotTrigger::Auto)
        .await
        .map_err(|e| format!("snapshot create failed: {e}"))?;

    state.mark_auto_snap();
    tracing::info!("auto-snapshot taken (hourly)");
    Ok(())
}
