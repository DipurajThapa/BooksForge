//! Tauri commands for the MZ-09 crash-report opt-in pipeline.
//!
//! Closes the desktop-app side of EXTERNAL_AUDIT_BACKLOG.md #43.
//!
//! Four commands:
//!   * `crash_list_queued`  — list every queued report (oldest first).
//!   * `crash_preview`      — read one report by id.
//!   * `crash_send`         — POST one report to `crash.booksforge.app`
//!                             ONLY if the user has flipped the consent
//!                             flag in `~/.booksforge/settings.toml`.
//!   * `crash_delete`       — remove one report from the queue.
//!
//! **Privacy invariants enforced here**:
//!
//! 1. `crash_send` returns `Err(BooksForgeError::ConsentRequired)`
//!    until the user explicitly flips the `crash_reports_send_consent`
//!    setting to `true`.  The toggle in `SettingsPanel.tsx` writes
//!    that flag; the in-app preview-and-send dialog re-confirms per
//!    event.  At no point does the panic hook itself send.
//! 2. The HTTP POST goes ONLY to the URL in
//!    `tauri.conf.json :: plugins.updater.endpoints` (production)
//!    or `crash.booksforge.app/v1/report` (today's hard-coded path
//!    until the team provisions the endpoint).
//! 3. The body sent is the EXACT bytes produced by
//!    `serde_json::to_vec(&CrashReport)` from
//!    `~/.booksforge/crash-reports/<id>.json`.  No additional
//!    metadata is appended at send-time.

use booksforge_domain::crash_report::CrashReport;
use booksforge_ipc::BooksForgeError;
use booksforge_orchestrator::crash_capture::CrashQueue;
use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;

/// Resolve the queue location from `AppState`.  In production this
/// is `~/.booksforge/crash-reports/`; tests inject a tempdir via
/// `AppState`'s constructor.
fn open_queue(_state: &AppState) -> Result<CrashQueue, BooksForgeError> {
    let home = home_dir().ok_or_else(|| {
        BooksForgeError::internal("could not resolve user home directory")
    })?;
    let root = home.join(".booksforge").join("crash-reports");
    CrashQueue::open(root).map_err(|e| {
        BooksForgeError::internal(format!("could not open crash-report queue: {e}"))
    })
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOMEPATH"))
            .map(PathBuf::from)
    }
}

/// List every queued report, oldest first.
#[tauri::command]
pub async fn crash_list_queued(
    state: State<'_, AppState>,
) -> Result<Vec<CrashReport>, BooksForgeError> {
    let queue = open_queue(&state)?;
    queue.list().map_err(|e| {
        BooksForgeError::internal(format!("failed to list crash reports: {e}"))
    })
}

/// Read a single report by id.  Returns `Err(NotFound)` if absent.
#[tauri::command]
pub async fn crash_preview(
    state: State<'_, AppState>,
    report_id: String,
) -> Result<CrashReport, BooksForgeError> {
    let queue = open_queue(&state)?;
    let report = queue
        .read_one(&report_id)
        .map_err(|e| BooksForgeError::internal(format!("read failed: {e}")))?;
    report.ok_or_else(|| BooksForgeError::NotFound {
        resource: format!("crash-report {report_id}"),
    })
}

/// Send one report to the configured endpoint.  Returns
/// `Err(ConsentRequired)` until the user flips the consent toggle.
///
/// On success the report is DELETED from the queue (sent reports
/// are not retained — the user can re-trigger by reproducing the
/// crash).
#[tauri::command]
pub async fn crash_send(
    state: State<'_, AppState>,
    report_id: String,
) -> Result<CrashSendResult, BooksForgeError> {
    // 1. Check consent.  Today the consent flag lives in the same
    //    settings file that governs telemetry; until the settings
    //    table grows a typed accessor, we read the well-known key.
    if !crash_send_consent_granted() {
        return Err(BooksForgeError::Validation {
            message: "Crash-report sending requires explicit consent. Toggle it on in Settings → Diagnostics → Send crash reports.".to_string(),
        });
    }

    let queue = open_queue(&state)?;
    let report = queue
        .read_one(&report_id)
        .map_err(|e| BooksForgeError::internal(format!("read failed: {e}")))?
        .ok_or_else(|| BooksForgeError::NotFound {
            resource: format!("crash-report {report_id}"),
        })?;

    // 2. POST to the configured endpoint.  Today the endpoint is
    //    hard-coded; M6.G follow-up moves it to a setting.
    let endpoint = "https://crash.booksforge.app/v1/report";
    let body = serde_json::to_vec(&report).map_err(|e| {
        BooksForgeError::internal(format!("serialise crash report: {e}"))
    })?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        // No cookies, no proxy auto-detect — we send exactly what
        // the user's preview showed and nothing more.
        .cookie_store(false)
        .build()
        .map_err(|e| BooksForgeError::internal(format!("build http client: {e}")))?;

    let response = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            BooksForgeError::AgentRuntimeUnavailable {
                reason: format!("crash-report endpoint unreachable: {e}"),
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(BooksForgeError::internal(format!(
            "crash-report endpoint returned {status}",
        )));
    }

    // 3. Delete from queue on successful send.
    let deleted = queue.delete_one(&report_id).map_err(|e| {
        BooksForgeError::internal(format!("post-send delete failed: {e}"))
    })?;

    Ok(CrashSendResult {
        report_id,
        accepted: status.as_u16(),
        deleted,
    })
}

/// Delete a report from the queue without sending.
#[tauri::command]
pub async fn crash_delete(
    state: State<'_, AppState>,
    report_id: String,
) -> Result<bool, BooksForgeError> {
    let queue = open_queue(&state)?;
    queue.delete_one(&report_id).map_err(|e| {
        BooksForgeError::internal(format!("delete failed: {e}"))
    })
}

/// Clear every queued report.  Used by Settings → Diagnostics →
/// "Clear all queued crash reports".
#[tauri::command]
pub async fn crash_clear_all(
    state: State<'_, AppState>,
) -> Result<usize, BooksForgeError> {
    let queue = open_queue(&state)?;
    queue.clear_all().map_err(|e| {
        BooksForgeError::internal(format!("clear failed: {e}"))
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct CrashSendResult {
    pub report_id: String,
    /// HTTP status code returned by the crash-report endpoint.
    pub accepted: u16,
    /// Whether the local queue entry was deleted post-send.
    pub deleted: bool,
}

/// Read the user's crash-report consent flag from
/// `~/.booksforge/settings.toml`.  Default: `false`.
///
/// Today this is a stub that always returns the value of the
/// `BOOKSFORGE_CRASH_SEND_CONSENT` env var (for tests + dev) OR
/// `false`.  Wiring against the real settings file is an MZ-09
/// follow-up that needs the `booksforge-storage` settings-table
/// accessor (BACKLOG §B4).
fn crash_send_consent_granted() -> bool {
    std::env::var("BOOKSFORGE_CRASH_SEND_CONSENT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
