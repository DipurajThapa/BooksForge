# CLAUDE.md — BooksForge Project Instructions for Claude Code

> **Where this file lives.** When the repo is initialised at MZ-01, copy this file to the **repository root** as `CLAUDE.md`. While we are still in the docs-only workspace, it lives at `outputs/CLAUDE.md`. The content does not change.

**Read this first, every session.** Then read `CLAUDE_CODE_CONTEXT_HARNESS.md` for the compressed map. Then proceed to the file the task points at.

---

## 1. Product

**Name.** BooksForge.

**One sentence.** A local-first desktop application that helps writers go from idea to publication-ready files (DOCX, PDF, EPUB-3) using a bounded swarm of local-LLM agents running on Ollama, with strong memory for full-book consistency.

**Targets.** macOS 13+ and Windows 10+ in MVP. Linux V1.0.

**Stack.**

- Tauri v2 (Rust + WebView).
- React + TypeScript (Vite).
- TipTap (ProseMirror-based) editor.
- SQLite (via `sqlx`) + Markdown mirror for storage.
- Ollama for local-LLM inference (HTTP on `127.0.0.1:11434`).
- Pandoc as a sidecar (DOCX, PDF only).
- `booksforge-export-epub` (Rust) for EPUB-3 packaging from canonical HTML.
- EPUBCheck as a sidecar.
- `cargo deny` for license enforcement, `clippy --all-targets -- -D warnings` mandatory.

## 2. Source-of-truth files

- **`CLAUDE.md`** (this file) — operating rules.
- **`CLAUDE_CODE_CONTEXT_HARNESS.md`** — compressed map; load this first.
- **`MVP_SCOPE.md`** — what's in/out for the first build.
- **`PRODUCT_REQUIREMENTS.md`** — full product spec.
- **`ARCHITECTURE.md`** — system design.
- **`ARCHITECTURE_DECISIONS.md`** — every locked decision.
- **`AGENTS.md`** — agent catalog.
- **`MEMORY_SYSTEM.md`** — memory subsystem.
- **`VOCABULARY_DICTIONARIES.md`** — vocabulary subsystem.
- **`DATA_MODEL.md`** — schema.
- **`UI_UX_SPEC.md`** — screens.
- **`BOOK_WORKFLOWS.md`** — every stage with acceptance criteria.
- **`IMPLEMENTATION_PLAN.md`** — milestones and tasks.
- **`TESTING_STRATEGY.md`** — test rules.
- **`SECURITY_PRIVACY.md`** — privacy invariants.
- **`EXPORT_EPUB_SPEC.md`** + **`EXPORT_EPUB_QA.md`** — ePUB pipeline.
- **`OLLAMA_LOCAL_LLM_SPEC.md`** — Ollama integration.
- **`_deep/13-glossary-and-decision-log.md`** — glossary and ADRs.

The deep specs (`01-…` through `13-…`) are reference. Where deep specs and the implementation pack disagree, **the implementation pack wins for MVP** (with a status note in the superseded file). `CONSISTENCY_MATRIX.md` is the index of every such conflict.

## 3. Coding standards

### 3.1 Rust

- Edition 2021. MSRV pinned in `rust-toolchain.toml`.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are CI gates.
- **No `unwrap()`, `expect()`, `panic!()` outside `#[cfg(test)]` and `main()`.** Use typed errors.
- Errors use `thiserror`-derived enums per crate; tagged variants only — never raw strings.
- Public APIs are `Send + Sync` where async is involved; concurrency goes through `tokio`.
- `cargo deny check licenses` rejects GPL-family crates statically. Pandoc, epubcheck, Ollama are sidecars/external.
- `cargo deny check bans` enforces layer boundaries (`booksforge-domain` cannot import `booksforge-storage`, etc.).

### 3.2 TypeScript

- `strict: true`. `noImplicitAny`, `strictNullChecks`, `noUncheckedIndexedAccess` all on.
- Prefer named exports; default exports only for React component files.
- `pnpm typecheck` and `pnpm lint` are CI gates.
- Generated types (from `booksforge-ipc` via `ts-rs`) live in `packages/shared-types/` and are committed.
- Component state via Zustand + Immer; IPC reads via TanStack Query.
- TipTap config is a single source — no per-component editor configs.

### 3.3 SQL

- All queries via `sqlx::query!` / `query_as!` macros (compile-time checked).
- No string-interpolated SQL.
- Migrations are forward-only at runtime, append-only in the repo. Reverse migrations are scripts under `migrations/reverse/`.
- A `pre_migration` snapshot fires before every migration.

### 3.4 IPC

- Every Tauri command has typed input, typed output, tagged-union error.
- Generated TS types are committed; CI fails on Rust↔TS drift.
- Long-running operations emit `progress` events keyed by `job_id`. A `cancel(job_id)` command aborts.

## 4. File / folder conventions

- Crates are kebab-case: `booksforge-orchestrator`.
- TS packages are kebab-case: `@booksforge/editor`.
- Markdown filenames are SCREAMING_SNAKE for top-level pack docs (`AGENTS.md`), kebab-case-numbered for deep specs (`_deep/04-data-model-and-project-format.md`).
- Templates and prompts: `templates/<id>/<version>.toml`, `prompts/<id>/<version>.toml`.
- Test fixtures: `crates/booksforge-test-fixtures/` is the single home.

## 5. Testing requirements

- **Unit + property tests** at Layer 3 (≥90% line coverage).
- **Integration tests** at Layer 4 with real adapters in tmpdirs.
- **Agent-specific patterns** (per `TESTING_STRATEGY.md §3`): prompt-render snapshot, schema-validation, semantic-validator, orchestrator-with-mock-Ollama, determinism, cap-enforcement, live-LLM smoke (nightly, non-gating).
- **Privacy invariants**: pcap assertion, no-network-by-default, manuscript-never-leaves grep, redaction filter.
- **Snapshot invariants**: every applied agent edit references a pre-edit snapshot whose `created_at < applied_at`.
- **Reproducibility**: byte-identical EPUB / DOCX / PDF for fixed input + template + engine.
- **ePUB QA**: golden-file regression on a 30-chapter fixture; EPUBCheck must pass; preview-vs-export visual diff under tolerance.

## 6. Documentation update rules

- Any user-visible behaviour change updates the relevant doc in this pack **in the same PR**.
- A change to schema bumps `schema_version` and updates `DATA_MODEL.md` in the same PR.
- A change to a prompt template version bumps the version number and keeps the prior version on disk for replay.
- A new ADR appends a new entry to `ARCHITECTURE_DECISIONS.md` and `_deep/13-glossary-and-decision-log.md`.
- A new doc adds a row to `DOCS_INVENTORY.md` and (if it changes any other file) an entry in `CHANGELOG_DOC_REFACTOR.md`.
- An ePUB pipeline change requires an entry in `EXPORT_EPUB_QA.md` if a new test or fixture is involved.

## 7. Security / privacy constraints

These are **invariants**. Each has at least one CI test (per `TESTING_STRATEGY.md §4`).

1. **No content leaves the device by default.** Outbound network only on user-initiated `OllamaSetup → Install`, `Ollama.pull`, and the opt-out `Update.check`.
2. **No manuscript content** is sent to a remote endpoint by BooksForge in MVP.
3. **No manuscript content** is logged or telemetered. Redaction filter at all log sinks.
4. **Ollama traffic stays on `127.0.0.1`.** Non-loopback host requires explicit consent.
5. **Crash reports are off by default.** When on, content is scrubbed before upload.
6. **AI capability is off per project until enabled** with a one-time consent prompt.
7. **No GPL crate** statically linked. `cargo deny` enforces.

## 8. Local-first constraints

- Every MVP feature must work with no network.
- Every agent must produce useful output on a 7B-Q4 model running locally via Ollama.
- The user's manuscript bundle (`*.booksforge/`) is portable: copy/zip/move/sync are native.
- Encryption at rest is **post-MVP**; MVP relies on filesystem permissions and the user's disk encryption.

## 9. Agent design constraints

- Agents are **prompt-in / schema-out** units. No tools, no recursion in MVP.
- Every agent has: id, name, purpose, input schema, output schema, prompt template id (versioned, hash-pinned), model preference, allowed memory reads, allowed memory writes, when-to-run, user-gate policy, validators, failure modes.
- The Orchestrator is the only mutator. Agents return proposals.
- Hard caps per workflow run: ≤8 calls, ≤10 minutes, ≤200k tokens, ≤3 retries.
- Per-chapter / per-scene workflows execute as **batches of independent runs**, each with its own caps.
- Pre-edit snapshot is mandatory before any accepted change.
- Audit ledger: `agent_runs`, `agent_tasks`, `agent_outputs`, `agent_applied_edits`.
- Agents can read/write **memory and vocabulary** within scopes declared in `AGENTS.md`.

## 10. Memory system constraints

- Memory is **continuous** — every chapter save, every accepted edit, and every chapter finalisation updates it.
- Memory writes are **typed and scoped**. An agent declares which memory tables it may write to; the orchestrator rejects out-of-scope writes.
- Memory reads are **explicit** — every prompt template declares which memory it pulls into context, and the assembled context is shown to the user before sending.
- Memory persists in SQLite with a Markdown mirror (for recovery) under `manuscript/.memory/`.

## 11. Vocabulary system constraints

- Vocabulary dictionaries are **layered**: book → genre → sub-genre → domain → audience → character voice → chapter type. Lookups merge all applicable layers, with the more specific layer winning on conflicts.
- A dictionary entry has: term, kind (`prefer | avoid | replace`), context (genre/sub-genre/domain/audience/voice/chapter-type), replacement (when `replace`), rationale, source, last-updated.
- The Vocabulary Dictionary Agent updates dictionaries from accepted user edits, accepted Copyeditor proposals, accepted Humanization proposals, and explicit user additions.
- Anti-robotic rules are vocabulary entries with `kind = avoid`. They are contextual (a tech book may use "robust"; a literary novel should not).
- Every dictionary update writes a row in `vocab_updates` for audit and reversibility.

## 12. Export / ePUB QA constraints

- The **canonical HTML** is the export source for EPUB-3. The editor preview renders the same HTML. **Drift is a CI failure.**
- Pandoc handles DOCX and PDF only.
- EPUBCheck is mandatory; errors block; warnings prompt.
- A 30-chapter golden-file fixture exports to a hash-stable EPUB; CI fails on drift without a baseline-update commit and a reason.
- Visual regression: Playwright renders the preview HTML and the unzipped EPUB content HTML using the same WebView; pixel diffs are gated under documented tolerances.

## 13. Forbidden assumptions

Do not assume:

- That a network call is acceptable just because it would be convenient.
- That a `.unwrap()` is acceptable in production paths.
- That a new agent is the right tool for a deterministic problem.
- That an agent can write to the manuscript without user accept and a pre-edit snapshot.
- That a stale doc is correct because it's prose-flavoured authoritative — check `CONSISTENCY_MATRIX.md`.
- That because two specs disagree, you can pick whichever is convenient — the matrix says which wins.
- That the user wants you to invent a feature that isn't in the docs.
- That a model digest can be empty — capture it explicitly or mark `model_digest = unknown`.
- That tests can be deferred to "after the feature works."
- That a guard rule has an exception "for this one case."

## 14. Required behaviour before making a major decision

A "major decision" is anything that:

- Adds or removes a crate.
- Changes the schema.
- Changes a prompt template (a new version replaces, never edits in place).
- Changes a hard rule, a CI gate, or a privacy invariant.
- Modifies a workflow's caps or gate policy.
- Adds a new agent or removes one.
- Changes an export pipeline.
- Touches the canonical-HTML / preview pipeline.

Before making a major decision:

1. Check `ARCHITECTURE_DECISIONS.md` for an existing answer.
2. Check the `CONSISTENCY_MATRIX.md` for a documented winner.
3. If silent: read the smallest doc set that resolves it, propose the change in a PR description, and **append an ADR** to `ARCHITECTURE_DECISIONS.md`.
4. If ambiguous between two reasonable defaults: write the question in `docs/open-questions.md` with `[ASKED-YYYY-MM-DD]` and the alternatives, pick one with `[ASSUMED]`, surface it in the PR.

## 15. The "ask only when blocked" rule

Default behaviour: **choose the documented default and proceed.**

Ask only if:

- The docs explicitly contradict each other and `CONSISTENCY_MATRIX.md` does not resolve the conflict, **or**
- The decision changes the privacy invariants, the security posture, or a hard rule, **or**
- The decision affects the user's data durability or recoverability.

Never ask:

- Stylistic preferences (use the existing convention).
- Library version pins (use the latest stable matching MSRV).
- Naming details (follow §4).
- Whether to add a test (yes — always, in the same PR).

## 16. Pull-request shape

Every PR has:

- A title with a milestone tag and a short verb-phrase: `[MZ-04] add Ollama HTTP client and Setup Wizard`.
- A description with: goal, files touched, tests added, risks, and any `[ASSUMED]` notes.
- Documentation co-changes (per §6).
- CI green: lints, tests, coverage, perf budgets, codegen-drift, license-deny, layered-imports.

## 17. When to load which doc (token-efficiency)

Load only what you need.

- Implementing an agent → `AGENTS.md` (the section for that agent), `MEMORY_SYSTEM.md` if it touches memory, `VOCABULARY_DICTIONARIES.md` if it touches vocab.
- Touching the editor → `UI_UX_SPEC.md §5`, `ARCHITECTURE.md §3`.
- Schema changes → `DATA_MODEL.md`.
- Memory changes → `MEMORY_SYSTEM.md`.
- Vocabulary changes → `VOCABULARY_DICTIONARIES.md`.
- ePUB changes → `EXPORT_EPUB_SPEC.md`, `EXPORT_EPUB_QA.md`.
- Security work → `SECURITY_PRIVACY.md`.

If you find yourself loading more than three full deep specs in one task, **stop** and update `CLAUDE_CODE_CONTEXT_HARNESS.md` to include the missing summary instead.

## 18. Skills, hooks, subagents

- **Skills** (`.claude/skills/`) are in `CLAUDE_CODE_SKILLS_SPEC.md`. Use the right skill for the right job (e.g., `agent-design-review` before merging a new agent).
- **Hooks** (`.claude/hooks/HOOKS.md`) automate guardrails — naming, lint, ePUB QA, secrets. Do not bypass them.
- **Subagents** (`.claude/agents/`) per `CLAUDE_CODE_SUBAGENTS_SPEC.md` are for review passes (architecture review, security review, prompt review). Invoke them when the criteria in the spec are met.

## 19. Glossary one-liner

For anything not defined here, see `_deep/13-glossary-and-decision-log.md §A`. If the term is missing, add it.

---

**This is the operating contract. If the contract and the docs diverge, fix the doc. If the docs and the user's intent diverge, ask the user.**
