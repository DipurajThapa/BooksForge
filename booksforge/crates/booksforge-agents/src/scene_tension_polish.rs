//! Scene Tension Polish Specialist (BACKLOG §A15 / Phase 2).
//!
//! Tightens the rising tension line per scene. Cuts slack passages
//! where the protagonist's situation does not change. Strengthens
//! scene-end hooks. Compresses repetitive description.

use booksforge_domain::{PolishProposal, PolishStageId};
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "scene-end-not-strengthened",
        description: "Last sentence of a scene still doesn't compel the next.",
        recoverable: true,
    },
    FailureMode {
        id: "plot-points-removed",
        description: "Stage cut a plot beat (events that change the protagonist's situation).",
        recoverable: true,
    },
    FailureMode {
        id: "dialogue-rewritten",
        description:
            "Stage touched dialogue lines (its remit is restructuring + cutting slack only).",
        recoverable: true,
    },
    FailureMode {
        id: "word-count-overshoot",
        description: "Stage compressed too aggressively (>30% cut).",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "scene-tension-polish",
        name:             "Scene Tension Polish",
        purpose:          "Tighten rising tension. Cut paragraphs where the protagonist's situation does not change. Strengthen scene-end hooks. Compress repetitive description. Restructures only — does not rewrite dialogue or plot beats.",
        input_schema_id:  "SceneTensionPolishInput",
        output_schema_id: "PolishProposal",
        prompt_template:  PromptTemplateId::new("scene-tension-polish", "v1"),
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
    crate::polish_common::parse_and_validate_polish(raw, PolishStageId::SceneTension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "scene-tension-polish");
        assert_eq!(s.output_schema_id, "PolishProposal");
    }

    #[test]
    fn parse_accepts_well_formed_proposal() {
        let raw = r#"{
          "stage_id": "scene_tension",
          "revised_pm_doc": {
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text":"x"}]}]
          },
          "revised_word_count": 1,
          "edit_notes": "Cut paragraph 7 (no change in situation). Tightened scene-end."
        }"#;
        assert!(parse_and_validate(raw).is_ok());
    }
}
