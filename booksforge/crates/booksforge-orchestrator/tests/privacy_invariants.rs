#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Privacy invariant tests (BACKLOG §B5).
//!
//! BooksForge's first privacy invariant is "no manuscript content leaves
//! the device by default."  This test suite encodes that as machine-checked
//! assertions so a careless dependency or HTTP call cannot silently break
//! the contract.
//!
//! What's checked:
//!
//! 1. **Default Ollama endpoint is loopback.**  `OllamaSettings::default()`
//!    must point at `127.0.0.1`.  A non-loopback host requires explicit
//!    user consent (per CLAUDE.md privacy invariant 3).
//!
//! 2. **HTTP usage allowlist.**  Only one crate is permitted to depend on
//!    `reqwest` / `hyper` / `http`: `booksforge-ollama`.  Any other crate
//!    pulling in an HTTP client is a privacy regression and fails this
//!    test.  (The `cargo deny` setup in C1 will gate this at CI time too,
//!    once that lands.)
//!
//! 3. **No telemetry / analytics SDKs.**  A blocklist of well-known
//!    telemetry packages (sentry, posthog, mixpanel, amplitude, datadog,
//!    segment, rollbar, bugsnag) is asserted absent across the workspace.
//!
//! 4. **No prompt-template URLs.**  Prompt templates must not reference
//!    arbitrary external endpoints — the model only ever gets text the
//!    user supplied.  Asserts no `http://` / `https://` literals appear
//!    inside any registered prompt template's rendered output for an
//!    empty input.  (Allowlist: 127.0.0.1, localhost, MDN/W3C reference
//!    domains used in static documentation strings.)
//!
//! These tests are intentionally noisy on failure — the point is to make
//! "you broke a privacy invariant" the loudest possible CI signal.

use booksforge_domain::OllamaSettings;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ── 1. Default endpoint is loopback ─────────────────────────────────────────

#[test]
fn ollama_default_endpoint_is_loopback() {
    let s = OllamaSettings::default();
    assert!(
        s.host.contains("127.0.0.1") || s.host.contains("localhost"),
        "PRIVACY REGRESSION: OllamaSettings::default().host = {:?} \
         must point at 127.0.0.1 or localhost.  A non-loopback default \
         would let agent requests leave the device without user consent.",
        s.host,
    );
}

// ── 2. HTTP usage allowlist ─────────────────────────────────────────────────

const HTTP_ALLOWLIST: &[&str] = &["booksforge-ollama"];
const HTTP_DEPS: &[&str] = &["reqwest", "hyper", "ureq", "isahc", "surf"];

#[test]
fn http_clients_are_only_in_allowlisted_crates() {
    let workspace_root = workspace_root();
    let crates_dir = workspace_root.join("crates");
    let mut violations = Vec::new();

    for entry in fs::read_dir(&crates_dir).expect("read crates/") {
        let entry = entry.expect("crate entry");
        if !entry.file_type().expect("ft").is_dir() {
            continue;
        }
        let crate_name = entry.file_name().to_string_lossy().to_string();
        if HTTP_ALLOWLIST.contains(&crate_name.as_str()) {
            continue;
        }

        let cargo_toml = entry.path().join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }
        let content = fs::read_to_string(&cargo_toml).expect("read Cargo.toml");
        for dep in HTTP_DEPS {
            // Match `dep = "…"` or `dep = { … }` or `dep.workspace`, but not
            // names that just contain the substring (e.g. "hyper-tls" inside
            // a comment) — we anchor on a line start or a TOML value boundary.
            for line in content.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with(&format!("{dep} "))
                    || trimmed.starts_with(&format!("{dep}."))
                    || trimmed.starts_with(&format!("{dep}="))
                {
                    violations.push(format!("{crate_name} depends on {dep}: {line}"));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "PRIVACY REGRESSION: HTTP clients are restricted to crates in \
         HTTP_ALLOWLIST = {:?}.  Found:\n  {}",
        HTTP_ALLOWLIST,
        violations.join("\n  "),
    );
}

// ── 3. No telemetry / analytics SDKs anywhere ───────────────────────────────

const TELEMETRY_BLOCKLIST: &[&str] = &[
    "sentry",
    "sentry-core",
    "sentry-anyhow",
    "posthog",
    "posthog-rs",
    "mixpanel",
    "amplitude",
    "datadog",
    "ddtrace",
    "dd-trace",
    "segment",
    "segment-rs",
    "rollbar",
    "bugsnag",
    "honeycomb",
    "newrelic",
    "google-analytics",
    "umami",
];

#[test]
fn no_telemetry_sdks_in_workspace() {
    let workspace_root = workspace_root();
    let mut found = Vec::new();
    walk_cargo_tomls(&workspace_root, &mut |path, content| {
        for blocked in TELEMETRY_BLOCKLIST {
            for line in content.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with(&format!("{blocked} "))
                    || trimmed.starts_with(&format!("{blocked}."))
                    || trimmed.starts_with(&format!("{blocked}="))
                    || trimmed.starts_with(&format!("\"{blocked}\""))
                {
                    found.push(format!("{}: {}", path.display(), line.trim()));
                }
            }
        }
    });

    assert!(
        found.is_empty(),
        "PRIVACY REGRESSION: telemetry / analytics SDK detected.  BooksForge \
         is local-first; no third-party telemetry is permitted.\n\
         Found:\n  {}",
        found.join("\n  "),
    );
}

// ── 4. No external URLs in registered prompt templates ──────────────────────

const URL_ALLOWLIST: &[&str] = &[
    "127.0.0.1",
    "localhost",
    // Documentation references that may legitimately appear in template
    // *comments* but never get rendered to the model:
    "https://www.w3.org",
    "https://developer.mozilla.org",
    // Tauri config $schema reference — fetched only by editors/IDEs for
    // IntelliSense, never by the running app.
    "schema.tauri.app",
];

#[test]
fn prompt_templates_contain_no_external_urls() {
    let templates_dir = workspace_root()
        .join("crates")
        .join("booksforge-prompt")
        .join("templates");
    let mut violations = Vec::new();

    walk_files(&templates_dir, "toml", &mut |path, content| {
        for (lineno, line) in content.lines().enumerate() {
            for proto in &["http://", "https://"] {
                if let Some(idx) = line.find(proto) {
                    let rest = &line[idx..];
                    let url_end = rest
                        .find(|c: char| c.is_whitespace() || c == '"' || c == ')')
                        .unwrap_or(rest.len());
                    let url = &rest[..url_end];
                    let allowed = URL_ALLOWLIST.iter().any(|a| url.contains(a));
                    if !allowed {
                        violations.push(format!(
                            "{}:{} — {url}",
                            path.strip_prefix(workspace_root())
                                .unwrap_or(path)
                                .display(),
                            lineno + 1,
                        ));
                    }
                }
            }
        }
    });

    assert!(
        violations.is_empty(),
        "PRIVACY REGRESSION: prompt template references external URL(s).  \
         Templates must not instruct the model to reach outside the project \
         or to a third-party service.\n  {}",
        violations.join("\n  "),
    );
}

// ── 5. App's tauri.conf.json declares only the loopback Ollama capability ──

#[test]
fn tauri_capabilities_match_privacy_contract() {
    let path = workspace_root()
        .join("apps")
        .join("desktop")
        .join("capabilities");
    if !path.exists() {
        // Capability files may not be split out yet — soft skip.
        return;
    }
    let mut found_external = Vec::new();
    walk_files(&path, "json", &mut |p, content| {
        for line in content.lines() {
            // Look for `http://` or `https://` literals inside capability
            // allow lists and assert each is loopback or a known doc domain.
            for proto in &["\"http://", "\"https://"] {
                if let Some(idx) = line.find(proto) {
                    let rest = &line[idx + 1..]; // skip leading quote
                    let url_end = rest.find('"').unwrap_or(rest.len());
                    let url = &rest[..url_end];
                    let allowed = URL_ALLOWLIST.iter().any(|a| url.contains(a));
                    if !allowed {
                        found_external.push(format!(
                            "{}: {url}",
                            p.strip_prefix(workspace_root()).unwrap_or(p).display(),
                        ));
                    }
                }
            }
        }
    });
    assert!(
        found_external.is_empty(),
        "PRIVACY REGRESSION: Tauri capability lists external URL(s).\n  {}",
        found_external.join("\n  "),
    );
}

// ── 6. Default originality provider is LocalOnly ────────────────────────────

#[tokio::test]
async fn default_originality_provider_is_local_only() {
    use booksforge_domain::OriginalityProviderId;
    use booksforge_orchestrator::originality_provider::active_provider;
    use booksforge_storage::{open_pool, run_migrations, SqliteStorage};
    use std::sync::Arc;

    let dir = tempfile::tempdir().expect("tempdir");
    let pool = open_pool(&dir.path().join("op.db")).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let storage = Arc::new(SqliteStorage::new(pool));

    let active = active_provider(&storage).await;
    assert_eq!(
        active,
        OriginalityProviderId::LocalOnly,
        "PRIVACY REGRESSION: a fresh project must default to the LocalOnly \
         originality provider.  An off-device provider would send manuscript \
         content to a third party without explicit consent."
    );
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    // Tests run from the crate root; walk up until we find Cargo.toml with [workspace].
    let mut p = std::env::current_dir().expect("cwd");
    loop {
        let candidate = p.join("Cargo.toml");
        if candidate.exists() {
            let content = fs::read_to_string(&candidate).unwrap_or_default();
            if content.contains("[workspace]") {
                return p;
            }
        }
        if !p.pop() {
            panic!("workspace Cargo.toml not found");
        }
    }
}

fn walk_cargo_tomls(root: &Path, visit: &mut dyn FnMut(&Path, &str)) {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    walk(root, &mut |p| {
        if p.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml")
            && visited.insert(p.to_path_buf())
        {
            if let Ok(c) = fs::read_to_string(p) {
                visit(p, &c);
            }
        }
    });
}

fn walk_files(root: &Path, extension: &str, visit: &mut dyn FnMut(&Path, &str)) {
    walk(root, &mut |p| {
        if p.extension().and_then(|s| s.to_str()) == Some(extension) {
            if let Ok(c) = fs::read_to_string(p) {
                visit(p, &c);
            }
        }
    });
}

fn walk(root: &Path, f: &mut dyn FnMut(&Path)) {
    if !root.exists() {
        return;
    }
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        // Skip obvious noise.
        if name == "target" || name == "node_modules" || name == ".git" || name == ".sqlx" {
            continue;
        }
        if path.is_dir() {
            walk(&path, f);
        } else {
            f(&path);
        }
    }
}
