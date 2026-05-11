//! Dialogue Polish Specialist (BACKLOG §A15 / Phase 2).
//!
//! Sharpens dialogue. Cuts exposition-as-dialogue. Adds subtext.
//! Differentiates speakers by cadence + lexicon. Voice-preserving by
//! design — touches dialogue + the action beats that bracket it, never
//! the surrounding narrative passages.

use booksforge_domain::{PolishProposal, PolishStageId};
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "speakers-undifferentiated",
        description: "Two characters in the same scene could swap lines without notice.",
        recoverable: true,
    },
    FailureMode {
        id: "narrative-mutated",
        description:
            "Stage touched narrative passages outside its dialogue + bracketing-beat remit.",
        recoverable: true,
    },
    FailureMode {
        id: "tag-adverb-overload",
        description:
            "Dialogue tags still carry adverbs (\"she said angrily\") instead of action beats.",
        recoverable: true,
    },
    FailureMode {
        id: "word-count-drift",
        description: "Revised chapter word count outside ±10% of the original.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "dialogue-polish",
        name:             "Dialogue Polish",
        purpose:          "Sharpen dialogue. Cut exposition-as-dialogue. Add subtext. Differentiate speakers by cadence and lexicon. Touches dialogue + bracketing action beats only — narrative passages stay exactly as written.",
        input_schema_id:  "DialoguePolishInput",
        output_schema_id: "PolishProposal",
        prompt_template:  PromptTemplateId::new("dialogue-polish", "v1"),
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
            CrossCuttingValidator::EntitySanity,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<PolishProposal, String> {
    crate::polish_common::parse_and_validate_polish(raw, PolishStageId::Dialogue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "dialogue-polish");
        assert_eq!(s.output_schema_id, "PolishProposal");
        assert_eq!(s.failure_modes.len(), 4);
    }

    #[test]
    fn parse_rejects_wrong_stage_id() {
        let raw = r#"{
          "stage_id": "voice",
          "revised_pm_doc": {"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"x"}]}]},
          "revised_word_count": 1,
          "edit_notes": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("stage_id mismatch"));
    }

    #[test]
    fn parse_accepts_well_formed_proposal() {
        let raw = r#"{
          "stage_id": "dialogue",
          "revised_pm_doc": {
            "type": "doc",
            "content": [
              {"type":"paragraph","content":[{"type":"text","text":"\"You up?\" she said."}]},
              {"type":"paragraph","content":[{"type":"text","text":"Nothing. Just the fridge clicking."}]}
            ]
          },
          "revised_word_count": 8,
          "edit_notes": "Cut exposition-as-dialogue in scene 2; replaced \"as you know\" beat with action beat."
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "expected ok, got {res:?}");
    }
}
