//! Ollama HTTP client (Layer 4 — infrastructure).
//!
//! All traffic stays on 127.0.0.1:11434 — the privacy invariant.
//! No content is ever sent to a remote server.
//!
//! The `OllamaClient` trait is the stable interface; `HttpOllamaClient` is the
//! production implementation.  Tests inject a mock via the trait.

#![forbid(unsafe_code)]

pub mod client;
pub mod types;

pub use client::{HttpOllamaClient, OllamaClient};
pub use types::{
    CancelToken, ChatMessage, ChatOutcome, ChatRequest, GenerateOutcome, GenerateRequest,
    LocalModel, OllamaVersion,
};

#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Ollama not running — is `ollama serve` active on 127.0.0.1:11434?")]
    NotRunning,

    #[error("model not found: {model}")]
    ModelNotFound { model: String },

    #[error("context window exceeded: {tokens} tokens > {limit}")]
    ContextWindowExceeded { tokens: usize, limit: usize },

    #[error("generation cancelled")]
    Cancelled,

    #[error("HTTP error {status}: {body}")]
    Http { status: u16, body: String },

    #[error("JSON decode error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
}
