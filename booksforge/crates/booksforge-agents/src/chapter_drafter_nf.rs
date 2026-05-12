//! Non-fiction sibling of the Chapter Drafting Agent.
//!
//! Same input (`SceneContext`-shaped) and output (`SceneDraftProposal`)
//! schema as `chapter_drafter`, but binds to the `chapter-drafter-nf/v1`
//! template, which removes fiction conventions (POV, in-medias-res,
//! beat-shifts) and replaces them with thesis-first expository structure
//! and an explicit fabrication ban.
//!
//! Selection between this and the fiction `chapter_drafter` is the
//! orchestrator's job — typically by inspecting `ProjectBrief.mode`.
//! Both agents are findable from `find_agent`.

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
        id: "argument-repetition",
        description: "Draft restates the same claim across paragraphs without advancing the argument.",
        recoverable: true,
    },
    FailureMode {
        id: "fabricated-precision",
        description: "Draft asserts a precise statistic, dollar figure, percentage, or quote not present in known_entities or prior_summary.",
        recoverable: false,
    },
    FailureMode {
        id: "word-count-undershoot",
        description: "Draft is < 50% of the section's target_words.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id: "chapter-drafter-nf",
        name: "Chapter Drafting (non-fiction)",
        purpose: "Draft a non-fiction section from a synopsis. Sibling of chapter-drafter; same input/output schema. Replaces fiction conventions with thesis-first expository structure, explicit no-repetition rule, and explicit fabrication ban (no invented stats, percentages, quotes, case studies, sources). Off by default — opt-in per project, selected when ProjectBrief.mode = non_fiction.",
        input_schema_id: "SceneContext",
        output_schema_id: "SceneDraftProposal",
        prompt_template: PromptTemplateId::new("chapter-drafter-nf", "v1"),
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
        failure_modes:    FAILURE_MODES,
        when_to_run:      WhenToRun::OnDemand,
        user_gate:        UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<SceneDraftProposal, String> {
    let parsed: SceneDraftProposal =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    let errs = parsed.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_findable_with_correct_template_binding() {
        let s = spec();
        assert_eq!(s.id, "chapter-drafter-nf");
        assert_eq!(s.prompt_template.id, "chapter-drafter-nf");
        assert_eq!(s.prompt_template.version, "v1");
        assert_eq!(s.default_thinking, DefaultThinking::Disabled);
    }

    #[test]
    fn nf_uses_same_output_schema_as_fiction_chapter_drafter() {
        // The orchestrator must be able to apply either output without
        // separate code paths.
        let nf = spec();
        let f = crate::chapter_drafter::spec();
        assert_eq!(nf.output_schema_id, f.output_schema_id);
        assert_eq!(nf.input_schema_id, f.input_schema_id);
    }

    #[test]
    fn nf_has_argument_repetition_failure_mode() {
        let s = spec();
        assert!(
            s.failure_modes
                .iter()
                .any(|fm| fm.id == "argument-repetition"),
            "non-fiction drafter must declare argument-repetition as a failure mode"
        );
    }
}
