# BooksForge Documentation — Index by Purpose

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Purpose:** Visual index of every spec file grouped by what it's for, so you can find the right doc in one glance.

For the canonical listing with statuses and dependencies see **`DOCS_INVENTORY.md`**.
For "what wins where if two docs disagree" see **`CONSISTENCY_MATRIX.md`**.

---

## 🚀 Start here (first read, always)

These four files are the entry points. If you read nothing else, read these.

| File | Purpose |
|------|---------|
| **`CLAUDE.md`** | Operating contract for Claude Code: rules, defaults, ask-only-when-blocked. |
| **`CLAUDE_CODE_CONTEXT_HARNESS.md`** | Compressed map of the project. Reduces token use. |
| **`CLAUDE_CODE_START_HERE.md`** | The reading order, hard rules, and Hello-World. |
| **`README.md`** | High-level overview and document index. |

## 🎯 Product (what we're building)

| File | Purpose |
|------|---------|
| **`PRODUCT_REQUIREMENTS.md`** | Vision, target users, journeys, MVP acceptance criteria. |
| **`MVP_SCOPE.md`** | What's in / out of the first 16-week build. |
| **`BOOK_WORKFLOWS.md`** | Every stage from intake to final QA, with acceptance per stage. |
| **`UI_UX_SPEC.md`** | MVP screens and behaviours. |

## 🏛️ Architecture (how we build it)

| File | Purpose |
|------|---------|
| **`ARCHITECTURE.md`** | System design, layering, crate layout, Ollama integration. |
| **`ARCHITECTURE_DECISIONS.md`** | Every locked decision (D-001 through D-027). |
| **`DATA_MODEL.md`** | SQLite schema, manifest, agent + memory + vocab tables. |
| **`OLLAMA_LOCAL_LLM_SPEC.md`** | Ollama HTTP client, setup wizard, curated model registry. |
| **`EXPORT_EPUB_SPEC.md`** | Canonical-HTML export pipeline (DOCX/PDF/EPUB-3). |

## 🤖 Agent system (the heart)

| File | Purpose |
|------|---------|
| **`AGENTS.md`** | Agent catalog (9 MVP, 19 total), specs, prompts, validators. |
| **`MEMORY_SYSTEM.md`** | Book / chapter / entity / style memory subsystem. |
| **`VOCABULARY_DICTIONARIES.md`** | Layered, continuously evolving dictionaries with anti-robotic rules. |

## ✅ Quality (how we prove it works)

| File | Purpose |
|------|---------|
| **`TESTING_STRATEGY.md`** | Pyramid + agent-specific patterns + privacy invariants. |
| **`EXPORT_EPUB_QA.md`** | ePUB QA checks, golden files, EPUBCheck, visual regression. |
| **`SECURITY_PRIVACY.md`** | Privacy invariants and threat model. |

## 🛠️ Implementation (sequence and process)

| File | Purpose |
|------|---------|
| **`IMPLEMENTATION_PLAN.md`** | Milestones M0–M6 and the first ten Claude Code tasks. |

## 🤝 Claude Code support (skills, hooks, subagents)

| File | Purpose |
|------|---------|
| **`CLAUDE_CODE_SKILLS_SPEC.md`** | Eight invokable review skills. |
| **`CLAUDE_CODE_HOOKS_SPEC.md`** | Ten automated guardrails (pre-edit, pre-commit, post-export). |
| **`CLAUDE_CODE_SUBAGENTS_SPEC.md`** | Ten review subagents. |
| `.claude/skills/<id>/SKILL.md` | Stub for each skill (auto-loaded). |
| `.claude/hooks/HOOKS.md` | Declarative hook list. |
| `.claude/agents/<id>.md` | Stub for each subagent. |
| `.claude/README.md` | Folder overview. |

## 🧭 Harness and change control

| File | Purpose |
|------|---------|
| **`DOCS_INVENTORY.md`** | Registry of every spec file with status and dependencies. |
| **`CONSISTENCY_MATRIX.md`** | What wins where if two docs disagree. |
| **`CHANGELOG_DOC_REFACTOR.md`** | Project-state snapshot for new sessions; minimal change history at the bottom. |
| **`INDEX.md`** | This file — visual index by purpose. |

## 📚 Deep specs (reference; read only when needed)

Long-form reference material. Read on demand for trade-off rationale, FR-IDs, or the full ADR log. The implementation pack above is the source of truth for current behaviour; where a deep spec describes a different approach, a status note at its top points at the current spec.

| File | Purpose | Status |
|------|---------|--------|
| `_deep/00-package-overview.md` | Package overview + cross-doc verification log | Reference |
| `_deep/01-BRD-business-requirements.md` | Why, who, what success | Reference |
| `_deep/02-FSD-functional-specifications.md` | Every FR-ID with phase tag | Reference |
| `_deep/03-TAD-technical-architecture.md` | Components, decisions, trade-offs | Reference |
| `_deep/04-data-model-and-project-format.md` | Schema reference for tables not changed by `DATA_MODEL.md` (greenfield v1) | Reference |
| `_deep/05-workflow-and-dataflow.md` | End-to-end process flows | Reference |
| `_deep/06-security-privacy-compliance.md` | Threat model, controls, GDPR | Reference |
| `_deep/07-plugin-architecture.md` | Plugin sandbox + capabilities | Reference (post-MVP) |
| `_deep/08-ai-integration.md` | AI principles, prompt-template format, audit log details | Reference (current AI runtime: `ARCHITECTURE.md §5` + `AGENTS.md`) |
| `_deep/09-export-pipeline.md` | DOCX / PDF export details (Pandoc) | Reference (EPUB pipeline: `EXPORT_EPUB_SPEC.md`) |
| `_deep/10-roadmap-and-phasing.md` | V1.0+ phase plan | Reference (MVP plan: `IMPLEMENTATION_PLAN.md`) |
| `_deep/11-test-and-validation-strategy.md` | Long-term test posture | Reference |
| `_deep/12-risk-register.md` | 32 risks with mitigations | Reference |
| `_deep/13-glossary-and-decision-log.md` | Glossary + ADR log (append-only) | Authoritative — append new ADRs here |

## 📜 Phase prompts (Claude Code)

`prompts/` holds phase-by-phase prompts.

| Range | Status |
|-------|--------|
| `prompts/phase-00..05.md` | Reference (the MVP plan is `IMPLEMENTATION_PLAN.md §3`, milestones M0–M6). |
| `prompts/phase-06..15.md` | Authoritative for V1.0+ work. |
| `prompts/README.md` | Universal guard-rails (G1–G18) — cross-cutting CI rules. |
| `prompts/STATUS.md` | Phase tracker for V1.0+ phases. |

## 🎨 Diagrams

`diagrams/` holds SVG companions to the prose. Where they conflict, **prose wins**.

| Diagram | Status | Notes |
|---------|--------|-------|
| `01-system-context.svg` | Active | High-level context, actors, opt-in cloud |
| `02-component-architecture.svg` | Active | 4-layer crate map with `booksforge-orchestrator`, `booksforge-memory`, `booksforge-vocab`, `booksforge-export-epub` |
| `03-dataflow-edit-loop.svg` | Active | Edit-loop hot path |
| `04-workflow-lifecycle.svg` | Active | Project state lifecycle |
| `05-plugin-architecture.svg` | Active (post-MVP) | Reserved for V1.0+ |
| `06-ai-flow.svg` | Active | Agent swarm + Ollama HTTP, 9 MVP agents, V1.0+ list |
| `07-export-pipeline.svg` | Active | Canonical-HTML pipeline; preview = export source |
| `08-roadmap-gantt.svg` | Stale | Refresh at every milestone close |

## Folder layout

```
outputs/
├── *.md                          ← 27 implementation-pack files (top-level)
├── _deep/                        ← 14 deep-spec reference files (00–13)
├── prompts/                      ← 18 phase prompts
├── diagrams/                     ← 9 SVGs + README
└── .claude/                      ← Skills, hooks, subagents seed tree
    ├── skills/<id>/SKILL.md      ← 8 skill stubs
    ├── hooks/HOOKS.md
    └── agents/<id>.md            ← 10 subagent stubs
```

## At-a-glance counts

- **27 top-level Markdown files** in `outputs/` (the implementation pack — clean, no numbered prefix interleaving).
- **14 deep-spec files** under `outputs/_deep/`.
- **18 phase prompts** in `outputs/prompts/`.
- **9 SVG diagrams + 1 README** in `outputs/diagrams/`.
- **20 files** under `outputs/.claude/` (1 README + 8 skill stubs + 1 hooks + 10 subagent stubs).
- **27 architecture decisions** (D-001 through D-027) in `ARCHITECTURE_DECISIONS.md` plus the deep ADRs in `_deep/13-glossary-and-decision-log.md`.
- **9 LLM agents in MVP**, 19 total (including the Orchestrator controller and V1.0+ agents).
- **8 Claude Code skills, 10 hooks, 10 subagents**.
- **6 user personas** acknowledged: Anya (MVP target), Theo, Aisha, Editor Emma, Beta-reader Ben, Plugin-author Priya.

## Reading order for a fresh session

If you have **15 minutes**: `INDEX.md` (this file) → `CLAUDE_CODE_CONTEXT_HARNESS.md` → `MVP_SCOPE.md`.

If you have **1 hour**: above + `PRODUCT_REQUIREMENTS.md` + `ARCHITECTURE.md` + `AGENTS.md`.

If you are about to **write code**: above + `CLAUDE.md` + `IMPLEMENTATION_PLAN.md §3` + the area-specific Tier 2 doc(s).

If you are **reviewing a PR**: invoke the relevant Claude Code subagent (per `CLAUDE_CODE_SUBAGENTS_SPEC.md`) — it loads the right specs.
