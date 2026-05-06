# Testing Strategy — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative test strategy.**

This document specifies the **MVP tests Claude Code must produce alongside code** and the **agent-specific test patterns** that govern every agent.

---

## 1. Testing pyramid

The MVP follows a strict pyramid:

```
                ┌────────────────┐
                │  E2E (Playwright) │   ~10 tests   slow, broad
                └────────────────┘
              ┌────────────────────┐
              │ Integration (Rust)   │   ~50 tests   medium
              └────────────────────┘
            ┌──────────────────────────┐
            │  Unit + property tests     │   hundreds   fast
            └──────────────────────────┘
```

We hold the line: every PR that adds an E2E test must justify why a unit or integration test would not have caught the same bug.

## 2. Layer-by-layer testing

### 2.1 Domain crates (Layer 3) — unit + property

`booksforge-domain`, `booksforge-template`, `booksforge-validator`, `booksforge-agents`, `booksforge-prompt`, `booksforge-export`. Targets: ≥90% line coverage, property tests for critical invariants.

Examples of property tests:

- `booksforge-domain::OutlineToTree`: any schema-valid `OutlineProposal` either produces a valid `NodeTreeDelta` or returns a typed error — never a partial tree.
- `booksforge-prompt`: any well-formed template renders deterministically given the same inputs.
- `booksforge-validator::Continuity`: deterministic linter is idempotent — running it twice on the same input produces identical findings.
- `booksforge-export::PandocAstBuilder`: round-trip ProseMirror → Pandoc-AST → ProseMirror is structure-preserving for a defined node set.

### 2.2 Infrastructure crates (Layer 4) — integration with mocks and real adapters

`booksforge-storage`, `booksforge-fs`, `booksforge-ollama`, `booksforge-orchestrator`, `booksforge-export-pandoc`, `booksforge-epubcheck`.

Each ships:

- **Mock-based unit tests** for the contract.
- **Real-adapter integration tests** that run the actual SQLite, file system, or sidecar binary in a tmpdir.

For Ollama specifically:

- Unit tests use `MockOllamaClient` from `booksforge-test-fixtures` and assert every code path.
- A nightly CI job runs against a real Ollama with a small model (`tinyllama:1.1b-chat-q4_K_M` or smaller) to smoke-test the real HTTP path. This job is non-gating during MVP — it surfaces drift but does not block PRs.

### 2.3 Application services (Layer 2) — integration

Tauri commands tested through the test harness. Each command has at least one happy-path test and at least one error-path test (typed error returned).

### 2.4 Frontend (Layer 1) — component + E2E

- Vitest for component logic and view-models.
- Playwright for E2E against the built Tauri app on each CI matrix entry.

## 3. Agent-specific test patterns

This is the part the deep spec does not yet cover. Six patterns; every agent ships tests in each category.

### 3.1 Prompt-rendering tests

Given a fixed input bundle, the prompt renders to an exact expected string. Snapshot tests with a stable fixture input. Catches accidental prompt drift. Each prompt template version has its own snapshot.

### 3.2 Schema-validation tests

Given a fixture set of (model output → expected validation outcome), the agent's validation step accepts the valid ones and rejects the invalid ones with the right error categories. Includes:

- Schema-valid happy path.
- Missing required field.
- Wrong type.
- Extra unknown field (should be tolerated or rejected per agent — both behaviours are tested explicitly).
- Off-by-one ranges (Copyeditor).
- Out-of-range severities.

### 3.3 Semantic-validator tests

Each agent's semantic validators (`EntitySanityCheck`, length checks, etc.) are unit-tested with positive and negative fixtures. Expand as we discover real-model failure modes.

### 3.4 Orchestrator-with-mock-Ollama tests

A `MockOllamaClient` returns scripted outputs. Test scenarios:

- Happy path: schema-valid output, success.
- Schema-invalid output once → retry with reminder → success.
- Schema-invalid output three times → `proposal_invalid` artifact, no crash.
- Mid-run cancellation → `cancelled` row, partial outputs preserved.
- Network/Ollama unreachable mid-run → `external_error`, partial outputs preserved.
- OOM/context-too-long signal → orchestrator's recovery (shrink context and retry once) verified.
- Output that triggers `EntitySanityCheck` violation → flagged in the proposal, not silently accepted.

### 3.5 Determinism tests

With a deterministic mock (canned output for fixed input), the orchestrator's recorded `agent_runs / agent_tasks / agent_outputs` rows are identical across runs (modulo timestamps and ULIDs). Catches non-determinism leaking into context selection or template rendering.

### 3.6 Cap enforcement tests (the "no infinite loop" tests)

A property test feeds the orchestrator with mocks that always produce schema-invalid output and asserts the run terminates within the cap budget (≤8 calls, ≤200k tokens, ≤10 minutes simulated). Another property test attempts to construct a workflow with > 8 steps and asserts the orchestrator refuses to start it.

### 3.7 Live local-LLM smoke tests (nightly, non-gating)

For each MVP agent, run against a small model (e.g., `phi3.5:latest` or `tinyllama:1.1b-chat-q4_K_M` if quality permits) and assert: the output is **schema-valid** at least 90% of the time across 50 trials. We track the rate over time to detect prompt or model-registry regressions. **Quality** is not asserted by this test — only schema validity.

## 4. Privacy invariant tests

Privacy is enforced by tests, not just lints.

- `no_network_by_default`: with the network mocked-fail at the `reqwest` layer, every MVP feature except `OllamaSetup → Install`, `Ollama.pull`, and `Update.check` works. Checked end-to-end in CI.
- `manuscript_never_leaves`: a grep test confirms no `tracing::*!` or telemetry sink writes a path that contains `manuscript`, `scene`, `node`, or any field carrying body content.
- `redaction_filter`: unit tests assert paths under `~`, emails, license keys, and content blobs are scrubbed before reaching any sink.
- `pcap_assertion`: an integration test boots the app with telemetry off, exercises the full MVP flow, and asserts zero outbound packets after the initial Ollama probe.

## 5. Snapshot invariant tests

- `pre_agent_edit_exists`: every `agent_applied_edits` row has a `snapshots` row whose `created_at < applied_at` (property test over random sequences).
- `restore_round_trip`: snapshot → modify → restore reproduces the prior state byte-for-byte for the persisted columns.
- `selective_restore_isolation`: restoring node A does not affect node B.

## 6. Reproducibility tests

- `export_byte_identity`: a fixture project + fixed export profile + same engine version produces a byte-identical output on two consecutive CI runs. We accept platform-specific differences only for documented file-format quirks.
- `prompt_template_hash_pinning`: every `agent_tasks` row records a `prompt_template_hash` matching the on-disk file at run time; the hash is reproducible across builds.

## 7. Performance tests

Budgets (from `ARCHITECTURE.md §10`) are enforced by CI benches. Tests:

- Cold launch (no project) p50 ≤1s on `macos-14`.
- Open 100k-word fixture project p50 ≤1.5s on `macos-14`.
- Editor keystroke latency p95 ≤30ms (instrumented under a synthetic typing load).
- Validator full-project run ≤10s for the 100k-word fixture.
- EPUB-3 export ≤30s for the 100k-word fixture.
- Agent first-token ≤2s on `macos-14` with a 7B-Q4 model on the nightly job.

A regression > 10% on any budget fails the PR unless the description includes a justification block referencing the bench result.

## 8. Accessibility tests

- axe-core via Playwright on every E2E run for every screen.
- Keyboard-only navigation E2E: complete the New Project Wizard end-to-end without a mouse.
- Reduced-motion mode E2E: animations stop.
- Contrast linter on the design tokens.

## 9. Cross-platform tests

The CI matrix runs on `macos-14` (Apple Silicon), `macos-13` (Intel), `windows-2022`. Linux is V1.0; we run a non-gating `ubuntu-22.04` smoke job during MVP to keep linux-friendliness.

Per-OS specifics tested:

- Bundle filename casing (macOS case-insensitive default vs. Windows).
- Path length limits (Windows MAX_PATH).
- Lockfile semantics across platforms.
- File system event delivery (notify) on each OS.

## 10. Failure injection

Beyond happy paths, we systematically inject failures:

- Disk full during save → typed error, manuscript content not lost.
- Read-only filesystem → typed error, no half-write.
- SQLite WAL corruption → recover or refuse to open with a clear message.
- Process killed mid-export → no half-export visible to user.
- Ollama killed mid-run → typed `external_error`, partial agent outputs preserved as inspectable artifacts.
- Network down during update check → silent failure, no UI noise.

## 11. Test fixtures

`booksforge-test-fixtures` is the shared fixture crate.

- `agent-fixtures/` — input/output pairs for each MVP agent (a `ProjectBrief` → `OutlineProposal` example, etc.).
- `manuscripts/` — small (1k words), medium (30k), large (100k), huge (200k) fixtures in canonical bundle form. Used for performance benches and import/export tests.
- `mock-ollama/` — scripted outputs for `MockOllamaClient`.
- `validators/` — known-bad documents that should produce known issues.

## 12. Coverage targets

- Domain crates: ≥90% line + branch.
- Infrastructure crates: ≥75% line.
- Application services: ≥80% line.
- Frontend: ≥70% on critical paths (editor, agent panel, project picker).

Coverage is tracked but not gated below the targets unless a regression > 5% lands.

## 13. CI orchestration

GitHub Actions workflow shape:

- **PR pipeline.** Matrix build + test + clippy + cargo-deny + ts codegen drift + Vitest + Playwright (smoke). Time budget ≤30 min.
- **Main pipeline.** PR pipeline + full Playwright + benches + reproducibility test + axe full sweep.
- **Nightly.** Live Ollama smoke tests; performance benchmarks with trend tracking; long fixture export.

## 14. Manual test plan (per release)

Even with strong automation, we perform a manual pre-release pass on a clean OS install:

- Install the signed app and Ollama.
- Run the New Project Wizard with the AI path.
- Draft a 5,000-word fixture chapter; run all **nine MVP agents** (Project Intake, Outline Architect, Memory Curator, Vocabulary Dictionary, Chapter Drafting [opt-in], Developmental Editor, Continuity, Copyeditor, Humanization); export to KDP-eBook.
- Confirm no surprising prompts, no telemetry leakage (with telemetry off), no unsigned binaries on disk.

A test reporter signs the manual checklist before the release tag is pushed.

## 15. What we explicitly do not test in MVP

- Real-time collaboration scenarios (post-MVP feature).
- Plugin sandbox edge cases (post-MVP).
- Cloud LLM providers (post-MVP).
- Multi-user concurrency on a shared bundle (post-MVP).
- Embedded llama.cpp (post-MVP runtime).

These have their own test plans when they ship.
