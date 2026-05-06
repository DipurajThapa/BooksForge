# Documentation Inventory — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Purpose:** Single-source registry of every spec file, its role, status, and dependencies.

If you add a new doc, add a row here. If you delete or merge a doc, mark the row resolved with the date and the replacement file.

---

## How to read this table

- **Layer** — what kind of doc this is.
- **Role** — `Authoritative` (source of truth), `Companion` (extends another doc), `Superseded` (kept for context but no longer the source of truth), `Reference` (background only).
- **Status** — `Locked` (no edits without ADR), `Active` (open to edits per the change-control rule), `Draft` (not yet finalised), `Stale` (needs an update).
- **Edits needed** — current outstanding work.

## Top-level documentation pack (Claude Code reads from here)

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `CLAUDE.md` (workspace root) | Claude Code instructions | Authoritative | Locked | — | This pack |
| `CLAUDE_CODE_CONTEXT_HARNESS.md` | Claude Code harness | Authoritative | Locked | Update on every locked-decision change | All |
| `CLAUDE_CODE_START_HERE.md` | Claude Code entry | Authoritative | Locked | — | All implementation-pack docs |
| `CLAUDE_CODE_SKILLS_SPEC.md` | Claude Code skills catalog | Authoritative | Locked | — | `CLAUDE.md` |
| `CLAUDE_CODE_HOOKS_SPEC.md` | Claude Code hooks catalog | Authoritative | Locked | — | `CLAUDE.md` |
| `CLAUDE_CODE_SUBAGENTS_SPEC.md` | Claude Code subagents catalog | Authoritative | Locked | — | `AGENTS.md` |
| `README.md` | Index | Reference | Active | Refresh when a new doc lands | All |
| `_deep/00-package-overview.md` | Index | Reference | Stale | Refresh inventory + verification log | All |

## Product / requirements

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `PRODUCT_REQUIREMENTS.md` | Product | Authoritative | Locked | — | `MVP_SCOPE.md` |
| `MVP_SCOPE.md` | Product | Authoritative | Locked | — | `PRODUCT_REQUIREMENTS.md` |
| `BOOK_WORKFLOWS.md` | Product | Authoritative | Locked | — | `AGENTS.md`, `UI_UX_SPEC.md` |
| `UI_UX_SPEC.md` | Product | Authoritative | Locked | — | `PRODUCT_REQUIREMENTS.md` |
| `_deep/01-BRD-business-requirements.md` | Product | Reference | Active | None | — |
| `_deep/02-FSD-functional-specifications.md` | Product | Reference | Active | Cross-reference any new FR | — |

## Architecture

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `TOOLCHAIN.md` | Architecture | Authoritative | Locked | Update when any tool version changes | `ARCHITECTURE.md` |
| `DESIGN_SYSTEM.md` | Architecture | Authoritative | Locked | Update when tokens or component conventions change | `UI_UX_SPEC.md` |
| `ARCHITECTURE.md` | Architecture | Authoritative | Locked | — | `ARCHITECTURE_DECISIONS.md`, `OLLAMA_LOCAL_LLM_SPEC.md` |
| `ARCHITECTURE_DECISIONS.md` | Architecture (ADR index) | Authoritative | Locked | Append-only on new ADR | All architecture docs |
| `DATA_MODEL.md` | Architecture | Authoritative | Locked | Bump `schema_version` on changes | `ARCHITECTURE.md` |
| `MEMORY_SYSTEM.md` | Architecture | Authoritative | Locked | — | `DATA_MODEL.md`, `AGENTS.md` |
| `VOCABULARY_DICTIONARIES.md` | Architecture | Authoritative | Locked | — | `MEMORY_SYSTEM.md` |
| `OLLAMA_LOCAL_LLM_SPEC.md` | Architecture | Authoritative | Locked | — | `ARCHITECTURE.md §5`, `AGENTS.md` |
| `EXPORT_EPUB_SPEC.md` | Architecture | Authoritative | Locked | — | `ARCHITECTURE.md`, `UI_UX_SPEC.md` |
| `EXPORT_EPUB_QA.md` | Architecture | Authoritative | Locked | — | `EXPORT_EPUB_SPEC.md`, `TESTING_STRATEGY.md` |
| `_deep/03-TAD-technical-architecture.md` | Architecture | Reference | Active | None | — |
| `_deep/04-data-model-and-project-format.md` | Architecture | Reference | Stale | Update only when the deep schema is touched | — |
| `_deep/05-workflow-and-dataflow.md` | Architecture | Reference | Active | None | — |
| `_deep/09-export-pipeline.md` | Architecture | Partially superseded | Stale | Marked superseded by `EXPORT_EPUB_SPEC.md` | — |

## Agents

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `AGENTS.md` | Agents | Authoritative | Locked | Add new agent specs as they land | `MEMORY_SYSTEM.md`, `VOCABULARY_DICTIONARIES.md` |
| `_deep/08-ai-integration.md` | Agents | Partially superseded | Stale | Has a status note pointing forward | — |

## Implementation / testing / security

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `IMPLEMENTATION_PLAN.md` | Implementation | Authoritative | Locked | Update at milestone exit | All MVP docs |
| `TESTING_STRATEGY.md` | Testing | Authoritative | Locked | — | All |
| `SECURITY_PRIVACY.md` | Security | Authoritative | Locked | — | `ARCHITECTURE.md` |
| `_deep/10-roadmap-and-phasing.md` | Implementation | Partially superseded | Active | MVP slice superseded; phases 6+ authoritative | — |
| `_deep/11-test-and-validation-strategy.md` | Testing | Reference | Active | None | — |
| `_deep/06-security-privacy-compliance.md` | Security | Reference | Active | None | — |
| `_deep/12-risk-register.md` | Risk | Reference | Active | Add new risks as discovered | All |
| `_deep/13-glossary-and-decision-log.md` | Glossary + ADR | Authoritative | Active | Append-only on new ADRs | All |

## Plugin / post-MVP

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `_deep/07-plugin-architecture.md` | Architecture | Reference (V1.5+) | Active | None — post-MVP | — |

## Harness / change control

| File | Layer | Role | Status | Edits needed | Depends on |
|------|-------|------|--------|--------------|------------|
| `DOCS_INVENTORY.md` | Harness | Authoritative | Active | Update on every doc addition / removal | — |
| `CONSISTENCY_MATRIX.md` | Harness | Authoritative | Active | Update when conflicts are found / resolved | All authoritative docs |
| `CHANGELOG_DOC_REFACTOR.md` | Harness | Authoritative | Active | Append-only on every doc change | — |

## Prompts (Claude Code phase-by-phase prompts)

| File | Layer | Role | Status | Edits needed |
|------|-------|------|--------|--------------|
| `prompts/README.md` | Reference | Active | Update with status note pointing to `IMPLEMENTATION_PLAN.md` |
| `prompts/STATUS.md` | Reference | Active | Replace with a pointer to `IMPLEMENTATION_PLAN.md` |
| `prompts/phase-00-bootstrap.md` | Superseded | Stale | Status note: "see MZ-01 in `IMPLEMENTATION_PLAN.md`" |
| `prompts/phase-01-foundations.md` | Superseded | Stale | Status note: "see MZ-02 / MZ-03" |
| `prompts/phase-02-editor-core.md` | Superseded | Stale | Status note: "see M1" |
| `prompts/phase-03-local-ai.md` | Superseded | Stale | Status note added |
| `prompts/phase-04-export-pipeline.md` | Superseded | Stale | Status note: "see M5; canonical-HTML pipeline" |
| `prompts/phase-05-validators-and-templates.md` | Superseded | Stale | Status note: "see M4" |
| `prompts/phase-06-non-fiction-and-academic.md` | Reference (V1.0) | Active | None |
| `prompts/phase-07-plugin-runtime.md` | Reference (V1.0) | Active | None |
| `prompts/phase-08-linux-and-signing.md` | Reference (V1.0) | Active | None |
| `prompts/phase-09-encryption-and-backups.md` | Reference (V1.0) | Active | None |
| `prompts/phase-10-marketplace-and-cloud-llm.md` | Reference (V1.5) | Active | None |
| `prompts/phase-11-sync.md` | Reference (V1.5) | Active | None |
| `prompts/phase-12-collaboration-v1.md` | Reference (V1.5) | Active | None |
| `prompts/phase-13-plugin-write-capabilities.md` | Reference (V2.0) | Active | None |
| `prompts/phase-14-voice-and-advanced-ai.md` | Reference (V2.0) | Active | None |
| `prompts/phase-15-translator-pack.md` | Reference (V2.0) | Active | None |

## Diagrams

| File | Layer | Role | Status | Edits needed |
|------|-------|------|--------|--------------|
| `diagrams/README.md` | Index | Active | None |
| `diagrams/01-system-context.svg` | Reference | Active | None |
| `diagrams/02-component-architecture.svg` | Reference | Active | None — shows `booksforge-orchestrator`, `booksforge-memory`, `booksforge-vocab`, `booksforge-export-epub` |
| `diagrams/03-dataflow-edit-loop.svg` | Reference | Active | None |
| `diagrams/04-workflow-lifecycle.svg` | Reference | Active | None |
| `diagrams/05-plugin-architecture.svg` | Reference | Active | None — post-MVP |
| `diagrams/06-ai-flow.svg` | Reference | Active | None — shows 9 MVP agents + Ollama HTTP |
| `diagrams/07-export-pipeline.svg` | Reference | Active | None — shows canonical-HTML pipeline |
| `diagrams/08-roadmap-gantt.svg` | Reference | Stale | Refresh after a milestone closes |

## Naming convention

Product name: `BooksForge`. Bundle extension: `*.booksforge/`. Crates: `booksforge-*`. Filesystem workspace folder may be `Booksforge/`; content references resolve to `BooksForge` regardless.
