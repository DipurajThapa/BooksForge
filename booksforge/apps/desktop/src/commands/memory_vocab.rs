//! Tauri commands for the memory + vocabulary subsystems.

use booksforge_domain::{MemoryEntry, MemoryScope, VocabEntry};
use booksforge_ipc::{
    BooksForgeError, MemoryEntryDto, MemoryListInput, VocabEntryDto, VocabListInput,
};
use booksforge_storage::StorageRepository;
use tauri::State;

use crate::state::AppState;

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
