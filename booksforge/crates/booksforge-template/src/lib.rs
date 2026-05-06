//! Template parsing and compilation (Layer 3 — pure logic).
//!
//! Templates live under `templates/<id>/<version>.toml` and define the initial
//! document structure, starter vocabulary, and mode-specific defaults for a
//! new project. Implemented in M4.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Unique identifier for a built-in template, e.g. `"fiction-generic-novel"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateId(pub String);

/// A minimal template descriptor. Full spec implemented in M4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    pub id:          TemplateId,
    pub version:     String,
    pub display_name: String,
    pub mode:        String,
    pub description: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("template not found: {id}")]
    NotFound { id: String },
    #[error("template parse error: {message}")]
    Parse { message: String },
}
