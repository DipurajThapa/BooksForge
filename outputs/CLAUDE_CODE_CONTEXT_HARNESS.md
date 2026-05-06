# Claude Code — Context Harness

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Purpose:** A compressed map of BooksForge so Claude Code can act without re-reading every file.

Read this file first whenever you start a fresh session. It is intentionally compact — load this, then load only the specific deep specs you need for the task at hand. Do not load every `.md` file by default; that wastes tokens.

---

## 1. Project summary

BooksForge is a **local-first desktop application** that helps writers go from idea to publication-ready files (DOCX, PDF, EPUB-3) without sending their manuscript to anyone else's server.

It runs on **macOS 13+ and Windows 10+** in MVP (Linux V1.0). It uses **Tauri v2 + React + Rust** for the app shell, **SQLite + a Markdown mirror** for storage, **TipTap** for the editor, **Ollama** as the local LLM runtime, **Pandoc** as a sidecar for DOCX/PDF/EPUB export, and **EPUBCheck** for EPUB validation.

The product's identity is a **bounded agent swarm** of specialized local-LLM agents (intake → outline → drafting → developmental → continuity → copyedit → review) with strong memory (book/chapter/character/style) and **continuously evolving vocabulary dictionaries** by genre, sub-genre, domain, and audience to keep prose human-sounding.

## 2. Locked architectural decisions

These are settled. Reverse only with an ADR and a stop-and-discuss with the human owner. Full text in `ARCHITECTURE_DECISIONS.md` and `_deep/13-glossary-and-decision-log.md`.

| # | Decision | Why |
|---|----------|-----|
| D-001 | Tauri v2 + React + Rust | Single binary, signed, fast, sandboxable |
| D-002 | TipTap (ProseMirror-based) editor | Mature, headless, novel-scale performance |
| D-003 | Rust sidecar for app services | Single binary, fast startup, easy embedding |
| D-004 | `*.booksforge/` folder bundle | Recoverable, git-friendly, inspectable |
| D-005 | Pandoc as sidecar (not linked) | Avoid GPL contamination |
| D-006-revB | **Ollama-first** local LLM | Smaller binary, stable HTTP API, supports Qwen/Llama/Mistral/Gemma/Phi |
| D-007 | SQLite + Markdown mirror | Source of truth + recovery surface |
| D-008 | WASM plugins (post-MVP) | Sandboxed extensibility |
| D-016 | Bounded agent swarm | Specialized agents with caps, gates, snapshots |
| D-017 | **Canonical-HTML ePUB pipeline** | Editor preview = export source; no drift |
| D-018 | Memory + Vocabulary as first-class subsystems | Continuity and human-sounding prose |

## 3. Key directories

```
booksforge/                              ← Repo root (created at MZ-01)
├── apps/desktop/                        ← Tauri app (Rust + React)
├── crates/                              ← Rust workspace
│   ├── booksforge-domain/               L3 — types, document tree
│   ├── booksforge-template/             L3 — template parsing
│   ├── booksforge-validator/            L3 — validators
│   ├── booksforge-agents/               L3 — agent registry + specs
│   ├── booksforge-prompt/               L3 — MiniJinja template engine
│   ├── booksforge-memory/               L3 — book/chapter/style memory
│   ├── booksforge-vocab/                L3 — vocabulary dictionaries
│   ├── booksforge-export/               L3 — canonical HTML export DOM
│   ├── booksforge-ipc/                  IPC types (codegen → TS via ts-rs)
│   ├── booksforge-storage/              L4 — SQLite adapter
│   ├── booksforge-fs/                   L4 — bundle filesystem adapter
│   ├── booksforge-ollama/               L4 — Ollama HTTP client
│   ├── booksforge-orchestrator/         L4 — agent orchestration
│   ├── booksforge-export-epub/          L4 — canonical HTML → EPUB packager
│   ├── booksforge-export-pandoc/        L4 — Pandoc sidecar (DOCX/PDF only)
│   ├── booksforge-epubcheck/            L4 — epubcheck sidecar
│   └── booksforge-test-fixtures/        Shared fixtures
├── packages/                            ← TS workspace
│   ├── ui/                              Design system
│   ├── editor/                          TipTap wrapper
│   ├── preview/                         WebView preview renderer (shared with export)
│   └── shared-types/                    Generated TS types
├── templates/                           ← Built-in templates (TOML)
├── docs/                                ← The spec (this folder)
├── .claude/                             ← Claude Code skills/hooks/agents
└── .github/                             ← CI workflows
```

## 4. Build stack

Exact version pins are in `TOOLCHAIN.md` (authoritative). Summary:

| Tool | Pinned version | Note |
|------|---------------|------|
| Rust | `1.82.0` | Edition 2021; MSRV; `rust-toolchain.toml` |
| Node.js | `22.11.0` LTS | `.nvmrc` |
| pnpm | `9.12.3` | `packageManager` in root `package.json` |
| Tauri | `2.2.3` | Exact patch; must match `tauri-build` crate |
| TypeScript | `5.6.3` | `strict: true`; `noUncheckedIndexedAccess` |
| Pandoc sidecar | `3.5` | GPL; sidecar only |
| EPUBCheck sidecar | `5.1.0` | SHA-256 verified at startup |

- `cargo deny` enforces licenses (no GPL static link). `cargo clippy --all-targets -- -D warnings` is a CI gate.
- `pnpm typecheck` and `pnpm lint` are CI gates. Generated TS types are committed; drift fails CI.
- **CI matrix**: `macos-14`, `macos-13`, `windows-2022` (gating). Non-gating `ubuntu-22.04` smoke job prevents Linux drift.

## 5. MVP scope (single source of truth: `MVP_SCOPE.md`)

**In MVP** (16 weeks, M0–M6):

- macOS + Windows desktop with Tauri v2.
- `*.booksforge/` projects with SQLite + Markdown mirror + snapshots.
- TipTap editor with the MVP node set; binder + outline view; find/replace; autosave; crash recovery.
- 3 templates: Generic Novel, Romance Novel, General Non-Fiction.
- Ollama HTTP integration with guided setup; curated model registry (Qwen 2.5 7B, Llama 3.1 8B, Mistral 7B, Gemma 2 9B, Phi-3.5 Mini).
- Quick-action presets: Sharpen, Continue, Rephrase, Shorten, Expand.
- **Nine MVP agents** (six were planned originally; three added: Memory Curator, Vocabulary Dictionary, Humanization): Project Intake, Outline Architect, Memory Curator, Vocabulary Dictionary, Chapter Drafting (opt-in), Developmental Editor, Continuity, Copyeditor, Humanization. See `AGENTS.md`.
- **Memory subsystem** (`booksforge-memory`): book + chapter + character + style memory tables.
- **Vocabulary subsystem** (`booksforge-vocab`): genre/sub-genre/domain/audience-aware dictionaries with a Vocabulary Dictionary Agent.
- ≥15 manuscript validators + KDP-eBook validator + EPUBCheck.
- **Canonical-HTML export pipeline**: same HTML for preview and EPUB; Pandoc only for DOCX/PDF.
- DOCX (manuscript), PDF (Trade 5×8 + 6×9), EPUB-3 (Generic + KDP-eBook).
- Pre-export gate; export history.
- Telemetry off by default; crash reports off by default; PII-redaction at log sinks.

**Not in MVP** (do not build): Linux build, plugin runtime, cloud LLMs, embedded llama.cpp, tracked-changes round-trip, CSL citations, IngramSpark/Apple/Kobo profiles, LaTeX export, encryption at rest, sync, real-time collaboration, marketplace, voice dictation, translator pack, children's-book layout.

## 6. Development rules (non-negotiable)

1. **No content leaves the device by default.** Outbound network only on user-initiated Ollama install/pull and the opt-out update check.
2. **No agent writes to manuscript without user accept + pre-edit snapshot.** No "auto-apply" toggle exists.
3. **No GPL crate** statically linked. Pandoc, epubcheck are sidecars.
4. **No untyped IPC.** Tauri commands have typed input/output/tagged-union error. CI fails on TS-Rust drift.
5. **No `unwrap()` outside `#[cfg(test)]` and `main()`.** Lints enforce.
6. **No infinite agent loops.** Hard caps: ≤8 calls, ≤10 min, ≤200k tokens, ≤3 retries per workflow run.
7. **Forward-compatible project format.** Newer reads older; older refuses newer with a clear message.
8. **Performance regression ≥10%** on any budget fails CI without a justified explanation in the PR.
9. **One PR = one task.** Tests in the same PR. Update relevant docs in the same PR.
10. **Ask only when blocked; otherwise choose the documented default.** All MVP defaults are in the locked-decision files. If something is genuinely silent or contradictory, write a question in `docs/open-questions.md`, make a defensible best-guess marked `[ASSUMED]` in code, surface it in the PR, and continue.

## 7. Testing rules

- **Unit + property tests** at Layer 3 (≥90% line coverage).
- **Integration tests** at Layer 4 (mocks + real adapters).
- **Agent-specific patterns**: prompt-render snapshot, schema-validation, semantic-validator, orchestrator-with-mock-Ollama, determinism, cap-enforcement.
- **Privacy invariants**: pcap assertion, no-network-by-default, manuscript-never-leaves grep, redaction filter unit test.
- **Snapshot invariants**: every applied agent edit references a pre-edit snapshot whose `created_at < applied_at`.
- **Reproducibility**: same input + same template + same engine version = byte-identical output for DOCX, EPUB, and PDF (modulo timestamps where unavoidable).
- **ePUB QA**: golden-file regression on a 30-chapter fixture; EPUBCheck must pass; preview-vs-export visual diff under tolerance.
- **Live local-LLM smoke** (nightly, non-gating): each MVP agent against `phi3.5` produces schema-valid output ≥90% over 50 trials.

Full strategy in `TESTING_STRATEGY.md`.

## 8. Agent architecture summary

**19 agents specified** (see `AGENTS.md`). MVP ships **9 LLM agents** (plus the always-present Orchestrator controller):

`intake`, `outline-architect`, `memory-curator`, `vocab-dictionary`, `chapter-drafter` (opt-in), `dev-editor`, `continuity`, `copyeditor`, `humanization`.

V1.0 adds: `book-strategy`, `research-organizer`, `chapter-planner`, `line-editor`, `style-guide`, `fact-check`, `formatting`, `epub-export-qa`, `final-review`, `orchestrator` (the orchestrator is always present; it is listed for completeness as the controller).

**Properties**:

- Each agent: name, purpose, input schema, output schema, prompt template (versioned, hash-pinned), model preference, allowed memory reads, allowed memory writes, when to run, user-gate policy, failure modes, validators.
- No agent writes the manuscript. Agents return proposals.
- Orchestrator enforces caps, approval gates, batch-of-runs for per-chapter workflows.
- Audit ledger: `agent_runs`, `agent_tasks`, `agent_outputs`, `agent_applied_edits`. Pre-edit snapshot mandatory before any apply.

## 9. Memory architecture summary

**Three layers** (see `MEMORY_SYSTEM.md`):

1. **Book memory** — title, mode, genre, sub-genre, audience, tone, voice, POV, tense, structure, themes, core promise, canonical style rules.
2. **Chapter memory** — summary, purpose, key events/arguments, characters/concepts introduced, timeline, setting, open loops, resolved loops, terminology used, continuity notes, revision history.
3. **Entity memory** — characters / concepts / claims / sources / definitions / frameworks / case studies / acronyms (typed per book mode).

**Style memory** — tone, sentence rhythm, reading level, narrative distance, formality, humor level, emotional intensity, repeated phrases, banned phrases, overused constructions, humanization rules.

The **Memory Curator Agent** maintains book + chapter + entity memory; the **Vocabulary Dictionary Agent** maintains the dictionaries; the **Continuity Agent** consults them; the **Humanization Agent** uses style memory + vocabulary to make prose human-sounding.

Memory is **continuously updated** at every chapter save, every accepted agent edit, and every chapter finalization. There is no one-time setup.

## 10. Vocabulary architecture summary

**Continuously evolving dictionaries** (see `VOCABULARY_DICTIONARIES.md`) keyed by:

- Genre (e.g., romance, mystery, sci-fi, business, memoir).
- Sub-genre (e.g., romance.regency, mystery.cosy).
- Domain (e.g., software, healthcare, law, finance, history).
- Audience (e.g., adult-trade, YA, academic, children, beginner-friendly business).
- Character voice (fiction/memoir).
- Chapter type (e.g., action, exposition, reflection, how-to).

**Each dictionary** stores: preferred words/phrases/idioms, words/phrases to avoid, robotic-AI words to avoid, overused transitions, audience-specific vocabulary, domain terminology, replacement suggestions, usage examples, last-updated, source of update.

**Anti-robotic rules** ban (contextually) phrases like "in today's world", "delve", "tapestry", "unlock", "transformative", "seamless", "robust", "leverage" (when unnecessary), "it's important to note", "whether you're", "not only … but also", and others. The bans are **contextual** — a tech book may legitimately use "robust"; a romance novel should not. The dictionaries decide.

## 11. ePUB / export architecture summary

**Decision** (`[DECISION-017]`): a **canonical-HTML pipeline** where the editor preview HTML and the EPUB content HTML are byte-identical (or visually identical under documented tolerance). See `EXPORT_EPUB_SPEC.md` and `EXPORT_EPUB_QA.md`.

- **DOCX and PDF** go through Pandoc as a sidecar.
- **EPUB-3** is built directly from the canonical HTML by `booksforge-export-epub` (Rust crate using `epub-builder` or equivalent), with a controlled stylesheet shared with the preview.
- **EPUBCheck** runs against every export; errors block, warnings prompt.
- **Visual regression**: Playwright renders the preview and the unzipped EPUB content with the same WebView; pixel-diff under tolerance gates merges.
- **Golden files**: a 30-chapter fixture exports to a hash-stable EPUB; CI fails on drift without a baseline-update commit.

This directly addresses the "downloaded ePUB doesn't match preview" problem: the preview **is** the export.

## 12. First implementation tasks (read `IMPLEMENTATION_PLAN.md` for the full list)

1. **MZ-01** Bootstrap Cargo workspace + Tauri v2 + React/TS + CI (3 OSes).
2. **MZ-02** Project bundle creation/opening (`*.booksforge/`).
3. **MZ-03** Single-scene TipTap editor + autosave + crash recovery.
4. **MZ-04** Ollama HTTP client + Setup Wizard.
5. **MZ-05** Prompt template engine + Outline Architect Agent (mocked Ollama OK).
6. **MZ-06** Snapshots v1 (manual + pre-agent-edit).
7. **MZ-07** Outline Architect → document tree creation flow.
8. **MZ-08** Quick-action presets (Sharpen / Continue / Rephrase).
9. **MZ-09** Telemetry/logging/redaction (off by default).
10. **MZ-10** CI gates + reproducibility seed.

After MZ-10, Milestone 1 begins (full editor + binder).

## 13. Files Claude Code should read first (priority order)

For any new session:

1. `CLAUDE.md` (at workspace root) — operating instructions.
2. **This file** (`CLAUDE_CODE_CONTEXT_HARNESS.md`).
3. `CLAUDE_CODE_START_HERE.md` — the entry doc with reading order.
4. `MVP_SCOPE.md` — what's in/out for the first build.
5. `IMPLEMENTATION_PLAN.md §3` — current task list.

Then load only the doc(s) relevant to the task at hand. Examples:

- Implementing an agent → `AGENTS.md` (just the section for that agent).
- Touching the editor → `UI_UX_SPEC.md §5` + `ARCHITECTURE.md §3`.
- Schema changes → `DATA_MODEL.md`.
- Memory changes → `MEMORY_SYSTEM.md`.
- Vocabulary changes → `VOCABULARY_DICTIONARIES.md`.
- ePUB changes → `EXPORT_EPUB_SPEC.md` and `EXPORT_EPUB_QA.md`.
- Security check → `SECURITY_PRIVACY.md` + relevant section of `_deep/06-security-privacy-compliance.md`.

## 14. Files Claude Code should NOT read by default

These are large, dense, and only needed for specific deep dives:

- `_deep/01-BRD-business-requirements.md` (read only for product context)
- `_deep/02-FSD-functional-specifications.md` (read only when looking up an FR-ID)
- `_deep/03-TAD-technical-architecture.md` (read only for trade-off rationale)
- `_deep/06-security-privacy-compliance.md`, `_deep/07-plugin-architecture.md` (deep specs; consult only as needed)
- `_deep/08-ai-integration.md` (partially superseded; consult only for the prompt-template format)
- `_deep/10-roadmap-and-phasing.md` (MVP slice superseded; phases 6+ are reference)
- `prompts/phase-*.md` (superseded by `IMPLEMENTATION_PLAN.md`; keep for V1.0 phases 6+)
- `diagrams/*.svg` (load only when you need a visual)

## 15. Token-efficiency rule

If the answer to a question is in the harness, **do not load the deep file**. If you find yourself reading more than three full deep specs in one session, stop and ask whether the harness is missing something — that's a signal to update the harness rather than re-read everything.

## 16. When you're stuck

1. Search `docs/open-questions.md` (will be created at MZ-01).
2. Check `ARCHITECTURE_DECISIONS.md` and `_deep/13-glossary-and-decision-log.md` for an existing answer.
3. Consult `CONSISTENCY_MATRIX.md` to see whether two docs disagree.
4. If still stuck, write the question in `docs/open-questions.md` with `[ASKED-YYYY-MM-DD]`, make a documented best-guess in code marked `[ASSUMED]`, and surface it in the PR description.

Do not block. Do not invent unconstrained features. Surface and proceed.
