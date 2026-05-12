//! Scene Critic Agent (BACKLOG §A15 / Phase 2 — per-scene critique-revise loop).
//!
//! Reads a drafted scene + the genre's critic axes, returns a JSON
//! object with per-axis scores (1-10) and concrete edit instructions.
//! The orchestrator's reviser then runs a narrow-scope revision pass
//! that applies the edits without rewriting the rest of the scene.

use booksforge_domain::SceneCritiqueProposal;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "scores-out-of-range",
        description: "An axis score is outside 1-10.",
        recoverable: true,
    },
    FailureMode {
        id: "weakest-axis-not-in-scores",
        description: "weakest_axis names an axis missing from the scores map.",
        recoverable: true,
    },
    FailureMode {
        id: "edit-quote-empty",
        description: "A targeted edit's problem_quote or fix is empty.",
        recoverable: true,
    },
    FailureMode {
        id: "summary-too-long",
        description: "overall_one_liner > 30 words (forces specificity).",
        recoverable: true,
    },
    FailureMode {
        id: "axes-mismatch",
        description: "Returned axis names don't match the genre's critic_axes input.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "scene-critic",
        name:             "Scene Critic",
        purpose:          "Score a drafted scene on the genre's craft axes (1-10 each) and return targeted edit instructions the reviser can apply without rewriting the whole scene. Drives the per-scene critique-revise loop in the fiction polish stack.",
        input_schema_id:  "SceneCritiqueInput",
        output_schema_id: "SceneCritiqueProposal",
        prompt_template:  PromptTemplateId::new("scene-critic", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Large,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 12_000,
            max_output_tokens:  3_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        // Critique benefits from thinking-mode — judging craft on
        // multiple axes at once is the kind of multi-step reasoning
        // small thinking budgets help with.
        default_thinking: DefaultThinking::Enabled,
    }
}

/// Parse the model's raw output into a `SceneCritiqueProposal` and run
/// `validate()`. Uses the strict-objects json_repair on the
/// `specific_edits` list (which must be objects, not stray strings).
pub fn parse_and_validate(raw: &str) -> Result<SceneCritiqueProposal, String> {
    let (repaired, audit) =
        crate::json_repair::parse_and_repair_strict_objects(raw, &["specific_edits"])?;
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "scene-critic",
            dropped = audit.dropped_list_elements,
            "json_repair salvaged malformed list elements before deserialise",
        );
    }
    let proposal: SceneCritiqueProposal = serde_json::from_value(repaired)
        .map_err(|e| format!("JSON parse error after repair: {e}"))?;
    let errs = proposal.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "scene-critic");
        assert_eq!(s.output_schema_id, "SceneCritiqueProposal");
        assert!(matches!(s.default_thinking, DefaultThinking::Enabled));
    }

    #[test]
    fn parse_rejects_score_out_of_range() {
        let raw = r#"{
          "scores": {"scene_goal_clear": 11},
          "weakest_axis": "scene_goal_clear",
          "specific_edits": [],
          "overall_one_liner": "Goal is unclear."
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("outside 1-10"));
    }

    #[test]
    fn parse_rejects_weakest_axis_not_in_scores() {
        let raw = r#"{
          "scores": {"scene_goal_clear": 5},
          "weakest_axis": "missing",
          "specific_edits": [],
          "overall_one_liner": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not in the scores map"));
    }

    #[test]
    fn parse_accepts_well_formed_critique() {
        let raw = r#"{
          "scores": {"scene_goal_clear": 4, "rising_tension": 6, "hook_ending": 3},
          "weakest_axis": "hook_ending",
          "specific_edits": [
            {"problem_quote": "She closed the door.", "fix": "She closed the door, and the lock clicked from outside.", "axis": "hook_ending"}
          ],
          "overall_one_liner": "Goal lands but the scene-end falls flat."
        }"#;
        assert!(parse_and_validate(raw).is_ok());
    }

    #[test]
    fn parse_repairs_string_in_specific_edits() {
        let raw = r#"{
          "scores": {"scene_goal_clear": 5},
          "weakest_axis": "scene_goal_clear",
          "specific_edits": [
            {"problem_quote": "x", "fix": "y", "axis": "scene_goal_clear"},
            "edit_2_placeholder"
          ],
          "overall_one_liner": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "json_repair should salvage; got {res:?}");
    }
}
