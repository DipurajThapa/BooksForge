//! SQLite storage layer (Layer 4 — infrastructure).
//!
//! Manages a single per-bundle SQLite database opened in WAL mode with
//! foreign-key enforcement.  All queries use compile-time–checked `sqlx`
//! macros; raw SQL strings are forbidden here.
//!
//! The pool is configured to allow at most one writer (SQLite limit) and
//! up to 4 read-only connections, matching the WAL concurrency model.

#![forbid(unsafe_code)]

pub mod pool;
pub mod migrations;

pub use pool::{DbPool, open_pool};
pub use migrations::run_migrations;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database not found at path: {path}")]
    NotFound { path: String },

    #[error("migration failed at version {version}: {source}")]
    MigrationFailed { version: i64, source: sqlx::Error },

    #[error("query error: {0}")]
    Query(#[from] sqlx::Error),

    #[error("schema too new: db version {db_version} > app version {app_version}")]
    SchemaTooNew { db_version: i64, app_version: i64 },

    #[error("constraint violation: {detail}")]
    ConstraintViolation { detail: String },
}
