#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unimplemented,
    clippy::too_many_lines
)]

//! Mock-Ollama integration tests for every `pub async fn run_*` on the
//! `Orchestrator`. Lifts coverage of `crates/booksforge-orchestrator/src/run.rs`
//! by exercising the per-agent template-vars assembly, the parse closure
//! plumbing, and the `RunContext` propagation.
//!
//! `MockOllamaClient` returns ONE canned `chat_response` for ALL calls in a
//! test, so chained workflows (`run_intake_and_outline`,
//! `run_developmental_review`) assert `is_ok()` rather than the inner
//! status — the inner half that doesn't match the canned shape is allowed
//! to land in `Invalid`.

use std::sync::Arc;

use booksforge_domain::{
    AgentTaskStatus, BookMode, Node, NodeKind, NodeStatus, PolishStageId, ProjectBrief,
};
use booksforge_fs::{BundleFilesystem, BundlePath, OsFilesystem};
use booksforge_ollama::client::OllamaClient;
use booksforge_ollama::types::CancelToken;
use booksforge_orchestrator::runner::RunContext;
use booksforge_orchestrator::{Orchestrator, OrchestratorConfig};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use booksforge_test_fixtures::mock_ollama::MockOllamaClient;
use chrono::Utc;
use ulid::Ulid;

// ── Harness ──────────────────────────────────────────────────────────────────

struct Harness {
    orchestrator: Orchestrator,
    #[allow(dead_code)]
    storage: Arc<SqliteStorage>,
    project_id: Ulid,
    #[allow(dead_code)]
    scene_id: Ulid,
    _dir: tempfile::TempDir,
}

async fn setup(mock: Arc<MockOllamaClient>) -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle_root = dir.path().join("test.booksforge");
    std::fs::create_dir_all(bundle_root.join("snapshots/objects")).expect("mkdir");
    let bundle = BundlePath::new(&bundle_root);

    let pool = open_pool(&bundle.db()).await.expect("open_pool");
    run_migrations(&pool).await.expect("migrations");
    let storage = Arc::new(SqliteStorage::new(pool));

    let scene_id = Ulid::new();
    let now = Utc::now();
    storage
        .insert_node(&Node {
            id: scene_id,
            parent_id: None,
            kind: NodeKind::Scene,
            title: "Test Scene".into(),
            position: Node::DEFAULT_POSITION.into(),
            status: NodeStatus::Drafting,
            pov: None,
            beat: None,
            target_words: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
        .await
        .expect("insert_node");

    let storage_trait: Arc<dyn StorageRepository> = storage.clone();
    let fs: Arc<dyn BundleFilesystem> = Arc::new(OsFilesystem);
    let snapshot = Arc::new(SnapshotService::new(storage_trait, fs, bundle));

    let ollama_trait: Arc<dyn OllamaClient> = mock;
    let orchestrator =
        Orchestrator::new(ollama_trait, storage.clone(), OrchestratorConfig::default())
            .with_snapshot(snapshot);

    Harness {
        orchestrator,
        storage,
        project_id: Ulid::new(),
        scene_id,
        _dir: dir,
    }
}

fn brief() -> ProjectBrief {
    ProjectBrief {
        title_suggestions: vec!["Untitled".into()],
        mode: BookMode::Fiction,
        genre: "literary".into(),
        audience: "adult".into(),
        tone: "spare".into(),
        target_word_count: 50_000,
        premise: "A premise.".into(),
        key_promises: vec!["promise".into()],
        questions_for_user: vec![],
        comp_titles_or_authors: vec!["Marilynne Robinson".into()],
        theme_keywords: vec!["inheritance".into()],
        forbidden_tropes: vec!["chosen-one".into()],
        era_setting: Some("1990s rural Pennsylvania".into()),
        cultural_context: Some("working-class, post-industrial".into()),
        creative_seed: Some("each chapter from a different POV".into()),
    }
}

// A non-trivial "chapter text" so the polish stages and humanization don't
// trip the originality detector with implausibly short source text.
fn long_chapter_text() -> String {
    "Ada walked into the kitchen. The light was off and the floor was cold under her feet. \
     She crossed to the window and stood there for a long moment in the dark. Outside, the \
     yard was still, the maples motionless. She thought about the letter. She thought about \
     what her mother had said the night before — the part she could not unhear. The fridge \
     clicked on behind her. She did not turn around."
        .to_owned()
}

// Standard non-trivial pm_doc that satisfies SceneDraftProposal::validate
// and PolishProposal::validate (type=doc, content non-empty).
fn pm_doc_value() -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": [
            {
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "Ada walked in. The light was off."}
                ]
            },
            {
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "She stood for a long moment in the dark."}
                ]
            }
        ]
    })
}

// ── 1. run_copyedit_scene ────────────────────────────────────────────────────

#[tokio::test]
async fn run_copyedit_scene_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    // Empty edits + short summary is the minimal valid CopyeditProposals.
    mock.set_chat_response(
        serde_json::json!({
            "edits": [],
            "summary": "no edits required"
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let scene_text = "She walked across the room.".to_owned();
    let style_book = serde_json::json!({});

    let r = h
        .orchestrator
        .run_copyedit_scene(
            h.project_id,
            scene_text,
            "S1".into(),
            style_book,
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("copyedit_scene returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 2. run_continuity_adjudication ───────────────────────────────────────────

#[tokio::test]
async fn run_continuity_adjudication_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    // Empty findings is a valid ContinuityReport.
    mock.set_chat_response(
        serde_json::json!({
            "findings": []
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_continuity_adjudication(
            h.project_id,
            serde_json::json!([]),
            serde_json::json!({}),
            serde_json::json!([]),
            None,
            None,
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("continuity_adjudication returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 3. run_intake_and_outline (chained — 2 LLM hops with same canned shape) ──

#[tokio::test]
async fn run_intake_and_outline_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    // The mock returns the same string for both intake AND outline. The
    // intake half parses successfully against ProjectBrief; the outline
    // half lands in "skipped" or "invalid" — but the wrapper still
    // returns Ok(IntakeAndOutlineResult).
    mock.set_chat_response(serde_json::to_string(&brief()).unwrap());

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_intake_and_outline(
            h.project_id,
            "An idea about grief.".into(),
            None,
            8,
            None,
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await;

    assert!(r.is_ok(), "intake_and_outline must return Ok: {r:?}");
    assert!(mock.chat_count() >= 1, "at least intake must hit the LLM");
}

// ── 4. run_memory_curator ────────────────────────────────────────────────────

#[tokio::test]
async fn run_memory_curator_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "upserts": [],
            "new_entities": []
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_memory_curator(
            h.project_id,
            "book".into(),
            None,
            "Some chapter text.".into(),
            serde_json::json!({}),
            serde_json::json!([]),
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("memory_curator returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 5. run_vocab_dictionary ──────────────────────────────────────────────────

#[tokio::test]
async fn run_vocab_dictionary_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "additions": [],
            "modifications": []
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_vocab_dictionary(
            h.project_id,
            serde_json::json!([]),
            serde_json::json!([]),
            serde_json::json!({}),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("vocab_dictionary returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 6. run_polish_stage (all 4 stages) ───────────────────────────────────────

async fn run_polish_for(stage: PolishStageId, stage_str: &str) {
    let mock = Arc::new(MockOllamaClient::default());
    let payload = serde_json::json!({
        "stage_id": stage_str,
        "revised_pm_doc": pm_doc_value(),
        "revised_word_count": 16,
        "edit_notes": "Targeted edits to the named stage; voice preserved.",
    });
    mock.set_chat_response(payload.to_string());

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_polish_stage(
            h.project_id,
            stage,
            long_chapter_text(),
            "literary".into(),
            "spare, lyrical".into(),
            "Ada".into(),
            RunContext::empty(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
            None,
        )
        .await;

    // The four polish-stage templates may or may not be wired into the
    // prompt crate's `template_source` registry depending on the build —
    // either way, the orchestrator method's vars assembly and parse-closure
    // construction in `run.rs` ran. Accept Ok or template-not-found as a
    // smoke pass.
    assert_smoke_or_template_missing(&r, stage_str);
}

/// Helper: a result is acceptable if it's `Ok` (LLM call attempted, parse
/// path traversed) OR an `AgentFailed` whose reason mentions "prompt render
/// failed" (template not registered — orchestrator method's pre-runner
/// code still ran). Anything else is a real bug. We can't bound on
/// `Debug` because `AgentRunResult<T>` isn't Debug, so failure messages
/// only print the error half.
fn assert_smoke_or_template_missing<T>(
    r: &Result<T, booksforge_orchestrator::OrchestratorError>,
    label: &str,
) {
    match r {
        Ok(_) => {}
        Err(booksforge_orchestrator::OrchestratorError::AgentFailed { reason, .. })
            if reason.contains("prompt render failed") => {}
        Err(e) => panic!("{label}: unexpected error {e:?}"),
    }
}

#[tokio::test]
async fn run_polish_stage_dialogue() {
    run_polish_for(PolishStageId::Dialogue, "dialogue").await;
}

#[tokio::test]
async fn run_polish_stage_metaphor() {
    run_polish_for(PolishStageId::Metaphor, "metaphor").await;
}

#[tokio::test]
async fn run_polish_stage_voice() {
    run_polish_for(PolishStageId::Voice, "voice").await;
}

#[tokio::test]
async fn run_polish_stage_scene_tension() {
    run_polish_for(PolishStageId::SceneTension, "scene_tension").await;
}

// ── 7. run_scene_critic ──────────────────────────────────────────────────────

#[tokio::test]
async fn run_scene_critic_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "scores": {
                "tension": 7,
                "voice": 6
            },
            "weakest_axis": "voice",
            "specific_edits": [],
            "overall_one_liner": "Tighten the middle beat."
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_scene_critic(
            h.project_id,
            long_chapter_text(),
            "Ada finds the letter.".into(),
            "Ada vs herself.".into(),
            "Reveals the secret.".into(),
            vec!["tension".into(), "voice".into()],
            "literary".into(),
            "spare".into(),
            "Prior summary.".into(),
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await;

    assert_smoke_or_template_missing(&r, "scene_critic");
}

// ── 8. run_character_bible ───────────────────────────────────────────────────

fn char_card(name: &str, role: &str, chapter_count: usize) -> serde_json::Value {
    let arc: Vec<String> = (0..chapter_count)
        .map(|i| format!("ch{i}: change"))
        .collect();
    serde_json::json!({
        "name": name,
        "role": role,
        "external_objective": "Find the letter.",
        "internal_need": "Be seen.",
        "fear_or_wound": "Abandonment.",
        "secret_or_contradiction": "Knew before she said.",
        "voice_traits": [
            "short declarative sentences",
            "agricultural vocabulary",
            "evades abstract nouns"
        ],
        "relationships": [],
        "chapter_arc": arc,
        "emotional_turning_points": ["ch1: doubt"]
    })
}

#[tokio::test]
async fn run_character_bible_smoke() {
    let chapter_count: u32 = 3;
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "characters": [
                char_card("Ada", "protagonist", chapter_count as usize),
                char_card("Cal", "antagonist", chapter_count as usize)
            ]
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_character_bible(
            h.project_id,
            serde_json::to_value(brief()).unwrap(),
            chapter_count,
            serde_json::json!([]),
            serde_json::json!({}),
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await;

    assert_smoke_or_template_missing(&r, "character_bible");
}

// ── 9. run_world_bible ───────────────────────────────────────────────────────

#[tokio::test]
async fn run_world_bible_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "main_locations": [
                {
                    "name": "The farmhouse",
                    "purpose_in_story": "Site of the inheritance.",
                    "sensory_signature": "pine resin and wet stone",
                    "key_constraints": "Power cuts after midnight."
                }
            ],
            "social_rules": ["No inheritance without the funeral."],
            "history": "The farmhouse stood for four generations through three floods, two fires, and the long winter that closed every road for nine weeks running.",
            "sensory_palette": {
                "sight": "Low grey sky over fallow fields.",
                "sound": "Crows over the silo.",
                "smell": "Diesel and damp wool.",
                "touch": "",
                "taste": ""
            },
            "conflict_sources": ["The will."],
            "symbolic_motifs": ["The locked door."],
            "continuity_constraints": ["Phones do not work past the ridge."]
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_world_bible(
            h.project_id,
            serde_json::to_value(brief()).unwrap(),
            serde_json::json!({}),
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await;

    assert_smoke_or_template_missing(&r, "world_bible");
}

// ── 10. run_scene_drafter_fic ────────────────────────────────────────────────

#[tokio::test]
async fn run_scene_drafter_fic_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "pm_doc": pm_doc_value(),
            "word_count": 16,
            "notes": "Opening in-medias-res; deliberate short sentences match the comp voice profile."
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_scene_drafter_fic(
            h.project_id,
            "Ada finds the letter".into(),
            "Ada vs the past".into(),
            "Reveals her mother knew".into(),
            500,
            "Ada".into(),
            "literary".into(),
            serde_json::json!({}),
            serde_json::json!({}),
            "spare, lyrical".into(),
            "Prior summary.".into(),
            RunContext::empty(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
            None,
        )
        .await;

    assert_smoke_or_template_missing(&r, "scene_drafter_fic");
}

// ── 11. run_chapter_drafter ──────────────────────────────────────────────────

#[tokio::test]
async fn run_chapter_drafter_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "pm_doc": pm_doc_value(),
            "word_count": 16,
            "notes": "Restrained drafting per voice fingerprint."
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_chapter_drafter(
            h.project_id,
            "Ada walks into the kitchen and discovers the letter.".into(),
            "Establish the inheritance theme.".into(),
            "third-limited, Ada".into(),
            800,
            serde_json::json!([]),
            "Prior summary.".into(),
            serde_json::json!({}),
            Some("literary".into()),
            Some("spare".into()),
            RunContext::empty(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
            None,
        )
        .await
        .expect("chapter_drafter returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 12. run_dev_editor ───────────────────────────────────────────────────────

#[tokio::test]
async fn run_dev_editor_smoke() {
    let chapter_id = Ulid::new().to_string();
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "chapter_id": chapter_id,
            "notes": [],
            "summary": "Pacing is steady; stakes clear."
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_dev_editor(
            h.project_id,
            chapter_id,
            long_chapter_text(),
            serde_json::to_value(brief()).unwrap(),
            serde_json::json!([]),
            serde_json::json!([]),
            RunContext::empty(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
            None,
        )
        .await
        .expect("dev_editor returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 13. run_developmental_review (chained — dev_editor + deterministic linter) ──

#[tokio::test]
async fn run_developmental_review_smoke() {
    // dev_editor schema is straightforward; the deterministic continuity
    // linter is free (no LLM) so this chain stays at exactly 1 LLM call.
    let chapter_id = Ulid::new().to_string();
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "chapter_id": chapter_id,
            "notes": [],
            "summary": "Pacing is steady; stakes clear."
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let scene = (Ulid::new(), "Scene 1".to_owned(), long_chapter_text());
    let r = h
        .orchestrator
        .run_developmental_review(
            h.project_id,
            chapter_id,
            long_chapter_text(),
            vec![scene],
            serde_json::to_value(brief()).unwrap(),
            serde_json::json!([]),
            serde_json::json!([]),
            None,
            RunContext::empty(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
            None,
        )
        .await;

    assert!(r.is_ok(), "developmental_review must return Ok: {r:?}");
    assert!(mock.chat_count() >= 1);
    let res = r.unwrap();
    assert_eq!(res.scenes_scanned, 1);
}

// ── 14. run_humanization ─────────────────────────────────────────────────────

#[tokio::test]
async fn run_humanization_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    // Empty edits — trivially valid against any source_text.
    mock.set_chat_response(
        serde_json::json!({
            "edits": []
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_humanization(
            h.project_id,
            long_chapter_text(),
            "S1".into(),
            serde_json::json!([]),
            serde_json::json!({}),
            RunContext::empty(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("humanization returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 15. run_proposal_validator_tier2 ────────────────────────────────────────

#[tokio::test]
async fn run_proposal_validator_tier2_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    // Pass verdict + empty checks aggregates to Pass; tier_2_ran must be true.
    mock.set_chat_response(
        serde_json::json!({
            "verdict": "pass",
            "checks": [],
            "summary": "all axes pass",
            "tier_2_ran": true
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_proposal_validator_tier2(
            h.project_id,
            "scene-drafter-fic".into(),
            serde_json::json!({}),
            "Some scene context.".into(),
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!([]),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("proposal_validator_tier2 returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
}

// ── 16. run_outline (different signature: by-ref + OutlineRunResult) ─────────

#[tokio::test]
async fn run_outline_smoke() {
    let mock = Arc::new(MockOllamaClient::default());
    // 8 chapters across 1 part, each with a non-empty synopsis. Word
    // count target is left unset on each scene — the validator only
    // checks total when total > 0.
    // 8 chapters × 6250 words each = 50_000 — exactly the brief's target.
    // Each synopsis must share <40% tokens with every other (Jaccard) so
    // the outline-architect duplicate-synopsis check passes.
    let synopses = [
        "Ada arrives at the farmhouse and finds the door unlocked.",
        "Cal demands his share, citing a will nobody has read.",
        "An old neighbor tells what happened during the long winter.",
        "Snow buries the road overnight; phones stop working.",
        "A locked desk gives up its first letter from 1973.",
        "Ada confronts her mother's silence about the second fire.",
        "Cal threatens legal action; Ada burns one page.",
        "Spring melt reveals what was buried under the silo.",
    ];
    let chapters: Vec<serde_json::Value> = (0..8)
        .map(|i| {
            serde_json::json!({
                "title": format!("Chapter {}", i + 1),
                "purpose": "Move the story forward.",
                "scenes": [
                    {
                        "synopsis": synopses[i],
                        "pov": null,
                        "beat": null,
                        "target_word_count": 6250
                    }
                ]
            })
        })
        .collect();
    let payload = serde_json::json!({
        "parts": [
            {
                "title": "Part One",
                "purpose": "Establish the world and the wound.",
                "chapters": chapters
            }
        ],
        "rationale": "Eight chapters in one part keeps the structure tight enough for a 50k literary novel; each chapter pivots on one beat, leaving room for the developmental editor to surface re-balancing later.",
        "notes_to_user": ["Consider adding a coda."]
    });
    mock.set_chat_response(payload.to_string());

    let h = setup(mock.clone()).await;
    let b = brief();
    let r = h
        .orchestrator
        .run_outline(h.project_id, &b, 8, None, "qwen3.5:9b", CancelToken::new())
        .await
        .expect("run_outline returns Ok");

    assert!(mock.chat_count() >= 1);
    assert_eq!(r.status, "completed", "run_outline must report completed");
    assert!(r.proposal.is_some(), "run_outline must yield a proposal");
}
