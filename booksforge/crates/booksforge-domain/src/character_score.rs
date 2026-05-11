//! Character critic output — Stage 3's quality gate.
//!
//! Scores each `CharacterCard` from the saved bible on five axes
//! (depth, consistency, uniqueness, narrative usefulness, emotional
//! impact). Returns per-card scores + per-card edits + a cross-card
//! consistency report (duplicate names, dangling relationships,
//! coverage percentage sanity).
//!
//! Gate (per the journey doc Stage 3): every card scores ≥ 8.5
//! composite AND zero duplicate-name findings AND zero dangling
//! relationships. Coverage sum ≤ 105%.

use crate::concept_score::ConceptScoreAxis;
use crate::quality_gate::{AXIS_FLOOR, COMPOSITE_THRESHOLD};
use crate::validator::{deserialize_severity_tolerant, Severity};
use serde::{Deserialize, Serialize};

/// Per-card edit suggestion. Identifies the card by name and the
/// specific `CharacterCard` field to revise.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterEdit {
    /// Name of the card the edit targets (must match a CharacterCard.name).
    pub character: String,
    /// Which field to change: `"name" | "role" | "external_objective"
    /// | "internal_need" | "fear_or_wound" | "secret_or_contradiction"
    /// | "voice_traits" | "relationships" | "chapter_arc"
    /// | "emotional_turning_points"`.
    pub field: String,
    /// Plain-English explanation.
    pub suggestion: String,
    /// Paste-ready replacement OR empty for structural edits.
    #[serde(default)]
    pub replacement: String,
}

/// Score + commentary for one `CharacterCard`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterScore {
    /// Name of the card this score belongs to.
    pub character: String,
    /// Five 0-10 axis scores (depth, consistency, uniqueness,
    /// narrative_usefulness, emotional_impact).
    pub depth: ConceptScoreAxis,
    pub consistency: ConceptScoreAxis,
    pub uniqueness: ConceptScoreAxis,
    pub narrative_usefulness: ConceptScoreAxis,
    pub emotional_impact: ConceptScoreAxis,
    /// One-paragraph editor's read of this character.
    #[serde(default)]
    pub overall_note: String,
}

impl CharacterScore {
    /// Mean of the five axes (0-10).
    pub fn composite(&self) -> f32 {
        (self.depth.score
            + self.consistency.score
            + self.uniqueness.score
            + self.narrative_usefulness.score
            + self.emotional_impact.score)
            / 5.0
    }

    /// Name of the lowest-scoring axis.
    pub fn weakest_axis(&self) -> &'static str {
        let pairs = [
            ("depth", self.depth.score),
            ("consistency", self.consistency.score),
            ("uniqueness", self.uniqueness.score),
            ("narrative_usefulness", self.narrative_usefulness.score),
            ("emotional_impact", self.emotional_impact.score),
        ];
        pairs
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("depth")
    }

    pub fn clamp_all(&mut self) {
        for a in [
            &mut self.depth,
            &mut self.consistency,
            &mut self.uniqueness,
            &mut self.narrative_usefulness,
            &mut self.emotional_impact,
        ] {
            a.clamp();
        }
    }
}

/// Cross-card structural finding (not per-card; about the whole bible).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossCardFinding {
    /// `"duplicate_name" | "dangling_relationship" | "coverage_sum_high"
    /// | "missing_antagonist" | "missing_protagonist" | "other"`.
    pub kind: String,
    /// Plain-English explanation, including the offending name(s).
    pub message: String,
    /// Severity grade. Tolerant deserialise: unknown / typo'd
    /// strings fall back to `Severity::Warning` rather than killing
    /// the whole proposal parse.
    #[serde(default, deserialize_with = "deserialize_severity_tolerant")]
    pub severity: Severity,
}

fn card_passes_gate(c: &CharacterScore) -> bool {
    c.composite() >= COMPOSITE_THRESHOLD
        && c.depth.score >= AXIS_FLOOR
        && c.consistency.score >= AXIS_FLOOR
        && c.uniqueness.score >= AXIS_FLOOR
        && c.narrative_usefulness.score >= AXIS_FLOOR
        && c.emotional_impact.score >= AXIS_FLOOR
}

/// Output of the `character-critic` agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CharacterCriticProposal {
    /// One entry per character in the bible. Order matches the
    /// bible's order (sorted by created_at on the storage side).
    #[serde(default)]
    pub scores: Vec<CharacterScore>,
    /// Structural findings about the bible as a whole.
    #[serde(default)]
    pub cross_card_findings: Vec<CrossCardFinding>,
    /// Per-character edit suggestions, sorted by impact.
    #[serde(default)]
    pub edits: Vec<CharacterEdit>,
    /// One-paragraph overall editor's read of the bible.
    #[serde(default)]
    pub overall_summary: String,
}

impl CharacterCriticProposal {
    /// Whole-bible composite (mean of every card's composite).
    pub fn composite(&self) -> f32 {
        if self.scores.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.scores.iter().map(|c| c.composite()).sum();
        sum / self.scores.len() as f32
    }

    /// Gate per the journey doc Stage 3:
    ///   - every card composite ≥ `COMPOSITE_THRESHOLD`
    ///   - every per-axis ≥ `AXIS_FLOOR`
    ///   - zero `Severity::Error` cross-card findings
    pub fn passes_gate(&self) -> bool {
        if self.scores.is_empty() {
            return false;
        }
        let all_cards_pass = self.scores.iter().all(card_passes_gate);
        let no_errors = self
            .cross_card_findings
            .iter()
            .all(|f| !f.severity.blocks_gate());
        all_cards_pass && no_errors
    }

    /// Names of cards that fail the per-card gate. Empty when the
    /// whole bible passes.
    pub fn weakest_cards(&self) -> Vec<String> {
        self.scores
            .iter()
            .filter(|c| !card_passes_gate(c))
            .map(|c| c.character.clone())
            .collect()
    }

    /// Clamp every axis on every card to [0, 10].
    pub fn clamp_all(&mut self) {
        for s in &mut self.scores {
            s.clamp_all();
        }
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

    fn char_score(name: &str, axes: [f32; 5]) -> CharacterScore {
        CharacterScore {
            character: name.to_owned(),
            depth: axis(axes[0]),
            consistency: axis(axes[1]),
            uniqueness: axis(axes[2]),
            narrative_usefulness: axis(axes[3]),
            emotional_impact: axis(axes[4]),
            overall_note: String::new(),
        }
    }

    #[test]
    fn composite_is_mean_per_card() {
        let c = char_score("Ada", [9.0, 8.0, 7.0, 8.0, 8.0]);
        assert!((c.composite() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn empty_proposal_does_not_pass_gate() {
        let p = CharacterCriticProposal::default();
        assert!(!p.passes_gate());
        assert!(p.weakest_cards().is_empty());
    }

    #[test]
    fn passes_gate_when_every_card_passes_and_no_errors() {
        let p = CharacterCriticProposal {
            scores: vec![
                char_score("Ada", [9.0, 9.0, 9.0, 9.0, 9.0]),
                char_score("Maeve", [8.5, 9.0, 8.5, 9.0, 8.5]),
            ],
            cross_card_findings: vec![CrossCardFinding {
                kind: "coverage_sum_high".into(),
                message: "Coverage sum is 110%, slightly over 105% target.".into(),
                severity: Severity::Warning,
            }],
            edits: vec![],
            overall_summary: String::new(),
        };
        assert!(p.passes_gate(), "warnings shouldn't block");
        assert!(p.weakest_cards().is_empty());
    }

    #[test]
    fn fails_gate_on_any_error_finding() {
        let p = CharacterCriticProposal {
            scores: vec![char_score("Ada", [9.5, 9.5, 9.5, 9.5, 9.5])],
            cross_card_findings: vec![CrossCardFinding {
                kind: "duplicate_name".into(),
                message: "Two characters named 'Ada'.".into(),
                severity: Severity::Error,
            }],
            edits: vec![],
            overall_summary: String::new(),
        };
        assert!(!p.passes_gate());
    }

    #[test]
    fn weakest_cards_lists_failing_names() {
        let p = CharacterCriticProposal {
            scores: vec![
                char_score("Strong", [9.0, 9.0, 9.0, 9.0, 9.0]),
                char_score("Weak", [6.0, 8.0, 8.0, 8.0, 8.0]), // axis 1 < 7
                char_score("Average", [7.0, 7.0, 7.0, 7.0, 7.0]), // composite 7.0 < 8.5
            ],
            ..Default::default()
        };
        let weak = p.weakest_cards();
        assert_eq!(weak.len(), 2);
        assert!(weak.contains(&"Weak".to_owned()));
        assert!(weak.contains(&"Average".to_owned()));
    }

    #[test]
    fn deserialise_tolerates_missing_optional_fields() {
        let json = r#"{
            "scores": [{
                "character": "Ada",
                "depth":                { "score": 9.0 },
                "consistency":          { "score": 9.0 },
                "uniqueness":           { "score": 9.0 },
                "narrative_usefulness": { "score": 9.0 },
                "emotional_impact":     { "score": 9.0 }
            }]
        }"#;
        let p: CharacterCriticProposal = serde_json::from_str(json).expect("parses");
        assert_eq!(p.scores.len(), 1);
        assert_eq!(p.cross_card_findings.len(), 0);
        assert_eq!(p.edits.len(), 0);
        assert!(p.passes_gate());
    }

    #[test]
    fn clamp_all_bounds_every_card_axes() {
        let mut p = CharacterCriticProposal {
            scores: vec![char_score("Ada", [15.0, -3.0, 9.0, 9.0, 9.0])],
            ..Default::default()
        };
        p.clamp_all();
        assert!(p.scores[0].depth.score <= 10.0);
        assert!(p.scores[0].consistency.score >= 0.0);
    }
}
