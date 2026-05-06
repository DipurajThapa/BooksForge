//! Run embedded migrations in order.
//!
//! Migration SQL lives in `migrations/` alongside this crate and is embedded
//! at compile time via `sqlx::migrate!`.  The migrator is idempotent: already-
//! applied migrations are skipped.

use crate::{DbPool, StorageError};

/// Apply all pending migrations to the pool.
pub async fn run_migrations(pool: &DbPool) -> Result<(), StorageError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| StorageError::MigrationFailed {
            version: -1,
            source: e.into(),
        })?;
    Ok(())
}
