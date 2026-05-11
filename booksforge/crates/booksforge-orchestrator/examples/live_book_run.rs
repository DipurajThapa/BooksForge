//! Headless live-Ollama smoke run for the integrated BooksForge
//! pipeline. Bypasses the Tauri shell entirely — drives the
//! `Orchestrator` API directly from a terminal-friendly Rust binary
//! so we can run the full intake → outline → bibles → drafter →
//! critic → polish → tells flow without a graphical session.
//!
//! Default configuration matches the user's machine (Apple Silicon,
//! qwen3.5:9b + qwen3.5:27b loaded in Ollama at 127.0.0.1:11434) and
//! the literary-fiction baseline from
//! `artifacts/ghostwriter/PROOF_RESULTS_LITERARY.md`.
//!
//! Run with:
//!
//!     cargo run --example live_book_run -p booksforge-orchestrator --release
//!
//! Output goes to stdout (per-stage timings + scores) and to
//! `book-output/integrated-runs/<timestamp>/` (full prose + JSON
//! payloads of each stage).
//!
//! This is a smoke run, not a production benchmark — it drafts ONE
//! literary scene end-to-end. To scale up to a 2-chapter run for
//! direct comparison with the 4.66/3.93/2.08 baseline, multiply the
//! per-scene loop in `main` by N. Each scene takes ~5–10 min on
//! Apple Silicon with Tier-1 optimizations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;
use std::time::Instant;

use booksforge_anti_ai_tells::tells_per_1000_words;
use booksforge_domain::{BookKind, Node, NodeKind, NodeStatus, PolishStageId, ProjectBrief};
use booksforge_fs::{BundleFilesystem, BundlePath, OsFilesystem};
use booksforge_genre_packs::{pack_for, skip_reason};
use booksforge_ollama::client::{HttpOllamaClient, OllamaClient};
use booksforge_ollama::types::CancelToken;
use booksforge_orchestrator::{
    creative_profile::CreativeProfile, runner::RunContext, Orchestrator, OrchestratorConfig,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use booksforge_voice::fingerprint;
use chrono::Utc;
use ulid::Ulid;

// ── Tunables ────────────────────────────────────────────────────────────────

/// The light model used for low-stakes structural calls (intake, critic,
/// vocab dictionary). Tier-1 O2 routes the critic here.
const MODEL_LIGHT: &str = "qwen3.5:9b";

/// The heavy model used for prose generation + polish stack.
///
/// Switched from `qwen3.5:27b` (dense) to `qwen3.6:latest` (Qwen 3.5
/// MoE 36B) after observing in run #2 that the dense 27B chronically
/// stalls on Apple Silicon despite full GPU offload. The MoE
/// architecture activates only ~3-5B parameters per generated token,
/// so end-to-end throughput is typically 2-4× faster than the dense
/// 27B for comparable quality on long-form generative tasks. Same
/// 4-bit quantization (Q4_K_M), much wider context (262k vs 32k).
const MODEL_HEAVY: &str = "qwen3.6:latest";

/// Brief input — matches the literary-fiction proof spec from
/// artifacts/ghostwriter/PROOF_RESULTS_LITERARY.md so the comparison is
/// apples-to-apples.
const IDEA_TEXT: &str = "\
A spare literary novel set in 1990s rural Pennsylvania, told from the \
perspective of a clockmaker's widow who discovers her husband's hidden \
correspondence after his sudden death. Themes: inheritance, grief, the \
weight of accumulated time. Comp authors: Marilynne Robinson, Cormac \
McCarthy. No chosen-one tropes, no AI-style 'tapestry of memory' \
language.";

const SCENE_GOAL:     &str = "The widow opens the locked drawer in her late husband's workshop for the first time and finds the bundle of letters.";
const SCENE_CONFLICT: &str = "She does not yet know whether to read them; her hand resists the act of intrusion even as her eyes have already begun.";
const SCENE_REVEAL:   &str = "The top letter is dated three weeks before he died, addressed to a woman she has never heard of.";
const TARGET_WORDS: u32 = 1_500;
const CHAPTER_POV: &str = "third-limited";

// ── Main ────────────────────────────────────────────────────────────────────

// Diagnostic eprintlns are deliberate in this example binary — it's the
// single integrated runner an operator invokes to drive a real book run
// against a live Ollama, and stderr is the natural channel for
// human-readable progress notes that should not pollute stdout (which
// the user may pipe to a file).
#[allow(clippy::print_stderr)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let total_start = Instant::now();
    println!("=== BooksForge integrated live run ===");
    println!("Time: {}", Utc::now().to_rfc3339());

    // 1. Probe Ollama. If it isn't reachable, fail fast.
    let ollama: Arc<dyn OllamaClient> = Arc::new(HttpOllamaClient::new());
    match ollama.version().await {
        Ok(v) => println!("Ollama reachable: version {}", v.version),
        Err(e) => {
            eprintln!("Ollama not reachable: {e}. Start it with `ollama serve` and rerun.");
            std::process::exit(1);
        }
    }
    let local = ollama.list_local_models().await.unwrap_or_default();
    let names: Vec<&str> = local.iter().map(|m| m.name.as_str()).collect();
    println!("Local models: {}", names.join(", "));
    for needed in [MODEL_LIGHT, MODEL_HEAVY] {
        if !names.iter().any(|n| n.starts_with(needed)) {
            eprintln!("Required model {needed} not found. Run `ollama pull {needed}`.");
            std::process::exit(1);
        }
    }

    // 2. Spin up a tempdir bundle and apply migrations. This mirrors what
    //    the Tauri command layer does when the user creates a project.
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle_root = dir.path().join("live-run.booksforge");
    std::fs::create_dir_all(bundle_root.join("snapshots/objects")).expect("mkdir");
    let bundle = BundlePath::new(&bundle_root);
    let pool = open_pool(&bundle.db()).await.expect("open_pool");
    run_migrations(&pool).await.expect("migrations");
    let storage = Arc::new(SqliteStorage::new(pool));

    // Seed a single scene node so the orchestrator's apply paths have a
    // target. (We don't apply in this smoke; we just inspect the typed
    // proposals as they come back. Apply is exercised in unit tests.)
    let scene_id = Ulid::new();
    let now = Utc::now();
    storage
        .insert_node(&Node {
            id: scene_id,
            parent_id: None,
            kind: NodeKind::Scene,
            title: "The locked drawer".into(),
            position: Node::DEFAULT_POSITION.into(),
            status: NodeStatus::Drafting,
            pov: Some(CHAPTER_POV.into()),
            beat: None,
            target_words: Some(TARGET_WORDS),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
        .await
        .expect("insert_node");

    let storage_trait: Arc<dyn StorageRepository> = storage.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    let snapshot = Arc::new(SnapshotService::new(storage_trait, fs, bundle));
    let orchestrator = Orchestrator::new(
        ollama.clone(),
        storage.clone(),
        OrchestratorConfig::default(),
    )
    .with_snapshot(snapshot);

    let project_id = Ulid::new();

    // 3. Intake (light model — Tier-1 O2 routing)
    let stage = StageTimer::new("intake");
    let intake_r = orchestrator
        .run_intake(
            project_id,
            IDEA_TEXT.to_owned(),
            Some("fiction".to_owned()),
            MODEL_LIGHT.to_owned(),
            CancelToken::new(),
        )
        .await
        .expect("run_intake");
    stage.done();
    let brief: ProjectBrief = intake_r
        .output
        .expect("intake produced no brief — check Ollama logs");
    println!("  brief.premise: {}", trim_to(&brief.premise, 120));
    println!("  brief.themes : {:?}", brief.theme_keywords);
    println!("  brief.comps  : {:?}", brief.comp_titles_or_authors);
    println!("  brief.seed   : {:?}", brief.creative_seed);

    // Build a creative_profile from the brief — feeds every downstream
    // stage so the prose carries the writer's signature signals.
    let creative_profile = CreativeProfile::from_brief(Some(BookKind::LiteraryFiction), &brief);
    let context = RunContext {
        creative_profile,
        ..RunContext::empty()
    };

    let pack = pack_for(BookKind::LiteraryFiction);

    // 4. Character bible — Round 7 RCA fix.
    //
    //    Original `run_character_bible` asks the model for 4-6 nested
    //    objects with cross-coupled constraints in a single response.
    //    Run #2 (qwen3.5:27b): exceeded 23 min and was killed.
    //    Run #3 (qwen3.5:9b): took 11.6 min, returned empty array
    //    because the runner cycled through max retries on validation
    //    failures.
    //
    //    The chunked variant generates ONE character per call,
    //    sequentially, feeding prior characters as context so names
    //    don't collide and relationships reference real prior names.
    //    Per-call output is ~250-400 tokens — fits 9b competence on
    //    paper. Run #6 observed 1 of 4 cards succeeded on 9b (the
    //    protagonist), with the antagonist exhausting retries on the
    //    chapter_arc length constraint. Round 7+ routes to MODEL_HEAVY
    //    (MoE 36B) — only activates ~3-5B params per token so speed
    //    stays comparable to dense 9b but capability is higher; AND
    //    the helper is now lenient (continues past failed cards;
    //    final cross-character validate decides if the assembled set
    //    is publishable).
    let stage = StageTimer::new("character_bible (chunked)");
    let cb = orchestrator
        .run_character_bible_chunked(
            project_id,
            serde_json::to_value(&brief).unwrap(),
            2, // chapter_count
            4, // desired character count: protagonist + antagonist + 2 supporting
            context.clone(),
            MODEL_HEAVY.to_owned(),
            CancelToken::new(),
        )
        .await
        .expect("run_character_bible_chunked");
    stage.done();
    println!("  characters.count: {}", cb.characters.len());
    for c in &cb.characters {
        println!("    - {} ({})", c.name, c.role);
    }
    // FEATURE_HARDENING_PLAN.md §1.6 — attach a numeric voice contract
    // to the bible. The drafter prompt template (scene-drafter-fic v1)
    // renders `voice_target_directive` from this; without it, the
    // drafter would fall back to the freeform per-character
    // voice_traits strings (which the Run #11 quality review showed
    // produce uniformly short prose because the model reads
    // descriptive constraints as ceilings).
    let mut cb_with_target = cb.clone();
    cb_with_target.voice_target = Some(booksforge_voice::VoiceTarget::literary_default());
    println!(
        "  voice_target attached: {}",
        cb_with_target.voice_target.as_ref().unwrap().label,
    );
    let cb_json = serde_json::to_value(&cb_with_target).unwrap();

    // 5. World bible — keep it on light model too. The world bible
    //    has fewer cross-coupled constraints than character bible (no
    //    chapter_arc-length match), so the monolithic prompt is more
    //    forgiving. If 9b struggles here we'd add a similar chunked
    //    variant in a future round.
    let stage = StageTimer::new("world_bible");
    let wb_r = orchestrator
        .run_world_bible(
            project_id,
            serde_json::to_value(&brief).unwrap(),
            serde_json::json!({}),
            context.clone(),
            MODEL_HEAVY.to_owned(),
            CancelToken::new(),
        )
        .await
        .expect("run_world_bible");
    stage.done();
    let wb_json = serde_json::to_value(wb_r.output.as_ref()).unwrap();

    // 6. Scene drafter (heavy model)
    let stage = StageTimer::new("scene_drafter_fic");
    let sd_r = orchestrator
        .run_scene_drafter_fic(
            project_id,
            SCENE_GOAL.to_owned(),
            SCENE_CONFLICT.to_owned(),
            SCENE_REVEAL.to_owned(),
            TARGET_WORDS,
            CHAPTER_POV.to_owned(),
            "literary_fiction".to_owned(),
            cb_json.clone(),
            wb_json.clone(),
            String::new(), // voice_constraints — none yet
            String::new(), // prior_summary — first scene
            context.clone(),
            MODEL_HEAVY.to_owned(),
            CancelToken::new(),
            None,
        )
        .await
        .expect("run_scene_drafter_fic");
    stage.done();
    eprintln!(
        "  [drafter] status={:?} output_present={} error={:?}",
        sd_r.status,
        sd_r.output.is_some(),
        sd_r.error,
    );
    if let Some(raw) = sd_r.raw_output.as_deref() {
        let preview: String = raw
            .chars()
            .take(400)
            .collect::<String>()
            .replace('\n', " ⏎ ");
        eprintln!("  [drafter] raw_preview: {preview}");
    }
    let scene_pm = sd_r
        .output
        .as_ref()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .unwrap_or_default();
    let mut current_text = pm_doc_to_plain(&scene_pm);
    let initial_words = current_text.split_whitespace().count();
    if initial_words == 0 {
        eprintln!(
            "  [drafter] WARN: 0 words after pm_doc_to_plain. scene_pm shape: {}",
            serde_json::to_string(&scene_pm)
                .unwrap_or_default()
                .chars()
                .take(400)
                .collect::<String>(),
        );
    }
    println!(
        "  drafted {} words ({}× target)",
        initial_words,
        fmt_pct_of(initial_words, TARGET_WORDS as usize)
    );

    // 7. Scene critic (light model — Tier-1 O2 routing)
    let stage = StageTimer::new("scene_critic");
    let _critic_r = orchestrator
        .run_scene_critic(
            project_id,
            current_text.clone(),
            SCENE_GOAL.to_owned(),
            SCENE_CONFLICT.to_owned(),
            SCENE_REVEAL.to_owned(),
            pack.critic_axes.clone(),
            pack.genre_label.clone(),
            String::new(),
            String::new(),
            context.clone(),
            MODEL_LIGHT.to_owned(),
            CancelToken::new(),
        )
        .await
        .expect("run_scene_critic");
    stage.done();

    // 8. Polish stack — genre-ordered. O1 conditional skip is honoured.
    for stage_str in &pack.polish_stack_order {
        let Some(stage_id) = PolishStageId::from_str(stage_str) else {
            println!("  unknown polish stage {stage_str}; skipping");
            continue;
        };
        if let Some(reason) = skip_reason(stage_id, &current_text) {
            println!("polish:{stage_str} SKIPPED — {reason}");
            continue;
        }
        let stage = StageTimer::new(&format!("polish:{stage_str}"));
        let polish_r = orchestrator
            .run_polish_stage(
                project_id,
                stage_id,
                current_text.clone(),
                pack.genre_label.clone(),
                String::new(),
                CHAPTER_POV.to_owned(),
                context.clone(),
                MODEL_HEAVY.to_owned(),
                CancelToken::new(),
                None,
            )
            .await
            .expect("run_polish_stage");
        stage.done();
        if let Some(p) = polish_r.output {
            let revised = serde_json::to_value(&p).unwrap_or_default();
            let revised_text = revised
                .get("revised_chapter")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .unwrap_or_default();
            if !revised_text.is_empty() {
                current_text = revised_text;
            }
        }
    }

    // 9. Final tells scan + voice fingerprint (deterministic; no LLM)
    let tells = tells_per_1000_words(&current_text);
    let voice = fingerprint(&current_text);
    let final_words = current_text.split_whitespace().count();

    println!("\n=== Final score card ===");
    println!("Final words           : {final_words}");
    println!("Tells verdict         : {}", tells.verdict);
    println!(
        "Tells weighted/1000   : {:.2}",
        tells.weighted_density_per_1000
    );
    println!(
        "Voice median sent len : {:.1} (p25 {:.1} / p75 {:.1})",
        voice.median_sentence_length, voice.p25_sentence_length, voice.p75_sentence_length,
    );
    println!("Voice em-dash/1000    : {:.2}", voice.em_dash_per_1000);
    println!("Voice dialogue ratio  : {:.2}", voice.dialogue_ratio);
    println!("Voice type-token ratio: {:.3}", voice.type_token_ratio);
    println!(
        "Total wall-clock      : {:.1}s ({:.1} min)",
        total_start.elapsed().as_secs_f32(),
        total_start.elapsed().as_secs_f32() / 60.0,
    );

    // 10. Write artifacts to book-output/integrated-runs/
    let out_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../book-output/integrated-runs")
        .join(Utc::now().format("%Y%m%d-%H%M%S").to_string());
    std::fs::create_dir_all(&out_root).expect("create out_root");
    std::fs::write(
        out_root.join("scene_final.md"),
        format!("# Final scene\n\n{current_text}\n"),
    )
    .ok();
    std::fs::write(
        out_root.join("brief.json"),
        serde_json::to_string_pretty(&brief).unwrap_or_default(),
    )
    .ok();
    std::fs::write(
        out_root.join("character_bible.json"),
        serde_json::to_string_pretty(&cb_json).unwrap_or_default(),
    )
    .ok();
    std::fs::write(
        out_root.join("world_bible.json"),
        serde_json::to_string_pretty(&wb_json).unwrap_or_default(),
    )
    .ok();
    std::fs::write(
        out_root.join("score_card.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "final_words":        final_words,
            "tells_verdict":      tells.verdict,
            "tells_weighted_per_1000": tells.weighted_density_per_1000,
            "voice_median_sentence_len": voice.median_sentence_length,
            "voice_em_dash":             voice.em_dash_per_1000,
            "voice_dialogue_ratio":      voice.dialogue_ratio,
            "voice_type_token_ratio":    voice.type_token_ratio,
            "total_seconds":      total_start.elapsed().as_secs_f32(),
        }))
        .unwrap_or_default(),
    )
    .ok();
    println!("\nArtifacts: {}", out_root.display());
}

// ── Helpers ─────────────────────────────────────────────────────────────────

struct StageTimer {
    label: String,
    start: Instant,
}

impl StageTimer {
    fn new(label: &str) -> Self {
        println!("\n→ {label} starting…");
        Self {
            label: label.to_owned(),
            start: Instant::now(),
        }
    }
    fn done(self) {
        let elapsed = self.start.elapsed();
        println!(
            "✓ {} done in {:.1}s ({:.1} min)",
            self.label,
            elapsed.as_secs_f32(),
            elapsed.as_secs_f32() / 60.0,
        );
    }
}

fn pm_doc_to_plain(doc: &serde_json::Value) -> String {
    let mut out = String::new();
    fn walk(node: &serde_json::Value, out: &mut String) {
        if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
            out.push_str(text);
        }
        if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
            for child in content {
                walk(child, out);
            }
        }
        if let Some(t) = node.get("type").and_then(|t| t.as_str()) {
            if t == "paragraph" || t == "heading" {
                out.push_str("\n\n");
            }
        }
    }
    if let Some(pm) = doc.get("pm_doc") {
        walk(pm, &mut out);
    } else {
        walk(doc, &mut out);
    }
    out.trim().replace("\n\n\n", "\n\n")
}

fn trim_to(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_owned()
    } else {
        format!("{}…", &s[..n])
    }
}

fn fmt_pct_of(actual: usize, target: usize) -> String {
    if target == 0 {
        return "n/a".into();
    }
    format!("{:.0}%", (actual as f32 / target as f32) * 100.0)
}
