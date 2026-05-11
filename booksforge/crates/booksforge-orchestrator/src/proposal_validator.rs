//! Phase 5 — ProposalValidator (Tier 1, deterministic).
//!
//! After every agent's primary parse, the orchestrator runs every
//! cross-cutting validator in the agent's `AgentSpec.validators` slice
//! against the raw model output and the parsed JSON.  The aggregate is
//! a `ProposalValidation` — `block`/`warn`/`pass` plus per-axis evidence.
//!
//! Tier 2 (LLM-backed) is wired separately via the `proposal-validator`
//! agent module and is opt-in per project.

use booksforge_agents::AgentSpec;
use booksforge_domain::{Entity, ProposalValidation, ValidationCheck, ValidationVerdict};

use crate::cross_cutting;

/// Run every cross-cutting validator declared on `spec` and return the
/// aggregate.  `parsed` should be the agent output deserialized into
/// `serde_json::Value` (the typed parse already validated structure;
/// this is the cross-cutting residual).
///
/// `source_text` and `prior_scene_corpus` power the `Originality`
/// validator.  Pass `None` when the agent has no source/corpus context
/// (e.g. intake) or when originality isn't in the spec's validators slice.
#[allow(clippy::too_many_arguments)]
pub fn run_tier1(
    spec: &AgentSpec,
    raw_output: &str,
    parsed: &serde_json::Value,
    entity_bible: &[Entity],
    proposed_memory_scopes: &[String],
    source_text: Option<&str>,
    prior_scene_corpus: Option<&str>,
) -> ProposalValidation {
    let mut checks: Vec<ValidationCheck> = Vec::with_capacity(spec.validators.len());
    for v in spec.validators {
        checks.push(cross_cutting::run_validator(
            *v,
            raw_output,
            parsed,
            entity_bible,
            spec.id,
            proposed_memory_scopes,
            source_text,
            prior_scene_corpus,
        ));
    }
    let verdict = ProposalValidation::verdict_from_checks(&checks);
    let summary = match verdict {
        ValidationVerdict::Pass => format!("All {} cross-cutting checks passed.", checks.len()),
        ValidationVerdict::Warn => {
            let n = checks
                .iter()
                .filter(|c| matches!(c.outcome, booksforge_domain::ValidationOutcome::Warn))
                .count();
            format!("{n} cross-cutting check(s) raised warnings — review before applying.")
        }
        ValidationVerdict::Block => {
            let n = checks
                .iter()
                .filter(|c| matches!(c.outcome, booksforge_domain::ValidationOutcome::Fail))
                .count();
            format!("{n} cross-cutting check(s) FAILED — proposal blocked.")
        }
    };
    ProposalValidation {
        verdict,
        checks,
        summary,
        tier_2_ran: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_agents::{
        AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, ModelFamily,
        ModelPreference, ModelSizeHint, UserGate, WhenToRun,
    };
    use booksforge_prompt::PromptTemplateId;

    fn dummy_spec() -> AgentSpec {
        AgentSpec {
            id: "copyeditor",
            name: "Copyeditor",
            purpose: "p",
            input_schema_id: "X",
            output_schema_id: "Y",
            prompt_template: PromptTemplateId::new("copyeditor", "v1"),
            model_preference: ModelPreference {
                family: ModelFamily::AnyInstruct,
                min_size: ModelSizeHint::Medium,
            },
            pinned_model: None,
            context_budget: ContextBudget {
                max_context_tokens: 4_000,
                max_output_tokens: 2_000,
            },
            validators: &[
                CrossCuttingValidator::Schema,
                CrossCuttingValidator::Redaction,
                CrossCuttingValidator::Length,
            ],
            failure_modes: &[],
            when_to_run: WhenToRun::OnDemand,
            user_gate: UserGate::Required,
            default_thinking: DefaultThinking::Disabled,
        }
    }

    #[test]
    fn passes_when_all_checks_clean() {
        let spec = dummy_spec();
        let parsed = serde_json::json!({"edits": [], "summary": "fine"});
        let raw = parsed.to_string();
        let out = run_tier1(&spec, &raw, &parsed, &[], &[], None, None);
        assert_eq!(out.verdict, ValidationVerdict::Pass);
    }

    #[test]
    fn blocks_on_empty_output() {
        let spec = dummy_spec();
        let parsed = serde_json::json!({});
        let out = run_tier1(&spec, "", &parsed, &[], &[], None, None);
        assert_eq!(out.verdict, ValidationVerdict::Block);
    }

    #[test]
    fn warns_on_chain_of_thought_leak() {
        let spec = dummy_spec();
        let parsed = serde_json::json!({"edits": []});
        let raw = "Let me think step by step about this. Output: {}";
        let out = run_tier1(&spec, raw, &parsed, &[], &[], None, None);
        assert!(matches!(
            out.verdict,
            ValidationVerdict::Warn | ValidationVerdict::Block
        ));
    }
}
