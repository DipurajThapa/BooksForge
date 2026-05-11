//! Metaphor Polish Specialist (BACKLOG §A15 / Phase 2).
//!
//! Replaces clichéd metaphors and dead similes with fresh,
//! character-specific images. Tunes density to the genre target.
//! Forbids generic-AI metaphors (tapestry, symphony, kaleidoscope,
//! dance, journey).

use booksforge_domain::{PolishProposal, PolishStageId};
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "ai-metaphor-substitution",
        description:
            "Replaced flagged words with OTHER LLM-favourite words (tapestry → symphony etc.).",
        recoverable: true,
    },
    FailureMode {
        id: "imagery-not-character-specific",
        description: "New images aren't drawn from the POV character's lived world.",
        recoverable: true,
    },
    FailureMode {
        id: "density-overshot",
        description: "Stage added far more images than the genre's density target.",
        recoverable: true,
    },
    FailureMode {
        id: "word-count-drift",
        description: "Revised chapter word count outside ±5% of the original.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "metaphor-polish",
        name:             "Metaphor Polish",
        purpose:          "Replace clichéd metaphors and dead similes with fresh, character-specific images. Tune density to the genre target (1-2/500w literary; 0.5-1/500w genre). Forbids generic-AI metaphors. Touches imagery only.",
        input_schema_id:  "MetaphorPolishInput",
        output_schema_id: "PolishProposal",
        prompt_template:  PromptTemplateId::new("metaphor-polish", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::LongContext,
            min_size: ModelSizeHint::Large,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 24_000,
            max_output_tokens:  6_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
            CrossCuttingValidator::Originality,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<PolishProposal, String> {
    crate::polish_common::parse_and_validate_polish(raw, PolishStageId::Metaphor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "metaphor-polish");
        assert_eq!(s.output_schema_id, "PolishProposal");
    }

    #[test]
    fn parse_rejects_empty_revised_doc() {
        let raw = r#"{
          "stage_id": "metaphor",
          "revised_pm_doc": {"type":"doc","content":[]},
          "revised_word_count": 0,
          "edit_notes": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("revised_pm_doc.content is empty"));
    }
}
