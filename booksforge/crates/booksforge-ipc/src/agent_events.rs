//! Live agent-run events (BACKLOG §E4).
//!
//! Emitted by the desktop layer over Tauri's event channel so the
//! frontend can show a floating progress widget for in-flight agent
//! runs and offer a Cancel button.  Two event types:
//!
//!   - `agent-run-started`   — fires immediately before dispatch, with
//!                              the `run_id` the frontend uses to
//!                              identify this run for Cancel.
//!   - `agent-run-completed` — fires after the dispatch resolves
//!                              (success, failure, or cancel).
//!
//! These are Tauri events, NOT IPC commands — the wire shape stays in
//! this crate so frontend and backend share one source of truth.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Event payload emitted on `agent-run-started`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentRunStartedEvent {
    /// UUID-ish id the frontend uses to match start ↔ complete and to
    /// pass to `agent_cancel`.  Distinct from the orchestrator's
    /// internal `task_id` because we issue this *before* the task row
    /// is created (so we can cancel even during setup).
    pub run_id: String,
    pub agent_id: String,
    /// ISO-8601 timestamp when the dispatch started.
    pub started_at: String,
}

/// Event payload emitted on `agent-run-completed`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentRunCompletedEvent {
    pub run_id: String,
    pub agent_id: String,
    /// `"completed"` | `"cancelled"` | `"error"`
    pub status: String,
    pub error: Option<String>,
    /// ISO-8601 timestamp when the dispatch finished.
    pub finished_at: String,
}

/// Periodic progress event emitted at ~4 Hz while a long-running
/// agent (chapter-drafter, dev-editor) is streaming tokens from
/// Ollama.  The frontend overlay uses this to show "342 tokens · 18 t/s".
///
/// Volume: ~4 events/sec while active.  Auto-stops on
/// `agent-run-completed`.  Other agents (intake, vocab, memory-curator)
/// don't emit this — they're fast enough that elapsed time alone is
/// adequate feedback.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentRunProgressEvent {
    pub run_id: String,
    /// Cumulative tokens received from Ollama since dispatch.
    pub tokens: u32,
    /// Wall-clock milliseconds since dispatch.  Frontend computes
    /// tokens/sec = tokens / (elapsed_ms/1000).
    pub elapsed_ms: u32,
}

/// Input to `agent_cancel`.  Cancel is best-effort: a cancelled run
/// stops at the next sink callback or retry boundary inside the
/// runner; in-flight HTTP requests to Ollama are torn down via the
/// existing `CancelToken` plumbing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentCancelInput {
    pub run_id: String,
}
