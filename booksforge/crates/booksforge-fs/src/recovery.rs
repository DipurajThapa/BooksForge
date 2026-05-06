//! Crash-recovery log for the autosave pipeline.
//!
//! The protocol is:
//!
//! 1. **Before** committing a scene save to SQLite, call `write_pending`.
//! 2. **After** a successful SQLite commit, call `write_committed`.
//! 3. On project open, call `check` to see if any uncommitted saves exist.
//!
//! The file is append-only JSONL. Each line is one of:
//!   `{"type":"pending","node_id":"…","ts":"…"}` (written before commit)
//!   `{"type":"committed","ts":"…"}`            (written after commit)
//!
//! A pending entry is "unresolved" when no committed entry at the same or
//! later timestamp follows it.  The newest unresolved pending entry is what
//! we surface as the crash recovery candidate.

use std::path::Path;

use crate::{bundle::BundlePath, FsError};

const LOG_FILENAME: &str = ".recovery.log";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RecoveryEntry {
    Pending { node_id: String, ts: String },
    Committed { ts: String },
}

/// Append a `pending` entry to the recovery log.
pub async fn write_pending(
    bundle: &BundlePath,
    node_id: &str,
    ts: &str,
) -> Result<(), FsError> {
    let entry = RecoveryEntry::Pending {
        node_id: node_id.to_owned(),
        ts: ts.to_owned(),
    };
    append_entry(bundle, &entry).await
}

/// Append a `committed` entry to the recovery log.
pub async fn write_committed(bundle: &BundlePath, ts: &str) -> Result<(), FsError> {
    let entry = RecoveryEntry::Committed { ts: ts.to_owned() };
    append_entry(bundle, &entry).await
}

/// Check whether the recovery log has any unresolved pending entry.
///
/// Returns `Some((node_id, ts))` for the most-recent unresolved pending save,
/// or `None` if all saves are committed.
pub async fn check(bundle: &BundlePath) -> Result<Option<(String, String)>, FsError> {
    let path = bundle.root().join(LOG_FILENAME);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = tokio::fs::read(&path).await.map_err(|e| FsError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let text = std::str::from_utf8(&bytes)
        .unwrap_or("")
        .trim_end()
        .to_string();

    // Walk lines newest-first to find the state.
    let mut last_committed_ts: Option<String> = None;
    let mut last_pending: Option<(String, String)> = None;

    for line in text.lines() {
        if let Ok(entry) = serde_json::from_str::<RecoveryEntry>(line) {
            match entry {
                RecoveryEntry::Committed { ts } => {
                    // Keep the newest committed ts seen.
                    if last_committed_ts.is_none()
                        || ts > *last_committed_ts.as_ref().unwrap()
                    {
                        last_committed_ts = Some(ts);
                    }
                }
                RecoveryEntry::Pending { node_id, ts } => {
                    // Keep the newest pending entry seen.
                    if last_pending.is_none()
                        || ts > last_pending.as_ref().unwrap().1
                    {
                        last_pending = Some((node_id, ts));
                    }
                }
            }
        }
    }

    match (last_pending, last_committed_ts) {
        (Some((node_id, ts)), Some(committed_ts)) if ts <= committed_ts => Ok(None),
        (Some(pending), _) => Ok(Some(pending)),
        _ => Ok(None),
    }
}

/// Clear the recovery log after a successful open or recovery.
pub async fn clear(bundle: &BundlePath) -> Result<(), FsError> {
    let path = bundle.root().join(LOG_FILENAME);
    if path.exists() {
        tokio::fs::remove_file(&path).await.map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    }
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

async fn append_entry(bundle: &BundlePath, entry: &RecoveryEntry) -> Result<(), FsError> {
    use tokio::io::AsyncWriteExt;

    let path = bundle.root().join(LOG_FILENAME);
    let mut line = serde_json::to_string(entry)
        .map_err(|e| FsError::Serialization(format!("recovery log serialize: {e}")))?;
    line.push('\n');

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;

    file.write_all(line.as_bytes()).await.map_err(|e| FsError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(())
}
