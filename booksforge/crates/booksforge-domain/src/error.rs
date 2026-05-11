use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid node operation: node kind {kind:?} cannot {operation}")]
    InvalidNodeOperation {
        kind: crate::NodeKind,
        operation: &'static str,
    },

    #[error("circular parent reference: node {id} cannot be its own ancestor")]
    CircularReference { id: String },

    #[error("schema version mismatch: file is v{file}, app supports up to v{app}")]
    SchemaTooNew { file: u32, app: u32 },

    #[error("project brief validation failed: {message}")]
    BriefValidation { message: String },
}
