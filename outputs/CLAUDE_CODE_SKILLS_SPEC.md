# Claude Code Skills — Spec

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Defines the eight skills BooksForge ships with for Claude Code.** Companion to `CLAUDE_CODE_HOOKS_SPEC.md`, `CLAUDE_CODE_SUBAGENTS_SPEC.md`.

A "skill" here is a reusable Claude Code workflow with a specific purpose — invoked when the right trigger fires. We seed skill stubs at `.claude/skills/<skill-id>/SKILL.md` so they auto-load.

---

## How to invoke

Each skill has a clear trigger condition (e.g., "before merging a PR that adds an agent"). When the condition is met, Claude Code loads the SKILL.md and follows its procedure. Skills have a defined input, procedure, output, and required checks. They are read-only by default; they do not write code unless explicitly told.

---

## 1. `docs-consistency-audit`

**Purpose.** Verify that the implementation pack and the deep specs are consistent — no stale references, no conflicting decisions, naming uniform.

**When Claude Code should use it.**

- Before merging a PR that touches more than two `.md` files in `outputs/`.
- After an architecture decision is added.
- When `CONSISTENCY_MATRIX.md` shows an open item.

**Inputs.** A PR diff (or a list of changed files); the current `outputs/` tree; `CONSISTENCY_MATRIX.md`.

**Procedure.**

1. Run a recursive grep for old names: `Bookforge|BookForge|Book Forge|bookforge` in `outputs/`. Should return zero matches.
2. For every locked decision in `ARCHITECTURE_DECISIONS.md`, search every doc for its inverse (e.g., D-006-revB locks Ollama-first; search for `embedded llama.cpp` in MVP context — only `_deep/08-ai-integration.md` should mention it, and only with a status note).
3. For every doc in `DOCS_INVENTORY.md`, verify the listed dependencies exist.
4. For every entry in `CONSISTENCY_MATRIX.md`, verify the resolution status holds.
5. Cross-check filenames mentioned in prose against the actual filesystem — broken links fail.

**Expected output.** A short report (≤300 words) per check: PASS or list of findings. If any finding, the PR is paused for human review.

**Required checks.**

- Naming: zero stale variants.
- Locked decisions: no contradicting prose in non-superseded files.
- Doc dependencies: every referenced file exists.
- Cross-doc claim numbers: agent counts, milestone counts, table counts agree.

**Files it may read.** All `.md` under `outputs/`.

**Files it should not read unless necessary.** SVGs, prompt-template TOMLs, Rust code.

**Seed file.** `.claude/skills/docs-consistency-audit/SKILL.md`.

---

## 2. `architecture-review`

**Purpose.** Review architecture decisions before implementation lands. Catch layering violations, premature abstractions, and underspecified interfaces.

**When Claude Code should use it.**

- Before merging a PR that adds or removes a crate.
- Before merging a PR that introduces a new IPC command, schema table, or trait at a layer boundary.
- Before merging a PR that touches `ARCHITECTURE.md` or `DATA_MODEL.md`.

**Inputs.** A PR diff; the current crate layout; `ARCHITECTURE.md`; `ARCHITECTURE_DECISIONS.md`.

**Procedure.**

1. Verify the new code respects the four-layer rule (no Layer-3 imports of Layer-4, etc.).
2. Verify any new IPC command is in `booksforge-ipc` with typed input / output / tagged error.
3. Verify any new schema bumps `schema_version`.
4. Verify any new dependency passes `cargo deny check licenses` and `bans`.
5. Verify any new trait at a layer boundary has at least one mock and one real implementation.
6. Check whether the change requires an ADR; if so, confirm one was added.

**Expected output.** A short PASS/FAIL with the specific finding for any FAIL. The PR cannot be merged until each FAIL is resolved.

**Files it may read.** Rust source, `Cargo.toml`s, `ARCHITECTURE.md`, `DATA_MODEL.md`, `ARCHITECTURE_DECISIONS.md`.

---

## 3. `agent-design-review`

**Purpose.** Validate agent definitions: bounded purpose, schema, prompt template hash-pinning, validators, failure modes, user gates, memory scope.

**When Claude Code should use it.**

- Before merging a PR that adds, modifies, or removes an agent in `booksforge-agents`.
- Before merging a PR that introduces a new prompt template version.
- When a new workflow is added to `booksforge-orchestrator/src/workflows.rs`.

**Inputs.** PR diff; `AGENTS.md`; the agent's source file; the prompt template TOML(s).

**Procedure.**

1. Confirm the agent has all 12 fields of `AgentSpec`.
2. Confirm input/output JSON Schemas exist and validate at the boundary.
3. Confirm prompt template is hash-pinned; the hash is recorded on every `agent_tasks` row.
4. Confirm validators (schema + semantic + cross-cutting) are wired; tests exist for each.
5. Confirm `model_preference` references the curated model registry.
6. Confirm `memory_reads` and `memory_writes` are within the scope declared in `AGENTS.md`.
7. Confirm `user_gate` is consistent with whether the output mutates the manuscript or memory.
8. Confirm orchestrator caps cannot be bypassed.
9. Confirm at least: prompt-render snapshot test, schema-validation test, mock-Ollama happy path, mock-Ollama invalid output retry test.

**Expected output.** A checklist with PASS/FAIL per item. Any FAIL blocks the merge.

**Files it may read.** `AGENTS.md`, `booksforge-agents/`, `booksforge-prompt/`, `booksforge-orchestrator/`, `MEMORY_SYSTEM.md`.

---

## 4. `memory-system-review`

**Purpose.** Validate memory-touching changes: schema invariants, scope boundaries, recovery, audit completeness.

**When Claude Code should use it.**

- Before merging a PR that touches `booksforge-memory`, the memory tables, or any agent's `memory_writes` declaration.
- After the Memory Curator's prompt template is updated.

**Inputs.** PR diff; `MEMORY_SYSTEM.md`; the changed source.

**Procedure.**

1. Confirm every `chapter_memory.node_id` in tests references an existing chapter; every `entity_memory.entity_id` references the entity.
2. Confirm the Markdown mirror under `manuscript/.memory/` is updated alongside SQL writes (best-effort, after commit).
3. Confirm any new memory write declares its scope and the orchestrator enforces it.
4. Confirm pre-edit snapshot fires before any agent-applied memory write.
5. Confirm the audit ledger (`last_writer`, `updated_at`) is populated.
6. Confirm `style_memory_history` records reversible deltas if `style_memory` was touched.

**Expected output.** PASS/FAIL with findings.

**Files it may read.** `MEMORY_SYSTEM.md`, `DATA_MODEL.md`, `booksforge-memory/`, `booksforge-orchestrator/`.

---

## 5. `epub-export-qa`

**Purpose.** Validate ePUB export pipeline changes: canonical-HTML invariants, EPUBCheck cleanliness, golden-file stability, visual regression.

**When Claude Code should use it.**

- Before merging a PR that touches `booksforge-export-epub`, `booksforge-export`, or any prose in `EXPORT_EPUB_SPEC.md` / `EXPORT_EPUB_QA.md`.
- After the canonical CSS or any template stylesheet is changed.
- Before tagging a release.

**Inputs.** PR diff; the medium and large fixtures; the golden-file hashes; the visual-regression baseline.

**Procedure.**

1. Run the full ePUB QA test suite (per `EXPORT_EPUB_QA.md §3`):
   - Structural checks (S1–S8).
   - Content checks (C1–C13).
   - Typography (T1–T7).
   - Metadata (M1–M6).
   - Accessibility (A1–A4).
2. Run EPUBCheck; require zero errors / warnings on KDP-eBook profile.
3. Run the visual regression test on the medium fixture; require under-tolerance pixel diff.
4. Verify the golden-file hashes match (no drift) **unless** the PR includes a baseline-update commit + reason.
5. If the PR changes the canonical CSS, manually inspect the diff and confirm the change is intentional.

**Expected output.** A pass/fail per check with detail on any failure.

**Files it may read.** `EXPORT_EPUB_SPEC.md`, `EXPORT_EPUB_QA.md`, `booksforge-export*`, fixtures, golden hashes.

---

## 6. `test-plan-generator`

**Purpose.** Convert a new requirement (FR-ID, feature description, or accepted user story) into a complete test plan.

**When Claude Code should use it.**

- After a new FR is added to `_deep/02-FSD-functional-specifications.md`.
- After a new agent or workflow is specified.
- After a new export profile is added.

**Inputs.** The requirement (FR-ID + prose, or a feature spec).

**Procedure.**

1. Identify the layer(s) the requirement touches (UI / Layer 2 / Layer 3 / Layer 4).
2. For each layer, propose tests:
   - Unit (pure logic).
   - Property (invariants).
   - Integration (with mocks and real adapters).
   - E2E (Playwright if UI).
   - Privacy invariant (if data-handling).
   - Snapshot invariant (if memory or manuscript).
   - Reproducibility (if export).
   - Performance budget (if user-facing).
3. Propose at least one negative-path test per layer.
4. Produce a checklist the implementing PR must satisfy.

**Expected output.** A test plan as Markdown, embedded in the PR description.

**Files it may read.** `TESTING_STRATEGY.md`, `EXPORT_EPUB_QA.md`, the affected spec doc, existing fixtures.

---

## 7. `implementation-slice-planner`

**Purpose.** Break a large feature into Claude Code-friendly development slices (one PR each), with sequencing and acceptance criteria per slice.

**When Claude Code should use it.**

- When a milestone task description spans more than 1 day of work or more than 3 files.
- When a new feature is requested mid-milestone.

**Inputs.** A feature description; the current milestone; `IMPLEMENTATION_PLAN.md`.

**Procedure.**

1. List the technical components needed (crates, IPC commands, UI screens, tests, docs).
2. Split into slices that each:
   - Take ≤ 1 day.
   - Touch ≤ 5 files.
   - Have a clear, testable acceptance criterion.
   - Don't break existing tests.
3. Sequence the slices so each builds on the previous.
4. Identify the first slice as a "thin vertical" that proves the architecture works.

**Expected output.** A slice list, with goal, files, tests, and acceptance per slice.

**Files it may read.** `IMPLEMENTATION_PLAN.md`, `ARCHITECTURE.md`, the affected spec docs.

---

## 8. `prompt-library-review`

**Purpose.** Review and improve prompts in `outputs/prompts/` and `templates/prompts/`. Ensure they align with the current architecture, agents, memory, vocab, and ePUB rules.

**When Claude Code should use it.**

- When a new prompt is added.
- When an agent's prompt template version is bumped.
- When the agent catalog changes.

**Inputs.** The prompt(s); `AGENTS.md`; `MEMORY_SYSTEM.md`; `VOCABULARY_DICTIONARIES.md`; `EXPORT_EPUB_SPEC.md`.

**Procedure.**

1. Verify the prompt declares: purpose, intended agent, inputs, outputs, required memory reads, required memory writes, failure handling, validation checklist, example output shape.
2. Verify untrusted user content is fenced with `<<<USER_CONTENT>>>` and the system prompt instructs the model to ignore embedded instructions.
3. Verify the prompt is consistent with vocabulary rules (the prompt does not encourage robotic phrases).
4. Verify the prompt is consistent with memory rules (does not claim to write outside its declared scope).
5. Verify the output schema is exact JSON; no ambiguous "or similar" language.
6. Verify the prompt template is hash-pinned and a snapshot test exists.
7. If the prompt is duplicated elsewhere, propose a merge.

**Expected output.** A diff or set of edits to the prompt; PASS once the checklist passes.

**Files it may read.** `prompts/`, `templates/prompts/`, `AGENTS.md`, `MEMORY_SYSTEM.md`, `VOCABULARY_DICTIONARIES.md`.

---

## Skill files location

The actual `.claude/skills/<skill-id>/SKILL.md` stubs ship with the repo — see `.claude/skills/` (created at MZ-01). Each stub references this spec by section number; the stub is the runnable form.

## When NOT to use a skill

- Trivial PRs (typo fixes, one-line code change). Skills are for non-trivial review.
- Pure prose changes in stable docs. The `docs-consistency-audit` covers naming and references; that's enough.
- Personal experimentation in a feature branch that won't be merged.

## Skills as a guard, not a bypass

A skill's PASS does not waive the standard review. CI gates and human review still apply. Skills accelerate review by surfacing issues consistently — they are not a substitute for thinking.
