//! `BundleFilesystem` trait — Layer 3 ↔ Layer 4 boundary for the filesystem.

use async_trait::async_trait;
use std::path::Path;

use crate::{bundle::BundlePath, FsError};

/// Stable interface for all bundle filesystem operations.
///
/// Implemented by `OsFilesystem` in production and `TmpDirFilesystem` in tests.
#[async_trait]
pub trait BundleFilesystem: Send + Sync {
    /// Create a new bundle at `final_path` atomically.
    ///
    /// The `manifest_toml` string is written verbatim as `manifest.toml`.
    /// The `db_init` callback receives the `project.db` path inside the temp
    /// dir so the caller can open SQLite and run migrations before the rename.
    async fn create_bundle(
        &self,
        final_path: &Path,
        manifest_toml: &str,
        db_path_callback: Box<dyn FnOnce(std::path::PathBuf) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), FsError>> + Send>> + Send>,
    ) -> Result<BundlePath, FsError>;

    /// Open an existing bundle, validating its structure.
    /// Acquires the advisory lock (fails if another live process holds it).
    fn open_bundle(&self, path: &Path) -> Result<(BundlePath, crate::lock::BundleLock), FsError>;

    /// Write `manifest.toml` atomically (tmp + rename).
    async fn write_manifest(&self, bundle: &BundlePath, toml: &str) -> Result<(), FsError>;

    /// Write a chapter Markdown mirror file.
    async fn write_chapter_md(
        &self,
        bundle: &BundlePath,
        node_ulid: &str,
        markdown: &str,
    ) -> Result<(), FsError>;

    /// Write a content-addressed snapshot object.
    /// Returns the blake3 hex hash (which is also the filename under
    /// `snapshots/objects/<prefix>/<rest>`).
    async fn write_snapshot_object(
        &self,
        bundle: &BundlePath,
        content: &[u8],
    ) -> Result<String, FsError>;

    /// Read a content-addressed snapshot object, verifying its hash on read.
    ///
    /// Returns `FsError::Io` if the object does not exist.  Returns
    /// `FsError::Serialization` if the bytes do not hash to `hash_hex` —
    /// the object was corrupted or tampered with.
    async fn read_snapshot_object(
        &self,
        bundle: &BundlePath,
        hash_hex: &str,
    ) -> Result<Vec<u8>, FsError>;
}

/// Production OS-backed implementation of `BundleFilesystem`.
pub struct OsFilesystem;

#[async_trait]
impl BundleFilesystem for OsFilesystem {
    async fn create_bundle(
        &self,
        final_path: &Path,
        manifest_toml: &str,
        db_path_callback: Box<dyn FnOnce(std::path::PathBuf) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), FsError>> + Send>> + Send>,
    ) -> Result<BundlePath, FsError> {
        crate::bundle::create_bundle(final_path, manifest_toml, |db_path| {
            db_path_callback(db_path)
        })
        .await
    }

    fn open_bundle(&self, path: &Path) -> Result<(BundlePath, crate::lock::BundleLock), FsError> {
        let bp = BundlePath::new(path);
        crate::bundle::validate_bundle(&bp)?;
        let lock = crate::lock::BundleLock::acquire(bp.lock_file())?;
        Ok((bp, lock))
    }

    async fn write_manifest(&self, bundle: &BundlePath, toml: &str) -> Result<(), FsError> {
        crate::atomic::atomic_write(&bundle.manifest(), toml.as_bytes()).await
    }

    async fn write_chapter_md(
        &self,
        bundle: &BundlePath,
        node_ulid: &str,
        markdown: &str,
    ) -> Result<(), FsError> {
        let path = bundle.chapter_file(node_ulid);
        tokio::fs::write(&path, markdown).await.map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    async fn write_snapshot_object(
        &self,
        bundle: &BundlePath,
        content: &[u8],
    ) -> Result<String, FsError> {
        let hash = blake3::hash(content).to_hex().to_string();
        let dest = bundle.snapshot_object(&hash);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| FsError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        // Only write if not already present (content-addressed dedup).
        // Atomic: write to a sibling tmp file, then rename — readers never
        // observe a half-written object.
        if !dest.exists() {
            let tmp_name = format!(
                "{}.{}.tmp",
                dest.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| hash.clone()),
                ulid::Ulid::new(),
            );
            let tmp = dest.with_file_name(tmp_name);
            tokio::fs::write(&tmp, content).await.map_err(|e| FsError::Io {
                path: tmp.display().to_string(),
                source: e,
            })?;
            // If a concurrent writer beat us to it, the rename is still safe —
            // both objects have the same content (same hash).
            if let Err(e) = std::fs::rename(&tmp, &dest) {
                // Tolerate "destination already exists" races; clean tmp.
                let _ = std::fs::remove_file(&tmp);
                if !dest.exists() {
                    return Err(FsError::Io {
                        path: format!("{} → {}", tmp.display(), dest.display()),
                        source: e,
                    });
                }
            }
        }
        Ok(hash)
    }

    async fn read_snapshot_object(
        &self,
        bundle: &BundlePath,
        hash_hex: &str,
    ) -> Result<Vec<u8>, FsError> {
        let path = bundle.snapshot_object(hash_hex);
        let bytes = tokio::fs::read(&path).await.map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let actual = blake3::hash(&bytes).to_hex().to_string();
        if actual != hash_hex {
            return Err(FsError::Serialization(format!(
                "snapshot object {hash_hex} hash mismatch (got {actual})"
            )));
        }
        Ok(bytes)
    }
}
