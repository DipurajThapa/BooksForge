//! Outline → document tree pure transformation (Layer 3).
//!
//! `outline_to_tree` walks an [`OutlineProposal`] and produces a
//! [`NodeTreeDelta`] whose `creates` list, when inserted into SQLite as a
//! single batch, materialises the project's document tree.
//!
//! No I/O.  ULIDs are minted via an injectable factory so tests can use
//! deterministic IDs.  Timestamps are passed in by the caller (same reason).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::lexorank::initial_positions;
use crate::node::{Node, NodeKind, NodeStatus};
use crate::outline::OutlineProposal;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum OutlineApplyError {
    #[error("outline has no parts")]
    EmptyProposal,

    #[error("part[{part_index}] has no chapters")]
    EmptyPart { part_index: usize },

    #[error("part[{part_index}].chapter[{chapter_index}] has no scenes")]
    EmptyChapter {
        part_index: usize,
        chapter_index: usize,
    },

    #[error("part[{part_index}].chapter[{chapter_index}].scene[{scene_index}] has empty synopsis")]
    EmptyScene {
        part_index: usize,
        chapter_index: usize,
        scene_index: usize,
    },
}

// ── NodeTreeDelta ─────────────────────────────────────────────────────────────

/// A planned mutation to the document tree.
///
/// MVP supports `creates` only — outline application populates an empty
/// project.  Updates and deletions are deferred to later milestones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTreeDelta {
    pub creates: Vec<Node>,
}

impl NodeTreeDelta {
    pub fn is_empty(&self) -> bool {
        self.creates.is_empty()
    }
    pub fn len(&self) -> usize {
        self.creates.len()
    }
}

// ── outline_to_tree ───────────────────────────────────────────────────────────

/// Materialise an [`OutlineProposal`] into a [`NodeTreeDelta`].
///
/// `id_factory` mints fresh ULIDs (in deterministic tests inject a counter).
/// `now` is used for every node's `created_at` / `updated_at`.
///
/// The first entry in `creates` is always the project-root node.  The order
/// of subsequent entries is depth-first by document order: Project → Part →
/// Chapter → Scene.  Sibling positions are filled with [`initial_positions`]
/// so binder rendering matches outline order out of the box.
///
/// Returns an [`OutlineApplyError`] if the proposal is structurally
/// invalid — empty parts, empty chapters, or empty scene synopses.  Never
/// returns a partially-constructed delta.
pub fn outline_to_tree(
    proposal: &OutlineProposal,
    project_title: &str,
    id_factory: &mut dyn FnMut() -> Ulid,
    now: DateTime<Utc>,
    // Optional ID of an EXISTING project root to mount the outline
    // under. When `Some(id)`, the function does NOT create a new
    // project-root Node — it uses `id` as the parent of all parts and
    // returns a delta with parts/chapters/scenes only. This is the
    // path the desktop app uses: `project_create` already inserted a
    // root, so a second one would be a structural bug (and was, until
    // 2026-05-11 — see book-output/design/UX_REDESIGN_2026-05.md and
    // the apply_outline duplicate-root RCA).
    //
    // When `None`, behaves as before: emits a fresh project root.
    // Kept available for the CLI examples + tests that build a tree
    // from scratch in a temp bundle that has no pre-existing root.
    existing_root_id: Option<Ulid>,
) -> Result<NodeTreeDelta, OutlineApplyError> {
    // ── Validation pass (no allocations beyond errors) ───────────────────
    if proposal.parts.is_empty() {
        return Err(OutlineApplyError::EmptyProposal);
    }
    for (pi, part) in proposal.parts.iter().enumerate() {
        if part.chapters.is_empty() {
            return Err(OutlineApplyError::EmptyPart { part_index: pi });
        }
        for (ci, ch) in part.chapters.iter().enumerate() {
            if ch.scenes.is_empty() {
                return Err(OutlineApplyError::EmptyChapter {
                    part_index: pi,
                    chapter_index: ci,
                });
            }
            for (si, sc) in ch.scenes.iter().enumerate() {
                if sc.synopsis.trim().is_empty() {
                    return Err(OutlineApplyError::EmptyScene {
                        part_index: pi,
                        chapter_index: ci,
                        scene_index: si,
                    });
                }
            }
        }
    }

    // ── Build pass ──────────────────────────────────────────────────────
    let mut creates: Vec<Node> = Vec::with_capacity(estimate_size(proposal));

    // 1. Project root — either reuse the caller-supplied existing one,
    //    or create a fresh one. The Ulid we return as `project_id` is
    //    used downstream as the parent of all the parts; whether it
    //    came from the caller or the id_factory makes no difference
    //    to the build pass.
    let project_id = match existing_root_id {
        Some(id) => id,
        None => {
            let new_id = id_factory();
            creates.push(Node {
                id: new_id,
                parent_id: None,
                kind: NodeKind::Project,
                title: project_title.to_owned(),
                position: Node::DEFAULT_POSITION.to_owned(),
                status: NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            });
            new_id
        }
    };

    // 2. Parts.
    let part_positions = initial_positions(proposal.parts.len());
    for (pi, part) in proposal.parts.iter().enumerate() {
        let part_id = id_factory();
        creates.push(Node {
            id: part_id,
            parent_id: Some(project_id),
            kind: NodeKind::Part,
            title: part.title.clone(),
            position: part_positions[pi].clone(),
            status: NodeStatus::Planned,
            pov: None,
            beat: None,
            target_words: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        });

        // 3. Chapters within this part.
        let chap_positions = initial_positions(part.chapters.len());
        for (ci, ch) in part.chapters.iter().enumerate() {
            let chap_id = id_factory();
            creates.push(Node {
                id: chap_id,
                parent_id: Some(part_id),
                kind: NodeKind::Chapter,
                title: ch.title.clone(),
                position: chap_positions[ci].clone(),
                status: NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            });

            // 4. Scenes within this chapter.
            let scene_positions = initial_positions(ch.scenes.len());
            for (si, sc) in ch.scenes.iter().enumerate() {
                let scene_id = id_factory();
                let scene_title = first_words(&sc.synopsis, 8);
                creates.push(Node {
                    id: scene_id,
                    parent_id: Some(chap_id),
                    kind: NodeKind::Scene,
                    title: scene_title,
                    position: scene_positions[si].clone(),
                    status: NodeStatus::Planned,
                    pov: sc.pov.clone(),
                    beat: sc.beat.clone(),
                    target_words: sc.target_word_count,
                    created_at: now,
                    updated_at: now,
                    deleted_at: None,
                });
            }
        }
    }

    Ok(NodeTreeDelta { creates })
}

/// Given the current node tree and a `has_prose(node_id) -> bool`
/// predicate, return the IDs of nodes whose subtrees contain ZERO
/// scenes with prose. These are the empty placeholders to soft-delete
/// before applying a fresh outline.
///
/// Used by `apply_outline` to clean up template scaffolding (Generic
/// Novel's "Chapter 1 / Opening / Development / Climax" placeholders)
/// the moment a real outline is being applied — so the writer doesn't
/// end up with 30 placeholder scenes alongside the 15 outline-architect
/// chapters. **Preserves any scene with prose** and every ancestor
/// chain up to it.
///
/// Rules:
///   - A `Scene` with prose is LIVE; its ancestors transitively are LIVE.
///   - A `Scene` without prose is EMPTY.
///   - A `Chapter` / `Part` / `FrontMatter` / `BackMatter` / `Project`
///     is LIVE iff any descendant scene has prose.
///   - The project root is NEVER returned (we never soft-delete the
///     root the user is operating in).
///
/// The walk is two passes: first mark "scenes with prose" as the seed,
/// then bubble LIVE status up to ancestors. Any node not reached is
/// returned for soft-deletion. Soft-deleted nodes (`deleted_at` already
/// set) are skipped — we never re-delete what's already gone.
pub fn empty_subtree_ids<F>(nodes: &[Node], has_prose: F) -> Vec<Ulid>
where
    F: Fn(Ulid) -> bool,
{
    use std::collections::{HashMap, HashSet};

    // Skip already-deleted nodes — they were either soft-deleted earlier
    // or by a previous cleanup pass.
    let live_nodes: Vec<&Node> = nodes.iter().filter(|n| n.deleted_at.is_none()).collect();
    if live_nodes.is_empty() {
        return Vec::new();
    }

    // Build a parent_id → child_ids map for the LIVE subtree only.
    let mut children: HashMap<Ulid, Vec<Ulid>> = HashMap::new();
    let parent_of: HashMap<Ulid, Option<Ulid>> = live_nodes
        .iter()
        .map(|n| {
            if let Some(p) = n.parent_id {
                children.entry(p).or_default().push(n.id);
            }
            (n.id, n.parent_id)
        })
        .collect();

    // Seed: every scene WITH prose marks itself LIVE.
    let mut live: HashSet<Ulid> = HashSet::new();
    for n in &live_nodes {
        if n.kind == NodeKind::Scene && has_prose(n.id) {
            live.insert(n.id);
        }
    }

    // Bubble: for each seed, walk up parent_of and mark every ancestor LIVE.
    let seeds: Vec<Ulid> = live.iter().copied().collect();
    for seed in seeds {
        let mut cur = parent_of.get(&seed).copied().flatten();
        while let Some(pid) = cur {
            if !live.insert(pid) {
                break; // already LIVE — no need to keep walking
            }
            cur = parent_of.get(&pid).copied().flatten();
        }
    }

    // The project root is always preserved even if entirely empty —
    // apply_outline needs it to mount the new tree under.
    for n in &live_nodes {
        if n.kind == NodeKind::Project {
            live.insert(n.id);
        }
    }

    // Anything not LIVE is empty placeholder → return for soft-delete.
    live_nodes
        .iter()
        .filter(|n| !live.contains(&n.id))
        .map(|n| n.id)
        .collect()
}

fn estimate_size(p: &OutlineProposal) -> usize {
    let parts = p.parts.len();
    let chapters: usize = p.parts.iter().map(|x| x.chapters.len()).sum();
    let scenes: usize = p
        .parts
        .iter()
        .flat_map(|x| x.chapters.iter())
        .map(|c| c.scenes.len())
        .sum();
    1 + parts + chapters + scenes
}

fn first_words(s: &str, n: usize) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return "(scene)".to_owned();
    }
    let collected: Vec<&str> = trimmed.split_whitespace().take(n).collect();
    collected.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::{ChapterPlan, PartPlan, ScenePlan};

    fn proposal(
        parts: usize,
        chapters_per_part: usize,
        scenes_per_chapter: usize,
    ) -> OutlineProposal {
        OutlineProposal {
            parts: (0..parts)
                .map(|pi| PartPlan {
                    title: format!("Part {}", pi + 1),
                    purpose: "purpose".to_owned(),
                    chapters: (0..chapters_per_part)
                        .map(|ci| ChapterPlan {
                            title: format!("Chapter {}.{}", pi + 1, ci + 1),
                            purpose: "purpose".to_owned(),
                            scenes: (0..scenes_per_chapter)
                                .map(|si| ScenePlan {
                                    synopsis: format!("synopsis p{pi}c{ci}s{si}"),
                                    pov: Some("Alice".to_owned()),
                                    beat: None,
                                    target_word_count: Some(1500),
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            rationale: "r".to_owned(),
            notes_to_user: vec![],
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

    #[test]
    fn twelve_chapter_shape() {
        let p = proposal(3, 4, 3); // 3 × 4 chapters × 3 scenes
        let mut f = deterministic_factory();
        let delta = outline_to_tree(&p, "My Book", &mut f, Utc::now(), None).unwrap();
        // 1 project + 3 parts + 12 chapters + 36 scenes = 52
        assert_eq!(delta.creates.len(), 52);

        let by_kind = |k: NodeKind| delta.creates.iter().filter(|n| n.kind == k).count();
        assert_eq!(by_kind(NodeKind::Project), 1);
        assert_eq!(by_kind(NodeKind::Part), 3);
        assert_eq!(by_kind(NodeKind::Chapter), 12);
        assert_eq!(by_kind(NodeKind::Scene), 36);
    }

    #[test]
    fn parent_chain_is_valid() {
        let p = proposal(2, 2, 2);
        let mut f = deterministic_factory();
        let delta = outline_to_tree(&p, "T", &mut f, Utc::now(), None).unwrap();

        let project = delta
            .creates
            .iter()
            .find(|n| n.kind == NodeKind::Project)
            .unwrap();
        assert!(project.parent_id.is_none());

        for n in &delta.creates {
            if n.kind == NodeKind::Part {
                assert_eq!(n.parent_id, Some(project.id));
            }
        }
        // every chapter's parent is one of the parts; every scene's parent is one of the chapters
        let part_ids: Vec<Ulid> = delta
            .creates
            .iter()
            .filter(|n| n.kind == NodeKind::Part)
            .map(|n| n.id)
            .collect();
        let chap_ids: Vec<Ulid> = delta
            .creates
            .iter()
            .filter(|n| n.kind == NodeKind::Chapter)
            .map(|n| n.id)
            .collect();
        for n in delta.creates.iter().filter(|n| n.kind == NodeKind::Chapter) {
            assert!(part_ids.contains(&n.parent_id.unwrap()));
        }
        for n in delta.creates.iter().filter(|n| n.kind == NodeKind::Scene) {
            assert!(chap_ids.contains(&n.parent_id.unwrap()));
        }
    }

    #[test]
    fn sibling_positions_are_increasing() {
        let p = proposal(3, 4, 3);
        let mut f = deterministic_factory();
        let delta = outline_to_tree(&p, "T", &mut f, Utc::now(), None).unwrap();

        // For each parent, its direct children must have strictly increasing positions.
        use std::collections::BTreeMap;
        let mut by_parent: BTreeMap<Option<Ulid>, Vec<&Node>> = BTreeMap::new();
        for n in &delta.creates {
            by_parent.entry(n.parent_id).or_default().push(n);
        }
        for (_, children) in by_parent {
            let mut prev: Option<&str> = None;
            for c in children {
                if let Some(p) = prev {
                    assert!(
                        p < c.position.as_str(),
                        "non-monotonic: {p} >= {}",
                        c.position
                    );
                }
                prev = Some(&c.position);
            }
        }
    }

    #[test]
    fn empty_proposal_errors() {
        let p = OutlineProposal {
            parts: vec![],
            rationale: "".into(),
            notes_to_user: vec![],
        };
        let mut f = deterministic_factory();
        assert!(matches!(
            outline_to_tree(&p, "T", &mut f, Utc::now(), None),
            Err(OutlineApplyError::EmptyProposal)
        ));
    }

    #[test]
    fn empty_chapter_errors_and_yields_no_partial_tree() {
        let mut p = proposal(1, 1, 1);
        p.parts[0].chapters[0].scenes.clear();
        let mut f = deterministic_factory();
        assert!(matches!(
            outline_to_tree(&p, "T", &mut f, Utc::now(), None),
            Err(OutlineApplyError::EmptyChapter { .. })
        ));
    }

    #[test]
    fn deterministic_ids_when_factory_is_deterministic() {
        let p = proposal(2, 2, 2);
        let mut f1 = deterministic_factory();
        let mut f2 = deterministic_factory();
        let a = outline_to_tree(&p, "T", &mut f1, Utc::now(), None).unwrap();
        let b = outline_to_tree(&p, "T", &mut f2, Utc::now(), None).unwrap();
        let ids_a: Vec<Ulid> = a.creates.iter().map(|n| n.id).collect();
        let ids_b: Vec<Ulid> = b.creates.iter().map(|n| n.id).collect();
        assert_eq!(ids_a, ids_b);
    }

    #[test]
    fn outline_to_tree_with_existing_root_does_not_emit_a_second_root() {
        // Regression for the 2026-05-11 duplicate-root bug. When the
        // caller supplies an existing project root, the function MUST
        // NOT emit a Project node — it must mount parts under the
        // supplied root id directly. Until this fix, every
        // apply_outline call after project_create produced a 2nd
        // root sibling, leaving the binder with two trees.
        let p = proposal(2, 3, 2);
        let existing_root = Ulid(0x0000_0000_0000_0000_0000_0000_DEAD_BEEF);
        let mut f = deterministic_factory();
        let delta =
            outline_to_tree(&p, "Anything", &mut f, Utc::now(), Some(existing_root)).unwrap();
        let project_count = delta
            .creates
            .iter()
            .filter(|n| n.kind == NodeKind::Project)
            .count();
        assert_eq!(
            project_count, 0,
            "must not emit a project root when one was supplied"
        );
        for n in delta.creates.iter().filter(|n| n.kind == NodeKind::Part) {
            assert_eq!(n.parent_id, Some(existing_root));
        }
        // Total nodes = parts + chapters + scenes (no project)
        // = 2 + (2 × 3) + (2 × 3 × 2) = 2 + 6 + 12 = 20
        assert_eq!(delta.creates.len(), 20);
    }

    #[test]
    fn empty_subtree_ids_returns_empty_when_no_nodes() {
        assert!(empty_subtree_ids(&[], |_| false).is_empty());
    }

    #[test]
    fn empty_subtree_ids_keeps_scenes_with_prose_and_their_ancestors() {
        // Tree:
        //   project
        //   ├─ chapter A
        //   │   ├─ scene 1 (has prose)
        //   │   └─ scene 2 (empty)
        //   └─ chapter B (empty subtree)
        //       └─ scene 3 (empty)
        let now = Utc::now();
        let mk = |id: u128, parent: Option<Ulid>, kind: NodeKind| Node {
            id: Ulid(id),
            parent_id: parent,
            kind,
            title: format!("{kind:?}-{id}"),
            position: "p".to_owned(),
            status: crate::node::NodeStatus::Planned,
            pov: None,
            beat: None,
            target_words: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        let project_id = Ulid(1);
        let chapter_a_id = Ulid(10);
        let chapter_b_id = Ulid(20);
        let scene1_id = Ulid(100);
        let scene2_id = Ulid(200);
        let scene3_id = Ulid(300);
        let nodes = vec![
            mk(1, None, NodeKind::Project),
            mk(10, Some(project_id), NodeKind::Chapter),
            mk(20, Some(project_id), NodeKind::Chapter),
            mk(100, Some(chapter_a_id), NodeKind::Scene),
            mk(200, Some(chapter_a_id), NodeKind::Scene),
            mk(300, Some(chapter_b_id), NodeKind::Scene),
        ];
        // Only scene 1 has prose.
        let has_prose = |id: Ulid| id == scene1_id;

        let to_delete: std::collections::HashSet<Ulid> =
            empty_subtree_ids(&nodes, has_prose).into_iter().collect();

        // Project + chapter A + scene 1 → all LIVE (preserved).
        assert!(!to_delete.contains(&project_id));
        assert!(!to_delete.contains(&chapter_a_id));
        assert!(!to_delete.contains(&scene1_id));
        // Scene 2 is empty but its parent chapter is LIVE (because of
        // scene 1) — scene 2 itself still gets soft-deleted.
        assert!(to_delete.contains(&scene2_id));
        // Chapter B's entire subtree is empty → both soft-deleted.
        assert!(to_delete.contains(&chapter_b_id));
        assert!(to_delete.contains(&scene3_id));
    }

    #[test]
    fn empty_subtree_ids_never_deletes_project_root_even_if_fully_empty() {
        // The whole tree has no prose — but the project root must
        // survive because apply_outline mounts the new tree under it.
        let now = Utc::now();
        let project_id = Ulid(1);
        let chapter_id = Ulid(2);
        let nodes = vec![
            Node {
                id: project_id,
                parent_id: None,
                kind: NodeKind::Project,
                title: "p".into(),
                position: "p".into(),
                status: crate::node::NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
            Node {
                id: chapter_id,
                parent_id: Some(project_id),
                kind: NodeKind::Chapter,
                title: "c".into(),
                position: "p".into(),
                status: crate::node::NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
        ];
        let to_delete: std::collections::HashSet<Ulid> =
            empty_subtree_ids(&nodes, |_| false).into_iter().collect();
        assert!(!to_delete.contains(&project_id));
        assert!(to_delete.contains(&chapter_id));
    }

    #[test]
    fn empty_subtree_ids_skips_already_soft_deleted_nodes() {
        // A pre-soft-deleted node should not be returned (we don't
        // re-delete) and should not affect liveness propagation.
        let now = Utc::now();
        let project_id = Ulid(1);
        let dead_id = Ulid(2);
        let scene_id = Ulid(3);
        let nodes = vec![
            Node {
                id: project_id,
                parent_id: None,
                kind: NodeKind::Project,
                title: "p".into(),
                position: "p".into(),
                status: crate::node::NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
            Node {
                id: dead_id,
                parent_id: Some(project_id),
                kind: NodeKind::Chapter,
                title: "already gone".into(),
                position: "p".into(),
                status: crate::node::NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: Some(now),
            },
            Node {
                id: scene_id,
                parent_id: Some(dead_id),
                kind: NodeKind::Scene,
                title: "orphan".into(),
                position: "p".into(),
                status: crate::node::NodeStatus::Planned,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
        ];
        let to_delete: std::collections::HashSet<Ulid> =
            empty_subtree_ids(&nodes, |_| false).into_iter().collect();
        // Dead chapter is NOT returned (already soft-deleted).
        assert!(!to_delete.contains(&dead_id));
        // The orphan scene (parent is dead) IS returned as empty.
        assert!(to_delete.contains(&scene_id));
    }

    #[test]
    fn outline_to_tree_without_existing_root_keeps_legacy_behavior() {
        // The CLI examples (multi_chapter_run, live_book_run) build a
        // tree from scratch in a temp bundle that has no pre-existing
        // root. Make sure passing `None` still emits a project root.
        let p = proposal(1, 1, 1);
        let mut f = deterministic_factory();
        let delta = outline_to_tree(&p, "T", &mut f, Utc::now(), None).unwrap();
        assert_eq!(
            delta
                .creates
                .iter()
                .filter(|n| n.kind == NodeKind::Project)
                .count(),
            1,
            "must emit a project root when none is supplied"
        );
    }
}
