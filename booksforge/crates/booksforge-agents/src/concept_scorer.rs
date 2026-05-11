//! Concept Scorer Agent — Stage 1 quality gate (Phase C).
//!
//! Reads a saved `ProjectBrief` and returns a `ConceptScoreProposal`:
//! five 0-10 axis scores (clarity, originality, emotional_pull,
//! market_fit, execution_potential) plus a composite, an overall
//! editor's note, and 0-5 specific revision suggestions.
//!
//! Why this is the first Phase C agent: every other quality gate
//! (Stage 2 audience map, Stage 3 character critic, Stage 4 structure
//! critic) follows the same pattern — small structured output, no
//! prose generation, runs in 20-60 s on the Light tier. Getting the
//! shape right here means the others can be copied in <30 minutes
//! each.
//!
//! Light tier (qwen3.5:9b) is the model picker default — the agent
//! reads ~2 KB of brief and writes ~500 tokens of structured JSON. A
//! larger model adds wall-clock without measurable score quality lift.

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "brief-too-thin",
        description: "Brief has no premise or key_promises — nothing to score.",
        recoverable: false,
    },
    FailureMode {
        id: "scorer-praise-bot",
        description: "All five axes return 9.0-10.0; suspect the model is praising every input.",
        recoverable: true,
    },
    FailureMode {
        id: "out-of-range-score",
        description: "An axis score is < 0 or > 10; clamped on deserialise but still flagged.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "concept-scorer",
        name:             "Concept Scorer",
        purpose:          "Score a ProjectBrief's concept (premise + key promises + audience + market fit) on five axes (0-10 each) and propose 0-5 targeted revisions when the composite is below 8.5. Used as Stage 1's quality gate.",
        input_schema_id:  "ConceptScorerInput",
        output_schema_id: "ConceptScoreProposal",
        prompt_template:  PromptTemplateId::new("concept-scorer", "v1"),
        model_preference: ModelPreference {
            // Light tier: brief is small, output is structured JSON,
            // no prose generation. 9B handles this cleanly in ~30-60s.
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Brief is ~2 KB / ~500 tokens; output is ~500 tokens of
            // structured JSON. Generous headroom on both sides for
            // long premises with rich background.
            max_context_tokens: 4_000,
            max_output_tokens:  2_000,
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
        // no quality lift here (same call we made for outline-architect).
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse a raw model response into a validated `ConceptScoreProposal`,
/// clamping any out-of-range axis scores.
pub fn parse_and_validate(raw: &str) -> Result<booksforge_domain::ConceptScoreProposal, String> {
    let mut proposal: booksforge_domain::ConceptScoreProposal =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    proposal.clamp_all();
    // Praise-bot detector: warn (but don't fail) when every axis ≥ 9.0
    // with a thin overall_summary. Lets the runner re-prompt with
    // "be harder on the concept" once we wire that loop.
    let all_nines = proposal.clarity.score >= 9.0
        && proposal.originality.score >= 9.0
        && proposal.emotional_pull.score >= 9.0
        && proposal.market_fit.score >= 9.0
        && proposal.execution_potential.score >= 9.0;
    if all_nines && proposal.overall_summary.trim().len() < 60 {
        tracing::warn!(
            agent = "concept-scorer",
            composite = proposal.composite(),
            "suspicious: all axes ≥ 9.0 with thin overall_summary — praise-bot warning",
        );
    }
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_fields_are_correct() {
        let s = spec();
        assert_eq!(s.id, "concept-scorer");
        assert_eq!(
            s.prompt_template,
            PromptTemplateId::new("concept-scorer", "v1")
        );
        assert_eq!(s.user_gate, UserGate::Required);
        assert_eq!(s.default_thinking, DefaultThinking::Disabled);
    }

    #[test]
    fn parse_and_validate_clamps_out_of_range_axes() {
        // Model returned a 15.0 — must be clamped to 10.0 before reaching
        // any consumer of the proposal.
        let raw = r#"{
            "clarity":             { "score": 15.0, "reason": "" },
            "originality":         { "score": 8.0 },
            "emotional_pull":      { "score": 7.0 },
            "market_fit":          { "score": -2.0 },
            "execution_potential": { "score": 9.0 }
        }"#;
        let p = parse_and_validate(raw).expect("parses");
        assert!(p.clarity.score <= 10.0);
        assert!(p.market_fit.score >= 0.0);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse_and_validate("not json").is_err());
    }

    #[test]
    fn parse_accepts_minimal_valid_response() {
        // Composite (sum / 5) must be ≥ 8.5 AND every axis ≥ 7.0.
        // [9.0, 8.0, 9.0, 8.5, 9.0] → composite 8.7. Passes.
        let raw = r#"{
            "clarity":             { "score": 9.0 },
            "originality":         { "score": 8.0 },
            "emotional_pull":      { "score": 9.0 },
            "market_fit":          { "score": 8.5 },
            "execution_potential": { "score": 9.0 }
        }"#;
        let p = parse_and_validate(raw).expect("parses");
        assert!(p.passes_gate(), "composite={}", p.composite());
    }
}
