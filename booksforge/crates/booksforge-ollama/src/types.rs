//! Request/response types for the Ollama REST API.

use serde::{Deserialize, Serialize};

/// Ollama server version reported by `GET /api/version`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaVersion {
    pub version: String,
}

/// A model available on the local Ollama instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub name:        String,
    pub modified_at: String,
    /// Size in bytes.
    pub size:        u64,
    pub digest:      String,
}

/// A single chat turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `"system"` | `"user"` | `"assistant"`
    pub role:    String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
    }
}

/// Parameters for `POST /api/generate`.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    pub model:  String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

/// Parameters for `POST /api/chat`.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model:    String,
    pub messages: Vec<ChatMessage>,
    pub stream:   bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

/// Shared model-level sampling options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerateOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p:       Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx:     Option<u32>,
    /// Hard token budget enforced by the orchestrator — passed through to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
}

/// Completed generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOutcome {
    pub model:              String,
    pub response:           String,
    pub prompt_eval_count:  u32,
    pub eval_count:         u32,
    /// Nanoseconds of total duration reported by Ollama.
    pub total_duration_ns:  u64,
}

/// Completed chat result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOutcome {
    pub model:              String,
    pub message:            ChatMessage,
    pub prompt_eval_count:  u32,
    pub eval_count:         u32,
    pub total_duration_ns:  u64,
}

/// Progress event emitted during `pull()` — mirrors Ollama's NDJSON stream.
#[derive(Debug, Clone)]
pub struct PullProgress {
    pub status:    String,
    pub completed: Option<u64>,
    pub total:     Option<u64>,
}

/// Callback type used by `OllamaClient::pull` to report download progress.
/// Each call corresponds to one NDJSON line from the Ollama stream.
pub type ProgressSink = Box<dyn Fn(PullProgress) + Send>;

/// Callback type used by `OllamaClient::generate` / `chat` to stream tokens.
/// Called once per token fragment as Ollama streams the response.
pub type TokenSink = Box<dyn Fn(&str) + Send>;

/// Cooperative cancellation token shared between the orchestrator and an
/// in-flight Ollama request.  `cancel()` is idempotent.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    inner: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Signal cancellation.
    pub fn cancel(&self) {
        self.inner.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// `true` if `cancel()` has been called.
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(std::sync::atomic::Ordering::Relaxed)
    }
}
