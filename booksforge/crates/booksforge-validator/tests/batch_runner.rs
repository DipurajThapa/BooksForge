#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration test for the batch validator runner + the export gate.

use booksforge_domain::{
    pre_export_gate, EntryKind, EntrySource, GateOutcome, Node, NodeKind, NodeStatus, Severity,
    StyleBook, ValidatorRunStatus, VocabEntry,
};
use booksforge_validator::{run_all_validators, SceneText, ValidatorContext};
use chrono::Utc;
use ulid::Ulid;

fn scene(id: Ulid, text: &str) -> SceneText {
    SceneText {
        node_id: id,
        text: text.to_owned(),
    }
}

fn node(id: Ulid, parent: Option<Ulid>, kind: NodeKind, title: &str) -> Node {
    let now = Utc::now();
    Node {
        id,
        parent_id: parent,
        kind,
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

#[test]
fn clean_manuscript_yields_pass_gate() {
    let scene_id = Ulid::new();
    let chap_id = Ulid::new();
    let proj_id = Ulid::new();
    let nodes = vec![
        node(proj_id, None, NodeKind::Project, "Test Book"),
        node(chap_id, Some(proj_id), NodeKind::Chapter, "Chapter 1"),
        node(scene_id, Some(chap_id), NodeKind::Scene, "Scene 1"),
    ];
    let scenes = vec![scene(
        scene_id,
        "A clean opening paragraph that goes for a while \
                         and uses normal punctuation and proper grammar.",
    )];
    let style = StyleBook::default();
    let ctx = ValidatorContext {
        nodes: &nodes,
        scenes: &scenes,
        style: &style,
        vocab: &[],
        active_vocab_layers: &["project", "ai_tells"],
        project: None,
    };

    let report = run_all_validators(&ctx);
    assert!(report.run.duration_ms < 5_000);
    assert_eq!(report.run.status, ValidatorRunStatus::Ok);
    assert!(matches!(pre_export_gate(&report), GateOutcome::Pass));
}

#[test]
fn manuscript_with_errors_blocks_export() {
    // Image without alt = ALT01 = Error.
    // Scene as direct child of Project = HRC01 (heading hierarchy) = Error.
    let project = Ulid::new();
    let bad_scene = Ulid::new();
    let nodes = vec![
        node(project, None, NodeKind::Project, "Test Book"),
        // Scene under project (should be under chapter) — HRC01.
        node(bad_scene, Some(project), NodeKind::Scene, "Stray Scene"),
    ];
    let scenes = vec![scene(
        bad_scene,
        "Look at this picture: ![](image.png) and beware  \
                          extra spaces.",
    )];
    let style = StyleBook::default();
    let ctx = ValidatorContext {
        nodes: &nodes,
        scenes: &scenes,
        style: &style,
        vocab: &[],
        active_vocab_layers: &["project", "ai_tells"],
        project: None,
    };

    let report = run_all_validators(&ctx);
    assert!(report.count(Severity::Error) >= 1);
    let outcome = pre_export_gate(&report);
    match outcome {
        GateOutcome::Block { errors, .. } => {
            assert!(errors.iter().any(|i| i.code == "ALT01"));
        }
        other => panic!("expected Block, got {other:?}"),
    }
}

#[test]
fn ai_tell_in_active_layer_warns_via_vocab() {
    let scene_id = Ulid::new();
    let chap_id = Ulid::new();
    let proj_id = Ulid::new();
    let nodes = vec![
        node(proj_id, None, NodeKind::Project, "Book"),
        node(chap_id, Some(proj_id), NodeKind::Chapter, "Chapter 1"),
        node(scene_id, Some(chap_id), NodeKind::Scene, "Scene 1"),
    ];
    let scenes = vec![scene(
        scene_id,
        "We will delve into the data and showcase the results.",
    )];
    let style = StyleBook::default();
    let vocab = vec![
        VocabEntry::new("ai_tells", "delve", EntryKind::Avoid, EntrySource::Starter),
        VocabEntry::new(
            "ai_tells",
            "showcase",
            EntryKind::Avoid,
            EntrySource::Starter,
        ),
    ];
    let ctx = ValidatorContext {
        nodes: &nodes,
        scenes: &scenes,
        style: &style,
        vocab: &vocab,
        active_vocab_layers: &["ai_tells"],
        project: None,
    };

    let report = run_all_validators(&ctx);
    let ai_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.validator_id == "ai-tells-detected")
        .collect();
    assert!(
        ai_issues.len() >= 2,
        "expected ≥2 ai-tells issues, got {}",
        ai_issues.len()
    );
    // ai_tells layer is Info severity per the spec.
    assert!(ai_issues.iter().all(|i| i.severity == Severity::Info));
}

#[test]
fn batch_runs_in_under_one_second_on_small_manuscript() {
    let scene_id = Ulid::new();
    let chap_id = Ulid::new();
    let proj_id = Ulid::new();
    let nodes = vec![
        node(proj_id, None, NodeKind::Project, "Book"),
        node(chap_id, Some(proj_id), NodeKind::Chapter, "Chapter 1"),
        node(scene_id, Some(chap_id), NodeKind::Scene, "Scene 1"),
    ];
    let big_text = "alpha bravo charlie ".repeat(2_000); // ~6000 words
    let scenes = vec![scene(scene_id, &big_text)];
    let style = StyleBook::default();
    let ctx = ValidatorContext {
        nodes: &nodes,
        scenes: &scenes,
        style: &style,
        vocab: &[],
        active_vocab_layers: &[],
        project: None,
    };

    let report = run_all_validators(&ctx);
    assert!(
        report.run.duration_ms < 1_000,
        "16 validators must finish in <1s on a single 6k-word scene; took {}ms",
        report.run.duration_ms,
    );
}
