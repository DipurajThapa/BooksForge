//! Audience Mapper Agent — Stage 2 (Phase C).
//!
//! Reads a saved `ProjectBrief` and emits a structured
//! `ReaderExpectationMap` (genre expectations + anti-patterns +
//! emotional promises + recommended themes/tropes + tropes to avoid +
//! pacing expectation).
//!
//! Unlike the four critic agents (concept/character/structure/scene),
//! this is a GENERATOR — there's no per-axis score, no 8.5/10 gate.
//! The "gate" is presence: Stage 2 passes when the map has non-empty
//! entries in the four required arrays
//! (`ReaderExpectationMap::is_complete()`).
//!
//! Light tier — the agent reads ~500 tokens and writes ~600 tokens of
//! structured JSON. 9B handles this cleanly in ~30-60 s.

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "brief-too-thin",
        description: "Brief has no premise — the mapper can't infer reader expectations from a blank seed.",
        recoverable: false,
    },
    FailureMode {
        id: "generic-genre-summary",
        description: "Output reads like a generic genre wikipedia entry, not a read of this specific brief.",
        recoverable: true,
    },
    FailureMode {
        id: "incomplete-map",
        description: "One or more required arrays (expectations, emotional_promises, themes, tropes_to_avoid) came back empty.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "audience-mapper",
        name:             "Audience Mapper",
        purpose:          "Generate a structured ReaderExpectationMap from a ProjectBrief: genre expectations + anti-patterns, emotional promises, recommended themes/tropes, tropes to avoid, pacing expectation, and an overall editor's note. Used as Stage 2's output.",
        input_schema_id:  "AudienceMapperInput",
        output_schema_id: "ReaderExpectationMap",
        prompt_template:  PromptTemplateId::new("audience-mapper", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Brief ~2 KB / ~500 tokens; output ~800 tokens of arrays.
            max_context_tokens: 4_000,
            max_output_tokens:  2_500,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes:   FAILURE_MODES,
        when_to_run:     WhenToRun::OnDemand,
        user_gate:       UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse a raw model response into a validated `ReaderExpectationMap`.
pub fn parse_and_validate(raw: &str) -> Result<booksforge_domain::ReaderExpectationMap, String> {
    let map: booksforge_domain::ReaderExpectationMap =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    // We don't gate on `is_complete()` here — the orchestrator retry
    // loop should handle that. Returning the map lets the UI surface
    // the soft-completeness even when one array is empty.
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::PacingExpectation;

    #[test]
    fn spec_fields_are_correct() {
        let s = spec();
        assert_eq!(s.id, "audience-mapper");
        assert_eq!(s.default_thinking, DefaultThinking::Disabled);
    }

    #[test]
    fn parse_accepts_minimal_valid_response() {
        let raw = r#"{
            "genre_expectations":  ["specific sensory detail"],
            "emotional_promises":  ["clarity after long ache"],
            "recommended_themes":  ["inheritance as silence"],
            "tropes_to_avoid":     ["chosen-one prophecy"]
        }"#;
        let m = parse_and_validate(raw).expect("parses");
        assert!(m.is_complete());
        assert_eq!(m.pacing_expectation, PacingExpectation::SlowBuild);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse_and_validate("not json").is_err());
    }

    #[test]
    fn parse_accepts_full_response_with_pacing() {
        let raw = r#"{
            "genre_expectations":  ["x"],
            "genre_anti_patterns": ["y"],
            "emotional_promises":  ["z"],
            "recommended_themes":  ["t"],
            "recommended_tropes":  ["u"],
            "tropes_to_avoid":     ["v"],
            "pacing_expectation":  "page_turner",
            "overall_note":        "An editor's note."
        }"#;
        let m = parse_and_validate(raw).expect("parses");
        assert_eq!(m.pacing_expectation, PacingExpectation::PageTurner);
        assert_eq!(m.total_entries(), 6);
    }
}
