//! Tauri commands for project lifecycle: create, open, close, recent.

use std::{path::Path, sync::Arc};

use booksforge_domain::project::BookMode;
use booksforge_domain::settings::RecentProject;
use booksforge_fs::{
    manifest::BundleManifest, settings, traits::OsFilesystem, BundleFilesystem, FsError,
};
use booksforge_ipc::{
    project::{CreateProjectInput, OpenProjectInput, OpenProjectResult, RecentProjectEntry},
    BooksForgeError,
};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use booksforge_vocab::all_starter_entries;
use chrono::Utc;
use tauri::State;

use crate::state::{AppState, OpenProject};

// ── project_create ────────────────────────────────────────────────────────────

/// Create a new project bundle and open it.
#[tauri::command]
pub async fn project_create(
    input: CreateProjectInput,
    state: State<'_, AppState>,
) -> Result<OpenProjectResult, BooksForgeError> {
    let final_path = Path::new(&input.bundle_path).to_path_buf();

    let manifest = BundleManifest::new(
        input.title.clone(),
        input.author.clone(),
        input.genre,
        BookMode::Fiction, // user chooses in step 2 of wizard; default for now
    );
    let manifest_toml = manifest
        .to_toml()
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let project_id = manifest.project.id.clone();

    let fs = OsFilesystem;
    let bundle = fs
        .create_bundle(
            &final_path,
            &manifest_toml,
            Box::new(move |db_path| {
                Box::pin(async move {
                    let pool = open_pool(&db_path)
                        .await
                        .map_err(|e| FsError::Serialization(e.to_string()))?;
                    run_migrations(&pool)
                        .await
                        .map_err(|e| FsError::Serialization(e.to_string()))?;
                    Ok(())
                })
            }),
        )
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // Open the newly created bundle.
    let bundle_root = bundle.root().to_path_buf();
    let (bundle, lock) = OsFilesystem
        .open_bundle(&bundle_root)
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let pool = open_pool(&bundle.db())
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let storage = Arc::new(SqliteStorage::new(pool));

    // Seed the shipped vocabulary dictionaries (Phase 3).  This is an
    // idempotent upsert keyed by `(layer, term, kind)` — re-opening an
    // existing project re-runs it harmlessly so future template updates
    // can land on top of older bundles.
    if let Ok(starters) = all_starter_entries() {
        let _ = storage.vocab_seed_starters(&starters).await;
    }

    // Update recent projects list.
    let mut settings = settings::load_settings()
        .await
        .unwrap_or_default();
    settings.recent_projects.touch(RecentProject {
        id: project_id.clone(),
        path: input.bundle_path.clone(),
        name: input.title.clone(),
        last_opened: Utc::now(),
    });
    let _ = settings::save_settings(&settings).await;

    let result = OpenProjectResult {
        project_id: project_id.clone(),
        title: input.title.clone(),
        author: input.author.clone(),
        bundle_path: input.bundle_path.clone(),
    };

    *state.open_project.lock().await = Some(Arc::new(OpenProject {
        bundle,
        storage,
        _lock: lock,
        project_id,
        title: input.title,
        author: input.author,
    }));

    Ok(result)
}

// ── project_open ─────────────────────────────────────────────────────────────

/// Open an existing bundle.
#[tauri::command]
pub async fn project_open(
    input: OpenProjectInput,
    state: State<'_, AppState>,
) -> Result<OpenProjectResult, BooksForgeError> {
    let path = Path::new(&input.bundle_path);

    let (bundle, lock) = OsFilesystem
        .open_bundle(path)
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let manifest = BundleManifest::read_from_bundle(&bundle)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let pool = open_pool(&bundle.db())
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let storage = Arc::new(SqliteStorage::new(pool));

    let author = manifest.meta.authors.first().cloned().unwrap_or_default();
    let project_id = manifest.project.id.clone();
    let title = manifest.meta.title.clone();

    // Update recent projects.
    let mut settings = settings::load_settings().await.unwrap_or_default();
    settings.recent_projects.touch(RecentProject {
        id: project_id.clone(),
        path: input.bundle_path.clone(),
        name: title.clone(),
        last_opened: Utc::now(),
    });
    let _ = settings::save_settings(&settings).await;

    let result = OpenProjectResult {
        project_id: project_id.clone(),
        title: title.clone(),
        author: author.clone(),
        bundle_path: input.bundle_path.clone(),
    };

    *state.open_project.lock().await = Some(Arc::new(OpenProject {
        bundle,
        storage,
        _lock: lock,
        project_id,
        title,
        author,
    }));

    Ok(result)
}

// ── project_close ─────────────────────────────────────────────────────────────

/// Release the open project's lock and drop its database connection.
#[tauri::command]
pub async fn project_close(state: State<'_, AppState>) -> Result<(), BooksForgeError> {
    *state.open_project.lock().await = None;
    Ok(())
}

// ── project_recent ────────────────────────────────────────────────────────────

/// Return the recent-projects list with `missing` flags for stale paths.
#[tauri::command]
pub async fn project_recent() -> Result<Vec<RecentProjectEntry>, BooksForgeError> {
    let settings = settings::load_settings().await.unwrap_or_default();
    let entries = settings
        .recent_projects
        .entries
        .into_iter()
        .map(|e| {
            let missing = !Path::new(&e.path).exists();
            RecentProjectEntry {
                id: e.id,
                name: e.name,
                last_opened: e.last_opened.to_rfc3339(),
                missing,
                path: e.path,
            }
        })
        .collect();
    Ok(entries)
}
