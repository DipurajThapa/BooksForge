#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Property test for `outline_to_tree` (MZ-07 acceptance criterion).
//!
//! Asserts: every randomly-generated `OutlineProposal` either
//!   (a) produces a `NodeTreeDelta` with the right invariants
//!       (counts match, parent chain is valid, sibling positions
//!        are strictly increasing, ULIDs are unique), or
//!   (b) returns a typed `OutlineApplyError`.
//!
//! It must **never** produce a partial tree, panic, or violate any
//! invariant.

use std::collections::{BTreeMap, HashSet};

use booksforge_domain::{
    outline_to_tree, ChapterPlan, NodeKind, NodeTreeDelta, OutlineApplyError, OutlineProposal,
    PartPlan, ScenePlan,
};
use chrono::Utc;
use proptest::prelude::*;
use ulid::Ulid;

// ── Generators ────────────────────────────────────────────────────────────────

fn arb_synopsis() -> impl Strategy<Value = String> {
    // Sometimes empty (to exercise the EmptyScene error path), often non-empty.
    prop_oneof![
        Just("".to_owned()),
        "[A-Za-z][A-Za-z0-9 ]{1,40}".prop_map(String::from),
    ]
}

fn arb_scene() -> impl Strategy<Value = ScenePlan> {
    (
        arb_synopsis(),
        prop::option::of("[A-Z][a-z]{1,10}".prop_map(String::from)),
        prop::option::of("[a-z]{3,15}".prop_map(String::from)),
        prop::option::of(500u32..5_000u32),
    ).prop_map(|(synopsis, pov, beat, target_word_count)| ScenePlan {
        synopsis, pov, beat, target_word_count,
    })
}

fn arb_chapter() -> impl Strategy<Value = ChapterPlan> {
    (
        "[A-Za-z][A-Za-z0-9 ]{1,30}".prop_map(String::from),
        "[A-Za-z][A-Za-z0-9 ]{1,40}".prop_map(String::from),
        prop::collection::vec(arb_scene(), 0..5),
    ).prop_map(|(title, purpose, scenes)| ChapterPlan { title, purpose, scenes })
}

fn arb_part() -> impl Strategy<Value = PartPlan> {
    (
        "[A-Za-z][A-Za-z0-9 ]{1,30}".prop_map(String::from),
        "[A-Za-z][A-Za-z0-9 ]{1,40}".prop_map(String::from),
        prop::collection::vec(arb_chapter(), 0..4),
    ).prop_map(|(title, purpose, chapters)| PartPlan { title, purpose, chapters })
}

fn arb_proposal() -> impl Strategy<Value = OutlineProposal> {
    (
        prop::collection::vec(arb_part(), 0..4),
        "[A-Za-z0-9 ,.]{0,80}".prop_map(String::from),
        prop::collection::vec("[A-Za-z0-9 ]{1,30}".prop_map(String::from), 0..3),
    ).prop_map(|(parts, rationale, notes_to_user)| OutlineProposal {
        parts, rationale, notes_to_user,
    })
}

// ── Invariant check (used in the Ok branch) ───────────────────────────────────

fn assert_delta_is_valid(proposal: &OutlineProposal, delta: &NodeTreeDelta) {
    let expected = 1
        + proposal.parts.len()
        + proposal.parts.iter().map(|p| p.chapters.len()).sum::<usize>()
        + proposal.parts.iter()
            .flat_map(|p| p.chapters.iter())
            .map(|c| c.scenes.len())
            .sum::<usize>();
    assert_eq!(delta.creates.len(), expected, "node count mismatch");

    // Exactly one project root.
    let projects: Vec<_> = delta.creates.iter().filter(|n| n.kind == NodeKind::Project).collect();
    assert_eq!(projects.len(), 1, "expected exactly one project node");
    assert!(projects[0].parent_id.is_none(), "project root has parent");

    // ULIDs unique.
    let ids: HashSet<Ulid> = delta.creates.iter().map(|n| n.id).collect();
    assert_eq!(ids.len(), delta.creates.len(), "duplicate ULIDs in delta");

    // Parent chain valid: every non-project node's parent is in the delta.
    for node in &delta.creates {
        if let Some(parent) = node.parent_id {
            assert!(ids.contains(&parent), "node {} references missing parent {}", node.id, parent);
        }
    }

    // Siblings (same parent) must have strictly increasing LexoRank positions.
    let mut by_parent: BTreeMap<Option<Ulid>, Vec<&str>> = BTreeMap::new();
    for n in &delta.creates {
        by_parent.entry(n.parent_id).or_default().push(&n.position);
    }
    for (_, positions) in by_parent {
        let mut iter = positions.iter();
        if let Some(mut prev) = iter.next() {
            for cur in iter {
                assert!(prev < cur, "non-monotonic sibling positions: {prev} >= {cur}");
                prev = cur;
            }
        }
    }
}

fn deterministic_factory() -> impl FnMut() -> Ulid {
    let mut counter: u128 = 1;
    move || {
        let id = Ulid(counter);
        counter += 1;
        id
    }
}

// ── The property ──────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        .. ProptestConfig::default()
    })]

    #[test]
    fn outline_to_tree_total_with_no_partial_output(proposal in arb_proposal()) {
        let mut factory = deterministic_factory();
        let result = outline_to_tree(&proposal, "Test Book", &mut factory, Utc::now());

        match result {
            Ok(delta) => assert_delta_is_valid(&proposal, &delta),
            Err(OutlineApplyError::EmptyProposal) => {
                prop_assert!(proposal.parts.is_empty());
            }
            Err(OutlineApplyError::EmptyPart { part_index }) => {
                prop_assert!(part_index < proposal.parts.len());
                prop_assert!(proposal.parts[part_index].chapters.is_empty());
            }
            Err(OutlineApplyError::EmptyChapter { part_index, chapter_index }) => {
                prop_assert!(part_index < proposal.parts.len());
                let part = &proposal.parts[part_index];
                prop_assert!(chapter_index < part.chapters.len());
                prop_assert!(part.chapters[chapter_index].scenes.is_empty());
            }
            Err(OutlineApplyError::EmptyScene { part_index, chapter_index, scene_index }) => {
                let scene = &proposal.parts[part_index].chapters[chapter_index].scenes[scene_index];
                prop_assert!(scene.synopsis.trim().is_empty());
            }
        }
    }
}
