# Product Requirements — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06

This document is the implementation-ready product spec — the contract for the first build.

---

## 1. Vision

A desktop application that helps writers move a book from idea to publication-ready files without sending their manuscript to anyone else's server. Local LLM agents handle the repetitive parts of editing, formatting, and review; the writer stays in control of the words.

## 2. Target users

The MVP serves three primary personas. Each persona uses the same core but with mode-specific templates, agents, and validators.

- **Novelist (Anya).** Writes 60k–120k-word fiction. Cares about pacing, character consistency, prose quality, and KDP/IngramSpark export. Works in long sessions, wants distraction-free editing.
- **Non-fiction author (Theo).** Writes business, self-help, or memoir books of 40k–80k words. Cares about argument structure, readability, citation handling, and clean DOCX for an editor.
- **Academic author (Aisha).** Writes monographs of 80k–150k words with footnotes, citations, figures, and an index. Cares about citation styles, structure, and a press-ready manuscript.

Three secondary personas inform but do not gate the MVP: editors and writing coaches who receive manuscripts (V1.0+), beta readers (V1.0+), and plugin authors (V1.5+).

## 3. Supported book types in the MVP

The MVP supports three book modes only. Each mode is a different combination of templates, agents, validators, and export profiles.

| Mode | Examples | MVP scope |
|------|----------|-----------|
| Fiction | Novel, novella, short-story collection | Full |
| General non-fiction | Memoir, business, self-help, popular science | Full minus citations |
| Academic / research | University-press monograph, thesis-as-book | **Reduced**: footnote handling and CSL citations only; full academic pipeline (index, advanced cross-refs, LaTeX export) is V1.0 |

Children's / illustrated books, technical/manual-style books, cookbooks, and textbooks are **post-MVP**. They have layout and asset-handling requirements that destabilize MVP scope.

## 4. Core user journeys (MVP)

The MVP must support these journeys end-to-end. Each journey is documented in detail in `UI_UX_SPEC.md`.

1. **Start a new book project** — pick a mode, pick a template, name the project, choose a folder. A `*.booksforge/` bundle is created.
2. **Import existing material** — DOCX, Markdown, or plain text. Images come along; structure is inferred.
3. **Generate or refine an outline** — the user provides a one-paragraph premise (or imports an existing outline); the **Outline Architect Agent** proposes a structure; the user accepts/edits.
4. **Draft chapters** — the user writes in a TipTap editor with quick-action presets (Sharpen, Continue, Rephrase) for inline help. Drafts can also be written outside BooksForge and pasted in.
5. **Revise and developmentally edit** — the **Developmental Editor Agent** runs against the manuscript chapter-by-chapter and produces structural notes. The user accepts or rejects each note.
6. **Line edit and copyedit** — the **Line Editor Agent** and **Copyeditor Agent** propose passage-level revisions and mechanical fixes. The user reviews each.
7. **Check continuity, tone, and style** — the **Continuity Agent** flags character/place name drift and POV/tense violations; the **Style Guide Agent** flags voice drift.
8. **Format the manuscript** — choose a template profile (e.g., "Trade Paperback 5×8") and the formatting engine compiles the export source.
9. **Export** — DOCX, PDF, EPUB-3 with at least one store profile (KDP-eBook). Pre-export validators run; errors block, warnings prompt.
10. **Manage versions** — automatic snapshots before any agent-applied edit and on a schedule; manual snapshots on demand; browse and restore.

Journeys for fact-checking-with-citations, multi-language manuscripts, and tracked-changes round-trip with an external editor are **V1.0** journeys, not MVP.

## 5. MVP scope — what ships in the first build

The MVP is the **smallest end-to-end product that is honestly useful for one persona end-to-end**. We pick **Novelist Anya** as the canonical MVP persona. Theo and Aisha are partially served (drafting, editing, and basic export work for them too) but their full workflows ship in V1.0.

### 5.1 In MVP

**Platform**

- Tauri v2 desktop app on **macOS 13+** and **Windows 10+** (x64 and ARM where Tauri supports it).
- Linux is **post-MVP** (V1.0).
- Code signing on macOS and Windows.
- Single-user, single-window, single-project at a time.

**Project & editor**

- Folder-bundle project format (`*.booksforge/`) with SQLite + Markdown mirror.
- TipTap editor with paragraph, heading (H1–H4), bold/italic/underline, blockquote, ordered/unordered lists, code block, image with alt text, footnote (basic), inline comment, link.
- Document tree: Project → Part → Chapter → Scene. Drag-reorder.
- Outline view (hierarchical list with synopsis/POV/status fields per scene).
- Find/replace with regex.
- Word count (project, chapter, scene, today's session).
- Autosave (5s debounce) and crash recovery.
- Snapshots: manual, scheduled (hourly active), and pre-agent-edit (mandatory).
- Distraction-free / focus mode.

**Templates (MVP)**

- **Generic Novel** (default fiction template).
- **Romance Novel** (genre-specific).
- **General Non-Fiction**.

**AI runtime**

- **Ollama** as the primary local LLM runtime, talked to over `127.0.0.1:11434`.
- A guided setup that detects Ollama, offers to install it (downloading from official sources) or shows clear instructions if we cannot install on the user's behalf.
- A curated model list with recommendations: Qwen 2.5 7B, Llama 3.1 8B, Mistral 7B, Gemma 2 9B, Phi-3.5 Mini. Users can use any model Ollama exposes.
- A model-selection UI per project that chooses defaults based on declared book mode and detected hardware (RAM and GPU).
- Inline quick-action presets (Sharpen, Continue, Rephrase, Shorten, Expand) — single-shot calls.

**Agentic swarm (MVP set)**

The MVP includes **nine LLM agents** plus the always-present Orchestrator (controller). Defined fully in `AGENTS.md`.

1. **Project Intake Agent** — turns a free-text idea into a structured project brief (mode, genre, audience, target word count, tone).
2. **Outline Architect Agent** — proposes a chapter/scene outline from a brief.
3. **Memory Curator Agent** — maintains book / chapter / entity memory; refreshes summaries on chapter finalise (per `MEMORY_SYSTEM.md`).
4. **Vocabulary Dictionary Agent** — maintains the project-layer vocabulary dictionary from accepted edits (per `VOCABULARY_DICTIONARIES.md`).
5. **Chapter Drafting Agent** — drafts a scene from a synopsis when the user explicitly requests drafting (off by default).
6. **Developmental Editor Agent** — produces structural notes per chapter.
7. **Continuity Agent** — flags name/place spelling drift, POV/tense issues, timeline contradictions.
8. **Copyeditor Agent** — mechanical fixes (punctuation, capitalisation, double-spaces, em-dash style, comma splices).
9. **Humanization Agent** — surfaces robotic / GenAI prose ("AI-tells") and proposes human alternatives using the merged vocabulary + style memory.

The remaining agents (Book Strategy, Research Organizer, Chapter Planning, Line Editor, Style Guide, Fact-Check, Formatting, ePUB Export QA, Final Review) are **V1.0+**. The Formatting and Export functions in the MVP are **rule-based, not agent-driven** — they call the export pipeline directly. We use agents only where their judgement value outweighs the cost and risk.

**Validators**

- ≥15 manuscript validators (heading hierarchy, broken refs, missing alt text, double spaces, em-dash style, oversized images, etc.).
- One store-targeted validator: **KDP-eBook** (EPUB-3 + KDP-specific rules + epubcheck integration).
- Pre-export validator gate: errors block export; warnings prompt; info silent.

**Export**

- DOCX (manuscript format suitable for sending to an editor).
- PDF (Trade 5×8 and 6×9).
- EPUB-3 (Generic + KDP-eBook profile).

**Settings**

- Theme (light/dark), font, autosave interval, AI on/off per project.
- Telemetry off by default.
- Crash reports off by default.
- License-free (the MVP itself is licence-free; pricing and licensing land in V1.0).

### 5.2 Explicitly post-MVP (V1.0 and beyond)

- Linux build.
- Plugin runtime and SDK (the plugin **architecture document** stays as the post-MVP guide).
- Cloud LLM providers (Anthropic / OpenAI / OpenRouter / OpenAI-compatible).
- Embedded llama.cpp.
- Tracked changes round-trip with Word.
- CSL citation engine + BibTeX/Zotero integration (academic mode upgrade).
- Index generator, cross-references, math rendering.
- IngramSpark, Apple Books, Kobo, Google Play export profiles.
- LaTeX export.
- Encryption at rest (SQLCipher).
- Multi-author / multi-machine licensing.
- Real-time collaboration, sync, presence.
- Marketplace.
- Voice dictation, audiobook export.
- Children's/illustrated/cookbook layout.
- Translator pack.

The full post-MVP roadmap is in `_deep/10-roadmap-and-phasing.md`. The implementation order is in `IMPLEMENTATION_PLAN.md`.

## 6. Functional requirements (MVP delta)

The MVP-tagged FRs in `_deep/02-FSD-functional-specifications.md` are still the source of truth for individual feature requirements. The MVP adds the following swarm-related FRs and tightens a few existing ones.

| ID | Requirement | Notes |
|----|-------------|-------|
| FR-AGENT-001 | The application has a defined Agent Catalog (`AGENTS.md`). At runtime, agents are loaded from a code-defined registry — not from configuration — so capabilities cannot be expanded by data alone. | Hard-coded list in MVP. |
| FR-AGENT-002 | Every agent run produces a row in `agent_tasks` and a row in `agent_outputs`, with the prompt template hash, model, input hash, output hash, and duration. | See `DATA_MODEL.md §5`. |
| FR-AGENT-003 | An agent may not write to the manuscript directly. Outputs are proposals; the user accepts/rejects. Accepting any agent output triggers a pre-edit snapshot. | Enforced in orchestrator. |
| FR-AGENT-004 | The Orchestrator enforces per-workflow caps: ≤8 agent calls per workflow run, ≤10 minutes wall clock, ≤200k tokens generated total, ≤3 retries per agent. | Configurable but capped. |
| FR-AGENT-005 | Every agent has an explicit input schema, output schema, and validation step. Outputs that fail validation are surfaced as agent errors with the raw output for the user to inspect; they are not silently retried. | See `AGENTS.md §3`. |
| FR-AGENT-006 | The Orchestrator must support cancellation; cancelling mid-workflow returns control to the user and preserves any partial outputs as inspectable artifacts (not applied). | UI exposes cancel. |
| FR-AGENT-007 | The Orchestrator never chains agent outputs without a user-confirmation gate when the chain involves a manuscript-mutating step. Read-only chains (e.g., Outline → Developmental notes) may proceed without gates if the user opted into "auto-run analysis." | Read vs. write distinction. |
| FR-OLLAMA-001 | The application detects Ollama on `127.0.0.1:11434` at startup and on a "Refresh models" action. If absent, it shows a guided setup. | See `ARCHITECTURE.md §5`. |
| FR-OLLAMA-002 | The application can pull a model via Ollama's API (`POST /api/pull`) with progress. | Models list curated by us. |
| FR-OLLAMA-003 | The application gracefully handles Ollama process restarts, version mismatches, and missing models with typed errors and recovery actions. | No silent failures. |
| FR-OLLAMA-004 | Inference calls use Ollama's streaming API. Cancellation aborts the HTTP request and signals Ollama to stop generation. | Critical for UX. |
| FR-OLLAMA-005 | The application records the Ollama version and model digest on every `agent_tasks` row for reproducibility. | Audit. |
| FR-MVP-EXPORT-001 | DOCX, PDF (Trade 5×8 and 6×9), and EPUB-3 (Generic + KDP-eBook) export profiles are available. | Three formats, four profiles. |
| FR-MVP-EXPORT-002 | A 100k-word fiction project exports to all four profiles in under 60s on the reference hardware (16 GB Apple-Silicon Mac or 16 GB Win10 laptop). | Hard budget. |

## 7. Non-functional requirements (MVP)

- **Privacy.** No user content leaves the device at runtime. The only outbound network calls in the MVP are: Ollama installer download (one-time, user-initiated), Ollama model pull (user-initiated), and update check (opt-out). Telemetry is off by default and is metadata-only when enabled.
- **Reliability.** Crash-free session rate ≥99% on the beta channel. No data loss on `kill -9` mid-edit (the crash recovery flow restores within a single prompt).
- **Performance.** Cold launch p50 ≤2s; open 100k-word project p50 ≤2s; keystroke latency p95 ≤30ms; full-project validator run ≤10s for 100k words; agent first-token latency ≤2s on the reference hardware with a 7B-Q4 model on Ollama.
- **Accessibility.** Keyboard-only operation for every MVP feature; high-contrast and dark themes; WCAG 2.2 AA contrast; screen-reader basics (full SR support is V1.0).
- **Internationalisation.** UI strings extracted (en in MVP); manuscripts can be in any UTF-8 language; per-project manuscript language drives spell-check and prompt language. Localised UIs land in V1.0.
- **Updates.** Tauri auto-updater with signed packages; user can defer.
- **Hardware floor.** 8 GB RAM minimum (3B-Q4 model usable but slow); 16 GB recommended (7B-Q4 default); 32 GB for 13B and above. CPU-only is supported but flagged as slow.

## 8. What's out of scope at every phase

The "out of scope" list from `01-BRD §14` stays in force: hosted publication or distribution, payment processing for end-readers, DRM, audio/video editing, AI image generation for cover art, social-network features, marketing automation, real-time voice chat. Add to that the MVP-specific exclusions in §5.2.

## 9. Acceptance criteria for the MVP as a whole

The MVP ships when:

1. The seven journeys in §4 work end-to-end on macOS and Windows for a fiction project of 60k–120k words.
2. The **nine MVP agents** in §5.1 run successfully on a clean install with a freshly pulled `qwen2.5:7b-instruct-q4_K_M` model on the reference hardware, and produce non-empty, schema-valid outputs.
3. A 100k-word manuscript exports to DOCX, PDF, and KDP-eBook EPUB-3 in under 60 seconds and passes the bundled KDP validator.
4. Privacy invariant test: with the network disabled, every MVP feature except update check, Ollama install, and Ollama model pull works.
5. Snapshot invariant test: every accepted agent edit produced a `pre_agent_edit` snapshot before applying.
6. A `kill -9` during an editing session results in zero data loss after the next launch's recovery prompt.
7. CI is green on the macOS-14 and windows-2022 matrix entries with all guards passing (`cargo deny`, lints, codegen drift, layered-imports, performance budgets, reproducibility).

## 10. Persona-specific MVP acceptance

- **Novelist Anya:** can complete journeys 1–10 from §4 for a 90k-word romance novel using the Romance template and produce a KDP-eBook EPUB.
- **Non-fiction Theo:** can complete journeys 1–6 and 8–10 (no citations in MVP) for a 60k-word business book using the General Non-Fiction template and produce a manuscript-DOCX an editor accepts.
- **Academic Aisha:** can complete journeys 1–6 with footnotes, but full citation/CSL/BibTeX integration is V1.0; she should not be told the MVP is academic-complete.

This honest scoping is what makes the MVP shippable on a realistic timeline.
