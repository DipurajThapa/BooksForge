//! Structure critic output — Stage 4's quality gate.
//!
//! Scores a saved `OutlineProposal` against the brief on four axes:
//!   - promise_payoff    — Do the key_promises each get a payoff scene?
//!   - flow              — Tension curve / pacing across parts and acts.
//!   - reader_satisfaction — Does the ending answer the questions raised?
//!   - length_realism    — Per-scene targets and chapter count vs. brief.
//!
//! Plus structural findings (orphan chapters, missing climax markers,
//! arc gaps, sagging middle) and per-scene/per-chapter edit
//! suggestions. Same schema tolerance as `ConceptScoreProposal` and
//! `CharacterCriticProposal` — `#[serde(default)]` on every optional
//! field so a 9B model's slightly malformed JSON still salvages.
//!
//! Gate per the journey doc Stage 4:
//!   - composite ≥ 8.5
//!   - every axis ≥ 7.0
//!   - zero "error"-severity structural findings

use crate::concept_score::ConceptScoreAxis;
use crate::quality_gate::{AXIS_FLOOR, COMPOSITE_THRESHOLD};
use crate::validator::{deserialize_severity_tolerant, Severity};
use serde::{Deserialize, Serialize};

/// One actionable revision targeting a specific outline location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureEdit {
    /// Which axis of the outline the edit targets:
    /// `"part" | "chapter" | "scene" | "rationale" | "pacing"`.
    pub target: String,
    /// Optional human-readable locator the writer can find in the
    /// outline tree — e.g. "Part II / Chapter 7 / Scene 3" or
    /// "Chapter 12: The Wrong-Side Light". Empty when the edit is
    /// whole-outline scope.
    #[serde(default)]
    pub locator: String,
    /// Plain-English explanation of what to change and why.
    pub suggestion: String,
    /// Optional paste-ready replacement text (synopsis / title /
    /// rationale). Empty when the suggestion is structural ("move
    /// this scene to Chapter 5 instead").
    #[serde(default)]
    pub replacement: String,
}

/// Structural finding about the outline as a whole.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureFinding {
    /// `"orphan_chapter" | "missing_climax" | "promise_unpaid"
    /// | "sagging_middle" | "underweight_part" | "overweight_part"
    /// | "duplicate_scene_purpose" | "no_inciting_incident"
    /// | "thin_third_act" | "other"`.
    pub kind: String,
    /// Plain-English explanation, with specific part/chapter refs.
    pub message: String,
    /// Severity grade. Tolerant deserialise: unknown / typo'd
    /// strings fall back to `Severity::Warning` rather than killing
    /// the whole proposal parse.
    #[serde(default, deserialize_with = "deserialize_severity_tolerant")]
    pub severity: Severity,
}

/// Output of the `structure-critic` agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StructureCriticProposal {
    /// 0.0 – 10.0 — Do the brief's `key_promises` map to outline
    /// beats with payoff scenes near the ending?
    #[serde(default = "default_axis_seven")]
    pub promise_payoff: ConceptScoreAxis,
    /// 0.0 – 10.0 — Tension curve. Rising action through acts,
    /// midpoint reversal, climax in the right part, no sagging middle.
    #[serde(default = "default_axis_seven")]
    pub flow: ConceptScoreAxis,
    /// 0.0 – 10.0 — Does the ending close the questions raised by
    /// the inciting incident? Are character arcs resolved?
    #[serde(default = "default_axis_seven")]
    pub reader_satisfaction: ConceptScoreAxis,
    /// 0.0 – 10.0 — Per-scene targets and chapter count vs. brief.
    /// Excessively heavy or light scenes drag this down; sane
    /// rescaling is the writer's recourse.
    #[serde(default = "default_axis_seven")]
    pub length_realism: ConceptScoreAxis,
    /// One-paragraph editor's read of the outline.
    #[serde(default)]
    pub overall_summary: String,
    /// Structural findings (zero "error" severity entries to pass gate).
    #[serde(default)]
    pub findings: Vec<StructureFinding>,
    /// Per-location revision suggestions. Sorted by impact.
    #[serde(default)]
    pub edits: Vec<StructureEdit>,
}

fn default_axis_seven() -> ConceptScoreAxis {
    ConceptScoreAxis {
        score: 7.0,
        reason: String::new(),
    }
}

impl StructureCriticProposal {
    /// Composite score = arithmetic mean of the four axes. Range [0, 10].
    pub fn composite(&self) -> f32 {
        (self.promise_payoff.score
            + self.flow.score
            + self.reader_satisfaction.score
            + self.length_realism.score)
            / 4.0
    }

    /// Name of the lowest-scoring axis.
    pub fn weakest_axis(&self) -> &'static str {
        let pairs = [
            ("promise_payoff", self.promise_payoff.score),
            ("flow", self.flow.score),
            ("reader_satisfaction", self.reader_satisfaction.score),
            ("length_realism", self.length_realism.score),
        ];
        pairs
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("flow")
    }

    /// Gate per the journey doc Stage 4:
    ///   - composite ≥ `COMPOSITE_THRESHOLD`
    ///   - every axis ≥ `AXIS_FLOOR`
    ///   - zero `Severity::Error` findings
    pub fn passes_gate(&self) -> bool {
        let axes_pass = self.promise_payoff.score >= AXIS_FLOOR
            && self.flow.score >= AXIS_FLOOR
            && self.reader_satisfaction.score >= AXIS_FLOOR
            && self.length_realism.score >= AXIS_FLOOR;
        let composite_pass = self.composite() >= COMPOSITE_THRESHOLD;
        let no_errors = self.findings.iter().all(|f| !f.severity.blocks_gate());
        axes_pass && composite_pass && no_errors
    }

    /// Clamp every axis to [0, 10].
    pub fn clamp_all(&mut self) {
        for a in [
            &mut self.promise_payoff,
            &mut self.flow,
            &mut self.reader_satisfaction,
            &mut self.length_realism,
        ] {
            a.clamp();
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

    #[test]
    fn composite_is_mean_of_four_axes() {
        let p = StructureCriticProposal {
            promise_payoff: axis(8.0),
            flow: axis(9.0),
            reader_satisfaction: axis(8.0),
            length_realism: axis(7.0),
            ..Default::default()
        };
        assert!((p.composite() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn passes_gate_when_all_axes_above_floor_and_composite_above_threshold() {
        // [9, 9, 8.5, 9] → composite 8.875. All axes ≥ 7. Should pass.
        let p = StructureCriticProposal {
            promise_payoff: axis(9.0),
            flow: axis(9.0),
            reader_satisfaction: axis(8.5),
            length_realism: axis(9.0),
            ..Default::default()
        };
        assert!(p.passes_gate(), "composite={}", p.composite());
    }

    #[test]
    fn fails_gate_when_any_axis_below_floor() {
        // composite 8.5 but flow axis is below floor.
        let p = StructureCriticProposal {
            promise_payoff: axis(10.0),
            flow: axis(6.5),
            reader_satisfaction: axis(9.0),
            length_realism: axis(8.5),
            ..Default::default()
        };
        assert!(!p.passes_gate());
        assert_eq!(p.weakest_axis(), "flow");
    }

    #[test]
    fn fails_gate_on_error_severity_finding() {
        let p = StructureCriticProposal {
            promise_payoff: axis(9.5),
            flow: axis(9.5),
            reader_satisfaction: axis(9.5),
            length_realism: axis(9.5),
            findings: vec![StructureFinding {
                kind: "missing_climax".into(),
                message: "No clear climax scene in Part III.".into(),
                severity: Severity::Error,
            }],
            ..Default::default()
        };
        assert!(!p.passes_gate());
    }

    #[test]
    fn deserialise_tolerates_missing_optional_fields() {
        // Minimal JSON — every field optional or defaulted.
        let json = r#"{}"#;
        let p: StructureCriticProposal = serde_json::from_str(json).expect("parses");
        // Defaults are axis-score 7.0, so axes pass the 7.0 floor;
        // composite is 7.0 < 8.5, so it does NOT pass the gate. This
        // is the intended fallback: a model that returns empty JSON
        // gets a neutral score that nudges the writer to revise.
        assert!(!p.passes_gate());
        assert_eq!(p.findings.len(), 0);
        assert_eq!(p.edits.len(), 0);
    }

    #[test]
    fn clamp_all_bounds_every_axis() {
        let mut p = StructureCriticProposal {
            promise_payoff: axis(15.0),
            flow: axis(-3.0),
            reader_satisfaction: axis(9.0),
            length_realism: axis(9.0),
            ..Default::default()
        };
        p.clamp_all();
        assert!(p.promise_payoff.score <= 10.0);
        assert!(p.flow.score >= 0.0);
    }
}
