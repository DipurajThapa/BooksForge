#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

//! E2E tests for the Phase C quality-gate agents:
//! concept-scorer (Stage 1), audience-mapper (Stage 2),
//! character-critic (Stage 3), structure-critic (Stage 4).
//!
//! Same harness pattern as `run_methods.rs` — a `MockOllamaClient`
//! returns a canned response, the orchestrator runs the full pipeline
//! (template render → mock chat → parse-and-validate → AgentRunResult),
//! and assertions cover:
//!
//!   - Happy path: the parse succeeds, the gate logic returns the
//!     expected verdict, and the output value matches the canned
//!     response in shape.
//!   - Edge cases: out-of-range axes are clamped; minimal JSON
//!     parses via `#[serde(default)]` defaults; malformed JSON
//!     surfaces as a non-Completed status rather than a panic.
//!   - Privacy: the mock asserts each agent call actually reached
//!     the chat endpoint (no silent no-op dispatch).

use std::sync::Arc;

use booksforge_domain::{
    AgentTaskStatus, BookMode, CharacterBibleProposal, CharacterCard, ChapterPlan, Node, NodeKind,
    NodeStatus, OutlineProposal, PacingExpectation, PartPlan, ProjectBrief, ScenePlan,
};
use booksforge_fs::{BundleFilesystem, BundlePath, OsFilesystem};
use booksforge_ollama::client::OllamaClient;
use booksforge_ollama::types::CancelToken;
use booksforge_orchestrator::{Orchestrator, OrchestratorConfig};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use booksforge_test_fixtures::mock_ollama::MockOllamaClient;
use chrono::Utc;
use ulid::Ulid;

// ── Harness ──────────────────────────────────────────────────────────────────

struct Harness {
    orchestrator: Orchestrator,
    project_id: Ulid,
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

    // Insert a placeholder scene so any storage-touching code path has
    // something to find. Phase C agents don't read nodes, but the
    // snapshot service initialisation expects an established tree.
    let now = Utc::now();
    storage
        .insert_node(&Node {
            id: Ulid::new(),
            parent_id: None,
            kind: NodeKind::Scene,
            title: "Scene 1".into(),
            position: Node::DEFAULT_POSITION.into(),
            status: NodeStatus::Drafting,
            pov: None,
            beat: None,
            target_words: None,
            synopsis: None,
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
        project_id: Ulid::new(),
        _dir: dir,
    }
}

fn sample_brief() -> ProjectBrief {
    ProjectBrief {
        title_suggestions: vec!["The Wrong-Side Light".into()],
        mode: BookMode::Fiction,
        genre: "literary fiction".into(),
        audience: "adult literary readers".into(),
        tone: "spare, lyrical".into(),
        target_word_count: 75_000,
        premise: "A widow finds letters in her late husband's drawer that point to a parallel life.".into(),
        key_promises: vec![
            "a deepening mystery about who the husband really was".into(),
            "an emotional reckoning with grief".into(),
            "a quiet, decisive ending".into(),
        ],
        questions_for_user: vec![],
        comp_titles_or_authors: vec!["Marilynne Robinson".into(), "Kent Haruf".into()],
        theme_keywords: vec!["inheritance".into(), "secrets".into(), "grief".into()],
        forbidden_tropes: vec!["chosen-one".into(), "fated mates".into()],
        era_setting: Some("1990s rural Pennsylvania".into()),
        cultural_context: Some("working-class, post-industrial".into()),
        creative_seed: Some("told through alternating chapters of present grief + past letters".into()),
    }
}

fn sample_bible() -> CharacterBibleProposal {
    CharacterBibleProposal {
        characters: vec![
            CharacterCard {
                name: "Ada".into(),
                role: "protagonist".into(),
                external_objective: "Find Maeve Kowalski and ask the question.".into(),
                internal_need: "To stop arranging the silence between them.".into(),
                fear_or_wound: "Being known second.".into(),
                secret_or_contradiction: "Read the letters once and put them back.".into(),
                voice_traits: vec![
                    "sentences truncated when cornered".into(),
                    "uses tool-shop vocabulary by reflex".into(),
                ],
                relationships: vec![],
                chapter_arc: vec!["Ch1: opens the drawer, finds the letters".into()],
                emotional_turning_points: vec![],
            },
            CharacterCard {
                name: "Maeve".into(),
                role: "foil".into(),
                external_objective: "Be left alone with the dog.".into(),
                internal_need: "To be remembered as the friend, not the secret.".into(),
                fear_or_wound: "That Ada will arrive with a list of demands.".into(),
                secret_or_contradiction: "Kept her own copies of every letter.".into(),
                voice_traits: vec!["doesn't finish sentences when angry".into()],
                relationships: vec![],
                chapter_arc: vec![],
                emotional_turning_points: vec![],
            },
        ],
        voice_target: None,
    }
}

fn sample_outline() -> OutlineProposal {
    OutlineProposal {
        parts: vec![PartPlan {
            title: "Part I".into(),
            purpose: "The discovery and the first refusal.".into(),
            chapters: vec![ChapterPlan {
                title: "The Drawer".into(),
                purpose: "Ada finds the letters; the world tilts.".into(),
                scenes: vec![ScenePlan {
                    synopsis: "Ada opens the drawer.".into(),
                    pov: Some("Ada".into()),
                    beat: Some("inciting incident".into()),
                    target_word_count: Some(1_200),
                }],
            }],
        }],
        rationale: "Three-act structure with a quiet inciting incident.".into(),
        notes_to_user: vec![],
    }
}

// ────────────────────────────────────────────────────────────────────────────
// concept-scorer (Stage 1)
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn concept_scorer_happy_path_passes_gate() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "clarity":             { "score": 9.0, "reason": "premise reads in one breath" },
            "originality":         { "score": 8.5, "reason": "fresh angle vs. comps" },
            "emotional_pull":      { "score": 9.0, "reason": "named wound + named question" },
            "market_fit":          { "score": 8.5, "reason": "literary, 75k, comp-aligned" },
            "execution_potential": { "score": 9.0, "reason": "voice-led, achievable scope" },
            "overall_summary":     "A grounded concept with a clear emotional spine. Move to outline.",
            "edits": []
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let brief = sample_brief();
    let r = h
        .orchestrator
        .run_concept_scorer(h.project_id, &brief, "qwen3.5:9b".into(), CancelToken::new())
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(mock.chat_count(), 1, "agent called the chat endpoint");
    assert_eq!(r.status, AgentTaskStatus::Completed);
    let proposal = r.output.expect("output present on completed run");
    assert!(
        proposal.passes_gate(),
        "happy-path concept must pass gate (composite={})",
        proposal.composite()
    );
    assert!(proposal.composite() >= 8.5);
}

#[tokio::test]
async fn concept_scorer_low_axis_fails_gate() {
    // Composite ≥ 8.5 but one axis below the 7.0 floor — gate must fail.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "clarity":             { "score": 9.5 },
            "originality":         { "score": 9.5 },
            "emotional_pull":      { "score": 9.5 },
            "market_fit":          { "score": 9.5 },
            "execution_potential": { "score": 5.0, "reason": "scope-creep risk" },
            "overall_summary": "Strong premise; execution path is the worry.",
            "edits": [
                {
                    "field": "premise",
                    "suggestion": "Narrow the timeframe to a single season.",
                    "replacement": "Over one autumn, a widow…"
                }
            ]
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_concept_scorer(
            h.project_id,
            &sample_brief(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(r.status, AgentTaskStatus::Completed);
    let p = r.output.expect("output");
    assert!(!p.passes_gate(), "low axis must fail the gate");
    assert_eq!(p.weakest_axis(), "execution_potential");
    assert_eq!(p.edits.len(), 1);
}

#[tokio::test]
async fn concept_scorer_clamps_out_of_range_axes() {
    // A misbehaving model returns 15.0 and -2.0. The parse closure
    // must clamp to [0, 10] before any consumer sees the value.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "clarity":             { "score": 15.0 },
            "originality":         { "score": 9.0 },
            "emotional_pull":      { "score": 9.0 },
            "market_fit":          { "score": -2.0 },
            "execution_potential": { "score": 8.0 }
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_concept_scorer(
            h.project_id,
            &sample_brief(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(p.clarity.score <= 10.0, "high axis clamped");
    assert!(p.market_fit.score >= 0.0, "low axis clamped");
}

#[tokio::test]
async fn concept_scorer_invalid_json_does_not_complete() {
    // The mock returns garbage. The runner must NOT panic; the
    // status should be something other than Completed (Invalid or
    // similar) and the chat endpoint should still have been called.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response("this is not JSON".into());

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_concept_scorer(
            h.project_id,
            &sample_brief(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator must not bubble OrchestratorError for parse failure");

    assert!(mock.chat_count() >= 1, "agent attempted the chat call");
    assert_ne!(
        r.status,
        AgentTaskStatus::Completed,
        "invalid JSON must NOT report Completed"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// audience-mapper (Stage 2)
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn audience_mapper_happy_path_is_complete() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "genre_expectations":  ["intimate POV", "slow-burn emotional reveal"],
            "genre_anti_patterns": ["info-dump prologues", "ironic-detachment narrator"],
            "emotional_promises":  ["earned ache", "tentative reconciliation"],
            "recommended_themes":  ["inheritance", "the unread letter"],
            "recommended_tropes":  ["epistolary fragments", "found family of one"],
            "tropes_to_avoid":     ["dead-spouse-as-only-motivator", "chosen-one"],
            "pacing_expectation":  "slow_build",
            "overall_note": "Literary readers want texture over plot velocity here."
        })
        .to_string(),
    );

    let h = setup(mock.clone()).await;
    let r = h
        .orchestrator
        .run_audience_mapper(
            h.project_id,
            &sample_brief(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(mock.chat_count(), 1);
    assert_eq!(r.status, AgentTaskStatus::Completed);
    let map = r.output.expect("output");
    assert!(
        map.is_complete(),
        "all five required lists are populated → complete"
    );
    assert!(map.total_entries() >= 12);
    assert_eq!(map.pacing_expectation, PacingExpectation::SlowBuild);
}

#[tokio::test]
async fn audience_mapper_partial_response_is_parsed_but_incomplete() {
    // Model returns only two of the five required lists. The schema
    // is tolerant (#[serde(default)]) so it parses, but the
    // `is_complete()` gate must report false so the UI knows to
    // prompt for regeneration.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "genre_expectations":  ["intimate POV"],
            "tropes_to_avoid":     ["chosen-one"],
            "pacing_expectation":  "page_turner"
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_audience_mapper(
            h.project_id,
            &sample_brief(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(r.status, AgentTaskStatus::Completed);
    let map = r.output.expect("output");
    assert!(!map.is_complete(), "missing required lists → incomplete");
    assert_eq!(map.pacing_expectation, PacingExpectation::PageTurner);
    // Defaults: missing lists are empty rather than None.
    assert!(map.emotional_promises.is_empty());
    assert!(map.recommended_themes.is_empty());
}

#[tokio::test]
async fn audience_mapper_unknown_pacing_enum_falls_back_to_default() {
    // Schema tolerance contract (see `deserialize_pacing_tolerant` in
    // `audience_map.rs`): an unknown pacing string must NOT kill the
    // parse. The agent must report Completed, the other lists must
    // survive, and the pacing must fall back to SlowBuild.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "genre_expectations":  ["a"],
            "emotional_promises":  ["b"],
            "recommended_themes":  ["c"],
            "tropes_to_avoid":     ["d"],
            "pacing_expectation":  "warp_speed"
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_audience_mapper(
            h.project_id,
            &sample_brief(),
            "qwen3.5:9b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(
        r.status,
        AgentTaskStatus::Completed,
        "tolerant parse must produce Completed"
    );
    let map = r.output.expect("output present");
    assert_eq!(
        map.pacing_expectation,
        PacingExpectation::SlowBuild,
        "unknown pacing → SlowBuild default"
    );
    assert!(map.is_complete(), "other lists survive the recovery");
}

// ────────────────────────────────────────────────────────────────────────────
// character-critic (Stage 3)
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn character_critic_happy_path_per_card_pass() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "scores": [
                {
                    "character": "Ada",
                    "depth":                { "score": 9.0, "reason": "named wound + named secret" },
                    "consistency":          { "score": 9.0, "reason": "objective and need cohere" },
                    "uniqueness":           { "score": 8.5, "reason": "voice traits distinctive" },
                    "narrative_usefulness": { "score": 9.0, "reason": "drives plot" },
                    "emotional_impact":     { "score": 9.0, "reason": "reader will care" },
                    "overall_note": "Strong protagonist with a clear, earnable arc."
                },
                {
                    "character": "Maeve",
                    "depth":                { "score": 8.5 },
                    "consistency":          { "score": 9.0 },
                    "uniqueness":           { "score": 8.5 },
                    "narrative_usefulness": { "score": 8.5 },
                    "emotional_impact":     { "score": 9.0 }
                }
            ],
            "cross_card_findings": [],
            "edits": [],
            "overall_summary": "Two distinct, useful cards."
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_character_critic(
            h.project_id,
            &sample_brief(),
            &sample_bible(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(r.status, AgentTaskStatus::Completed);
    let p = r.output.expect("output");
    assert!(p.passes_gate(), "two strong cards + no findings → pass");
    assert_eq!(p.scores.len(), 2);
    assert!(p.weakest_cards().is_empty());
}

#[tokio::test]
async fn character_critic_error_finding_blocks_gate() {
    // Even with strong per-card scores, an "error"-severity
    // cross-card finding must block the gate.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "scores": [
                {
                    "character": "Ada",
                    "depth":                { "score": 9.5 },
                    "consistency":          { "score": 9.5 },
                    "uniqueness":           { "score": 9.5 },
                    "narrative_usefulness": { "score": 9.5 },
                    "emotional_impact":     { "score": 9.5 }
                }
            ],
            "cross_card_findings": [
                {
                    "kind": "duplicate_name",
                    "message": "Two cards both named 'Ada'.",
                    "severity": "error"
                }
            ],
            "edits": [
                {
                    "character": "Ada",
                    "field": "name",
                    "suggestion": "Rename the second Ada to her middle name.",
                    "replacement": "Adelaide"
                }
            ],
            "overall_summary": "Fix the duplicate name and you're shipping."
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_character_critic(
            h.project_id,
            &sample_brief(),
            &sample_bible(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(
        !p.passes_gate(),
        "error-severity finding must block even with perfect scores"
    );
    assert_eq!(p.edits.len(), 1);
}

#[tokio::test]
async fn character_critic_warning_finding_does_not_block() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "scores": [
                {
                    "character": "Ada",
                    "depth":                { "score": 9.0 },
                    "consistency":          { "score": 9.0 },
                    "uniqueness":           { "score": 9.0 },
                    "narrative_usefulness": { "score": 9.0 },
                    "emotional_impact":     { "score": 9.0 }
                }
            ],
            "cross_card_findings": [
                {
                    "kind": "coverage_sum_high",
                    "message": "Coverage 108% — slightly over 105% target.",
                    "severity": "warning"
                }
            ]
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_character_critic(
            h.project_id,
            &sample_brief(),
            &sample_bible(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(p.passes_gate(), "warning-severity finding must NOT block");
}

#[tokio::test]
async fn character_critic_empty_scores_does_not_pass_gate() {
    // Model returned `scores: []` — gate should fail rather than
    // panic on the empty composite.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "scores": [],
            "cross_card_findings": [],
            "edits": []
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_character_critic(
            h.project_id,
            &sample_brief(),
            &sample_bible(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(!p.passes_gate());
    assert_eq!(p.composite(), 0.0);
}

// ────────────────────────────────────────────────────────────────────────────
// structure-critic (Stage 4)
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn structure_critic_happy_path_passes_gate() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "promise_payoff":      { "score": 9.0, "reason": "every promise has a payoff beat" },
            "flow":                { "score": 9.0, "reason": "tension rises through Part II" },
            "reader_satisfaction": { "score": 8.5, "reason": "ending closes the open question" },
            "length_realism":      { "score": 9.0, "reason": "scene targets sum to 76k vs 75k" },
            "overall_summary": "Structurally sound. Move to drafting.",
            "findings": [],
            "edits": []
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_structure_critic(
            h.project_id,
            &sample_brief(),
            &sample_outline(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    assert_eq!(r.status, AgentTaskStatus::Completed);
    let p = r.output.expect("output");
    assert!(p.passes_gate(), "composite={}", p.composite());
}

#[tokio::test]
async fn structure_critic_missing_climax_blocks_gate() {
    // Strong axes, but an "error"-severity finding must block.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "promise_payoff":      { "score": 9.5 },
            "flow":                { "score": 9.5 },
            "reader_satisfaction": { "score": 9.5 },
            "length_realism":      { "score": 9.5 },
            "findings": [
                {
                    "kind": "missing_climax",
                    "message": "No identifiable climax beat in Part III.",
                    "severity": "error"
                }
            ],
            "edits": [
                {
                    "target": "scene",
                    "locator": "Part III / Chapter 14 / Scene 2",
                    "suggestion": "Insert a confrontation scene where Ada finally asks Maeve the question.",
                    "replacement": "Ada confronts Maeve in the kitchen…"
                }
            ]
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_structure_critic(
            h.project_id,
            &sample_brief(),
            &sample_outline(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(!p.passes_gate());
    assert_eq!(p.findings.len(), 1);
    assert_eq!(p.edits.len(), 1);
}

#[tokio::test]
async fn structure_critic_empty_response_gives_neutral_score() {
    // Model returns `{}` — every field falls back to its serde default.
    // Per `structure_score.rs` the axis defaults are 7.0 each so the
    // axis-floor passes but composite = 7.0 < 8.5, so the gate fails
    // cleanly without a parse error.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(serde_json::json!({}).to_string());

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_structure_critic(
            h.project_id,
            &sample_brief(),
            &sample_outline(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(!p.passes_gate());
    assert!((p.composite() - 7.0).abs() < 1e-3);
    assert!(p.findings.is_empty());
}

#[tokio::test]
async fn structure_critic_clamps_axes_to_bounds() {
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "promise_payoff":      { "score": 99.0 },
            "flow":                { "score": -5.0 },
            "reader_satisfaction": { "score": 8.0 },
            "length_realism":      { "score": 8.0 }
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_structure_critic(
            h.project_id,
            &sample_brief(),
            &sample_outline(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(p.promise_payoff.score <= 10.0);
    assert!(p.flow.score >= 0.0);
}

#[tokio::test]
async fn structure_critic_weakest_axis_is_reported() {
    // Spread the scores so the weakest is unambiguous.
    let mock = Arc::new(MockOllamaClient::default());
    mock.set_chat_response(
        serde_json::json!({
            "promise_payoff":      { "score": 9.0 },
            "flow":                { "score": 9.0 },
            "reader_satisfaction": { "score": 9.0 },
            "length_realism":      { "score": 4.5, "reason": "scene targets sum to 130k vs brief 75k" }
        })
        .to_string(),
    );

    let h = setup(mock).await;
    let r = h
        .orchestrator
        .run_structure_critic(
            h.project_id,
            &sample_brief(),
            &sample_outline(),
            "qwen3.5:27b".into(),
            CancelToken::new(),
        )
        .await
        .expect("orchestrator returns Ok");

    let p = r.output.expect("output");
    assert!(!p.passes_gate());
    assert_eq!(p.weakest_axis(), "length_realism");
}
