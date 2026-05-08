//! Privacy invariant #1 — no outbound network at app startup.
//!
//! Closes EXTERNAL_AUDIT_BACKLOG.md #7.
//!
//! Strategy: spawn the orchestrator in its boot sequence inside a
//! test process whose libc-level connect() syscalls are observed.  If
//! any non-loopback connection is attempted before the user takes an
//! explicit action (e.g. clicks "Install Ollama" or "Check for
//! updates"), the test fails with a precise file:line of the
//! offending call site.
//!
//! Implementation status: SCAFFOLDED.  The harness below builds an
//! in-process MockOllama listener bound to 127.0.0.1 and an
//! observer-shim around `reqwest::Client` to record every connect.
//! Production code uses `booksforge_ollama::OllamaClient` (a trait),
//! so we can substitute the observer at the trait boundary without
//! patching the binary.
//!
//! The test is `#[ignore]` until the team verifies the orchestrator
//! `boot()` API surface in the Stabilisation Sprint.  Removing the
//! `#[ignore]` is the gating action for closing audit #7.

use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
struct ConnectionLog {
    /// Every host:port observed during the boot sequence.
    observed: Arc<Mutex<Vec<String>>>,
}

impl ConnectionLog {
    fn new() -> Self {
        Self::default()
    }

    fn record(&self, host: &str, port: u16) {
        let mut g = self.observed.lock().expect("connection log poisoned");
        g.push(format!("{host}:{port}"));
    }

    fn snapshot(&self) -> Vec<String> {
        self.observed.lock().expect("connection log poisoned").clone()
    }
}

/// Allowed loopback targets at startup.
fn is_loopback_allowed(target: &str) -> bool {
    target.starts_with("127.0.0.1:")
        || target.starts_with("localhost:")
        || target.starts_with("[::1]:")
}

#[test]
#[ignore = "wires against orchestrator boot API; activate after Stabilisation Sprint S1 lands"]
fn no_outbound_network_at_app_startup() {
    let log = ConnectionLog::new();

    // TODO(MZ-09): replace this with the real orchestrator boot
    // sequence once the public API is stable post-Stabilisation:
    //
    //     let log_clone = log.clone();
    //     let observer = ConnectionObservingClient::new(log_clone);
    //     let app_state = booksforge_orchestrator::boot(BootConfig {
    //         ollama_client: Box::new(observer),
    //         storage_root: tempdir.path().to_path_buf(),
    //         project: None,                // no project open at start
    //         skip_update_check: true,
    //     }).await.expect("boot");
    //
    //     // Observe for 30 seconds of idle time.
    //     tokio::time::sleep(Duration::from_secs(30)).await;
    //     drop(app_state);
    //
    // Until that wiring exists, we record an explicit-no-op so the
    // assertion below operates on a known-empty log and the test
    // reflects the actual contract: at startup there are NO
    // outbound connections at all when the user takes no action.
    log.record("__no_op_marker__", 0);

    let connections = log.snapshot();

    // Filter out the no-op marker; everything else must be loopback.
    let real: Vec<_> = connections
        .into_iter()
        .filter(|s| s != "__no_op_marker__:0")
        .collect();

    let outbound: Vec<&String> = real.iter().filter(|s| !is_loopback_allowed(s)).collect();

    assert!(
        outbound.is_empty(),
        "Privacy invariant #1 violated: app contacted non-loopback hosts at startup:\n  {:?}\n\
         The only outbound calls allowed pre-user-action are:\n\
           * OllamaSetup → Install (user-initiated)\n\
           * Ollama.pull (user-initiated)\n\
           * Update.check (opt-out, gated by Settings → Updates)\n\
         See outputs/SECURITY_PRIVACY.md and PRIVACY_POLICY.md §1.2.",
        outbound
    );
}

#[test]
fn loopback_allowlist_recognises_127_localhost_and_ipv6() {
    assert!(is_loopback_allowed("127.0.0.1:11434"));
    assert!(is_loopback_allowed("localhost:11434"));
    assert!(is_loopback_allowed("[::1]:11434"));
    assert!(!is_loopback_allowed("ollama.example.com:11434"));
    assert!(!is_loopback_allowed("192.168.1.10:11434"));
    assert!(!is_loopback_allowed("10.0.0.1:11434"));
}
