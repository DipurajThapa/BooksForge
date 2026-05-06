# Claude Code Subagents — Spec

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Defines the ten Claude Code subagents BooksForge uses for review passes.**

A "subagent" here is a focused Claude Code review specialist invoked at well-defined moments in the workflow. They are **review-only** by default — they read, analyse, and produce reports. They do not write code. Where appropriate, they propose specific edits the main agent applies.

These are different from the BooksForge **product** agents (Memory Curator, Copyeditor, etc., in `AGENTS.md`). The product agents run inside BooksForge for end-users; these subagents help us *build* BooksForge.

Seed files: `.claude/agents/<subagent-id>.md`.

---

## 1. Product Requirements Reviewer (`product-requirements-reviewer`)

**Purpose.** Ensure every implementation slice maps to a documented requirement (FR-ID or implementation-pack section). Flag scope creep.

**When to invoke.**

- Before merging any PR that adds new behaviour.
- When a PR description does not cite a requirement.

**Tools allowed.** Read-only access to all `outputs/` docs. No code edits.

**Files to inspect.** `PRODUCT_REQUIREMENTS.md`, `MVP_SCOPE.md`, `_deep/02-FSD-functional-specifications.md`, the PR diff.

**Output format.** A short report:

```
[product-requirements-reviewer]
- PR maps to: <FR-ID(s) or section reference>
- Out-of-scope risk: <none | low | medium | high>
- Recommendation: <merge | clarify | split | defer to V1.0>
- Notes: <one paragraph max>
```

**Success criteria.** Every merged PR has a clear requirement mapping or a documented `[ASSUMED]` note.

---

## 2. Architecture Reviewer (`architecture-reviewer`)

**Purpose.** Validate architectural changes — layer rules, crate boundaries, IPC types, schema migrations, license posture.

**When to invoke.**

- Before merging any PR that touches `ARCHITECTURE.md`, `DATA_MODEL.md`, or adds/removes a crate.
- When a new IPC command, trait, or schema table is introduced.

**Tools allowed.** Read-only.

**Files to inspect.** `ARCHITECTURE.md`, `ARCHITECTURE_DECISIONS.md`, `DATA_MODEL.md`, the PR diff, `Cargo.toml`s.

**Output format.**

```
[architecture-reviewer]
- Layer rule violations: <list or none>
- New IPC commands typed correctly: <yes/no>
- Schema migrations safe: <yes/no/needs ADR>
- License posture: <pass/fail with detail>
- ADR required: <yes (D-NNN) | no>
- Recommendation: <merge | request changes>
```

**Success criteria.** No layer violations; all migrations reversible or documented as one-way.

---

## 3. Documentation Consistency Reviewer (`docs-consistency-reviewer`)

**Purpose.** Cross-check the implementation pack against the deep specs and `CONSISTENCY_MATRIX.md`. Wraps the `docs-consistency-audit` skill.

**When to invoke.**

- Before merging any PR that touches more than two `.md` files.
- After a locked decision is added or revised.

**Tools allowed.** Read-only.

**Output format.** Either PASS or a list of inconsistencies with specific file:line references.

---

## 4. Agentic Workflow Designer (`agentic-workflow-designer`)

**Purpose.** Review proposed new workflows or agent compositions. Ensure caps, gates, schemas, and memory scopes are defined.

**When to invoke.**

- When a new workflow is added to `booksforge-orchestrator`.
- When an existing workflow's caps or gates are loosened.

**Tools allowed.** Read-only.

**Files to inspect.** `AGENTS.md`, the workflow source, `booksforge-agents/`.

**Output format.**

```
[agentic-workflow-designer]
- Caps respected: yes/no
- Approval gates correct for mutation level: yes/no
- Memory scope correctly declared: yes/no
- Validators present: yes/no
- Cancellation handled: yes/no
- Recommendation: merge | request changes
```

**Success criteria.** No workflow can run more than 8 calls per run; per-chapter / per-scene workflows execute as batch-of-runs (per `AGENTS.md §7`).

---

## 5. Memory System Designer (`memory-system-designer`)

**Purpose.** Review memory-touching changes: schema invariants, scope boundaries, audit completeness, recovery.

**When to invoke.**

- Before merging any PR that touches `booksforge-memory`, `MEMORY_SYSTEM.md`, or memory tables.

**Tools allowed.** Read-only.

**Files to inspect.** `MEMORY_SYSTEM.md`, `DATA_MODEL.md`, the PR diff.

**Output format.** PASS or specific findings.

**Success criteria.** Every memory write is scoped, audited, snapshotted, and reversible.

---

## 6. Export Pipeline QA Reviewer (`export-pipeline-qa-reviewer`)

**Purpose.** Validate ePUB / DOCX / PDF pipeline changes against the canonical-HTML invariants and `EXPORT_EPUB_QA.md` checks.

**When to invoke.**

- Before merging any PR that touches `booksforge-export*`, `EXPORT_EPUB_SPEC.md`, or `EXPORT_EPUB_QA.md`.
- Before tagging a release.

**Tools allowed.** Read-only + ability to invoke the `epub-export-qa` skill.

**Files to inspect.** `EXPORT_EPUB_SPEC.md`, `EXPORT_EPUB_QA.md`, the export crates, fixtures, golden hashes.

**Output format.**

```
[export-pipeline-qa-reviewer]
- EPUBCheck: <pass/fail with errors+warnings>
- Visual regression: <pass/fail with delta>
- Reproducibility: <pass/fail>
- Golden file drift: <none | with reason>
- Recommendation: merge | request changes
```

**Success criteria.** Every release passes the full ePUB QA suite on the medium and large fixtures.

---

## 7. Test Strategy Reviewer (`test-strategy-reviewer`)

**Purpose.** Verify that PRs include the right tests for the level of change. Wraps the `test-plan-generator` skill.

**When to invoke.**

- Before merging any PR.

**Tools allowed.** Read-only.

**Files to inspect.** `TESTING_STRATEGY.md`, the PR diff, existing test files for the affected modules.

**Output format.**

```
[test-strategy-reviewer]
- Required test types added: <list>
- Coverage delta: <number>
- Negative-path tests present: yes/no
- Recommendation: merge | add tests
```

**Success criteria.** No PR merges without tests in the same PR (per `CLAUDE.md §6`).

---

## 8. Security & Privacy Reviewer (`security-privacy-reviewer`)

**Purpose.** Verify that no change weakens the privacy invariants or introduces a security risk.

**When to invoke.**

- Before merging any PR that touches network code, `reqwest`, `tracing` sinks, telemetry, or `manifest.toml.[ai]`.
- Before tagging a release.

**Tools allowed.** Read-only + a CI test invocation (the privacy invariant tests).

**Files to inspect.** `SECURITY_PRIVACY.md`, `_deep/06-security-privacy-compliance.md`, the PR diff.

**Output format.**

```
[security-privacy-reviewer]
- Privacy invariants maintained: yes/no
- New outbound endpoints: <list or none>
- New trust boundaries: <list or none>
- Redaction filter still complete: yes/no
- Recommendation: merge | request changes
```

**Success criteria.** No outbound network call without an explicit user-initiated trigger; no manuscript content in any log or telemetry sink.

---

## 9. UI/UX Flow Reviewer (`ui-ux-flow-reviewer`)

**Purpose.** Verify that UI changes align with `UI_UX_SPEC.md` — accessibility, keyboard support, error messages, screen states.

**When to invoke.**

- Before merging any PR that touches `apps/desktop/src-ui` or `packages/ui`.
- After a user-flow change in `BOOK_WORKFLOWS.md`.

**Tools allowed.** Read-only + ability to run Playwright accessibility checks.

**Files to inspect.** `UI_UX_SPEC.md`, the changed UI source, Playwright tests.

**Output format.**

```
[ui-ux-flow-reviewer]
- Keyboard reachable: yes/no
- WCAG 2.2 AA contrast: pass/fail
- Error states defined: yes/no
- Reduced-motion respected: yes/no
- Recommendation: merge | request changes
```

**Success criteria.** Every actionable element is keyboard-reachable; every screen has an empty state and an error state.

---

## 10. Prompt Library Reviewer (`prompt-library-reviewer`)

**Purpose.** Review prompts in `prompts/` and `templates/prompts/`. Wraps the `prompt-library-review` skill.

**When to invoke.**

- Before merging a PR that adds or modifies a prompt.
- After an agent's `prompt_template_id` is bumped.

**Tools allowed.** Read-only.

**Files to inspect.** The prompts; `AGENTS.md`; `MEMORY_SYSTEM.md`; `VOCABULARY_DICTIONARIES.md`.

**Output format.** Pass/fail per prompt with specific findings.

**Success criteria.** Every prompt declares its full metadata; user content is fenced; output schema is exact JSON.

---

## When to chain subagents

Some PRs trigger multiple subagents. Default chain by PR class:

- **Architecture PR.** `architecture-reviewer` → `docs-consistency-reviewer` → `test-strategy-reviewer` → `security-privacy-reviewer`.
- **New agent PR.** `agentic-workflow-designer` → `memory-system-designer` (if memory-touching) → `prompt-library-reviewer` → `test-strategy-reviewer`.
- **Export PR.** `export-pipeline-qa-reviewer` → `test-strategy-reviewer` → `docs-consistency-reviewer`.
- **UI PR.** `ui-ux-flow-reviewer` → `test-strategy-reviewer`.
- **Documentation-only PR.** `docs-consistency-reviewer` → `product-requirements-reviewer`.

The chain runs sequentially. Any subagent that returns "request changes" pauses the merge.

## Subagents are advisory, not gating

Their reports inform the human reviewer. The human reviewer (or in the early days, the project owner) is the merge authority. Subagents accelerate review, surface what was missed, and document the rationale.

## Where subagents live

`.claude/agents/<subagent-id>.md` ships at MZ-01 with a stub for each. The stub references this spec by section.
