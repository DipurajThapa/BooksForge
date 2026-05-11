//! Deterministic "should we run this polish stage?" probes.
//!
//! Optimization O1 of `docs/VSM_LLM_OPTIMIZATION.md`. Each polish stage
//! has a hot-path detector that scans the prose for evidence the stage
//! has anything to operate on. When the detector returns `false`, the
//! orchestrator skips the stage entirely — no LLM call, no tokens, no
//! wall-clock. Estimated 30–40% wall-clock saving across a typical
//! literary-fiction full-scene pipeline run because most scenes have
//! at least one polish stage that's a near-no-op (a quiet introspective
//! scene with no dialogue, a dialogue-driven scene with no figurative
//! language, etc.).
//!
//! Each detector is deliberately conservative — when in doubt, run the
//! stage. False negatives (skipping when we shouldn't) cost quality;
//! false positives (running when we shouldn't) cost only the LLM call,
//! which is what we already do today. So the bias is toward running.
//!
//! Detection rules (each ~1ms; pure pattern scan, no parsing):
//!
//! - **Dialogue polish**: skip if the prose contains < 4 quote
//!   characters total (single + double + curly). 4 chars ≈ one
//!   back-and-forth pair; below that, there's nothing meaningful for
//!   a dialogue editor to sharpen. Apostrophes count: contraction-
//!   heavy prose ("I won't go", "she'd never") is "speech-flavoured"
//!   even without quotation marks.
//!
//! - **Metaphor polish**: skip if no figurative-language pattern
//!   matches. Patterns include "like a", "as if", "as though", "felt
//!   like", "looked like", "tapestry", "kaleidoscope", "symphony",
//!   "dance of", "journey", "weight of", and a small set of common
//!   simile/metaphor markers. Conservative: literary prose with even
//!   a couple of similes will trip a match.
//!
//! - **Voice polish**: never skip. Voice is the load-bearing axis for
//!   literary fiction (genre packs put it first in the polish stack)
//!   and operates at the sentence level on every sentence — no scene
//!   is "voice-empty."
//!
//! - **Scene-tension polish**: skip if the scene already ends on a
//!   strong hook (last sentence ends with `?`, `!`, `…`, or contains a
//!   reversal cue like "but", "until", "and then", "the door opened",
//!   etc.) AND has fewer than 3 sequential paragraphs of pure
//!   description (which is what scene-tension polish targets — slack
//!   in the middle).

use booksforge_domain::PolishStageId;

/// Returns `true` when the polish stage should run for the given
/// chapter/scene text. `false` means the orchestrator can skip it
/// safely. Conservative by design — when ambiguous, prefer to run.
pub fn should_run(stage: PolishStageId, prose: &str) -> bool {
    if prose.trim().is_empty() {
        // Empty input: nothing to polish anywhere.
        return false;
    }
    match stage {
        PolishStageId::Dialogue => has_meaningful_dialogue(prose),
        PolishStageId::Metaphor => has_figurative_language(prose),
        // Voice polish always runs — it operates at the sentence level
        // and there is no such thing as a "voice-empty" scene worth
        // including in a manuscript.
        PolishStageId::Voice => true,
        PolishStageId::SceneTension => needs_tension_polish(prose),
    }
}

/// Companion to `should_run` — returns a one-line reason the stage was
/// skipped, for the audit-ledger and the UI's per-stage badge. Returns
/// `None` when the stage SHOULD run.
pub fn skip_reason(stage: PolishStageId, prose: &str) -> Option<&'static str> {
    if prose.trim().is_empty() {
        return Some("scene is empty");
    }
    match stage {
        PolishStageId::Dialogue if !has_meaningful_dialogue(prose) => {
            Some("no meaningful dialogue (< 4 quote-flavoured chars)")
        }
        PolishStageId::Metaphor if !has_figurative_language(prose) => {
            Some("no figurative-language patterns detected")
        }
        PolishStageId::SceneTension if !needs_tension_polish(prose) => {
            Some("scene ends on a strong hook + no slack mid-scene")
        }
        _ => None,
    }
}

// ── Detectors ────────────────────────────────────────────────────────────────

fn has_meaningful_dialogue(prose: &str) -> bool {
    // Run #14 fix — the v1 detector required ≥ 4 quote-flavoured
    // characters, which correctly skipped scenes without spoken
    // dialogue. But fully-interior scenes (one character alone with
    // their thoughts) still benefit from the dialogue polish stage:
    // free-indirect discourse, interior-monologue rendered as direct
    // speech, and self-questioning forms (*Was she doing this? Yes,
    // she was doing this.*) are exactly what the dialogue polisher
    // sharpens. The Run #14 prose contained one such beat ("What was
    // she doing? She was doing this.") and the polish stage SKIPPED.
    //
    // New rule — run polish:dialogue if EITHER:
    //   1. ≥ 4 quote-flavoured characters (spoken dialogue), OR
    //   2. ≥ 1 sentence-final question mark *paired with* a
    //      first-person or third-person pronoun in the same sentence
    //      (interior monologue signature: "What was she doing?"
    //      "Was he sure?" "Could she stop?"), OR
    //   3. ≥ 2 italics-style emphatic markers (`*…*` or `_…_`)
    //      which most prose engines render as interior thought.
    let quote_chars: usize = prose
        .chars()
        .filter(|c| {
            matches!(
                c,
                '"' | '\'' | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}'
            )
        })
        .count();
    if quote_chars >= 4 {
        return true;
    }

    // Interior-monologue signature: question marks within sentences
    // that contain a first/third-person pronoun. Counts the rough
    // number of "Did she …?" / "What was he …?" beats.
    let interior_questions: usize = prose
        .split('?')
        .filter(|chunk| {
            let lc = chunk.to_lowercase();
            lc.contains(" she ")
                || lc.contains(" he ")
                || lc.contains(" they ")
                || lc.contains(" i ")
                || lc.starts_with("she ")
                || lc.starts_with("he ")
                || lc.starts_with("was ")
                || lc.starts_with("did ")
                || lc.starts_with("could ")
                || lc.starts_with("would ")
                || lc.starts_with("what ")
        })
        .count()
        .saturating_sub(1); // last split chunk has no closing '?'
    if interior_questions >= 1 {
        return true;
    }

    // Italics-style interior thought.
    let italics_pairs = prose.matches('*').count() / 2 + prose.matches('_').count() / 2;
    italics_pairs >= 2
}

const FIGURATIVE_PATTERNS: &[&str] = &[
    // Simile markers
    " like a ",
    " like an ",
    " like the ",
    " as if ",
    " as though ",
    " felt like ",
    " looked like ",
    " sounded like ",
    " seemed like ",
    " resembled ",
    // AI-pile metaphor cliches that ALWAYS deserve a polish pass
    "tapestry",
    "kaleidoscope",
    "symphony of",
    "dance of",
    "journey of",
    "weight of",
    "river of",
    // Body-as-object metaphors common in literary fiction
    " heart was ",
    " eyes were like ",
    " stomach turned ",
    // Generic "metaphor signal" verbs
    " bloomed",
    " unfolded",
    " cascaded",
    " shimmered",
];

fn has_figurative_language(prose: &str) -> bool {
    // Lowercase pass — the patterns above are lowercase and sentences
    // typically start with capitals, so a case-insensitive scan keeps
    // false negatives low.
    let lc = prose.to_lowercase();
    FIGURATIVE_PATTERNS.iter().any(|p| lc.contains(p))
}

const HOOK_WORDS: &[&str] = &[
    " but ",
    " until ",
    " and then ",
    " however ",
    " before ",
    "the door opened",
    "she turned",
    "he turned",
    "phone rang",
    "knock at",
    "stepped inside",
    "wasn't until",
];

fn needs_tension_polish(prose: &str) -> bool {
    // Two heuristics combine:
    //   1. Last sentence is a hook → likely doesn't need tension polish
    //   2. Sequence of ≥3 paragraphs without a verb-of-action → SLACK,
    //      definitely needs tension polish (overrides #1)
    let trimmed = prose.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Heuristic 2 wins if both fire — slack is more important than
    // a strong ending.
    if has_long_descriptive_run(trimmed) {
        return true;
    }
    !ends_on_hook(trimmed)
}

fn ends_on_hook(prose: &str) -> bool {
    let last = prose
        .split(['\n', '.'])
        .rev()
        .find(|s| !s.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if last.ends_with('?') || last.ends_with('!') || last.ends_with('\u{2026}') {
        return true;
    }
    HOOK_WORDS.iter().any(|w| last.contains(w))
}

fn has_long_descriptive_run(prose: &str) -> bool {
    // Very rough proxy: 3+ consecutive paragraphs with > 200 chars
    // each AND no question/exclamation mark = static description run.
    let paras: Vec<&str> = prose.split("\n\n").collect();
    let mut run = 0usize;
    for p in paras {
        let pt = p.trim();
        if pt.len() > 200 && !pt.contains('?') && !pt.contains('!') {
            run += 1;
            if run >= 3 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_prose_skips_all_stages() {
        for stage in [
            PolishStageId::Dialogue,
            PolishStageId::Metaphor,
            PolishStageId::Voice,
            PolishStageId::SceneTension,
        ] {
            assert!(!should_run(stage, ""), "{stage:?} ran on empty prose");
            assert!(skip_reason(stage, "").is_some());
        }
    }

    #[test]
    fn dialogue_skips_when_no_quotes() {
        let prose = "She walked into the kitchen and stared out the window for a long time.";
        assert!(!should_run(PolishStageId::Dialogue, prose));
        assert_eq!(
            skip_reason(PolishStageId::Dialogue, prose),
            Some("no meaningful dialogue (< 4 quote-flavoured chars)"),
        );
    }

    #[test]
    fn dialogue_runs_when_quotes_present() {
        let prose = r#""I won't go," she said. "Don't make me." "It's already done.""#;
        assert!(should_run(PolishStageId::Dialogue, prose));
        assert!(skip_reason(PolishStageId::Dialogue, prose).is_none());
    }

    #[test]
    fn dialogue_runs_on_interior_monologue_questions() {
        // Run #14 regression — the prose contained `What was she
        // doing? She was doing this.` which the v1 detector skipped
        // (no quote chars). The hardened detector recognises
        // interior-monologue questions and runs the polish stage.
        let prose = "She walked to the window. What was she doing? She did not know.";
        assert!(should_run(PolishStageId::Dialogue, prose));
        assert!(skip_reason(PolishStageId::Dialogue, prose).is_none());
    }

    #[test]
    fn dialogue_runs_on_italics_interior_thought() {
        // Free-indirect / italics-marked thought also counts.
        let prose = "She picked up the letter. *Was she doing this?* *She was doing this.*";
        assert!(should_run(PolishStageId::Dialogue, prose));
    }

    #[test]
    fn dialogue_still_skips_pure_narration_without_questions() {
        // No spoken dialogue, no interior questions, no italics — skip.
        let prose = "The wind moved across the field. The house was silent.";
        assert!(!should_run(PolishStageId::Dialogue, prose));
        assert!(skip_reason(PolishStageId::Dialogue, prose).is_some());
    }

    #[test]
    fn metaphor_skips_on_plain_prose() {
        let prose = "The kettle boiled. She poured the water into the cup and waited.";
        assert!(!should_run(PolishStageId::Metaphor, prose));
    }

    #[test]
    fn metaphor_runs_on_simile() {
        let prose = "He moved like a man underwater, slow and careful.";
        assert!(should_run(PolishStageId::Metaphor, prose));
    }

    #[test]
    fn metaphor_runs_on_ai_cliche() {
        let prose = "The grief was a tapestry of memory.";
        assert!(should_run(PolishStageId::Metaphor, prose));
    }

    #[test]
    fn voice_polish_always_runs() {
        assert!(should_run(PolishStageId::Voice, "anything"));
        assert!(should_run(PolishStageId::Voice, "x"));
    }

    #[test]
    fn scene_tension_skips_when_ending_on_hook() {
        let prose = "She closed the book. She walked to the door. The door opened.";
        assert!(!should_run(PolishStageId::SceneTension, prose));
    }

    #[test]
    fn scene_tension_runs_on_static_description_run() {
        // Three long descriptive paragraphs back-to-back = slack;
        // tension polish must run even though we technically end "fine."
        let p = "x".repeat(220);
        let prose = format!("{p}\n\n{p}\n\n{p}\n\nShe sat down and stared at the wall.",);
        assert!(should_run(PolishStageId::SceneTension, &prose));
    }

    #[test]
    fn scene_tension_runs_when_ending_flat() {
        let prose = "She walked across the room. She sat down. She thought about it.";
        assert!(should_run(PolishStageId::SceneTension, prose));
    }
}
