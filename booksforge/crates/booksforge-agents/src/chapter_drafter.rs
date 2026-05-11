//! Chapter Drafting Agent (AGENTS.md §4.3).
//!
//! Drafts a scene from a synopsis.  Off by default per project — the
//! orchestrator only invokes it after explicit user opt-in.

use booksforge_domain::SceneDraftProposal;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "fact-invention",
        description: "Draft introduces facts not in the brief or memory.",
        recoverable: true,
    },
    FailureMode {
        id: "voice-mismatch",
        description: "Draft voice deviates from declared style/tone.",
        recoverable: true,
    },
    FailureMode {
        id: "wrong-pov",
        description: "Draft switches POV against project's declared POV.",
        recoverable: true,
    },
    FailureMode {
        id: "word-count-undershoot",
        description: "Draft is < 50% of the scene's target_words.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "chapter-drafter",
        name:             "Chapter Drafting",
        purpose:          "Draft a scene from a synopsis when explicitly requested. Writes in the project's established voice (per VoiceFingerprint), respects POV and tense, never invents facts absent from the brief or memory, and surfaces a confidence rating. Off by default — opt-in per project.",
        input_schema_id:  "SceneContext",
        output_schema_id: "SceneDraftProposal",
        prompt_template:  PromptTemplateId::new("chapter-drafter", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::LongContext,
            min_size: ModelSizeHint::Large,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 16_000,
            max_output_tokens:  4_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
            CrossCuttingValidator::EntitySanity,
            CrossCuttingValidator::Originality,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<SceneDraftProposal, String> {
    // Defensive JSON repair (BACKLOG §A10) — local 9B/27B occasionally emits
    // a string placeholder where a list-of-objects is expected (e.g.
    // `pm_doc.content: [{...}, "text_node_2", {...}]`). Drop those before
    // serde-deserialise rather than wasting a full retry. Repair is logged
    // for the audit trail.
    let (repaired, audit) = crate::json_repair::parse_and_repair(raw)?;
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "chapter-drafter",
            dropped = audit.dropped_list_elements,
            "json_repair salvaged malformed list elements before deserialise",
        );
    }
    let parsed: SceneDraftProposal = serde_json::from_value(repaired)
        .map_err(|e| format!("JSON parse error after repair: {e}"))?;
    let errs = parsed.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}
