//! Bootstrap example — ingest Run #14's prose into the
//! `agent_exemplars` table so subsequent drafter calls receive
//! it as in-context examples.
//!
//! Item 5 of FEATURE_HARDENING_PLAN. Run #14 produced 501 words
//! of literary-fiction prose that the quality review judged
//! "publishable-feeling." Those paragraphs are exemplar #1 of the
//! self-learning compounding-quality store. Without seeding,
//! the table is empty and `fetch_top_exemplars` returns nothing
//! — every drafter run starts cold.
//!
//! Usage:
//!
//!   cargo run --example bootstrap_exemplars -p booksforge-orchestrator -- \
//!       <bundle_db_path>
//!
//! Reads `book-output/integrated-runs/20260510-110737/scene_final.md`
//! (the canonical Run #14 prose), splits it into paragraphs, scores
//! each via `booksforge_anti_ai_tells::score_paragraph`, and inserts
//! every paragraph that scores ≥ 5.0 into the target SQLite bundle's
//! `agent_exemplars` table for the `scene-drafter-fic` agent.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr
)]

use std::path::PathBuf;

use booksforge_anti_ai_tells::{rank_paragraphs, ParagraphQualityScore};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage};
use ulid::Ulid;

const RUN_14_SCENE_PATH: &str =
    "/Users/dipurajthapa/Work/AIProjects/BooksForge/book-output/integrated-runs/20260510-110737/scene_final.md";

const QUALITY_THRESHOLD: f64 = 5.0;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bundle_db_path: PathBuf = if args.len() >= 2 {
        PathBuf::from(&args[1])
    } else {
        // Default — write next to the source artifact so the DB
        // survives the process for inspection. (Avoids `mem::forget`
        // on a tempdir, which clippy correctly flags.)
        let path = std::env::temp_dir().join("bootstrap-demo.db");
        println!(
            "(no path arg given — using {} for the bootstrap DB)",
            path.display()
        );
        path
    };

    println!("=== Exemplar bootstrap — Run #14 → agent_exemplars ===");
    println!("Target DB: {}", bundle_db_path.display());

    // Read the Run #14 scene.
    let scene_text = std::fs::read_to_string(RUN_14_SCENE_PATH).unwrap_or_else(|e| {
        eprintln!(
            "Could not read Run #14 prose at {RUN_14_SCENE_PATH}: {e}\n\
             This is the bootstrap source; ensure the file exists."
        );
        std::process::exit(1);
    });
    // Strip the markdown header — splitter would otherwise pick it up
    // as a single-sentence paragraph and waste a quality-score slot.
    let scene: String = scene_text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    println!("Read Run #14 scene ({} bytes)", scene.len());

    // Open / migrate the target DB.
    let pool = open_pool(&bundle_db_path).await.expect("open_pool");
    run_migrations(&pool).await.expect("migrations");
    let storage = SqliteStorage::new(pool);

    // Score every paragraph — best first.
    let ranked = rank_paragraphs(&scene);
    println!("\n=== Paragraph quality scores ===");
    for (i, (snippet, score)) in ranked.iter().enumerate() {
        let preview: String = snippet.chars().take(60).collect();
        println!(
            "  #{:<2} score={:>4.2}  ({} words)  '{}…'",
            i + 1,
            score.overall,
            score.word_count,
            preview,
        );
    }

    // Insert every paragraph at or above the threshold.
    let project_id = Ulid::new();
    let mut inserted = 0u32;
    let mut skipped_low_quality = 0u32;
    println!("\n=== Inserting (threshold ≥ {QUALITY_THRESHOLD:.1}) ===");
    for (snippet, score) in &ranked {
        if score.overall < QUALITY_THRESHOLD {
            skipped_low_quality += 1;
            continue;
        }
        let tags = derive_tags(snippet, score);
        let id = storage
            .insert_exemplar(
                project_id,
                "scene-drafter-fic",
                snippet,
                score.overall,
                normalised_voice_match(score),
                &tags,
                None, // Run #14 was a pre-exemplar run; no source_run_id
            )
            .await
            .expect("insert_exemplar");
        println!("  ✓ {} (score {:.2}, tags={:?})", id, score.overall, tags,);
        inserted += 1;
    }

    let final_count = storage
        .fetch_top_exemplars("scene-drafter-fic", None, 100)
        .await
        .map(|v| v.len())
        .unwrap_or(0);

    println!("\n=== Summary ===");
    println!("  inserted              : {inserted}");
    println!("  skipped (low quality) : {skipped_low_quality}");
    println!("  total in DB now       : {final_count}");
    println!(
        "\nFuture drafter runs will receive the top exemplars in\n\
         their prompt as in-context examples — house-style learning\n\
         compounds across runs."
    );
}

/// Heuristic tags derived from the score breakdown. These let
/// future readers filter exemplars by scene-shape.
fn derive_tags(_snippet: &str, score: &ParagraphQualityScore) -> Vec<String> {
    let mut tags = Vec::new();
    if score.sensory > 1.0 {
        tags.push("sensory".to_owned());
    }
    if score.figurative > 0.7 {
        tags.push("figurative".to_owned());
    }
    if score.rhythm > 1.0 {
        tags.push("varied-rhythm".to_owned());
    }
    if score.no_structural > 1.5 {
        tags.push("clean-structure".to_owned());
    }
    if tags.is_empty() {
        tags.push("baseline".to_owned());
    }
    tags
}

/// Normalise the score's overall to a 0.0-1.0 stand-in for
/// voice_profile_match (we don't have a per-paragraph profile
/// distance to compare against without a target). The exemplar
/// schema requires the field; we use overall/10 as a reasonable
/// proxy for the bootstrap path.
fn normalised_voice_match(score: &ParagraphQualityScore) -> f64 {
    (score.overall / 10.0).clamp(0.0, 1.0)
}
