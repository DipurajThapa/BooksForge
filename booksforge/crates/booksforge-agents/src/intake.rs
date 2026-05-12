//! Project Intake Agent (AGENTS.md §4.1).
//!
//! Turns a free-text idea into a structured `ProjectBrief`.  Output flows
//! into the New Project Wizard's "review brief" step before the
//! Outline Architect runs.

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "empty-idea",
        description: "Idea text is empty or whitespace.",
        recoverable: false,
    },
    FailureMode {
        id: "off-topic",
        description: "Idea text is not a book pitch (e.g., a poem).",
        recoverable: false,
    },
    FailureMode {
        id: "word-count-extreme",
        description: "Target word count outside 5k–250k range.",
        recoverable: true,
    },
    FailureMode {
        id: "too-many-promises",
        description: "key_promises array > 6 items.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "intake",
        name:             "Project Intake",
        purpose:          "Convert the writer's free-text book idea into a typed ProjectBrief with mode, genre, audience, tone, premise, key promises, and up to five questions to surface anything critical that's missing. Never expands or fictionalises the idea.",
        input_schema_id:  "RawIdea",
        output_schema_id: "ProjectBrief",
        prompt_template:  PromptTemplateId::new("intake", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 4_000,
            max_output_tokens:  2_000,
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

/// Parse the model's raw output into a typed `ProjectBrief` and run the
/// brief's own `validate()`.  Returns `Err(reason)` so the runner can retry.
pub fn parse_and_validate(raw: &str) -> Result<booksforge_domain::ProjectBrief, String> {
    let brief: booksforge_domain::ProjectBrief =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    brief
        .validate()
        .map_err(|e| format!("brief validation failed: {e}"))?;
    Ok(brief)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "intake");
        assert_eq!(s.input_schema_id, "RawIdea");
        assert_eq!(s.output_schema_id, "ProjectBrief");
        assert_eq!(s.failure_modes.len(), 4);
    }
}
