# Architecture Decisions — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative ADR index for the implementation pack.** Companion to `_deep/13-glossary-and-decision-log.md` (the deep ADR log), which remains the **append-only source of truth**. This file is a curated, implementation-pack-relevant index of the ADRs that govern the MVP build.

---

## How to use this file

When in doubt about whether a choice is settled, check this file first. If it's listed and Status is `Confirmed` or `Confirmed for MVP`, proceed. If it's not listed, check the deep ADR log. If still silent, follow the **Tough Decision Protocol** in `CLAUDE.md §14` and append a new ADR.

Every ADR has a stable id (D-NNN). Once issued, ids are never reused or renumbered. A superseded ADR keeps its id and links to the revision.

---

## D-001 — Pricing: perpetual + subscription

**Status:** Confirmed (V1.0+).
**Decision.** Pro perpetual ($129) and Pro monthly ($7); Studio subscription ($19/mo).
**Source.** `01-BRD §7`, `_deep/13-glossary-and-decision-log.md`.
**MVP impact.** None — MVP is licence-free.

## D-002 — Editor framework: TipTap (ProseMirror-based)

**Status:** Confirmed.
**Decision.** TipTap with custom UI; avoid TipTap Pro extensions in MVP.
**Source.** `_deep/13-glossary-and-decision-log.md` `[DECISION-002]`; `03-TAD §6`; `ARCHITECTURE.md §3`.

## D-003 — Sidecar runtime: Rust (in-process modules)

**Status:** Confirmed.
**Decision.** Rust sidecar for all application services. External processes only for Pandoc, epubcheck, optional Ollama detection.
**Source.** `[DECISION-003]`; `ARCHITECTURE.md §2`.

## D-004 — Project file format: directory bundle

**Status:** Confirmed.
**Decision.** `*.booksforge/` directory bundle with SQLite + Markdown mirror + content-addressed assets and snapshots.
**Source.** `[DECISION-004]`; `_deep/04-data-model-and-project-format.md §2`; `ARCHITECTURE.md §7`.

## D-005 — Pandoc: sidecar process, not statically linked

**Status:** Confirmed.
**Decision.** Spawn Pandoc as a separate process via stdin/stdout JSON. Bundle the binary. GPL stays at process boundary.
**Source.** `[DECISION-005]`; `_deep/09-export-pipeline.md`.
**MVP scope.** Pandoc handles **DOCX and PDF only**. EPUB-3 goes through the canonical-HTML pipeline (D-017).

## D-006 — Local LLM runtime: Ollama-first

**Status:** Confirmed for MVP and V1.0.
**Decision.** Ollama is the primary local-LLM runtime, accessed over HTTP on `127.0.0.1:11434`. Setup is guided through the OllamaSetupWizard. Embedded llama.cpp is post-V1.0 behind a feature flag.
**Source.** `ARCHITECTURE.md §5`; `OLLAMA_LOCAL_LLM_SPEC.md`.
**Why.** Ollama provides model download, version pinning, GPU detection, and a stable HTTP API. BooksForge avoids shipping custom llama.cpp Rust bindings.

## D-007 — PDF engine: Typst preferred, LaTeX fallback

**Status:** Provisional (V1.0+).
**Decision.** Default profiles use Typst when supported; academic profiles default to LaTeX.
**MVP impact.** Trade 5×8 and 6×9 PDFs use Pandoc with the engine selected per profile; current default is Typst with LaTeX fallback.
**Source.** `[DECISION-007]`.

## D-008 — Plugin sandbox: WASM compute + isolated WebView UI

**Status:** Confirmed (V1.0+ build).
**Decision.** Compute plugins in `wasmtime` with WIT-typed host API and capability tokens. UI plugins in isolated Tauri WebViews.
**MVP impact.** **Not built in MVP.** The plugin runtime is V1.0.
**Source.** `_deep/07-plugin-architecture.md`.

## D-009 — Frontend state: Zustand + Immer + TanStack Query

**Status:** Confirmed.
**Source.** `[DECISION-009]`; `ARCHITECTURE.md §3`.

## D-010 — Localisation: ICU MessageFormat

**Status:** Provisional.
**Decision.** ICU MessageFormat as the source of truth; `intl-messageformat` for TS, `icu4x` for Rust.
**MVP impact.** UI strings extracted; en in MVP. Localised UIs land in V1.0.

## D-011 — Telemetry vendor: self-hosted PostHog + Sentry-compatible

**Status:** Provisional.
**MVP impact.** Telemetry off by default. When enabled, only event names + duration + non-PII metadata.

## D-012 — Update mechanism: Tauri auto-updater with signed packages

**Status:** Confirmed.

## D-013 — Database: SQLite with `sqlx` (compile-time-checked queries)

**Status:** Confirmed.
**MVP impact.** WAL mode, FK on, schema_version starts at 1.
**Source.** `DATA_MODEL.md`.

## D-014 — Snapshot store: content-addressed under `snapshots/objects/`

**Status:** Confirmed.
**Source.** `DATA_MODEL.md`, `_deep/04-data-model-and-project-format.md §7`.

## D-015 — Code-signing: Apple Developer ID + Microsoft EV cert in MVP

**Status:** Confirmed.

## D-016 — Bounded agent swarm replaces ad-hoc multi-step prompting

**Status:** Confirmed for MVP and V1.0.
**Decision.** A hard-coded agent registry with input/output schemas, versioned hash-pinned prompts, an Orchestrator with caps and approval gates. No tools, no recursion in MVP.
**Source.** `[DECISION-016]`; `AGENTS.md`; `ARCHITECTURE.md §6`.

## D-017 — Canonical-HTML ePUB pipeline

**Status:** Confirmed for MVP and V1.0.
**Decision.** The editor preview HTML is the export source for EPUB-3. The same canonical HTML and CSS power the preview and the EPUB. Pandoc handles DOCX and PDF only.
**Source.** `EXPORT_EPUB_SPEC.md`.
**Why.** Eliminates the preview-vs-export drift the user flagged.

## D-018 — Memory + Vocabulary as first-class subsystems

**Status:** Confirmed for MVP.
**Decision.** Book / chapter / entity / style memory and layered vocabulary dictionaries are first-class subsystems with their own crates (`booksforge-memory`, `booksforge-vocab`), schema, agents (Memory Curator, Vocabulary Dictionary, Humanization), and audit ledgers. Continuously updated, not one-time setup.
**Source.** `MEMORY_SYSTEM.md`, `VOCABULARY_DICTIONARIES.md`, `AGENTS.md §4.7–4.9`.
**Why.** Without them, full-book consistency and human-sounding prose are untestable goals. With them, they are mechanical guarantees.

## D-019 — `agent_runs/` artifact storage in the bundle

**Status:** Confirmed.
**Decision.** Agent outputs > 4 KB are stored on disk under `agent_runs/<run_id>/<task_id>.json`; SQLite stores path + hash. Smaller outputs are inline.
**Source.** `DATA_MODEL.md §5`.

## D-020 — Hard caps on every agent workflow

**Status:** Confirmed.
**Decision.** Per workflow run: ≤8 calls, ≤10 minutes, ≤200k tokens, ≤3 retries per step. Per-chapter / per-scene workflows execute as batches of independent runs, each with its own caps.
**Source.** `AGENTS.md §6`, `ARCHITECTURE.md §6.3`.

## D-021 — Privacy: no content leaves device by default

**Status:** Confirmed (invariant).
**Decision.** No outbound network calls except user-initiated `OllamaSetup → Install`, `Ollama.pull`, and the opt-out `Update.check`. Verified by pcap assertion in CI.
**Source.** `SECURITY_PRIVACY.md §4`.

## D-022 — Schema migrations are forward-only at runtime

**Status:** Confirmed.
**Decision.** Forward migrations run automatically on open after a `pre_migration` snapshot. Reverse migrations are manual scripts.
**Source.** `DATA_MODEL.md §9`.

## D-023 — Reproducibility: byte-identical export for fixed inputs

**Status:** Confirmed (invariant).
**Decision.** Same Canonical Document + same template version + same engine version produces byte-identical EPUB / DOCX / PDF. CI tests the invariant on the medium fixture.
**Source.** `EXPORT_EPUB_SPEC.md §10`.

## D-024 — No `unwrap()` in production paths

**Status:** Confirmed (invariant).
**Source.** `CLAUDE.md §3.1`.

## D-025 — No GPL crate statically linked

**Status:** Confirmed (invariant).
**Decision.** `cargo deny check licenses` enforces. Pandoc, epubcheck, and any future GPL tooling are sidecars only.
**Source.** `CLAUDE.md §3.1`.

## D-026 — IPC types are codegen'd, drift fails CI

**Status:** Confirmed (invariant).
**Source.** `ARCHITECTURE.md §4`.

## D-027 — One PR = one task, with tests and doc updates

**Status:** Confirmed (process invariant).
**Source.** `CLAUDE.md §16`.
