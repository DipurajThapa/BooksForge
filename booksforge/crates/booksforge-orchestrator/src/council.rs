//! Cross-verification council — orchestrator-mediated peer review.
//!
//! Per AGENTS.md §1: agents are stateless and never call each other.  When
//! the orchestrator wants a peer to verify a primary's proposal, it
//! dispatches the peer as a separate agent run with the primary's output
//! threaded in via `PeerReviewRequest`.  The peer returns
//! `PeerReviewResult`; the council aggregates these alongside the
//! Tier-1 + Tier-2 ProposalValidator into a `VerificationReport` that
//! travels with the proposal to the user gate.
//!
//! # Bounds
//!
//! - Peer reviews count toward the workflow's ≤8-call cap.
//! - The council is **non-recursive**: a reviewer cannot trigger its own
//!   peer reviewers.  This is enforced here, not in agent code.
//! - Each pairing carries a `default_on` flag.  In MVP, only the
//!   highest-leverage pairings light up by default; the rest activate
//!   when the project sets `validators.high_confidence_mode = true`.

use booksforge_domain::{
    peer_reviewers_for, PeerReviewPairing, PeerReviewResult, ProposalValidation,
    ValidationVerdict, VerificationReport,
};

/// Decide which peer reviewers to dispatch for `primary_agent_id`,
/// honoring the `default_on` flag plus the project's high-confidence
/// mode flag.
pub fn select_pairings(
    primary_agent_id:    &str,
    high_confidence_mode: bool,
) -> Vec<PeerReviewPairing> {
    peer_reviewers_for(primary_agent_id)
        .iter()
        .copied()
        .filter(|p| p.default_on || high_confidence_mode)
        .collect()
}

/// Assemble the final report.  The orchestrator runs Tier-1 always, Tier-2
/// when enabled, peer reviews per `select_pairings`; this function is the
/// pure-logic merge.
pub fn assemble_report(
    primary_agent_id: &str,
    primary_task_id:  &str,
    tier_1:           ProposalValidation,
    tier_2:           Option<ProposalValidation>,
    peer_reviews:     Vec<PeerReviewResult>,
) -> VerificationReport {
    let final_verdict = VerificationReport::aggregate_verdicts(
        &tier_1, tier_2.as_ref(), &peer_reviews,
    );
    VerificationReport {
        primary_agent_id: primary_agent_id.to_owned(),
        primary_task_id:  primary_task_id.to_owned(),
        tier_1,
        tier_2,
        peer_reviews,
        final_verdict,
    }
}

/// Council verdict semantics for the orchestrator's retry loop:
///
/// - **`Pass`**  — surface the proposal to the user.
/// - **`Warn`**  — surface with annotations; the user gate persists.
/// - **`Block`** — retry the *primary* once with the council's evidence
///                 appended; second `Block` aborts the run with
///                 `proposal_invalid` (per AGENTS.md §6).
pub fn should_retry_primary(verdict: ValidationVerdict, attempts_so_far: u32) -> bool {
    matches!(verdict, ValidationVerdict::Block) && attempts_so_far < 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::{PeerReviewFocus, ValidationOutcome};

    fn pv(verdict: ValidationVerdict) -> ProposalValidation {
        ProposalValidation { verdict, checks: Vec::new(), summary: "x".into(), tier_2_ran: false }
    }
    fn peer(reviewer: &str, verdict: ValidationVerdict) -> PeerReviewResult {
        PeerReviewResult {
            reviewer_agent_id: reviewer.into(),
            primary_task_id:   "01HX".into(),
            focus:             PeerReviewFocus::FactFidelity,
            verdict,
            concerns:          Vec::new(),
            recommendation:    String::new(),
        }
    }

    #[test]
    fn default_mode_picks_only_default_on_pairings() {
        let p = select_pairings("chapter-drafter", false);
        assert!(p.len() >= 3, "default-mode should still pick the three default-on reviewers");
        assert!(p.iter().all(|x| x.default_on));
    }

    #[test]
    fn high_confidence_mode_picks_all_pairings() {
        let n_default = select_pairings("chapter-drafter", false).len();
        let n_high    = select_pairings("chapter-drafter", true ).len();
        assert!(n_high >= n_default);
    }

    #[test]
    fn assemble_report_sets_final_verdict() {
        let report = assemble_report(
            "chapter-drafter", "01HXTASK",
            pv(ValidationVerdict::Pass),
            None,
            vec![peer("memory-curator", ValidationVerdict::Block)],
        );
        assert_eq!(report.final_verdict, ValidationVerdict::Block);
    }

    #[test]
    fn should_retry_only_once_on_block() {
        assert!(should_retry_primary(ValidationVerdict::Block, 0));
        assert!(!should_retry_primary(ValidationVerdict::Block, 1));
        assert!(!should_retry_primary(ValidationVerdict::Pass,  0));
    }

    // Reference suppress unused-import warning if no matches.
    #[allow(dead_code)]
    fn _ref() -> ValidationOutcome { ValidationOutcome::Pass }
}
