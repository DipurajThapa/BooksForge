//! SQLite storage layer (Layer 4 — infrastructure).
//!
//! Manages a single per-bundle SQLite database opened in WAL mode with
//! foreign-key enforcement.  All queries use `sqlx::query!` macros.
//!
//! Public API: open the pool with `open_pool`, run migrations with
//! `run_migrations`, then use `SqliteStorage` to satisfy the
//! `StorageRepository` trait.

#![forbid(unsafe_code)]
// BACKLOG §C4: tests freely use `.unwrap()` / `.expect()` against canned
// fixtures; the workspace-level clippy lints fire only on shipped code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod agent_exemplars;
pub mod migrations;
pub mod pool;
pub mod repository;
pub mod sqlite;

pub use agent_exemplars::{
    fetch_top_exemplars, insert_exemplar, render_exemplars_block, AgentExemplar,
};
pub use migrations::run_migrations;
pub use pool::{open_pool, DbPool};
pub use repository::StorageRepository;
pub use sqlite::SqliteStorage;

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

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
