# Roadmap & Phasing — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

> **Status note (2026-05-06):** The MVP slice of this document (phases 0–5, weeks 0–18) is **superseded** by `../IMPLEMENTATION_PLAN.md`. The implementation pack tightens the MVP window to ~16 weeks by deferring embedded llama.cpp, the plugin runtime, and Linux to V1.0 — and by reorganising the phases around milestones M0–M6 instead of phases 0–5. **Phases 6+ (V1.0 and beyond) below remain authoritative** and are unchanged. Where this document and `../IMPLEMENTATION_PLAN.md` differ for the MVP slice, the implementation pack wins.

This document is the build plan: phases, what enters each, exit criteria, dependencies, and the Claude Code prompt mapping. The detailed prompts are in `prompts/`.

---

## Phase map at a glance

| Phase | Window | Theme | Headline outcome |
|-------|--------|-------|------------------|
| 0 — Bootstrap | weeks 0–2 | Repo, tooling, CI | Tauri + React + Cargo workspace boots; CI green on three OSes |
| 1 — Foundations | weeks 2–6 | Project model, storage, IPC | A new project can be created, opened, edited, saved, reopened |
| 2 — Editor core | weeks 4–10 | TipTap, doc tree, outline | A 100k-word project can be edited fluidly with auto-save |
| 3 — Local AI | weeks 8–14 | llama.cpp, presets, audit | "Sharpen prose" works offline on the reference Mac |
| 4 — Export pipeline | weeks 10–16 | Pandoc embed, profiles | DOCX, PDF, EPUB-3 export with KDP profile |
| 5 — Validators & templates | weeks 14–18 | Built-in templates and rules | KDP-eBook validator passes a 50k-word fiction project |
| **MVP release** | week 18 |  | Windows + macOS public beta, fiction mode |
| 6 — Non-fiction & academic | weeks 18–24 | Citations, tracked changes, footnotes | Academic monograph round-trip DOCX with footnotes & citations |
| 7 — Plugin runtime | weeks 22–28 | WASM compute + UI panels (read-only caps) | Sideload a validator plugin, capability prompt works |
| 8 — Linux build & code-sign | weeks 26–30 | All-OS signing | Signed installers on Win/mac/Linux, auto-update verified |
| 9 — Encryption & advanced backup | weeks 28–32 | Per-project encryption, snapshots | Encrypted project survives passphrase round-trip |
| **V1.0 release** | month 8 |  | All-OS GA |
| 10 — Marketplace & cloud LLM | months 9–11 | Marketplace, BYO key, Studio credits | First paid plugin sold; Studio credits billed |
| 11 — Sync (encrypted) | months 11–13 | E2EE cloud sync | Two-device sync of one project, conflict resolution |
| 12 — Collaboration v1 (CRDT) | months 12–14 | Comments + suggestions live | Two users co-comment in real time |
| **V1.5 release** | month 14 |  |  |
| 13 — Plugin write capabilities | months 15–16 | Importer/exporter plugins | Zotero importer ships |
| 14 — Voice + advanced AI | months 15–17 | Dictation, voice playback, long-context | Dictate a chapter |
| 15 — Translator pack | months 16–18 | Terminology preservation | Translate a chapter with glossary lock |
| **V2.0 release** | month 18 |  |  |

## Phase 0 — Bootstrap (weeks 0–2)

**Inputs.** None. Greenfield.
**Outputs.** Repo, CI, dev docs, signing certificates procured.

The repo is initialised as a Cargo workspace with a Tauri app and a React frontend. CI runs on Windows / macOS / Linux for lint, typecheck, test, and a smoke build. We procure: Microsoft EV cert, Apple Developer ID, Linux signing keys. Pre-commit hooks and CODEOWNERS land. Initial design tokens and a one-screen "Hello, manuscript" UI prove the IPC works end-to-end.

**Exit criteria.** A PR opens a tracked TODO bug, hits CI, lands signed nightly build to `nightly.booksforge.app`. Tech lead signs the architecture review.

**Claude Code prompt:** `prompts/phase-00-bootstrap.md`.

## Phase 1 — Foundations (weeks 2–6)

**Inputs.** Phase 0 repo. TAD (03), Data Model (04).
**Outputs.** Project lifecycle works.

`booksforge-domain` types stabilise. `booksforge-storage` with SQLite migration v1 lands. `booksforge-fs` with atomic bundle creation. Tauri commands for `project.create`, `project.open`, `project.close`, `project.recent`. UI: project picker, recent list, "New project" wizard with built-in templates loaded from a stub. Autosave + crash recovery. Lock file with conflict UI.

**Exit criteria.** A new project survives kill -9 mid-edit and recovers. A project moves between Win/mac/Linux machines. Schema migration round-trip tested. Performance probe: open a 200k-word fixture in ≤2 s on reference hardware.

## Phase 2 — Editor core (weeks 4–10)

**Inputs.** Phase 1. FSD §2.
**Outputs.** A boring, fast editor.

TipTap integration with our schema (Part/Chapter/Scene/Block). Document tree (binder), corkboard, outline view, find/replace, footnotes, basic citations (placeholder until Phase 6), word count, focus modes, tracked-spell-check, snapshot v1 (manual).

**Exit criteria.** Performance budgets in TAD §18 met. ProseMirror round-trip tests at ≥ 95 % coverage of supported nodes. Accessibility audit pass on editor surface.

## Phase 3 — Local AI (weeks 8–14)

**Inputs.** Phase 2. AI Integration spec (08).
**Outputs.** Local AI on default presets.

llama.cpp Rust binding integrated as `LlmProvider::Embedded`. Curated catalogue v1 (3 models). Auto-download with hash verification. Prompt templates engine. `Sharpen / Shorten / Expand / Continue` presets. AI sidebar with diff view. Audit log table populated. Cancel works mid-stream. Pre-AI snapshot.

**Exit criteria.** Reference-hardware benchmark passes (FR-AI-001 §10). Privacy invariant tested: with network mocked-fail, AI still works. Audit row written for every call.

## Phase 4 — Export pipeline (weeks 10–16)

**Inputs.** Phase 1–3. Export Pipeline spec (09).
**Outputs.** DOCX/PDF/EPUB out the door.

Pandoc sidecar integration. BooksForge-AST → Pandoc-AST transformer. Profiles: Manuscript-DOCX, Generic-PDF, Generic-EPUB-3, KDP-eBook EPUB-3. Asset pipeline. Font subsetting (subset of fonts). epubcheck integration (with bundled small JRE). Reproducibility test in CI.

**Exit criteria.** A 100k-word fixture exports to all three formats; reproducibility hash test green; epubcheck warnings/errors handled per spec; KDP-eBook validator passes the export.

## Phase 5 — Validators & templates (weeks 14–18)

**Inputs.** Phase 4. FSD §5, §3.
**Outputs.** Pre-export gate works; first three templates ship.

Validator engine API. ≥20 manuscript validators (heading hierarchy, alt text, broken refs, etc.). KDP and EPUB-3 store validators. Templates: Generic Novel, Romance, Sci-Fi/Fantasy. Pre-export gate UI.

**Exit criteria.** A new "Romance" project, written end-to-end, passes pre-export gate and produces a valid KDP EPUB-3 in ≤ 30 s.

## MVP release (week 18)

Public beta on Windows and macOS, fiction mode only. Signed installers, auto-update on a `beta` channel. Onboarding tour. In-app help (offline).

**Definition of done:** all MVP-tagged FRs in FSD pass acceptance; security checklist (06 §13) signed; documentation complete; first 100 beta testers onboarded; crash-free session rate ≥ 99 % during beta week.

## Phase 6 — Non-fiction & academic (weeks 18–24)

Citation engine with CSL styles + BibTeX import. Native footnote/endnote rendering. Tracked changes with author attribution and round-trip DOCX. Tables. Math (KaTeX in editor; LaTeX in export). Cross-references that survive renumbering. Index generation.

**Exit criteria.** A 50k-word academic fixture round-trips DOCX with footnotes and citations losslessly (compared via `pandoc-diff`).

## Phase 7 — Plugin runtime (weeks 22–28)

WASM compute plugin host (wasmtime). Capability model. UI plugin WebView. SDK in Rust + TS. Plugin CLI. Sideload UX. Three first-party plugins ship (Romance Trope Validator, Save the Cat panel, Mystery Pack).

**Exit criteria.** Sideload a validator plugin; capabilities prompted; plugin runs sandboxed; resource caps enforced. Adversarial fixture (allocator bomb) is killed.

## Phase 8 — Linux + signing (weeks 26–30)

Linux Tauri build (Ubuntu 22.04 baseline). AppImage and Flatpak. Code signing on all three OSes; auto-update verifies signatures.

## Phase 9 — Encryption & advanced backups (weeks 28–32)

SQLCipher integration; Argon2id KDF; per-blob asset encryption; passphrase UX; OS-keyring optional; selective restore from snapshots; pre-AI snapshot on by default.

## V1.0 release (month 8)

All-OS GA. All V1.0-tagged FRs pass. External pen-test passed. Marketing site live. Pricing live. Pro license activation works offline.

## Phases 10–12 — V1.5 (months 9–14)

**Marketplace.** Server-side, web-side, signing key infrastructure. Stripe Connect for payouts.

**Cloud LLM.** Anthropic, OpenAI, OpenRouter providers. BYO key. Studio credits + billing. Cost estimates.

**Sync (E2EE).** Encrypted project ciphertext upload to a cloud bucket. CRDT-based conflict resolution starting with comments and suggestions; manuscript text remains last-writer-wins for now (full content CRDT is V2.0+).

**Collaboration v1.** Live comments, live suggestions; identity via BooksForge account; presence indicators.

**V1.5 exit criteria:** marketplace has ≥ 10 plugins; first paying customer transacts; two-device sync of a real project survives a week of dogfooding; live comments stable.

## Phases 13–15 — V2.0 (months 15–18)

**Plugin write capabilities.** Importer/exporter plugin types. Zotero importer ships as flagship example.

**Voice & advanced AI.** Dictation (Whisper.cpp). TTS playback for self-edit. Long-context features for whole-novel critique using long-context cloud models.

**Translator pack.** Glossary-locked translation that preserves entity names and key terminology.

**V2.0 exit criteria:** ≥ 40 marketplace plugins; ARR ≥ $1.2M; net churn ≤ 3 % monthly; KDP first-upload rejection rate ≤ 5 % among validator users.

## Cross-phase commitments

Every phase exits the architecture review gate (TAD §24). Every phase produces or updates: changelog, in-app help, regression test additions, performance benchmark deltas, accessibility audit notes, security checklist updates, documentation. **No phase exits with a regressed performance budget.** Risk register reviewed at the end of each phase.

## Dependencies between phases

Phase 2 depends on Phase 1 SQLite layer being stable (otherwise editor commits churn under the editor). Phase 4 depends on Phase 2 because the export reads from the canonical document. Phase 5 depends on Phase 4 (pre-export gate is meaningless without exports). Phase 7 depends on Phase 1's IPC and Phase 4's content reads. Phase 11 (sync) depends on Phase 9 (encryption). Phase 12 (collaboration) depends on Phase 11 (sync transport). Everything else is parallelisable for a team of three or more.

## Contingencies

If Tauri v2 stable slips: stay on v2 RC and pin; v2 stable replaces RC at the next minor with a guarded migration. If llama.cpp Rust bindings regress: fall back to the FFI route (cc + bindgen). If Pandoc can't produce a profile we need: write a small format-specific writer in Rust that bypasses Pandoc for that profile only — always avoid bypassing the pipeline overall. If epubcheck JRE size becomes a release blocker: ship a smaller JRE distribution (e.g., `jlink`-stripped to 30 MB) before considering a port.
