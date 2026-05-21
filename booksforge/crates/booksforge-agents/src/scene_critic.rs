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
        // 2026-05-15: flipped Enabled → Disabled.
        //
        // The original Enabled setting reasoned that multi-axis
        // craft critique benefits from thinking-mode. In practice
        // on qwen3.5:9b (LIGHT) and qwen3.6:latest (MoE), enabling
        // thinking-mode routes the entire critic output into
        // `message.thinking` and leaves `message.content` empty
        // — producing `EOF while parsing a value at line 1 column 0`
        // across all 3 retry attempts and gating the per-scene
        // polish stack from ever running.
        //
        // Same failure mode the comments on world-bible (Run #16)
        // and scene-drafter-fic call out: "explicit reasoning isn't
        // earning its budget cost on this prompt class." Disable
        // here too. The critique still works — the model just emits
        // its reasoning inline rather than separating it.
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse the model's raw output into a `SceneCritiqueProposal` and run
/// `validate()`. Uses the strict-objects json_repair on the
/// `specific_edits` list (which must be objects, not stray strings).
///
/// 2026-05-15: derive `weakest_axis` from `scores` when the model
/// omits it. Observed in qwen3.6:latest output — the critic emits a
/// well-formed `scores` map + `specific_edits` + `overall_one_liner`
/// but skips the explicit `weakest_axis` field. We can recover it
/// deterministically from the scores (the key with the minimum value)
/// rather than retry the call.
pub fn parse_and_validate(raw: &str) -> Result<SceneCritiqueProposal, String> {
    let (mut repaired, audit) =
        crate::json_repair::parse_and_repair_strict_objects(raw, &["specific_edits"])?;
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "scene-critic",
            dropped = audit.dropped_list_elements,
            "json_repair salvaged malformed list elements before deserialise",
        );
    }

    // Fill in optional-but-required fields the model intermittently
    // omits. Each fill is a deterministic recovery: we never invent
    // values that change the critique's meaning.
    if let serde_json::Value::Object(map) = &mut repaired {
        // weakest_axis ← min-scoring axis in `scores`.
        let has_axis = map
            .get("weakest_axis")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_axis {
            if let Some(serde_json::Value::Object(scores)) = map.get("scores") {
                let weakest = scores
                    .iter()
                    .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n)))
                    .min_by_key(|(_, n)| *n)
                    .map(|(k, _)| k);
                if let Some(name) = weakest {
                    tracing::warn!(
                        agent = "scene-critic",
                        derived_axis = %name,
                        "model omitted weakest_axis — derived from min-scoring axis",
                    );
                    map.insert("weakest_axis".to_owned(), serde_json::Value::String(name));
                }
            }
        }

        // specific_edits ← empty Vec when missing; filter out partial
        // edit objects when present. `TargetedEdit` requires three
        // fields (problem_quote, fix, axis); the model intermittently
        // emits objects missing one of them. Dropping the bad element
        // is better than rejecting the whole critique — the scores +
        // remaining valid edits are still useful.
        if !map.contains_key("specific_edits")
            || matches!(map.get("specific_edits"), Some(serde_json::Value::Null))
        {
            tracing::warn!(
                agent = "scene-critic",
                "model omitted specific_edits — defaulting to empty list",
            );
            map.insert(
                "specific_edits".to_owned(),
                serde_json::Value::Array(Vec::new()),
            );
        } else if let Some(serde_json::Value::Array(edits)) = map.get_mut("specific_edits") {
            let before = edits.len();
            edits.retain(|e| {
                if let serde_json::Value::Object(eo) = e {
                    let has_quote = eo
                        .get("problem_quote")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    let has_fix = eo
                        .get("fix")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    let has_axis = eo
                        .get("axis")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    has_quote && has_fix && has_axis
                } else {
                    false
                }
            });
            let dropped = before - edits.len();
            if dropped > 0 {
                tracing::warn!(
                    agent = "scene-critic",
                    dropped,
                    "filtered specific_edits missing problem_quote / fix / axis",
                );
            }
        }

        // overall_one_liner ← placeholder if missing. The scores +
        // edits carry the actionable information; the one-liner is
        // a UI nicety we can synthesise.
        let has_one_liner = map
            .get("overall_one_liner")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_one_liner {
            tracing::warn!(
                agent = "scene-critic",
                "model omitted overall_one_liner — defaulting",
            );
            map.insert(
                "overall_one_liner".to_owned(),
                serde_json::Value::String(
                    "(critic emitted scores without a one-line summary)".to_owned(),
                ),
            );
        }
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
        // 2026-05-15: flipped to Disabled — see comment in spec().
        assert!(matches!(s.default_thinking, DefaultThinking::Disabled));
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
