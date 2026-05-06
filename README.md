# BooksForge

Local-first desktop application for writers — produces DOCX, PDF, and EPUB-3 from local-LLM agent workflows.  No content leaves your device.

## Requirements

| Tool | Version |
|------|---------|
| Rust | 1.82.0 (pinned via `rust-toolchain.toml`) |
| Node.js | 22.11.0 |
| pnpm | 9.12.3 |
| Tauri CLI | 2.x (`cargo tauri`) |
| Ollama | latest (`ollama serve` on 127.0.0.1:11434) |

## Quick start

```bash
# 1 — Install frontend dependencies
cd booksforge && pnpm install

# 2 — Run the desktop app in dev mode (starts Vite + Tauri)
pnpm dev

# 3 — Run all Rust tests
cargo test --workspace

# 4 — Regenerate TypeScript IPC bindings (after changing booksforge-ipc)
cargo test -p booksforge-ipc
```

## Architecture

BooksForge is a 4-layer application:

```
Layer 1  React/TypeScript UI (apps/desktop/src-ui)
Layer 2  Tauri commands / app services (apps/desktop/src)
Layer 3  Pure Rust domain logic — no I/O (crates/booksforge-*)
Layer 4  Infrastructure — SQLite, filesystem, Ollama, sidecars
```

See [`CLAUDE.md`](CLAUDE.md) for the full operating contract and [`outputs/ARCHITECTURE.md`](outputs/ARCHITECTURE.md) for the detailed design.

## Milestones

| Milestone | Status | Description |
|-----------|--------|-------------|
| MZ-01 | In progress | Workspace scaffold, all crates, CI |
| MZ-02 | Planned | Project creation, node tree, SQLite persistence |
| MZ-03 | Planned | Book intake agent, outline-architect agent |
| MZ-04 | Planned | Chapter drafter, dev-editor, ProseMirror editor |
| MZ-05 | Planned | EPUB-3 export, PDF/DOCX via Pandoc |

## Privacy

Ollama traffic is bound to `127.0.0.1:11434`.  No manuscript content is sent to any remote server.
