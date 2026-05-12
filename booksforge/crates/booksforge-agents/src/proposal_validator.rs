//! Proposal Validator Agent — 360° review of another agent's proposal.
//!
//! This is the Tier-2 (LLM-backed) half of the validation system.  Tier 1
//! (deterministic, pure Rust) lives in `booksforge_orchestrator::cross_cutting`
//! and runs always.  Tier 2 runs after Tier 1 passes, opt-in per project,
//! and adds four context-fitness axes the deterministic checks can't see:
//!
//!   - **Faithfulness**      — proposal reflects source text accurately
//!   - **Style**             — proposal respects the StyleBook + active vocabulary
//!   - **Coherence**         — proposal does not contradict project memory
//!   - **SelfConsistency**   — proposal items don't contradict each other
//!
//! The validator is not in the user-facing agent catalog (per AGENTS.md §2:
//! "MVP runs nine LLM agents" — this one is internal/orchestrator-grade).
//! It cannot mutate state.  It produces a `ProposalValidation` report; the
//! orchestrator decides whether to surface, retry, or block based on the
//! verdict.

use booksforge_domain::ProposalValidation;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "verdict-out-of-enum",
        description: "Verdict not in pass/warn/block.",
        recoverable: true,
    },
    FailureMode {
        id: "axis-out-of-enum",
        description: "Check axis not in the four T2 axes.",
        recoverable: true,
    },
    FailureMode {
        id: "no-evidence",
        description: "Check has no evidence string.",
        recoverable: true,
    },
    FailureMode {
        id: "loops-back-to-self",
        description: "Validator hallucinates that itself is wrong.",
        recoverable: false,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id: "proposal-validator",
        name: "Proposal Validator",
        purpose: "360° review of another agent's proposal across four context-fitness axes.",
        input_schema_id: "ProposalValidationInput",
        output_schema_id: "ProposalValidation",
        prompt_template: PromptTemplateId::new("proposal-validator", "v1"),
        model_preference: ModelPreference {
            // Cheap, fast — this gate runs on every primary agent call.
            family: ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Small,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Holds the primary agent's output + relevant context excerpt.
            max_context_tokens: 6_000,
            max_output_tokens: 1_500,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        // Internal agent — runs automatically when Tier-2 is enabled.
        when_to_run: WhenToRun::Automatic,
        user_gate: UserGate::NotRequired,
        default_thinking: DefaultThinking::Enabled,
    }
}

pub fn parse_and_validate(raw: &str) -> Result<ProposalValidation, String> {
    let parsed: ProposalValidation =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    // Validation: tier_2_ran should be true; verdict aggregation must
    // match the per-axis outcomes.
    if !parsed.tier_2_ran {
        return Err("Tier-2 validator returned tier_2_ran=false".to_owned());
    }
    let recomputed = ProposalValidation::verdict_from_checks(&parsed.checks);
    if recomputed != parsed.verdict {
        return Err(format!(
            "verdict mismatch: model said {:?} but checks aggregate to {:?}",
            parsed.verdict, recomputed
        ));
    }
    Ok(parsed)
}
