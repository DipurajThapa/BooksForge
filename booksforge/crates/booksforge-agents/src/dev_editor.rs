//! Developmental Editor Agent (AGENTS.md §4.4).
//!
//! Produces structural notes per chapter — pacing, stakes, character,
//! POV tension, theme, structural balance.  Notes are advisory; the
//! user can convert any one into a TODO note attached to a scene.

use booksforge_domain::DevelopmentalNotes;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "axis-out-of-enum",
        description: "Note axis is not in the closed enum.",
        recoverable: true,
    },
    FailureMode {
        id: "evidence-missing",
        description: "Note has no evidence span.",
        recoverable: true,
    },
    FailureMode {
        id: "vague-suggestion",
        description: "Suggestion is non-actionable boilerplate.",
        recoverable: true,
    },
    FailureMode {
        id: "too-many-notes",
        description: "More than 25 notes for one chapter.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "dev-editor",
        name:             "Developmental Editor",
        purpose:          "Produce per-chapter structural notes across six axes (pacing, stakes, character, POV tension, theme, structural balance). Notes are advisory; each carries evidence and an actionable suggestion the writer can promote into a scene-level TODO.",
        input_schema_id:  "ChapterContext",
        output_schema_id: "DevelopmentalNotes",
        prompt_template:  PromptTemplateId::new("dev-editor", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::LongContext,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 24_000,
            max_output_tokens:  4_000,
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
        default_thinking: DefaultThinking::Enabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<DevelopmentalNotes, String> {
    let parsed: DevelopmentalNotes =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    let errs = parsed.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}
