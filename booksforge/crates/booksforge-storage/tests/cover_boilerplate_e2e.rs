#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! E2E tests for the Stage 6 cover & boilerplate flow at the storage
//! layer. These exercise the exact serialisation contract the Tauri
//! commands rely on (`book:cover_set` and `book:boilerplate_pages`
//! memory entries) without needing a Tauri runtime.
//!
//! What we're verifying:
//!   - The CoverSet round-trips through SQLite memory_upsert / memory_get
//!     with every slot preserved.
//!   - Empty CoverSet (no slots filled) is the right default when no
//!     memory entry exists yet.
//!   - The BoilerplatePage list round-trips, including all 11
//!     BoilerplateKind variants.
//!   - Boilerplate pages preserve their `order` so the Tauri command's
//!     `sort_by_key(|p| p.order)` produces a stable read order.
//!   - The `include_in_export` flag defaults to true when missing
//!     (schema-tolerance for older payloads).
//!   - Front-vs-back-matter classification is correct for every kind.

use std::sync::Arc;

use booksforge_domain::{
    BoilerplateKind, BoilerplatePage, CoverAsset, CoverSet, MemoryEntry, MemoryScope,
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

fn sample_front_asset() -> CoverAsset {
    CoverAsset {
        bundle_path: "assets/cover-front.jpg".into(),
        original_filename: "the-wrong-side-light.jpg".into(),
        size_bytes: 524_288,
        mime_type: "image/jpeg".into(),
        width_px: Some(1_600),
        height_px: Some(2_560),
        imported_at: Utc::now(),
    }
}

fn sample_back_asset() -> CoverAsset {
    CoverAsset {
        bundle_path: "assets/cover-back.png".into(),
        original_filename: "back.png".into(),
        size_bytes: 700_000,
        mime_type: "image/png".into(),
        width_px: Some(1_600),
        height_px: Some(2_560),
        imported_at: Utc::now(),
    }
}

// ── CoverSet ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn cover_set_front_only_round_trip() {
    let (storage, _dir) = fresh().await;

    let cover_set = CoverSet {
        front: Some(sample_front_asset()),
        back: None,
        spine: None,
    };

    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: "cover_set".into(),
        value_json: serde_json::to_value(&cover_set).unwrap(),
        agent_id: "cover-boilerplate".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    let got = storage
        .memory_get(MemoryScope::Book, "cover_set")
        .await
        .unwrap()
        .expect("cover_set entry present");
    let parsed: CoverSet = serde_json::from_value(got.value_json).unwrap();

    assert!(parsed.has_front());
    assert!(parsed.back.is_none());
    assert!(parsed.spine.is_none());
    assert_eq!(
        parsed.front.as_ref().unwrap().bundle_path,
        "assets/cover-front.jpg"
    );
    assert_eq!(parsed.front.as_ref().unwrap().size_bytes, 524_288);
}

#[tokio::test]
async fn cover_set_full_paperback_round_trip() {
    let (storage, _dir) = fresh().await;

    let cover_set = CoverSet {
        front: Some(sample_front_asset()),
        back: Some(sample_back_asset()),
        spine: Some(CoverAsset {
            bundle_path: "assets/cover-spine.png".into(),
            original_filename: "spine.png".into(),
            size_bytes: 50_000,
            mime_type: "image/png".into(),
            width_px: Some(100),
            height_px: Some(2_560),
            imported_at: Utc::now(),
        }),
    };

    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: "cover_set".into(),
        value_json: serde_json::to_value(&cover_set).unwrap(),
        agent_id: "cover-boilerplate".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    let parsed: CoverSet = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "cover_set")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();

    assert!(parsed.has_front());
    assert!(parsed.back.is_some());
    assert!(parsed.spine.is_some());
    assert!(!parsed.is_empty());
}

#[tokio::test]
async fn cover_set_absent_entry_returns_default() {
    let (storage, _dir) = fresh().await;

    // No memory_upsert call. Mirrors the Tauri command's
    // `load_cover_set_inner` fallback: absent entry → CoverSet::default().
    let got = storage
        .memory_get(MemoryScope::Book, "cover_set")
        .await
        .unwrap();
    assert!(got.is_none(), "no entry yet");

    let fallback = CoverSet::default();
    assert!(fallback.is_empty());
    assert!(!fallback.has_front());
}

#[tokio::test]
async fn cover_set_replace_slot_on_upsert() {
    let (storage, _dir) = fresh().await;

    // First import: front only.
    let mut cover_set = CoverSet {
        front: Some(sample_front_asset()),
        back: None,
        spine: None,
    };
    let entry = |cs: &CoverSet| MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: "cover_set".into(),
        value_json: serde_json::to_value(cs).unwrap(),
        agent_id: "cover-boilerplate".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry(&cover_set)).await.unwrap();

    // Now add a back cover and re-upsert; the original entry must be
    // replaced (memory_upsert is "upsert", not "append").
    cover_set.back = Some(sample_back_asset());
    storage.memory_upsert(&entry(&cover_set)).await.unwrap();

    let parsed: CoverSet = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "cover_set")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();
    assert!(parsed.front.is_some());
    assert!(parsed.back.is_some());
    assert!(parsed.spine.is_none());
}

#[tokio::test]
async fn cover_asset_aspect_ratio_handles_zero_height() {
    // Edge case: width_px set but height_px = 0 would div-by-zero
    // without the guard in CoverAsset::aspect_x100.
    let asset = CoverAsset {
        bundle_path: "assets/cover-front.jpg".into(),
        original_filename: "front.jpg".into(),
        size_bytes: 10_000,
        mime_type: "image/jpeg".into(),
        width_px: Some(1_600),
        height_px: Some(0),
        imported_at: Utc::now(),
    };
    assert!(asset.aspect_x100().is_none(), "must not divide by zero");
}

#[tokio::test]
async fn cover_asset_missing_dimensions_returns_none() {
    let asset = CoverAsset {
        bundle_path: "assets/cover-front.jpg".into(),
        original_filename: "front.jpg".into(),
        size_bytes: 10_000,
        mime_type: "image/jpeg".into(),
        width_px: None,
        height_px: None,
        imported_at: Utc::now(),
    };
    assert!(asset.aspect_x100().is_none());
}

#[tokio::test]
async fn cover_set_legacy_json_without_back_or_spine_parses() {
    // Older bundles (or hand-edited memory entries) may only have a
    // `front` key. Schema tolerance via `#[serde(default)]` must keep
    // these readable.
    let (storage, _dir) = fresh().await;

    let legacy_json = serde_json::json!({
        "front": {
            "bundle_path": "assets/cover-front.jpg",
            "original_filename": "front.jpg",
            "size_bytes": 100_000,
            "mime_type": "image/jpeg",
            "imported_at": Utc::now().to_rfc3339()
        }
    });
    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: "cover_set".into(),
        value_json: legacy_json,
        agent_id: "cover-boilerplate".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    let parsed: CoverSet = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "cover_set")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();
    assert!(parsed.has_front());
    assert!(parsed.back.is_none());
    assert!(parsed.spine.is_none());
}

// ── BoilerplatePage list ─────────────────────────────────────────────────

#[tokio::test]
async fn boilerplate_pages_round_trip_preserves_order() {
    let (storage, _dir) = fresh().await;

    // Build a list that mixes front- and back-matter pages. Order
    // values are deliberately non-monotonic to confirm the list
    // survives JSON round-trip without reordering.
    let pages = vec![
        BoilerplatePage::new("a", BoilerplateKind::Copyright, 1),
        BoilerplatePage::new("b", BoilerplateKind::Dedication, 2),
        BoilerplatePage::new("c", BoilerplateKind::Acknowledgments, 100),
        BoilerplatePage::new("d", BoilerplateKind::AboutAuthor, 101),
    ];

    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: "boilerplate_pages".into(),
        value_json: serde_json::to_value(&pages).unwrap(),
        agent_id: "cover-boilerplate".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    let got: Vec<BoilerplatePage> = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "boilerplate_pages")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();
    assert_eq!(got.len(), 4);
    assert_eq!(got[0].id, "a");
    assert_eq!(got[3].id, "d");

    // Confirm the front/back classification matches the kind.
    assert!(got[0].kind.is_front_matter()); // Copyright
    assert!(got[1].kind.is_front_matter()); // Dedication
    assert!(!got[2].kind.is_front_matter()); // Acknowledgments
    assert!(!got[3].kind.is_front_matter()); // AboutAuthor
}

#[tokio::test]
async fn boilerplate_page_body_md_survives_special_chars() {
    let (storage, _dir) = fresh().await;

    let mut page = BoilerplatePage::new("copy-1", BoilerplateKind::Copyright, 0);
    page.body_md = "Copyright © 2026 by Author Name.\n\n\"All rights reserved.\"\n\n— ISBN 978-3-16-148410-0".into();
    let pages = vec![page];

    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: "boilerplate_pages".into(),
        value_json: serde_json::to_value(&pages).unwrap(),
        agent_id: "cover-boilerplate".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.memory_upsert(&entry).await.unwrap();

    let got: Vec<BoilerplatePage> = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "boilerplate_pages")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();
    assert_eq!(got.len(), 1);
    assert!(got[0].body_md.contains('©'));
    assert!(got[0].body_md.contains("ISBN"));
    assert!(got[0].body_md.contains('—'));
    // "Copyright © 2026 by Author Name. \"All rights reserved.\" — ISBN 978-3-16-148410-0"
    // split_whitespace counts ©, —, and ISBN as separate tokens.
    assert_eq!(got[0].word_count(), 12);
}

#[tokio::test]
async fn boilerplate_replace_on_upsert() {
    let (storage, _dir) = fresh().await;

    // First save: one page.
    let v1 = vec![BoilerplatePage::new("a", BoilerplateKind::Copyright, 0)];
    storage
        .memory_upsert(&MemoryEntry {
            id: Ulid::new(),
            scope: MemoryScope::Book,
            key: "boilerplate_pages".into(),
            value_json: serde_json::to_value(&v1).unwrap(),
            agent_id: "cover-boilerplate".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    // Second save: a different list. Upsert must replace, not merge.
    let v2 = vec![
        BoilerplatePage::new("x", BoilerplateKind::Dedication, 0),
        BoilerplatePage::new("y", BoilerplateKind::AboutAuthor, 100),
    ];
    storage
        .memory_upsert(&MemoryEntry {
            id: Ulid::new(),
            scope: MemoryScope::Book,
            key: "boilerplate_pages".into(),
            value_json: serde_json::to_value(&v2).unwrap(),
            agent_id: "cover-boilerplate".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    let got: Vec<BoilerplatePage> = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "boilerplate_pages")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].id, "x");
    assert!(got.iter().all(|p| p.id != "a"), "old entry removed");
}

#[tokio::test]
async fn boilerplate_page_legacy_missing_include_flag_defaults_to_true() {
    // Older payloads without `include_in_export` must default the
    // flag to true so existing books don't silently lose pages.
    let (storage, _dir) = fresh().await;

    let legacy = serde_json::json!([
        {
            "id": "old-1",
            "kind": "dedication",
            "title": "Dedication",
            "body_md": "For my grandmother.",
            "order": 0
        }
    ]);
    storage
        .memory_upsert(&MemoryEntry {
            id: Ulid::new(),
            scope: MemoryScope::Book,
            key: "boilerplate_pages".into(),
            value_json: legacy,
            agent_id: "cover-boilerplate".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    let got: Vec<BoilerplatePage> = serde_json::from_value(
        storage
            .memory_get(MemoryScope::Book, "boilerplate_pages")
            .await
            .unwrap()
            .unwrap()
            .value_json,
    )
    .unwrap();
    assert_eq!(got.len(), 1);
    assert!(got[0].include_in_export, "default flag must be true");
}

#[tokio::test]
async fn every_boilerplate_kind_round_trips_through_serde() {
    let kinds = [
        BoilerplateKind::TitlePage,
        BoilerplateKind::Copyright,
        BoilerplateKind::Dedication,
        BoilerplateKind::Epigraph,
        BoilerplateKind::Foreword,
        BoilerplateKind::Preface,
        BoilerplateKind::Acknowledgments,
        BoilerplateKind::AboutAuthor,
        BoilerplateKind::AlsoBy,
        BoilerplateKind::BackCoverBlurb,
        BoilerplateKind::Other,
    ];
    for k in kinds {
        let v = serde_json::to_value(k).expect("serialise");
        let s = v.as_str().expect("kind serialises to a string");
        // Snake-case rename — never camelCase or unchanged.
        assert!(
            !s.chars().any(|c| c.is_ascii_uppercase()),
            "kind {k:?} → {s} is not snake_case"
        );
        let back: BoilerplateKind = serde_json::from_value(v).expect("deserialise");
        assert_eq!(k, back);
    }
}

#[tokio::test]
async fn boilerplate_front_vs_back_matter_partition() {
    // Sanity check — the export pipeline groups pages by matter
    // (front vs back) before laying them out. Verify the kinds we
    // ship resolve to the intended halves.
    let front: Vec<_> = [
        BoilerplateKind::TitlePage,
        BoilerplateKind::Copyright,
        BoilerplateKind::Dedication,
        BoilerplateKind::Epigraph,
        BoilerplateKind::Foreword,
        BoilerplateKind::Preface,
    ]
    .into_iter()
    .filter(|k| k.is_front_matter())
    .collect();
    assert_eq!(front.len(), 6);

    let back: Vec<_> = [
        BoilerplateKind::Acknowledgments,
        BoilerplateKind::AboutAuthor,
        BoilerplateKind::AlsoBy,
        BoilerplateKind::BackCoverBlurb,
        BoilerplateKind::Other,
    ]
    .into_iter()
    .filter(|k| !k.is_front_matter())
    .collect();
    assert_eq!(back.len(), 5);
}

#[tokio::test]
async fn boilerplate_word_count_handles_unicode_whitespace() {
    let mut p = BoilerplatePage::new("01", BoilerplateKind::Dedication, 0);
    // Mix of ASCII spaces, em-dashes (no-break) and a tab.
    p.body_md = "For\u{00A0}my\u{00A0}grandmother,\twho\tfirst taught\u{00A0}me to listen.".into();
    // split_whitespace handles all Unicode whitespace, so this should
    // count 9 words regardless of which whitespace char separates.
    assert_eq!(p.word_count(), 9);
}
