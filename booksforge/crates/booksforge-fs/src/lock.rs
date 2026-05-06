//! Advisory process lock for a bundle directory.
//!
//! A second process attempting to open the same bundle will get `LockError::AlreadyLocked`
//! rather than silently corrupting the database.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("bundle already open by another process: {path}")]
    AlreadyLocked { path: String },

    #[error("I/O error acquiring lock at {path}: {source}")]
    Io { path: String, source: std::io::Error },
}

/// RAII guard that holds a lock file for the duration of its lifetime.
/// Drop to release the lock.
pub struct BundleLock {
    path: PathBuf,
}

impl BundleLock {
    /// Attempt to acquire the lock.  Fails immediately (no blocking wait) if
    /// the lock file already exists.
    pub fn acquire(lock_path: PathBuf) -> Result<Self, LockError> {
        use std::fs::OpenOptions;
        use std::io::Write;

        if lock_path.exists() {
            return Err(LockError::AlreadyLocked {
                path: lock_path.display().to_string(),
            });
        }

        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| LockError::Io {
                path: lock_path.display().to_string(),
                source: e,
            })?;

        writeln!(f, "{}", std::process::id()).map_err(|e| LockError::Io {
            path: lock_path.display().to_string(),
            source: e,
        })?;

        Ok(Self { path: lock_path })
    }
}

impl Drop for BundleLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
