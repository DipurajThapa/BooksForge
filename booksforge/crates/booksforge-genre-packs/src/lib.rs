//! Genre packs (Layer 3, pure logic).
//!
//! BACKLOG §A16 / Phase 3C — Rust port of `artifacts/ghostwriter/
//! genre_packs.py`. Three distinct verticals (literary fiction, genre
//! fiction, non-fiction), each with:
//!
//!   - `system_prompt`        — used at every stage for that vertical
//!   - `draft_lens`           — per-scene drafter system addition
//!   - `critic_axes`          — 4-6 axes the per-scene critic scores
//!   - `polish_stack_order`   — ordered list of `PolishStageId`
//!   - `rubric_weights`       — per-axis weights (literary weighs voice
//!                              3×, pacing 1×; genre weighs pacing 3×,
//!                              voice 1.5×; etc.)
//!   - `hard_rules`           — non-negotiables (no fake stats for non-
//!                              fiction, etc.)
//!
//! The same orchestrator routes through different packs by `BookKind`.

#![forbid(unsafe_code)]

// Phase 4 of `PRODUCT_ROADMAP_E2E.md` — `BookKind` moved to
// `booksforge-domain` so the project schema, brief, and orchestrator
// can refer to the same type without a circular dependency. We re-export
// it here so existing call sites (`booksforge_genre_packs::BookKind`)
// keep working.
pub use booksforge_domain::BookKind;

use booksforge_domain::PolishStageId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// O1 of docs/VSM_LLM_OPTIMIZATION.md — deterministic per-stage
// "should we run this?" probes. Re-exported so the orchestrator can
// import them without a third-crate detour.
pub mod skip_detector;
pub use skip_detector::{should_run, skip_reason};
use ts_rs::TS;

/// One genre pack — system prompt, lens, critic axes, polish stack
/// ordering, rubric weights, hard rules.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GenrePack {
    pub kind: BookKind,
    /// Genre-label string the agent prompt templates branch on
    /// (`"literary_fiction" | "genre_fiction" | "non_fiction"`).
    pub genre_label: String,
    pub system_prompt: String,
    pub draft_lens: String,
    pub critic_axes: Vec<String>,
    pub polish_stack_order: Vec<String>, // serialised PolishStageId
    pub rubric_weights: BTreeMap<String, f32>,
    pub hard_rules: Vec<String>,
}

/// Build the pack for a given book kind. Returned by value because it
/// includes per-book localisation surfaces (would be configurable in a
/// later phase).
///
/// `Memoir` falls back to `LiteraryFiction` (similar prose-craft
/// emphasis with no-fabrication discipline). `ChildrensBook` falls back
/// to `LiteraryFiction` too — the wizard refuses ChildrensBook in MVP
/// per `BookKind::is_supported_in_mvp`.
pub fn pack_for(kind: BookKind) -> GenrePack {
    match kind {
        BookKind::LiteraryFiction | BookKind::Memoir | BookKind::ChildrensBook => literary_pack(),
        BookKind::GenreFiction => genre_pack(),
        BookKind::NonFiction => non_fiction_pack(),
    }
}

fn literary_pack() -> GenrePack {
    let mut weights: BTreeMap<String, f32> = BTreeMap::new();
    weights.insert("voice".to_owned(), 3.0);
    weights.insert("prose_quality".to_owned(), 3.0);
    weights.insert("originality".to_owned(), 2.0);
    weights.insert("character_depth".to_owned(), 2.0);
    weights.insert("emotional_impact".to_owned(), 2.0);
    weights.insert("pacing".to_owned(), 1.0);
    weights.insert("hook_strength".to_owned(), 1.0);
    weights.insert("dialogue".to_owned(), 2.0);
    weights.insert("structure".to_owned(), 1.5);
    weights.insert("commercial_readiness".to_owned(), 1.0);
    weights.insert("argument_strength".to_owned(), 0.5);
    weights.insert("evidence_handling".to_owned(), 0.5);
    weights.insert("authority_voice".to_owned(), 0.5);
    weights.insert("continuity".to_owned(), 1.5);
    weights.insert("formatting_readiness".to_owned(), 0.5);

    GenrePack {
        kind: BookKind::LiteraryFiction,
        genre_label: "literary_fiction".to_owned(),
        system_prompt: "\
You are working on a LITERARY fiction manuscript. Priorities, in order: \
voice, prose at the sentence level, interiority, subtext, originality of \
perception. Plot-velocity is secondary. Generic 'AI competent' prose is \
rejected — every sentence should feel placed, not generated. The narrator's \
specific perception of the world is the product."
            .to_owned(),
        draft_lens: "\
Drafting lens: literary. Prefer specificity over abstraction. The \
protagonist sees the world through a particular lens — every observation \
should reveal what THIS character notices that another wouldn't. Avoid \
generic mood-setting; pick one concrete sensory detail and let it carry. \
Sentences should vary — short, then long, then short. Subtext over \
subtext: dialogue says X, scene says Y, narrator's silence says Z."
            .to_owned(),
        critic_axes: vec![
            "scene_goal_clear".to_owned(),
            "specificity_of_perception".to_owned(),
            "voice_distinct".to_owned(),
            "subtext_present".to_owned(),
            "image_freshness".to_owned(),
            "interiority_earned".to_owned(),
        ],
        polish_stack_order: vec![
            PolishStageId::Voice.as_str().to_owned(),
            PolishStageId::Metaphor.as_str().to_owned(),
            PolishStageId::Dialogue.as_str().to_owned(),
            PolishStageId::SceneTension.as_str().to_owned(),
        ],
        rubric_weights: weights,
        hard_rules: vec![
            "Prefer specificity over abstraction.".to_owned(),
            "Every metaphor must come from the POV character's lived world.".to_owned(),
            "Do not flatten distinctive sentence shapes.".to_owned(),
        ],
    }
}

fn genre_pack() -> GenrePack {
    let mut weights: BTreeMap<String, f32> = BTreeMap::new();
    weights.insert("voice".to_owned(), 1.5);
    weights.insert("prose_quality".to_owned(), 1.5);
    weights.insert("originality".to_owned(), 1.5);
    weights.insert("character_depth".to_owned(), 1.5);
    weights.insert("emotional_impact".to_owned(), 2.0);
    weights.insert("pacing".to_owned(), 3.0);
    weights.insert("hook_strength".to_owned(), 2.5);
    weights.insert("dialogue".to_owned(), 2.0);
    weights.insert("structure".to_owned(), 2.5);
    weights.insert("commercial_readiness".to_owned(), 2.5);
    weights.insert("argument_strength".to_owned(), 0.5);
    weights.insert("evidence_handling".to_owned(), 0.5);
    weights.insert("authority_voice".to_owned(), 1.0);
    weights.insert("continuity".to_owned(), 2.0);
    weights.insert("formatting_readiness".to_owned(), 0.5);

    GenrePack {
        kind: BookKind::GenreFiction,
        genre_label: "genre_fiction".to_owned(),
        system_prompt: "\
You are working on a GENRE fiction manuscript (cozy fantasy / thriller / \
romance / mystery / YA). Priorities, in order: pacing, hooks, character \
agency, genre-conventional beats hit reliably. Each scene must turn the \
page. Predictable comfort is a feature, not a bug — but execution must \
feel fresh within the convention."
            .to_owned(),
        draft_lens: "\
Drafting lens: genre fiction. Each scene must (a) set a clear goal in the \
first 100 words, (b) escalate in the middle, (c) end on a hook. \
Protagonist agency: the protagonist makes a choice that matters in every \
scene, never a pure observer. Dialogue carries the story — keep narrative \
passages tight. Genre conventions are friends — hit them with style, don't \
subvert for the sake of it."
            .to_owned(),
        critic_axes: vec![
            "scene_goal_clear".to_owned(),
            "stakes_visible".to_owned(),
            "rising_tension".to_owned(),
            "hook_ending".to_owned(),
            "agency_of_protagonist".to_owned(),
            "convention_landed_with_flair".to_owned(),
        ],
        polish_stack_order: vec![
            PolishStageId::SceneTension.as_str().to_owned(),
            PolishStageId::Dialogue.as_str().to_owned(),
            PolishStageId::Metaphor.as_str().to_owned(),
            PolishStageId::Voice.as_str().to_owned(),
        ],
        rubric_weights: weights,
        hard_rules: vec![
            "Every scene ends on a hook.".to_owned(),
            "Protagonist agency in every scene.".to_owned(),
            "No info-dumps over 100 words.".to_owned(),
        ],
    }
}

fn non_fiction_pack() -> GenrePack {
    let mut weights: BTreeMap<String, f32> = BTreeMap::new();
    weights.insert("voice".to_owned(), 2.0);
    weights.insert("prose_quality".to_owned(), 1.5);
    weights.insert("originality".to_owned(), 2.0);
    weights.insert("character_depth".to_owned(), 0.5);
    weights.insert("emotional_impact".to_owned(), 1.0);
    weights.insert("pacing".to_owned(), 1.5);
    weights.insert("hook_strength".to_owned(), 1.5);
    weights.insert("dialogue".to_owned(), 0.5);
    weights.insert("structure".to_owned(), 2.5);
    weights.insert("commercial_readiness".to_owned(), 2.0);
    weights.insert("argument_strength".to_owned(), 3.0);
    weights.insert("evidence_handling".to_owned(), 3.0);
    weights.insert("authority_voice".to_owned(), 2.5);
    weights.insert("continuity".to_owned(), 1.5);
    weights.insert("formatting_readiness".to_owned(), 1.0);

    GenrePack {
        kind: BookKind::NonFiction,
        genre_label: "non_fiction".to_owned(),
        system_prompt: "\
You are working on a NON-FICTION manuscript (strategy / popular science / \
long-form essay / business). Priorities, in order: argument structure, \
evidence handling, authority voice, reader take-away clarity. NEVER invent \
statistics, studies, dates, or quotes — order-of-magnitude phrasing is \
acceptable; [SOURCE NEEDED] is acceptable; fabrication is not."
            .to_owned(),
        draft_lens: "\
Drafting lens: non-fiction. Every chapter has a thesis stated in the first \
200 words. Every section advances the thesis. Examples are concrete and \
earn their place — abstract claims without examples are rejected. Voice is \
authoritative but not lecturing — the reader is a peer, not a student. \
Quantitative claims are tagged [VERIFIED] / [SOURCE NEEDED] / \
[APPROXIMATE] / [FOR ILLUSTRATION]."
            .to_owned(),
        critic_axes: vec![
            "thesis_clear".to_owned(),
            "argument_advances".to_owned(),
            "evidence_handled_honestly".to_owned(),
            "examples_concrete".to_owned(),
            "authority_voice".to_owned(),
            "no_fabricated_specifics".to_owned(),
        ],
        // Non-fiction doesn't use the four fiction-polish stages directly;
        // the orchestrator should invoke chapter-level argument + evidence
        // polish here (planned for the non-fiction polish stack in a later
        // phase). For now we list the same 4 stages in argument-first
        // order so the wiring is consistent and the user sees the
        // intent — the underlying templates are voice-preserving by
        // design and won't actively hurt non-fiction prose.
        polish_stack_order: vec![
            PolishStageId::SceneTension.as_str().to_owned(),
            PolishStageId::Voice.as_str().to_owned(),
            PolishStageId::Dialogue.as_str().to_owned(),
            PolishStageId::Metaphor.as_str().to_owned(),
        ],
        rubric_weights: weights,
        hard_rules: vec![
            "NEVER fabricate stats, dates, names, studies, or quotes.".to_owned(),
            "Every chapter has an explicit thesis.".to_owned(),
            "Tag uncertain quantitative claims inline.".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_kind_roundtrip() {
        for k in [
            BookKind::LiteraryFiction,
            BookKind::GenreFiction,
            BookKind::NonFiction,
        ] {
            assert_eq!(BookKind::from_str(k.as_str()), Some(k));
        }
        // Underscore + collapsed aliases for forgiving parses.
        assert_eq!(
            BookKind::from_str("literary"),
            Some(BookKind::LiteraryFiction)
        );
        assert_eq!(BookKind::from_str("nonfiction"), Some(BookKind::NonFiction));
    }

    #[test]
    fn literary_weights_voice_higher_than_pacing() {
        let p = pack_for(BookKind::LiteraryFiction);
        let voice = p.rubric_weights["voice"];
        let pacing = p.rubric_weights["pacing"];
        assert!(voice > pacing, "literary should weigh voice > pacing");
    }

    #[test]
    fn genre_weights_pacing_higher_than_voice() {
        let p = pack_for(BookKind::GenreFiction);
        let voice = p.rubric_weights["voice"];
        let pacing = p.rubric_weights["pacing"];
        assert!(pacing > voice, "genre should weigh pacing > voice");
    }

    #[test]
    fn non_fiction_weights_argument_strength_max() {
        let p = pack_for(BookKind::NonFiction);
        let arg = p.rubric_weights["argument_strength"];
        let voice = p.rubric_weights["voice"];
        assert!(arg > voice);
    }

    #[test]
    fn polish_stack_order_starts_genre_correctly() {
        // Literary → Voice first.
        let lit = pack_for(BookKind::LiteraryFiction);
        assert_eq!(lit.polish_stack_order[0], "voice");
        // Genre → SceneTension first.
        let g = pack_for(BookKind::GenreFiction);
        assert_eq!(g.polish_stack_order[0], "scene_tension");
    }

    #[test]
    fn critic_axes_are_genre_specific() {
        let lit_axes = pack_for(BookKind::LiteraryFiction).critic_axes;
        let g_axes = pack_for(BookKind::GenreFiction).critic_axes;
        assert!(lit_axes.contains(&"subtext_present".to_owned()));
        assert!(lit_axes.contains(&"interiority_earned".to_owned()));
        assert!(g_axes.contains(&"hook_ending".to_owned()));
        assert!(g_axes.contains(&"agency_of_protagonist".to_owned()));
        // Non-overlapping at the right places.
        assert!(!lit_axes.contains(&"hook_ending".to_owned()));
        assert!(!g_axes.contains(&"interiority_earned".to_owned()));
    }

    #[test]
    fn non_fiction_hard_rules_include_no_fabrication() {
        let p = pack_for(BookKind::NonFiction);
        assert!(p.hard_rules.iter().any(|r| r.contains("NEVER fabricate")));
    }

    #[test]
    fn weighted_score_calculation_example() {
        // Rubric scores (1-10) for a literary manuscript.
        let lit = pack_for(BookKind::LiteraryFiction);
        let mut scores: BTreeMap<String, f32> = BTreeMap::new();
        scores.insert("voice".to_owned(), 8.0);
        scores.insert("prose_quality".to_owned(), 7.5);
        scores.insert("pacing".to_owned(), 5.0);
        scores.insert("dialogue".to_owned(), 7.0);
        // Weighted total / weight sum
        let mut total = 0.0;
        let mut wt_sum = 0.0;
        for (k, score) in &scores {
            if let Some(w) = lit.rubric_weights.get(k) {
                total += score * w;
                wt_sum += w;
            }
        }
        let weighted = total / wt_sum;
        // Voice + prose dominate (3.0 each), pacing barely counts (1.0).
        assert!(
            weighted > 7.0,
            "voice-heavy scoring should weight high; got {weighted}"
        );
    }
}
