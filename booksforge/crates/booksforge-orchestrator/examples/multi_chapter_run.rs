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

const MODEL_LIGHT: &str = "qwen3.5:9b";
const MODEL_HEAVY: &str = "qwen3.6:latest";

const IDEA_TEXT: &str = "\
A spare literary novel set in 1990s rural Pennsylvania. After her clockmaker \
husband Arthur dies suddenly of a heart attack, Elara — his widow of forty \
years — discovers a locked drawer in his workshop. Inside: twenty-three \
sealed letters dated to the three weeks before his death, addressed to a \
woman named Maeve Kowalski she has never heard of. Across two chapters, \
Elara finds the letters (Chapter 1) and drives to Maeve's house to confront \
her (Chapter 2). The novel is about inheritance, accumulated time, the way \
a long marriage can quietly hold a thing the other person doesn't know. \
Comp authors: Marilynne Robinson, Cormac McCarthy. No chosen-one tropes, no \
'tapestry of memory' AI prose.";

const CHAPTER_POV: &str = "third-limited";
const TARGET_WORDS_PER_SCENE: u32 = 800;

/// One scene's spec.
struct SceneSpec {
    chapter: u32,
    scene: u32,
    title: &'static str,
    goal: &'static str,
    conflict: &'static str,
    reveal: &'static str,
}

const SCENES: &[SceneSpec] = &[
    SceneSpec {
        chapter:  1,
        scene:    1,
        title:    "The Locked Drawer",
        goal:     "Elara opens the drawer she has avoided for three weeks and lifts out the leather folio inside.",
        conflict: "Her hand resists the act of intrusion even as her eyes have already begun. The workshop holds her grief like a held breath.",
        reveal:   "Inside the folio: a stack of twenty-three sealed envelopes, each addressed in Arthur's careful hand. The top one is dated three weeks before he died.",
    },
    SceneSpec {
        chapter:  1,
        scene:    2,
        title:    "Maeve Kowalski",
        goal:     "Elara breaks the wax seal of the top letter and reads it.",
        conflict: "She does not recognise the name on the envelope. The opening line of the letter is more intimate than anything Arthur ever said to her.",
        reveal:   "The letter is addressed to Maeve Kowalski. The first sentence is: 'I have spent forty years building a machine that keeps time, but I cannot fix the way I am breaking.'",
    },
    SceneSpec {
        chapter:  2,
        scene:    1,
        title:    "The Drive",
        goal:     "Elara drives the back roads to the address she found in Arthur's ledger — Maeve Kowalski's farmhouse, two counties over.",
        conflict: "She tells herself she only wants to look at the house. She knows this is not true.",
        reveal:   "The farmhouse is smaller than she imagined, and a woman in her sixties is already standing on the porch when Elara pulls into the gravel drive — as if she has been waiting.",
    },
    SceneSpec {
        chapter:  2,
        scene:    2,
        title:    "The Threshold",
        goal:     "Elara walks up to Maeve's porch and speaks her name.",
        conflict: "She prepared a thousand things to say on the drive. Standing in front of the woman, she finds none of them true.",
        reveal:   "Maeve does not invite her in. She says only: 'He told me you would come. He said it might take you a year.' Elara realises Maeve has been waiting since the funeral.",
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
    for needed in [MODEL_LIGHT, MODEL_HEAVY] {
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
                synopsis: None,
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
            MODEL_HEAVY.to_owned(),
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
            MODEL_HEAVY.to_owned(),
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
                MODEL_HEAVY.to_owned(),
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
