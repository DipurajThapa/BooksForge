//! Cross-verification council types — orchestrator-mediated agent peer review.
//!
//! Per AGENTS.md §1: agents are stateless prompt-in/schema-out units.  They
//! do **not** call each other directly.  When two agents need to "talk",
//! the orchestrator dispatches each as a separate run and threads their
//! outputs through these typed messages.  The audit trail (`agent_runs`
//! rows + `parent_task_id`) makes every cross-verification fully traceable.
//!
//! Three roles in the protocol:
//!
//! - **Primary** — the agent whose proposal is under review.  Always one.
//! - **Reviewers** — zero or more peer agents the orchestrator dispatches
//!   to verify the primary's proposal from their own perspective
//!   (e.g. Continuity reviews a Copyedit proposal for accidental name
//!   changes; Memory-Curator reviews a Chapter-Drafter proposal against
//!   established memory).
//! - **Council** — the deterministic aggregator (in `booksforge-orchestrator`)
//!   that decides who reviews whom, bounds depth, and merges verdicts.
//!
//! All cross-reviews count toward the workflow's ≤8-call cap.  The council
//! never recurses: a reviewer cannot trigger its own peer reviews.

use serde::{Deserialize, Serialize};

use crate::agent_io::{ProposalValidation, ValidationVerdict};

// ──────────────────────────────────────────────────────────────────────────────
// Pairing rules (who reviews whom, on which axis)
// ──────────────────────────────────────────────────────────────────────────────

/// Which axis a peer reviewer should focus on when reviewing a primary
/// agent's proposal.  Different from the `ValidationAxis` taxonomy used
/// for the Tier-1/Tier-2 ProposalValidator — that's general; this is
/// peer-specific (e.g. "Continuity should check copyedits for *name
/// preservation*", not generic "coherence").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerReviewFocus {
    /// "Did this prose-emitting agent invent or contradict facts in the bible?"
    FactFidelity,
    /// "Did this edit preserve the project's voice/tone fingerprint?"
    VoicePreservation,
    /// "Did this prose introduce or strengthen AI-tells?"
    AiTellResidue,
    /// "Did this edit accidentally change a character name or POV?"
    NamePovPreservation,
    /// "Did this draft serve the chapter's structural purpose?"
    StructuralPurpose,
    /// "Did this proposal contradict prior chapter memory?"
    MemoryConsistency,
    /// "Are emotional beats and stakes still legible after this change?"
    EmotionalClarity,
}

/// Static pairing table: which peers should review each primary, and on
/// which focus.  Encoded here so the orchestrator can ask "who should
/// review a `copyeditor` proposal?" without per-call branching.
#[derive(Debug, Clone, Copy)]
pub struct PeerReviewPairing {
    pub reviewer_agent_id: &'static str,
    pub focus: PeerReviewFocus,
    /// `true` = always run for high-stakes work.  `false` = opt-in
    /// behind a project-level flag.
    pub default_on: bool,
}

/// AGENTS.md §6.5 (added in Phase 5): the canonical peer-review pairings
/// for each primary agent.  An empty slice means the agent has no peer
/// reviewers — Tier-1 + Tier-2 ProposalValidator are sufficient.
///
/// Conservative by default: only the highest-leverage pairings are
/// `default_on: true`.  The rest light up when the user opts in to
/// "high-confidence mode" for a critical pass.
pub fn peer_reviewers_for(primary_agent_id: &str) -> &'static [PeerReviewPairing] {
    match primary_agent_id {
        // Outline-architect: verify it didn't invent characters absent from the brief.
        "outline-architect" => &[PeerReviewPairing {
            reviewer_agent_id: "memory-curator",
            focus: PeerReviewFocus::FactFidelity,
            default_on: false,
        }],
        // Chapter-drafter: highest-stakes prose creation.  Three reviewers default-on.
        "chapter-drafter" => &[
            PeerReviewPairing {
                reviewer_agent_id: "memory-curator",
                focus: PeerReviewFocus::MemoryConsistency,
                default_on: true,
            },
            PeerReviewPairing {
                reviewer_agent_id: "continuity",
                focus: PeerReviewFocus::NamePovPreservation,
                default_on: true,
            },
            PeerReviewPairing {
                reviewer_agent_id: "humanization",
                focus: PeerReviewFocus::AiTellResidue,
                default_on: true,
            },
            PeerReviewPairing {
                reviewer_agent_id: "dev-editor",
                focus: PeerReviewFocus::StructuralPurpose,
                default_on: false,
            },
        ],
        // Copyeditor: small mechanical fixes; verify nothing structural changed.
        "copyeditor" => &[PeerReviewPairing {
            reviewer_agent_id: "continuity",
            focus: PeerReviewFocus::NamePovPreservation,
            default_on: true,
        }],
        // Humanization: rewriting prose to remove AI-tells; verify it didn't break facts/voice.
        "humanization" => &[
            PeerReviewPairing {
                reviewer_agent_id: "memory-curator",
                focus: PeerReviewFocus::FactFidelity,
                default_on: true,
            },
            PeerReviewPairing {
                reviewer_agent_id: "final-review-editor",
                focus: PeerReviewFocus::VoicePreservation,
                default_on: false,
            },
        ],
        // Continuity: rename/annotate proposals; verify they don't contradict memory.
        "continuity" => &[PeerReviewPairing {
            reviewer_agent_id: "memory-curator",
            focus: PeerReviewFocus::MemoryConsistency,
            default_on: true,
        }],
        // Dev-editor: structural notes; verify they're consistent with established memory.
        "dev-editor" => &[PeerReviewPairing {
            reviewer_agent_id: "memory-curator",
            focus: PeerReviewFocus::MemoryConsistency,
            default_on: false,
        }],
        // Final-review-editor: world-class polish; verify it didn't strip humanity.
        "final-review-editor" => &[
            PeerReviewPairing {
                reviewer_agent_id: "humanization",
                focus: PeerReviewFocus::AiTellResidue,
                default_on: true,
            },
            PeerReviewPairing {
                reviewer_agent_id: "memory-curator",
                focus: PeerReviewFocus::FactFidelity,
                default_on: true,
            },
            PeerReviewPairing {
                reviewer_agent_id: "continuity",
                focus: PeerReviewFocus::NamePovPreservation,
                default_on: false,
            },
        ],
        // Memory-curator + vocab-dictionary: meta-agents that mutate memory/vocab.
        // No peer reviewers in MVP — Tier-1 ProposalValidator + MemoryScope check is enough.
        _ => &[],
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Wire types
// ──────────────────────────────────────────────────────────────────────────────

/// What the orchestrator sends to a peer reviewer.  The reviewer runs as a
/// normal agent invocation — same JSON-out contract — but the prompt
/// template is `<agent_id>-peer-review/v1.toml` and the output schema
/// is locked to `PeerReviewResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReviewRequest {
    /// The primary agent whose work is being reviewed.
    pub primary_agent_id: String,
    /// The primary's `agent_tasks.id` (so the audit trail can stitch them).
    pub primary_task_id: String,
    /// JSON-serialised primary output.  The reviewer should not mutate it.
    pub primary_output: serde_json::Value,
    /// What to focus on (drives which axes the reviewer's prompt asks about).
    pub focus: PeerReviewFocus,
    /// Excerpt of the source text or context the primary saw, so the
    /// reviewer can verify against the same ground truth.
    pub context_excerpt: String,
}

/// What a peer reviewer returns.  Same shape regardless of which agent
/// is reviewing — the orchestrator merges verdicts uniformly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReviewResult {
    pub reviewer_agent_id: String,
    pub primary_task_id: String,
    pub focus: PeerReviewFocus,
    /// `pass` / `warn` / `block`.  Verdict aggregation: any `block` from
    /// any reviewer escalates the council verdict to `block`.
    pub verdict: ValidationVerdict,
    /// Specific concerns the reviewer raised.  Each concern carries
    /// evidence (a quoted span, a referenced memory key, etc.).
    pub concerns: Vec<PeerReviewConcern>,
    /// Reviewer's freeform recommendation (≤80 words).
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReviewConcern {
    /// Severity of this single concern (the result-level verdict is the
    /// per-reviewer aggregate).
    pub severity: PeerConcernSeverity,
    /// Quote from the primary's output that triggered the concern.
    pub quote: String,
    /// Reviewer's reasoning.
    pub reason: String,
    /// Evidence: a memory key, an entity id, a vocab term, a prior
    /// chapter reference, etc.  Free-form so future reviewers can
    /// reference whatever they need.
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PeerConcernSeverity {
    Info,
    Warning,
    Error,
}

// ──────────────────────────────────────────────────────────────────────────────
// Aggregated verification report
// ──────────────────────────────────────────────────────────────────────────────

/// What the orchestrator produces after running Tier-1 + (opt-in) Tier-2
/// + (opt-in) peer reviews.  This travels with the primary's proposal
/// to the user gate so the writer can see *who validated what*.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Primary agent + task.
    pub primary_agent_id: String,
    pub primary_task_id: String,
    /// Tier-1 always ran.
    pub tier_1: ProposalValidation,
    /// Tier-2 ran iff `validators.tier_2_enabled`.
    pub tier_2: Option<ProposalValidation>,
    /// Peer reviews — zero or more, depending on `peer_reviewers_for`
    /// and whether the user opted in to high-confidence mode.
    pub peer_reviews: Vec<PeerReviewResult>,
    /// The council's final verdict, computed by `aggregate_verdicts`.
    pub final_verdict: ValidationVerdict,
}

impl VerificationReport {
    /// Aggregate all verdicts: any `Block` → `Block`; any `Warn` → `Warn`;
    /// otherwise `Pass`.  Mirrors `ProposalValidation::verdict_from_checks`.
    pub fn aggregate_verdicts(
        tier_1: &ProposalValidation,
        tier_2: Option<&ProposalValidation>,
        peers: &[PeerReviewResult],
    ) -> ValidationVerdict {
        let mut verdicts: Vec<ValidationVerdict> = vec![tier_1.verdict];
        if let Some(t2) = tier_2 {
            verdicts.push(t2.verdict);
        }
        verdicts.extend(peers.iter().map(|p| p.verdict));
        if verdicts
            .iter()
            .any(|v| matches!(v, ValidationVerdict::Block))
        {
            ValidationVerdict::Block
        } else if verdicts
            .iter()
            .any(|v| matches!(v, ValidationVerdict::Warn))
        {
            ValidationVerdict::Warn
        } else {
            ValidationVerdict::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_io::{ValidationOutcome, ValidationVerdict};

    fn pv(verdict: ValidationVerdict) -> ProposalValidation {
        ProposalValidation {
            verdict,
            checks: Vec::new(),
            summary: "x".into(),
            tier_2_ran: false,
        }
    }
    fn peer(verdict: ValidationVerdict) -> PeerReviewResult {
        PeerReviewResult {
            reviewer_agent_id: "memory-curator".into(),
            primary_task_id: "01HX".into(),
            focus: PeerReviewFocus::FactFidelity,
            verdict,
            concerns: Vec::new(),
            recommendation: String::new(),
        }
    }

    #[test]
    fn aggregate_pass_when_all_pass() {
        let v = VerificationReport::aggregate_verdicts(&pv(ValidationVerdict::Pass), None, &[]);
        assert_eq!(v, ValidationVerdict::Pass);
    }

    #[test]
    fn aggregate_block_when_any_peer_blocks() {
        let v = VerificationReport::aggregate_verdicts(
            &pv(ValidationVerdict::Pass),
            Some(&pv(ValidationVerdict::Pass)),
            &[peer(ValidationVerdict::Block)],
        );
        assert_eq!(v, ValidationVerdict::Block);
    }

    #[test]
    fn aggregate_warn_when_warn_no_block() {
        let v = VerificationReport::aggregate_verdicts(
            &pv(ValidationVerdict::Pass),
            Some(&pv(ValidationVerdict::Warn)),
            &[peer(ValidationVerdict::Pass)],
        );
        assert_eq!(v, ValidationVerdict::Warn);
    }

    #[test]
    fn pairings_for_chapter_drafter_default_on_three() {
        let pairings = peer_reviewers_for("chapter-drafter");
        let on = pairings.iter().filter(|p| p.default_on).count();
        assert_eq!(on, 3, "chapter-drafter should default-on three reviewers");
    }

    #[test]
    fn unknown_agent_has_no_peer_reviewers() {
        assert!(peer_reviewers_for("does-not-exist").is_empty());
    }

    #[test]
    fn copyeditor_paired_with_continuity() {
        let pairings = peer_reviewers_for("copyeditor");
        assert!(pairings.iter().any(|p| p.reviewer_agent_id == "continuity"
            && matches!(p.focus, PeerReviewFocus::NamePovPreservation)));
    }

    // Reference one of the imports so it isn't flagged unused if removed later.
    #[allow(dead_code)]
    fn _ref_outcome() -> ValidationOutcome {
        ValidationOutcome::Pass
    }
}
