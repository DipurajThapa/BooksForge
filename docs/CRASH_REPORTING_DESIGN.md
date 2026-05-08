# BooksForge — Crash Reporting Design

> **Status: design only.** The Rust + React implementation is the
> team's work to land in MZ-09 (Telemetry, logging, crash reports).
> This document fixes the design choices before code goes in, so
> we don't accidentally violate a privacy invariant.
>
> *Refs:* `EXTERNAL_AUDIT_BACKLOG.md #43`,
> `outputs/SECURITY_PRIVACY.md`, `MILESTONES.md M6.G`,
> `booksforge/BACKLOG.md §B1–B5`.

---

## 1. Non-negotiable principles

The product's defining promise is **no manuscript content leaves
the device by default.** Crash reporting is the most likely place to
accidentally violate this. Therefore:

| Principle | Rule |
|-----------|------|
| **Off by default** | No crash report is generated, sent, or even *queued* unless the user has explicitly opted in via *Settings → Diagnostics → Send crash reports*. |
| **Explicit per-event consent (V1)** | Even with the "send crash reports" toggle ON, every individual crash produces a *queued* report that the user reviews and explicitly sends. No auto-send. The toggle on its own only enables the queue. |
| **Local-first review** | The user can open the queued report in plain text and see exactly what would be sent before it is sent. |
| **No third-party SaaS** | No Sentry, Bugsnag, Rollbar, etc. The sink is self-hosted, behind our own domain (`crash.booksforge.app` or similar), under our own legal terms. |
| **No identifying data** | Reports never carry: project IDs, scene IDs, file paths inside the bundle, user names, machine names, IP-derived identifiers, persistent installation IDs. |
| **Manuscript redaction is mechanical, not heuristic** | Redaction is the *exclusion of all manuscript-touching types from the report's serialised form*, not a regex sweep over a free-form blob. |
| **Reproducible offline** | The diagnostic bundle command (already shipped per `booksforge/BACKLOG.md §B3`) is the same data the user can inspect; the crash-report flow reuses it. |

---

## 2. Architecture

```
panic / Result::Err in production
            │
            ▼
┌───────────────────────────────────────────────────────────┐
│ 1. Crash hook captures stack frames + thread ID           │
│    (no captured locals, no captured arguments)            │
└───────────────────────┬───────────────────────────────────┘
                        ▼
┌───────────────────────────────────────────────────────────┐
│ 2. CrashReport built from a *typed* allowlist of fields:  │
│      app_version, os_family, arch, panic_message_template │
│      (NOT panic_message — we strip the formatted args),   │
│      symbolicated stack frames (file:line, no values)     │
└───────────────────────┬───────────────────────────────────┘
                        ▼
┌───────────────────────────────────────────────────────────┐
│ 3. Report queued at ~/.booksforge/crash-reports/          │
│    <ulid>.json. Stays local indefinitely until the user   │
│    sends or deletes it.                                   │
└───────────────────────┬───────────────────────────────────┘
                        ▼
┌───────────────────────────────────────────────────────────┐
│ 4. UI: "We crashed. View report?"                          │
│    Shows the JSON inline. Two buttons: Send / Delete.     │
└───────────────────────┬───────────────────────────────────┘
                        ▼
┌───────────────────────────────────────────────────────────┐
│ 5. POST https://crash.booksforge.app/v1/report            │
│    HTTPS, Content-Type: application/json, no cookies,     │
│    no auth header, no User-Agent containing app version.  │
└───────────────────────────────────────────────────────────┘
```

---

## 3. The `CrashReport` schema (allowlist)

```rust
// crates/booksforge-domain/src/crash_report.rs (proposed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub schema_version: u32,            // = 1
    pub report_id: Ulid,                // local-only; not sent until user clicks Send
    pub captured_at: DateTime<Utc>,

    // App + environment.
    pub app_version: String,            // "0.1.0"
    pub os_family: OsFamily,            // MacOS | Windows | Linux
    pub os_version: String,             // "14.4.1"
    pub arch: Arch,                     // X86_64 | Aarch64

    // Crash itself.
    pub kind: CrashKind,                // Panic | UncaughtTokio | OllamaConnection | Sqlx | Export
    pub panic_message_template: String, // "internal error: assertion failed: {}"
                                        // — the *template*, not the formatted output.
                                        //   Rust's std::panic carries this separately.
    pub stack_frames: Vec<StackFrame>,  // file:line + symbol name only

    // *Derived* counters that don't carry identifying data.
    pub project_open: bool,             // a project was open at crash time
    pub agent_running: Option<AgentKind>,  // generic agent kind (e.g. Copyedit), not run id
    pub elapsed_since_launch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub symbol: Option<String>,         // "booksforge_orchestrator::run::dispatch"
    pub file: Option<PathBuf>,          // relative path like "crates/booksforge-orchestrator/src/run.rs"
    pub line: Option<u32>,
    // NO captured argument values.  NO captured locals.
}
```

**What is NOT in the schema** (mechanical exclusion via the type
system, not a redaction sweep):

- `Manuscript`, `SceneContent`, `Outline`, `Brief`, any TipTap
  document types — these types simply do not appear in
  `CrashReport`'s definition.
- `ProjectId`, `BundlePath` — not in schema.
- `UserId` / installation-id — not generated, not stored.
- `IpAddr` — never captured.
- Process-wide environment variables — never captured.

A new contributor cannot accidentally add manuscript content because
the type doesn't have a slot for it.

---

## 4. The submission endpoint

| Property | Value |
|---|---|
| Host | `crash.booksforge.app` (founder-controlled) |
| Path | `/v1/report` |
| Method | `POST` |
| Body | The `CrashReport` JSON above. |
| TLS | Required. Pinned via Tauri-side cert verification. |
| Cookies | None sent. |
| Auth | None — anonymous submission. |
| Rate limit | 10 req/min/IP at the edge. Excess returns 429. |
| Response | `200 {ok: true, server_id: "..."}` or `429`/`5xx` errors with a clean message. |
| Server retention | 90 days, then automated deletion. |
| Server access | Founder + named maintainers only. |

The server is intentionally trivial: a small Rust service behind
Cloudflare that writes incoming reports to a private S3-compatible
bucket and exposes a simple read endpoint to the maintainer dashboard.
**No third-party error-tracking SaaS is in the path.**

---

## 5. Privacy invariant tests this design must pass

When the implementation lands, these tests must be added:

| Test | Asserts |
|------|---------|
| `crash_report_struct_does_not_carry_manuscript_types` | A compile-time test that uses `static_assertions::assert_not_impl_any!` to forbid `CrashReport` from containing any field whose type derives from `Manuscript` / `SceneContent` / `Outline` / `Brief` / TipTap document types. |
| `crash_report_disabled_by_default` | A fresh project / fresh install does not generate a queued report when a panic is induced. |
| `crash_report_no_send_without_explicit_click` | With "send crash reports" toggle ON, panic generates a queued report but does NOT issue an HTTP POST until the user clicks Send. |
| `crash_report_queue_persists_across_relaunches` | Queue survives `kill -9`. |
| `crash_report_endpoint_pinned` | The POST URL is `https://crash.booksforge.app/v1/report` and nothing else. Setting `OLLAMA_HOST` or any other env var does not redirect the crash sink. |
| `crash_report_redaction_review` | The user can preview the exact JSON that would be sent, and the preview matches the actual POST body bit-for-bit. |

---

## 6. UX flow (V1)

### 6.1 Toggle (Settings → Diagnostics)

```
[ ] Send crash reports (off by default)

When enabled, BooksForge will queue a report whenever it
crashes.  You will be asked to review and send each report
individually — nothing is sent automatically.

What's in a report:
  • The app version, your OS family + version, and CPU architecture.
  • The crash type and stack frames (file:line, function names).
  • Whether a project was open and which agent was running, if any.

What's NOT in a report (verified by automated tests):
  • Your manuscript content.
  • Project file paths.
  • Your name, email, IP address, or any persistent identifier.

[ Open queued reports... ]   (shows the local queue)
```

### 6.2 Per-report preview

```
We crashed.  A report is queued at:
  ~/.booksforge/crash-reports/01HXXXX...json

[ Preview ]   [ Send ]   [ Delete ]   [ Send & Delete ]

Preview opens the JSON in a read-only modal so the user can verify
exactly what would be sent.
```

---

## 7. Implementation checklist (for whoever picks up MZ-09's
crash-report subtask)

- [ ] Add `crates/booksforge-domain/src/crash_report.rs` with the
      schema in §3.
- [ ] Add `crates/booksforge-orchestrator/src/crash_capture.rs`
      with `std::panic::set_hook` integration that builds a
      `CrashReport`. **No `Manuscript` access from this module.**
- [ ] Persistence layer: write queued reports to
      `~/.booksforge/crash-reports/<ulid>.json` atomically.
- [ ] Tauri commands: `crash.list_queued`, `crash.preview`,
      `crash.send`, `crash.delete`.
- [ ] React UI: Settings panel toggle + dialog preview component.
- [ ] All six privacy invariant tests in §5.
- [ ] Server: a thin Rust service for `crash.booksforge.app`.
      Provisioning is in `docs/DISTRIBUTION.md` scope.
- [ ] Update `PRIVACY_POLICY.md §1.1` once the toggle ships.

---

## 8. What this design deliberately does NOT include

- **Stack-frame argument values.** Many crash reporters capture
  `format!`-style argument values; we do not, because those values
  often contain manuscript-derived content.
- **A persistent installation ID.** Useful for de-duplicating crash
  reports across users; replaces nothing the maintainer cannot do
  via panic-template clustering on the server side. Not worth the
  privacy cost.
- **Background submission.** Even with the user's consent, no
  report is sent without an explicit click. Users on metered
  connections see no surprise traffic.
- **Pre-flight crash analytics.** No "n users had this crash this
  week" dashboard in the app. That belongs server-side, viewable
  by maintainers only.

These are deliberate limitations. They make the system less useful
to the maintainer than a typical crash reporter and that's the
point — the privacy invariants come first.

---

*Last updated 2026-05-08 (initial design).*
