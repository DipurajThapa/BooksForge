//! Filesystem abstractions for project bundles (Layer 4 — infrastructure).
//!
//! # Bundle layout
//!
//! ```text
//! my-novel.booksforge/
//! ├── manifest.toml           ← project metadata
//! ├── project.db              ← SQLite database (WAL mode)
//! ├── .booksforge-version     ← minimum app version that can open this bundle
//! ├── .booksforge.lock        ← advisory process lock (PID text file)
//! ├── manuscript/             ← per-chapter Markdown mirrors
//! ├── assets/                 ← content-addressed asset store
//! ├── snapshots/objects/      ← content-addressed snapshot objects
//! ├── exports/                ← output artefacts
//! ├── agent_runs/             ← per-run agent artifacts
//! └── plugins/                ← empty in MVP
//! ```
//!
//! # Atomic creation guarantee
//!
//! `create_bundle` writes to a temp directory first, then renames it.
//! A crash between start and rename leaves only a temp dir (cleaned up on
//! next launch by `cleanup_orphan_temp_dirs`), never a partial bundle.

// unsafe_code is permitted in this crate: `pid_is_alive` in lock.rs uses
// `libc::kill(pid, 0)` on Unix and `OpenProcess` on Windows — both require
// unsafe blocks that are tightly scoped and well-justified.

pub mod atomic;
pub mod bundle;
pub mod lock;
pub mod manifest;
pub mod markdown_mirror;
pub mod recovery;
pub mod settings;
pub mod traits;

pub use bundle::{create_bundle, validate_bundle};
pub use lock::{BundleLock, LockError};
pub use manifest::BundleManifest;
pub use settings::{load_settings, save_settings, settings_path};
pub use traits::{BundleFilesystem, OsFilesystem};

// Re-export BundlePath for convenience.
pub use bundle::BundlePath;

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("bundle not found at: {path}")]
    BundleNotFound { path: String },

    #[error("bundle already exists at: {path}")]
    BundleAlreadyExists { path: String },

    #[error("not a valid bundle directory (missing manifest.toml): {path}")]
    NotABundle { path: String },

    #[error("bundle is locked by another process (PID {pid})")]
    AlreadyLocked { pid: u32 },

    #[error("I/O error at {path}: {source}")]
    Io { path: String, source: std::io::Error },

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<LockError> for FsError {
    fn from(e: LockError) -> Self {
        match e {
            LockError::AlreadyLocked { pid } => FsError::AlreadyLocked { pid },
            LockError::Io { path, source } => FsError::Io { path, source },
        }
    }
}
