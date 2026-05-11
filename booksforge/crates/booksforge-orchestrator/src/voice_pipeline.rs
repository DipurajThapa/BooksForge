//! Voice fingerprint pipeline — compute, persist, retrieve.
//!
//! The fingerprint is stored as a `MemoryEntry` in the `Style` scope under
//! key `voice_fingerprint`.  Storing it through memory rather than as a
//! dedicated column keeps the audit trail consistent (every change rides
//! through `memory_upsert` and gets a `last_writer` stamp) and avoids a
//! schema migration.
//!
//! Lifecycle:
//!   1. **Refresh** — `refresh_from_corpus` reads every accepted scene's
//!      `pm_doc` text content, runs `VoiceFingerprint::compute` on the
//!      union, and upserts the result.  Called by Memory Curator on
//!      chapter finalise; also exposed as a manual refresh command for
//!      tests and one-off recomputation.
//!   2. **Load** — `load_or_default` returns the stored fingerprint, or
//!      `VoiceFingerprint::default()` if no row exists yet.  Used by the
//!      Tauri command layer to populate `RunContext.voice_fingerprint`.

use std::sync::Arc;

use booksforge_domain::{pm_doc_to_text, MemoryEntry, MemoryScope, VoiceFingerprint};
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use ulid::Ulid;

const VOICE_KEY: &str = "voice_fingerprint";

/// Load the project's voice fingerprint, or return a sensible default if
/// no row exists.  Never errors — a corrupted blob falls through to default.
pub async fn load_or_default(storage: &Arc<SqliteStorage>) -> VoiceFingerprint {
    let entry = storage
        .memory_get(MemoryScope::Style, VOICE_KEY)
        .await
        .ok()
        .flatten();
    match entry {
        Some(e) => serde_json::from_value(e.value_json).unwrap_or_default(),
        None => VoiceFingerprint::default(),
    }
}

/// Recompute the fingerprint from every accepted scene's text content
/// and upsert it.  Returns the freshly-computed fingerprint.
///
/// `agent_id` should be the caller's own id (e.g. `"memory-curator"`)
/// so the audit trail attributes the write correctly.
pub async fn refresh_from_corpus(
    storage: &Arc<SqliteStorage>,
    agent_id: &str,
) -> Result<VoiceFingerprint, booksforge_storage::StorageError> {
    let nodes_with_scenes = storage.list_nodes_with_scene_content_consistent().await?;
    let (_, scenes) = nodes_with_scenes;

    // Concatenate every scene's plain-text body.
    let mut corpus = String::with_capacity(64 * 1024);
    for sc in &scenes {
        corpus.push_str(&pm_doc_to_text(&sc.pm_doc));
        corpus.push('\n');
    }

    let fingerprint = VoiceFingerprint::compute(&corpus);

    let now = Utc::now();
    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Style,
        key: VOICE_KEY.to_owned(),
        value_json: serde_json::to_value(&fingerprint).unwrap_or_else(|_| serde_json::json!({})),
        agent_id: agent_id.to_owned(),
        created_at: now,
        updated_at: now,
    };
    storage.memory_upsert(&entry).await?;
    Ok(fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_storage::{open_pool, run_migrations, SqliteStorage};
    use std::sync::Arc;

    async fn fresh_storage() -> (Arc<SqliteStorage>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = open_pool(&dir.path().join("voice.db")).await.unwrap();
        run_migrations(&pool).await.unwrap();
        (Arc::new(SqliteStorage::new(pool)), dir)
    }

    #[tokio::test]
    async fn load_returns_default_when_no_row() {
        let (storage, _dir) = fresh_storage().await;
        let fp = load_or_default(&storage).await;
        assert!(!fp.is_established());
    }

    #[tokio::test]
    async fn refresh_with_empty_corpus_returns_default() {
        let (storage, _dir) = fresh_storage().await;
        let fp = refresh_from_corpus(&storage, "memory-curator")
            .await
            .unwrap();
        // Empty corpus → default fingerprint with corpus_tokens=0.
        assert_eq!(fp.corpus_tokens, 0);
        // And the row was upserted — load now returns it.
        let loaded = load_or_default(&storage).await;
        assert_eq!(loaded.corpus_tokens, 0);
    }
}
