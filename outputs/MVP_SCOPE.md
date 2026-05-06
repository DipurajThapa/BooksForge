# MVP Scope — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for what's in and out of the first build.** Companion to `PRODUCT_REQUIREMENTS.md` (full spec) and `IMPLEMENTATION_PLAN.md` (build order).

This is the contract for the first 16 weeks. If a feature is not on this list, **do not build it**.

---

## 1. The MVP promise (one paragraph)

A novelist on macOS or Windows can install BooksForge, install Ollama via the guided setup, pull a recommended local model, create a Romance Novel project from a template, draft 60k–120k words with TipTap, run nine specialised local-LLM agents (intake → outline → memory curator → vocabulary dictionary → opt-in drafter → developmental editor → continuity → copyeditor → humanization) with full continuity and human-sounding prose, and export to a KDP-eBook-validated EPUB-3 (plus DOCX, PDF) where the downloaded EPUB matches the in-app preview byte-for-byte (modulo trivial anti-aliasing tolerances). Nothing leaves their device.

## 2. In MVP

### 2.1 Platform

- Tauri v2 desktop app on **macOS 13+** and **Windows 10+**, x64 and Apple Silicon.
- Linux is **post-MVP** (V1.0).
- Code-signed installers on both OSes (Apple Developer ID + notarisation; Microsoft EV cert).
- Single-user, single-window, single-project at a time.

### 2.2 Project + editor

- Folder-bundle project format (`*.booksforge/`) with SQLite + Markdown mirror + content-addressed assets + content-addressed snapshots.
- TipTap editor with paragraph, headings (H1–H4), bold/italic/underline, blockquote, ordered/unordered lists, code block, image with alt text, footnote (basic), inline comment, link.
- Document tree: Project → Part → Chapter → Scene. Drag-reorder.
- Outline view (hierarchical list with synopsis/POV/status/target word count fields per scene).
- Find/replace with regex.
- Word count rollups (project, chapter, scene, today's session, by author).
- Autosave (5s debounce after last keystroke or on blur) and crash recovery.
- Snapshots: manual, scheduled (hourly during active sessions), and pre-agent-edit (mandatory).
- Distraction-free / focus mode.

### 2.3 Templates

Three built-in:

- **Generic Novel** (default fiction).
- **Romance Novel** (genre-specific with starter vocabulary and beats).
- **General Non-Fiction**.

### 2.4 AI runtime

- **Ollama** as the primary local LLM runtime, talked to over `127.0.0.1:11434`.
- Guided **OllamaSetupWizard**: detect, install (with pinned hash), launch, pull a curated default model.
- Curated model registry: Qwen 2.5 7B, Llama 3.1 8B, Mistral 7B, Gemma 2 9B, Phi-3.5 Mini.
- Model-selection UI per project; defaults driven by mode + detected hardware.

### 2.5 Quick-action presets (single-shot inline)

- Sharpen, Continue, Rephrase, Shorten, Expand.
- Pre-edit snapshot before any apply.
- Cancellable; audit row recorded.

### 2.6 Agentic swarm — nine MVP agents

(Per `AGENTS.md`.)

1. Project Intake.
2. Outline Architect.
3. Memory Curator.
4. Vocabulary Dictionary.
5. Chapter Drafting (opt-in, off by default).
6. Developmental Editor.
7. Continuity.
8. Copyeditor.
9. Humanization.

Plus the always-present **Orchestrator** (controller).

### 2.7 Memory subsystem

(Per `MEMORY_SYSTEM.md`.)

- Book / chapter / entity / style memory tables.
- Markdown mirror under `manuscript/.memory/`.
- Continuously updated by the Memory Curator on chapter finalise.

### 2.8 Vocabulary subsystem

(Per `VOCABULARY_DICTIONARIES.md`.)

- Layered dictionaries: project / genre / sub-genre / domain / audience / character-voice / chapter-type.
- Built-in starter dictionaries for the three MVP templates plus the audience and AI-tells layers.
- Vocabulary Dictionary Agent updates the project-layer dict from accepted edits and proposals.
- Anti-robotic rules baseline.

### 2.9 Validators

- ≥15 manuscript validators (heading hierarchy, broken refs, missing alt text, double spaces, em-dash style, oversized images, unmatched quotes, etc.).
- One store-targeted validator: **KDP-eBook**.
- EPUBCheck integration.
- Pre-export validator gate: errors block; warnings prompt; info silent.

### 2.10 Export pipeline

(Per `EXPORT_EPUB_SPEC.md`.)

- **Canonical-HTML pipeline** for EPUB-3.
- DOCX (manuscript profile via Pandoc).
- PDF (Trade 5×8 + 6×9 via Pandoc).
- EPUB-3 (Generic + KDP-eBook profile via `booksforge-export-epub`).
- EPUBCheck mandatory.
- Visual regression tests.
- Reproducibility tests.
- Export history.

### 2.11 Settings + privacy

- Theme (light/dark/high-contrast/system).
- Editor font, autosave interval, AI on/off per project (off by default with one-time consent).
- Telemetry off by default.
- Crash reports off by default.
- License-free (the MVP itself is licence-free; pricing lands in V1.0).

### 2.12 Tests

(Per `TESTING_STRATEGY.md` and `EXPORT_EPUB_QA.md`.)

- Unit + property at L3.
- Integration (mock + real adapters) at L4.
- Six agent-specific test patterns.
- Privacy invariant tests.
- Snapshot invariants.
- Reproducibility tests.
- ePUB QA (golden files + EPUBCheck + visual regression).
- Performance budgets gated.
- Live local-LLM smoke (nightly, non-gating).

### 2.13 Documentation

- The harness (`CLAUDE.md`, `CLAUDE_CODE_CONTEXT_HARNESS.md`, `DOCS_INVENTORY.md`, etc.).
- All implementation-pack docs.
- An offline in-app help drawer (basic; full help in V1.0).

## 3. Explicitly NOT in MVP

These are tracked but **must not be built** in the first 16 weeks:

- Linux build.
- Plugin runtime and SDK.
- Cloud LLM providers (Anthropic, OpenAI, OpenRouter).
- Embedded llama.cpp.
- Tracked-changes round-trip with Word.
- CSL citation engine + BibTeX/Zotero integration.
- Index generator, advanced cross-references, math rendering in export.
- IngramSpark / Apple Books / Kobo / Google Play export profiles.
- LaTeX export.
- Encryption at rest (SQLCipher).
- Multi-author / multi-machine licensing.
- Real-time collaboration / sync / presence / live cursors.
- Marketplace (browse, install, purchase, ratings).
- Voice dictation, audiobook export.
- Children's / illustrated / cookbook / textbook layouts.
- Translator pack.
- Cover-art image generation.
- Mobile companion app.
- The Book Strategy / Research Organizer / Chapter Planning / Line Editor / Style Guide / Fact-Check / Formatting / ePUB Export QA / Final Review agents (V1.0+).
- Full academic pipeline (footnotes are MVP; the rest is V1.0).

## 4. MVP user persona target

**Novelist Anya** — fiction author writing a 60k–120k-word book. The MVP must serve Anya end-to-end. Theo (non-fiction) and Aisha (academic) are partially served (drafting, editing, basic export work for them too) but their full workflows ship in V1.0.

## 5. MVP timeline

16 weeks across milestones M0–M6. Detail in `IMPLEMENTATION_PLAN.md §1`.

## 6. MVP acceptance criteria (the "ship gate")

Per `PRODUCT_REQUIREMENTS.md §9`, with two additions for the post-Pass-2 changes:

1. The seven journeys (intake → outline → drafting → revising → continuity → copyedit → humanization → export) work end-to-end on macOS and Windows for a fiction project of 60k–120k words.
2. The nine MVP agents run successfully on a clean install with `qwen2.5:7b-instruct-q4_K_M` and produce schema-valid outputs on the fixture suite.
3. A 100k-word manuscript exports to DOCX, PDF (Trade 5×8 and 6×9), and KDP-eBook EPUB-3 in under 60 seconds and passes the bundled KDP validator and EPUBCheck.
4. Privacy invariant test: with the network disabled, every MVP feature except update check, Ollama install, and Ollama model pull works.
5. Snapshot invariant test: every accepted agent edit produced a `pre_agent_edit` snapshot before applying.
6. A `kill -9` during an editing session results in zero data loss after the next launch's recovery prompt.
7. The downloaded EPUB matches the in-app preview under documented tolerance (visual regression).
8. The vocabulary system has populated starter dictionaries for the active template and surfaces at least one Humanization proposal on a fixture passage that contains an AI-tell.
9. CI is green on the macOS-14, macOS-13, and Windows-2022 matrix entries with all guards passing (`cargo deny`, lints, codegen drift, layered-imports, performance budgets, reproducibility, golden-file ePUB).

## 7. Constraints that govern the scope

The MVP is bounded by eight constraints. Honour all eight; trade off only with an ADR.

1. **Time.** 16 weeks. Anything that requires research-level engineering (embedded llama.cpp, plugin sandbox, CRDT) is V1.0+.
2. **Privacy invariants.** Cloud LLM and sync expand the threat model — V1.0+.
3. **Linux velocity tax.** Linux doubles the QA burden; in MVP a non-gating ubuntu smoke job guards drift without blocking. Linux ship is V1.0.
4. **Persona focus.** One persona end-to-end (Anya) takes priority over three personas half-served.
5. **Memory + vocabulary are MVP, not V1.0.** Without them, full-book consistency and human-sounding prose are untestable goals.
6. **EPUB credibility.** The canonical-HTML pipeline is MVP-critical because preview-vs-export drift is unacceptable.
7. **Reversibility.** Every accepted change has a snapshot. Every vocab / memory write has a ledger.
8. **Reproducibility.** Byte-equivalent output across CI runs is verified continuously.
