//! Agent workflow orchestrator (Layer 4 — infrastructure).
//!
//! The Orchestrator runs bounded agent swarms: it enforces the hard caps from
//! ARCHITECTURE.md §5.3 (≤8 calls, ≤10 min, ≤200 k tokens, ≤3 retries).
//!
//! Agents are selected by the orchestrator based on the workflow trigger;
//! no user interaction is required unless a `UserGate::Required` agent is
//! invoked (in that case the orchestrator suspends and emits a `RunEvent`).

#![forbid(unsafe_code)]
// BACKLOG §C4: tests freely use `.unwrap()` / `.expect()` against canned
// fixtures; the workspace-level clippy lints fire only on shipped code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod apply;
pub mod apply_continuity;
pub mod apply_copyedit;
pub mod config;
pub mod context_builder;
pub mod council;
pub mod crash_capture;
pub mod cross_cutting;
pub mod originality_provider;
pub mod proposal_validator;
pub mod prompt_guard;
pub mod quick_action;
pub mod run;
pub mod runner;
pub mod voice_pipeline;
pub mod event;

pub use apply::ApplyOutlineResult;
pub use apply_continuity::ApplyContinuityResult;
pub use apply_copyedit::ApplyCopyeditResult;
pub use config::OrchestratorConfig;
pub use quick_action::{ApplyOp, ApplyQuickActionResult, QuickActionOptions, QuickActionOutcome};
pub use run::{Orchestrator, OutlineRunResult, RunHandle, WorkflowTrigger};
pub use event::RunEvent;

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("workflow aborted: agent call limit ({limit}) exceeded")]
    AgentCallLimitExceeded { limit: u32 },

    #[error("workflow aborted: time limit ({limit_secs}s) exceeded")]
    TimeLimitExceeded { limit_secs: u64 },

    #[error("workflow aborted: token budget ({limit}) exceeded")]
    TokenBudgetExceeded { limit: u32 },

    #[error("agent '{agent_id}' failed after {retries} retries: {reason}")]
    AgentFailed { agent_id: String, retries: u32, reason: String },

    #[error("workflow cancelled by user")]
    Cancelled,

    #[error("Ollama unavailable: {0}")]
    OllamaUnavailable(String),

    #[error("storage error: {0}")]
    Storage(String),

    /// MZ-07: outline-to-tree validation failure.
    #[error("outline apply error: {0}")]
    OutlineApply(String),

    /// MZ-07: idempotency guard — re-applying a task that already has
    /// `agent_applied_edits` rows is refused so we never double-create the
    /// document tree.
    #[error("outline already applied for task {task_id}")]
    AlreadyApplied { task_id: ulid::Ulid },
}
