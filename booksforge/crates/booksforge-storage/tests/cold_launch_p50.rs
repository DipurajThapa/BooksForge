#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout, clippy::print_stderr, clippy::unimplemented)]

//! C6 — Cold-launch p50 perf gate.
//!
//! The single-iteration `cold_open_perf.rs` test asserts the cold path
//! is *under* 2 seconds.  This test is the stricter spec gate: N=10
//! cold launches, each on its own fresh SQLite file, with the **median**
//! latency required to meet the BACKLOG §C6 budget.
//!
//! ## What "cold launch" means here
//!
//! For the storage layer, a cold launch is:
//!   1. `open_pool(<fresh path>)` — first SQLite open, allocates pool.
//!   2. `run_migrations(&pool)` — applies every migration up to head.
//!   3. `list_nodes_with_scene_content_consistent()` — the same call
//!      the `node_list` Tauri command makes when the app's binder
//!      first renders.
//!
//! We do *not* measure pool warmup or WAL checkpoint cost — those are
//! amortised across a session.
//!
//! ## Fixture
//!
//! Smaller than `cold_open_perf` (10 chapters × 5 scenes × 200 words ≈
//! 10 000 words).  At MVP, a 10k-word project is the realistic median
//! for "open a project I worked on yesterday"; the 100k case is the
//! stress test.
//!
//! ## Running
//!
//! ```bash
//! cargo test -p booksforge-storage --release --test cold_launch_p50 -- --ignored
//! ```
//!
//! CI runs this on the gating macOS runner (see `.github/workflows/ci.yml`
//! `perf-cold-launch` job).

use std::sync::Arc;
use std::time::{Duration, Instant};

use booksforge_domain::{Node, NodeKind, NodeStatus, SceneContent};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use ulid::Ulid;

const ITERATIONS:        usize = 10;
const CHAPTER_COUNT:     usize = 10;
const SCENES_PER_CHAPTER: usize = 5;
const WORDS_PER_SCENE:   usize = 200;

/// Per-launch p50 budget.  The BACKLOG §C6 spec calls for ≤1000 ms p50
/// on macos-14 for the *full* app cold launch; the storage slice gets
/// the lion's share of that budget but not all of it (Tauri builder
/// construction + window paint are the rest).  900 ms is the storage
/// budget.  If your dev machine is slow, run the test in release mode.
const P50_BUDGET_MS: u128 = 900;
/// Per-launch p95 budget — generous tail to absorb GC/IO noise.
const P95_BUDGET_MS: u128 = 1_500;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // perf — opt-in.  Run with `--ignored`; CI runs in release mode.
async fn cold_launch_p50_within_budget() {
    let mut samples: Vec<Duration> = Vec::with_capacity(ITERATIONS);

    for i in 0..ITERATIONS {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("cold.db");

        // ── Seed ─────────────────────────────────────────────
        // Done outside the timing window — only the cold-open
        // (open + migrate + list) is measured.
        {
            let pool = open_pool(&db_path).await.expect("seed: open pool");
            run_migrations(&pool).await.expect("seed: migrate");
            let storage = Arc::new(SqliteStorage::new(pool));
            seed_10k(&storage).await;
        }

        // ── Time the cold path ───────────────────────────────
        let t0 = Instant::now();
        let pool = open_pool(&db_path).await.expect("cold: open pool");
        run_migrations(&pool).await.expect("cold: migrate (no-op)");
        let storage = Arc::new(SqliteStorage::new(pool));
        let (nodes, _scenes) = storage
            .list_nodes_with_scene_content_consistent()
            .await
            .expect("cold: list nodes");
        let elapsed = t0.elapsed();
        samples.push(elapsed);

        // Light correctness check so a regression in the storage layer
        // can't pretend to be fast by returning empty results.
        let scenes = nodes.iter().filter(|n| matches!(n.kind, NodeKind::Scene)).count();
        assert_eq!(scenes, CHAPTER_COUNT * SCENES_PER_CHAPTER,
                   "iter {i}: scene count mismatch");
    }

    samples.sort();
    let min = samples.first().expect("at least one sample").as_millis();
    let p50 = samples[ITERATIONS / 2].as_millis();
    let p95 = samples[((ITERATIONS as f64) * 0.95).ceil() as usize - 1].as_millis();
    let max = samples.last().expect("at least one sample").as_millis();

    println!(
        "[cold-launch-p50] N={ITERATIONS}  min={min}ms  p50={p50}ms  p95={p95}ms  max={max}ms  \
         (budget: p50≤{P50_BUDGET_MS}ms, p95≤{P95_BUDGET_MS}ms)"
    );

    assert!(
        p50 <= P50_BUDGET_MS,
        "p50 cold-launch latency {p50}ms exceeded budget {P50_BUDGET_MS}ms — see BACKLOG §C6"
    );
    assert!(
        p95 <= P95_BUDGET_MS,
        "p95 cold-launch latency {p95}ms exceeded budget {P95_BUDGET_MS}ms — see BACKLOG §C6"
    );
}

// ── Fixture (smaller variant of cold_open_perf::seed_100k_word_manuscript) ──

async fn seed_10k(storage: &Arc<SqliteStorage>) {
    let now = Utc::now();
    let project_id = Ulid::new();
    let project = Node {
        id: project_id, parent_id: None, kind: NodeKind::Project,
        title: "10k cold-launch fixture".into(),
        position: "0|hzzzzz:".into(),
        status: NodeStatus::Drafting,
        pov: None, beat: None, target_words: Some(10_000),
        created_at: now, updated_at: now, deleted_at: None,
    };

    let mut all_nodes: Vec<Node> = Vec::with_capacity(1 + CHAPTER_COUNT * (1 + SCENES_PER_CHAPTER));
    all_nodes.push(project);
    let mut scene_ids: Vec<Ulid> = Vec::with_capacity(CHAPTER_COUNT * SCENES_PER_CHAPTER);

    for c in 0..CHAPTER_COUNT {
        let chapter_id = Ulid::new();
        all_nodes.push(Node {
            id: chapter_id, parent_id: Some(project_id),
            kind: NodeKind::Chapter,
            title: format!("Chapter {}", c + 1),
            position: format!("0|i{:05x}:", c + 1),
            status: NodeStatus::Drafting,
            pov: None, beat: None, target_words: None,
            created_at: now, updated_at: now, deleted_at: None,
        });
        for s in 0..SCENES_PER_CHAPTER {
            let scene_id = Ulid::new();
            scene_ids.push(scene_id);
            all_nodes.push(Node {
                id: scene_id, parent_id: Some(chapter_id),
                kind: NodeKind::Scene,
                title: format!("Scene {}", s + 1),
                position: format!("0|i{:05x}:", s + 1),
                status: NodeStatus::Drafting,
                pov: None, beat: None, target_words: Some(WORDS_PER_SCENE as u32),
                created_at: now, updated_at: now, deleted_at: None,
            });
        }
    }

    storage.insert_nodes_batch(&all_nodes).await.expect("insert nodes");

    let pm_doc = build_pm_doc(WORDS_PER_SCENE);
    let pm_doc_bytes = serde_json::to_vec(&pm_doc).expect("serialize pm_doc");
    let hash = blake3::hash(&pm_doc_bytes).to_hex().to_string();
    for scene_id in scene_ids {
        storage.save_scene(&SceneContent {
            node_id:    scene_id,
            pm_doc:     pm_doc.clone(),
            word_count: WORDS_PER_SCENE as u32,
            char_count: (WORDS_PER_SCENE * 6) as u32,
            hash:       hash.clone(),
            updated_at: now,
        }).await.expect("save scene");
    }
}

fn build_pm_doc(word_count: usize) -> serde_json::Value {
    const WORDS_PER_PARA: usize = 20;
    let mut paragraphs: Vec<serde_json::Value> = Vec::new();
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
