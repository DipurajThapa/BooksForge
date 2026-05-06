//! MiniJinja-based prompt template engine (Layer 3 — pure logic).
//!
//! Each prompt template is a TOML file at `templates/<id>/<version>.toml`.
//! The engine renders variables into the template, fences untrusted content
//! with `<<<USER_CONTENT>>>` markers to mitigate prompt injection, and
//! returns a blake3 hash of the rendered template for audit logging.
//!
//! Full implementation in M5.

#![forbid(unsafe_code)]

use blake3::Hash;
use serde::{Deserialize, Serialize};

/// A versioned reference to a prompt template file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromptTemplateId {
    pub id:      String,
    pub version: String,
}

/// The result of rendering a prompt template.
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    pub template_id:   PromptTemplateId,
    /// blake3 hash of the on-disk template file — recorded in `agent_tasks`.
    pub template_hash: Hash,
    pub system:        String,
    pub user:          String,
}

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("template not found: {id} v{version}")]
    NotFound { id: String, version: String },
    #[error("render error in template '{id}': {message}")]
    Render { id: String, message: String },
    #[error("template hash mismatch — template may have been modified without version bump")]
    HashMismatch,
}
