# Implementation Plan — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06

This document is the implementation-ordered task plan. It covers Milestone Zero through MVP and lists the **first ten concrete Claude Code tasks** with acceptance criteria.

---

## 1. Milestones

| ID | Name | Window | Theme | Exit |
|----|------|--------|-------|------|
| **M0** | Bootstrap | Week 1 | Repo, CI, smallest end-to-end slice | Tauri+React+Rust workspace boots; can create/open/edit/save a project; one agent (`OutlineArchitectAgent`) runs against a mocked Ollama |
| **M1** | Project & editor | Weeks 2–4 | Real editor on real storage | TipTap editor with binder; 100k-word fixture opens in <2s; autosave + crash recovery green |
| **M2** | Ollama + first three agents | Weeks 4–7 | Local LLM + intake/outline/copyedit | Ollama detected, model pulled; `IntakeAndOutline` and `Copyedit` workflows produce schema-valid output on the reference hardware |
| **M3** | Developmental + continuity | Weeks 7–9 | Critique agents | `DevelopmentalReview` and `ContinuityCheck` workflows green; entity bible powers continuity |
| **M4** | Templates + validators | Weeks 9–12 | Three templates + KDP validator | A new Romance project passes the pre-export gate |
| **M5** | Export pipeline | Weeks 11–14 | Pandoc + EPUB/PDF/DOCX | A 100k-word project exports to all four MVP profiles in <60s |
| **M6** | MVP polish | Weeks 14–16 | Stabilisation, accessibility, signing, beta | Public beta on macOS and Windows |
| **MVP release** | Week 16 | | | All MVP acceptance criteria in `PRODUCT_REQUIREMENTS.md §9` pass |

This is **two weeks tighter than the deep roadmap's MVP** because we removed: embedded llama.cpp, plugin runtime in MVP, Linux build in MVP, and full validator coverage (we ship 15+ in MVP, not 20+).

## 2. Dependencies between milestones

- M1 depends on M0 (workspace and storage scaffolding).
- M2 depends on M1 (real storage to feed the agent context builder).
- M3 depends on M2 (the orchestrator must be solid before adding more agents).
- M4 depends on M2 (templates assume agent-aware project creation).
- M5 depends on M4 (export reads from validated, formatted templates).
- M6 depends on M3 + M4 + M5 (everything must converge for beta).

Within each milestone, tasks are sequenced; the next task assumes the previous task landed.

## 3. Milestone Zero — the first ten Claude Code tasks

These are the **first ten tasks** Claude Code should pick up, in order. Each task corresponds to one PR. Each PR must include tests as defined in `TESTING_STRATEGY.md`.

### MZ-01 — Bootstrap workspace

**Goal.** Cargo workspace + Tauri v2 + React/TS/Vite frontend that builds and runs on macOS and Windows.

**Steps.**

1. Initialise the repo at `booksforge/` with the crate layout from `ARCHITECTURE.md §3`.
2. Add Tauri v2 to `apps/desktop/`.
3. Add the React/TS/Vite frontend to `apps/desktop/src-ui/`.
4. Wire Tauri ↔ React with one IPC command `app.version` returning a typed `AppVersion` struct codegened to TS via `ts-rs`.
5. Render `App version: 0.0.1` in the UI.
6. Set up CI on `macos-14`, `macos-13`, `windows-2022` running `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `pnpm typecheck`, `pnpm test`.
7. Add `cargo deny check licenses` to CI; configure to reject GPL-family crates.
8. Add a basic README and CONTRIBUTING.

**Acceptance criteria.**

- `cargo build` and `cargo test` pass on the matrix.
- The dev build runs and the IPC roundtrips.
- `pnpm typecheck` passes, and the `AppVersion` TS type matches the Rust struct.
- CI green; no GPL crate warnings; clippy with `-D warnings`.

**Out of scope.** Signing, notarisation, distribution, plugin runtime.

### MZ-02 — Project bundle creation and opening

**Goal.** A user can create a `.booksforge/` bundle from the UI, open it, and reopen it later.

**Steps.**

1. Implement `booksforge-fs` with atomic bundle creation, bundle layout per `DATA_MODEL.md §13`.
2. Implement `booksforge-storage` with SQLite migration v1 covering all tables in `DATA_MODEL.md §3` (nodes, scene_content, notes, entities, entity_aliases, entity_scene_appearances, snapshots, validator_runs, validator_issues, refs, comments, tracked_changes, exports) plus §4 (style_book), §5 (agent layer), §6 (model_settings). The v1 migration creates every table so future migrations are always additive.
3. Implement `booksforge-domain` types per `DATA_MODEL.md`: `Project`, `Node`, `SceneContent`, `Entity`, `StyleBook`.
4. Implement the `StorageRepository` and `BundleFilesystem` traits per `ARCHITECTURE.md §2.1`.
5. Tauri commands: `project.create({path, mode, template_id, meta})`, `project.open({path})`, `project.close({})`, `project.recent({})`.
6. UI: Project Picker (per `UI_UX_SPEC.md §2`) and a stub New Project Wizard (Steps 1–2 + 4 only — no agent wiring yet).
7. Lock file + recent-projects list persisted to `~/.booksforge/settings.toml` per `DATA_MODEL.md §9`.

**Atomic bundle creation — implementation contract.**

Bundle creation must be atomic: a crashed process leaves no half-bundle. The exact sequence:

```
1. Generate a temp path: system_temp_dir() / "booksforge-create-{ulid}"
2. Create the temp directory.
3. Write all bundle files into the temp directory:
     a. manifest.toml  (rendered from ProjectMeta + defaults)
     b. .booksforge-version  (minimum app version string)
     c. Subdirectories: manuscript/, assets/, snapshots/objects/,
        exports/, agent_runs/, plugins/
4. Open and migrate project.db inside the temp directory (migration v1).
5. Write the initial style_book and model_settings rows.
6. Atomic rename: std::fs::rename(temp_path, final_path)
      - On POSIX this is atomic within the same filesystem.
      - On Windows use MoveFileExW with MOVEFILE_REPLACE_EXISTING
        only after verifying final_path does not exist.
7. If rename fails: remove temp_path (best-effort); propagate FsError.
8. On success: acquire the lock file, add to recent-projects list.
```

**Orphan temp-dir recovery.** On every app launch, `booksforge-fs` scans `system_temp_dir()` for directories matching `"booksforge-create-*"` that are older than 5 minutes and deletes them silently. This handles crashes between steps 2 and 6 above.

**Lock file lifecycle.** The lock file (`.booksforge.lock`) is created immediately after a successful `open_bundle` and removed in the RAII guard's `drop`. If the process crashes, the lock file persists but contains the dead PID. On next open, `booksforge-fs` checks liveness (Unix: `kill(pid, 0)`; Windows: `OpenProcess`) and evicts stale locks.

**Acceptance criteria.**

- Creating a project produces a valid `.booksforge/` bundle on disk with all required subdirectories.
- Killing the app (SIGKILL / Windows terminate) mid-create leaves no bundle at the final path.
- Orphan temp dirs from a killed create are cleaned up on next launch.
- Reopening the project shows it in recent and loads correctly.
- Moving the bundle directory and reopening shows a "missing" state with Locate / Remove actions.
- Schema migration v1 runs cleanly on a fresh database.
- A reversal script (`migrations/reverse/0001_initial_reverse.sql`) exists; an integration test verifies a clean v1 → reverse → v1 round-trip leaves the same schema.
- `~/.booksforge/settings.toml` contains the opened project in `recent_projects.entries` after first open.

### MZ-03 — Single-scene editor

**Goal.** A user can type into one scene and the content survives close/reopen.

**Steps.**

1. Add TipTap to `packages/editor/` with the MVP node set (paragraph, H1–H4, bold/italic/underline, blockquote, lists, code block, image, link).
2. Implement `scene.save({node_id, pm_doc_json})` and `scene.load({node_id})` IPC commands.
3. Wire autosave at 5s after last keystroke or on blur.
4. Implement Markdown mirror writer (best-effort, after SQLite commit).
5. Wire crash recovery: on launch, if `.recovery.log` has entries newer than `project_db_hash`, surface a recovery prompt.

**Acceptance criteria.**

- Typing then closing the app and reopening shows the content.
- `kill -9` mid-edit followed by relaunch surfaces a single recovery prompt and restores the buffer.
- The Markdown mirror at `manuscript/.../<scene>.md` exists and is readable after a save.

### MZ-04 — Ollama HTTP client + Setup Wizard

**Goal.** The app detects Ollama, launches it if installed, or guides the user to install it. The user can pick and pull a model.

**Steps.**

1. Implement `booksforge-ollama` with `OllamaClient` trait per `ARCHITECTURE.md §5.4` and a real HTTP implementation using `reqwest`.
2. Implement the model registry from `booksforge-ollama/models.toml` per `ARCHITECTURE.md §5.3`.
3. Implement Tauri commands: `ollama.probe()`, `ollama.list_models()`, `ollama.pull({model})` with progress events, `ollama.smoke_test({model})`.
4. UI: Ollama Setup Wizard per `UI_UX_SPEC.md §4` — detection, install offer, model pick, pull, smoke test.
5. CI: a mock Ollama server fixture exercises the client without requiring real Ollama.

**Acceptance criteria.**

- On a clean machine without Ollama, the wizard offers a guided install and proceeds.
- On a machine with Ollama running, the wizard detects it and lists installed models.
- Pulling a model shows progress and completes; failure surfaces a typed error and recovery actions.
- The smoke test returns a non-empty completion.
- The HTTP client is testable: mock-based tests pass in CI without contacting a real Ollama.

### MZ-05 — Prompt template engine + the Outline Architect Agent

**Goal.** The first agent runs end-to-end against either a real or mocked Ollama and produces a schema-valid `OutlineProposal`.

**Steps.**

1. Implement `booksforge-prompt` with MiniJinja-based rendering, fence handling for `<<<USER_CONTENT>>>` blocks, JSON-Schema export for the prompt's output schema.
2. Implement `booksforge-agents` with `AgentSpec` types and the `OutlineArchitectAgent` definition per `AGENTS.md §4.2`.
3. Implement minimal `booksforge-orchestrator` with one workflow (`IntakeAndOutline` reduced to just `outline-architect` for now).
4. Persist `agent_runs`, `agent_tasks`, `agent_outputs` rows.
5. UI: a stub form on a debug screen that takes a `ProjectBrief` JSON and runs the agent.

**Acceptance criteria.**

- Given a fixed brief input + a mocked Ollama returning canned text, the orchestrator runs the agent, validates the output, persists the rows, and returns the proposal.
- A failing-schema mock causes a retry, then a `proposal_invalid` artifact with no crash.
- A cancellation mid-run leaves a `cancelled` row with partial inputs preserved.
- A blake3 hash of the prompt template is recorded on every run, matching the on-disk file.

### MZ-06 — Snapshots v1 (manual + pre-agent-edit)

**Goal.** Manual snapshots work. Any agent-applied edit takes a `pre_agent_edit` snapshot first.

**Steps.**

1. Implement content-addressed snapshot store under `snapshots/objects/`.
2. Implement `booksforge-storage` snapshot manifest table per `04-data-model §4`.
3. Tauri commands: `snapshot.create({scope, scope_id, label, trigger})`, `snapshot.list({})`, `snapshot.diff({a, b})`, `snapshot.restore({snapshot_id, selective?})`.
4. Wire the orchestrator: before applying any edit (Outline-architect produces a tree creation, which counts as an edit), take a `pre_agent_edit` snapshot.
5. UI: Snapshots panel skeleton with timeline and Restore action.

**Acceptance criteria.**

- A property test throws random apply/restore sequences at the system and asserts no data loss.
- Every `agent_applied_edits` row has a matching pre-edit snapshot whose `created_at < applied_at`.
- Selective restore restores chosen nodes without touching others.

### MZ-07 — Outline Architect → document tree creation

**Goal.** Accepting an outline proposal creates the document tree (Parts → Chapters → Scenes) atomically.

**Steps.**

1. Implement `booksforge-domain::OutlineToTree` — a pure function from `OutlineProposal` to a `NodeTreeDelta`.
2. Implement `project.apply_outline({proposal_id})` that takes a snapshot, applies the delta, and returns the new project state.
3. Wire the New Project Wizard end-to-end: step 3 runs the agent, step 4 applies on Confirm.

**Acceptance criteria.**

- On a 12-chapter outline proposal, the document tree has the right shape with stable ULIDs.
- Reverting the pre-edit snapshot restores the project to its pre-outline state.
- A property test asserts: any outline proposal that schema-validates either produces a valid tree or returns a typed error — never a partial tree.

### MZ-08 — Quick-action presets (Sharpen, Continue, Rephrase)

**Goal.** Single-shot inline presets work in the editor and are recorded in `ai_calls`.

**Steps.**

1. Implement three prompt templates: `sharpen-prose.v1`, `continue-paragraph.v1`, `rephrase.v1`.
2. Tauri command `ai.suggest({preset, scope_text, options})` streaming tokens via events.
3. UI: quick-action bar (`Cmd/Ctrl+K`) with a side-panel diff view, accept/reject/regenerate.
4. Pre-edit snapshot before applying.

**Acceptance criteria.**

- Selecting a paragraph and hitting Sharpen returns suggestions in <8s on the reference hardware with a 7B-Q4 model.
- Cancellation mid-stream aborts and the partial output can be inspected or discarded.
- Audit row is written for every call.

### MZ-09 — Telemetry, logging, and crash reports — all opt-in

**Goal.** Logs exist; nothing leaves the device by default.

**Steps.**

1. Set up `tracing` with a rotating file appender (5 MB × 5).
2. Implement PII redaction filter at the sink: scrub paths, manuscript content, license keys.
3. Add a "Save diagnostic bundle" command that produces a redacted ZIP.
4. Settings UI: telemetry off by default with a clear "what is sent" panel; crash reports off by default.

**Acceptance criteria.**

- A grep test in CI asserts no `tracing::info!`/`error!` includes the manuscript content variable.
- A redaction unit test ensures emails, paths under home, and content are scrubbed.
- With telemetry off, no outbound network call is made (test with a local pcap or a mock network sink).

### MZ-10 — CI gates + reproducibility seed

**Goal.** CI mechanises every guard so we don't drift.

**Steps.**

1. Add `cargo deny check licenses` and `bans` (no GPL static linking).
2. Add a layered-imports lint (`booksforge-domain` cannot import `booksforge-storage`, etc.).
3. Add IPC-codegen drift check (regenerate TS, fail if uncommitted changes).
4. Add a clippy-with-`-D warnings` gate.
5. Add the first reproducibility test: a fixture project + a fixed export profile produces a byte-identical output on two CI runs.
6. Add the first performance budget: cold launch p50 ≤1s on `macos-14`, asserted by a startup probe.

**Acceptance criteria.**

- Each gate runs in CI and gates merges.
- Removing or weakening a gate requires a documented ADR.

---

## 4. After M0: how M1–M6 unfold

Each subsequent milestone has its own task list with the same level of detail, but enumerating them all here is overkill — the patterns established by MZ-01 through MZ-10 carry forward. The sequence is:

**M1 (project & editor).** Full TipTap node set; binder UI; outline view; status bar; word counts; auto-snapshots; recovery hardening; benchmark fixture for cold-open.

**M2 (Ollama + first three agents).** `IntakeAndOutline` end-to-end (real); `Copyedit` workflow; context builder with token budgeting; live run UI; cancel; output validators; `agent_applied_edits` ledger.

**M3 (developmental + continuity).** Deterministic continuity linter in `booksforge-validator`; `DevelopmentalReview` workflow; `ContinuityCheck` workflow with LLM adjudicator; bible auto-extraction; alias handling.

**M4 (templates + validators).** Three templates (Generic Novel, Romance, General Non-Fiction); ≥15 manuscript validators; KDP-eBook validator; pre-export gate; one-click fixes for deterministic issues.

**M5 (export pipeline).** Pandoc sidecar; epubcheck sidecar; DOCX/PDF/EPUB-3 profiles; reproducibility tests; export history.

**M6 (MVP polish).** Accessibility audit; signing & notarisation; beta channel updater; in-app help; onboarding tour.

## 5. Definition of done per milestone

A milestone exits only when all of the following are true:

1. CI is green on the matrix.
2. All cross-cutting guards pass (license, layering, codegen, perf budgets, reproducibility).
3. Acceptance criteria for each task in the milestone pass.
4. The milestone's user-facing flows are demo-able on a clean machine.
5. The relevant docs in this pack are updated.
6. A milestone retro is recorded with anything to roll into the next milestone.

## 6. Risks specific to the MVP build

These risks are tracked in `_deep/12-risk-register.md` plus the additions below. Each has an owner and a mitigation.

| ID | Risk | Mitigation |
|----|------|------------|
| MVP-R1 | Ollama installer changes break our pinned hash | Pin against the latest at MVP start; have a fallback "manual install" path that always works |
| MVP-R2 | A 7B-Q4 model produces consistently bad outlines for non-English books | Curate the registry: Qwen 2.5 7B is the non-English default; the agent's prompt template uses the project's manuscript language as a constraint |
| MVP-R3 | Tauri v2 stable slips on Windows ARM | Pin to v2 RC and test; ship Win x64 first if Windows ARM blocks |
| MVP-R4 | Pandoc sidecar size adds 100+ MB to installer | Acceptable for MVP; revisit reductions in V1.0 |
| MVP-R5 | Agent first-token latency exceeds budget on Windows CPU-only | Detect CPU-only and offer "Slow mode" with a smaller model recommendation; never hide the latency |
| MVP-R6 | Schema-valid but semantically poor agent output | Cross-cutting validators (`EntitySanityCheck`, length, redaction) and a UI that shows the issue rather than hiding it |
| MVP-R7 | Users grant Ollama installer privileges and the OS later blocks it | Detect block and surface a recovery action; never crash |

## 7. Cross-milestone commitments

Every milestone produces or updates:

- Changelog entries (user-visible).
- In-app help (offline content).
- Regression tests (added, never deleted without an ADR).
- Performance benchmark deltas.
- Accessibility audit notes.
- Security checklist updates.
- Documentation in this pack.

A milestone that regresses any guard is rolled back, not paved over.
