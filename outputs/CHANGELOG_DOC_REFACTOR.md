# Project State — BooksForge Documentation

> **What this file is.** A compact, forward-looking snapshot of the documentation state Claude Code starts from. Read this once at the start of the project; from then on, follow `CLAUDE.md` and the milestone tasks in `IMPLEMENTATION_PLAN.md`.
>
> The previous name of this file (Documentation Refactor Changelog) was a history of how the docs got here. Claude Code does not need that history. It needs the current state and a clear starting point. That is what follows.
>
> A minimal change-history appendix is at the bottom for humans only.

---

## 1. Where you are

The `outputs/` folder holds the complete documentation BooksForge will be built from. Everything Claude Code needs to start coding is here. There are five groups:

| Group | Location | What it contains |
|-------|----------|------------------|
| Implementation pack | `outputs/*.md` (27 files) | The contract for the first build — read these |
| Deep specs | `outputs/_deep/*.md` (14 files) | Reference material for trade-off rationale and FR-IDs — read on demand |
| Phase prompts | `outputs/prompts/*.md` (18 files) | Phases 00–05 are superseded by `IMPLEMENTATION_PLAN.md`; phases 06+ are V1.0+ work |
| Diagrams | `outputs/diagrams/*.svg` (8 SVGs + README) | Visual companions; prose wins on conflict |
| Claude Code support | `outputs/.claude/` (20 files) | Skill, hook, and subagent stubs that auto-load |

## 2. The state of the locked decisions

Twenty-seven architecture decisions are locked. Don't reverse without an ADR and a stop-and-discuss with the human owner. The full list is in `ARCHITECTURE_DECISIONS.md`. The ones that most shape day-one decisions:

- **D-002** Editor framework — TipTap (ProseMirror-based), with a custom UI.
- **D-003** Sidecar runtime — Rust in-process, with Pandoc / epubcheck / Ollama as external processes.
- **D-004** Project format — `*.booksforge/` directory bundle with SQLite + Markdown mirror + content-addressed snapshots.
- **D-006-revB** Local-LLM runtime — **Ollama** over HTTP on `127.0.0.1:11434`. Embedded llama.cpp is post-V1.0.
- **D-013** Database — SQLite with `sqlx`, WAL mode, schema_version starts at 1 (greenfield).
- **D-016** Agent architecture — bounded swarm, hard caps, approval gates, no tools, no recursion.
- **D-017** ePUB pipeline — canonical-HTML; the editor preview HTML is the export source. Pandoc handles DOCX and PDF only.
- **D-018** Memory and Vocabulary as first-class subsystems.
- **D-020** Hard caps per workflow run: ≤8 calls, ≤10 minutes, ≤200k tokens, ≤3 retries.
- **D-021** Privacy invariant — no content leaves the device by default.
- **D-023** Reproducibility invariant — byte-identical export for fixed inputs.
- **D-024** No `unwrap()` in production paths.
- **D-025** No GPL crate statically linked.
- **D-026** All IPC types codegened; drift fails CI.

## 3. The product, in one frame

BooksForge is a **local-first desktop app for writing, editing, formatting, and exporting books** on macOS 13+ and Windows 10+. The MVP serves a novelist end-to-end with three book modes (fiction, general non-fiction, academic-reduced), three templates (Generic Novel, Romance Novel, General Non-Fiction), and **nine specialised LLM agents** running through Ollama, plus the always-present Orchestrator. Memory and vocabulary subsystems keep prose continuous and human-sounding. The export pipeline produces DOCX, two PDF profiles, and a KDP-validated EPUB-3 — with the EPUB built from the same canonical HTML as the editor preview to eliminate drift.

Linux build, plugin runtime, cloud LLMs, embedded llama.cpp, sync, collaboration, marketplace, voice, translator pack, and tracked-changes round-trip are **all post-MVP**.

## 4. The nine MVP LLM agents

Each one has a fixed role, an input schema, an output schema, a versioned prompt template, declared memory scope, validators, and a user-gate policy. The full specs are in `AGENTS.md §4`.

1. **Project Intake** — turns a free-text idea into a structured `ProjectBrief`.
2. **Outline Architect** — proposes a chapter/scene outline from a brief.
3. **Memory Curator** — maintains book / chapter / entity memory; refreshes summaries on chapter finalise.
4. **Vocabulary Dictionary** — maintains the project-layer vocab dictionary from accepted edits.
5. **Chapter Drafting** — drafts a scene from a synopsis (off by default).
6. **Developmental Editor** — produces structural notes per chapter.
7. **Continuity** — flags name drift, POV violations, timeline issues.
8. **Copyeditor** — mechanical fixes (punctuation, spacing, em-dashes).
9. **Humanization** — surfaces robotic / GenAI prose and proposes human alternatives.

V1.0+ adds another nine agents (Book Strategy, Research Organizer, Chapter Planning, Line Editor, Style Guide, Fact-Check, Formatting, ePUB Export QA, Final Review). Don't build them in MVP.

## 5. Where to start

Read the docs in this order, then start coding.

1. `CLAUDE.md` — the operating contract. Coding standards, hard rules, ask-only-when-blocked.
2. `CLAUDE_CODE_CONTEXT_HARNESS.md` — the compressed map. Reduces token use across sessions.
3. `MVP_SCOPE.md` — what's in / out of the first 16 weeks.
4. `PRODUCT_REQUIREMENTS.md` — target users, journeys, acceptance.
5. `ARCHITECTURE.md` — system design with Ollama-first AI runtime.
6. `AGENTS.md` — the agent catalog. The heart of the product.
7. `IMPLEMENTATION_PLAN.md §3` — pick **MZ-01** and start.

Tier-2 docs (`MEMORY_SYSTEM.md`, `VOCABULARY_DICTIONARIES.md`, `DATA_MODEL.md`, `UI_UX_SPEC.md`, `BOOK_WORKFLOWS.md`, `EXPORT_EPUB_SPEC.md`, `EXPORT_EPUB_QA.md`, `OLLAMA_LOCAL_LLM_SPEC.md`, `TESTING_STRATEGY.md`, `SECURITY_PRIVACY.md`) are read on demand when the task touches their area. `CLAUDE_CODE_START_HERE.md §2` has the full tiered reading list.

## 6. Milestone Zero — the first ten tasks

`IMPLEMENTATION_PLAN.md §3` lists tasks **MZ-01 through MZ-10** in order. Each is one PR, with tests in the same PR. The sequence:

1. **MZ-01** Bootstrap workspace (Tauri v2 + React/TS + Rust + CI on macOS + Windows).
2. **MZ-02** Project bundle creation and opening.
3. **MZ-03** Single-scene editor + autosave + crash recovery.
4. **MZ-04** Ollama HTTP client + Setup Wizard.
5. **MZ-05** Prompt template engine + the Outline Architect Agent.
6. **MZ-06** Snapshots v1 (manual + pre-agent-edit).
7. **MZ-07** Outline Architect → document tree creation.
8. **MZ-08** Quick-action presets (Sharpen / Continue / Rephrase).
9. **MZ-09** Telemetry / logging / redaction (off by default).
10. **MZ-10** CI gates + reproducibility seed.

After MZ-10, **M1** begins (full editor + binder), then **M2 through M6**.

## 7. Hard rules

These are non-negotiable. Each is enforced by a lint, a test, or a code-review gate.

1. **No content leaves the device by default.** Outbound network only on user-initiated Ollama install/pull and the opt-out update check.
2. **No agent writes to the manuscript without user accept + pre-edit snapshot.** No "auto-apply" toggle exists.
3. **No GPL crate** statically linked. Pandoc, epubcheck, Ollama are sidecars or external runtimes.
4. **No untyped IPC.** Tauri commands have typed input / output / tagged-union error. CI fails on Rust↔TS drift.
5. **No `unwrap()` in production paths.** Reserved for tests and `main()`.
6. **No infinite agent loops.** Hard caps per workflow run. Property tests assert termination.
7. **Forward-compatible project format.** Newer reads older; older refuses newer with a clear message.
8. **Performance regressions ≥10%** on any budget fail CI without a justification block.
9. **One PR = one task.** Tests in the same PR. Update relevant docs in the same PR.
10. **Ask only when blocked; otherwise choose the documented default.** Defaults are in the locked-decision list. If something is genuinely silent or contradictory, write to `docs/open-questions.md`, mark `[ASSUMED]` in code, surface in the PR, continue.

## 8. What's ready, day one

- 27 implementation-pack docs, internally consistent, naming uniform, every reference resolvable.
- 27 architecture decisions locked; ADR template in place for new ones.
- 10 MVP agents specified (the original 9 + Final Review Editor) plus 1 internal Proposal Validator (orchestrator-grade), all with prompt templates, typed input/output schemas, cross-cutting validators, memory scopes, user-gate policies, and explicit failure-mode documentation.
- Memory and Vocabulary subsystems specified end-to-end with schema, audit ledgers, Markdown mirrors, and reversibility property tests.
- Canonical-HTML ePUB pipeline specified with EPUBCheck integration, visual regression, golden-file regression, and reproducibility tests.
- Eight Claude Code skills, ten hooks, ten subagents specified with seed stubs in `.claude/`.
- Three architecture diagrams refreshed to reflect the locked decisions; the rest are accurate at the high level.
- A 16-week milestone plan (M0–M6) with the first ten concrete PR-sized tasks defined.

## 9. What still needs human input (open decisions)

- **Bundle Ollama with the installer or require user install?** Today's plan requires user install with guided setup. Bundling is heavier but more frictionless. Confirm before V1.0.
- **Pricing & licensing UI surface.** MVP is licence-free. V1.0 needs to land Pro / Studio per `[DECISION-001]`.
- **Quality bar for the agent-output review.** Schema-validity ≥90% over 50 trials is mechanical; "good prose" is subjective. A writer-panel rubric is needed before V1.0 GA.
- **Diagram 08 (roadmap gantt).** Stale; refresh at every milestone close.
- **Workspace folder name.** Filesystem folder is `Booksforge/`; product is `BooksForge`. Cosmetic; rename at the user's discretion.

---

## Appendix — Change history (compressed, for humans)

| Date | Pass | What changed |
|------|------|--------------|
| 2026-05-06 | Pass 1 | Created the original 9-doc implementation pack. Pivoted local-LLM runtime to Ollama-first ([D-006-revB]). Added bounded agent swarm ([D-016]). |
| 2026-05-06 | Pass 2 | Renamed everything `Bookforge` → `BooksForge`. Created the harness (`DOCS_INVENTORY`, `CONSISTENCY_MATRIX`, `CLAUDE_CODE_CONTEXT_HARNESS`, `CLAUDE.md`). Specced memory ([D-018]) + vocabulary subsystems. Specced canonical-HTML ePUB pipeline ([D-017]). Expanded agent catalog from 12 to 19 (9 MVP). Added Claude Code skills / hooks / subagents specs and `.claude/` seeds. |
| 2026-05-06 | Pass 3 | Moved the 14 deep-spec files into `_deep/`. Refreshed diagrams 02 (component arch), 06 (AI flow), 07 (export pipeline). Updated all cross-references. |
| 2026-05-06 | Pass 4 | Filled the UI surface gap for Memory Curator, Vocabulary Dictionary, and Humanization in `UI_UX_SPEC.md`. Repurposed this file from a refactor changelog to a forward-looking project-state document. |

For a finer-grained log, use `git log` once the repo is initialised at MZ-01.
