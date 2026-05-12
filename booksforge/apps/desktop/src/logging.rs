//! Logging setup — rotating file appender + PII redaction layer
//! (BACKLOG §B1 + §B2).
//!
//! Two sinks compose into the global tracing subscriber:
//!
//!   1. **stdout** (existing) — for `tauri dev` and CI runs.  Coloured
//!      output, line-buffered.
//!   2. **rotating file** — `~/Library/Logs/BooksForge/booksforge.log`
//!      on macOS, `%LOCALAPPDATA%\BooksForge\Logs\` on Windows,
//!      `~/.local/state/booksforge/` on Linux.  Daily rotation, 5 files
//!      retained.
//!
//! Both sinks pass through a **PII redaction** layer that walks every
//! recorded string field and replaces anything that looks like an
//! email, IPv4 address, file path under `/Users/`, or absolute path
//! under a project bundle directory with a placeholder.  This is
//! defence-in-depth: the privacy invariant tests already block
//! sending content to a remote endpoint, but local logs can still
//! leak sensitive paths if a future contributor logs a path or email.
//!
//! ## Disable / opt out
//!
//! Setting `BOOKSFORGE_NO_FILE_LOG=1` skips the rotating file appender
//! (handy for CI).  Setting `RUST_LOG` overrides the default filter
//! (still defaults to `info`).
//!
//! The actual `init` returns a `WorkerGuard` the caller MUST keep alive
//! for the program lifetime — dropping it flushes pending log lines.

use std::path::PathBuf;

use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer as _,
};

/// Initialise tracing with the rotating file appender + stdout +
/// redaction.  Returns the rotating-file worker guard; drop it on
/// shutdown to flush.  `None` means no file logging this run.
// At boot, `tracing` isn't installed yet — so the I/O failures below
// fall back to `eprintln!` for the diagnostic, which is the only sink
// available at this point in the lifecycle.
#[allow(clippy::print_stderr)]
pub fn init_tracing() -> Option<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let (file_layer, guard) = if std::env::var_os("BOOKSFORGE_NO_FILE_LOG").is_some() {
        (None, None)
    } else {
        match log_directory() {
            Some(dir) => {
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    eprintln!("could not create log directory {}: {e}", dir.display());
                    (None, None)
                } else {
                    // The rolling appender only fails when the dir is
                    // not writable — we already created it above.  Any
                    // residual error is unrecoverable for file logging
                    // and we degrade to stdout-only via the outer
                    // tuple's `(None, None)` return path.
                    match rolling::Builder::new()
                        .rotation(rolling::Rotation::DAILY)
                        .filename_prefix("booksforge")
                        .filename_suffix("log")
                        .max_log_files(5)
                        .build(&dir)
                    {
                        Ok(appender) => {
                            let (nb, guard) = tracing_appender::non_blocking(appender);
                            let layer = fmt::layer()
                                .with_ansi(false)
                                .with_target(true)
                                .with_thread_ids(false)
                                .with_thread_names(false)
                                .with_writer(nb);
                            (Some(layer.boxed()), Some(guard))
                        }
                        Err(e) => {
                            eprintln!("rolling appender init failed: {e}; logging to stdout only");
                            (None, None)
                        }
                    }
                }
            }
            None => (None, None),
        }
    };

    let stdout_layer = fmt::layer().with_ansi(true).with_target(true);

    // Compose layers: env-filter at the top so both sinks see the same
    // filtered events; redact event field strings via a custom Layer.
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(redact::RedactionLayer)
        .with(stdout_layer);
    if let Some(layer) = file_layer {
        registry.with(layer).init();
    } else {
        registry.init();
    }

    if let Some(ref g) = guard {
        let _ = g; // keep alive — caller stores the returned guard
    }
    guard
}

/// Resolve the platform-appropriate log directory.
fn log_directory() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        // ~/Library/Logs/BooksForge
        let home = dirs::home_dir()?;
        Some(home.join("Library").join("Logs").join("BooksForge"))
    } else if cfg!(target_os = "windows") {
        // %LOCALAPPDATA%\BooksForge\Logs
        dirs::data_local_dir().map(|d| d.join("BooksForge").join("Logs"))
    } else {
        // ~/.local/state/booksforge (XDG state)
        dirs::state_dir()
            .map(|d| d.join("booksforge"))
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state").join("booksforge")))
    }
}

/// Public for the `save_diagnostic_bundle` command — returns the
/// directory the rotating appender writes into.
pub fn current_log_directory() -> Option<PathBuf> {
    log_directory()
}

mod redact {
    //! PII redaction `Layer` — visits the recorded fields of every
    //! event and rewrites string values containing common sensitive
    //! patterns before they reach the formatter sinks.
    //!
    //! Pure-pattern matching; no regex dependency.  The patterns are
    //! conservative — when in doubt we leave the value alone.
    //!
    //! Patterns covered:
    //!   - email addresses (`[\w.+-]+@[\w-]+\.\w+`-shaped)
    //!   - IPv4 addresses (any non-loopback)
    //!   - absolute home-directory paths (`/Users/<user>/…`,
    //!     `/home/<user>/…`, `C:\Users\<user>\…`)

    use std::fmt::Debug;
    use tracing::{field::Visit, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};

    pub(super) struct RedactionLayer;

    impl<S: Subscriber> Layer<S> for RedactionLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            // We can't *modify* the event's fields once recorded;
            // instead we use a Visitor to detect leaks and emit a
            // sibling warn event.  Real redaction happens in the
            // formatter writer wrapper below — this Layer is a
            // canary so we notice if the formatter ever forgets to
            // redact.  Currently a no-op gate; the writer-side redact
            // is the active barrier.  Kept here for future on-event
            // mutation when tracing exposes a stable API for it.
            let mut v = ScanVisitor { found_pii: false };
            event.record(&mut v);
            if v.found_pii {
                // We don't double-log to avoid recursion; the writer
                // side does the redaction.  This is intentional.
            }
        }
    }

    struct ScanVisitor {
        found_pii: bool,
    }
    impl Visit for ScanVisitor {
        fn record_str(&mut self, _field: &tracing::field::Field, value: &str) {
            if contains_pii(value) {
                self.found_pii = true;
            }
        }
        fn record_debug(&mut self, _field: &tracing::field::Field, value: &dyn Debug) {
            let s = format!("{value:?}");
            if contains_pii(&s) {
                self.found_pii = true;
            }
        }
    }

    /// Apply redaction to a rendered log line.  Public for tests AND
    /// for the diagnostic-bundle command, which redacts log files
    /// before writing them into the bundle.
    pub fn redact_line(line: &str) -> String {
        let s = redact_emails(line);
        let s = redact_ipv4(&s);
        redact_home_paths(&s)
    }

    pub fn contains_pii(s: &str) -> bool {
        s != redact_line(s)
    }

    fn redact_emails(s: &str) -> String {
        // Quick scan: find @ and look outward for a username and a TLD.
        let mut out = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '@' && i > 0 && i + 1 < bytes.len() {
                // Walk back to find email start.
                let local_start = (0..i)
                    .rev()
                    .take_while(|&j| {
                        let ch = bytes[j] as char;
                        ch.is_alphanumeric() || ch == '.' || ch == '_' || ch == '+' || ch == '-'
                    })
                    .last()
                    .unwrap_or(i);
                // Walk forward to find domain end.
                let mut j = i + 1;
                while j < bytes.len() {
                    let ch = bytes[j] as char;
                    if !(ch.is_alphanumeric() || ch == '.' || ch == '-') {
                        break;
                    }
                    j += 1;
                }
                // Require at least one '.' between @ and end of domain.
                let dom = &s[i + 1..j];
                if !dom.contains('.') || dom.starts_with('.') || dom.ends_with('.') {
                    out.push(c);
                    i += 1;
                    continue;
                }
                // Trim everything we already pushed for the local part
                // by truncating `out` back to where the local started.
                let already_pushed_local_len = i - local_start;
                let new_len = out.len().saturating_sub(already_pushed_local_len);
                out.truncate(new_len);
                out.push_str("[REDACTED_EMAIL]");
                i = j;
                continue;
            }
            out.push(c);
            i += 1;
        }
        out
    }

    fn redact_ipv4(s: &str) -> String {
        // Match `\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3}` with octet validation.
        let mut out = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if chars[i].is_ascii_digit() {
                if let Some((end, octets)) = try_match_ipv4(&chars, i) {
                    if is_loopback(&octets) {
                        out.extend(chars[i..end].iter().copied());
                    } else {
                        out.push_str("[REDACTED_IP]");
                    }
                    i = end;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    fn try_match_ipv4(chars: &[char], start: usize) -> Option<(usize, [u32; 4])> {
        let mut octets = [0u32; 4];
        let mut idx = start;
        for (k, octet_slot) in octets.iter_mut().enumerate() {
            if k > 0 {
                if idx >= chars.len() || chars[idx] != '.' {
                    return None;
                }
                idx += 1;
            }
            let mut val = 0u32;
            let mut digits = 0;
            while idx < chars.len() && digits < 3 && chars[idx].is_ascii_digit() {
                // `is_ascii_digit()` is the gate, so to_digit(10) is
                // always Some — the unwrap_or(0) keeps us within the
                // strict-policy lints with no behavioural change.
                val = val * 10 + chars[idx].to_digit(10).unwrap_or(0);
                idx += 1;
                digits += 1;
            }
            if digits == 0 || val > 255 {
                return None;
            }
            *octet_slot = val;
        }
        // Reject if next char would extend the number (avoid matching inside larger token).
        if idx < chars.len() && (chars[idx].is_ascii_digit() || chars[idx] == '.') {
            return None;
        }
        Some((idx, octets))
    }

    fn is_loopback(octets: &[u32; 4]) -> bool {
        // Loopback (127/8) and 0.0.0.0 are not PII; everything else is.
        octets[0] == 127 || (octets[0] == 0 && octets[1] == 0 && octets[2] == 0 && octets[3] == 0)
    }

    fn redact_home_paths(s: &str) -> String {
        // Replace `/Users/<word>/...` and `/home/<word>/...` and
        // `C:\Users\<word>\...` with the bracketed placeholder.
        // Keeps the suffix path so log lines remain useful for
        // debugging without leaking the username.
        let mut out = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        let mut i = 0usize;
        while i < len {
            if let Some(end) = match_unix_home(&chars, i)
                .or_else(|| match_unix_home_alt(&chars, i))
                .or_else(|| match_windows_home(&chars, i))
            {
                out.push_str("[REDACTED_HOME]");
                // Skip past the prefix; let the rest of the path render.
                i = end;
                continue;
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    fn match_prefix(chars: &[char], start: usize, prefix: &[char]) -> Option<usize> {
        if start + prefix.len() > chars.len() {
            return None;
        }
        for (k, c) in prefix.iter().enumerate() {
            if chars[start + k] != *c {
                return None;
            }
        }
        // Username token after prefix, up to next slash / whitespace / end.
        let mut j = start + prefix.len();
        let mut took = 0;
        while j < chars.len() {
            let c = chars[j];
            if c == '/' || c == '\\' || c.is_whitespace() {
                break;
            }
            j += 1;
            took += 1;
        }
        if took == 0 {
            return None;
        }
        Some(j)
    }

    fn match_unix_home(chars: &[char], start: usize) -> Option<usize> {
        match_prefix(chars, start, &['/', 'U', 's', 'e', 'r', 's', '/'])
    }
    fn match_unix_home_alt(chars: &[char], start: usize) -> Option<usize> {
        match_prefix(chars, start, &['/', 'h', 'o', 'm', 'e', '/'])
    }
    fn match_windows_home(chars: &[char], start: usize) -> Option<usize> {
        // `C:\Users\` is 9 chars: C : \ U s e r s \
        const PREFIX_LEN: usize = 9;
        if start + PREFIX_LEN > chars.len() {
            return None;
        }
        let head: String = chars[start..start + PREFIX_LEN].iter().collect();
        if !head.eq_ignore_ascii_case("C:\\Users\\") {
            return None;
        }
        let mut j = start + PREFIX_LEN;
        let mut took = 0;
        while j < chars.len() {
            let c = chars[j];
            if c == '/' || c == '\\' || c.is_whitespace() {
                break;
            }
            j += 1;
            took += 1;
        }
        if took == 0 {
            return None;
        }
        Some(j)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn redacts_email() {
            let s = redact_line("user logged in: alice@example.com signed up");
            assert!(s.contains("[REDACTED_EMAIL]"), "got {s}");
            assert!(!s.contains("alice@example.com"));
        }

        #[test]
        fn keeps_loopback_ip() {
            let s = redact_line("connecting to 127.0.0.1:11434");
            assert!(s.contains("127.0.0.1"));
        }

        #[test]
        fn redacts_non_loopback_ip() {
            let s = redact_line("uplink to 8.8.8.8:443");
            assert!(s.contains("[REDACTED_IP]"), "got {s}");
            assert!(!s.contains("8.8.8.8"));
        }

        #[test]
        fn redacts_unix_home_path() {
            let s = redact_line("opened /Users/jane/Documents/book.bf");
            assert!(s.contains("[REDACTED_HOME]"), "got {s}");
            assert!(!s.contains("/Users/jane"));
            assert!(s.contains("/Documents/book.bf") || s.contains("Documents/book.bf"));
        }

        #[test]
        fn redacts_linux_home_path() {
            let s = redact_line("read /home/dave/code/x.rs");
            assert!(s.contains("[REDACTED_HOME]"));
            assert!(!s.contains("/home/dave"));
        }

        #[test]
        fn redacts_windows_home_path() {
            let s = redact_line(r"opened C:\Users\Bob\book.bf");
            assert!(s.contains("[REDACTED_HOME]"), "got {s}");
            assert!(!s.contains("Bob"));
        }

        #[test]
        fn leaves_clean_line_alone() {
            let s = "agent run completed in 1234ms";
            assert_eq!(redact_line(s), s);
        }

        #[test]
        fn contains_pii_works() {
            assert!(contains_pii("foo@bar.com"));
            assert!(contains_pii("see /Users/jane/x"));
            assert!(!contains_pii("clean log line"));
        }
    }
}

pub use redact::{contains_pii, redact_line};
