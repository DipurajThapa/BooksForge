//! Structure Critic Agent — Stage 4 quality gate (Phase C).
//!
//! Reads the saved `OutlineProposal` + brief, returns a
//! `StructureCriticProposal` with:
//!   - Four axis scores (promise_payoff, flow, reader_satisfaction,
//!     length_realism), each 0-10.
//!   - Structural findings (missing climax, sagging middle, promise
//!     unpaid, orphan chapter, …).
//!   - Per-location edit suggestions (synopsis rewrites, scene
//!     reorders, length rebalances).
//!
//! Medium tier — the outline can run 8-15 KB of structured JSON, and
//! the editor needs to reason about whole-book pacing, which is past
//! the comfort zone for the Light tier. Wall-clock 60-180 s on
//! qwen3.5:27b.

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "no-outline",
        description: "No outline saved yet — nothing to score.",
        recoverable: false,
    },
    FailureMode {
        id: "praise-bot",
        description: "All four axes return 9.0-10.0 with thin reasoning — model is praising.",
        recoverable: true,
    },
    FailureMode {
        id: "fabricated-locator",
        description: "Edit's `locator` references a part/chapter/scene that isn't in the outline.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "structure-critic",
        name:             "Structure Critic",
        purpose:          "Score an OutlineProposal against the brief on four axes (promise_payoff, flow, reader_satisfaction, length_realism); flag structural findings (missing climax, sagging middle, promise unpaid); propose targeted edits. Used as Stage 4's quality gate.",
        input_schema_id:  "StructureCriticInput",
        output_schema_id: "StructureCriticProposal",
        prompt_template:  PromptTemplateId::new("structure-critic", "v1"),
        model_preference: ModelPreference {
            // Medium tier: outlines run 8-15 KB; whole-book pacing
            // reasoning is past the Light tier's comfort zone.
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Outline + brief ~10 KB / ~2 500 tokens; output ~1.5 KB of
            // structured JSON (4 axes + findings + edits). Generous
            // headroom for ~30-chapter outlines.
            max_context_tokens: 12_000,
            max_output_tokens:  3_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes:   FAILURE_MODES,
        when_to_run:     WhenToRun::OnDemand,
        user_gate:       UserGate::Required,
        // Structured small output — thinking-mode adds wall-clock with
        // no quality lift here (same call as concept-scorer /
        // character-critic).
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse a raw model response into a validated
/// `StructureCriticProposal`. Clamps every axis to [0, 10].
pub fn parse_and_validate(raw: &str) -> Result<booksforge_domain::StructureCriticProposal, String> {
    let mut proposal: booksforge_domain::StructureCriticProposal =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    proposal.clamp_all();
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_fields_are_correct() {
        let s = spec();
        assert_eq!(s.id, "structure-critic");
        assert_eq!(s.default_thinking, DefaultThinking::Disabled);
        assert_eq!(s.user_gate, UserGate::Required);
    }

    #[test]
    fn parse_and_validate_clamps_out_of_range() {
        let raw = r#"{
            "promise_payoff":      { "score": 15.0 },
            "flow":                { "score": -2.0 },
            "reader_satisfaction": { "score": 9.0 },
            "length_realism":      { "score": 9.0 }
        }"#;
        let p = parse_and_validate(raw).expect("parses");
        assert!(p.promise_payoff.score <= 10.0);
        assert!(p.flow.score >= 0.0);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse_and_validate("not json").is_err());
    }

    #[test]
    fn parse_accepts_full_response() {
        let raw = r#"{
            "promise_payoff":      { "score": 9.0, "reason": "every promise has a payoff scene in Part III" },
            "flow":                { "score": 9.0, "reason": "tension rises through Part II" },
            "reader_satisfaction": { "score": 8.5, "reason": "Ada's arc closes; Maeve's stays open" },
            "length_realism":      { "score": 9.0, "reason": "scene targets sum to 78k, brief is 75k" },
            "overall_summary": "A clean structural pass.",
            "findings": [],
            "edits": []
        }"#;
        let p = parse_and_validate(raw).expect("parses");
        assert!(p.passes_gate(), "composite={}", p.composite());
    }
}
