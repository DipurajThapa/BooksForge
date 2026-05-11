//! Concept scorer — Stage 1's quality gate output.
//!
//! Runs against a saved `ProjectBrief` and returns per-axis scores
//! (0-10) on five dimensions documented in
//! `book-output/design/WRITER_JOURNEY_REDESIGN_2026-05.md` §4 Stage 1:
//!   - Clarity         — does the premise read in one breath?
//!   - Originality     — vs. an embedded public-domain corpus + comps
//!   - Emotional pull  — is there a wound, a wonder, a question?
//!   - Market fit      — kind × genre × target word count coherent
//!   - Execution potential — can this book deliver what the premise promises?
//!
//! Plus a composite (mean of the five) and a short suggested-edit list
//! the writer can apply with one click. The gate passes when the
//! composite ≥ 8.5 AND every axis ≥ 7.0 (so a perfect 10 on four axes
//! can't mask a 4 on one — the weakest-axis is the prompt for revision).
//!
//! Schema is `#[serde(default)]` everywhere reasonable so a slightly
//! malformed JSON response from a 9B model still deserialises — same
//! tolerance pattern as `OutlineProposal` (see 2026-05-11 schema
//! relaxation in `outline.rs`).

use serde::{Deserialize, Serialize};

/// One axis of the concept score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ConceptScoreAxis {
    /// 0.0 – 10.0; clipped on deserialise.
    pub score: f32,
    /// One-sentence justification the writer reads under the score.
    #[serde(default)]
    pub reason: String,
}

impl ConceptScoreAxis {
    /// Bound score to [0, 10] so a misbehaving model can't poison
    /// downstream comparisons.
    pub fn clamp(&mut self) {
        if !self.score.is_finite() || self.score < 0.0 {
            self.score = 0.0;
        } else if self.score > 10.0 {
            self.score = 10.0;
        }
    }
}

/// One actionable revision suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptEdit {
    /// Which brief field the edit targets:
    /// `"premise" | "key_promises" | "audience" | "genre" | "tone" | "comp_titles_or_authors"`.
    pub field: String,
    /// Plain-English explanation of what to change and why.
    pub suggestion: String,
    /// Replacement text the writer can paste in. Empty when the
    /// suggestion is structural ("split this premise into two
    /// sentences") rather than substitutional.
    #[serde(default)]
    pub replacement: String,
}

/// Output of the `concept_scorer` agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptScoreProposal {
    pub clarity: ConceptScoreAxis,
    pub originality: ConceptScoreAxis,
    pub emotional_pull: ConceptScoreAxis,
    pub market_fit: ConceptScoreAxis,
    pub execution_potential: ConceptScoreAxis,
    /// One-paragraph overall read of the concept — the "editor's note"
    /// the writer sees alongside the dial.
    #[serde(default)]
    pub overall_summary: String,
    /// 0-5 specific edits the writer can apply. Sorted by impact: the
    /// first edit is the highest-leverage change.
    #[serde(default)]
    pub edits: Vec<ConceptEdit>,
}

impl ConceptScoreProposal {
    /// Composite score = arithmetic mean of the five axes. Range [0, 10].
    pub fn composite(&self) -> f32 {
        (self.clarity.score
            + self.originality.score
            + self.emotional_pull.score
            + self.market_fit.score
            + self.execution_potential.score)
            / 5.0
    }

    /// Name of the lowest-scoring axis. Used by the UI to point the
    /// writer at the field that most needs work.
    pub fn weakest_axis(&self) -> &'static str {
        let pairs = [
            ("clarity", self.clarity.score),
            ("originality", self.originality.score),
            ("emotional_pull", self.emotional_pull.score),
            ("market_fit", self.market_fit.score),
            ("execution_potential", self.execution_potential.score),
        ];
        pairs
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("clarity")
    }

    /// Quality gate: composite ≥ `COMPOSITE_THRESHOLD` AND every axis
    /// ≥ `AXIS_FLOOR`.
    ///
    /// The dual threshold prevents a perfect score on four axes from
    /// masking a catastrophic failure on the fifth — that's how
    /// "great premise, no market fit" books slip through generic
    /// scorers. The 7.0 floor is intentionally permissive (it's a
    /// "not embarrassing" bar, not "great"); the 8.5 composite is
    /// the "publishable" bar. Thresholds live in `crate::quality_gate`.
    pub fn passes_gate(&self) -> bool {
        use crate::quality_gate::{AXIS_FLOOR, COMPOSITE_THRESHOLD};
        self.composite() >= COMPOSITE_THRESHOLD
            && self.clarity.score >= AXIS_FLOOR
            && self.originality.score >= AXIS_FLOOR
            && self.emotional_pull.score >= AXIS_FLOOR
            && self.market_fit.score >= AXIS_FLOOR
            && self.execution_potential.score >= AXIS_FLOOR
    }

    /// Clamp every axis to [0, 10]. Call after deserialise so a
    /// 100-out-of-10 prank score from a model can't break the UI.
    pub fn clamp_all(&mut self) {
        self.clarity.clamp();
        self.originality.clamp();
        self.emotional_pull.clamp();
        self.market_fit.clamp();
        self.execution_potential.clamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(score: f32) -> ConceptScoreAxis {
        ConceptScoreAxis {
            score,
            reason: String::new(),
        }
    }

    fn proposal(scores: [f32; 5]) -> ConceptScoreProposal {
        ConceptScoreProposal {
            clarity: axis(scores[0]),
            originality: axis(scores[1]),
            emotional_pull: axis(scores[2]),
            market_fit: axis(scores[3]),
            execution_potential: axis(scores[4]),
            overall_summary: String::new(),
            edits: Vec::new(),
        }
    }

    #[test]
    fn composite_is_mean_of_five_axes() {
        let p = proposal([10.0, 8.0, 6.0, 9.0, 7.0]);
        // mean = 40/5 = 8.0
        assert!((p.composite() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn weakest_axis_returns_the_lowest_scorer() {
        let p = proposal([9.0, 9.0, 5.5, 9.0, 9.0]);
        assert_eq!(p.weakest_axis(), "emotional_pull");
    }

    #[test]
    fn passes_gate_requires_both_composite_and_axis_floor() {
        // Perfect except one weak axis below floor → fails gate.
        let p = proposal([10.0, 10.0, 5.0, 10.0, 10.0]);
        assert!(p.composite() >= 8.5);
        assert!(
            !p.passes_gate(),
            "axis floor (7.0) must veto a high composite"
        );

        // All axes ≥ 7.0 and composite ≥ 8.5 → passes.
        let p = proposal([8.5, 8.5, 8.5, 8.5, 8.5]);
        assert!(p.passes_gate());

        // Composite below 8.5 → fails even if every axis ≥ 7.
        let p = proposal([7.5, 7.5, 7.5, 7.5, 7.5]);
        assert!(!p.passes_gate(), "composite floor (8.5) must veto");
    }

    #[test]
    fn clamp_all_bounds_misbehaving_scores() {
        let mut p = proposal([15.0, -3.0, f32::NAN, f32::INFINITY, 5.0]);
        p.clamp_all();
        assert!(p.clarity.score >= 0.0 && p.clarity.score <= 10.0);
        assert!(p.originality.score >= 0.0 && p.originality.score <= 10.0);
        assert!(p.emotional_pull.score.is_finite());
        assert!(p.market_fit.score >= 0.0 && p.market_fit.score <= 10.0);
        assert_eq!(p.execution_potential.score, 5.0);
    }

    #[test]
    fn deserialise_tolerates_missing_optional_fields() {
        // Minimal JSON the model might return — no `reason`, no
        // `overall_summary`, no `edits`. Should deserialise cleanly
        // thanks to `#[serde(default)]`.
        // Scores chosen to comfortably pass the gate: composite = 8.7,
        // every axis ≥ 8.0.
        let json = r#"{
            "clarity":              { "score": 9.0 },
            "originality":          { "score": 8.0 },
            "emotional_pull":       { "score": 9.0 },
            "market_fit":           { "score": 8.5 },
            "execution_potential":  { "score": 9.0 }
        }"#;
        let p: ConceptScoreProposal = serde_json::from_str(json).expect("parses");
        assert_eq!(p.clarity.reason, "");
        assert_eq!(p.edits.len(), 0);
        assert!(
            p.passes_gate(),
            "composite={}, weakest={}",
            p.composite(),
            p.weakest_axis()
        );
    }
}
