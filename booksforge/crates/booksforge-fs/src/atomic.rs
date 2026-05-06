//! Atomic file/directory operations (tmp + rename).

use std::path::Path;
use crate::FsError;

/// Atomically rename `src` to `dst`.
///
/// On POSIX: `std::fs::rename` is guaranteed atomic within the same filesystem.
/// On Windows: we use `MoveFileExW` with MOVEFILE_REPLACE_EXISTING only when
/// `dst` does not already exist — failing early prevents accidentally
/// overwriting an existing bundle.
pub fn atomic_rename(src: &Path, dst: &Path) -> Result<(), FsError> {
    if dst.exists() {
        return Err(FsError::BundleAlreadyExists {
            path: dst.display().to_string(),
        });
    }

    std::fs::rename(src, dst).map_err(|e| FsError::Io {
        path: format!("{} → {}", src.display(), dst.display()),
        source: e,
    })
}

/// Write `content` to `path` atomically: write to `path.tmp` then rename.
///
/// Used for `manifest.toml`, `settings.toml`, and other human-editable files.
pub async fn atomic_write(path: &Path, content: &[u8]) -> Result<(), FsError> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, content).await.map_err(|e| FsError::Io {
        path: tmp.display().to_string(),
        source: e,
    })?;
    std::fs::rename(&tmp, path).map_err(|e| FsError::Io {
        path: format!("{} → {}", tmp.display(), path.display()),
        source: e,
    })
}
