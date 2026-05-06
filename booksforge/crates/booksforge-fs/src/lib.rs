//! Filesystem abstractions for project bundles (Layer 4 — infrastructure).
//!
//! A project bundle is a directory named `<slug>.booksforge/`.  All paths
//! within the bundle are derived from `BundlePath` so no other crate needs
//! to hard-code subdirectory names.
//!
//! `BundlePath` is `Clone + Send + Sync` and cheap to pass around.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

pub mod lock;

pub use lock::{BundleLock, LockError};

/// Canonical layout of a `*.booksforge/` bundle directory.
///
/// ```text
/// my-novel.booksforge/
/// ├── manifest.toml           ← project metadata (human-editable)
/// ├── project.db              ← SQLite database (WAL mode)
/// ├── manuscript/             ← per-chapter Markdown snapshots
/// │   └── <node-ulid>.md
/// ├── snapshots/              ← content-addressed immutable snapshots
/// │   └── <blake3-hex>
/// ├── agent_runs/             ← structured logs for each orchestrator run
/// │   └── <run-ulid>/
/// │       ├── run.json
/// │       └── steps/
/// └── exports/                ← output artefacts (EPUB, PDF, DOCX)
///     └── <profile>-<hash>.*
/// ```
#[derive(Debug, Clone)]
pub struct BundlePath {
    root: PathBuf,
}

impl BundlePath {
    /// Create a `BundlePath` from any path that ends in `*.booksforge`.
    ///
    /// Does **not** check whether the path exists on disk.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }

    /// The bundle root directory itself.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/manifest.toml`
    pub fn manifest(&self) -> PathBuf {
        self.root.join("manifest.toml")
    }

    /// `<root>/project.db`
    pub fn db(&self) -> PathBuf {
        self.root.join("project.db")
    }

    /// `<root>/manuscript/`
    pub fn manuscript(&self) -> PathBuf {
        self.root.join("manuscript")
    }

    /// `<root>/manuscript/<ulid>.md`
    pub fn chapter_file(&self, node_ulid: &str) -> PathBuf {
        self.manuscript().join(format!("{node_ulid}.md"))
    }

    /// `<root>/snapshots/`
    pub fn snapshots(&self) -> PathBuf {
        self.root.join("snapshots")
    }

    /// `<root>/snapshots/<blake3-hex>`
    pub fn snapshot_file(&self, hash_hex: &str) -> PathBuf {
        self.snapshots().join(hash_hex)
    }

    /// `<root>/agent_runs/`
    pub fn agent_runs(&self) -> PathBuf {
        self.root.join("agent_runs")
    }

    /// `<root>/agent_runs/<run-ulid>/`
    pub fn run_dir(&self, run_ulid: &str) -> PathBuf {
        self.agent_runs().join(run_ulid)
    }

    /// `<root>/agent_runs/<run-ulid>/run.json`
    pub fn run_manifest(&self, run_ulid: &str) -> PathBuf {
        self.run_dir(run_ulid).join("run.json")
    }

    /// `<root>/agent_runs/<run-ulid>/steps/`
    pub fn run_steps_dir(&self, run_ulid: &str) -> PathBuf {
        self.run_dir(run_ulid).join("steps")
    }

    /// `<root>/exports/`
    pub fn exports(&self) -> PathBuf {
        self.root.join("exports")
    }

    /// `<root>/.booksforge.lock`  — process-level advisory lock file.
    pub fn lock_file(&self) -> PathBuf {
        self.root.join(".booksforge.lock")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("bundle not found at: {path}")]
    BundleNotFound { path: String },

    #[error("bundle already exists at: {path}")]
    BundleAlreadyExists { path: String },

    #[error("not a valid bundle directory (missing manifest.toml): {path}")]
    NotABundle { path: String },

    #[error("I/O error at {path}: {source}")]
    Io { path: String, source: std::io::Error },
}

/// Initialise a new empty bundle directory layout on disk.
pub async fn init_bundle(path: &BundlePath) -> Result<(), FsError> {
    let root = path.root();

    if root.exists() {
        return Err(FsError::BundleAlreadyExists {
            path: root.display().to_string(),
        });
    }

    tokio::fs::create_dir_all(root).await.map_err(|e| FsError::Io {
        path: root.display().to_string(),
        source: e,
    })?;

    for sub in &[
        path.manuscript(),
        path.snapshots(),
        path.agent_runs(),
        path.exports(),
    ] {
        tokio::fs::create_dir_all(sub).await.map_err(|e| FsError::Io {
            path: sub.display().to_string(),
            source: e,
        })?;
    }

    Ok(())
}

/// Validate that a path looks like an existing bundle (has `manifest.toml`).
pub fn validate_bundle(path: &BundlePath) -> Result<(), FsError> {
    let root = path.root();
    if !root.exists() {
        return Err(FsError::BundleNotFound {
            path: root.display().to_string(),
        });
    }
    if !path.manifest().exists() {
        return Err(FsError::NotABundle {
            path: root.display().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_paths_are_deterministic() {
        let bp = BundlePath::new("/tmp/my-novel.booksforge");
        assert_eq!(bp.manifest(), PathBuf::from("/tmp/my-novel.booksforge/manifest.toml"));
        assert_eq!(bp.db(),       PathBuf::from("/tmp/my-novel.booksforge/project.db"));
        assert_eq!(
            bp.chapter_file("01ABCDEF"),
            PathBuf::from("/tmp/my-novel.booksforge/manuscript/01ABCDEF.md")
        );
        assert_eq!(
            bp.lock_file(),
            PathBuf::from("/tmp/my-novel.booksforge/.booksforge.lock")
        );
    }
}
