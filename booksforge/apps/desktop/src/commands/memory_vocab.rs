//! Tauri commands for the memory + vocabulary subsystems.

use booksforge_domain::{
    memory::MemoryError,
    vocab::{EntryKind, EntrySource},
    MemoryEntry, MemoryScope, VocabEntry,
};
use booksforge_ipc::{
    BooksForgeError, MemoryDeleteInput, MemoryEntryDto, MemoryListInput, MemoryUpsertInput,
    VocabEntryDto, VocabListInput, VocabUpsertInput,
};
use booksforge_storage::StorageRepository;
use chrono::Utc;
use tauri::State;
use ulid::Ulid;

use crate::state::AppState;

/// `agent_id` we attribute manual user writes to.  Distinct from the agent
/// names in `booksforge_domain::memory::allowed_write_scopes`, so the audit
/// ledger can tell user-curated entries from agent-curated ones.
const USER_AGENT_ID: &str = "user";

// ── Memory ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn memory_list(
    input: MemoryListInput,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntryDto>, BooksForgeError> {
    let project = require_project(&state).await?;
    let scope = MemoryScope::from_str(&input.scope)
        .ok_or_else(|| BooksForgeError::validation(format!("invalid memory scope: {}", input.scope)))?;
    let entries = project.storage.memory_list_by_scope(scope).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(entries.into_iter().map(memory_to_dto).collect())
}

// ── Vocabulary ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn vocab_list(
    input: VocabListInput,
    state: State<'_, AppState>,
) -> Result<Vec<VocabEntryDto>, BooksForgeError> {
    let project = require_project(&state).await?;
    let layers: Vec<&str> = input.layers.iter().map(|s| s.as_str()).collect();
    let entries = project.storage.vocab_list_by_layers(&layers).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(entries.into_iter().map(vocab_to_dto).collect())
}

// ── Manual CRUD (audit #30) ──────────────────────────────────────────────────
//
// User-driven memory + vocabulary edits.  Distinct from agent writes:
// `agent_id = "user"` and the per-agent scope authorisation in
// `booksforge_domain::memory::authorise_write` is bypassed (the user owns
// every scope).  The audit ledger preserves authorship via `agent_id` so
// reviewers can tell user-authored entries from agent-curated ones.

#[tauri::command]
pub async fn memory_upsert(
    input: MemoryUpsertInput,
    state: State<'_, AppState>,
) -> Result<MemoryEntryDto, BooksForgeError> {
    let project = require_project(&state).await?;
    let scope = MemoryScope::from_str(&input.scope)
        .ok_or_else(|| BooksForgeError::validation(format!("invalid memory scope: {}", input.scope)))?;
    if input.key.trim().is_empty() {
        return Err(BooksForgeError::validation("memory key cannot be empty".to_owned()));
    }
    let value_json: serde_json::Value = serde_json::from_str(&input.value_json)
        .map_err(|e| BooksForgeError::validation(format!("value_json is not valid JSON: {e}")))?;

    // Distinguish edit vs create by id presence + lookup.
    let now = Utc::now();
    let (id, created_at) = if let Some(id_str) = &input.id {
        let id = Ulid::from_string(id_str)
            .map_err(|e| BooksForgeError::validation(format!("invalid memory id: {e}")))?;
        // Preserve created_at on edit by reading the existing row.
        let existing = project.storage.memory_get(scope, &input.key).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        let created = existing.map(|e| e.created_at).unwrap_or(now);
        (id, created)
    } else {
        (Ulid::new(), now)
    };

    let entry = MemoryEntry {
        id,
        scope,
        key: input.key.clone(),
        value_json,
        agent_id: USER_AGENT_ID.to_owned(),
        created_at,
        updated_at: now,
    };
    project.storage.memory_upsert(&entry).await
        .map_err(|e| match e {
            // Domain-level scope errors should not occur for USER_AGENT_ID
            // (which has implicit all-scopes access via this command's
            // contract), but if storage propagates one we surface it as
            // validation rather than internal.
            ref err if err.to_string().contains("OutOfScopeWrite") => {
                BooksForgeError::validation(format!("write rejected: {err}"))
            }
            _ => BooksForgeError::internal(e.to_string()),
        })?;
    let _ = MemoryError::NotFound { scope, key: String::new() }; // touch import to silence unused-warn until compile
    Ok(memory_to_dto(entry))
}

#[tauri::command]
pub async fn memory_delete(
    input: MemoryDeleteInput,
    state: State<'_, AppState>,
) -> Result<bool, BooksForgeError> {
    let project = require_project(&state).await?;
    let id = Ulid::from_string(&input.id)
        .map_err(|e| BooksForgeError::validation(format!("invalid memory id: {e}")))?;
    project.storage.memory_delete(id).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(true)
}

#[tauri::command]
pub async fn vocab_upsert(
    input: VocabUpsertInput,
    state: State<'_, AppState>,
) -> Result<VocabEntryDto, BooksForgeError> {
    let project = require_project(&state).await?;
    if input.term.trim().is_empty() {
        return Err(BooksForgeError::validation("vocab term cannot be empty".to_owned()));
    }
    let kind = EntryKind::from_str(&input.kind)
        .ok_or_else(|| BooksForgeError::validation(format!("invalid vocab kind: {} (expected prefer|avoid|replace)", input.kind)))?;

    let now = Utc::now();
    let id = match &input.id {
        Some(s) => Ulid::from_string(s)
            .map_err(|e| BooksForgeError::validation(format!("invalid vocab id: {e}")))?,
        None => Ulid::new(),
    };

    let entry = VocabEntry {
        id,
        layer:        input.layer.clone(),
        term:         input.term.clone(),
        display_term: if input.display_term.is_empty() { input.term.clone() } else { input.display_term.clone() },
        kind,
        replacement:  input.replacement.clone(),
        rationale:    input.rationale.clone(),
        source:       EntrySource::User,
        created_at:   now,
        updated_at:   now,
    };
    project.storage.vocab_upsert(&entry).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(vocab_to_dto(entry))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn require_project(
    state: &State<'_, AppState>,
) -> Result<std::sync::Arc<crate::state::OpenProject>, BooksForgeError> {
    let guard = state.open_project.lock().await;
    guard.as_ref().cloned()
        .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))
}

fn memory_to_dto(e: MemoryEntry) -> MemoryEntryDto {
    MemoryEntryDto {
        id:         e.id.to_string(),
        scope:      e.scope.as_str().to_owned(),
        key:        e.key,
        value_json: e.value_json.to_string(),
        agent_id:   e.agent_id,
        created_at: e.created_at.to_rfc3339(),
        updated_at: e.updated_at.to_rfc3339(),
    }
}

fn vocab_to_dto(v: VocabEntry) -> VocabEntryDto {
    VocabEntryDto {
        id:           v.id.to_string(),
        layer:        v.layer,
        term:         v.term,
        display_term: v.display_term,
        kind:         v.kind.as_str().to_owned(),
        replacement:  v.replacement,
        rationale:    v.rationale,
        source:       v.source.as_str().to_owned(),
        created_at:   v.created_at.to_rfc3339(),
        updated_at:   v.updated_at.to_rfc3339(),
    }
}
