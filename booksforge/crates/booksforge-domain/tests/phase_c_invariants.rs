#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

//! Domain-level invariant tests for the Phase C quality gates and the
//! apply-outline pre-cleanup. These exercise the pure-logic boundary
//! between the agents (which produce these types) and the orchestrator
//! (which acts on them). No I/O — every test runs on in-memory data.
//!
//! Scope:
//!   - Cross-gate consistency: the four critic types share the same
//!     (composite ≥ 8.5 AND every axis ≥ 7.0) pattern; verify the
//!     boundary behaves identically.
//!   - Schema tolerance: an agent that returns `{}` must produce a
//!     parseable value the UI can show, not a parse failure.
//!   - Apply-outline pre-cleanup: a realistic "regenerate outline
//!     over a partially-drafted book" scenario where some scenes
//!     have prose and others don't.

use booksforge_domain::{
    empty_subtree_ids, BoilerplateKind, BoilerplatePage, CharacterCriticProposal, CharacterScore,
    ConceptEdit, ConceptScoreAxis, ConceptScoreProposal, CrossCardFinding, Node, NodeKind,
    NodeStatus, PacingExpectation, ReaderExpectationMap, Severity, StructureCriticProposal,
    StructureEdit, StructureFinding,
};
use chrono::Utc;
use ulid::Ulid;

// ── Test helpers ─────────────────────────────────────────────────────────────

fn axis(score: f32) -> ConceptScoreAxis {
    ConceptScoreAxis {
        score,
        reason: String::new(),
    }
}

fn axis_with_reason(score: f32, reason: &str) -> ConceptScoreAxis {
    ConceptScoreAxis {
        score,
        reason: reason.into(),
    }
}

fn passing_concept() -> ConceptScoreProposal {
    ConceptScoreProposal {
        clarity: axis(9.0),
        originality: axis(8.5),
        emotional_pull: axis(9.0),
        market_fit: axis(8.5),
        execution_potential: axis(9.0),
        overall_summary: "passing".into(),
        edits: vec![],
    }
}

fn passing_structure() -> StructureCriticProposal {
    StructureCriticProposal {
        promise_payoff: axis(9.0),
        flow: axis(9.0),
        reader_satisfaction: axis(8.5),
        length_realism: axis(9.0),
        overall_summary: "passing".into(),
        findings: vec![],
        edits: vec![],
    }
}

fn passing_character_score(name: &str) -> CharacterScore {
    CharacterScore {
        character: name.into(),
        depth: axis(9.0),
        consistency: axis(9.0),
        uniqueness: axis(8.5),
        narrative_usefulness: axis(9.0),
        emotional_impact: axis(9.0),
        overall_note: "strong".into(),
    }
}

// ── Cross-gate consistency: composite + axis floor + error severity ─────────

#[test]
fn concept_passes_only_when_composite_and_axes_both_meet_threshold() {
    // Composite 8.5 (threshold) AND all axes ≥ 7.0 (floor) — pass.
    let mut p = passing_concept();
    // Push one axis to exactly 7.0 (the floor).
    p.market_fit = axis(7.0);
    assert!(p.passes_gate(), "axis at floor and composite ≥ threshold");

    // Same composite but one axis just below the floor — fail.
    p.market_fit = axis(6.99);
    // Boost another axis to keep composite passing.
    p.clarity = axis(10.0);
    assert!(p.composite() >= 8.5);
    assert!(
        !p.passes_gate(),
        "axis below floor must fail even when composite passes"
    );
}

#[test]
fn structure_passes_only_when_composite_and_axes_both_meet_threshold() {
    let mut p = passing_structure();
    assert!(p.passes_gate());

    // Push one axis to exactly 7.0 (floor).
    p.length_realism = axis(7.0);
    // Recompute composite: (9 + 9 + 8.5 + 7) / 4 = 8.375 < 8.5 → fail.
    assert!(!p.passes_gate(), "composite slips below 8.5");

    // Restore margin.
    p.flow = axis(10.0);
    // (9 + 10 + 8.5 + 7) / 4 = 8.625 → pass.
    assert!(p.passes_gate());
}

#[test]
fn character_critic_aggregates_across_cards_with_axis_floor() {
    // Two strong cards: bible passes.
    let p = CharacterCriticProposal {
        scores: vec![
            passing_character_score("Ada"),
            passing_character_score("Maeve"),
        ],
        cross_card_findings: vec![],
        edits: vec![],
        overall_summary: String::new(),
    };
    assert!(p.passes_gate());
    assert!(p.weakest_cards().is_empty());

    // One card drops to a 6.9 axis: that card fails per-card and the
    // bible fails overall, even though the second card is strong.
    let mut weak = passing_character_score("Maeve");
    weak.uniqueness = axis(6.9);
    let p2 = CharacterCriticProposal {
        scores: vec![passing_character_score("Ada"), weak],
        ..p
    };
    assert!(!p2.passes_gate());
    assert_eq!(p2.weakest_cards(), vec!["Maeve".to_owned()]);
}

#[test]
fn error_finding_blocks_every_critic_gate() {
    let structure_with_error = StructureCriticProposal {
        findings: vec![StructureFinding {
            kind: "missing_climax".into(),
            message: "no climax".into(),
            severity: Severity::Error,
        }],
        ..passing_structure()
    };
    assert!(!structure_with_error.passes_gate());

    let character_with_error = CharacterCriticProposal {
        scores: vec![passing_character_score("Ada")],
        cross_card_findings: vec![CrossCardFinding {
            kind: "duplicate_name".into(),
            message: "two Adas".into(),
            severity: Severity::Error,
        }],
        edits: vec![],
        overall_summary: String::new(),
    };
    assert!(!character_with_error.passes_gate());
}

#[test]
fn warning_finding_never_blocks_any_critic_gate() {
    let structure_with_warning = StructureCriticProposal {
        findings: vec![StructureFinding {
            kind: "sagging_middle".into(),
            message: "minor".into(),
            severity: Severity::Warning,
        }],
        ..passing_structure()
    };
    assert!(structure_with_warning.passes_gate());

    let character_with_warning = CharacterCriticProposal {
        scores: vec![passing_character_score("Ada")],
        cross_card_findings: vec![CrossCardFinding {
            kind: "coverage_sum_high".into(),
            message: "108%".into(),
            severity: Severity::Warning,
        }],
        edits: vec![],
        overall_summary: String::new(),
    };
    assert!(character_with_warning.passes_gate());
}

#[test]
fn unknown_severity_string_falls_back_to_warning_and_does_not_block() {
    // A model emits a typo like "errror". Tolerant deserialisation
    // lands it as Warning rather than failing the whole proposal,
    // and Warning never blocks the gate. (The closely-related
    // documented variant "info" is also non-blocking, so a typo
    // can't accidentally upgrade a note into a gate-blocker.)
    let json = serde_json::json!({
        "promise_payoff":      { "score": 9.0 },
        "flow":                { "score": 9.0 },
        "reader_satisfaction": { "score": 9.0 },
        "length_realism":      { "score": 9.0 },
        "findings": [
            { "kind": "thin_third_act", "message": "FYI", "severity": "errror" }
        ]
    });
    let p: StructureCriticProposal = serde_json::from_value(json).expect("parses");
    assert_eq!(p.findings[0].severity, Severity::Warning);
    assert!(p.passes_gate());
}

// ── Schema tolerance: minimal JSON parses + clamps cleanly ───────────────────

#[test]
fn concept_score_proposal_clamps_all_axes_in_one_call() {
    let mut p = ConceptScoreProposal {
        clarity: axis(15.0),
        originality: axis(-3.0),
        emotional_pull: axis(f32::NAN),
        market_fit: axis(8.0),
        execution_potential: axis(8.0),
        overall_summary: String::new(),
        edits: vec![],
    };
    p.clamp_all();
    assert!(p.clarity.score <= 10.0);
    assert!(p.originality.score >= 0.0);
    assert_eq!(p.emotional_pull.score, 0.0, "NaN clamps to 0.0");
    assert_eq!(p.market_fit.score, 8.0, "in-range value is unchanged");
}

#[test]
fn character_critic_clamp_propagates_to_every_card() {
    let mut p = CharacterCriticProposal {
        scores: vec![
            CharacterScore {
                character: "Ada".into(),
                depth: axis(15.0),
                consistency: axis(9.0),
                uniqueness: axis(9.0),
                narrative_usefulness: axis(9.0),
                emotional_impact: axis(9.0),
                overall_note: String::new(),
            },
            CharacterScore {
                character: "Maeve".into(),
                depth: axis(9.0),
                consistency: axis(-2.0),
                uniqueness: axis(9.0),
                narrative_usefulness: axis(9.0),
                emotional_impact: axis(9.0),
                overall_note: String::new(),
            },
        ],
        cross_card_findings: vec![],
        edits: vec![],
        overall_summary: String::new(),
    };
    p.clamp_all();
    assert!(p.scores[0].depth.score <= 10.0);
    assert!(p.scores[1].consistency.score >= 0.0);
}

#[test]
fn structure_critic_neutral_default_axes_dont_pass_gate() {
    // A `{}` JSON parses to defaults (7.0 each). Axis floor passes
    // (every axis = 7.0 = floor), but composite = 7.0 < 8.5 → fail.
    let p: StructureCriticProposal = serde_json::from_str("{}").expect("parses");
    assert!((p.composite() - 7.0).abs() < 1e-3);
    assert!(!p.passes_gate());
}

#[test]
fn audience_map_is_complete_requires_all_four_lists() {
    let mut m = ReaderExpectationMap::default();
    assert!(!m.is_complete(), "empty map → incomplete");

    m.genre_expectations = vec!["a".into()];
    m.emotional_promises = vec!["b".into()];
    m.recommended_themes = vec!["c".into()];
    assert!(
        !m.is_complete(),
        "missing tropes_to_avoid → still incomplete"
    );

    m.tropes_to_avoid = vec!["d".into()];
    assert!(m.is_complete(), "all four lists populated");
}

#[test]
fn audience_map_unknown_pacing_string_falls_back_to_default() {
    // Schema tolerance: an out-of-vocab pacing string must NOT kill
    // the parse. The map should land with SlowBuild (the documented
    // default) and the rest of its fields preserved. See
    // `deserialize_pacing_tolerant` in `audience_map.rs`.
    let bad = serde_json::json!({
        "genre_expectations": ["a"],
        "pacing_expectation": "warp_speed"
    });
    let m: ReaderExpectationMap =
        serde_json::from_value(bad).expect("tolerant parse must succeed");
    assert_eq!(m.pacing_expectation, PacingExpectation::SlowBuild);
    assert_eq!(m.genre_expectations, vec!["a".to_owned()]);
}

#[test]
fn audience_map_known_pacing_string_parses_each_variant() {
    for pacing in ["slow_build", "page_turner", "episodic", "lyrical"] {
        let v = serde_json::json!({ "pacing_expectation": pacing });
        let m: ReaderExpectationMap = serde_json::from_value(v).expect("parses");
        // Each pacing should round-trip back to one of the four enum variants.
        match m.pacing_expectation {
            PacingExpectation::SlowBuild
            | PacingExpectation::PageTurner
            | PacingExpectation::Episodic
            | PacingExpectation::Lyrical => {}
        }
    }
}

// ── Edit suggestion plumbing ─────────────────────────────────────────────────

#[test]
fn concept_edit_round_trips_with_optional_replacement_empty() {
    let edit = ConceptEdit {
        field: "premise".into(),
        suggestion: "split into two sentences".into(),
        replacement: String::new(),
    };
    let v = serde_json::to_value(&edit).unwrap();
    let back: ConceptEdit = serde_json::from_value(v).unwrap();
    assert_eq!(back.field, "premise");
    assert!(back.replacement.is_empty());
}

#[test]
fn structure_edit_round_trips_with_locator() {
    let edit = StructureEdit {
        target: "scene".into(),
        locator: "Part II / Chapter 7 / Scene 3".into(),
        suggestion: "rewrite synopsis".into(),
        replacement: "Maeve hesitates at the kitchen door.".into(),
    };
    let v = serde_json::to_value(&edit).unwrap();
    let back: StructureEdit = serde_json::from_value(v).unwrap();
    assert_eq!(back.locator, "Part II / Chapter 7 / Scene 3");
    assert_eq!(back.replacement, "Maeve hesitates at the kitchen door.");
}

#[test]
fn structure_edit_legacy_payload_without_locator_is_tolerated() {
    // Older models may emit just `target` / `suggestion`. Schema
    // tolerance must not fail the whole proposal on that.
    let v = serde_json::json!({
        "target": "rationale",
        "suggestion": "soften the opening claim"
    });
    let back: StructureEdit = serde_json::from_value(v).expect("legacy parses");
    assert!(back.locator.is_empty());
    assert!(back.replacement.is_empty());
}

// ── Apply-outline pre-cleanup: empty_subtree_ids E2E ─────────────────────────

#[test]
fn pre_cleanup_preserves_drafted_subtrees() {
    // Scenario: writer regenerates the outline after having drafted
    // a scene in Chapter 1. The pre-cleanup must NOT delete Chapter 1
    // or its parent Part I, but should delete the empty Chapter 2
    // and Part II that the template seeded but never got prose.
    let project_id = Ulid::new();
    let part_i = Ulid::new();
    let part_ii = Ulid::new();
    let chap_1 = Ulid::new();
    let chap_2 = Ulid::new();
    let scene_drafted = Ulid::new();
    let scene_empty = Ulid::new();
    let chap_3 = Ulid::new(); // empty chapter under part_ii
    let scene_empty_2 = Ulid::new();

    let now = Utc::now();
    let mk = |id, parent, kind, title: &str| Node {
        id,
        parent_id: parent,
        kind,
        title: title.into(),
        position: Node::DEFAULT_POSITION.into(),
        status: NodeStatus::Drafting,
        pov: None,
        beat: None,
        target_words: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    let nodes = vec![
        mk(project_id, None, NodeKind::Project, "Book"),
        mk(part_i, Some(project_id), NodeKind::Part, "Part I"),
        mk(part_ii, Some(project_id), NodeKind::Part, "Part II"),
        mk(chap_1, Some(part_i), NodeKind::Chapter, "Chapter 1"),
        mk(chap_2, Some(part_i), NodeKind::Chapter, "Chapter 2"),
        mk(chap_3, Some(part_ii), NodeKind::Chapter, "Chapter 3"),
        mk(scene_drafted, Some(chap_1), NodeKind::Scene, "Scene 1"),
        mk(scene_empty, Some(chap_2), NodeKind::Scene, "Scene 2"),
        mk(scene_empty_2, Some(chap_3), NodeKind::Scene, "Scene 3"),
    ];

    // Only Scene 1 has prose.
    let cleanup = empty_subtree_ids(&nodes, |id| id == scene_drafted);

    // Project is preserved. Part I and Chapter 1 are LIVE
    // (ancestors of a drafted scene). Chapter 2 / its empty scene /
    // Part II / Chapter 3 / its empty scene are all cleanup targets.
    assert!(!cleanup.contains(&project_id), "project root preserved");
    assert!(!cleanup.contains(&part_i), "Part I has a live scene");
    assert!(!cleanup.contains(&chap_1), "Chapter 1 has a live scene");
    assert!(!cleanup.contains(&scene_drafted), "drafted scene LIVE");

    assert!(cleanup.contains(&chap_2), "empty chapter cleanup");
    assert!(cleanup.contains(&scene_empty));
    assert!(cleanup.contains(&part_ii), "Part II entirely empty");
    assert!(cleanup.contains(&chap_3));
    assert!(cleanup.contains(&scene_empty_2));

    assert_eq!(cleanup.len(), 5);
}

#[test]
fn pre_cleanup_with_no_drafted_scenes_preserves_only_root() {
    // Empty template: nothing drafted. The cleanup must return every
    // node EXCEPT the project root. This is the "user regenerates an
    // outline on an empty template" case — wipe placeholders cleanly.
    let project_id = Ulid::new();
    let part_a = Ulid::new();
    let chap_a = Ulid::new();
    let scene_a = Ulid::new();

    let now = Utc::now();
    let mk = |id, parent, kind| Node {
        id,
        parent_id: parent,
        kind,
        title: String::new(),
        position: Node::DEFAULT_POSITION.into(),
        status: NodeStatus::Drafting,
        pov: None,
        beat: None,
        target_words: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let nodes = vec![
        mk(project_id, None, NodeKind::Project),
        mk(part_a, Some(project_id), NodeKind::Part),
        mk(chap_a, Some(part_a), NodeKind::Chapter),
        mk(scene_a, Some(chap_a), NodeKind::Scene),
    ];

    let cleanup = empty_subtree_ids(&nodes, |_id| false);
    assert_eq!(cleanup.len(), 3, "everything but project root");
    assert!(!cleanup.contains(&project_id));
    assert!(cleanup.contains(&part_a));
    assert!(cleanup.contains(&chap_a));
    assert!(cleanup.contains(&scene_a));
}

#[test]
fn pre_cleanup_with_all_drafted_scenes_returns_empty_list() {
    // Every scene has prose. Cleanup is a no-op.
    let project_id = Ulid::new();
    let chap = Ulid::new();
    let s1 = Ulid::new();
    let s2 = Ulid::new();

    let now = Utc::now();
    let mk = |id, parent, kind| Node {
        id,
        parent_id: parent,
        kind,
        title: String::new(),
        position: Node::DEFAULT_POSITION.into(),
        status: NodeStatus::Drafting,
        pov: None,
        beat: None,
        target_words: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let nodes = vec![
        mk(project_id, None, NodeKind::Project),
        mk(chap, Some(project_id), NodeKind::Chapter),
        mk(s1, Some(chap), NodeKind::Scene),
        mk(s2, Some(chap), NodeKind::Scene),
    ];

    let cleanup = empty_subtree_ids(&nodes, |id| id == s1 || id == s2);
    assert!(
        cleanup.is_empty(),
        "every scene drafted → nothing to clean up"
    );
}

#[test]
fn pre_cleanup_ignores_already_soft_deleted_nodes() {
    // Nodes with `deleted_at` set are skipped (no double-delete).
    let project_id = Ulid::new();
    let chap_live = Ulid::new();
    let chap_dead = Ulid::new();
    let scene_live = Ulid::new();
    let scene_dead = Ulid::new();

    let now = Utc::now();
    let mk = |id, parent, kind, dead: bool| Node {
        id,
        parent_id: parent,
        kind,
        title: String::new(),
        position: Node::DEFAULT_POSITION.into(),
        status: NodeStatus::Drafting,
        pov: None,
        beat: None,
        target_words: None,
        created_at: now,
        updated_at: now,
        deleted_at: if dead { Some(now) } else { None },
    };
    let nodes = vec![
        mk(project_id, None, NodeKind::Project, false),
        mk(chap_live, Some(project_id), NodeKind::Chapter, false),
        mk(chap_dead, Some(project_id), NodeKind::Chapter, true),
        mk(scene_live, Some(chap_live), NodeKind::Scene, false),
        mk(scene_dead, Some(chap_dead), NodeKind::Scene, true),
    ];

    // scene_live has prose; scene_dead is already deleted so its
    // prose state shouldn't matter to the result.
    let cleanup = empty_subtree_ids(&nodes, |id| id == scene_live);
    assert!(
        !cleanup.contains(&chap_dead),
        "soft-deleted nodes are skipped, not re-deleted"
    );
    assert!(
        !cleanup.contains(&scene_dead),
        "soft-deleted nodes are skipped, not re-deleted"
    );
    assert!(!cleanup.contains(&chap_live));
    assert!(!cleanup.contains(&scene_live));
}

#[test]
fn pre_cleanup_handles_empty_node_list() {
    let cleanup = empty_subtree_ids(&[], |_| false);
    assert!(cleanup.is_empty(), "no nodes → nothing to delete");
}

// ── ConceptScoreAxis: edge cases tied to UI rendering ────────────────────────

#[test]
fn axis_reason_field_round_trips_with_non_ascii() {
    // The UI shows the reason verbatim. Verify special chars + Unicode
    // don't get mangled in the serialisation contract.
    let original = axis_with_reason(8.5, "voice is spare — sentences trimmed for cadence");
    let v = serde_json::to_value(&original).unwrap();
    let back: ConceptScoreAxis = serde_json::from_value(v).unwrap();
    assert_eq!(back.score, 8.5);
    assert_eq!(back.reason, original.reason);
    assert!(back.reason.contains('—'));
}

#[test]
fn axis_default_is_zero_score_and_empty_reason() {
    let d = ConceptScoreAxis::default();
    assert_eq!(d.score, 0.0);
    assert!(d.reason.is_empty());
}

// ── BoilerplatePage: word-count + new() defaults ─────────────────────────────

#[test]
fn boilerplate_new_seeds_title_from_kind() {
    let p = BoilerplatePage::new("01", BoilerplateKind::Acknowledgments, 0);
    assert_eq!(p.title, "Acknowledgments");
    assert!(p.include_in_export);
    assert!(p.body_md.is_empty());
    assert_eq!(p.word_count(), 0);
}

#[test]
fn boilerplate_new_title_for_title_page_is_empty() {
    // Title page is the one kind where the export template renders
    // no heading — `new()` reflects that with an empty title default.
    let p = BoilerplatePage::new("01", BoilerplateKind::TitlePage, 0);
    assert!(p.title.is_empty());
}

#[test]
fn boilerplate_word_count_collapses_consecutive_whitespace() {
    // split_whitespace treats any run of whitespace as ONE separator,
    // so "for  my   grandmother" counts as 3 tokens, not 5. Verifies
    // the UI's "12 words" hint won't double-count from accidental
    // double-spaces in pasted text.
    let mut p = BoilerplatePage::new("01", BoilerplateKind::Dedication, 0);
    p.body_md = "for  my   grandmother\n\n\nAuthor".into();
    assert_eq!(p.word_count(), 4);
}
