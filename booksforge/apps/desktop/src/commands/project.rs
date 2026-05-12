//! Tauri commands for project lifecycle: create, open, close, recent.

use std::{path::Path, sync::Arc};

use booksforge_domain::project::BookMode;
use booksforge_domain::settings::RecentProject;
use booksforge_domain::{BookKind, Node, NodeKind, NodeStatus};
use booksforge_fs::{
    manifest::BundleManifest, settings, traits::OsFilesystem, BundleFilesystem, FsError,
};
use booksforge_ipc::{
    project::{
        CreateProjectInput, OpenProjectInput, OpenProjectResult, ProjectBriefDto,
        ProjectBriefSaveInput, ProjectKindSetInput, ProjectKindSetResult, RecentProjectEntry,
        RecentRemoveInput,
    },
    BooksForgeError,
};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use booksforge_vocab::all_starter_entries;
use chrono::Utc;
use tauri::State;
use ulid::Ulid;

use crate::state::{AppState, OpenProject};

// ── project_create ────────────────────────────────────────────────────────────

/// Create a new project bundle and open it.
#[tauri::command]
pub async fn project_create(
    input: CreateProjectInput,
    state: State<'_, AppState>,
) -> Result<OpenProjectResult, BooksForgeError> {
    let final_path = Path::new(&input.bundle_path).to_path_buf();

    // Phase 4 of PRODUCT_ROADMAP_E2E.md — the wizard supplies `book_kind`
    // upfront. Forgiving parse via `BookKind::from_str` accepts kebab,
    // snake, and bare aliases. None is acceptable (the onboarding overlay
    // catches it post-create).
    let book_kind: Option<BookKind> = input.book_kind.as_deref().and_then(BookKind::from_str);

    // Map the chosen book_kind to the legacy BookMode for the manifest's
    // `mode` field. NonFiction → NonFiction; everything else (literary /
    // genre / memoir / childrens) → Fiction. The mode is kept for
    // backwards compatibility; the workflow router uses book_kind.
    let mode = match book_kind {
        Some(BookKind::NonFiction) => BookMode::NonFiction,
        Some(BookKind::Memoir) => BookMode::Memoir,
        _ => BookMode::Fiction,
    };

    let manifest = BundleManifest::new(
        input.title.clone(),
        input.author.clone(),
        input.genre,
        mode,
        book_kind,
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

    // Seed the project root node so the document tree has an anchor for
    // template seeding, outline application, and binder rendering. Without
    // this, the frontend's `applyTemplate` (and any agent that walks the
    // tree) fails with "project root node missing." Idempotent: re-opening
    // an existing project finds the root already and skips this step.
    let has_root = storage
        .list_nodes()
        .await
        .map(|nodes| nodes.iter().any(|n| n.kind == NodeKind::Project))
        .unwrap_or(false);
    if !has_root {
        let now = Utc::now();
        let root = Node {
            id: Ulid::new(),
            parent_id: None,
            kind: NodeKind::Project,
            title: input.title.clone(),
            position: Node::DEFAULT_POSITION.to_owned(),
            status: NodeStatus::Planned,
            pov: None,
            beat: None,
            target_words: None,
            synopsis: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        storage
            .insert_node(&root)
            .await
            .map_err(|e| BooksForgeError::internal(format!("seed project root: {e}")))?;
    }

    // Seed the shipped vocabulary dictionaries (Phase 3).  This is an
    // idempotent upsert keyed by `(layer, term, kind)` — re-opening an
    // existing project re-runs it harmlessly so future template updates
    // can land on top of older bundles.
    if let Ok(starters) = all_starter_entries() {
        let _ = storage.vocab_seed_starters(&starters).await;
    }

    // Update recent projects list.
    let mut settings = settings::load_settings().await.unwrap_or_default();
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
        book_kind: book_kind.map(|k| k.as_str().to_owned()),
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
        // Surface the manifest's `book_kind` (None for projects created
        // before Phase 4 — the UI catches this and shows the onboarding
        // overlay so the user picks before the first agent run).
        book_kind: manifest.project.book_kind.map(|k| k.as_str().to_owned()),
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

// ── project_kind_set (Phase 4 / 5B of PRODUCT_ROADMAP_E2E.md) ───────────────

/// Update the open project's `book_kind` (manifest.toml field).
/// Called from the SettingsPanel and the onboarding overlay.
#[tauri::command]
pub async fn project_kind_set(
    input: ProjectKindSetInput,
    state: State<'_, AppState>,
) -> Result<ProjectKindSetResult, BooksForgeError> {
    let book_kind = BookKind::from_str(&input.book_kind).ok_or_else(|| {
        BooksForgeError::validation(format!("unknown book kind: {}", input.book_kind))
    })?;
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let mut manifest = BundleManifest::read_from_bundle(&project.bundle)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    manifest.set_book_kind(book_kind);
    let toml = manifest
        .to_toml()
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let manifest_path = project.bundle.manifest();
    booksforge_fs::atomic::atomic_write(&manifest_path, toml.as_bytes())
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ProjectKindSetResult {
        project_id: project.project_id.clone(),
        book_kind: book_kind.as_str().to_owned(),
    })
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

// ── project_recent_remove ────────────────────────────────────────────────────

/// Remove a single entry from the recent-projects list.  Does NOT
/// delete the bundle on disk — only removes the row from
/// `~/.booksforge/settings.toml` so it stops appearing in the picker.
/// Returns the post-removal list so the UI can re-render in one round-trip.
#[tauri::command]
pub async fn project_recent_remove(
    input: RecentRemoveInput,
) -> Result<Vec<RecentProjectEntry>, BooksForgeError> {
    let mut settings = settings::load_settings().await.unwrap_or_default();
    settings.recent_projects.remove(&input.path);
    settings::save_settings(&settings)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

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

// ── project_brief_load / project_brief_save ───────────────────────────────────

/// Read the open project's `ProjectBrief` from book-scope memory
/// (key `project_brief`). Returns `loaded: false` and an empty object
/// when no brief has been saved yet (the project is pre-intake).
#[tauri::command]
pub async fn project_brief_load(
    state: State<'_, AppState>,
) -> Result<ProjectBriefDto, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let entry = project
        .storage
        .memory_get(booksforge_domain::MemoryScope::Book, "project_brief")
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(match entry {
        Some(e) => ProjectBriefDto {
            loaded: true,
            brief_json: e.value_json,
            source: e.agent_id,
            updated_at: e.updated_at.to_rfc3339(),
        },
        None => ProjectBriefDto {
            loaded: false,
            brief_json: serde_json::json!({}),
            source: String::new(),
            updated_at: String::new(),
        },
    })
}

/// Persist a manually-edited `ProjectBrief` to book-scope memory.
/// Validated against the typed `ProjectBrief` shape before write so a
/// malformed brief from the UI never poisons the orchestrator's
/// `creative_profile` injection. Round 5 of PRODUCT_ROADMAP_E2E.md.
#[tauri::command]
pub async fn project_brief_save(
    input: ProjectBriefSaveInput,
    state: State<'_, AppState>,
) -> Result<ProjectBriefDto, BooksForgeError> {
    // Validate by round-tripping through the typed shape — refuses
    // unknown shapes early rather than letting the orchestrator
    // discover them mid-run.
    let parsed: booksforge_domain::ProjectBrief = serde_json::from_value(input.brief_json.clone())
        .map_err(|e| BooksForgeError::validation(format!("brief shape rejected: {e}")))?;
    parsed
        .validate()
        .map_err(|e| BooksForgeError::validation(e.to_owned()))?;
    let canonical =
        serde_json::to_value(&parsed).map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let now = Utc::now();
    let entry = booksforge_domain::MemoryEntry {
        id: Ulid::new(),
        scope: booksforge_domain::MemoryScope::Book,
        key: "project_brief".to_owned(),
        value_json: canonical.clone(),
        // `user-edit` makes the audit-ledger origin visible — distinct
        // from `intake` (the auto-extracted brief).
        agent_id: "user-edit".to_owned(),
        created_at: now,
        updated_at: now,
    };
    project
        .storage
        .memory_upsert(&entry)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ProjectBriefDto {
        loaded: true,
        brief_json: canonical,
        source: entry.agent_id,
        updated_at: entry.updated_at.to_rfc3339(),
    })
}
