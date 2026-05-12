//! SQLite connection pool — WAL mode, FK enforcement, busy-timeout.

use crate::StorageError;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    SqlitePool,
};
use std::path::Path;

/// Alias so callers import from this crate only.
pub type DbPool = SqlitePool;

/// Open (or create) the SQLite database at `db_path` with production settings:
///
/// - WAL journal mode for concurrent reads
/// - `PRAGMA foreign_keys = ON`
/// - `PRAGMA synchronous = NORMAL` (safe with WAL)
/// - `PRAGMA busy_timeout = 5000` ms
/// - Max 4 read connections + 1 writer (SQLite WAL model)
pub async fn open_pool(db_path: &Path) -> Result<DbPool, StorageError> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_millis(5_000))
        .foreign_keys(true);

    let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    Ok(pool)
}
