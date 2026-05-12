#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! MZ-06 acceptance criteria #3 (property test) and #4 (selective restore).
//!
//! These tests exercise `SnapshotService` end-to-end against a real SQLite
//! database and the production `OsFilesystem`, so a regression in any layer
//! (storage, fs, snapshot service) is caught.

use std::sync::Arc;

use booksforge_domain::{Node, NodeKind, NodeStatus, SceneContent, SnapshotScope, SnapshotTrigger};
use booksforge_fs::{BundleFilesystem, BundlePath, OsFilesystem};
use booksforge_snapshot::{SnapshotError, SnapshotService};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use proptest::prelude::*;
use ulid::Ulid;

// ── Test harness ──────────────────────────────────────────────────────────────

struct Harness {
    pub service: Arc<SnapshotService>,
    pub storage: Arc<SqliteStorage>,
    pub _dir: tempfile::TempDir, // kept alive for the lifetime of the harness
}

async fn setup_harness() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    // Lay out a minimal bundle: snapshots/objects/ + project.db.
    let bundle_root = dir.path().join("test.booksforge");
    std::fs::create_dir_all(bundle_root.join("snapshots/objects"))
        .expect("create snapshots/objects");
    let bundle = BundlePath::new(&bundle_root);

    let pool = open_pool(&bundle.db()).await.expect("open_pool");
    run_migrations(&pool).await.expect("migrations");
    let storage = Arc::new(SqliteStorage::new(pool));

    let storage_trait: Arc<dyn StorageRepository> = storage.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    let service = Arc::new(SnapshotService::new(storage_trait, fs, bundle));

    Harness {
        service,
        storage,
        _dir: dir,
    }
}

fn make_scene(node_id: Ulid, title: &str) -> Node {
    let now = Utc::now();
    Node {
        id: node_id,
        parent_id: None,
        kind: NodeKind::Scene,
        title: title.to_owned(),
        position: Node::DEFAULT_POSITION.to_owned(),
        status: NodeStatus::Drafting,
        pov: None,
        beat: None,
        target_words: None,
        synopsis: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

fn make_scene_content(node_id: Ulid, body: &str) -> SceneContent {
    let pm_doc = serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": body }]
        }]
    });
    let bytes = serde_json::to_vec(&pm_doc).unwrap();
    let hash = blake3::hash(&bytes).to_hex().to_string();
    SceneContent {
        node_id,
        pm_doc,
        word_count: body.split_whitespace().count() as u32,
        char_count: body.chars().count() as u32,
        hash,
        updated_at: Utc::now(),
    }
}

/// Read every scene's `pm_doc` body text.  Returns a `(node_id → text)` map
/// so two harness states can be compared without caring about row ordering.
async fn scene_bodies(storage: &SqliteStorage) -> std::collections::BTreeMap<Ulid, String> {
    let nodes = storage.list_nodes().await.expect("list_nodes");
    let mut out = std::collections::BTreeMap::new();
    for node in nodes {
        if node.kind != NodeKind::Scene {
            continue;
        }
        let scene = storage.load_scene(node.id).await.expect("load_scene");
        let text = scene
            .and_then(|s| extract_first_text(&s.pm_doc))
            .unwrap_or_default();
        out.insert(node.id, text);
    }
    out
}

fn extract_first_text(pm_doc: &serde_json::Value) -> Option<String> {
    pm_doc
        .get("content")?
        .as_array()?
        .first()?
        .get("content")?
        .as_array()?
        .first()?
        .get("text")?
        .as_str()
        .map(str::to_owned)
}

// ── Property test (criterion 3) ───────────────────────────────────────────────

/// Apply / restore round-trip: for any sequence of scene-content writes, the
/// state captured at snapshot S must be byte-equal to `list_nodes` + scene
/// bodies after `restore(S)` — even if the project has been further mutated
/// in between.  No data loss, no partial state.
fn run_round_trip(initial: Vec<String>, mutations: Vec<String>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        let h = setup_harness().await;

        // Insert one scene per `initial` body.
        let ids: Vec<Ulid> = (0..initial.len()).map(|_| Ulid::new()).collect();
        for (id, body) in ids.iter().zip(&initial) {
            h.storage
                .insert_node(&make_scene(*id, "scene"))
                .await
                .expect("insert_node");
            h.storage
                .save_scene(&make_scene_content(*id, body))
                .await
                .expect("save_scene");
        }

        // Capture baseline state.
        let baseline = scene_bodies(&h.storage).await;

        // Take snapshot S.
        let snap = h
            .service
            .create(
                SnapshotScope::Project,
                None,
                Some("S".into()),
                SnapshotTrigger::Manual,
            )
            .await
            .expect("create snapshot");

        // Apply mutations: overwrite each scene with the i-th `mutations` body
        // (cyclic if mutations is shorter than ids).
        if !mutations.is_empty() {
            for (i, id) in ids.iter().enumerate() {
                let body = &mutations[i % mutations.len()];
                h.storage
                    .save_scene(&make_scene_content(*id, body))
                    .await
                    .expect("save_scene");
            }
        }

        // Restore S — pre-restore safety snapshot is created by the service.
        let report = h.service.restore(snap.id, None).await.expect("restore");
        assert_eq!(report.nodes_restored as usize, ids.len());

        // After restore, scene bodies must match the captured baseline.
        let after = scene_bodies(&h.storage).await;
        assert_eq!(
            after, baseline,
            "restore must reproduce the snapshot state exactly"
        );
    });
}

// ── Generators ────────────────────────────────────────────────────────────────

fn arb_body() -> impl Strategy<Value = String> {
    "[a-z][a-z ]{0,40}".prop_map(String::from)
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Each case spins up a tempdir + sqlite + WAL + migrations, so keep
        // the case count modest while still hitting the round-trip property.
        cases: 16,
        .. ProptestConfig::default()
    })]

    /// Criterion #3 — random apply/restore must never lose data.
    #[test]
    fn snapshot_apply_restore_is_lossless(
        initial   in prop::collection::vec(arb_body(), 1..6),
        mutations in prop::collection::vec(arb_body(), 0..6),
    ) {
        run_round_trip(initial, mutations);
    }
}

// ── Selective restore (criterion 4) ───────────────────────────────────────────

#[tokio::test]
async fn selective_restore_only_touches_chosen_nodes() {
    let h = setup_harness().await;

    // Two scenes: A and B, each with a distinctive body.
    let id_a = Ulid::new();
    let id_b = Ulid::new();
    h.storage
        .insert_node(&make_scene(id_a, "Scene A"))
        .await
        .unwrap();
    h.storage
        .insert_node(&make_scene(id_b, "Scene B"))
        .await
        .unwrap();
    h.storage
        .save_scene(&make_scene_content(id_a, "A original"))
        .await
        .unwrap();
    h.storage
        .save_scene(&make_scene_content(id_b, "B original"))
        .await
        .unwrap();

    let snap = h
        .service
        .create(
            SnapshotScope::Project,
            None,
            Some("S".into()),
            SnapshotTrigger::Manual,
        )
        .await
        .expect("create");

    // Mutate both.
    h.storage
        .save_scene(&make_scene_content(id_a, "A mutated"))
        .await
        .unwrap();
    h.storage
        .save_scene(&make_scene_content(id_b, "B mutated"))
        .await
        .unwrap();

    // Restore only A.
    let report = h
        .service
        .restore(snap.id, Some(vec![id_a]))
        .await
        .expect("selective restore");
    assert_eq!(report.nodes_restored, 1);

    let bodies = scene_bodies(&h.storage).await;
    assert_eq!(
        bodies.get(&id_a).map(String::as_str),
        Some("A original"),
        "A must revert to snapshot state"
    );
    assert_eq!(
        bodies.get(&id_b).map(String::as_str),
        Some("B mutated"),
        "B must keep its post-mutation state — selective restore must not touch it"
    );
}

// ── Pre-restore safety snapshot is recorded ──────────────────────────────────

#[tokio::test]
async fn restore_takes_pre_restore_safety_snapshot_first() {
    let h = setup_harness().await;
    let id = Ulid::new();
    h.storage
        .insert_node(&make_scene(id, "scene"))
        .await
        .unwrap();
    h.storage
        .save_scene(&make_scene_content(id, "v1"))
        .await
        .unwrap();

    let snap = h
        .service
        .create(
            SnapshotScope::Project,
            None,
            Some("baseline".into()),
            SnapshotTrigger::Manual,
        )
        .await
        .expect("create");

    h.storage
        .save_scene(&make_scene_content(id, "v2"))
        .await
        .unwrap();

    let before = h.storage.list_snapshots(None).await.unwrap().len();
    let report = h.service.restore(snap.id, None).await.expect("restore");
    let after = h.storage.list_snapshots(None).await.unwrap().len();

    // The safety snapshot must be a real, addressable record.
    assert!(
        after > before,
        "restore must add a pre-restore snapshot to the manifest"
    );
    let pre_id_str = report.pre_restore_snapshot_id.to_string();
    let found = h
        .storage
        .list_snapshots(None)
        .await
        .unwrap()
        .into_iter()
        .any(|s| s.id.to_string() == pre_id_str);
    assert!(
        found,
        "the returned pre_restore_snapshot_id must exist in the snapshots table"
    );
}

#[tokio::test]
async fn restore_failure_after_safety_carries_safety_id() {
    // Trigger a post-safety failure by feeding `restore` an unknown id —
    // the safety snapshot still gets written, then `load_tree` fails.
    let h = setup_harness().await;
    let unknown = Ulid::new();
    let err = h
        .service
        .restore(unknown, None)
        .await
        .expect_err("must fail");
    match err {
        SnapshotError::RestoreFailedAfterSafety { safety_id, source } => {
            // The safety snapshot must exist in the manifest.
            let exists = h
                .storage
                .list_snapshots(None)
                .await
                .unwrap()
                .into_iter()
                .any(|s| s.id == safety_id);
            assert!(
                exists,
                "safety snapshot {safety_id} must persist on failure"
            );
            // And the wrapped error tells us why.
            assert!(matches!(*source, SnapshotError::NotFound(_)));
        }
        other => panic!("expected RestoreFailedAfterSafety, got {other:?}"),
    }
}
