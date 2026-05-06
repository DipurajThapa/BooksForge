//! Ollama HTTP client (Layer 4 — infrastructure).
//!
//! All traffic stays on 127.0.0.1:11434 — the privacy invariant.
//! No content is ever sent to a remote server.
//!
//! The `OllamaClient` trait is the stable interface; `HttpOllamaClient` is the
//! production implementation.  Tests inject a mock via the trait.

#![forbid(unsafe_code)]

pub mod client;
pub mod registry;
pub mod types;

pub use client::{HttpOllamaClient, OllamaClient};
pub use types::{
    CancelToken, ChatMessage, ChatOutcome, ChatRequest, GenerateOutcome, GenerateRequest,
    LocalModel, OllamaVersion, ProgressSink, PullProgress, TokenSink,
};

#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Ollama not running — is `ollama serve` active on 127.0.0.1:11434?")]
    NotRunning,

    #[error("model not found: {model}")]
    ModelNotFound { model: String },

    #[error("no suitable model available: {reason}")]
    NoSuitableModel { reason: String },

    #[error("context window exceeded: {tokens} tokens > {limit}")]
    ContextWindowExceeded { tokens: usize, limit: usize },

    #[error("context window too long for this model")]
    ContextTooLong,

    #[error("out of memory — reduce model size or close other applications")]
    OutOfMemory,

    #[error("insufficient disk space: need {required_bytes} bytes, only {available_bytes} available")]
    DiskSpaceInsufficient { required_bytes: u64, available_bytes: u64 },

    #[error("generation cancelled")]
    Cancelled,

    #[error("HTTP error {status}: {body}")]
    Http { status: u16, body: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON decode error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
}
