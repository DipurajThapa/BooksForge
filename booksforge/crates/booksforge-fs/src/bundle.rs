//! Bundle path layout and atomic bundle creation.

use std::path::{Path, PathBuf};
use ulid::Ulid;

use crate::{atomic::atomic_rename, FsError};

/// Canonical layout of a `*.booksforge/` bundle directory.
///
/// All sub-paths are derived from a single root so no other crate needs to
/// hard-code directory names.
#[derive(Debug, Clone)]
pub struct BundlePath {
    root: PathBuf,
}

impl BundlePath {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }

    pub fn root(&self) -> &Path { &self.root }

    pub fn manifest(&self)  -> PathBuf { self.root.join("manifest.toml") }
    pub fn db(&self)        -> PathBuf { self.root.join("project.db") }
    pub fn version_file(&self) -> PathBuf { self.root.join(".booksforge-version") }
    pub fn lock_file(&self) -> PathBuf { self.root.join(".booksforge.lock") }
    pub fn manuscript(&self)  -> PathBuf { self.root.join("manuscript") }
    pub fn assets(&self)      -> PathBuf { self.root.join("assets") }
    pub fn snapshots(&self)   -> PathBuf { self.root.join("snapshots") }
    pub fn snapshots_objects(&self) -> PathBuf { self.snapshots().join("objects") }
    pub fn exports(&self)     -> PathBuf { self.root.join("exports") }
    pub fn agent_runs(&self)  -> PathBuf { self.root.join("agent_runs") }
    pub fn plugins(&self)     -> PathBuf { self.root.join("plugins") }

    pub fn chapter_file(&self, node_ulid: &str) -> PathBuf {
        self.manuscript().join(format!("{node_ulid}.md"))
    }

    pub fn run_dir(&self, run_ulid: &str) -> PathBuf {
        self.agent_runs().join(run_ulid)
    }

    pub fn snapshot_object(&self, hash_hex: &str) -> PathBuf {
        // Two-char prefix directory for content-addressed storage.
        let (prefix, rest) = hash_hex.split_at(2.min(hash_hex.len()));
        self.snapshots_objects().join(prefix).join(rest)
    }
}

/// The minimum app version written into `.booksforge-version`.
pub const MIN_APP_VERSION: &str = "0.0.1";

/// The `.gitignore` content shipped inside every new bundle.
const BUNDLE_GITIGNORE: &str = "\
# BooksForge bundle — git-friendly defaults\n\
project.db-wal\n\
project.db-shm\n\
.booksforge.lock\n\
.recovery.log\n\
agent_runs/\n\
exports/\n\
";

/// Create a new bundle at `final_path` atomically.
///
/// Steps:
/// 1. Create a temp dir in the system temp directory.
/// 2. Write all bundle files into the temp dir.
/// 3. `manifest_toml` is written as `manifest.toml` verbatim.
/// 4. Rename temp dir → `final_path`.
/// 5. On any failure before rename: delete the temp dir.
///
/// The `db_init` callback receives the `project.db` path inside the temp dir
/// so the caller can open SQLite and run migrations before the rename.
pub async fn create_bundle<F, Fut>(
    final_path: &Path,
    manifest_toml: &str,
    db_init: F,
) -> Result<BundlePath, FsError>
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: std::future::Future<Output = Result<(), FsError>>,
{
    if final_path.exists() {
        return Err(FsError::BundleAlreadyExists {
            path: final_path.display().to_string(),
        });
    }

    // ── Step 1: temp directory ───────────────────────────────────────────
    let temp_name = format!("booksforge-create-{}", Ulid::new());
    let temp_path = std::env::temp_dir().join(&temp_name);

    tokio::fs::create_dir_all(&temp_path).await.map_err(|e| FsError::Io {
        path: temp_path.display().to_string(),
        source: e,
    })?;

    let cleanup = || {
        let _ = std::fs::remove_dir_all(&temp_path);
    };

    // ── Step 2: write files ──────────────────────────────────────────────
    let bp = BundlePath::new(&temp_path);

    // Owned-content closure — taking `&str` triggers an async-closure
    // lifetime issue under newer Rust, so we copy into a String per write.
    let write_str = |path: PathBuf, content: String| async move {
        tokio::fs::write(&path, content).await.map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })
    };

    if let Err(e) = write_str(bp.manifest(), manifest_toml.to_owned()).await {
        cleanup();
        return Err(e);
    }

    if let Err(e) = write_str(bp.version_file(), MIN_APP_VERSION.to_owned()).await {
        cleanup();
        return Err(e);
    }

    // .gitignore
    let gitignore_path = temp_path.join(".gitignore");
    if let Err(e) = tokio::fs::write(&gitignore_path, BUNDLE_GITIGNORE).await {
        cleanup();
        return Err(FsError::Io { path: gitignore_path.display().to_string(), source: e });
    }

    // Subdirectories
    for sub in &[
        bp.manuscript(),
        bp.assets(),
        bp.snapshots_objects(),
        bp.exports(),
        bp.agent_runs(),
        bp.plugins(),
    ] {
        if let Err(e) = tokio::fs::create_dir_all(sub).await {
            cleanup();
            return Err(FsError::Io { path: sub.display().to_string(), source: e });
        }
    }

    // ── Step 3: caller initialises project.db ────────────────────────────
    if let Err(e) = db_init(bp.db()).await {
        cleanup();
        return Err(e);
    }

    // ── Step 4: atomic rename ────────────────────────────────────────────
    if let Err(e) = atomic_rename(&temp_path, final_path) {
        cleanup();
        return Err(e);
    }

    Ok(BundlePath::new(final_path))
}

/// Validate that a directory is a complete bundle (manifest.toml present).
pub fn validate_bundle(path: &BundlePath) -> Result<(), FsError> {
    if !path.root().exists() {
        return Err(FsError::BundleNotFound {
            path: path.root().display().to_string(),
        });
    }
    if !path.manifest().exists() {
        return Err(FsError::NotABundle {
            path: path.root().display().to_string(),
        });
    }
    Ok(())
}

/// Scan the system temp directory for orphan `booksforge-create-*` directories
/// older than 5 minutes and remove them.  Called on every app launch.
pub async fn cleanup_orphan_temp_dirs() {
    let temp = std::env::temp_dir();
    let cutoff = std::time::Duration::from_secs(5 * 60);

    let Ok(mut entries) = tokio::fs::read_dir(&temp).await else { return };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("booksforge-create-") { continue; }
        if let Ok(meta) = entry.metadata().await {
            if let Ok(modified) = meta.modified() {
                if modified.elapsed().unwrap_or_default() > cutoff {
                    let _ = tokio::fs::remove_dir_all(entry.path()).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_paths_are_deterministic() {
        let bp = BundlePath::new("/tmp/test.booksforge");
        assert_eq!(bp.manifest(),  PathBuf::from("/tmp/test.booksforge/manifest.toml"));
        assert_eq!(bp.db(),        PathBuf::from("/tmp/test.booksforge/project.db"));
        assert_eq!(bp.lock_file(), PathBuf::from("/tmp/test.booksforge/.booksforge.lock"));
        assert_eq!(
            bp.chapter_file("01ABC"),
            PathBuf::from("/tmp/test.booksforge/manuscript/01ABC.md")
        );
    }
}
