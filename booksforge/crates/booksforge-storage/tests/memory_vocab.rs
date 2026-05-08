#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Phase 3 — memory and vocabulary CRUD round-trip tests.

use std::sync::Arc;

use booksforge_domain::{
    EntryKind, EntrySource, MemoryEntry, MemoryScope, VocabEntry,
};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use ulid::Ulid;

async fn fresh() -> (Arc<SqliteStorage>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = open_pool(&dir.path().join("test.db")).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (Arc::new(SqliteStorage::new(pool)), dir)
}

// ── Memory ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn memory_upsert_then_get_returns_same_value() {
    let (storage, _dir) = fresh().await;
    let now = Utc::now();
    let entry = MemoryEntry {
        id:         Ulid::new(),
        scope:      MemoryScope::Book,
        key:        "premise".into(),
        value_json: serde_json::json!({ "text": "Alice opens the locked archive." }),
        agent_id:   "memory-curator".into(),
        created_at: now,
        updated_at: now,
    };
    storage.memory_upsert(&entry).await.unwrap();

    let got = storage.memory_get(MemoryScope::Book, "premise").await.unwrap().unwrap();
    assert_eq!(got.key, "premise");
    assert_eq!(got.scope, MemoryScope::Book);
    assert_eq!(got.value_json["text"], "Alice opens the locked archive.");
}

#[tokio::test]
async fn memory_upsert_replaces_value_on_conflict() {
    let (storage, _dir) = fresh().await;
    let mut entry = MemoryEntry {
        id:         Ulid::new(),
        scope:      MemoryScope::Style,
        key:        "em_dash".into(),
        value_json: serde_json::json!("em"),
        agent_id:   "copyeditor".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    entry.value_json = serde_json::json!("en");
    entry.updated_at = Utc::now();
    storage.memory_upsert(&entry).await.unwrap();

    let got = storage.memory_get(MemoryScope::Style, "em_dash").await.unwrap().unwrap();
    assert_eq!(got.value_json, serde_json::json!("en"));
}

#[tokio::test]
async fn memory_delete_returns_one_then_zero() {
    let (storage, _dir) = fresh().await;
    let entry = MemoryEntry {
        id:         Ulid::new(),
        scope:      MemoryScope::Entity,
        key:        "alice".into(),
        value_json: serde_json::json!({ "role": "protagonist" }),
        agent_id:   "memory-curator".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    let first = storage.memory_delete(MemoryScope::Entity, "alice").await.unwrap();
    assert_eq!(first, 1);
    let second = storage.memory_delete(MemoryScope::Entity, "alice").await.unwrap();
    assert_eq!(second, 0);
}

// ── Vocabulary ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn vocab_seed_starters_writes_known_layers() {
    let (storage, _dir) = fresh().await;
    let starters = booksforge_vocab::all_starter_entries().unwrap();
    storage.vocab_seed_starters(&starters).await.unwrap();

    let ai_tells = storage.vocab_count_by_layer("ai_tells").await.unwrap();
    let fantasy  = storage.vocab_count_by_layer("genre:fantasy").await.unwrap();
    assert!(ai_tells >= 12, "ai_tells must have ≥12 starter entries, got {ai_tells}");
    assert!(fantasy  >= 6,  "genre:fantasy must have ≥6 starter entries, got {fantasy}");
}

#[tokio::test]
async fn vocab_resolve_with_active_layers_picks_most_specific() {
    let (storage, _dir) = fresh().await;

    let mut e_ai = VocabEntry::new("ai_tells", "Tapestry", EntryKind::Replace, EntrySource::Starter);
    e_ai = e_ai.with_replacement("pattern");
    let mut e_proj = VocabEntry::new("project", "Tapestry", EntryKind::Replace, EntrySource::User);
    e_proj = e_proj.with_replacement("weave");

    storage.vocab_upsert(&e_ai).await.unwrap();
    storage.vocab_upsert(&e_proj).await.unwrap();

    let entries = storage
        .vocab_list_by_layers(&["project", "ai_tells"])
        .await
        .unwrap();

    let merged = booksforge_domain::resolve_vocab(&entries, &["project", "ai_tells"]);
    let tapestry = merged.iter().find(|e| e.term == "tapestry").unwrap();
    assert_eq!(tapestry.layer, "project", "project layer must beat ai_tells");
    assert_eq!(tapestry.replacement.as_deref(), Some("weave"));
}

#[tokio::test]
async fn vocab_starters_are_not_overwritten_by_user_rows_when_reseeded() {
    let (storage, _dir) = fresh().await;

    // Seed once, then write a user-curated row.
    let starters = booksforge_vocab::all_starter_entries().unwrap();
    storage.vocab_seed_starters(&starters).await.unwrap();

    let mut user_row = VocabEntry::new(
        "project", "lighthouse", EntryKind::Prefer, EntrySource::User,
    );
    user_row = user_row.with_rationale("Recurring symbol in the manuscript.");
    storage.vocab_upsert(&user_row).await.unwrap();

    // Re-seed (e.g. on every project_open) — user row must survive.
    storage.vocab_seed_starters(&starters).await.unwrap();
    let proj = storage.vocab_count_by_layer("project").await.unwrap();
    assert_eq!(proj, 1, "user-curated project row must survive a re-seed");
}

#[tokio::test]
async fn vocab_seed_replaces_existing_starter_rows() {
    let (storage, _dir) = fresh().await;
    let starters = booksforge_vocab::all_starter_entries().unwrap();

    storage.vocab_seed_starters(&starters).await.unwrap();
    let n1 = storage.vocab_count_by_layer("ai_tells").await.unwrap();
    storage.vocab_seed_starters(&starters).await.unwrap();
    let n2 = storage.vocab_count_by_layer("ai_tells").await.unwrap();
    assert_eq!(n1, n2, "re-seeding must not double the starter count");
}
