use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

/// Tagged-union error returned by every Tauri IPC command.
///
/// Rules:
/// - Add a new variant rather than using `Internal` for a known failure mode.
/// - Never put raw stack traces or file paths in the payload — they leak internals
///   to the frontend and to any crash report.
/// - The `kind` discriminant is the TypeScript type tag.
#[derive(Debug, Clone, Serialize, Deserialize, TS, Error)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
#[serde(tag = "kind", content = "payload")]
pub enum BooksForgeError {
    #[error("not found: {resource}")]
    NotFound { resource: String },

    #[error("validation failed: {message}")]
    Validation { message: String },

    #[error("io error: {message}")]
    Io { message: String },

    #[error("database error: {message}")]
    Storage { message: String },

    #[error("agent runtime unavailable: {reason}")]
    AgentRuntimeUnavailable { reason: String },

    #[error("agent run cancelled")]
    Cancelled,

    #[error("schema too new: project requires app v{required_min}, running v{running}")]
    SchemaTooNew {
        required_min: String,
        running: String,
    },

    #[error("internal error — please report this: {message}")]
    Internal { message: String },
}

impl BooksForgeError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }
}
