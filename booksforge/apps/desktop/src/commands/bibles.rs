//! Bible-editing commands. Lets the writer enter character + world
//! bibles directly through the UI instead of (or in addition to)
//! generating them via the AI agents.
//!
//! Why this exists: agent-generated bibles cost 2-5 minutes per call
//! on a 27B-class model, and writers with strong existing world-bible
//! material want to skip the generation entirely. The book pipeline
//! already auto-skips stages whose memory entries are present, so
//! the only piece missing was a write path for bible content. This
//! file is that path.
//!
//! Storage convention (mirrors what `apply_character_bible` and
//! `apply_world_bible` write):
//!   - characters → entity scope, key `character:<slug-of-name>`
//!   - locations  → entity scope, key `location:<slug-of-name>`
//!   - world freeform fields (history, social_rules, sensory_palette,
//!     conflict_sources, symbolic_motifs, continuity_constraints) →
//!     book scope, key `world:<field>`
//!
//! The pipeline reads from these same keys, so a hand-typed bible is
//! indistinguishable from an AI-generated one downstream.

use booksforge_domain::{
    CharacterCard, MemoryEntry, MemoryScope, SensoryPalette, WorldBibleProposal,
};
use booksforge_ipc::BooksForgeError;
use booksforge_storage::StorageRepository;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;
use ulid::Ulid;

use crate::commands::agents::require_open_project;
use crate::state::AppState;

// ── DTOs ────────────────────────────────────────────────────────────────────

/// Result of `bibles_load`. Mirrors the shape the UI form needs:
/// the typed CharacterCards in their existing order plus a
/// reconstructed WorldBibleProposal (or empty when no world fields
/// have been written yet).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BiblesLoadResult {
    /// One entry per character currently in entity-scope memory under
    /// the `character:*` key prefix. Order = oldest-first by creation.
    #[ts(type = "unknown[]")]
    pub characters: Vec<serde_json::Value>,
    /// Reconstructed world bible. `null` if no world entries exist
    /// at all (UI shows a "set up world bible" CTA in that case).
    #[ts(type = "unknown | null")]
    pub world: Option<serde_json::Value>,
    /// `true` when entity-scope memory contains at least one
    /// `character:*` entry. The book pipeline reads this to decide
    /// whether to skip the character-bible stage.
    pub has_character_bible: bool,
    /// Same for the world bible (any `entity:location:*` or
    /// `book:world:*` entry counts).
    pub has_world_bible: bool,
}

/// Input to `bibles_save`. Either array can be `None` to leave that
/// half of the bible untouched (so the UI can save just characters
/// without overwriting the world bible, and vice versa). When an
/// array IS supplied, it is treated as the authoritative full list:
/// characters / locations not in the new list are removed from
/// memory.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BiblesSaveInput {
    /// New full list of CharacterCards. `None` = don't touch
    /// character entries.
    #[ts(type = "unknown[] | null")]
    pub characters: Option<Vec<serde_json::Value>>,
    /// New full WorldBibleProposal. `None` = don't touch world entries.
    #[ts(type = "unknown | null")]
    pub world: Option<serde_json::Value>,
}

/// Result of `bibles_save`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BiblesSaveResult {
    pub characters_written: u32,
    pub characters_removed: u32,
    pub world_locations_written: u32,
    pub world_locations_removed: u32,
    /// Names of `world:*` book-scope keys that were written.
    pub world_fields_written: Vec<String>,
}

// ── bibles_load ─────────────────────────────────────────────────────────────

/// Load the project's character + world bibles from memory. The
/// `BiblesPanel` UI uses this on mount; the `BookGenerationPanel`
/// uses `has_*` flags to render "Will skip — already provided" badges.
#[tauri::command]
pub async fn bibles_load(state: State<'_, AppState>) -> Result<BiblesLoadResult, BooksForgeError> {
    let project = require_open_project(&state).await?;

    let entity_mem = project
        .storage
        .memory_list_by_scope(MemoryScope::Entity)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let book_mem = project
        .storage
        .memory_list_by_scope(MemoryScope::Book)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let mut characters: Vec<MemoryEntry> = entity_mem
        .iter()
        .filter(|m| m.key.starts_with("character:"))
        .cloned()
        .collect();
    characters.sort_by_key(|m| m.created_at);
    let character_payload: Vec<serde_json::Value> =
        characters.iter().map(|m| m.value_json.clone()).collect();
    let has_character_bible = !character_payload.is_empty();

    let mut locations: Vec<MemoryEntry> = entity_mem
        .iter()
        .filter(|m| m.key.starts_with("location:"))
        .cloned()
        .collect();
    locations.sort_by_key(|m| m.created_at);
    let location_payload: Vec<serde_json::Value> =
        locations.iter().map(|m| m.value_json.clone()).collect();

    let world_fields: std::collections::BTreeMap<String, serde_json::Value> = book_mem
        .into_iter()
        .filter(|m| m.key.starts_with("world:"))
        .map(|m| {
            let field = m.key.strip_prefix("world:").unwrap_or(&m.key).to_owned();
            (field, m.value_json)
        })
        .collect();

    let has_world_bible = !location_payload.is_empty() || !world_fields.is_empty();

    let world: Option<serde_json::Value> = if has_world_bible {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "main_locations".to_owned(),
            serde_json::Value::Array(location_payload),
        );
        for (k, v) in world_fields {
            obj.insert(k, v);
        }
        Some(serde_json::Value::Object(obj))
    } else {
        None
    };

    Ok(BiblesLoadResult {
        characters: character_payload,
        world,
        has_character_bible,
        has_world_bible,
    })
}

// ── bibles_save ─────────────────────────────────────────────────────────────

/// Write the writer-supplied bibles to memory. Validates each
/// CharacterCard / WorldLocation on the way through so a malformed
/// payload is rejected before any rows land. Idempotent — safe to
/// call repeatedly.
#[tauri::command]
pub async fn bibles_save(
    input: BiblesSaveInput,
    state: State<'_, AppState>,
) -> Result<BiblesSaveResult, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let now = Utc::now();
    let mut result = BiblesSaveResult {
        characters_written: 0,
        characters_removed: 0,
        world_locations_written: 0,
        world_locations_removed: 0,
        world_fields_written: Vec::new(),
    };

    // ── Characters ────────────────────────────────────────────────
    if let Some(raw_chars) = input.characters {
        // Round-trip each card through the typed schema so we reject
        // malformed input the same way the AI-generated bibles get
        // validated. Empty `name` is the one hard requirement; the
        // rest defaults sensibly for partial bibles.
        let mut cards: Vec<CharacterCard> = Vec::with_capacity(raw_chars.len());
        for (i, v) in raw_chars.into_iter().enumerate() {
            let card: CharacterCard = serde_json::from_value(v).map_err(|e| {
                BooksForgeError::validation(format!("character[{i}] shape rejected: {e}"))
            })?;
            if card.name.trim().is_empty() {
                return Err(BooksForgeError::validation(format!(
                    "character[{i}] has empty name"
                )));
            }
            cards.push(card);
        }

        // Diff against existing entity:character:* entries — write
        // the new set, remove anything not in it. Matches what the
        // AI apply path does.
        let existing = project
            .storage
            .memory_list_by_scope(MemoryScope::Entity)
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?
            .into_iter()
            .filter(|m| m.key.starts_with("character:"))
            .collect::<Vec<_>>();

        let new_keys: std::collections::HashSet<String> = cards
            .iter()
            .map(|c| memory_key_for_character(&c.name))
            .collect();
        for old in &existing {
            if !new_keys.contains(&old.key) {
                let removed = project
                    .storage
                    .memory_delete(MemoryScope::Entity, &old.key)
                    .await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
                result.characters_removed = result.characters_removed.saturating_add(removed);
            }
        }

        for card in &cards {
            let key = memory_key_for_character(&card.name);
            let value =
                serde_json::to_value(card).map_err(|e| BooksForgeError::internal(e.to_string()))?;
            let entry = MemoryEntry {
                id: Ulid::new(),
                scope: MemoryScope::Entity,
                key,
                value_json: value,
                agent_id: "user-edit".to_owned(),
                created_at: now,
                updated_at: now,
            };
            project
                .storage
                .memory_upsert(&entry)
                .await
                .map_err(|e| BooksForgeError::internal(e.to_string()))?;
            result.characters_written = result.characters_written.saturating_add(1);
        }
    }

    // ── World ─────────────────────────────────────────────────────
    if let Some(raw_world) = input.world {
        // Validate via the typed shape; we ALLOW a partial world
        // bible during save (e.g. user has filled in locations but
        // not history yet) by tolerating empty arrays/strings here —
        // the publishing-export validator will flag any missing
        // fields later. The ONE hard rule: every supplied location
        // must have a non-empty name.
        let world: WorldBibleProposal = serde_json::from_value(raw_world)
            .map_err(|e| BooksForgeError::validation(format!("world bible shape rejected: {e}")))?;
        for (i, loc) in world.main_locations.iter().enumerate() {
            if loc.name.trim().is_empty() {
                return Err(BooksForgeError::validation(format!(
                    "location[{i}] has empty name"
                )));
            }
        }

        // Diff & rewrite locations (entity scope).
        let existing_locs = project
            .storage
            .memory_list_by_scope(MemoryScope::Entity)
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?
            .into_iter()
            .filter(|m| m.key.starts_with("location:"))
            .collect::<Vec<_>>();
        let new_loc_keys: std::collections::HashSet<String> = world
            .main_locations
            .iter()
            .map(|l| memory_key_for_location(&l.name))
            .collect();
        for old in &existing_locs {
            if !new_loc_keys.contains(&old.key) {
                let removed = project
                    .storage
                    .memory_delete(MemoryScope::Entity, &old.key)
                    .await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
                result.world_locations_removed =
                    result.world_locations_removed.saturating_add(removed);
            }
        }
        for loc in &world.main_locations {
            let entry = MemoryEntry {
                id: Ulid::new(),
                scope: MemoryScope::Entity,
                key: memory_key_for_location(&loc.name),
                value_json: serde_json::to_value(loc)
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?,
                agent_id: "user-edit".to_owned(),
                created_at: now,
                updated_at: now,
            };
            project
                .storage
                .memory_upsert(&entry)
                .await
                .map_err(|e| BooksForgeError::internal(e.to_string()))?;
            result.world_locations_written = result.world_locations_written.saturating_add(1);
        }

        // Freeform world fields → book scope under `world:<field>`.
        // Skip empty strings/arrays so we don't pollute memory with
        // empties; the writer can save partial bibles without
        // creating phantom entries. Built as a list of (field, value)
        // pairs and then written in one pass — keeps the code tight
        // and the writes properly async (no block_on).
        let mut field_writes: Vec<(&'static str, serde_json::Value)> = Vec::new();
        if !world.history.trim().is_empty() {
            field_writes.push(("history", serde_json::Value::String(world.history.clone())));
        }
        if !world.social_rules.is_empty() {
            field_writes.push(("social_rules", serde_json::json!(world.social_rules)));
        }
        if !is_empty_palette(&world.sensory_palette) {
            field_writes.push((
                "sensory_palette",
                serde_json::to_value(&world.sensory_palette)
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?,
            ));
        }
        if !world.conflict_sources.is_empty() {
            field_writes.push((
                "conflict_sources",
                serde_json::json!(world.conflict_sources),
            ));
        }
        if !world.symbolic_motifs.is_empty() {
            field_writes.push(("symbolic_motifs", serde_json::json!(world.symbolic_motifs)));
        }
        if !world.continuity_constraints.is_empty() {
            field_writes.push((
                "continuity_constraints",
                serde_json::json!(world.continuity_constraints),
            ));
        }
        for (field, value) in field_writes {
            let entry = MemoryEntry {
                id: Ulid::new(),
                scope: MemoryScope::Book,
                key: format!("world:{field}"),
                value_json: value,
                agent_id: "user-edit".to_owned(),
                created_at: now,
                updated_at: now,
            };
            project
                .storage
                .memory_upsert(&entry)
                .await
                .map_err(|e| BooksForgeError::internal(e.to_string()))?;
            result.world_fields_written.push(field.to_owned());
        }
    }

    Ok(result)
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// Slug a name for use as a memory key. Lowercase, spaces → underscores,
/// strip anything outside [a-z0-9_]. Mirrors the AI apply path.
fn slug(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn memory_key_for_character(name: &str) -> String {
    format!("character:{}", slug(name))
}

fn memory_key_for_location(name: &str) -> String {
    format!("location:{}", slug(name))
}

fn is_empty_palette(p: &SensoryPalette) -> bool {
    p.sight.trim().is_empty()
        && p.sound.trim().is_empty()
        && p.smell.trim().is_empty()
        && p.touch.trim().is_empty()
        && p.taste.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_normalises_names() {
        assert_eq!(slug("Elara Kowalski"), "elara_kowalski");
        assert_eq!(slug("  Father O'Hara  "), "father_o_hara");
        assert_eq!(slug("Maeve"), "maeve");
    }

    #[test]
    fn memory_keys_match_apply_path_format() {
        assert_eq!(memory_key_for_character("Ada"), "character:ada");
        assert_eq!(
            memory_key_for_location("The Workshop"),
            "location:the_workshop"
        );
    }

    #[test]
    fn empty_palette_detected() {
        assert!(is_empty_palette(&SensoryPalette::default()));
        let p = SensoryPalette {
            sight: "low gray light".to_owned(),
            ..SensoryPalette::default()
        };
        assert!(!is_empty_palette(&p));
    }
}
