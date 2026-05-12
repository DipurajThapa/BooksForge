//! Tauri commands for the MZ-06 snapshot subsystem.
//!
//! Each command resolves the snapshot service against the currently-open
//! project, runs the operation, and maps the result to the IPC DTO shape.

use std::sync::Arc;

use booksforge_domain::{NodeDiff, NodeDiffKind, SnapshotRecord, SnapshotScope, SnapshotTrigger};
use booksforge_fs::{BundleFilesystem, OsFilesystem};
use booksforge_ipc::{
    BooksForgeError, NodeDiffDto, SnapshotCreateInput, SnapshotDiffInput, SnapshotDto,
    SnapshotListInput, SnapshotRestoreInput, SnapshotRestoreResult,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::StorageRepository;
use tauri::State;
use ulid::Ulid;

use crate::state::AppState;

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn resolve_service(
    state: &State<'_, AppState>,
) -> Result<Arc<SnapshotService>, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    };
    let project =
        project.ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;
    let storage: Arc<dyn StorageRepository> = project.storage.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    Ok(Arc::new(SnapshotService::new(
        storage,
        fs,
        project.bundle.clone(),
    )))
}

fn parse_scope(s: &str) -> Result<SnapshotScope, BooksForgeError> {
    match s {
        "project" => Ok(SnapshotScope::Project),
        "part" => Ok(SnapshotScope::Part),
        "chapter" => Ok(SnapshotScope::Chapter),
        "scene" => Ok(SnapshotScope::Scene),
        other => Err(BooksForgeError::validation(format!(
            "invalid scope: {other}"
        ))),
    }
}

fn parse_trigger(s: &str) -> Result<SnapshotTrigger, BooksForgeError> {
    match s {
        "manual" => Ok(SnapshotTrigger::Manual),
        "auto" => Ok(SnapshotTrigger::Auto),
        "pre_ai" => Ok(SnapshotTrigger::PreAi),
        "pre_export" => Ok(SnapshotTrigger::PreExport),
        "pre_migration" => Ok(SnapshotTrigger::PreMigration),
        "pre_agent_edit" => Ok(SnapshotTrigger::PreAgentEdit),
        "pre_restore" => Ok(SnapshotTrigger::PreRestore),
        "crash_recovery" => Ok(SnapshotTrigger::CrashRecovery),
        other => Err(BooksForgeError::validation(format!(
            "invalid trigger: {other}"
        ))),
    }
}

fn record_to_dto(r: SnapshotRecord) -> SnapshotDto {
    SnapshotDto {
        id: r.id.to_string(),
        scope: scope_str(r.scope).to_owned(),
        scope_id: r.scope_id.map(|u| u.to_string()),
        label: r.label,
        trigger: trigger_str(r.trigger).to_owned(),
        tree_hash: r.tree_hash,
        created_at: r.created_at.to_rfc3339(),
        size_bytes: r.size_bytes,
    }
}

fn scope_str(s: SnapshotScope) -> &'static str {
    match s {
        SnapshotScope::Project => "project",
        SnapshotScope::Part => "part",
        SnapshotScope::Chapter => "chapter",
        SnapshotScope::Scene => "scene",
    }
}

fn trigger_str(t: SnapshotTrigger) -> &'static str {
    match t {
        SnapshotTrigger::Manual => "manual",
        SnapshotTrigger::Auto => "auto",
        SnapshotTrigger::PreAi => "pre_ai",
        SnapshotTrigger::PreExport => "pre_export",
        SnapshotTrigger::PreMigration => "pre_migration",
        SnapshotTrigger::PreAgentEdit => "pre_agent_edit",
        SnapshotTrigger::PreRestore => "pre_restore",
        SnapshotTrigger::CrashRecovery => "crash_recovery",
    }
}

fn diff_to_dto(d: NodeDiff) -> NodeDiffDto {
    NodeDiffDto {
        node_id: d.node_id.to_string(),
        kind: match d.kind {
            NodeDiffKind::Added => "added",
            NodeDiffKind::Removed => "removed",
            NodeDiffKind::Changed => "changed",
        }
        .to_owned(),
        title: d.title,
    }
}

fn parse_ulid(s: &str, label: &str) -> Result<Ulid, BooksForgeError> {
    Ulid::from_string(s)
        .map_err(|e| BooksForgeError::validation(format!("invalid {label} ULID: {e}")))
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn snapshot_create(
    input: SnapshotCreateInput,
    state: State<'_, AppState>,
) -> Result<SnapshotDto, BooksForgeError> {
    let service = resolve_service(&state).await?;
    let scope = parse_scope(&input.scope)?;
    let trigger = parse_trigger(&input.trigger)?;
    let scope_id = input
        .scope_id
        .as_deref()
        .map(|s| parse_ulid(s, "scope_id"))
        .transpose()?;

    let record = service
        .create(scope, scope_id, input.label, trigger)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(record_to_dto(record))
}

#[tauri::command]
pub async fn snapshot_list(
    input: SnapshotListInput,
    state: State<'_, AppState>,
) -> Result<Vec<SnapshotDto>, BooksForgeError> {
    let service = resolve_service(&state).await?;
    let scope_id = input
        .scope_id
        .as_deref()
        .map(|s| parse_ulid(s, "scope_id"))
        .transpose()?;
    let records = service
        .list(scope_id)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(records.into_iter().map(record_to_dto).collect())
}

#[tauri::command]
pub async fn snapshot_diff(
    input: SnapshotDiffInput,
    state: State<'_, AppState>,
) -> Result<Vec<NodeDiffDto>, BooksForgeError> {
    let service = resolve_service(&state).await?;
    let a = parse_ulid(&input.a, "a")?;
    let b = parse_ulid(&input.b, "b")?;
    let diffs = service
        .diff(a, b)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(diffs.into_iter().map(diff_to_dto).collect())
}

#[tauri::command]
pub async fn snapshot_restore(
    input: SnapshotRestoreInput,
    state: State<'_, AppState>,
) -> Result<SnapshotRestoreResult, BooksForgeError> {
    let service = resolve_service(&state).await?;
    let snapshot_id = parse_ulid(&input.snapshot_id, "snapshot_id")?;
    let selective = match input.selective {
        None => None,
        Some(list) => {
            let mut out = Vec::with_capacity(list.len());
            for s in list {
                out.push(parse_ulid(&s, "selective[]")?);
            }
            Some(out)
        }
    };
    let report = service
        .restore(snapshot_id, selective)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(SnapshotRestoreResult {
        pre_restore_snapshot_id: report.pre_restore_snapshot_id.to_string(),
        nodes_restored: report.nodes_restored,
        scenes_restored: report.scenes_restored,
    })
}
