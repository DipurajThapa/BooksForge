# Claude Code Prompt Pack — BooksForge

**Version:** 1.0.0-draft  •  **Date:** 2026-05-06

> **Status note (2026-05-06):** Phases 00–05 of this pack are **superseded** by `IMPLEMENTATION_PLAN.md` (milestones M0–M6). The MVP build follows the milestones, not these phase prompts. Phases 06+ (V1.0 and beyond) remain authoritative. The **universal guard-rails (G1–G18)** below are still in force as cross-cutting CI rules. Read `CLAUDE_CODE_CONTEXT_HARNESS.md` first; this pack is reference for V1.0+ work.

This pack contains phase-by-phase prompts for Claude Code, with hard guard-rails, acceptance criteria, and review gates. The goal is to keep agentic development inside the architectural envelope defined in the rest of this documentation set.

## How to use this pack

Run prompts in order. Do not skip a phase. Each prompt is **self-contained** but assumes prior phases are complete and CI is green. Each prompt has the following sections:

- **Goal** — the outcome of the phase.
- **Pre-conditions** — what must be true before starting.
- **Inputs** — files Claude Code must read first (always start there).
- **Deliverables** — what Claude Code must produce.
- **Guard-rails** — non-negotiable constraints. Violations require a stop-and-discuss.
- **Acceptance criteria** — automated checks that must pass before the phase is done.
- **Review gate** — what a human must inspect before merging.
- **Out of scope** — explicit non-goals to keep the agent focused.

## Universal guard-rails (apply to every phase)

These constraints apply to *every* prompt and never need restating in the prompt body. They are the architectural envelope.

**[GUARD-G1] Read the spec first.** Before writing code, read the relevant spec files listed in **Inputs**. If the spec contradicts itself or is silent on an important point, stop and write a question into `docs/open-questions.md` rather than guessing.

**[GUARD-G2] Layering.** No `apps/desktop` or `crates/booksforge-storage` import in `crates/booksforge-domain`. The domain crates depend on nothing project-specific except other domain crates. The lint `cargo deny check bans` is configured to enforce.

**[GUARD-G3] No `unwrap()` in production paths.** `unwrap`, `expect`, and `panic!` are forbidden outside `#[cfg(test)]` and `main()`. Use typed errors. Lint enforces.

**[GUARD-G4] No untyped IPC.** Every Tauri command uses typed input and typed output via `booksforge-ipc`. TypeScript types are regenerated and committed in the same PR. CI fails on TS-Rust drift.

**[GUARD-G5] No network without capability.** No HTTP client may be invoked from any code that is not behind an explicit user-enabled feature flag and capability check. CI grep guards on `reqwest::`/`fetch(`/etc.

**[GUARD-G6] No GPL dependency statically linked.** `cargo deny check licenses` rejects GPL family. Pandoc and epubcheck are sidecars only.

**[GUARD-G7] Performance budgets.** Any change with a >10 % regression on any budget in TAD §18 fails CI. Budgets live in `benches/budgets.toml`.

**[GUARD-G8] Snapshots before destructive operations.** Any code path that mutates the manuscript on the user's behalf must take a snapshot first. This includes: AI-applied edits, plugin-applied edits, importer overwrites, schema migrations.

**[GUARD-G9] Markdown mirror.** Every save to `scene_content` is followed (best-effort) by a Markdown mirror write. Tests assert this for a representative scene.

**[GUARD-G10] Determinism in domain crates.** No clock, no randomness, no I/O in domain code. Inject as parameters where needed. Property tests assert determinism.

**[GUARD-G11] Telemetry off by default.** No `tracing::info!` or analogous call adds anything to a network sink without explicit user opt-in. PII redaction filter is applied at sinks.

**[GUARD-G12] AI capability gate.** Any AI call site checks `project.ai_enabled` and returns an early typed error if disabled.

**[GUARD-G13] Tests with code.** A new function in domain crates ships with at least one unit test in the same PR. New IPC commands ship with at least one integration test. New UI flows ship with at least one E2E test.

**[GUARD-G14] Documentation co-located.** A change to user-visible behaviour updates the in-app help. A change to the IPC surface updates `docs/api/`. A change to the plugin SDK updates `docs/plugin-sdk/`.

**[GUARD-G15] Reproducibility.** Any export pipeline change preserves byte-identical output for the reproducibility test fixtures, or includes a baseline-update commit explaining why.

**[GUARD-G16] No silent data loss.** Any write that may fail surfaces a typed error to UI and never silently drops user content. Disk-full and read-only-fs cases are tested.

**[GUARD-G17] Plugin sandbox.** Any change to plugin host calls passes through capability checks. New host calls require an updated capability list and a capability prompt UX update.

**[GUARD-G18] Cross-platform testing.** Any feature Hosting OS-specific behaviour ships with tests on the OS matrix. Don't merge if it's only been tested on one OS.

## Universal review gate

A phase exits only when:

1. CI is green on the full matrix.
2. All guard-rails are satisfied (lints + grep guards + reproducibility tests).
3. The architecture review checklist for the phase is signed by the tech lead.
4. The risk register is updated with any new risks discovered.
5. The decision log has any new ADRs from this phase.
6. The changelog has user-visible entries.
7. The performance budget report shows no >10% regressions.
8. Documentation is updated (in-app help, public API docs, plugin SDK docs as applicable).

## Phase index

- `phase-00-bootstrap.md` — Repo, tooling, CI, signing certs procurement
- `phase-01-foundations.md` — Project lifecycle, SQLite, IPC, autosave
- `phase-02-editor-core.md` — TipTap, document tree, outline, snapshots-v1
- `phase-03-local-ai.md` — llama.cpp, presets, audit log
- `phase-04-export-pipeline.md` — Pandoc sidecar, profiles, EPUB/PDF/DOCX
- `phase-05-validators-and-templates.md` — Validator engine, KDP, EPUB-3, three templates
- `phase-06-non-fiction-and-academic.md` — Citations, footnotes, tracked changes
- `phase-07-plugin-runtime.md` — WASM host, capability prompt, three first-party plugins
- `phase-08-linux-and-signing.md` — All-OS signing, AppImage/Flatpak
- `phase-09-encryption-and-backups.md` — SQLCipher, Argon2id, snapshot policies
- `phase-10-marketplace-and-cloud-llm.md` — Server-side marketplace; cloud providers
- `phase-11-sync.md` — E2EE cloud sync; conflict resolution UI
- `phase-12-collaboration-v1.md` — Comments + suggestions live; CRDT for comments
- `phase-13-plugin-write-capabilities.md` — Importer/exporter plugins; Zotero
- `phase-14-voice-and-advanced-ai.md` — Whisper.cpp; long-context; voice playback
- `phase-15-translator-pack.md` — Glossary-locked translation flow

## How to invoke a prompt

In Claude Code, paste the contents of a phase file and ensure the working directory is the repository root. The prompt is intentionally directive: "Read X. Implement Y. Verify Z."

## How to recover from a half-finished phase

If Claude Code stops mid-phase, the **next** invocation starts with: "Inspect the repo. List what was completed against this phase's deliverables. Resume from the first incomplete item." Do not paste the full prompt again — it leads to duplicated work.

## Discipline

Resist the urge to take Claude Code "off-roading" mid-phase to fix unrelated bugs. File issues, finish the phase, address them in their own ticket. The guard-rails are tight on purpose — they are why we can let the agent build at all.
