//! Events emitted by the orchestrator to the Tauri frontend via IPC.

use serde::{Deserialize, Serialize};

/// Events emitted during a workflow run.
/// The Tauri layer subscribes to these and forwards them to the React UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunEvent {
    /// An agent step started.
    StepStarted {
        run_id: String,
        step: u32,
        agent_id: String,
    },

    /// An agent step completed successfully.
    StepCompleted {
        run_id: String,
        step: u32,
        agent_id: String,
        tokens_used: u32,
        duration_ms: u64,
    },

    /// An agent step failed and will be retried.
    StepRetrying {
        run_id: String,
        step: u32,
        agent_id: String,
        attempt: u32,
        reason: String,
    },

    /// The whole run succeeded.
    RunCompleted {
        run_id: String,
        total_steps: u32,
        total_tokens: u32,
        duration_ms: u64,
    },

    /// The run failed and cannot continue.
    RunFailed { run_id: String, reason: String },

    /// A `UserGate::Required` agent is waiting for the user to proceed.
    AwaitingUserGate {
        run_id: String,
        agent_id: String,
        prompt: String,
    },
}
