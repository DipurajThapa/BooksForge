//! J1 — Best-effort Markdown memory mirror.
//!
//! After every memory_upsert, the caller should write a human-readable
//! Markdown copy of the entry under `manuscript/.memory/<scope>/<key>.md`.
//! The mirror is intentionally lossy (the canonical source of truth is the
//! `memory_entries` SQLite table) and exists so:
//!
//! 1. Writers can grep their book's accumulated memory with their own tools.
//! 2. A `git diff` over the bundle reveals exactly what the agents learnt.
//! 3. Disaster-recovery: if the SQLite file corrupts, the JSON values are
//!    still readable from disk.
//!
//! Failure modes are logged and swallowed — never block the upsert.

use std::path::PathBuf;

use serde_json::Value;

use crate::{bundle::BundlePath, FsError};

/// Compute the on-disk path for `(scope, key)`.
///
/// Sanitises the key by replacing any character outside `[A-Za-z0-9._-]`
/// with `_`, and truncating to 96 chars.  The original key is preserved
/// inside the file's frontmatter for round-tripping.
pub fn memory_path(bundle: &BundlePath, scope: &str, key: &str) -> PathBuf {
    let safe_key = sanitise_key(key);
    bundle
        .manuscript()
        .join(".memory")
        .join(scope)
        .join(format!("{safe_key}.md"))
}

/// Write `manuscript/.memory/<scope>/<key>.md` for one memory entry.
///
/// Creates parent directories as needed.  Returns `Ok(())` on success;
/// caller should log and ignore errors so a mirror failure never blocks
/// a database commit.
pub async fn write_memory_mirror(
    bundle: &BundlePath,
    scope: &str,
    key: &str,
    agent_id: &str,
    value_json: &Value,
    updated_at_iso: &str,
) -> Result<(), FsError> {
    let path = memory_path(bundle, scope, key);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| FsError::Io {
            path: parent.display().to_string(),
            source: e,
        })?;
    }
    let body = render(scope, key, agent_id, value_json, updated_at_iso);
    tokio::fs::write(&path, body.as_bytes())
        .await
        .map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })
}

/// Remove the on-disk mirror for `(scope, key)`.  Missing files are not an error.
pub async fn delete_memory_mirror(bundle: &BundlePath, scope: &str, key: &str) -> Result<(), FsError> {
    let path = memory_path(bundle, scope, key);
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(FsError::Io {
            path: path.display().to_string(),
            source: e,
        }),
    }
}

fn render(scope: &str, key: &str, agent_id: &str, value: &Value, updated_at_iso: &str) -> String {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    format!(
        "---\nscope: {scope}\nkey: {key}\nagent_id: {agent_id}\nupdated_at: {updated_at_iso}\n---\n\n```json\n{pretty}\n```\n"
    )
}

fn sanitise_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.len() > 96 {
        out.truncate(96);
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_replaces_unsafe_chars() {
        assert_eq!(sanitise_key("chapter:01/summary"), "chapter_01_summary");
        assert_eq!(sanitise_key("simple"), "simple");
        assert_eq!(sanitise_key(""), "_");
    }

    #[test]
    fn render_includes_frontmatter() {
        let body = render("book", "title", "outline-architect", &serde_json::json!({"v": 1}), "2026-05-07T00:00:00Z");
        assert!(body.contains("scope: book"));
        assert!(body.contains("key: title"));
        assert!(body.contains("\"v\": 1"));
    }
}
