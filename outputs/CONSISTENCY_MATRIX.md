# Consistency Matrix — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Purpose:** Track cross-document consistency. Update when a conflict is found or resolved.

The implementation pack and the deep specs sometimes diverge. This matrix records where they agree, where they disagree, and which file wins.

---

## How to read this table

For each topic:

- **Authoritative file** — single source of truth.
- **Other files that touch it** — and how to read them in light of the authoritative file.
- **Resolution status** — `Aligned`, `Resolved (note in superseded file)`, `Open`.


---

## Topic 1 — Product scope, target users, MVP boundaries

| File | Says |
|------|------|
| **AUTHORITATIVE: `MVP_SCOPE.md`** | Three modes (fiction, non-fiction, memoir, academic-reduced). Three personas. 16-week MVP. Eight MVP agents. Linux deferred. |
| `PRODUCT_REQUIREMENTS.md` | Same scope, more detail on user journeys |
| `_deep/01-BRD-business-requirements.md` | Broader vision; references "fiction mode only" for MVP — note: `MVP_SCOPE.md` widens to include non-fiction and memoir at the cost of full academic features. |
| `_deep/02-FSD-functional-specifications.md` | FR-IDs tagged M / 1.0 / 1.5 / 2.0 are still authoritative for individual feature requirements |

**Resolution status.** `Aligned`. The BRD's "fiction-only MVP" line is widened by `MVP_SCOPE.md` because the agent swarm + memory subsystem makes other modes feasible at MVP without disproportionate cost. **Last verified:** 2026-05-06.

## Topic 2 — Local-LLM runtime

| File | Says |
|------|------|
| **AUTHORITATIVE: `ARCHITECTURE.md §5` + `OLLAMA_LOCAL_LLM_SPEC.md`** | Ollama-first; HTTP API on `127.0.0.1:11434`; embedded llama.cpp post-V1.0 |
| `[DECISION-006-revB]` in `_deep/13-glossary-and-decision-log.md` | Same |
| `08-ai-integration §3` | Carries the original "embedded llama.cpp" framing. Now superseded; status note at top points at the current docs |
| `prompts/phase-03-local-ai.md` | Carries the embedded-llama.cpp framing. Superseded; status note at top points at the current docs |
| `_deep/10-roadmap-and-phasing.md` Phase 3 | Same (says llama.cpp). **Superseded.** Status note at top |

**Resolution status.** `Resolved`. **Last verified:** 2026-05-06.

## Topic 3 — Agent architecture

| File | Says |
|------|------|
| **AUTHORITATIVE: `AGENTS.md`** | 19 agents catalogued, 9 in MVP; bounded orchestrator; per-run caps; pre-edit snapshots |
| `[DECISION-016]` | Same |
| `08-ai-integration §5–9` | Single-shot prompt-preset framing. Compatible with the agent surface: presets are one-shot inline; agents are multi-step. Both surfaces coexist. |
| `02-FSD §4` | FR-AI-001 … FR-AI-015 — about presets. Compatible. |

**Resolution status.** `Aligned`. The two surfaces (presets + agent swarm) are described as separate features. **Last verified:** 2026-05-06.

## Topic 4 — Memory subsystem

| File | Says |
|------|------|
| **AUTHORITATIVE: `MEMORY_SYSTEM.md`** | Book / chapter / entity / style memory; continuously updated; tied to the Memory Curator and Continuity agents |
| `DATA_MODEL.md §5+` | Mirrors the schema additions |
| `[DECISION-018]` | Memory and vocabulary are first-class subsystems |
| `02-FSD §6` (series bible) | Compatible — the entity bible feeds into the entity-memory layer |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

## Topic 5 — Vocabulary dictionaries

| File | Says |
|------|------|
| **AUTHORITATIVE: `VOCABULARY_DICTIONARIES.md`** | Genre/sub-genre/domain/audience dictionaries; continuously evolving; anti-robotic rules; Vocabulary Dictionary Agent |
| `DATA_MODEL.md` (vocab tables) | Schema mirrors the spec |
| `MEMORY_SYSTEM.md` | Style memory cross-references vocab |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

## Topic 6 — UI / UX

| File | Says |
|------|------|
| **AUTHORITATIVE: `UI_UX_SPEC.md`** | MVP screens, Project Picker, New Project Wizard, Workspace, Ollama Setup, AI/Agents panel, Validators, Snapshots, Bible, Export dialog |
| `BOOK_WORKFLOWS.md` | Cross-references the same screens for each workflow stage |
| `02-FSD` user stories | Compatible — user stories are summarised; `UI_UX_SPEC.md` is the screen-level spec |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

## Topic 7 — Data model

| File | Says |
|------|------|
| **AUTHORITATIVE: `DATA_MODEL.md`** | Greenfield `schema_version = 1`; full MVP schema in one migration; agent + memory + vocab tables added |
| `_deep/04-data-model-and-project-format.md` | Notional v5 baseline; superseded for the schema baseline. Still describes the table semantics for `nodes` and `scene_content` |

**Resolution status.** `Resolved`. **Last verified:** 2026-05-06.

## Topic 8 — Export / ePUB pipeline

| File | Says |
|------|------|
| **AUTHORITATIVE: `EXPORT_EPUB_SPEC.md` + `EXPORT_EPUB_QA.md`** | Canonical-HTML pipeline; preview = export source; EPUB-3 packaged by `booksforge-export-epub`; Pandoc only for DOCX/PDF; EPUBCheck mandatory; visual regression in Playwright |
| `[DECISION-017]` | Same |
| `_deep/09-export-pipeline.md` | Pandoc-everywhere description. Superseded for EPUB by the canonical-HTML pipeline; still authoritative for DOCX/PDF profiles |
| `ARCHITECTURE.md §9` | Updated to point at the canonical-HTML pipeline |

**Resolution status.** `Resolved`. **Last verified:** 2026-05-06.

## Topic 9 — Claude Code setup

| File | Says |
|------|------|
| **AUTHORITATIVE: `CLAUDE.md`** | Project-wide rules for Claude Code; reading order; ask-only-when-blocked; defaults |
| `CLAUDE_CODE_CONTEXT_HARNESS.md` | Compressed map; Claude Code loads this first |
| `CLAUDE_CODE_START_HERE.md` | First-day reading list and Milestone Zero |
| `CLAUDE_CODE_SKILLS_SPEC.md` | Skill catalog |
| `CLAUDE_CODE_HOOKS_SPEC.md` | Hook catalog |
| `CLAUDE_CODE_SUBAGENTS_SPEC.md` | Subagent catalog |
| `.claude/skills/`, `.claude/hooks/`, `.claude/agents/` | Seed Markdown stubs that align to the specs |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

## Topic 10 — Prompts / phase prompts

| File | Says |
|------|------|
| **AUTHORITATIVE: `IMPLEMENTATION_PLAN.md §3 (M0–M6)` for MVP, `prompts/phase-06+.md` for V1.0+** | Tasks, acceptance criteria, sequence |
| `prompts/README.md` | Universal guard-rails (G1–G18) — authoritative as the cross-cutting CI rules |
| `prompts/phase-00–05.md` | **Superseded.** Status notes added to each pointing to the milestone equivalent in `IMPLEMENTATION_PLAN.md` |

**Resolution status.** `Resolved`. **Last verified:** 2026-05-06.

## Topic 11 — Diagrams

| File | Says |
|------|------|
| **AUTHORITATIVE: the prose docs** | The text wins on every conflict |
| `diagrams/02-component-architecture.svg` | Reflects the 16-crate MVP layout including `booksforge-orchestrator`, `booksforge-memory`, `booksforge-vocab`, `booksforge-export-epub`. |
| `diagrams/06-ai-flow.svg` | Shows the agent swarm with the 9 MVP agents and Ollama HTTP runtime. |
| `diagrams/07-export-pipeline.svg` | Shows the canonical-HTML pipeline (preview = EPUB content). |
| `diagrams/08-roadmap-gantt.svg` | Stale — based on the 18-week original plan. Refresh at every milestone close. |
| Other diagrams (01, 03, 04, 05) | Active — accurate at the high level. |
| `diagrams/README.md` | Marks active vs. stale per diagram. |

**Resolution status.** `Aligned (08 stale until next milestone close)`. **Last verified:** 2026-05-06.

## Topic 12 — Testing strategy

| File | Says |
|------|------|
| **AUTHORITATIVE: `TESTING_STRATEGY.md`** | Pyramid; agent-specific patterns; privacy invariants; reproducibility; ePUB QA; performance budgets |
| `EXPORT_EPUB_QA.md` | The detailed export QA spec — referenced from `TESTING_STRATEGY.md` |
| `_deep/11-test-and-validation-strategy.md` | Reference doc with the long-term posture; not contradicted |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

## Topic 13 — Security / privacy

| File | Says |
|------|------|
| **AUTHORITATIVE: `SECURITY_PRIVACY.md`** | Privacy invariants; agent-layer risks; controls for MVP |
| `_deep/06-security-privacy-compliance.md` | Reference deep spec; not contradicted |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

## Topic 14 — MVP build window

| File | Says |
|------|------|
| **AUTHORITATIVE: `IMPLEMENTATION_PLAN.md`** | 16 weeks (M0–M6) |
| `MVP_SCOPE.md` | Same |
| `_deep/10-roadmap-and-phasing.md` | Original 18-week MVP. **Superseded** for the MVP slice |
| `01-BRD §11` | "Months 0–4" — coarse-grained reference, consistent with 16 weeks |

**Resolution status.** `Resolved (note in `10-…`)`. **Last verified:** 2026-05-06.

## Topic 15 — Naming

| File | Says |
|------|------|
| **AUTHORITATIVE: this document set** | Product is `BooksForge`; bundle ext is `.booksforge`; crates are `booksforge-*` |
| Filesystem folder | User's workspace folder is `Booksforge/` (lowercase 'f') — content references it correctly; folder rename is the user's call |

**Resolution status.** `Aligned (file content)`. `Open (filesystem folder rename)`. **Last verified:** 2026-05-06.

## Topic 16 — Folder layout

| File | Says |
|------|------|
| **AUTHORITATIVE: `INDEX.md` + `DOCS_INVENTORY.md`** | Implementation pack at `outputs/*.md` (top-level, 27 files). Deep specs at `outputs/_deep/*.md`. Phase prompts at `outputs/prompts/`. Diagrams at `outputs/diagrams/`. Claude Code support at `outputs/.claude/`. |
| Cross-doc links | Top-level docs reference `_deep/NN-...` for deep specs; deep specs reference `../IMPL_PACK.md` for implementation-pack files; prompts reference `../_deep/NN-...`. |

**Resolution status.** `Aligned`. **Last verified:** 2026-05-06.

---

## Topic 17 — Crate list (`booksforge-memory`, `booksforge-vocab`)

| File | Says |
|------|------|
| **AUTHORITATIVE: `ARCHITECTURE.md §3`** | 16 crates including `booksforge-memory` (L3) and `booksforge-vocab` (L3) |
| `CLAUDE_CODE_CONTEXT_HARNESS.md §3` | Same (was already correct) |
| Prior `ARCHITECTURE.md §3` (before 2026-05-06 patch) | Listed 14 crates; omitted `booksforge-memory` and `booksforge-vocab` |

**Resolution status.** `Resolved`. `ARCHITECTURE.md §3` patched 2026-05-06 to add both crates. The harness was already correct. **Last verified:** 2026-05-06.

## Topic 18 — Tool version pins

| File | Says |
|------|------|
| **AUTHORITATIVE: `TOOLCHAIN.md`** | Rust 1.82.0, Node 22.11.0, pnpm 9.12.3, Tauri 2.2.3, TS 5.6.3, Pandoc 3.5, EPUBCheck 5.1.0 |
| `CLAUDE_CODE_CONTEXT_HARNESS.md §4` (prior) | "Rust 1.78+", "TypeScript 5.5+", "Tauri v2 stable, pinned to a specific minor" — vague |
| `CLAUDE_CODE_CONTEXT_HARNESS.md §4` (patched) | Updated to reference `TOOLCHAIN.md` and summary table |

**Resolution status.** `Resolved`. `TOOLCHAIN.md` created 2026-05-06 as authoritative; harness updated. **Last verified:** 2026-05-06.

## Topic 19 — Design system specification

| File | Says |
|------|------|
| **AUTHORITATIVE: `DESIGN_SYSTEM.md`** | Colour tokens, typography, spacing, shadow, radius, motion, iconography, component conventions |
| `UI_UX_SPEC.md` | "Visual fidelity (typography, colour, spacing tokens) is not specified here; the design system lives in `packages/ui/`" — gap acknowledged |

**Resolution status.** `Resolved`. `DESIGN_SYSTEM.md` created 2026-05-06. **Last verified:** 2026-05-06.

---

## Failure-mode walk-through

If Claude Code follows the docs, what could still be ambiguous? Current findings:

1. **Diagrams 02, 06, 07 are now refreshed.** Diagram 08 (roadmap gantt) remains stale until a milestone closes; the prose in `IMPLEMENTATION_PLAN.md` wins on that one.
2. **Phase 06+ prompts assume the older roadmap.** Mitigation: they remain authoritative for V1.0+ tasks; status notes added to phase-00–05.
3. **`docs/open-questions.md` does not yet exist.** Mitigation: Claude Code creates it on first ambiguity per the harness rule. Verified during MZ-01.
4. **The `_deep/09-export-pipeline.md` and `EXPORT_EPUB_SPEC.md` describe two different ePUB pipelines.** Mitigation: status note added; `09-…` is now Pandoc-only for DOCX/PDF.

No remaining contradictions block implementation start.
