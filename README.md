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
| MZ-01 — Bootstrap workspace | ✅ Shipped |
| MZ-02 — Project bundle creation and opening | ✅ Shipped |
| MZ-03 — Single-scene editor (autosave + crash recovery) | ✅ Shipped |
| MZ-04 — Ollama HTTP client + Setup Wizard | ✅ Shipped |
| MZ-05 → MZ-09 partial + Phase 1–6 | 🟡 Landed (uncommitted in working tree) |
| Stabilisation Sprint | 🟡 Pending — split working tree into milestone commits |
| Public-beta gate | ⬜ All `CRITICAL`/`HIGH` audit items closed |

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
