//! Multi-chapter live run — produces a 2-chapter manuscript draft
//! through the full BooksForge pipeline (intake → bibles → world →
//! drafter+critic+polish per scene).
//!
//! Scope: 2 chapters × 2 scenes = 4 total scenes. At Run #14's
//! observed pace (~16 min per scene end-to-end), expected wall-clock
//! is ~70 min. The shared setup (intake + chunked bibles + world
//! bible) runs ONCE and is reused across all four scenes.
//!
//! Output: a single manuscript markdown file under
//! `book-output/multi-chapter-runs/<timestamp>/manuscript.md` with
//! `# Chapter 1` / `# Chapter 2` headings and per-scene polished prose.
//! A separate `score_card.json` records per-scene timings and metrics.
//!
//! This example is the smallest realistic test of the pipeline's
//! ability to produce a coherent multi-scene narrative — short
//! enough to complete in one work session, long enough to evaluate
//! continuity, voice consistency, and storytelling craft across more
//! than a single moment.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines
)]

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

// Three-tier model routing (2026-05-15 refactor — reserve the heaviest
// model for the steps that genuinely need its critical-thinking budget).
//
//   LIGHT  qwen3.5:9b     basic structured-output tasks (intake,
//                         character/world bible, scene critic)
//   MEDIUM qwen3.5:27b    creative drafting — scene_drafter_fic. The
//                         dense 27B produces consistent literary prose
//                         and is fast enough on Apple Silicon. Using
//                         9B here was visibly hurting voice quality
//                         (frequent AI-tell phrasing); using 3.6
//                         here burns context for prose that isn't
//                         doing the work the MoE excels at.
//   HEAVY  qwen3.6:latest final polish ONLY — voice / metaphor /
//                         dialogue / scene-tension. The MoE's wider
//                         context (262k) and reasoning budget pay
//                         off here because the polish stack reads the
//                         entire scene and emits a craft-grade rewrite
//                         that benefits from interpretive analysis.
const MODEL_LIGHT: &str = "qwen3.5:9b";
const MODEL_MEDIUM: &str = "qwen3.5:27b";
const MODEL_HEAVY: &str = "qwen3.6:latest";

// IDEA_TEXT swapped for "My Confused Life" — same canonical pipeline,
// different premise. Sourced from book-output/my-confused-life/01-brief.json.
const IDEA_TEXT: &str = "Arjun, a burnt-out corporate man in his early thirties, hits a low point marked by job exhaustion, a cold relationship, and the death of a mentor. Through a slow, embarrassing encounter with the devotional tradition of Radha and Krishna, he discovers that peace is a by-product of service and that he can remain in the world while reshaping his sense of self through the practice of sharing more than he takes.";

const CHAPTER_POV: &str = "first-person past tense";
const TARGET_WORDS_PER_SCENE: u32 = 1200;

/// One scene's spec.
struct SceneSpec {
    chapter: u32,
    scene: u32,
    title: &'static str,
    goal: &'static str,
    conflict: &'static str,
    reveal: &'static str,
}

// SCENES[] swapped for "My Confused Life" — 18 scenes across 6 chapters,
// auto-decomposed from 02-outline.json by book-output/outline_to_scenes.py
// using qwen3.5:9b. Pipeline (intake → bibles → drafter → critic →
// polish stack) is unchanged.
const SCENES: &[SceneSpec] = &[
    SceneSpec {
        chapter:  1,
        scene:    1,
        title:    "Ch1 S1 — The Commute Home",
        goal:     "Arjun navigates the chaotic Mumbai traffic while observing the city's energy.",
        conflict: "He feels internally hollowed out by his demanding job.",
        reveal:   "His detachment from his own family is highlighted by his inability to connect with the vibrant life around him.",
    },
    SceneSpec {
        chapter:  1,
        scene:    2,
        title:    "Ch1 S2 — The Commute Home",
        goal:     "Arjun enters his silent apartment and moves through the cold kitchen to prepare a meal.",
        conflict: "His wife's emotional distance and the stark absence of warmth in the home resist his desire for connection and rest.",
        reveal:   "The reader learns that Arjun has become so detached from his own family that he treats the silence of the house as a normal part of his daily routine.",
    },
    SceneSpec {
        chapter:  1,
        scene:    3,
        title:    "Ch1 S3 — The Commute Home",
        goal:     "Arjun receives the news of his mentor's death while navigating the relentless mechanical exhaustion of his daily life in Mumbai.",
        conflict: "This shocking news shatters his remaining sense of professional order and deepens his detachment from his own family.",
        reveal:   "The collapse of his professional composure exposes the fragility of his existence beneath the city's relentless rhythm.",
    },
    SceneSpec {
        chapter:  2,
        scene:    1,
        title:    "Ch2 S1 — The Unraveling",
        goal:     "Arjun attempts to force his mind to focus on spreadsheets and strategy during a work crisis.",
        conflict: "His mind refuses to concentrate, causing his physical and emotional state to collapse under the pressure of functioning without his usual anchors.",
        reveal:   "The reader learns that Arjun's ability to function has completely disintegrated, leaving him unable to engage with his professional responsibilities.",
    },
    SceneSpec {
        chapter:  2,
        scene:    2,
        title:    "Ch2 S2 — The Unraveling",
        goal:     "Arjun wanders through a crowded Mumbai market, navigating the chaotic flow of people while trying to maintain his composure.",
        conflict: "He feels like a ghost among fully alive people, struggling to connect with the vibrant energy around him as his emotional anchors crumble.",
        reveal:   "The physical sensation of the crowd pressing against him highlights the widening gap between his internal isolation and the external reality of life continuing without him.",
    },
    SceneSpec {
        chapter:  2,
        scene:    3,
        title:    "Ch2 S3 — The Unraveling",
        goal:     "Arjun returns to his apartment and lies awake staring at the ceiling.",
        conflict: "He struggles to sleep as the fading memories of his mentor leave him emotionally adrift.",
        reveal:   "The absence of his mentor's guidance causes his physical and emotional life to begin collapsing.",
    },
    SceneSpec {
        chapter:  3,
        scene:    1,
        title:    "Ch3 S1 — The Temple of Names",
        goal:     "Arjun enters a small, crowded temple in the hinterland town and attempts to navigate the space while feeling out of place among devotees chanting names he does not know.",
        conflict: "Arjun's internal skepticism and self-consciousness clash with the overwhelming, unfamiliar spiritual energy of the crowd, making him feel like an intruder in a world he does not understand.",
        reveal:   "Despite his initial resistance, a sudden emotional shift occurs as Arjun is drawn into the collective devotion, realizing the sincerity behind the chants he previously dismissed as meaningless.",
    },
    SceneSpec {
        chapter:  3,
        scene:    2,
        title:    "Ch3 S2 — The Temple of Names",
        goal:     "Arjun observes the vibrant iconography of Radha and Krishna while noting the intense devotion of the surrounding crowd.",
        conflict: "He struggles to comprehend the deep theological significance behind the rituals because he views the scene only through his own skeptical and self-conscious lens.",
        reveal:   "A surprising emotional shift occurs as Arjun begins to feel the weight of the collective faith he previously dismissed.",
    },
    SceneSpec {
        chapter:  3,
        scene:    3,
        title:    "Ch3 S3 — The Temple of Names",
        goal:     "Arjun joins the rhythmic chant of the kirtan despite his initial embarrassment.",
        conflict: "His self-conscious skepticism clashes with the sudden, strange warmth rising in his chest.",
        reveal:   "The immersive power of the devotional world shifts his perspective, transforming his doubt into a moment of unexpected emotional connection.",
    },
    SceneSpec {
        chapter:  4,
        scene:    1,
        title:    "Ch4 S1 — The First Offering",
        goal:     "Arjun returns to the temple town to volunteer for distributing food to the hungry.",
        conflict: "His hands shake violently from the effort of giving, revealing that his ego still resists the act of selfless service.",
        reveal:   "The reader learns that Arjun's desire to help is currently undermined by his own inability to let go of his pride.",
    },
    SceneSpec {
        chapter:  4,
        scene:    2,
        title:    "Ch4 S2 — The First Offering",
        goal:     "Arjun watches a local devotee serve others with quiet joy while he struggles to overcome his own internal resistance to giving.",
        conflict: "His ego clings to the belief that he is not yet worthy of such an act, creating a sharp contrast with the devotee's effortless ease.",
        reveal:   "The scene exposes that his hesitation is not a lack of desire to help, but a stubborn attachment to his own sense of superiority.",
    },
    SceneSpec {
        chapter:  4,
        scene:    3,
        title:    "Ch4 S3 — The First Offering",
        goal:     "Arjun attempts to distribute the remaining supplies to the needy villagers despite his lingering hesitation.",
        conflict: "His deep-seated ego and fear of appearing weak resist the act of giving, causing him to stumble over his words and movements.",
        reveal:   "A sudden, unexplainable sense of lightness washes over him as he finally hands out the last bag, hinting that his internal shift has begun.",
    },
    SceneSpec {
        chapter:  5,
        scene:    1,
        title:    "Ch5 S1 — The Language of Names",
        goal:     "Arjun sits with a group of devotees and listens to the repetition of divine names until the sound becomes a physical vibration.",
        conflict: "The physical effort of maintaining stillness and focus battles the natural tendency of his mind to wander or the vibration to fade.",
        reveal:   "He discovers that the chanting transforms from an external ritual into an internal language that speaks directly to his own heart.",
    },
    SceneSpec {
        chapter:  5,
        scene:    2,
        title:    "Ch5 S2 — The Language of Names",
        goal:     "Arjun begins to weave the names of deities into his daily thoughts.",
        conflict: "The sharp edges of his anxiety resist this new practice.",
        reveal:   "He discovers that the divine names soften his internal turmoil and become a language for his own heart.",
    },
    SceneSpec {
        chapter:  5,
        scene:    3,
        title:    "Ch5 S3 — The Language of Names",
        goal:     "Arjun sits in silence and begins to chant the names of the deities to align his heart with the divine.",
        conflict: "His mind resists the practice by clinging to the urgent need to fix his chaotic life rather than simply receiving peace.",
        reveal:   "He discovers that the act of service itself is the language through which the divine speaks directly to his own heart.",
    },
    SceneSpec {
        chapter:  6,
        scene:    1,
        title:    "Ch6 S1 — The Return",
        goal:     "Arjun walks back into Mumbai traffic, embracing his role as a participant in the city's flow.",
        conflict: "He must reconcile his past detachment with the present reality of being fully engaged in his daily life.",
        reveal:   "The reader learns that peace is no longer a distant destination but a by-product of his daily service.",
    },
    SceneSpec {
        chapter:  6,
        scene:    2,
        title:    "Ch6 S2 — The Return",
        goal:     "Arjun approaches his apartment door to face his wife with a new openness instead of the coldness of the past.",
        conflict: "The weight of his previous emotional distance and the fear of failing to convey his transformed heart before entering.",
        reveal:   "The reader learns that his daily service has reshaped his corporate life into a foundation for genuine peace and connection.",
    },
    SceneSpec {
        chapter:  6,
        scene:    3,
        title:    "Ch6 S3 — The Return",
        goal:     "Arjun enters his home and offers a simple gesture of care to his family.",
        conflict: "He must reconcile his desire to remain in the corporate world with his newfound understanding that true peace comes from daily service.",
        reveal:   "The reader learns that Arjun has successfully reshaped his sense of self by finding peace within his ordinary daily actions.",
    },
];

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // RCA Fix 2 — install tracing subscriber so the runner's per-call
    // telemetry (num_ctx, prompt size, est tokens) shows up in stdout
    // alongside the per-stage timings. RUST_LOG=info turns it on.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("booksforge_orchestrator=info")
            }),
        )
        .init();

    let total_start = Instant::now();
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let out_dir = std::path::PathBuf::from(format!(
        "/Users/dipurajthapa/Work/AIProjects/BooksForge/book-output/multi-chapter-runs/{timestamp}"
    ));
    std::fs::create_dir_all(&out_dir).expect("mkdir out_dir");

    println!("=== BooksForge multi-chapter live run ===");
    println!("Time:       {}", Utc::now().to_rfc3339());
    println!("Output dir: {}", out_dir.display());
    println!("Scenes:     {}", SCENES.len());

    // ── Probes ────────────────────────────────────────────────────────────
    let ollama: Arc<dyn OllamaClient> = Arc::new(HttpOllamaClient::new());
    match ollama.version().await {
        Ok(v) => println!("Ollama OK   : {}", v.version),
        Err(e) => {
            eprintln!("Ollama not reachable: {e}. Start `ollama serve` and rerun.");
            std::process::exit(1);
        }
    }
    let local = ollama.list_local_models().await.unwrap_or_default();
    let names: Vec<&str> = local.iter().map(|m| m.name.as_str()).collect();
    for needed in [MODEL_LIGHT, MODEL_MEDIUM, MODEL_HEAVY] {
        if !names.iter().any(|n| n.starts_with(needed)) {
            eprintln!("Required model {needed} not found. `ollama pull {needed}`.");
            std::process::exit(1);
        }
    }

    // ── Bundle setup ──────────────────────────────────────────────────────
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle_root = dir.path().join("multi-chapter.booksforge");
    std::fs::create_dir_all(bundle_root.join("snapshots/objects")).expect("mkdir bundle");
    let bundle = BundlePath::new(&bundle_root);
    let pool = open_pool(&bundle.db()).await.expect("open_pool");
    run_migrations(&pool).await.expect("migrations");
    let storage = Arc::new(SqliteStorage::new(pool));

    // Seed nodes for all scenes so apply paths have targets.
    let now = Utc::now();
    for spec in SCENES {
        let scene_id = Ulid::new();
        storage
            .insert_node(&Node {
                id: scene_id,
                parent_id: None,
                kind: NodeKind::Scene,
                title: format!("Ch{} Sc{} — {}", spec.chapter, spec.scene, spec.title),
                position: Node::DEFAULT_POSITION.into(),
                status: NodeStatus::Drafting,
                pov: Some(CHAPTER_POV.into()),
                beat: None,
                target_words: Some(TARGET_WORDS_PER_SCENE),
                created_at: now,
                updated_at: now,
                deleted_at: None,
            })
            .await
            .expect("insert_node");
    }

    let storage_trait: Arc<dyn StorageRepository> = storage.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    let snapshot = Arc::new(SnapshotService::new(storage_trait, fs, bundle));

    // RCA_RUN15_THRASH.md Fix 1 — pin pipeline num_ctx so qwen3.6 doesn't
    // re-initialise its KV cache for every agent's individual num_ctx
    // (5-15 min reload tax per transition on Apple Silicon). Pinned at
    // 64k = the maximum any agent in this run will need (drafter
    // 32k+32k). All other agents (bibles 24k, world bible 24k, critic
    // 15k, polish 30k) use 64k too — modest extra KV cache, zero reloads.
    const PIPELINE_NUM_CTX: u32 = 64_000;
    let config = OrchestratorConfig::default().with_pipeline_num_ctx(PIPELINE_NUM_CTX);
    let orchestrator =
        Orchestrator::new(ollama.clone(), storage.clone(), config).with_snapshot(snapshot);

    let project_id = Ulid::new();

    // RCA Fix 3 — explicit pre-warm. One throwaway 4-token call to
    // qwen3.6:latest at PIPELINE_NUM_CTX so the model reload + KV
    // cache init costs are paid HERE (where they're labelled and
    // expected) instead of disguised as part of the first agent
    // call (where they look like "the drafter is broken"). Pre-warm
    // also confirms Ollama can hold the model at this num_ctx — a
    // failure here is faster + clearer than a 30-min mystery hang.
    {
        let warm_t = Instant::now();
        println!("\n→ pre-warming {MODEL_HEAVY} at num_ctx={PIPELINE_NUM_CTX}…");
        let warm_req = booksforge_ollama::types::ChatRequest {
            model: MODEL_HEAVY.to_owned(),
            messages: vec![booksforge_ollama::types::ChatMessage::user("ok")],
            stream: true,
            think: Some(false),
            format: None,
            options: Some(booksforge_ollama::types::GenerateOptions {
                temperature: Some(0.0),
                top_p: None,
                num_ctx: Some(PIPELINE_NUM_CTX),
                num_predict: Some(4),
                repeat_penalty: None,
                stop: None,
            }),
        };
        let sink: booksforge_ollama::TokenSink = Box::new(|_t: &str| {});
        match ollama.chat(warm_req, sink, CancelToken::new()).await {
            Ok(_) => println!(
                "✓ pre-warm done in {:.1}s (model loaded + KV cache initialised at {PIPELINE_NUM_CTX} ctx)",
                warm_t.elapsed().as_secs_f32(),
            ),
            Err(e) => {
                eprintln!("pre-warm failed: {e}. Aborting before paying any agent cost.");
                std::process::exit(1);
            }
        }
    }

    // ── Setup: intake + bibles (run ONCE) ────────────────────────────────
    let stage_t = Instant::now();
    println!("\n→ intake starting…");
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
    let brief: ProjectBrief = intake_r
        .output
        .as_ref()
        .map(|b| serde_json::from_value(serde_json::to_value(b).unwrap()).unwrap())
        .expect("intake brief");
    println!("✓ intake done in {:.1}s", stage_t.elapsed().as_secs_f32());

    let creative_profile = CreativeProfile::from_brief(Some(BookKind::LiteraryFiction), &brief);
    let context = RunContext {
        creative_profile,
        ..RunContext::empty()
    };
    let pack = pack_for(BookKind::LiteraryFiction);

    let stage_t = Instant::now();
    println!("\n→ character_bible (chunked) starting…");
    let cb = orchestrator
        .run_character_bible_chunked(
            project_id,
            serde_json::to_value(&brief).unwrap(),
            2, // chapter_count
            4, // characters
            context.clone(),
            // LIGHT — character cards are structured JSON output, not
            // critical-thinking work. The 9B handles voice_traits +
            // chapter_arc reliably within its budget. Reserves the
            // heavy model for the polish stack.
            MODEL_LIGHT.to_owned(),
            CancelToken::new(),
        )
        .await
        .expect("character_bible_chunked");
    println!(
        "✓ bibles done in {:.1}s ({} characters)",
        stage_t.elapsed().as_secs_f32(),
        cb.characters.len()
    );

    let mut cb_with_target = cb.clone();
    cb_with_target.voice_target = Some(booksforge_voice::VoiceTarget::literary_default());
    let cb_json = serde_json::to_value(&cb_with_target).unwrap();

    let stage_t = Instant::now();
    println!("\n→ world_bible starting…");
    let wb_r = orchestrator
        .run_world_bible(
            project_id,
            serde_json::to_value(&brief).unwrap(),
            serde_json::json!({}),
            context.clone(),
            // LIGHT — world bible is structured-output reference data
            // (locations + social rules + sensory palette), not
            // narrative craft work. 9B emits the schema reliably with
            // the coercion + lenient-parse defenses in place.
            MODEL_LIGHT.to_owned(),
            CancelToken::new(),
        )
        .await
        .expect("run_world_bible");
    let wb_json = serde_json::to_value(wb_r.output.as_ref()).unwrap();
    println!(
        "✓ world_bible done in {:.1}s",
        stage_t.elapsed().as_secs_f32()
    );

    // ── Per-scene loop ───────────────────────────────────────────────────
    let mut chapter_prose: std::collections::BTreeMap<u32, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    let mut score_cards: Vec<serde_json::Value> = Vec::new();
    let mut prior_summary = String::new();

    for spec in SCENES {
        let scene_label = format!("Ch{} Sc{} — {}", spec.chapter, spec.scene, spec.title);
        println!("\n────────────────────────────────────────────────");
        println!("=== SCENE {scene_label} ===");

        // Drafter
        let stage_t = Instant::now();
        println!("→ drafter starting…");
        let sd_r = orchestrator
            .run_scene_drafter_fic(
                project_id,
                spec.goal.to_owned(),
                spec.conflict.to_owned(),
                spec.reveal.to_owned(),
                TARGET_WORDS_PER_SCENE,
                CHAPTER_POV.to_owned(),
                "literary_fiction".to_owned(),
                cb_json.clone(),
                wb_json.clone(),
                String::new(),
                prior_summary.clone(),
                context.clone(),
                // MEDIUM — drafting is craft work, not critical-
                // thinking work. Dense 27B produces consistent
                // literary prose with the right voice cadence and
                // doesn't burn the heavy model's reasoning budget
                // on what amounts to prose generation. The polish
                // stack (HEAVY) does the interpretive lifting.
                MODEL_MEDIUM.to_owned(),
                CancelToken::new(),
                None,
            )
            .await
            .expect("run_scene_drafter_fic");
        let drafter_secs = stage_t.elapsed().as_secs_f32();
        let scene_pm = sd_r
            .output
            .as_ref()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .unwrap_or_default();
        let mut current_text = pm_doc_to_plain(&scene_pm);
        let drafted_words = current_text.split_whitespace().count();
        println!("✓ drafter done in {drafter_secs:.1}s ({drafted_words} words)");

        if drafted_words == 0 {
            println!("  WARN: drafter returned 0 words — skipping critic + polish for this scene");
            score_cards.push(serde_json::json!({
                "chapter": spec.chapter, "scene": spec.scene, "title": spec.title,
                "drafted_words": 0, "drafter_secs": drafter_secs,
                "polished_words": 0, "tells_verdict": "n/a",
                "paragraph_quality_overall": 0.0,
            }));
            chapter_prose.entry(spec.chapter).or_default().push((
                spec.title.into(),
                "(drafter produced no prose for this scene)".into(),
            ));
            continue;
        }

        // Run #16 speed fix — critic skip when fresh draft is already
        // PUBLISHABLE on tells AND scores ≥ 6.5/10 on the deterministic
        // paragraph_quality rubric. Saves ~6 min per skip-eligible scene
        // (typically 2-3 of 4 scenes once the pipeline is calibrated).
        let pre_polish_tells = tells_per_1000_words(&current_text);
        let pre_polish_quality = booksforge_anti_ai_tells::score_paragraph(&current_text);
        let critic_skip_eligible =
            pre_polish_tells.verdict == "PUBLISHABLE" && pre_polish_quality.overall >= 6.5;

        if critic_skip_eligible {
            println!(
                "→ critic SKIPPED — fresh draft already PUBLISHABLE (quality {:.2}/10)",
                pre_polish_quality.overall,
            );
        } else {
            // Critic
            let stage_t = Instant::now();
            println!(
                "→ critic starting (pre-polish quality {:.2}/10, tells={})…",
                pre_polish_quality.overall, pre_polish_tells.verdict,
            );
            let _critic_r = orchestrator
                .run_scene_critic(
                    project_id,
                    current_text.clone(),
                    spec.goal.to_owned(),
                    spec.conflict.to_owned(),
                    spec.reveal.to_owned(),
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
            println!("✓ critic done in {:.1}s", stage_t.elapsed().as_secs_f32());
        }

        // Polish stack
        for stage_str in &pack.polish_stack_order {
            let Some(stage_id) = PolishStageId::from_str(stage_str) else {
                continue;
            };
            if let Some(reason) = skip_reason(stage_id, &current_text) {
                println!("  polish:{stage_str} SKIPPED — {reason}");
                continue;
            }
            let stage_t = Instant::now();
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
            println!(
                "  ✓ polish:{stage_str} {:.1}s",
                stage_t.elapsed().as_secs_f32()
            );
            if let Some(p) = polish_r.output {
                let revised = serde_json::to_value(&p).unwrap_or_default();
                if let Some(rev) = revised.get("revised_chapter").and_then(|v| v.as_str()) {
                    if !rev.is_empty() {
                        current_text = rev.to_owned();
                    }
                }
            }
        }

        // Tells + voice fingerprint for this scene's score card.
        // (`tells` and `voice` here are the pre-rhythm-expansion read;
        // both are recomputed AFTER the rhythm pass below for the
        // final scorecard.)
        let tells = tells_per_1000_words(&current_text);
        let polished_words = current_text.split_whitespace().count();
        let quality = booksforge_anti_ai_tells::score_paragraph(&current_text);
        println!(
            "  scene metrics: words={polished_words}, tells={}, quality={:.2}/10 (rhythm {:.2}/2.0)",
            tells.verdict, quality.overall, quality.rhythm,
        );

        // Run #17 axis-targeted polish — when rhythm < 1.5/2.0, run a
        // focused single-LLM-call rewrite that finds runs of 4+ short
        // sentences and merges them into mid-length / long sentences.
        // This is the "post-draft enforcement" the prompt directives
        // alone cannot achieve: qwen3.6 doesn't reliably count its
        // own sentence-length distribution, so a deterministic detector
        // identifies the shortfall and a targeted polish call lifts it.
        if quality.rhythm < 1.5 {
            let rt = Instant::now();
            println!(
                "  → rhythm-expansion polish (current rhythm {:.2}/2.0 — target ≥ 1.5)",
                quality.rhythm,
            );
            let rhythm_prompt = format!(
                "You are a sentence-rhythm editor.\n\n\
                 The prose below has a STACCATO problem: too many runs of 4+ \
                 consecutive sentences of 8 words or fewer. Find every such \
                 run and rewrite the run as 1-2 longer compound or complex \
                 sentences (12-25 words each) that preserve EXACTLY:\n\
                   - the same meaning\n\
                   - every concrete sensory image\n\
                   - the POV character's voice and register\n\
                 Do NOT add new content. Do NOT change the plot. Do NOT remove \
                 any imagery. ONLY merge consecutive short sentences into longer \
                 sentences using commas, semicolons, em-dashes, conjunctions \
                 (\"and\", \"but\", \"because\", \"while\", \"as\").\n\n\
                 Return ONLY the revised prose. No JSON, no commentary.\n\n\
                 PROSE:\n{current_text}"
            );
            let warm_req = booksforge_ollama::types::ChatRequest {
                model: MODEL_HEAVY.to_owned(),
                messages: vec![booksforge_ollama::types::ChatMessage::user(&rhythm_prompt)],
                stream: true,
                think: Some(false),
                format: None,
                options: Some(booksforge_ollama::types::GenerateOptions {
                    temperature: Some(0.3),
                    top_p: None,
                    num_ctx: Some(PIPELINE_NUM_CTX),
                    num_predict: Some(8_000),
                    repeat_penalty: None,
                    stop: None,
                }),
            };
            let revised_buf = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
            let buf_clone = revised_buf.clone();
            let sink: booksforge_ollama::TokenSink = Box::new(move |t: &str| {
                if let Ok(mut b) = buf_clone.lock() {
                    b.push_str(t);
                }
            });
            match ollama.chat(warm_req, sink, CancelToken::new()).await {
                Ok(_) => {
                    let revised = revised_buf.lock().map(|b| b.clone()).unwrap_or_default();
                    let revised_words = revised.split_whitespace().count();
                    if revised_words >= polished_words / 2 {
                        // Sanity guard — accept only if revised prose is at
                        // least half the original length (prevents the
                        // model from "summarising" the scene to nothing).
                        let new_quality = booksforge_anti_ai_tells::score_paragraph(&revised);
                        if new_quality.rhythm > quality.rhythm {
                            current_text = revised;
                            println!(
                                "    ✓ rhythm-expansion {:.1}s — rhythm {:.2} → {:.2}, overall {:.2} → {:.2}",
                                rt.elapsed().as_secs_f32(),
                                quality.rhythm, new_quality.rhythm,
                                quality.overall, new_quality.overall,
                            );
                        } else {
                            println!(
                                "    × rhythm-expansion did not improve rhythm ({:.2} → {:.2}); keeping original",
                                quality.rhythm, new_quality.rhythm,
                            );
                        }
                    } else {
                        println!(
                            "    × rhythm-expansion produced too-short output ({revised_words} words vs original {polished_words}); keeping original"
                        );
                    }
                }
                Err(e) => {
                    println!("    × rhythm-expansion failed: {e}; keeping original");
                }
            }
        }
        // Recompute quality + tells from the (possibly rhythm-expanded) text.
        let tells = tells_per_1000_words(&current_text);
        let voice = fingerprint(&current_text);
        let polished_words = current_text.split_whitespace().count();
        let quality = booksforge_anti_ai_tells::score_paragraph(&current_text);
        println!(
            "  final scene metrics: words={polished_words}, tells={}, quality={:.2}/10, median_sent={:.1}, MATTR={:.2}",
            tells.verdict, quality.overall, voice.median_sentence_length, voice.mattr_50,
        );

        score_cards.push(serde_json::json!({
            "chapter":                 spec.chapter,
            "scene":                   spec.scene,
            "title":                   spec.title,
            "drafted_words":           drafted_words,
            "drafter_secs":            drafter_secs,
            "polished_words":          polished_words,
            "tells_verdict":           tells.verdict,
            "tells_per_1000":          tells.weighted_density_per_1000,
            "paragraph_quality_overall": quality.overall,
            "paragraph_quality_breakdown": {
                "sensory":         quality.sensory,
                "figurative":      quality.figurative,
                "rhythm":          quality.rhythm,
                "mattr":           quality.mattr,
                "no_structural":   quality.no_structural,
                "low_token_tells": quality.low_token_tells,
            },
            "voice_median":    voice.median_sentence_length,
            "voice_mattr_50":  voice.mattr_50,
            "voice_ttr":       voice.type_token_ratio,
            "dialogue_ratio":  voice.dialogue_ratio,
        }));

        // Append to chapter
        chapter_prose
            .entry(spec.chapter)
            .or_default()
            .push((spec.title.into(), current_text.clone()));

        // Update prior summary for the next scene's drafter call.
        let recap: String = current_text.chars().take(800).collect();
        prior_summary = format!(
            "{}\n\nMost recent scene ({}): {}…",
            prior_summary.trim_end(),
            scene_label,
            recap,
        );
    }

    // ── Write manuscript + scorecard ─────────────────────────────────────
    let manuscript_path = out_dir.join("manuscript.md");
    let mut manuscript = String::new();
    manuscript.push_str("# Incomplete Curse — 2-chapter draft\n\n");
    manuscript.push_str(&format!(
        "_Generated by BooksForge {}_\n\n",
        Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));
    for (chapter, scenes) in &chapter_prose {
        manuscript.push_str(&format!("\n# Chapter {chapter}\n\n"));
        for (title, prose) in scenes {
            manuscript.push_str(&format!("## {title}\n\n{prose}\n\n"));
        }
    }
    std::fs::write(&manuscript_path, &manuscript).expect("write manuscript");
    println!("\nManuscript written to {}", manuscript_path.display());

    let scorecard_path = out_dir.join("score_card.json");
    let total_polished_words: usize = chapter_prose
        .values()
        .flat_map(|v| v.iter())
        .map(|(_, s)| s.split_whitespace().count())
        .sum();

    // Run #16 — manuscript-level honest quality: score the whole
    // assembled prose as one unit, not just per-scene. This is the
    // number a human can trust when deciding "is this output usable".
    let full_manuscript_prose: String = chapter_prose
        .values()
        .flat_map(|v| v.iter())
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let manuscript_quality = booksforge_anti_ai_tells::score_paragraph(&full_manuscript_prose);
    let manuscript_tells = tells_per_1000_words(&full_manuscript_prose);
    let scenes_with_prose = score_cards
        .iter()
        .filter(|s| {
            s.get("polished_words")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();

    let summary = serde_json::json!({
        "scenes_attempted":      SCENES.len(),
        "scenes_with_prose":     scenes_with_prose,
        "total_polished_words":  total_polished_words,
        "wall_clock_seconds":    total_start.elapsed().as_secs_f32(),
        "wall_clock_minutes":    total_start.elapsed().as_secs_f32() / 60.0,
        "manuscript_quality":    {
            "overall":        manuscript_quality.overall,
            "sensory":        manuscript_quality.sensory,
            "figurative":     manuscript_quality.figurative,
            "rhythm":         manuscript_quality.rhythm,
            "mattr":          manuscript_quality.mattr,
            "no_structural":  manuscript_quality.no_structural,
            "low_token_tells": manuscript_quality.low_token_tells,
            "tells_verdict":  manuscript_tells.verdict,
            "tells_per_1000": manuscript_tells.weighted_density_per_1000,
        },
        "per_scene":             score_cards,
    });
    std::fs::write(
        &scorecard_path,
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .expect("write scorecard");

    println!("\n=== Final summary ===");
    println!(
        "  scenes attempted        : {} (with prose: {})",
        SCENES.len(),
        scenes_with_prose
    );
    println!("  total polished words    : {total_polished_words}");
    println!(
        "  wall-clock              : {:.1} min",
        total_start.elapsed().as_secs_f32() / 60.0
    );
    println!(
        "  manuscript quality      : {:.2} / 10.0  (tells={})",
        manuscript_quality.overall, manuscript_tells.verdict,
    );
    println!("  manuscript path         : {}", manuscript_path.display());
    println!("  scorecard path          : {}", scorecard_path.display());
}

/// Walk a ProseMirror `pm_doc` JSON into plain prose with paragraph breaks.
fn pm_doc_to_plain(doc: &serde_json::Value) -> String {
    fn walk(v: &serde_json::Value, out: &mut String) {
        if let Some(content) = v.get("content").and_then(|c| c.as_array()) {
            for child in content {
                let t = child.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if t == "text" {
                    if let Some(text) = child.get("text").and_then(|v| v.as_str()) {
                        out.push_str(text);
                    }
                } else {
                    walk(child, out);
                    if matches!(t, "paragraph" | "heading") {
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
    let mut out = String::new();
    if let Some(pm) = doc.get("pm_doc") {
        walk(pm, &mut out);
    } else {
        walk(doc, &mut out);
    }
    out.trim().replace("\n\n\n", "\n\n")
}
