//! IPC types for the MZ-08 quick-action presets.
//!
//! - `ai_suggest` returns a `job_id` and streams `ai-suggest:<id>:token`
//!   events; the final `ai-suggest:<id>:done` event carries the full text
//!   plus the `ai_call_id` that `ai_apply` will reference.
//! - `ai_cancel({job_id})` aborts the in-flight stream.
//! - `ai_apply({ai_call_id, ...})` takes the pre-edit snapshot and writes
//!   the accepted text to the scene.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input to `ai_suggest`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSuggestInput {
    /// The scene node the call runs against.
    pub node_id:    String,
    /// One of: "sharpen" | "continue" | "rephrase".
    pub preset:     String,
    /// The selected passage (or current paragraph when no selection).
    pub scope_text: String,
    /// Ollama model tag.  When omitted the project default is used.
    pub model:      Option<String>,
    /// Optional context the UI may pass through (e.g. preceding paragraphs
    /// for "continue", tone target for "rephrase").  Stringified JSON object
    /// merged into the prompt template variables.
    pub options_json: Option<String>,
}

/// Result of `ai_suggest` — returned synchronously while the stream runs
/// in a background task.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSuggestStartedResult {
    /// Opaque id used for `ai_cancel` and as the prefix of the streamed
    /// event channel: `ai-suggest:<job_id>:token` / `:done`.
    pub job_id: String,
}

/// Payload emitted on `ai-suggest:<job_id>:token`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSuggestTokenEvent {
    pub job_id: String,
    pub delta:  String,
}

/// Payload emitted on `ai-suggest:<job_id>:done`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSuggestDoneEvent {
    pub job_id:      String,
    /// "ok" | "cancelled" | "error".
    pub status:      String,
    /// `ai_calls.id` — the audit row that was just written.  Always present
    /// regardless of status, since the audit row is always persisted.
    pub ai_call_id:  String,
    /// Full accumulated text (may be partial when `status != "ok"`).
    pub full_text:   String,
    pub duration_ms: u64,
    pub error:       Option<String>,
}

/// Input to `ai_cancel`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiCancelInput {
    pub job_id: String,
}

/// Input to `ai_apply`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiApplyInput {
    pub ai_call_id:    String,
    /// The text the user accepted in the diff panel.  Usually equal to the
    /// `full_text` from the done event but the user may have edited it.
    pub accepted_text: String,
    /// "replace" (Sharpen / Rephrase) or "append" (Continue).
    pub op:            String,
}

/// Result of `ai_apply`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiApplyResult {
    pub ai_call_id:      String,
    pub pre_snapshot_id: String,
    /// ISO-8601 UTC timestamp.
    pub applied_at:      String,
}
