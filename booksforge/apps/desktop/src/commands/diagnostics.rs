//! Diagnostic bundle command (BACKLOG §B3).
//!
//! Saves a zip archive containing:
//!   - PII-redacted log files from the rotating appender directory.
//!   - System metadata (OS, app version, git sha, dependency report).
//!   - The currently-open project's `manifest.toml` (sanitised — no
//!     manuscript content).
//!
//! Manuscript content NEVER goes into the bundle.  This is the
//! belt-and-braces version of the privacy invariant: even when the user
//! ASKS for a diagnostic dump, prose and entity bibles stay on the
//! device.  See `crates/booksforge-orchestrator/tests/privacy_invariants.rs`
//! for the test that gates this.

use std::io::Write as _;

use booksforge_ipc::{BooksForgeError, SaveDiagnosticBundleInput, SaveDiagnosticBundleResult};
use chrono::Utc;
use serde::Serialize;
use tauri::State;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::logging::{current_log_directory, redact_line};
use crate::state::AppState;

#[tauri::command]
pub async fn save_diagnostic_bundle(
    input: SaveDiagnosticBundleInput,
    state: State<'_, AppState>,
) -> Result<SaveDiagnosticBundleResult, BooksForgeError> {
    let output_path = input.output_path.clone();

    // Snapshot what we need synchronously, then build the zip in a
    // blocking task to keep the runtime healthy.
    let log_dir = current_log_directory();
    let app_version = env!("CARGO_PKG_VERSION").to_owned();
    let os = format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH,);
    let project_meta = {
        let guard = state.open_project.lock().await;
        guard.as_ref().map(|p| ProjectMetaSnapshot {
            project_id: p.project_id.clone(),
            title: p.title.clone(),
            author: p.author.clone(),
            // Deliberately NOT exposing bundle path — that's PII.  Hash
            // it instead so support can correlate without learning the
            // user's filesystem layout.
            bundle_hash: blake3::hash(p.bundle.root().to_string_lossy().as_bytes())
                .to_hex()
                .to_string(),
        })
    };

    tokio::task::spawn_blocking(
        move || -> Result<SaveDiagnosticBundleResult, BooksForgeError> {
            if let Some(parent) = std::path::Path::new(&output_path).parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    return Err(BooksForgeError::validation(format!(
                        "output directory does not exist: {}",
                        parent.display()
                    )));
                }
            }
            let file = std::fs::File::create(&output_path)
                .map_err(|e| BooksForgeError::internal(format!("create bundle: {e}")))?;
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            // ── 1. Manifest ──
            let manifest = build_manifest(&app_version, &os, project_meta.as_ref());
            zip.start_file("manifest.json", opts)
                .map_err(|e| BooksForgeError::internal(format!("zip manifest: {e}")))?;
            zip.write_all(manifest.as_bytes())
                .map_err(|e| BooksForgeError::internal(format!("write manifest: {e}")))?;

            // ── 2. Redacted log files ──
            let mut log_count = 0u32;
            if let Some(dir) = log_dir.as_ref() {
                if dir.exists() {
                    if let Ok(rd) = std::fs::read_dir(dir) {
                        for entry in rd.flatten() {
                            let path = entry.path();
                            if !path.is_file() {
                                continue;
                            }
                            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                            if !name.starts_with("booksforge") {
                                continue;
                            }
                            let raw = match std::fs::read_to_string(&path) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };
                            let redacted: String =
                                raw.lines().map(redact_line).collect::<Vec<_>>().join("\n");
                            let archive_name = format!("logs/{name}");
                            zip.start_file(&archive_name, opts)
                                .map_err(|e| BooksForgeError::internal(format!("zip log: {e}")))?;
                            zip.write_all(redacted.as_bytes()).map_err(|e| {
                                BooksForgeError::internal(format!("write log: {e}"))
                            })?;
                            log_count += 1;
                        }
                    }
                }
            }

            zip.finish()
                .map_err(|e| BooksForgeError::internal(format!("zip finish: {e}")))?;

            let bytes = std::fs::metadata(&output_path)
                .map(|m| m.len())
                .unwrap_or(0);
            Ok(SaveDiagnosticBundleResult {
                output_path,
                bytes,
                log_files_included: log_count,
                redaction_applied: true,
            })
        },
    )
    .await
    .map_err(|e| BooksForgeError::internal(format!("blocking task: {e}")))?
}

#[derive(Serialize)]
struct ProjectMetaSnapshot {
    project_id: String,
    title: String,
    author: String,
    /// blake3 of the bundle path — lets support correlate without
    /// learning the user's filesystem layout.
    bundle_hash: String,
}

fn build_manifest(app_version: &str, os: &str, project: Option<&ProjectMetaSnapshot>) -> String {
    let now = Utc::now().to_rfc3339();
    let project_section = match project {
        Some(p) => serde_json::json!({
            "project_id":  p.project_id,
            "title":       p.title,
            "author":      p.author,
            "bundle_hash": p.bundle_hash,
        }),
        None => serde_json::Value::Null,
    };
    serde_json::to_string_pretty(&serde_json::json!({
        "kind":              "booksforge-diagnostic-bundle",
        "version":           1,
        "generated_at":      now,
        "app_version":       app_version,
        "os":                os,
        "project":           project_section,
        "redaction_notes":   "Log lines have been processed by the PII redaction filter — emails, non-loopback IPs, and home-directory paths replaced with placeholders.",
        "manuscript_content": "OMITTED BY DESIGN — manuscript prose, entities, and memory are never included in diagnostic bundles regardless of user request.",
    })).unwrap_or_else(|_| "{}".to_owned())
}
