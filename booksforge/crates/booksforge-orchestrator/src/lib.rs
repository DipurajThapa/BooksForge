//! Agent workflow orchestrator (Layer 4 — infrastructure).
//!
//! The Orchestrator runs bounded agent swarms: it enforces the hard caps from
//! ARCHITECTURE.md §5.3 (≤8 calls, ≤10 min, ≤200 k tokens, ≤3 retries).
//!
//! Agents are selected by the orchestrator based on the workflow trigger;
//! no user interaction is required unless a `UserGate::Required` agent is
//! invoked (in that case the orchestrator suspends and emits a `RunEvent`).

#![forbid(unsafe_code)]

pub mod config;
pub mod run;
pub mod event;

pub use config::OrchestratorConfig;
pub use run::{RunHandle, WorkflowTrigger};
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
}
