//! Voice Polish Specialist (BACKLOG §A15 / Phase 2).
//!
//! The opposite of "fix style". Voice-PRESERVING by design — its only
//! job is to amplify what is distinctive about the prose. The previous
//! single "polish" pass often made prose worse by smoothing distinctive
//! sentence shapes into generic AI-acceptable ones; this stage exists
//! to be the counterweight.

use booksforge_domain::{PolishProposal, PolishStageId};
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "voice-flattened",
        description: "Distinctive sentence shapes were smoothed into generic prose.",
        recoverable: true,
    },
    FailureMode {
        id: "hedge-openers-added",
        description:
            "Stage added \"Indeed,\" / \"Furthermore,\" / \"Moreover,\" to smooth transitions.",
        recoverable: true,
    },
    FailureMode {
        id: "marketing-adjectives-added",
        description: "Stage added marketing adjectives (vibrant, dynamic, bustling, intricate).",
        recoverable: true,
    },
    FailureMode {
        id: "voice-constraints-not-met",
        description: "Revised prose's measured fingerprint drifted from the supplied constraints.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "voice-polish",
        name:             "Voice Polish",
        purpose:          "Preserve and amplify author voice. Numeric voice constraints (median sentence length, dialogue ratio, em-dash density, etc.) come in as a vars block. The reader should be able to identify this as the same author across chapters by sentence cadence alone.",
        input_schema_id:  "VoicePolishInput",
        output_schema_id: "PolishProposal",
        prompt_template:  PromptTemplateId::new("voice-polish", "v1"),
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
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<PolishProposal, String> {
    crate::polish_common::parse_and_validate_polish(raw, PolishStageId::Voice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "voice-polish");
        assert_eq!(s.output_schema_id, "PolishProposal");
    }

    #[test]
    fn parse_accepts_well_formed_proposal() {
        let raw = r#"{
          "stage_id": "voice",
          "revised_pm_doc": {
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text":"She didn't turn the light on."}]}]
          },
          "revised_word_count": 6,
          "edit_notes": "Restored the comma splice in para 3 — it carries meaning here."
        }"#;
        assert!(parse_and_validate(raw).is_ok());
    }
}
