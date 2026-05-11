//! Deterministic paragraph-quality scorer (Item 5 of FEATURE_HARDENING_PLAN).
//!
//! Computes a 0.0–10.0 quality score for one paragraph (or short
//! prose chunk) using the existing voice + anti-tells signals.
//! Used by the exemplar-memory bootstrap path and by the runtime
//! drafter critique to decide which paragraphs are worth promoting
//! into `agent_exemplars`.
//!
//! The scorer is deterministic — no LLM call. The signals it
//! consumes are the same ones the voice contract and tells report
//! already produce, so the score is consistent with the verdict the
//! polish stack uses.
//!
//! Score components (each contributes a weighted slice of the 10.0
//! total):
//!
//!   - **Sensory grounding (2.0)** — concrete-noun count per
//!     1000 words. The single strongest signal of body-shaped
//!     vs model-shaped prose.
//!   - **Figurative language (1.5)** — simile/metaphor density.
//!     Real literary fiction lands 1-3 figures per paragraph;
//!     prose without any reads as flat.
//!   - **Sentence-rhythm variety (2.0)** — interquartile range of
//!     sentence lengths. IQR ≥ 4 = varied; IQR < 2 = mechanical.
//!   - **MATTR-50 (1.5)** — length-stable lexical diversity.
//!   - **No structural tells (2.0)** — penalises anaphora chains,
//!     substitution-game patterns, low-burstiness paragraphs,
//!     and pronoun-template runs.
//!   - **Low per-token tells (1.0)** — penalises lexicon hits
//!     (`delve`, `tapestry`, `intricate`, etc.), hedge openers,
//!     and clichés.

use crate::{find_all_tells, structural_tells};
use booksforge_voice::fingerprint;

/// Lookup set for concrete sensory nouns. Mirrors the
/// `CONCRETE_NOUNS` list in `structural.rs` but exposed here for
/// the scorer's grounding component.
const CONCRETE_NOUNS_SAMPLE: &[&str] = &[
    "skin", "hand", "knuckle", "wrist", "throat", "shoulder", "tongue", "lip", "tooth", "jaw",
    "hair", "eye", "ear", "nose", "cheek", "breath", "blood", "scar", "smell", "scent", "smoke",
    "salt", "metal", "iron", "rust", "leather", "soap", "wood", "earth", "rain", "mud", "fog",
    "dust", "ash", "click", "creak", "rasp", "scrape", "thud", "tap", "weight", "warmth", "heat",
    "cold", "chill", "draft", "breeze", "wind", "steam", "brass", "copper", "tin", "cloth",
    "linen", "wool", "paper", "twine", "rope", "knife", "spoon", "bowl", "cup", "kettle", "pan",
    "pot", "wall", "window", "door", "floor", "ceiling", "stair", "key", "lock", "stone", "gravel",
    "twig", "leaf", "branch", "root", "soil", "sand", "snow", "ice", "bread", "tea", "coffee",
    "wine", "milk", "butter", "cheese", "egg", "honey",
];

const FIGURATIVE_MARKERS: &[&str] = &[
    " like a ",
    " like an ",
    " like the ",
    " as if ",
    " as though ",
    " felt like ",
    " looked like ",
    " sounded like ",
    " seemed like ",
];

/// One paragraph's quality breakdown. Returned by [`score_paragraph`]
/// so the caller can audit which components landed which points.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphQualityScore {
    pub overall: f64,         // 0.0 - 10.0
    pub sensory: f64,         // 0.0 - 2.0
    pub figurative: f64,      // 0.0 - 1.5
    pub rhythm: f64,          // 0.0 - 2.0
    pub mattr: f64,           // 0.0 - 1.5
    pub no_structural: f64,   // 0.0 - 2.0
    pub low_token_tells: f64, // 0.0 - 1.0
    pub word_count: u32,
}

/// Score one paragraph against the deterministic quality rubric.
/// Returns the breakdown plus an `overall` 0.0-10.0.
///
/// Texts shorter than 30 words receive a uniformly low score (≤ 3.0);
/// the rubric needs enough material to evaluate.
pub fn score_paragraph(prose: &str) -> ParagraphQualityScore {
    let profile = fingerprint(prose);
    let word_count = profile.word_count;
    if word_count < 30 {
        return ParagraphQualityScore {
            overall: (word_count as f64 / 30.0) * 3.0,
            sensory: 0.0,
            figurative: 0.0,
            rhythm: 0.0,
            mattr: 0.0,
            no_structural: 0.0,
            low_token_tells: 0.0,
            word_count,
        };
    }

    // 1. Sensory grounding — concrete nouns per 1000 words, capped at 30/1000.
    let lower = prose.to_lowercase();
    let sensory_hits = CONCRETE_NOUNS_SAMPLE
        .iter()
        .map(|n| {
            lower.matches(&format!(" {n} ")).count() + lower.matches(&format!(" {n}.")).count()
        })
        .sum::<usize>() as f64;
    let sensory_per_k = 1000.0 * sensory_hits / word_count.max(1) as f64;
    // 30+ per 1000 = full marks; 0 = zero.
    let sensory = (sensory_per_k / 30.0).min(1.0) * 2.0;

    // 2. Figurative language — simile markers per paragraph, capped at 3.
    let figurative_hits = FIGURATIVE_MARKERS
        .iter()
        .map(|m| lower.matches(m).count())
        .sum::<usize>() as f64;
    let figurative = (figurative_hits / 3.0).min(1.0) * 1.5;

    // 3. Sentence-rhythm variety — IQR.
    let iqr = (profile.p75_sentence_length - profile.p25_sentence_length).max(0.0);
    let rhythm = ((iqr as f64) / 6.0).min(1.0) * 2.0;

    // 4. MATTR-50 — clamp [0.4, 0.8] → [0.0, 1.0].
    let mattr_norm = (((profile.mattr_50 as f64) - 0.4) / 0.4).clamp(0.0, 1.0);
    let mattr = mattr_norm * 1.5;

    // 5. No structural tells — penalise each by 0.5.
    let structural_count = structural_tells(prose).len() as f64;
    let no_structural = (2.0 - 0.5 * structural_count).max(0.0);

    // 6. Low per-token tells — measure weighted density per 1000 words.
    let all = find_all_tells(prose);
    let weighted: u32 = all.iter().map(|h| h.severity as u32).sum();
    let weighted_per_k = 1000.0 * weighted as f64 / word_count.max(1) as f64;
    // 0 weighted = full 1.0; 6+ weighted/1k = 0.
    let low_token_tells = (1.0 - (weighted_per_k / 6.0)).clamp(0.0, 1.0);

    let overall = sensory + figurative + rhythm + mattr + no_structural + low_token_tells;
    ParagraphQualityScore {
        overall,
        sensory,
        figurative,
        rhythm,
        mattr,
        no_structural,
        low_token_tells,
        word_count,
    }
}

/// Convenience: split a prose blob on `\n\n` into paragraphs and
/// score each one. Returns `(paragraph_text, score)` pairs sorted
/// by `score.overall` descending — top of list = best paragraph.
pub fn rank_paragraphs(prose: &str) -> Vec<(String, ParagraphQualityScore)> {
    let mut out: Vec<(String, ParagraphQualityScore)> = prose
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| (p.to_owned(), score_paragraph(p)))
        .collect();
    out.sort_by(|a, b| {
        b.1.overall
            .partial_cmp(&a.1.overall)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_prose_scores_low() {
        let s = score_paragraph("Too short.");
        assert!(s.overall < 3.0);
        assert_eq!(s.sensory, 0.0);
    }

    #[test]
    fn run14_quality_paragraph_scores_high() {
        // Reconstructed from Run #14's actual prose — the paragraph
        // that the user asked us to call out as quality output.
        let prose = "The iron key scraped against the tumblers. Elara held her breath. \
                     The lock gave with a heavy click. She pushed the drawer open. \
                     Dust motes drifted in the slanted light. The smell of machine oil \
                     and old paper filled her nose. Her fingers hovered over the wood. \
                     It was cold. She had avoided this drawer for three weeks. The \
                     silence of the workshop pressed against her ears. The fourteen \
                     clocks on the walls ticked out of sync. She reached in. Her \
                     knuckles brushed against brass weights.";
        let s = score_paragraph(prose);
        // Should clear 5.0 — concrete sensory grounding is the dominant
        // signal here (key, tumblers, drawer, dust, oil, paper, wood,
        // clocks, knuckles, brass) and there are no anaphora chains.
        assert!(
            s.overall >= 5.0,
            "Run #14 prose should score ≥ 5.0; got {:.2} ({:?})",
            s.overall,
            s,
        );
        assert!(s.sensory > 0.5, "sensory grounding should land");
    }

    #[test]
    fn run11_collapsed_prose_scores_low() {
        // Reconstructed from the Run #11 `the hand held` anaphora
        // paragraph — should score badly because the structural
        // detectors fire and there's no figurative/sensory variety.
        let prose = "The hand held the letter. The hand held the date. \
                     The hand held the name. The hand held the truth. \
                     The hand held the lie. The hand held the silence. \
                     The hand held the grief. The hand held the inheritance.";
        let s = score_paragraph(prose);
        assert!(
            s.overall < 5.0,
            "anaphora-collapsed prose must score < 5.0; got {:.2}",
            s.overall,
        );
    }

    #[test]
    fn rank_paragraphs_orders_best_first() {
        let prose = "Bad paragraph. Bad bad bad. Repeated repeated.\n\n\
                     The iron key scraped against the tumblers. The brass weights \
                     pressed against her knuckles. The dust hung in the slanted \
                     light. Her fingers hovered over the cold wood. The smell of \
                     oil and paper filled her nose.";
        let ranked = rank_paragraphs(prose);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].1.overall > ranked[1].1.overall);
    }

    #[test]
    fn score_breakdown_components_within_caps() {
        let s = score_paragraph(
            "The iron key scraped against the tumblers. The brass weights pressed against her knuckles. \
             The dust hung in the slanted light. Her fingers hovered over the cold wood. \
             The smell of oil and paper filled her nose like an echo of the year before.",
        );
        assert!(s.sensory <= 2.0);
        assert!(s.figurative <= 1.5);
        assert!(s.rhythm <= 2.0);
        assert!(s.mattr <= 1.5);
        assert!(s.no_structural <= 2.0);
        assert!(s.low_token_tells <= 1.0);
        assert!(s.overall <= 10.0);
    }
}
