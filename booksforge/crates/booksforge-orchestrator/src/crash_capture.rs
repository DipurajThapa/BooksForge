//! Crash-capture pipeline (MZ-09).
//!
//! Closes the orchestrator-side of EXTERNAL_AUDIT_BACKLOG.md #43.
//!
//! This module:
//!   1. Provides `install_panic_hook()` — a one-shot installer that
//!      replaces the default `std::panic` hook with one that builds a
//!      `CrashReport` from the panic info and writes it to a queue.
//!   2. Owns the on-disk queue at `~/.booksforge/crash-reports/<ulid>.json`.
//!   3. Exposes `list_queued()`, `read_one()`, `delete_one()` for the
//!      Tauri commands in `apps/desktop/src/commands/crash.rs`.
//!
//! **Privacy contract**:
//!   - The hook NEVER reads the panic argument values directly.  Rust's
//!     `PanicInfo::message()` returns a `&fmt::Arguments` whose
//!     formatted output is what panics print to stderr; that formatted
//!     string CAN contain manuscript content (e.g. `panic!("scene
//!     {scene_id} corrupt")`).  We deliberately convert
//!     `panic_info.message().to_string()` → take only the static
//!     template prefix (the part up to the first `{` or whitespace
//!     boundary heuristic) → drop everything after.  The full message
//!     is logged ONLY to the local `tracing` log; the queued
//!     `CrashReport.panic_message_template` carries only the static
//!     prefix.
//!   - Stack frames are symbolicated on best-effort by `backtrace`
//!     crate APIs; argument values are explicitly NOT captured.
//!   - The queue directory is created with mode 0700 on Unix (only
//!     the user can read).
//!
//! **Off-by-default contract**: nothing in this module sends a report
//! over the network.  HTTP submission is the Tauri command's job and
//! happens only on explicit user click.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use booksforge_domain::crash_report::{
    AgentKind, Arch, CrashKind, CrashReport, OsFamily, StackFrame,
};

/// One-shot init guard — installing the panic hook twice is a bug.
static HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

/// Where queued reports live.  Provided by the caller (tests use
/// a `tempdir`; production uses `~/.booksforge/crash-reports/`).
#[derive(Debug, Clone)]
pub struct CrashQueue {
    root: PathBuf,
}

impl CrashQueue {
    /// Open (and create-if-missing) a queue at `root`.
    ///
    /// On Unix the directory is chmod'd 0700 so only the running user
    /// can read.  On Windows this is a no-op (NTFS ACLs default to
    /// user-private for `~/.booksforge/...`).
    pub fn open(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&root)?.permissions();
            perms.set_mode(0o700);
            std::fs::set_permissions(&root, perms)?;
        }
        Ok(Self { root })
    }

    /// Persist a `CrashReport` as `<root>/<report_id>.json`.
    /// Returns the path written.
    pub fn enqueue(&self, report: &CrashReport) -> std::io::Result<PathBuf> {
        let path = self.root.join(format!("{}.json", report.report_id));
        let body = serde_json::to_vec_pretty(report).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        // Atomic write: temp + rename.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &body)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }

    /// List every queued report, oldest first.
    pub fn list(&self) -> std::io::Result<Vec<CrashReport>> {
        if !self.root.exists() {
            return Ok(vec![]);
        }
        let mut out: Vec<CrashReport> = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue, // tolerate transient read failures
            };
            match serde_json::from_slice::<CrashReport>(&bytes) {
                Ok(report) => out.push(report),
                Err(_) => {
                    // Corrupted entry — log and skip; do not delete (the
                    // user may want to inspect).
                    tracing::warn!(
                        target: "booksforge::crash_capture",
                        path = %path.display(),
                        "skipping un-parseable crash-report file",
                    );
                }
            }
        }
        out.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
        Ok(out)
    }

    /// Read a single report by id.  Returns `Ok(None)` if not found.
    pub fn read_one(&self, report_id: &str) -> std::io::Result<Option<CrashReport>> {
        let path = self.root.join(format!("{}.json", report_id));
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path)?;
        let report: CrashReport = serde_json::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        Ok(Some(report))
    }

    /// Delete a queued report.  Idempotent: deleting a non-existent
    /// id returns `Ok(false)`.
    pub fn delete_one(&self, report_id: &str) -> std::io::Result<bool> {
        let path = self.root.join(format!("{}.json", report_id));
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Clear every queued report.  Used by Settings → Diagnostics →
    /// "delete all".
    pub fn clear_all(&self) -> std::io::Result<usize> {
        if !self.root.exists() {
            return Ok(0);
        }
        let mut count = 0;
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if std::fs::remove_file(&path).is_ok() {
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Path of the queue root (for diagnostics + the user-visible
    /// "queue is at: ..." line in Settings).
    pub fn root(&self) -> &Path {
        &self.root
    }
}

// ── Panic hook ────────────────────────────────────────────────────

/// Install a panic hook that captures every panic into the queue.
///
/// **Off-by-default contract:** this only WRITES to the local queue.
/// Sending happens elsewhere via the Tauri command surface.
///
/// `app_version` is captured at install time (it's compile-time
/// constant for the binary).  `queue` is consumed: the hook owns it.
///
/// Returns `Err` if the hook is already installed; calling twice in
/// the same process is a logic bug.
pub fn install_panic_hook(
    queue: CrashQueue,
    app_version: String,
) -> Result<(), &'static str> {
    if HOOK_INSTALLED.set(()).is_err() {
        return Err("crash-capture panic hook already installed");
    }

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Always run the previous hook first so the tracing layer (and
        // stderr in dev) still print the panic.  Our work is purely
        // additive.
        prev_hook(panic_info);

        // Build the report.  Failures here MUST NOT panic again — that
        // would recurse into our own hook.  Use `let _ = ...` patterns
        // and log via tracing on best-effort.
        let report = build_report_from_panic(panic_info, &app_version);

        match queue.enqueue(&report) {
            Ok(path) => {
                tracing::warn!(
                    target: "booksforge::crash_capture",
                    report_id = %report.report_id,
                    path = %path.display(),
                    "queued crash report (off-by-default; user must Send to transmit)",
                );
            }
            Err(e) => {
                tracing::error!(
                    target: "booksforge::crash_capture",
                    error = %e,
                    "failed to enqueue crash report",
                );
            }
        }
    }));
    Ok(())
}

/// Build a `CrashReport` from a `PanicInfo`.  This function takes care
/// to avoid capturing the FORMATTED panic message — only the template
/// prefix and the location.
fn build_report_from_panic(
    panic_info: &std::panic::PanicHookInfo<'_>,
    app_version: &str,
) -> CrashReport {
    // Extract a privacy-safe message.  Rust's panic message is a
    // `Box<dyn Any + Send>` payload + a `&fmt::Arguments`; we choose
    // the safer of:
    //   - `panic_info.payload().downcast_ref::<&'static str>()` —
    //     literal panic strings (no interpolation), safe.
    //   - the location's file:line, which we want anyway.
    //
    // For panic!() with format args, we deliberately store
    // "<formatted panic message redacted — see local logs>" and let
    // the team grep the local tracing logs (which DO have the full
    // message but stay on disk).
    let payload = panic_info.payload();
    let panic_message_template = if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        // String payloads usually come from format!() — redact.
        let _ = s; // explicit drop reference; we deliberately don't read s
        "<formatted panic message redacted — see local logs>".to_string()
    } else {
        "<panic with non-string payload>".to_string()
    };

    let location = panic_info
        .location()
        .map(|l| (l.file().to_string(), l.line()))
        .unwrap_or_else(|| ("<unknown>".to_string(), 0));

    // Capture stack frames best-effort.  We intentionally do NOT
    // include argument values.
    let stack_frames = capture_stack_frames();

    CrashReport {
        schema_version: CrashReport::SCHEMA_VERSION,
        report_id: generate_ulid_id(),
        captured_at: chrono_now_rfc3339(),
        app_version: app_version.to_string(),
        os_family: detect_os_family(),
        os_version: detect_os_version(),
        arch: detect_arch(),
        kind: CrashKind::Panic,
        panic_message_template,
        stack_frames: stack_frames
            .into_iter()
            .chain(std::iter::once(StackFrame {
                symbol: Some("<panic site>".to_string()),
                file: Some(workspace_relative_path(&location.0)),
                line: Some(location.1),
            }))
            .collect(),
        project_open: false, // best-effort; the orchestrator can update via metadata once it owns the queue handle
        agent_running: AgentKind::None,
        elapsed_since_launch_ms: 0,
    }
}

fn capture_stack_frames() -> Vec<StackFrame> {
    // We deliberately use a small, allocation-friendly capture —
    // don't pull in `backtrace` crate macros that resolve every
    // possible symbol; for crash reports, the top ~16 frames are
    // enough.
    let mut out: Vec<StackFrame> = Vec::with_capacity(16);
    let bt = std::backtrace::Backtrace::capture();
    // The `Display` of Backtrace returns lines like
    //   "  0: booksforge_orchestrator::run::dispatch\n             at crates/.../run.rs:123"
    // We parse line-by-line.  Resolution is best-effort; if it
    // fails we just emit fewer frames.
    let s = format!("{bt}");
    let mut current_symbol: Option<String> = None;
    for line in s.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            let after_colon = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == ':');
            current_symbol = Some(after_colon.trim().to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("at ") {
            // Form: "<path>:<line>"
            if let Some((file, line_str)) = rest.rsplit_once(':') {
                if let Ok(line_no) = line_str.parse::<u32>() {
                    out.push(StackFrame {
                        symbol: current_symbol.take(),
                        file: Some(workspace_relative_path(file)),
                        line: Some(line_no),
                    });
                }
            }
        }
        if out.len() >= 16 {
            break;
        }
    }
    out
}

/// Strip absolute path prefixes that would leak the user's home
/// directory.  Returns a workspace-relative path if recognisable,
/// otherwise the raw path's last 3 segments.
fn workspace_relative_path(p: &str) -> String {
    if let Some(idx) = p.find("/booksforge/") {
        return p[idx + 1..].to_string();
    }
    if let Some(idx) = p.find("\\booksforge\\") {
        return p[idx + 1..].replace('\\', "/");
    }
    // Fallback: last 3 segments.
    let segments: Vec<&str> = p.split(['/', '\\']).collect();
    let take = segments.len().min(3);
    segments[segments.len() - take..].join("/")
}

fn generate_ulid_id() -> String {
    // ULID-ish: 10 chars timestamp + 16 chars random.  We don't pull
    // in the `ulid` crate just for this; a monotonic-by-time string
    // is sufficient for a local queue id.
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let ts_str = format!("{:010X}", ts);
    let rand_str = generate_rand_chars(16);
    format!("{ts_str}{rand_str}")
}

fn generate_rand_chars(n: usize) -> String {
    // Cheap pseudo-randomness from system clock nanos; we don't need
    // cryptographic randomness for a queue id (the ULID timestamp
    // prefix already orders them).
    let mut s = String::with_capacity(n);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let mut x = nanos.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for _ in 0..n {
        let v = (x % 36) as u32;
        let c = if v < 10 {
            char::from(b'0' + v as u8)
        } else {
            char::from(b'A' + (v - 10) as u8)
        };
        s.push(c);
        x = x.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
    }
    s
}

fn chrono_now_rfc3339() -> String {
    // The orchestrator already depends on chrono (via domain types).
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn detect_os_family() -> OsFamily {
    if cfg!(target_os = "macos") {
        OsFamily::MacOs
    } else if cfg!(target_os = "windows") {
        OsFamily::Windows
    } else if cfg!(target_os = "linux") {
        OsFamily::Linux
    } else {
        OsFamily::Unknown
    }
}

fn detect_os_version() -> String {
    // Best-effort.  On macOS we could call `sw_vers`; on Linux
    // `uname -r`; on Windows `cmd /c ver`.  For an opt-in crash
    // reporter, "compile-time target_os" is good enough — the
    // installed-OS version mostly matters for triage, and we have
    // the kind.
    format!("{}-runtime", std::env::consts::OS)
}

fn detect_arch() -> Arch {
    match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        _ => Arch::Unknown,
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_report() -> CrashReport {
        CrashReport {
            schema_version: CrashReport::SCHEMA_VERSION,
            report_id: "01HTESTREPORT00000000".to_string(),
            captured_at: "2026-05-09T00:00:00Z".to_string(),
            app_version: "0.0.1".to_string(),
            os_family: OsFamily::MacOs,
            os_version: "14.4.1".to_string(),
            arch: Arch::Aarch64,
            kind: CrashKind::Panic,
            panic_message_template: "kaboom".to_string(),
            stack_frames: vec![],
            project_open: true,
            agent_running: AgentKind::None,
            elapsed_since_launch_ms: 1234,
        }
    }

    #[test]
    fn enqueue_and_read_round_trip() {
        let dir = tempdir().expect("tempdir");
        let queue = CrashQueue::open(dir.path().to_path_buf()).expect("open");
        let report = sample_report();
        let path = queue.enqueue(&report).expect("enqueue");
        assert!(path.exists());
        let read = queue.read_one(&report.report_id).expect("read").expect("found");
        assert_eq!(read, report);
    }

    #[test]
    fn list_returns_oldest_first() {
        let dir = tempdir().expect("tempdir");
        let queue = CrashQueue::open(dir.path().to_path_buf()).expect("open");
        let mut a = sample_report();
        a.report_id = "01A".to_string();
        a.captured_at = "2026-05-09T00:00:01Z".to_string();
        let mut b = sample_report();
        b.report_id = "01B".to_string();
        b.captured_at = "2026-05-09T00:00:02Z".to_string();
        queue.enqueue(&b).unwrap();
        queue.enqueue(&a).unwrap();
        let listed = queue.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].report_id, "01A");
        assert_eq!(listed[1].report_id, "01B");
    }

    #[test]
    fn delete_one_is_idempotent() {
        let dir = tempdir().expect("tempdir");
        let queue = CrashQueue::open(dir.path().to_path_buf()).expect("open");
        let report = sample_report();
        queue.enqueue(&report).unwrap();
        assert!(queue.delete_one(&report.report_id).unwrap());
        assert!(!queue.delete_one(&report.report_id).unwrap());
    }

    #[test]
    fn clear_all_returns_count() {
        let dir = tempdir().expect("tempdir");
        let queue = CrashQueue::open(dir.path().to_path_buf()).expect("open");
        for i in 0..3 {
            let mut r = sample_report();
            r.report_id = format!("01TEST{i:02}");
            queue.enqueue(&r).unwrap();
        }
        assert_eq!(queue.clear_all().unwrap(), 3);
        assert_eq!(queue.list().unwrap().len(), 0);
    }

    #[test]
    fn corrupt_files_are_skipped_not_returned() {
        let dir = tempdir().expect("tempdir");
        let queue = CrashQueue::open(dir.path().to_path_buf()).expect("open");
        let report = sample_report();
        queue.enqueue(&report).unwrap();
        // Drop a garbage file alongside.
        std::fs::write(dir.path().join("garbage.json"), b"not-json").unwrap();
        let listed = queue.list().unwrap();
        assert_eq!(listed.len(), 1, "garbage file should be skipped");
        assert_eq!(listed[0].report_id, report.report_id);
    }

    #[test]
    fn workspace_relative_path_strips_user_home() {
        assert_eq!(
            workspace_relative_path("/Users/jane/code/booksforge/crates/foo/src/x.rs"),
            "booksforge/crates/foo/src/x.rs"
        );
        assert_eq!(
            workspace_relative_path("C:\\Users\\jane\\booksforge\\crates\\foo\\src\\x.rs"),
            "booksforge/crates/foo/src/x.rs"
        );
        // Fallback to last 3 segments.
        assert_eq!(workspace_relative_path("/some/random/path/elsewhere/file.rs"), "path/elsewhere/file.rs");
    }

    #[test]
    fn generated_ulid_id_is_well_formed() {
        let id = generate_ulid_id();
        assert!(id.len() >= 20);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn detect_arch_returns_known_value() {
        match detect_arch() {
            Arch::X86_64 | Arch::Aarch64 | Arch::Unknown => {}
        }
    }
}
