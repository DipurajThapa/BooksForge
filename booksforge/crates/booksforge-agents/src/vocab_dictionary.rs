//! Vocabulary Dictionary Agent (AGENTS.md §4.8).
//!
//! Maintains project-layer vocabulary dictionaries from accepted edits.
//! Reads the recent accepted-edit log; proposes additions / modifications
//! that flow through the user gate before being applied.

use booksforge_domain::VocabUpdateProposals;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "duplicate-term",
        description: "Proposed addition duplicates an existing term in the same layer.",
        recoverable: true,
    },
    FailureMode {
        id: "kind-out-of-enum",
        description: "Entry kind is not prefer/avoid/replace.",
        recoverable: true,
    },
    FailureMode {
        id: "replace-without-target",
        description: "kind=replace without a replacement string.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "vocab-dictionary",
        name:             "Vocabulary Dictionary",
        purpose:          "Promote patterns the writer has accepted (or repeatedly rejected) into the project-layer vocabulary, so future agents apply the same preferences without re-asking. Adds prefer/avoid/replace entries with rationale and source-of-evidence; user-gated when promoting to higher layers.",
        input_schema_id:  "VocabUpdateInput",
        output_schema_id: "VocabUpdateProposals",
        prompt_template:  PromptTemplateId::new("vocab-dictionary", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 8_000,
            max_output_tokens:  2_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::Scheduled,
        user_gate:     UserGate::NotRequired,
        default_thinking: DefaultThinking::Disabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<VocabUpdateProposals, String> {
    let parsed: VocabUpdateProposals =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    let errs = parsed.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}
