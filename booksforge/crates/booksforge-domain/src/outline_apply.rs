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
    EmptyChapter { part_index: usize, chapter_index: usize },

    #[error("part[{part_index}].chapter[{chapter_index}].scene[{scene_index}] has empty synopsis")]
    EmptyScene {
        part_index:    usize,
        chapter_index: usize,
        scene_index:   usize,
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
    pub fn is_empty(&self) -> bool { self.creates.is_empty() }
    pub fn len(&self) -> usize { self.creates.len() }
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
    proposal:        &OutlineProposal,
    project_title:   &str,
    id_factory:      &mut dyn FnMut() -> Ulid,
    now:             DateTime<Utc>,
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
                    part_index: pi, chapter_index: ci,
                });
            }
            for (si, sc) in ch.scenes.iter().enumerate() {
                if sc.synopsis.trim().is_empty() {
                    return Err(OutlineApplyError::EmptyScene {
                        part_index: pi, chapter_index: ci, scene_index: si,
                    });
                }
            }
        }
    }

    // ── Build pass ──────────────────────────────────────────────────────
    let mut creates: Vec<Node> = Vec::with_capacity(estimate_size(proposal));

    // 1. Project root.
    let project_id = id_factory();
    creates.push(Node {
        id:           project_id,
        parent_id:    None,
        kind:         NodeKind::Project,
        title:        project_title.to_owned(),
        position:     Node::DEFAULT_POSITION.to_owned(),
        status:       NodeStatus::Planned,
        pov:          None,
        beat:         None,
        target_words: None,
        created_at:   now,
        updated_at:   now,
        deleted_at:   None,
    });

    // 2. Parts.
    let part_positions = initial_positions(proposal.parts.len());
    for (pi, part) in proposal.parts.iter().enumerate() {
        let part_id = id_factory();
        creates.push(Node {
            id:           part_id,
            parent_id:    Some(project_id),
            kind:         NodeKind::Part,
            title:        part.title.clone(),
            position:     part_positions[pi].clone(),
            status:       NodeStatus::Planned,
            pov:          None,
            beat:         None,
            target_words: None,
            created_at:   now,
            updated_at:   now,
            deleted_at:   None,
        });

        // 3. Chapters within this part.
        let chap_positions = initial_positions(part.chapters.len());
        for (ci, ch) in part.chapters.iter().enumerate() {
            let chap_id = id_factory();
            creates.push(Node {
                id:           chap_id,
                parent_id:    Some(part_id),
                kind:         NodeKind::Chapter,
                title:        ch.title.clone(),
                position:     chap_positions[ci].clone(),
                status:       NodeStatus::Planned,
                pov:          None,
                beat:         None,
                target_words: None,
                created_at:   now,
                updated_at:   now,
                deleted_at:   None,
            });

            // 4. Scenes within this chapter.
            let scene_positions = initial_positions(ch.scenes.len());
            for (si, sc) in ch.scenes.iter().enumerate() {
                let scene_id = id_factory();
                let scene_title = first_words(&sc.synopsis, 8);
                creates.push(Node {
                    id:           scene_id,
                    parent_id:    Some(chap_id),
                    kind:         NodeKind::Scene,
                    title:        scene_title,
                    position:     scene_positions[si].clone(),
                    status:       NodeStatus::Planned,
                    pov:          sc.pov.clone(),
                    beat:         sc.beat.clone(),
                    target_words: sc.target_word_count,
                    created_at:   now,
                    updated_at:   now,
                    deleted_at:   None,
                });
            }
        }
    }

    Ok(NodeTreeDelta { creates })
}

fn estimate_size(p: &OutlineProposal) -> usize {
    let parts = p.parts.len();
    let chapters: usize = p.parts.iter().map(|x| x.chapters.len()).sum();
    let scenes: usize = p
        .parts.iter()
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

    fn proposal(parts: usize, chapters_per_part: usize, scenes_per_chapter: usize) -> OutlineProposal {
        OutlineProposal {
            parts: (0..parts).map(|pi| PartPlan {
                title: format!("Part {}", pi + 1),
                purpose: "purpose".to_owned(),
                chapters: (0..chapters_per_part).map(|ci| ChapterPlan {
                    title: format!("Chapter {}.{}", pi + 1, ci + 1),
                    purpose: "purpose".to_owned(),
                    scenes: (0..scenes_per_chapter).map(|si| ScenePlan {
                        synopsis: format!("synopsis p{pi}c{ci}s{si}"),
                        pov: Some("Alice".to_owned()),
                        beat: None,
                        target_word_count: Some(1500),
                    }).collect(),
                }).collect(),
            }).collect(),
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
        let p = proposal(3, 4, 3);   // 3 × 4 chapters × 3 scenes
        let mut f = deterministic_factory();
        let delta = outline_to_tree(&p, "My Book", &mut f, Utc::now()).unwrap();
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
        let delta = outline_to_tree(&p, "T", &mut f, Utc::now()).unwrap();

        let project = delta.creates.iter().find(|n| n.kind == NodeKind::Project).unwrap();
        assert!(project.parent_id.is_none());

        for n in &delta.creates {
            if n.kind == NodeKind::Part {
                assert_eq!(n.parent_id, Some(project.id));
            }
        }
        // every chapter's parent is one of the parts; every scene's parent is one of the chapters
        let part_ids: Vec<Ulid> = delta.creates.iter().filter(|n| n.kind == NodeKind::Part).map(|n| n.id).collect();
        let chap_ids: Vec<Ulid> = delta.creates.iter().filter(|n| n.kind == NodeKind::Chapter).map(|n| n.id).collect();
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
        let delta = outline_to_tree(&p, "T", &mut f, Utc::now()).unwrap();

        // For each parent, its direct children must have strictly increasing positions.
        use std::collections::BTreeMap;
        let mut by_parent: BTreeMap<Option<Ulid>, Vec<&Node>> = BTreeMap::new();
        for n in &delta.creates { by_parent.entry(n.parent_id).or_default().push(n); }
        for (_, children) in by_parent {
            let mut prev: Option<&str> = None;
            for c in children {
                if let Some(p) = prev {
                    assert!(p < c.position.as_str(), "non-monotonic: {p} >= {}", c.position);
                }
                prev = Some(&c.position);
            }
        }
    }

    #[test]
    fn empty_proposal_errors() {
        let p = OutlineProposal { parts: vec![], rationale: "".into(), notes_to_user: vec![] };
        let mut f = deterministic_factory();
        assert!(matches!(
            outline_to_tree(&p, "T", &mut f, Utc::now()),
            Err(OutlineApplyError::EmptyProposal)
        ));
    }

    #[test]
    fn empty_chapter_errors_and_yields_no_partial_tree() {
        let mut p = proposal(1, 1, 1);
        p.parts[0].chapters[0].scenes.clear();
        let mut f = deterministic_factory();
        assert!(matches!(
            outline_to_tree(&p, "T", &mut f, Utc::now()),
            Err(OutlineApplyError::EmptyChapter { .. })
        ));
    }

    #[test]
    fn deterministic_ids_when_factory_is_deterministic() {
        let p = proposal(2, 2, 2);
        let mut f1 = deterministic_factory();
        let mut f2 = deterministic_factory();
        let a = outline_to_tree(&p, "T", &mut f1, Utc::now()).unwrap();
        let b = outline_to_tree(&p, "T", &mut f2, Utc::now()).unwrap();
        let ids_a: Vec<Ulid> = a.creates.iter().map(|n| n.id).collect();
        let ids_b: Vec<Ulid> = b.creates.iter().map(|n| n.id).collect();
        assert_eq!(ids_a, ids_b);
    }
}
