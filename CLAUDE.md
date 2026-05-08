# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Current state (2026-05-08)

Code lives in [`booksforge/`](booksforge/) (Cargo + pnpm workspace). MZ-01 → MZ-04 plus the Phase 1–4 follow-ups have landed; further milestones (MZ-05 → MZ-10) are tracked in [`booksforge/BACKLOG.md`](booksforge/BACKLOG.md) and [`outputs/IMPLEMENTATION_PLAN.md`](outputs/IMPLEMENTATION_PLAN.md).

When working in this repository, the **operating contract for coding work is [`booksforge/CLAUDE.md`](booksforge/CLAUDE.md)** — read that one first. The file you are reading now (the root `CLAUDE.md`) is a thin map: it points to the spec archive in `outputs/` and the code in `booksforge/`, summarises the product, and lists the hard rules that apply across the whole repo.

The independent release-readiness audit lives in [`EXTERNAL_AUDIT_BACKLOG.md`](EXTERNAL_AUDIT_BACKLOG.md). Treat that file as the gating list before any external beta or public release.

---

## Product

**BooksForge** — a local-first desktop application that helps writers go from idea to publication-ready files (DOCX, PDF, EPUB-3) using a bounded swarm of local-LLM agents running on Ollama, with strong memory for full-book consistency.

**Targets:** macOS 13+ and Windows 10+ for MVP. Linux V1.0.

---

## Tech stack

| Layer | Technology |
|-------|-----------|
| App shell | Tauri `2.2.3` (Rust + WebView) |
| Frontend | React `18.3.x`, TypeScript `5.6.3`, Vite `5.4.x`, TipTap (ProseMirror) |
| Rust | `1.82.0` (Edition 2021, MSRV), Cargo workspace, `sqlx` (SQLite), `tokio`, `thiserror`, MiniJinja |
| JS runtime | Node.js `22.11.0` LTS, pnpm `9.12.3` |
| Local LLM | Ollama over `127.0.0.1:11434` |
| Export sidecars | Pandoc `3.5` (DOCX/PDF), `booksforge-export-epub` (Rust crate), EPUBCheck `5.1.0` |
| CI matrix | `macos-14`, `macos-13`, `windows-2022` (gating) + `ubuntu-22.04` (smoke) |

Exact pins and seed config files are in `outputs/TOOLCHAIN.md` (authoritative).

---

## Build commands (run from `booksforge/`)

```bash
# Rust
cargo build
cargo test
cargo clippy --all-targets -- -D warnings   # CI gate — must be clean
cargo fmt --check                           # CI gate
cargo deny check licenses                   # CI gate — rejects GPL-family crates

# TypeScript
pnpm typecheck    # CI gate
pnpm lint         # CI gate
pnpm test

# Run a single Rust test
cargo test -p <crate-name> <test_name>

# Tauri dev (hot-reload UI + Rust backend)
cargo tauri dev
```

---

## Architecture (four-layer, strictly enforced)

```
Layer 1 — Presentation   TypeScript / React / TipTap
Layer 2 — App services   Rust Tauri commands (IPC boundary)
Layer 3 — Domain         Pure-logic Rust crates (no I/O, no clocks)
Layer 4 — Infrastructure SQLite, filesystem, Ollama HTTP, sidecars
```

Cross-layer calls go through trait boundaries — this is what makes the agent orchestrator unit-testable without a live Ollama process. `cargo deny check bans` enforces that `booksforge-domain` cannot import `booksforge-storage`.

**Crate layout:** `booksforge-domain`, `booksforge-template`, `booksforge-validator`, `booksforge-agents`, `booksforge-prompt`, `booksforge-memory`, `booksforge-vocab`, `booksforge-export` (all L3); `booksforge-storage`, `booksforge-fs`, `booksforge-ollama`, `booksforge-orchestrator`, `booksforge-export-epub`, `booksforge-export-pandoc`, `booksforge-epubcheck` (all L4); `booksforge-ipc` (codegen → TS via `ts-rs`).

**Project bundle format:** `*.booksforge/` directory with `manifest.toml`, `project.db` (SQLite), `manuscript/` (Markdown mirror), `assets/`, `snapshots/`, `exports/`, `agent_runs/`.

---

## Hard coding rules

### Rust
- No `unwrap()`, `expect()`, or `panic!()` outside `#[cfg(test)]` and `main()`. Use typed `thiserror`-derived enums.
- All SQL via `sqlx::query!` / `query_as!` macros (compile-time checked). No string-interpolated SQL.
- Migrations are forward-only at runtime; append-only in the repo.

### TypeScript
- `strict: true`, `noImplicitAny`, `strictNullChecks`, `noUncheckedIndexedAccess` all on.
- Generated types from `booksforge-ipc` live in `packages/shared-types/` and are committed. CI fails on Rust↔TS drift.

### IPC
- Every Tauri command has typed input, typed output, tagged-union error.
- Long-running operations emit `progress` events keyed by `job_id`; a `cancel(job_id)` command aborts.

---

## Privacy invariants (every one has a CI test)

1. **No content leaves the device by default.** Outbound network only on `OllamaSetup → Install`, `Ollama.pull`, and opt-out `Update.check`.
2. No manuscript content is sent to a remote endpoint in MVP.
3. Ollama traffic stays on `127.0.0.1`. Non-loopback host requires explicit user consent.
4. AI capability is off per project until enabled with a one-time consent prompt.
5. No GPL crate statically linked — `cargo deny` enforces.

---

## Agent system constraints

- Agents are **prompt-in / schema-out** — no tools, no recursion in MVP.
- The Orchestrator is the only mutator; agents return proposals.
- Hard caps per workflow run: ≤8 calls, ≤10 min, ≤200k tokens, ≤3 retries.
- Pre-edit snapshot is mandatory before any accepted change.
- Audit ledger: `agent_runs`, `agent_tasks`, `agent_outputs`, `agent_applied_edits` tables in SQLite.

---

## Key spec documents (`outputs/`)

Load only what the task needs:

| Task area | Documents to load |
|-----------|------------------|
| First coding task | `outputs/IMPLEMENTATION_PLAN.md §3` (MZ-01) |
| Tool version pins | `outputs/TOOLCHAIN.md` |
| Agent work | `outputs/AGENTS.md`, `outputs/MEMORY_SYSTEM.md` |
| Schema / storage | `outputs/DATA_MODEL.md` |
| Export / EPUB | `outputs/EXPORT_EPUB_SPEC.md`, `outputs/EXPORT_EPUB_QA.md` |
| Editor / UI | `outputs/UI_UX_SPEC.md`, `outputs/ARCHITECTURE.md §3` |
| UI visual tokens | `outputs/DESIGN_SYSTEM.md` |
| Vocabulary | `outputs/VOCABULARY_DICTIONARIES.md` |
| Security | `outputs/SECURITY_PRIVACY.md` |
| Decisions | `outputs/ARCHITECTURE_DECISIONS.md`, `outputs/CONSISTENCY_MATRIX.md` |
| Full context map | `outputs/CLAUDE_CODE_CONTEXT_HARNESS.md` (read this first each session) |

The deep specs under `outputs/_deep/` are reference material. Where they conflict with the implementation pack, **the implementation pack wins for MVP** — see `outputs/CONSISTENCY_MATRIX.md`.

---

## PR shape

Every PR title: `[MZ-NN] short verb-phrase`. Description must include goal, files touched, tests added, risks, and `[ASSUMED]` notes for any undocumented choice. Documentation co-changes are required in the same PR. All CI gates must be green.
