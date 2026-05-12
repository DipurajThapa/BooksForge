//! Humanization Agent (AGENTS.md §4.9).
//!
//! Surfaces AI-tells using the layered vocabulary `avoid` rules; proposes
//! human-sounding alternatives drawn from the project's style memory.
//! Output is concrete `before/after` edit pairs (same shape as Copyeditor)
//! but justified by a `triggered_rule` from the vocab table.

use booksforge_domain::HumanizationProposals;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "rule-not-in-vocab",
        description: "triggered_rule does not match any active vocab `avoid` entry.",
        recoverable: true,
    },
    FailureMode {
        id: "range-mismatch",
        description: "before-text doesn't match source at the given range.",
        recoverable: true,
    },
    FailureMode {
        id: "voice-drift",
        description: "Replacement deviates from the project's style memory.",
        recoverable: true,
    },
    FailureMode {
        id: "over-correction",
        description: "Replaces a vocab term with another flagged term.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "humanization",
        name:             "Humanization",
        purpose:          "Detect AI-tells (cliché vocabulary, uniform sentence cadence, stock discourse markers) and propose human-sounding replacements grounded in the project's voice fingerprint and active avoid-rules. Output is concrete before/after edit pairs justified by a triggered_rule.",
        input_schema_id:  "HumanizationInput",
        output_schema_id: "HumanizationProposals",
        prompt_template:  PromptTemplateId::new("humanization", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 12_000,
            max_output_tokens:  4_000,
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

pub fn parse_and_validate(raw: &str, source_text: &str) -> Result<HumanizationProposals, String> {
    let parsed: HumanizationProposals =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    let errs = parsed.validate(source_text);
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}
