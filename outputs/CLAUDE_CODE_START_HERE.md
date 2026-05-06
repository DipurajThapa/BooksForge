# Claude Code — Start Here

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Audience:** Claude Code (and any engineer pairing with it)

This is the entry point for implementation. If you are about to write code for BooksForge, read this file first, then the files it points at. Do not skim — every paragraph below is load-bearing.

---

## 1. What you are building

BooksForge is a **local-first, cross-platform desktop application for writing, editing, formatting, and exporting books**. It targets fiction, non-fiction, memoir, business/self-help, and academic authors with one shared core and mode-specific templates and validators.

The differentiator is **a bounded agentic swarm of specialized local-LLM agents** that runs on the user's machine via Ollama, plus a Pandoc-based export pipeline that produces store-ready DOCX, PDF, and EPUB.

The product name is a working name. The architecture is the asset.

## 2. Read these files in this exact order

The reading list is layered. **Tier 1** is mandatory before you write code. **Tier 2** is consulted when the task touches its area. **Tier 3** is reference.

### Tier 1 — Mandatory first read (in order)

1. **`CLAUDE.md`** — operating contract. Coding standards, hard rules, ask-only-when-blocked protocol.
2. **`CLAUDE_CODE_CONTEXT_HARNESS.md`** — compressed map of the project. Reduces token use across sessions.
3. **`CLAUDE_CODE_START_HERE.md`** — this file.
4. **`MVP_SCOPE.md`** — what is in / out for the first build.
5. **`PRODUCT_REQUIREMENTS.md`** — target users, journeys, acceptance criteria.
6. **`ARCHITECTURE.md`** — system design with Ollama-first AI runtime.
7. **`ARCHITECTURE_DECISIONS.md`** — every locked decision. Check here before proposing alternatives.
8. **`AGENTS.md`** — the agent catalog (nine MVP, ten more in V1.0+). **The heart of the product.**
9. **`IMPLEMENTATION_PLAN.md`** — milestones and the first ten concrete tasks.

### Tier 2 — Read when the task touches that area

- **`MEMORY_SYSTEM.md`** — touch only when implementing memory or `booksforge-memory`.
- **`VOCABULARY_DICTIONARIES.md`** — touch only when implementing vocab or `booksforge-vocab`.
- **`DATA_MODEL.md`** — schema work.
- **`UI_UX_SPEC.md`** — UI work.
- **`BOOK_WORKFLOWS.md`** — when wiring a stage end-to-end.
- **`EXPORT_EPUB_SPEC.md`** + **`EXPORT_EPUB_QA.md`** — export pipeline work.
- **`OLLAMA_LOCAL_LLM_SPEC.md`** — Ollama integration.
- **`TESTING_STRATEGY.md`** — when adding or modifying tests.
- **`SECURITY_PRIVACY.md`** — when touching network code, telemetry, or user data.

### Tier 3 — Claude Code support and harness

- **`CLAUDE_CODE_SKILLS_SPEC.md`** + `.claude/skills/` — invokable review skills.
- **`CLAUDE_CODE_HOOKS_SPEC.md`** + `.claude/hooks/HOOKS.md` — automated guardrails.
- **`CLAUDE_CODE_SUBAGENTS_SPEC.md`** + `.claude/agents/` — review subagents.
- **`DOCS_INVENTORY.md`** — registry of every spec file.
- **`CONSISTENCY_MATRIX.md`** — what wins where if two docs disagree.
- **`CHANGELOG_DOC_REFACTOR.md`** — append-only history.

### Tier 4 — Deep specs (reference only)

The numbered files (`01-BRD-…` through `13-glossary-…`) are the detail reference. Read only when looking up an FR-ID, an ADR, or a trade-off rationale. The implementation pack supersedes them where they disagree (per `CONSISTENCY_MATRIX.md`).

The phase-by-phase prompts (`prompts/phase-*.md`) — phases 00–05 are **superseded** by `IMPLEMENTATION_PLAN.md`; phases 06+ are authoritative for V1.0+ work.

## 3. The two product-level decisions you must respect

Two architectural decisions are non-negotiable without an ADR and a stop-and-discuss with the human owner.

### Decision A — Ollama is the primary local-LLM runtime

Ollama is the local-LLM runtime for MVP and V1.0:

- Ollama runs as a separate process the user installs (or that BooksForge installs through a guided flow — see `ARCHITECTURE.md §5`).
- BooksForge talks to it over its local HTTP API on `127.0.0.1:11434`.
- The curated registry covers Qwen, Gemma, Llama, Mistral, and Phi; users can use any model Ollama exposes.
- Embedded `llama.cpp` is **post-V1.0** behind a feature flag — not built in MVP.

Why: Ollama gives us model download, version pinning, hardware detection, GPU offload, and a stable API for free. The cost is one extra installation step for the user, mitigated by the guided OllamaSetupWizard.

### Decision B — A bounded agent swarm coordinates multi-stage work

Two surfaces:

**Single-shot inline assists** (Sharpen, Shorten, Continue, Rephrase) are quick-action presets in the editor. They are one-prompt operations, not agents.

**The Agent Swarm** handles anything that spans more than one prompt or more than one part of the manuscript. Defined in `AGENTS.md`:

- Every agent has a fixed role, a versioned prompt, a typed input schema, and a typed output schema.
- Every agent run is recorded in `agent_tasks` with full inputs, outputs, model, prompt hash, and duration.
- No agent writes to the manuscript directly. Agents produce **proposals**; the user accepts, rejects, or edits before any change is applied. A snapshot is taken before any accepted change.
- The Orchestrator enforces depth limits, time limits, token budgets, and approval gates. There is no "self-driving book writer" loop.
- The swarm is offline-capable. It must work with the local Ollama runtime alone.

## 4. The Hello-World you must produce in the first week

Before any agent work, before any export pipeline, before any plugin runtime, you must reach **Milestone Zero**. Milestone Zero is the smallest end-to-end slice that proves the stack works.

Milestone Zero deliverables:

1. A Tauri v2 + React + Rust workspace that builds on macOS and Windows.
2. The user can click "New Project," name it, and a `*.booksforge/` bundle is created on disk with a SQLite `project.db` and a `manuscript/` mirror directory.
3. The user can type into a single scene editor (TipTap with a paragraph and heading mark — that is enough). Autosave fires every 5 seconds.
4. The user can close and reopen the project; their text is still there.
5. There is a "Models" settings screen that detects whether Ollama is running on `127.0.0.1:11434` and, if so, lists the user's installed models.
6. There is a single agent — `OutlineArchitectAgent` — that takes a one-paragraph book idea and returns a three-act outline as JSON. The output is shown in a side panel; nothing is written to the manuscript.

Milestone Zero is the first PR series. After Milestone Zero, follow `IMPLEMENTATION_PLAN.md` from Milestone One.

## 5. Hard rules you must never break

These are the non-negotiables. Each is enforced by either a lint, a test, or a code review checklist item.

1. **No content leaves the device by default.** No HTTP client may run unless the user has explicitly enabled a network feature for that project. Cloud LLM is post-V1.0.
2. **No agent writes to the manuscript without user approval and a pre-edit snapshot.** This is enforced in the orchestrator. There is no "auto-apply" toggle, even as an option.
3. **No GPL dependency is statically linked into the host binary.** Pandoc and any GPL tooling are sidecars. (See `_deep/13-glossary-and-decision-log.md`.)
4. **No untyped IPC.** Every Tauri command has a typed input, typed output, and tagged-union error. TypeScript types are codegenerated from Rust. CI fails on drift.
5. **No `unwrap()` in production paths.** Use typed errors. `unwrap` and `panic!` are reserved for tests and `main()`.
6. **No infinite agent loops.** The Orchestrator hard-caps iteration count, total tokens, and wall-clock time per workflow. See `AGENTS.md §6`.
7. **Forward-compatible project format.** A project written by a newer version must remain readable in spirit by an older version, or refuse to open with a clear message. Never silently corrupt.
8. **Performance regressions ≥10%** on any budget in `ARCHITECTURE.md §10` fail CI.

## 6. What you must NOT build yet

These are explicitly out of scope for the first build. Do not build them, even if they seem easy. They live on the roadmap and have their own risks; build them when their phase comes up.

- Real-time collaboration (CRDTs, presence, live cursors).
- Cloud sync.
- Plugin marketplace (the plugin **runtime** is also deferred past MVP — see `IMPLEMENTATION_PLAN.md`).
- Cloud LLM providers (Anthropic, OpenAI, OpenRouter).
- Embedded `llama.cpp` Rust bindings.
- Voice dictation.
- Translator pack.
- Mobile companion app.
- Cover-art image generation.
- DRM, payment processing, audiobook export, real-time voice chat.

If a user request seems to require any of the above, raise it as a question, do not invent it.

## 7. Where to ask questions

If a spec is ambiguous, contradictory, or silent on something you must decide:

1. Append a question to `docs/open-questions.md` (create the file if it does not exist).
2. Make a defensible best-guess, mark it `[ASSUMED]` in code comments, and continue.
3. Surface the assumption in the PR description so a human can confirm or override.

Do not block on questions. Do not invent unconstrained features. Surface and proceed.

## 8. How to verify you are on track

Before pushing any PR, confirm:

- [ ] CI is green on the full matrix configured in `IMPLEMENTATION_PLAN.md`.
- [ ] `cargo deny check licenses` passes (no GPL crates statically linked).
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] `pnpm typecheck` passes and the generated TS types are committed.
- [ ] A new feature has a test in the same PR (unit, integration, or E2E as appropriate).
- [ ] The relevant doc in this pack is updated if user-visible behaviour changed.
- [ ] No new `unwrap()` outside `#[cfg(test)]` and `main()`.
- [ ] No new direct-network call site without an explicit capability gate.
- [ ] The agent budget and approval gates were not bypassed.

## 9. Where to start, mechanically

Once you have read the nine files in §2, follow these mechanical steps:

1. Open `IMPLEMENTATION_PLAN.md §3` (Milestone Zero task list).
2. Pick task **MZ-01** (`bootstrap-workspace`).
3. Mark it in-progress in your task tracker.
4. Implement, test, commit, and open a PR with the milestone tag in the title.
5. After review, take **MZ-02**, and so on.

Do not jump ahead. Each task in the milestone is sequenced so that the next task has the scaffolding it needs.

## 10. What "done" looks like for the MVP

The MVP is done when, on a clean Windows or macOS machine, a user can:

1. Install BooksForge and Ollama using the bundled guided setup.
2. Create a new fiction project from a template.
3. Type or import a 30k–80k-word draft.
4. Run the **Outline Architect**, **Developmental Editor**, **Continuity**, and **Copyeditor** agents and accept/reject their proposals.
5. Run the pre-export validator and resolve any errors.
6. Export to DOCX, PDF, and KDP-compatible EPUB-3 in under 60 seconds for a 100k-word project.
7. Reopen the project a week later, see all snapshots, and continue working — entirely offline.

Anything beyond that ships in V1.0 and beyond.

---

**Now go to `PRODUCT_REQUIREMENTS.md`.**
