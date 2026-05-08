//! Memory Curator Agent (AGENTS.md §4.7).
//!
//! Maintains book / chapter / entity memory.  Refreshes chapter summaries
//! on finalise; suggests new entity cards.  Writes to memory go through
//! the orchestrator's `allowed_write_scopes("memory-curator")` check.

use booksforge_domain::MemoryRefreshProposals;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, FailureMode, ModelFamily, ModelPreference,
    ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode { id: "out-of-scope-write", description: "Proposed write outside book/chapter/entity scope.",          recoverable: true  },
    FailureMode { id: "duplicate-entity",   description: "Suggested new entity name collides with an existing one.",   recoverable: true  },
    FailureMode { id: "stale-summary",      description: "Summary describes events that aren't in the current text.",  recoverable: true  },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "memory-curator",
        name:             "Memory Curator",
        purpose:          "Maintain book/chapter/entity memory and the project's VoiceFingerprint. Refreshes chapter summaries on finalise; suggests new entity stubs from observed proper nouns; keeps the fingerprint current as the corpus grows. Writes are scope-checked against allowed_write_scopes(memory-curator).",
        input_schema_id:  "MemoryRefreshInput",
        output_schema_id: "MemoryRefreshProposals",
        prompt_template:  PromptTemplateId::new("memory-curator", "v1"),
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
            CrossCuttingValidator::MemoryScope,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::Scheduled,
        user_gate:     UserGate::NotRequired,
    }
}

/// Parse the model's raw output into a typed `MemoryRefreshProposals` and
/// run its semantic validators.
pub fn parse_and_validate(raw: &str) -> Result<MemoryRefreshProposals, String> {
    let parsed: MemoryRefreshProposals = serde_json::from_str(raw)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    let errs = parsed.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}
