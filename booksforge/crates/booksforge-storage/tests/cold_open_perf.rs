#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout, clippy::print_stderr, clippy::unimplemented)]

//! D8 — Cold-open performance benchmark.
//!
//! Goal: opening a 100k-word manuscript bundle and loading the binder must
//! complete in well under two seconds on a developer machine, so the
//! perceived launch time stays snappy.
//!
//! "Cold open" means: connect to the SQLite file, run migrations, then
//! call `list_nodes_with_scene_content_consistent` (the same path the
//! `node_list` Tauri command uses).  We do NOT measure pool warmup or
//! WAL checkpoint cost — those are amortised across a session.
//!
//! The fixture is generated once per test invocation: 50 chapters × 5
//! scenes × ~400 words each ≈ 100 000 words.  We seed every scene with
//! the same paragraph because the test exercises I/O / row-count behaviour,
//! not text-content variability.
//!
//! The test is `#[ignore]`d by default so it doesn't slow regular CI; run
//! it explicitly with:
//!
//! ```bash
//! cargo test -p booksforge-storage --release -- --ignored cold_open
//! ```

use std::sync::Arc;
use std::time::Instant;

use booksforge_domain::{Node, NodeKind, NodeStatus, SceneContent};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use ulid::Ulid;

const CHAPTER_COUNT:        usize = 50;
const SCENES_PER_CHAPTER:   usize = 5;
/// Roughly 400 words per scene → 50 × 5 × 400 = 100 000.
const WORDS_PER_SCENE:      usize = 400;

/// Perf budget — wall-clock, end-to-end.  See the module doc for what
/// "cold-open" measures.  Generous because the test runs on whatever
/// machine the dev happens to be on.
const COLD_OPEN_BUDGET_MS:  u128  = 2_000;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // perf — opt-in.  See module doc.
async fn cold_open_100k_words_under_two_seconds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("bench.db");

    // ── Phase 1: generate the fixture ─────────────────────────────────────
    {
        let pool = open_pool(&db_path).await.expect("open pool");
        run_migrations(&pool).await.expect("run migrations");
        let storage = Arc::new(SqliteStorage::new(pool));
        seed_100k_word_manuscript(&storage).await;
        // Drop the pool so the next phase reopens cold.
    }

    // ── Phase 2: cold open + binder load ─────────────────────────────────
    let started = Instant::now();
    let pool = open_pool(&db_path).await.expect("open pool 2");
    run_migrations(&pool).await.expect("re-run migrations is a no-op");
    let storage = Arc::new(SqliteStorage::new(pool));
    let (nodes, scenes) = storage
        .list_nodes_with_scene_content_consistent()
        .await
        .expect("list nodes");
    let elapsed_ms = started.elapsed().as_millis();

    let scene_count = nodes.iter().filter(|n| matches!(n.kind, NodeKind::Scene)).count();
    let total_words: u32 = scenes.iter().map(|s| s.word_count).sum();

    println!(
        "[cold-open] {scene_count} scenes · {total_words} words · {elapsed_ms} ms (budget {COLD_OPEN_BUDGET_MS} ms)"
    );

    assert_eq!(scene_count, CHAPTER_COUNT * SCENES_PER_CHAPTER, "scene count mismatch");
    assert!(
        (95_000..=105_000).contains(&total_words),
        "total word count out of expected range: {total_words}"
    );
    assert!(
        elapsed_ms < COLD_OPEN_BUDGET_MS,
        "cold-open took {elapsed_ms} ms, exceeding budget of {COLD_OPEN_BUDGET_MS} ms"
    );
}

// ── Fixture generator ────────────────────────────────────────────────────────

async fn seed_100k_word_manuscript(storage: &Arc<SqliteStorage>) {
    let now = Utc::now();
    let project_id = Ulid::new();
    let project = Node {
        id:           project_id,
        parent_id:    None,
        kind:         NodeKind::Project,
        title:        "100k benchmark".into(),
        position:     "0|hzzzzz:".into(),
        status:       NodeStatus::Drafting,
        pov:          None,
        beat:         None,
        target_words: Some(100_000),
        created_at:   now,
        updated_at:   now,
        deleted_at:   None,
    };

    let mut all_nodes: Vec<Node> = Vec::with_capacity(1 + CHAPTER_COUNT * (1 + SCENES_PER_CHAPTER));
    all_nodes.push(project);

    let mut scene_ids: Vec<Ulid> = Vec::with_capacity(CHAPTER_COUNT * SCENES_PER_CHAPTER);

    for c in 0..CHAPTER_COUNT {
        let chapter_id = Ulid::new();
        all_nodes.push(Node {
            id:           chapter_id,
            parent_id:    Some(project_id),
            kind:         NodeKind::Chapter,
            title:        format!("Chapter {}", c + 1),
            position:     rank_for(c + 1),
            status:       NodeStatus::Drafting,
            pov:          None,
            beat:         None,
            target_words: None,
            created_at:   now,
            updated_at:   now,
            deleted_at:   None,
        });
        for s in 0..SCENES_PER_CHAPTER {
            let scene_id = Ulid::new();
            scene_ids.push(scene_id);
            all_nodes.push(Node {
                id:           scene_id,
                parent_id:    Some(chapter_id),
                kind:         NodeKind::Scene,
                title:        format!("Scene {}", s + 1),
                position:     rank_for(s + 1),
                status:       NodeStatus::Drafting,
                pov:          None,
                beat:         None,
                target_words: Some(WORDS_PER_SCENE as u32),
                created_at:   now,
                updated_at:   now,
                deleted_at:   None,
            });
        }
    }

    storage.insert_nodes_batch(&all_nodes).await.expect("insert batch");

    let pm_doc = build_pm_doc(WORDS_PER_SCENE);
    let pm_doc_bytes = serde_json::to_vec(&pm_doc).unwrap();
    let hash = blake3::hash(&pm_doc_bytes).to_hex().to_string();

    for scene_id in scene_ids {
        let sc = SceneContent {
            node_id:    scene_id,
            pm_doc:     pm_doc.clone(),
            word_count: WORDS_PER_SCENE as u32,
            char_count: (WORDS_PER_SCENE * 6) as u32, // ~6 chars/word incl. space
            hash:       hash.clone(),
            updated_at: now,
        };
        storage.save_scene(&sc).await.expect("save scene");
    }
}

fn rank_for(n: usize) -> String {
    // Simple LexoRank-ish positional string sufficient for the perf seed.
    format!("0|i{n:05x}:")
}

fn build_pm_doc(word_count: usize) -> serde_json::Value {
    // 20-word paragraphs to keep the JSON node count similar to a real scene.
    const WORDS_PER_PARA: usize = 20;
    let mut paragraphs: Vec<serde_json::Value> = Vec::with_capacity(word_count / WORDS_PER_PARA + 1);
    let mut remaining = word_count;
    while remaining > 0 {
        let n = remaining.min(WORDS_PER_PARA);
        let mut text = String::with_capacity(n * 6);
        for i in 0..n {
            if i > 0 { text.push(' '); }
            text.push_str("alpha");
        }
        paragraphs.push(serde_json::json!({
            "type":    "paragraph",
            "content": [{ "type": "text", "text": text }],
        }));
        remaining -= n;
    }
    serde_json::json!({ "type": "doc", "content": paragraphs })
}
