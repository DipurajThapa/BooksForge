//! IPC types for per-agent run commands (Phase 5 close-out).
//!
//! Each agent's `agent_run_<id>` command returns a typed result that
//! carries:
//!   - the typed proposal (as JSON for portability — the UI is TS-typed),
//!   - the multi-tier `VerificationReport` (Tier-1 + optional Tier-2 +
//!     peer reviews + final verdict),
//!   - run metadata (run_id, task_id, status, raw output for debug).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One axis of validation report shaped for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidationCheckDto {
    /// "schema" | "redaction" | "length" | "entity_sanity" | "memory_scope" |
    /// "idempotent" | "range" | "contract" |
    /// "faithfulness" | "style" | "coherence" | "self_consistency"
    pub axis: String,
    /// "pass" | "warn" | "fail"
    pub outcome: String,
    pub evidence: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProposalValidationDto {
    /// "pass" | "warn" | "block"
    pub verdict: String,
    pub checks: Vec<ValidationCheckDto>,
    pub summary: String,
    pub tier_2_ran: bool,
}

/// Peer-reviewer concern shaped for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PeerReviewConcernDto {
    /// "info" | "warning" | "error"
    pub severity: String,
    pub quote: String,
    pub reason: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PeerReviewResultDto {
    pub reviewer_agent_id: String,
    pub primary_task_id: String,
    /// "fact_fidelity" | "voice_preservation" | "ai_tell_residue" |
    /// "name_pov_preservation" | "structural_purpose" | "memory_consistency" |
    /// "emotional_clarity"
    pub focus: String,
    /// "pass" | "warn" | "block"
    pub verdict: String,
    pub concerns: Vec<PeerReviewConcernDto>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VerificationReportDto {
    pub primary_agent_id: String,
    pub primary_task_id: String,
    pub tier_1: ProposalValidationDto,
    pub tier_2: Option<ProposalValidationDto>,
    pub peer_reviews: Vec<PeerReviewResultDto>,
    /// "pass" | "warn" | "block"
    pub final_verdict: String,
}

/// Generic agent-run result shape used by the per-agent commands.  The
/// `proposal_json` carries the typed output (e.g. `CopyeditProposals`,
/// `ContinuityReport`) as a JSON string so the TypeScript side can
/// `JSON.parse` it into its own typed view.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentRunResultDto {
    pub run_id: String,
    pub task_id: String,
    /// "completed" | "invalid" | "error" | "cancelled"
    pub status: String,
    pub agent_id: String,
    pub proposal_json: Option<String>,
    pub verification: VerificationReportDto,
    pub error: Option<String>,
    pub raw_output: Option<String>,
}

// ── Per-agent inputs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunCopyeditInput {
    pub project_id: String,
    pub node_id: String,
    pub model: String,
    /// When true, the orchestrator dispatches the council's high-confidence
    /// peer reviewers in addition to the always-on default-on pairings.
    pub high_confidence_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunContinuityInput {
    pub project_id: String,
    pub node_id: String,
    pub model: String,
    pub project_pov: Option<String>,
    pub prior_summary: Option<String>,
    pub high_confidence_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunIntakeInput {
    pub project_id: String,
    pub idea_text: String,
    pub preferred_mode: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunMemoryCuratorInput {
    pub project_id: String,
    /// "book" | "chapter" | "entity"
    pub scope: String,
    pub node_id: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunVocabDictionaryInput {
    pub project_id: String,
    pub model: String,
    /// How many recent edit decisions to feed in.  Default 30.
    pub lookback: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunChapterDrafterInput {
    pub project_id: String,
    pub node_id: String,
    pub scene_synopsis: String,
    pub chapter_purpose: String,
    pub project_pov: String,
    pub target_words: u32,
    pub model: String,
    pub genre: Option<String>,
    pub tone: Option<String>,
    /// Opt into the council's non-default-on peer reviewers.  See AGENTS.md §6.5.
    pub high_confidence_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunDevEditorInput {
    pub project_id: String,
    pub chapter_id: String,
    pub model: String,
    pub high_confidence_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunHumanizationInput {
    pub project_id: String,
    pub node_id: String,
    pub model: String,
    pub high_confidence_mode: Option<bool>,
}

/// Tier-2 ProposalValidator input — caller supplies the primary agent's
/// output and the orchestrator enriches with context.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunProposalValidatorInput {
    pub project_id: String,
    pub primary_agent_id: String,
    /// JSON-stringified primary agent output (what the user is reviewing).
    pub primary_output_json: String,
    pub context_excerpt: String,
    /// JSON-stringified Tier-1 ProposalValidation (already computed by the
    /// orchestrator on the primary call).
    pub tier_1_findings_json: String,
    pub model: String,
}
