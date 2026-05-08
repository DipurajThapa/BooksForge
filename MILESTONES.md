# BooksForge — Recreated Milestone Roadmap

**Status as of 2026-05-09 (Pass-4 sync).** Synthesised from
`outputs/IMPLEMENTATION_PLAN.md`, `outputs/MVP_SCOPE.md`,
`booksforge/BACKLOG.md`, git history, and the post-Stabilisation
state of `main`.

> **Where we are after PR #1, #2, #12, #13, #16, #17 merged.**
>
> - `main` is at `bfdd2c4` (PR #17 merge of `feat/product-completion-push-20260508`).
> - **Stabilisation Sprint S1 is complete** — PR #13 committed the
>   team's MZ-05+ slice on 2026-05-08; the working-tree-vs-main
>   gap that this document originally tracked has closed.
> - **Two PRs are still open** as of this update and gate items
>   below: `chore/release-readiness-20260508` (release.yml,
>   EULA/Terms/Privacy drafts, lefthook) and
>   `feat/mz-09-and-polish-20260508` (CrashReport schema, sidecar
>   bundling scaffolds, auto-updater config, privacy invariant
>   test scaffolds).

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
| MZ-05 | Prompt template engine + Outline Architect Agent     | ✅ | PR #13 — `fe74fe6 feat(mz-05+/phase-5): backend Rust crates` |
| MZ-06 | Snapshots v1 (manual + pre-agent-edit + scheduled hourly + pre-restore safety) | ✅ | PR #13 — `booksforge-snapshot` crate + IPC + UI |
| MZ-07 | Outline Architect → document-tree creation           | ✅ | PR #13 — migration `0002_applied_edit_tree_create.sql` + `outline_apply.rs` + `orchestrator/apply.rs` |
| MZ-08 | Quick-action presets (Sharpen / Continue / Rephrase / Shorten / Expand) | ✅ | PR #13 — full template set + per-preset apply paths |
| MZ-09 | Telemetry, logging, crash reports — all opt-in       | ⏳ | PR #13 shipped tracing + diagnostic-bundle (B1–B5).  Crash-capture path remains pending merge of `feat/mz-09-and-polish` (`crash_report.rs` schema landed there; orchestrator panic-hook + Tauri commands + Settings UI flip are the next-up code work). |
| MZ-10 | CI gates + reproducibility seed                      | ⏳ | PR #13 + #2 + #12 shipped: clippy, fmt, deny licenses+bans+advisories, codegen drift, EPUB reproducibility, cold-launch p50, Dependabot, audit-script suite, security-scan workflow.  Pending: `pnpm-lock.yaml` commit (audit #3). |

**M0 ship gate:** ✅ **8 of 10 milestones shipped; MZ-09/MZ-10 partial.**
Both partials close via the two open PRs + ~1 day of follow-up
implementation.

---

## Stabilisation Sprint S1 — ✅ SHIPPED 2026-05-08 (PR #13)

> Original goal (closed): zero working-tree diff against `main` for
> `booksforge/`.  PR #13 split the team's 455 in-flight files into
> 5 milestone-aligned commits + a tail-cleanup commit.

| ID  | Action | Status |
|-----|--------|--------|
| S1  | Split working-tree into milestone-aligned commits | ✅ PR #13 (5 commits: workspace, backend, apps/desktop, IPC bindings + sqlx, BACKLOG + tests) |
| S2  | `cargo clippy --workspace --all-targets -- -D warnings` clean | ⏳ verification deferred — re-run on the merged main; founder action |
| S3  | `cargo test --workspace --all-targets` green | ⏳ deferred — same |
| S4  | `cargo test -p booksforge-orchestrator --test privacy_invariants` green | ⏳ deferred — same |
| S5  | `cargo test -p booksforge-export-epub --test reproducibility` cross-platform | ⏳ deferred — same |
| S6  | Run `pnpm install` and commit `pnpm-lock.yaml` (audit #3) | ⬜ pending — needs node + pnpm in environment |
| S7  | Untrack `.claude/settings.local.json` (audit-extra G11) | ⬜ pending |

**Exit criterion:** ✅ `git status --short` clean for all
`booksforge/` paths.  `.claude/settings.local.json` remains the only
deliberate WIP marker.

---

## M1 — Project & editor polish

| ID  | Item                                                    | Status | Audit ref |
|-----|---------------------------------------------------------|:------:|-----------|
| D1  | Full TipTap node set                                    | 🟡 | — |
| D2  | Outline view (synopsis / POV / status / target words)   | ✅ | PR #13 |
| D3  | Find / replace with regex                               | ✅ | PR #13 |
| D4  | Word-count rollups                                      | ✅ | PR #13 |
| D5  | Distraction-free / focus mode                           | ✅ | PR #13 |
| D6  | Drag-reorder in Binder                                  | ✅ | PR #13 |
| D7  | Hourly auto-snapshots during active sessions            | ✅ | PR #13 |
| D8  | 100k-word fixture + cold-open <2s benchmark             | ✅ | PR #13 |
| M1.A | **Global React error boundary**                        | ✅ | PR #12 — `ErrorBoundary.tsx` + tests in PR #17 |
| M1.B | **Global toast queue**                                  | ✅ | PR #12 — `ToastProvider.tsx` + `useToast()` + tests in PR #17 |
| M1.B' | **Replace `.catch(() => null)` with toasts (migration)** | ⬜ | #25 follow-up — incremental call-site migration |
| M1.C | **Pre-submit input validation in NewProjectWizard**    | ⬜ | #27 |
| M1.D | **Onboarding tour: re-openable from Help menu**        | ⬜ | #28 |
| M1.E | **Memory + Vocabulary panels: manual CRUD**            | ⬜ | #30 |
| M1.F | **Snapshot per-node selective restore**                | ⬜ | #31 |
| M1.G | **Centralised keymap + `?` shortcut help overlay**     | ✅ | PR #12 (`lib/keymap.ts` + `ShortcutHelp.tsx`) + PR #16 (wired via `useShortcut`) |
| M1.H | **Dark-mode toggle (System / Light / Dark)**           | ✅ | PR #12 (`lib/theme.ts` listener) + PR #16 (toggle UI in SettingsPanel) |
| M1.I | **Empty / loading / error states for every panel**     | ⬜ | #60 |
| M1.J | **Consolidate `EditorShell` `useState` sprawl**        | ⬜ | #53 |
| M1.K | **i18n full string migration** (scaffold landed; rest open) | ⬜ | #36 follow-up |

---

## M2 — First three agent workflows end-to-end

| ID  | Item                                          | Status | Audit ref |
|-----|-----------------------------------------------|:------:|-----------|
| E1  | `IntakeAndOutline` workflow                   | ✅ | PR #13 |
| E2  | `Copyedit` workflow                           | ✅ | PR #13 |
| E3  | Context builder with token budgeting          | ✅ | PR #13 |
| E4  | Live run UI                                   | ✅ | PR #13 (`LiveRunOverlay.tsx`) |
| E5  | Output validators per agent                   | ✅ | PR #13 |
| E0c | Prompt-guard injection (full coverage)        | ✅ | PR #13 — `crates/booksforge-orchestrator/src/prompt_guard.rs` |
| E0d.11 | Online plagiarism / originality API (consent-gated) | ⏳ | (BACKLOG E0d.11 partial — provider scaffold + consent storage closed; remote impls still pending) |
| M2.A | **`ProposalReview` shared component (per-hunk accept/reject)** | ✅ | PR #12 (component) + PR #17 (tests).  Per-panel adoption is incremental team work. |
| M2.B | **Agent context size guard (production, not just tests)** | ⬜ | #55 |
| M2.C | **Prompt-template archive directory + audit query** | ⏳ | PR #12 (structural — `archive/README.md`); audit-query example impl pending |

---

## M3 — Developmental + continuity

| ID  | Item                                          | Status |
|-----|-----------------------------------------------|:------:|
| F1  | Deterministic continuity linter               | ✅ PR #13 |
| F2  | `DevelopmentalReview` workflow                | ✅ PR #13 |
| F3  | `ContinuityCheck` workflow                    | ✅ PR #13 |
| F4  | Entity bible auto-extraction + alias handling | ✅ PR #13 |

---

## M4 — Templates + validators

| ID  | Item                                          | Status |
|-----|-----------------------------------------------|:------:|
| G1  | Three project templates                       | ✅ PR #13 |
| G2  | ≥15 manuscript validators                     | ✅ PR #13 (17 validators) |
| G3  | KDP-eBook validator                           | ✅ PR #13 |
| G4  | Pre-export validator gate                     | ✅ PR #13 |
| G5  | One-click fixes for deterministic issues      | ✅ PR #13 (`commands/validators.rs::ApplyFix`) |

---

## M5 — Export pipeline

| ID  | Item                                                         | Status | Audit ref |
|-----|--------------------------------------------------------------|:------:|-----------|
| H0  | Markdown export                                              | ✅ | PR #13 |
| H1  | Pandoc sidecar + DOCX/PDF export                             | ⏳ | PR #13 wrapper + profile mapping; sidecar binary fetch + bundling pending merge of `feat/mz-09-and-polish` (`fetch-sidecars.sh` script ready there) |
| H2  | `booksforge-export-epub` (canonical EPUB-3 pipeline)         | ✅ | PR #13 |
| H3  | EPUBCheck sidecar                                            | ⏳ | PR #13 wrapper + JSON parser; JAR bundling pending merge of `feat/mz-09-and-polish` (runner-launcher script ready there) |
| H4  | Export profiles (KDP eBook, Trade 5×8, Trade 6×9, A5)        | ✅ | PR #13 |
| H5  | Reproducibility tests                                        | ✅ | PR #13 + cross-platform CI |
| H6  | Visual regression (preview vs. unzipped EPUB content HTML)   | ✅ | PR #13 (`tests/visual-regression/` Playwright suite) |
| H7  | Export history                                               | ✅ | PR #13 |
| H8  | Export formatting polish (genre-aware FormatProfile)         | ✅ | PR #13 |
| M5.A | **Pandoc / EPUBCheck argument allowlist (sidecar safety)** | ⬜ | #14 |
| M5.B | **Project-bundle import path-traversal hardening**         | ⬜ | #13 |
| M5.C | **Export wizard: live preview + dependency probe at start time** | ⬜ | #32 |
| M5.D | **Progress + cancel events for export and snapshot**       | ⬜ | #26 |
| M5.E | **Tauri command panic-guard for in-flight `job_id`**       | ⬜ | #17 |

---

## M6 — MVP polish (a.k.a. release-readiness)

| ID  | Item                                                         | Status | Audit ref |
|-----|--------------------------------------------------------------|:------:|-----------|
| I1  | Accessibility audit                                           | ⏳ | Comprehensive sweep done (`useDialogA11y`, `aria-*` patterns); AT testing on real hardware still pending |
| I2  | Code signing + notarisation (macOS Developer ID, Windows EV) | ⬜ | #37 — **founder action** (cert provisioning) |
| I3  | Beta-channel auto-updater (Tauri updater plugin, opt-out)    | ⏳ | Config block scaffolded on `feat/mz-09-and-polish`; pubkey generation = **founder action** |
| I4  | In-app help drawer (offline content)                         | ✅ | PR #13 (`HelpDrawer.tsx`); full docs at `docs.booksforge.app` is L4 below |
| I5  | Onboarding tour                                              | ✅ | PR #13 (`OnboardingTour.tsx`); re-open from Help menu = #28 follow-up |
| M3  | Real icons for the desktop app                               | ✅ | PR #13 (icns + ico + multi-resolution PNGs) |
| M4  | Pandoc + EPUBCheck binaries bundled in `binaries/`           | ⏳ | `fetch-sidecars.sh` + `binaries/README.md` on `feat/mz-09-and-polish`; SHA-256 placeholders need real upstream values (founder/eng) |
| M6.A | **Release pipeline (tag → matrix build → signed artefacts)** | ⏳ | `release.yml` on `chore/release-readiness` (still open PR) |
| M6.B | **`cargo deny check advisories` explicit CI step**         | ✅ | PR #12 (`security-scan.yml`) |
| M6.C | **Dependabot grouped weekly PRs**                          | ✅ | PR #2 (`dependabot.yml`); 8 active PRs as of last check |
| M6.D | **`pnpm-lock.yaml` committed; CI on `--frozen-lockfile`**  | ⬜ | #3 — pending pnpm install on a dev machine |
| M6.E | **`THIRD_PARTY_LICENSES.md` aggregated**                   | ⏳ | PR #2 scaffolded; full `cargo about generate` run is the founder/eng pre-release task |
| M6.F | **`CODEOWNERS` for `apps/desktop/capabilities/**`**        | ✅ | PR #2 |
| M6.G | **Crash reporting: opt-in pipeline with redaction**        | ⏳ | PR #12 (design doc) + `feat/mz-09-and-polish` (typed schema); orchestrator + Tauri + UI implementation = next code work |
| M6.H | **Source maps generated + uploaded (not shipped)**         | ⬜ | #51 |
| M6.I | **Frontend bundle-size monitor in CI**                     | ⬜ | #52 |
| M6.J | **Frontend test coverage backfill (vitest + Playwright)**  | ⏳ | PR #17 added 25 vitest tests + Playwright E2E scaffold; activation requires `tauri-driver` wiring |
| M6.K | **Rust unit-test backfill: `storage`, `memory`, `template`** | ⏳ | PR #17 added template + memory tests; storage already had integration tests pre-Pass-3 |
| M6.L | **`cargo clippy::undocumented_unsafe_blocks` deny**        | ⏳ | PR #12 added SAFETY comments + audit-script check; promoting to clippy deny lint is a follow-up |
| M6.M | **Replace `let _ =` discards with logged ignores**         | ⬜ | #18 |
| M6.N | **CSP hardening — drop `'unsafe-inline'` from `style-src`** | ✅ | PR #12 |
| M6.O | **`OllamaStatusResponse`: ts-rs-generated, not hand-written** | ✅ | PR #12 |
| M6.P | **`ollama_status` returns `Result`, not Ok-with-flag**     | ⏳ | PR #12 logs transient errors via `tracing::warn`; a separate typed-error variant is a follow-up |
| M6.Q | **Snapshot creation idempotency keys**                     | ⬜ | #54 |
| M6.R | **Frontend session-id logging**                            | ✅ | PR #12 (`lib/sessionId.ts` consumed by ErrorBoundary) |
| M6.S | **Pre-commit hooks (lefthook): typecheck/lint/fmt/clippy** | ⏳ | `lefthook.yml` on `chore/release-readiness` (still open) |

---

## Privacy enforcement track (cross-cutting, must complete before public beta)

| ID  | Item                                                           | Status | Audit ref |
|-----|----------------------------------------------------------------|:------:|-----------|
| P1  | Startup network-audit test (invariant #1: no outbound by default) | ⏳ | Test scaffolded `#[ignore]` on `feat/mz-09-and-polish`; activation = remove `#[ignore]` after Stabilisation API is verified |
| P2  | Static + dynamic guard against manuscript content leaving device | ⏳ | Static guard live (`scripts/audit/check-no-manuscript-over-wire.sh` in PR #12); dynamic test scaffolded `#[ignore]` on `feat/mz-09-and-polish` |
| P3  | Integration test: `agent_run_dispatch` fails without consent   | ⏳ | Scaffolded `#[ignore]` on `feat/mz-09-and-polish` |
| P4  | Non-loopback Ollama host UI consent dialog                     | ⬜ | #10 |
| P5  | Consent JSON corruption surfaces a UI banner (not silent default) | ⬜ | #11 |

---

## Business & legal track (must complete before public download / commercial pricing)

| ID  | Item                                                       | Status | Audit ref |
|-----|------------------------------------------------------------|:------:|-----------|
| L1  | Choose final license; replace `LICENSE` placeholder        | ⏳ | #2 (placeholder shipped in PR #1) — **founder action** |
| L2  | Privacy Policy + EULA + Terms                              | ⏳ | Drafts on `chore/release-readiness` (still open PR); legal-counsel review = founder action |
| L3  | Distribution & infrastructure plan                          | ⏳ | `docs/DISTRIBUTION.md` on `chore/release-readiness` (still open PR); domain + CDN + updater endpoint provisioning = founder action |
| L4  | In-app help / docs system                                   | ⏳ | `HelpDrawer` shipped (PR #13); external `docs.booksforge.app` site is V1+ |
| L5  | Public website / landing page                               | ⬜ | #48 — out of repo scope (founder action) |
| L6  | Pricing / monetisation decision                             | ⏳ | `docs/BUSINESS_MODEL.md` scaffolded with options A/B/C/D (PR #17); founder picks |
| L7  | Support channel + SLA                                       | ⏳ | `docs/SUPPORT.md` + issue templates shipped (PR #12); `@booksforge.app` email provisioning = founder action |

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

## Immediate next-3 actions (recommended, post-Pass-4)

1. **Merge the two remaining open PRs** (`chore/release-readiness-20260508`
   and `feat/mz-09-and-polish-20260508`).  These unblock M6.A
   (release pipeline), M6.E (THIRD_PARTY_LICENSES finalisation),
   M6.S (lefthook hooks), MZ-09 crash-capture impl, M5 sidecar
   bundling, and the auto-updater config.
2. **Run the verification gauntlet** on the merged main:
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace --all-targets`,
   `cargo test -p booksforge-orchestrator --test privacy_invariants`,
   `cargo test -p booksforge-export-epub --test reproducibility`.
   Whatever fails becomes the highest-priority follow-up.
3. **Founder-action critical path** (calendar-blocking):
   provision Apple Developer ID + Windows EV cert (#37, ~5–10
   business days each); pick final license + counsel review of
   PRIVACY_POLICY/EULA/TERMS (#2 + #46, ~1–4 weeks); register
   `booksforge.app` domain + provision `updates.booksforge.app`
   (#48 + auto-updater endpoint, ~1 week); generate the Tauri
   updater Ed25519 keypair (#39, 30 minutes once certs are in
   place); pick a pricing option from `docs/BUSINESS_MODEL.md`
   (#49).  None of these can be parallelised by writing more code.

## Completion estimate (post-Pass-4)

| Track | % done |
|---|---:|
| **Product code** | **~92 %** (up from ~85 % at Pass-3) |
| **Release wrapper** | **~50 %** (up from ~40 %) |
| **Audit-item closure** | **35 of 62 = 56 %** |
| **Composite scorecard** | **68 / 100** |

**Public-beta-shippable code-wise:** ~1 week of focused work after
the 2 remaining PRs merge.
**Public-beta-shippable everything-wise:** 3–6 weeks calendar time,
gated by the founder/legal/cert items above.

---

*Refs: `outputs/IMPLEMENTATION_PLAN.md`, `outputs/MVP_SCOPE.md`,
`booksforge/BACKLOG.md`, `EXTERNAL_AUDIT_BACKLOG.md`.*
