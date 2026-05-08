# BooksForge

Local-first desktop application for writers. Take an idea from outline
to publication-ready DOCX, PDF, and EPUB-3 with the help of a bounded
fleet of local-LLM agents running on Ollama. **Your manuscript never
leaves your device.**

## Requirements

| Tool | Version |
|------|---------|
| Rust | 1.82.0 (pinned via `rust-toolchain.toml`) |
| Node.js | 22.11.0 |
| pnpm | 9.12.3 |
| Tauri CLI | 2.x (`cargo tauri`) |
| Ollama | latest (`ollama serve` on `127.0.0.1:11434`) |

## Quick start

```bash
# 1 — Install frontend dependencies
cd booksforge && pnpm install --frozen-lockfile

# 2 — Run the desktop app in dev mode (starts Vite + Tauri)
cargo tauri dev

# 3 — Run all Rust tests
cargo test --workspace

# 4 — Regenerate TypeScript IPC bindings (after changing booksforge-ipc)
cargo test -p booksforge-ipc
```

A short architecture overview lives at [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
The canonical specs live under [`outputs/`](outputs/).

## Architecture (in 30 seconds)

BooksForge is a 4-layer application:

```
L1  Presentation       React/TypeScript/Vite/TipTap (apps/desktop/src-ui)
L2  App services       Tauri command handlers (apps/desktop/src)
L3  Domain             Pure-logic Rust crates, no I/O (crates/booksforge-{domain,template,...})
L4  Infrastructure     SQLite via sqlx, filesystem, Ollama HTTP, sidecars (crates/booksforge-{storage,fs,ollama,...})
```

`cargo deny check bans` mechanically enforces that L3 cannot import
L4. See [`CLAUDE.md`](CLAUDE.md) for the operating contract and
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the one-page
contributor onboarding view.

## Status & roadmap

The current ship-readiness gating list lives at [`EXTERNAL_AUDIT_BACKLOG.md`](EXTERNAL_AUDIT_BACKLOG.md).
The milestone roadmap lives at [`MILESTONES.md`](MILESTONES.md).
The team's engineering-task list lives at [`booksforge/BACKLOG.md`](booksforge/BACKLOG.md).

| Milestone | Status |
|-----------|--------|
| MZ-01 — Bootstrap workspace                                                | ✅ Shipped |
| MZ-02 — Project bundle creation and opening                                | ✅ Shipped |
| MZ-03 — Single-scene editor (autosave + crash recovery)                    | ✅ Shipped |
| MZ-04 — Ollama HTTP client + Setup Wizard                                  | ✅ Shipped |
| MZ-05 — Prompt template engine + Outline Architect                         | ✅ Shipped (PR #13) |
| MZ-06 — Snapshots v1 (manual + pre-agent-edit + scheduled hourly)          | ✅ Shipped (PR #13) |
| MZ-07 — Outline Architect → document-tree creation                         | ✅ Shipped (PR #13) |
| MZ-08 — Quick-action presets (Sharpen / Continue / Rephrase / Shorten / Expand) | ✅ Shipped (PR #13) |
| MZ-09 — Telemetry / logging / crash reports (opt-in)                       | 🟡 Tracing + diagnostic-bundle shipped; crash-capture pending merge of `feat/mz-09-and-polish` |
| MZ-10 — CI gates + reproducibility seed                                    | 🟡 Most gates shipped (clippy, fmt, deny, drift, repro, advisories, Dependabot); `pnpm-lock.yaml` not yet committed |
| Stabilisation Sprint S1                                                    | ✅ Shipped (PR #13 — committed the team's MZ-05+ slice) |
| M1 — Project & editor polish                                               | 🟡 ErrorBoundary + ToastProvider + ShortcutHelp + theme toggle UI + i18n scaffold wired; per-panel CRUD, selective restore, full a11y sweep, full string migration still open |
| M2 — First three agent workflows + Phase 5 (10 agents end-to-end)          | ✅ Shipped (PR #13) |
| M3 — Developmental + continuity                                            | ✅ Shipped (PR #13) |
| M4 — Templates + validators                                                | ✅ Shipped (PR #13) |
| M5 — Export pipeline (DOCX / PDF / EPUB-3 + reproducibility + visual-regression) | 🟡 Pipeline shipped; sidecar binary actual fetch + bundling pending merge of `feat/mz-09-and-polish` |
| M6 — MVP polish (release-readiness)                                        | ⬜ `release.yml` scaffold + auto-updater config + legal drafts pending merge of `chore/release-readiness` + `feat/mz-09-and-polish`; Apple Developer ID + Windows EV cert + final license + legal review + `booksforge.app` domain are founder/legal action |
| Public-beta gate                                                           | ⬜ All `CRITICAL`/`HIGH` audit items closed + cert provisioning + first signed `release.yml` dry-run |
| 1.0 GA gate                                                                | ⬜ Pricing decision (`docs/BUSINESS_MODEL.md`) + legal-counsel review + accessibility audit + remaining polish items |

## Privacy

- Ollama traffic stays on `127.0.0.1:11434` by default. A non-loopback
  host requires explicit user consent.
- AI features are **off per project** until you flip a one-time consent
  toggle. Default state on parse failure is "off", not "on".
- No telemetry, no analytics, no remote crash reporting in MVP.
- See [`PRIVACY_POLICY.md`](PRIVACY_POLICY.md) (user-facing) and
  [`outputs/SECURITY_PRIVACY.md`](outputs/SECURITY_PRIVACY.md)
  (technical reference).

## Governance & legal

| Topic | Doc |
|-------|-----|
| License | [`LICENSE`](LICENSE) *(provisional placeholder)* |
| Third-party licenses | [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md) |
| End-user licence agreement | [`EULA.md`](EULA.md) *(draft)* |
| Terms of service (online surfaces) | [`TERMS_OF_SERVICE.md`](TERMS_OF_SERVICE.md) *(draft)* |
| Privacy policy | [`PRIVACY_POLICY.md`](PRIVACY_POLICY.md) *(draft)* |
| Security policy | [`SECURITY.md`](SECURITY.md) |
| Code of conduct | [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) |
| Distribution & release plan | [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) |
| Operations runbook | [`docs/RUNBOOK.md`](docs/RUNBOOK.md) |

## Reporting issues

- **Security or privacy vulnerabilities:** see [`SECURITY.md`](SECURITY.md).
  **Do not** open a public issue for these.
- **Bugs and feature requests:** use the GitHub issue templates.
- **Open questions on spec / API:** add to [`docs/open-questions.md`](docs/open-questions.md).

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a PR.
The PR checklist (`cargo fmt`, `cargo clippy -D warnings`, `cargo
test`, `cargo deny check`, `pnpm typecheck`, no `unwrap()` in
production, layer boundaries, privacy invariants) is enforced by
CI on every PR. To run the same gates locally, install lefthook:

```bash
cargo install lefthook
lefthook install
```

## License

See [`LICENSE`](LICENSE). The placeholder will be replaced with
final terms before any public download is offered.
