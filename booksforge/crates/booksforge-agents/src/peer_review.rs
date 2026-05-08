//! Peer Review Agent — single agent module that any registered agent can
//! impersonate when invoked in peer-review mode.
//!
//! Per AGENTS.md §1, agents are stateless prompt-in / schema-out.  Peer
//! review is a *role* the orchestrator assigns by routing a primary agent's
//! output through this module's spec + the shared `peer-review/v1.toml`
//! template.  The reviewer's identity (`memory-curator`, `continuity`, …)
//! is threaded through `vars["reviewer_agent_id"]` so the prompt knows who
//! it's playing.
//!
//! Output is locked to `PeerReviewResult` (see
//! `booksforge_domain::council`).  Verdict aggregation is enforced in
//! `parse_and_validate`: any concern with `Error` severity must escalate
//! the result-level verdict to `Block`; any `Warning` to at least `Warn`.

use booksforge_domain::{PeerConcernSeverity, PeerReviewResult, ValidationVerdict};
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, FailureMode, ModelFamily, ModelPreference,
    ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode { id: "verdict-out-of-enum",  description: "Verdict not in pass/warn/block.",                         recoverable: true  },
    FailureMode { id: "focus-out-of-enum",    description: "Focus axis not in the seven peer-review axes.",           recoverable: true  },
    FailureMode { id: "concern-without-quote", description: "Concern row missing the source quote.",                   recoverable: true  },
    FailureMode { id: "verdict-aggregate-mismatch", description: "Result verdict softer than concern severities imply.", recoverable: false },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "peer-review",
        name:             "Peer Review",
        purpose:          "Cross-agent verification on a single focus axis (fact_fidelity, voice_preservation, ai_tell_residue, name_pov_preservation, structural_purpose, memory_consistency, emotional_clarity).",
        input_schema_id:  "PeerReviewRequest",
        output_schema_id: "PeerReviewResult",
        prompt_template:  PromptTemplateId::new("peer-review", "v1"),
        model_preference: ModelPreference {
            // Same scale as Tier-2 ProposalValidator — runs alongside it.
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Small,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 6_000,
            max_output_tokens:  1_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::Automatic,
        user_gate:     UserGate::NotRequired,
    }
}

/// Parse a model response into `PeerReviewResult` and enforce that the
/// result-level verdict is at least as strict as the concern severities
/// (any `Error` → `Block`; any `Warning` → `Warn`-or-stricter).
// The `(verdict, implied)` arms below describe distinct logical
// cases (verdict consistent with implied severity); collapsing
// them via `|` would hide the contract.
#[allow(clippy::match_same_arms)]
pub fn parse_and_validate(raw: &str) -> Result<PeerReviewResult, String> {
    let parsed: PeerReviewResult = serde_json::from_str(raw)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    let mut implied = ValidationVerdict::Pass;
    for c in &parsed.concerns {
        match c.severity {
            PeerConcernSeverity::Error   => { implied = ValidationVerdict::Block; }
            PeerConcernSeverity::Warning => {
                if !matches!(implied, ValidationVerdict::Block) {
                    implied = ValidationVerdict::Warn;
                }
            }
            PeerConcernSeverity::Info    => {}
        }
    }
    let consistent = match (parsed.verdict, implied) {
        (ValidationVerdict::Block, _) => true,
        (ValidationVerdict::Warn,  ValidationVerdict::Block) => false,
        (ValidationVerdict::Warn,  _) => true,
        (ValidationVerdict::Pass,  ValidationVerdict::Pass) => true,
        (ValidationVerdict::Pass,  _) => false,
    };
    if !consistent {
        return Err(format!(
            "verdict {:?} is softer than concerns imply ({:?})",
            parsed.verdict, implied
        ));
    }
    if parsed.recommendation.split_whitespace().count() > 80 {
        return Err("recommendation > 80 words".to_owned());
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::{PeerReviewConcern, PeerReviewFocus};

    fn pr(verdict: ValidationVerdict, sev: PeerConcernSeverity) -> String {
        let p = PeerReviewResult {
            reviewer_agent_id: "memory-curator".into(),
            primary_task_id:   "01HX".into(),
            focus:             PeerReviewFocus::FactFidelity,
            verdict,
            concerns: vec![PeerReviewConcern {
                severity: sev,
                quote:    "x".into(),
                reason:   "y".into(),
                evidence: "z".into(),
            }],
            recommendation: "ok".into(),
        };
        serde_json::to_string(&p).unwrap()
    }

    #[test]
    fn rejects_pass_with_error_concern() {
        let raw = pr(ValidationVerdict::Pass, PeerConcernSeverity::Error);
        assert!(parse_and_validate(&raw).is_err());
    }

    #[test]
    fn accepts_block_with_error_concern() {
        let raw = pr(ValidationVerdict::Block, PeerConcernSeverity::Error);
        assert!(parse_and_validate(&raw).is_ok());
    }

    #[test]
    fn accepts_warn_with_warning_concern() {
        let raw = pr(ValidationVerdict::Warn, PeerConcernSeverity::Warning);
        assert!(parse_and_validate(&raw).is_ok());
    }

    #[test]
    fn accepts_pass_with_only_info_concerns() {
        let raw = pr(ValidationVerdict::Pass, PeerConcernSeverity::Info);
        assert!(parse_and_validate(&raw).is_ok());
    }
}
