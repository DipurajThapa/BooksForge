//! Scene Planner Agent (Item 4 of FEATURE_HARDENING_PLAN).
//!
//! Reads `VoiceScore.failed_dimensions` + the tells report from a
//! fresh draft and emits a [`booksforge_domain::PolishPlan`] — an
//! ordered list of polish stages to invoke with TARGETED instructions
//! rather than the static genre-pack polish-stack order.
//!
//! ### Why
//!
//! The genre pack ran every polish stage in a fixed order, with each
//! stage's `should_run` detector offering a coarse skip/run gate.
//! Stages that ran did so on instinct — `polish:voice` ran the same
//! cadence-matching prompt regardless of whether the gap was anaphora
//! collapse, missing medium-band sentences, or em-dash overuse.
//!
//! The planner closes the loop. It reads the *specific* signals the
//! draft produced and tells each polish stage exactly what to fix:
//!
//!   - `voice_score.failed_dimensions = ["sentence_length_bucket[1]
//!      (9-17w) actual 0.00 not in [0.20,0.50]"]` →
//!     plan emits `polish:voice` with instruction
//!     "the medium sentence-length band (9-17 words) is empty;
//!      convert ~25% of the short sentences into mid-length
//!      compound sentences."
//!
//!   - `tells_report.by_category = {"structural:no_concrete_noun": 1}`
//!     → plan emits `polish:metaphor` with instruction
//!     "paragraph 3 has zero concrete sensory nouns; insert
//!      tactile / olfactory anchoring on every other sentence."
//!
//! ### What this agent does NOT do
//!
//! It does not REWRITE prose — it only chooses which polish stages
//! to run and what to ask them. The polish stages themselves remain
//! responsible for the actual rewrites.

use booksforge_domain::PolishPlan;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "stage-id-unknown",
        description: "An entry's stage_id is not in the known polish stage list.",
        recoverable: true,
    },
    FailureMode {
        id: "instruction-empty",
        description: "An entry's instruction is empty — defeats the purpose of the plan.",
        recoverable: true,
    },
    FailureMode {
        id: "severity-out-of-range",
        description: "An entry's severity is outside 1..=3.",
        recoverable: true,
    },
    FailureMode {
        id: "rationale-too-long",
        description: "rationale > 100 words — forces specificity.",
        recoverable: true,
    },
    FailureMode {
        id: "plan-bloat",
        description: "Plan includes stages without specific signal evidence in the inputs.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "scene-planner",
        name:             "Adaptive Polish Planner",
        purpose:          "Read VoiceScore.failed_dimensions + the tells report from a fresh scene draft and emit a PolishPlan: which polish stages to run, in what order, with what targeted instructions. Replaces the static genre-pack polish-stack order with a signal-driven DAG.",
        input_schema_id:  "ScenePlannerInput",
        output_schema_id: "PolishPlan",
        prompt_template:  PromptTemplateId::new("scene-planner", "v1"),
        model_preference: ModelPreference {
            // Planner output is small + structured. Medium model is fine —
            // the input signals do most of the work; the planner just maps
            // them to stage selections and instructions.
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Inputs: VoiceScore JSON (~1k), TellsReport JSON (~1k),
            // scene_card (~500 tokens), prose excerpt (~600 chars).
            // Output: PolishPlan with up to 4 entries × ~100 tokens each.
            max_context_tokens: 8_000,
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
        // Planning benefits from thinking-mode — multi-stage selection
        // with severity reasoning is exactly the kind of compact
        // structured analysis small thinking budgets help with.
        default_thinking: DefaultThinking::Enabled,
    }
}

/// Parse the model's raw output into a `PolishPlan` and run its
/// `validate()`. Uses schema-aware repair so field-name typos in
/// the planner output get healed before deserialise.
pub fn parse_and_validate(raw: &str) -> Result<PolishPlan, String> {
    const ALLOWED_FIELDS: &[&str] = &[
        "entries",
        "rationale",
        "stage_id",
        "reason",
        "instruction",
        "severity",
    ];
    let (repaired, audit) =
        crate::json_repair::parse_and_repair_with_schema_keys(raw, ALLOWED_FIELDS)
            .map_err(|e| format!("JSON parse error: {e}"))?;
    if !audit.field_renames.is_empty() {
        tracing::warn!(
            agent   = "scene-planner",
            renames = ?audit.field_renames,
            "schema-aware repair healed {} field-name typo(s) before deserialise",
            audit.field_renames.len(),
        );
    }
    let plan: PolishPlan =
        serde_json::from_value(repaired).map_err(|e| format!("PolishPlan parse error: {e}"))?;
    let errs = plan.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "scene-planner");
        assert_eq!(s.input_schema_id, "ScenePlannerInput");
        assert_eq!(s.output_schema_id, "PolishPlan");
        assert!(matches!(s.user_gate, UserGate::Required));
        assert!(matches!(s.default_thinking, DefaultThinking::Enabled));
        assert_eq!(s.failure_modes.len(), 5);
    }

    #[test]
    fn parse_accepts_well_formed_plan() {
        let raw = r#"{
            "entries": [
                {
                    "stage_id":    "polish:voice",
                    "reason":      "median sentence length 4 < target 7",
                    "instruction": "convert 25% of short sentences into 9-17 word compounds",
                    "severity":    3
                },
                {
                    "stage_id":    "polish:metaphor",
                    "reason":      "paragraph 3 has zero concrete sensory nouns",
                    "instruction": "anchor in tactile / olfactory detail every other sentence",
                    "severity":    2
                }
            ],
            "rationale": "Two gaps: bimodal distribution (no medium band) and one abstract paragraph. Voice first because higher severity."
        }"#;
        let r = parse_and_validate(raw);
        assert!(r.is_ok(), "expected ok, got {r:?}");
        let plan = r.unwrap();
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.entries[0].stage_id, "polish:voice");
    }

    #[test]
    fn parse_accepts_empty_plan() {
        // Empty entries list = "no polish needed" — the correct answer
        // when the draft passes every signal.
        let raw = r#"{
            "entries": [],
            "rationale": "Draft passes all dimensions; no polish needed."
        }"#;
        let r = parse_and_validate(raw);
        assert!(r.is_ok(), "expected ok, got {r:?}");
        assert!(r.unwrap().entries.is_empty());
    }

    #[test]
    fn parse_rejects_bad_stage_id_prefix() {
        let raw = r#"{
            "entries": [
                {
                    "stage_id":    "voice",
                    "reason":      "x",
                    "instruction": "y",
                    "severity":    1
                }
            ],
            "rationale": "x"
        }"#;
        let r = parse_and_validate(raw);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("must start with 'polish:'"));
    }

    #[test]
    fn parse_rejects_severity_out_of_range() {
        let raw = r#"{
            "entries": [
                {
                    "stage_id":    "polish:voice",
                    "reason":      "x",
                    "instruction": "y",
                    "severity":    5
                }
            ],
            "rationale": "x"
        }"#;
        let r = parse_and_validate(raw);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not in 1..=3"));
    }

    #[test]
    fn parse_self_heals_field_name_typo() {
        // Model emits `stage` instead of `stage_id` (distance 3 in
        // 8-char name = 0.375 normalized) — over the default cap.
        // But `severty` → `severity` is distance 1, normalized 0.125 — heals.
        let raw = r#"{
            "entries": [
                {
                    "stage_id":    "polish:voice",
                    "reason":      "x",
                    "instruction": "y",
                    "severty":     2
                }
            ],
            "rationale": "x"
        }"#;
        let r = parse_and_validate(raw);
        assert!(r.is_ok(), "severty should heal to severity, got {r:?}");
    }

    #[test]
    fn polish_plan_instructions_for_filters_by_stage() {
        let plan = PolishPlan {
            entries: vec![
                booksforge_domain::PolishPlanEntry {
                    stage_id: "polish:voice".into(),
                    reason: "x".into(),
                    instruction: "voice instruction".into(),
                    severity: 3,
                },
                booksforge_domain::PolishPlanEntry {
                    stage_id: "polish:metaphor".into(),
                    reason: "y".into(),
                    instruction: "metaphor instruction".into(),
                    severity: 2,
                },
            ],
            rationale: "x".into(),
        };
        let voice = plan.instructions_for("polish:voice");
        assert_eq!(voice.len(), 1);
        assert_eq!(voice[0].instruction, "voice instruction");
        let none = plan.instructions_for("polish:dialogue");
        assert_eq!(none.len(), 0);
    }
}
