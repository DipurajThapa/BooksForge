# BooksForge — Recreated Milestone Roadmap

**Status as of 2026-05-08.** Synthesised from `outputs/IMPLEMENTATION_PLAN.md`,
`outputs/MVP_SCOPE.md`, `booksforge/BACKLOG.md` (closures + open items),
git history, and the working-tree state.

> **Working-tree vs. main-branch reality.**
>
> - `main` is at `b3469d2` — last *coding* commit `c331011 [MZ-04]`, plus
>   four governance commits from PR #1.
> - The working tree contains a very large body of uncommitted work that
>   `booksforge/BACKLOG.md` records as closed: MZ-05 → MZ-09 partial +
>   Phase 1–6 + Phase 5 (Turns A → S). 455 files differ from `main`.
> - This roadmap therefore distinguishes **shipped** (in `main`),
>   **landed-but-uncommitted** (working tree only), and **open**.
> - Closing the gap between working tree and `main` is itself a roadmap
>   item — see **Stabilisation Sprint** below.

---

## Legend

- ✅ **Shipped** — committed to `main` and verified.
- 🟡 **Landed (uncommitted)** — implemented in the working tree per
  `booksforge/BACKLOG.md`, but not yet on `main`.
- ⏳ **Partial** — substantial work landed, named gaps remain.
- ⬜ **Open** — not started or scoped only.
- 🚫 **Out of MVP** — explicitly deferred per `outputs/MVP_SCOPE.md §3`.

Audit references throughout the document point to
[`EXTERNAL_AUDIT_BACKLOG.md`](EXTERNAL_AUDIT_BACKLOG.md).

---

## Milestone Zero (M0) — Foundation

| ID  | Milestone                                              | Status | Notes |
|-----|--------------------------------------------------------|:------:|-------|
| MZ-01 | Bootstrap workspace                                  | ✅ | `f6e9cb5 feat(mz-01)` |
| MZ-02 | Project bundle creation and opening                  | ✅ | `72dc40c feat(mz-02)` |
| MZ-03 | Single-scene editor (autosave + crash recovery)      | ✅ | `226d4fd feat(mz-03)` |
| MZ-04 | Ollama HTTP client + Setup Wizard                    | ✅ | `c331011 [MZ-04]` |
| MZ-05 | Prompt template engine + Outline Architect Agent     | 🟡 | Working tree — see `BACKLOG.md` E0a/E0/E1. Needs commit. |
| MZ-06 | Snapshots v1 (manual + pre-agent-edit)               | 🟡 | Working tree — A1-A8 closed. Needs commit. |
| MZ-07 | Outline Architect → document-tree creation           | 🟡 | Working tree — `outline_apply.rs`, IPC bindings present. Needs commit. |
| MZ-08 | Quick-action presets (Sharpen / Continue / Rephrase / Shorten / Expand) | 🟡 | Working tree — K1-K5 closed. Needs commit. |
| MZ-09 | Telemetry, logging, crash reports — all opt-in       | 🟡 | Working tree — B1-B5 closed (rotating tracing, PII redaction, diagnostic bundle, settings UI, privacy tests). Crash-report sink itself is still ⬜ (audit #43). |
| MZ-10 | CI gates + reproducibility seed                      | 🟡 | Working tree — C1-C6 closed (cargo deny, layered-imports, codegen drift, clippy gate, EPUB reproducibility, cold-launch p50). Pending: cargo deny check advisories step (audit #40), Dependabot (audit #41), pnpm lockfile (audit #3). |

**M0 ship gate (target: this sprint):** every 🟡 item lands as a properly
attributed commit on `main`. The Stabilisation Sprint below is the
entry point.

---

## Stabilisation Sprint (NEW — must run before M1 work)

> Goal: zero working-tree diff against `main` for `booksforge/`. Prevents
> further drift between BACKLOG.md (closed) and the commit graph.

| ID  | Action | Owner | Audit ref |
|-----|--------|-------|-----------|
| S1  | Split the 455 working-tree files into milestone-aligned commits: MZ-05 → MZ-06 → MZ-07 → MZ-08 → MZ-09 → MZ-10 → Phase-5 turns. One PR per milestone. | Eng | — |
| S2  | Verify `cargo clippy --workspace --all-targets -- -D warnings` clean before each commit. | Eng | — |
| S3  | Verify `cargo test --workspace --all-targets` clean before each commit. | Eng | — |
| S4  | Verify `cargo test -p booksforge-orchestrator --test privacy_invariants` clean. | Eng | — |
| S5  | Run `cargo test -p booksforge-export-epub --test reproducibility` cross-platform. | Eng | — |
| S6  | After landing the in-flight work, run `pnpm install` at workspace root and commit `pnpm-lock.yaml`. | Eng | #3 |
| S7  | After landing, untrack `.claude/settings.local.json` (`git rm --cached`). | Eng | (audit-extra G11) |

**Estimated effort:** 1–2 weeks if commits are properly split; 2 days if a
single squash is acceptable.

**Exit criterion:** `git status --short` clean for all `booksforge/` and
`outputs/` paths; `git diff main..HEAD` for the next branch starts from
zero.

---

## M1 — Project & editor polish

| ID  | Item                                                    | Status | Audit ref |
|-----|---------------------------------------------------------|:------:|-----------|
| D1  | Full TipTap node set                                    | 🟡 | — |
| D2  | Outline view (synopsis / POV / status / target words)   | 🟡 | — |
| D3  | Find / replace with regex                               | 🟡 | — |
| D4  | Word-count rollups                                      | 🟡 | — |
| D5  | Distraction-free / focus mode                           | 🟡 | — |
| D6  | Drag-reorder in Binder                                  | 🟡 | — |
| D7  | Hourly auto-snapshots during active sessions            | 🟡 | — |
| D8  | 100k-word fixture + cold-open <2s benchmark             | 🟡 | — |
| M1.A | **Global React error boundary**                        | ⬜ | #24 |
| M1.B | **Global toast queue + replace `.catch(() => null)`**  | ⬜ | #25 |
| M1.C | **Pre-submit input validation in NewProjectWizard**    | ⬜ | #27 |
| M1.D | **Onboarding tour: re-openable from Help menu**        | ⬜ | #28 |
| M1.E | **Memory + Vocabulary panels: manual CRUD**            | ⬜ | #30 |
| M1.F | **Snapshot per-node selective restore**                | ⬜ | #31 |
| M1.G | **Centralised keymap + `?` shortcut help overlay**     | ⬜ | #33 |
| M1.H | **Dark-mode toggle (System / Light / Dark)**           | ⬜ | #35 |
| M1.I | **Empty / loading / error states for every panel**     | ⬜ | #60 |
| M1.J | **Consolidate `EditorShell` `useState` sprawl**        | ⬜ | #53 |

---

## M2 — First three agent workflows end-to-end

| ID  | Item                                          | Status | Audit ref |
|-----|-----------------------------------------------|:------:|-----------|
| E1  | `IntakeAndOutline` workflow                   | 🟡 | — |
| E2  | `Copyedit` workflow                           | 🟡 | — |
| E3  | Context builder with token budgeting          | 🟡 | — |
| E4  | Live run UI                                   | 🟡 | — |
| E5  | Output validators per agent                   | 🟡 | — |
| E0c | Prompt-guard injection (full coverage)        | ⏳ | (BACKLOG E0c partial) |
| E0d.11 | Online plagiarism / originality API (consent-gated) | ⏳ | (BACKLOG E0d.11 partial) |
| M2.A | **`ProposalReview` shared component (per-hunk accept/reject)** | ⬜ | #29 |
| M2.B | **Agent context size guard (production, not just tests)** | ⬜ | #55 |
| M2.C | **Prompt-template archive directory + audit query** | ⬜ | #56 |

---

## M3 — Developmental + continuity

| ID  | Item                                          | Status |
|-----|-----------------------------------------------|:------:|
| F1  | Deterministic continuity linter               | 🟡 |
| F2  | `DevelopmentalReview` workflow                | 🟡 |
| F3  | `ContinuityCheck` workflow                    | 🟡 |
| F4  | Entity bible auto-extraction + alias handling | 🟡 |

---

## M4 — Templates + validators

| ID  | Item                                          | Status |
|-----|-----------------------------------------------|:------:|
| G1  | Three project templates                       | 🟡 |
| G2  | ≥15 manuscript validators                     | 🟡 |
| G3  | KDP-eBook validator                           | 🟡 |
| G4  | Pre-export validator gate                     | 🟡 |
| G5  | One-click fixes for deterministic issues      | 🟡 |

---

## M5 — Export pipeline

| ID  | Item                                                         | Status | Audit ref |
|-----|--------------------------------------------------------------|:------:|-----------|
| H0  | Markdown export                                              | 🟡 | — |
| H1  | Pandoc sidecar + DOCX/PDF export                             | ⏳ | (binary bundling pending §M4) |
| H2  | `booksforge-export-epub` (canonical EPUB-3 pipeline)         | 🟡 | — |
| H3  | EPUBCheck sidecar                                            | ⏳ | (JAR bundling pending §M4) |
| H4  | Export profiles (KDP eBook, Trade 5×8, Trade 6×9, A5)        | 🟡 | — |
| H5  | Reproducibility tests                                        | 🟡 | — |
| H6  | Visual regression (preview vs. unzipped EPUB content HTML)   | 🟡 | — |
| H7  | Export history                                               | 🟡 | — |
| H8  | Export formatting polish (genre-aware FormatProfile)         | 🟡 | — |
| M5.A | **Pandoc / EPUBCheck argument allowlist (sidecar safety)** | ⬜ | #14 |
| M5.B | **Project-bundle import path-traversal hardening**         | ⬜ | #13 |
| M5.C | **Export wizard: live preview + dependency probe at start time** | ⬜ | #32 |
| M5.D | **Progress + cancel events for export and snapshot**       | ⬜ | #26 |
| M5.E | **Tauri command panic-guard for in-flight `job_id`**       | ⬜ | #17 |

---

## M6 — MVP polish (a.k.a. release-readiness)

| ID  | Item                                                         | Status | Audit ref |
|-----|--------------------------------------------------------------|:------:|-----------|
| I1  | Accessibility audit                                           | ⏳ | — |
| I2  | Code signing + notarisation (macOS Developer ID, Windows EV) | ⬜ | #37 |
| I3  | Beta-channel auto-updater (Tauri updater plugin, opt-out)    | ⬜ | #39 |
| I4  | In-app help drawer (offline content)                         | 🟡 | — |
| I5  | Onboarding tour                                              | 🟡 | — |
| M3  | Real icons for the desktop app                               | 🟡 | (working-tree — verify) |
| M4  | Pandoc + EPUBCheck binaries bundled in `binaries/`           | ⬜ | (BACKLOG M4) |
| M6.A | **Release pipeline (tag → matrix build → signed artefacts)** | ⬜ | #38 |
| M6.B | **`cargo deny check advisories` explicit CI step**         | ⬜ | #40 |
| M6.C | **Dependabot grouped weekly PRs**                          | ⬜ | #41 |
| M6.D | **`pnpm-lock.yaml` committed; CI on `--frozen-lockfile`**  | ⬜ | #3 |
| M6.E | **`THIRD_PARTY_LICENSES.md` aggregated**                   | ⬜ | #4 |
| M6.F | **`CODEOWNERS` for `apps/desktop/capabilities/**`**        | ⬜ | #16 |
| M6.G | **Crash reporting: opt-in pipeline with redaction**        | ⬜ | #43 |
| M6.H | **Source maps generated + uploaded (not shipped)**         | ⬜ | #51 |
| M6.I | **Frontend bundle-size monitor in CI**                     | ⬜ | #52 |
| M6.J | **Frontend test coverage backfill (vitest + Playwright)**  | ⬜ | #22 |
| M6.K | **Rust unit-test backfill: `storage`, `memory`, `template`** | ⬜ | #21 |
| M6.L | **`cargo clippy::undocumented_unsafe_blocks` deny**        | ⬜ | #12 |
| M6.M | **Replace `let _ =` discards with logged ignores**         | ⬜ | #18 |
| M6.N | **CSP hardening — drop `'unsafe-inline'` from `style-src`** | ⬜ | #15 |
| M6.O | **`OllamaStatusResponse`: ts-rs-generated, not hand-written** | ⬜ | #19 |
| M6.P | **`ollama_status` returns `Result`, not Ok-with-flag**     | ⬜ | #20 |
| M6.Q | **Snapshot creation idempotency keys**                     | ⬜ | #54 |
| M6.R | **Frontend session-id logging**                            | ⬜ | #57 |
| M6.S | **Pre-commit hooks (lefthook): typecheck/lint/fmt/clippy** | ⬜ | #59 |

---

## Privacy enforcement track (cross-cutting, must complete before public beta)

| ID  | Item                                                           | Status | Audit ref |
|-----|----------------------------------------------------------------|:------:|-----------|
| P1  | Startup network-audit test (invariant #1: no outbound by default) | ⬜ | #7 |
| P2  | Static + dynamic guard against manuscript content leaving device | ⬜ | #8 |
| P3  | Integration test: `agent_run_dispatch` fails without consent   | ⬜ | #9 |
| P4  | Non-loopback Ollama host UI consent dialog                     | ⬜ | #10 |
| P5  | Consent JSON corruption surfaces a UI banner (not silent default) | ⬜ | #11 |

---

## Business & legal track (must complete before public download / commercial pricing)

| ID  | Item                                                       | Status | Audit ref |
|-----|------------------------------------------------------------|:------:|-----------|
| L1  | Choose final license; replace `LICENSE` placeholder        | ⏳ | #2 (placeholder shipped in PR #1) |
| L2  | Privacy Policy + EULA + Terms                              | ⬜ | #46 |
| L3  | Distribution & infrastructure plan                          | ⬜ | #45 |
| L4  | In-app help / docs system                                   | ⬜ | #47 |
| L5  | Public website / landing page                               | ⬜ | #48 |
| L6  | Pricing / monetisation decision                             | ⬜ | #49 |
| L7  | Support channel + SLA                                       | ⬜ | #50 |

---

## Out of MVP (per `outputs/MVP_SCOPE.md §3`)

🚫 Linux build · 🚫 Plugin runtime / SDK · 🚫 Cloud LLM providers ·
🚫 llama.cpp embed · 🚫 Tracked-changes Word round-trip · 🚫 CSL / BibTeX ·
🚫 Index / advanced cross-refs · 🚫 IngramSpark / Apple / Kobo / Google Play ·
🚫 LaTeX export · 🚫 SQLCipher · 🚫 Multi-author licensing ·
🚫 Real-time collab · 🚫 Marketplace · 🚫 Voice / audiobook ·
🚫 Children's / illustrated / cookbook layouts · 🚫 Translator pack ·
🚫 Cover-art image generation · 🚫 Mobile companion ·
🚫 V1+ agents (Strategy / Research Organiser / Chapter Planning / Line Editor / Style Guide / Fact-Check / Formatting / EPUB QA / Final Review).

---

## Immediate next-3 actions (recommended)

1. **Stabilisation Sprint S1** — split the 455 working-tree files into
   commit-aligned milestones. Until this is done, every other change
   risks getting tangled.
2. **Privacy track P1–P3** — the headline product promise has no CI
   enforcement today; this is the single highest-risk technical gap.
3. **M6.B + M6.C + M6.D + M6.F** — small, mechanical CI / governance
   wins (advisories step, Dependabot, lockfile, CODEOWNERS) that
   significantly raise release-readiness with low risk. *This branch
   starts that work.*

---

*Refs: `outputs/IMPLEMENTATION_PLAN.md`, `outputs/MVP_SCOPE.md`,
`booksforge/BACKLOG.md`, `EXTERNAL_AUDIT_BACKLOG.md`.*
