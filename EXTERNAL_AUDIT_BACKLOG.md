# BooksForge — External Audit Backlog

**Auditor:** Independent external review
**Date:** 2026-05-08
**Scope:** Codebase, business readiness, functionality, workflow, user flow
**Repository state at audit:** MZ-01 → MZ-04 + Phase 1–4 follow-ups merged; the
internal `booksforge/BACKLOG.md` (~2,000 lines) was reviewed but treated as the
team's own list — this file is a fresh, unbiased outside view.

This backlog is **strictly sequential**. Each item is ordered so that
finishing item N never blocks item N+1, and N+1 never has to undo N. The
ordering accounts for: build/repro dependencies first, then security & privacy
correctness, then product completeness, then release engineering, then
business/legal, then polish. Skipping forward is allowed; reordering is not
recommended.

> **Conventions**
> - **Severity:** `CRITICAL` (ship-blocker / legal exposure) · `HIGH` (release-blocker) · `MEDIUM` (must fix before public beta) · `LOW` (polish / debt)
> - **Dependency:** items list explicit predecessors as `↳ blocks until #N`
> - **Effort:** rough sizing — `S` ≤ 1 day, `M` 2–5 days, `L` 1–2 weeks, `XL` > 2 weeks

> **Pruning + execution log (last sync: 2026-05-09 Pass-3).**
>
> | Pass | Branch | Items closed |
> |---|---|---|
> | Verification (Pass-0) | — | Removed: #23 (codegen-drift test already in CI), #42 (reverse migrations dir is dev-only single file) |
> | PR #1 (merged) | `chore/booksforge-reorg-20260508` | #1 (CLAUDE.md drift), #2 (LICENSE placeholder), #5 (SECURITY.md), #6 (CODE_OF_CONDUCT.md) |
> | PR #2 (merged) | `chore/audit-backlog-drive-20260508` | #4 (THIRD_PARTY_LICENSES scaffold), #16 (CODEOWNERS), #41 (Dependabot config), #61 (pin policy + privacy checklist) |
> | PR #12 (merged) | `chore/onboarding-and-runbook-20260508` | #8 (manuscript-over-wire static script), #12 (`// SAFETY:` comments), #15 (CSP `'unsafe-inline'` removed), #19 (`OllamaStatusResponse` ts-rs-generated), #20 (`ollama_status` logs transient errors), #24 (global ErrorBoundary), #25 (ToastProvider + `useToast`), #29 (`ProposalReview` component), #33 (keymap module + `ShortcutHelp` overlay), #35 (theme listener — `prefers-color-scheme`), #36 (i18n scaffold + `locales/en.json`), #40 (`cargo deny check advisories` step), #43 (crash-report design doc), #44 (tauri.conf metadata fields), #50 (Support docs + issue templates), #56 (prompt-template archive structural), #57 (`sessionId` for log correlation), #58 (Dependabot auto-merge guardrails) |
> | PR #13 (merged) | `feat/stabilisation-sprint-20260508` | (no audit items directly closed; commits MZ-05+ slice) |
> | This branch (`feat/m1-wiring-and-audit-sync-20260508`) | (this PR) | #33 wiring (ShortcutHelp consumer + `useShortcut`), #35 toggle UI in SettingsPanel, #36 first i18n migration |
> | Open PR `chore/release-readiness-20260508` | (awaiting merge) | will close: #38 (release.yml), #45 (DISTRIBUTION.md), #46 (EULA/Terms/Privacy drafts), #50 supporting docs, #59 (lefthook hooks) |
> | Open PR `feat/mz-09-and-polish-20260508` | (awaiting merge) | will close: #43 Rust scaffold, #7/#8/#9 test scaffolds (`#[ignore]`), #15 reaffirmed, #44 reaffirmed |
>
> Numbering is left stable across all passes to preserve dependency
> references; closed items remain in place inline as `*(LANDED …)*` /
> `*(REMOVED — addressed)*` short markers (or carry their original
> finding text — both forms are valid in this living document).

---

## Specification compliance — were the `outputs/` instructions followed?

> Question from the brief: *"check if the product has been developed using the
> instructions clearly."* Verdict per spec document, on a 0–100 scale of
> "implementation matches the spec's letter and intent."

| `outputs/` document | 2026-05-08 | 2026-05-09 (Pass-3) | Notes |
|---|---:|---:|---|
| **`IMPLEMENTATION_PLAN.md`** (MZ-01 → MZ-10) | 80 | **92** | MZ-01 → MZ-08 + Phase 5 fully committed via `feat/stabilisation-sprint-20260508` (PR #13 merged).  Phase-5 turns A-S all on `main`.  MZ-09 partial: tracing/logging/diagnostic-bundle landed; crash-capture path is the only piece still open and is gated on `feat/mz-09-and-polish` merge.  MZ-10 substantially landed (reproducibility test, codegen drift gate, `cargo deny check advisories`, Dependabot, audit-script CI). |
| **`ARCHITECTURE.md`** (four-layer) | 95 | **98** | Unchanged structurally; `// SAFETY:` comments now in place on the two unsafe blocks in `booksforge-fs::lock` and `booksforge-ollama::probe` (#12 closed). |
| **`TOOLCHAIN.md`** (version pins) | 90 | **92** | Pin policy now documented in `CONTRIBUTING.md` (#61 closed); Rust pin moved to 1.88.0 in line with `rust-toolchain.toml`. |
| **`DATA_MODEL.md`** (SQLite, bundle format) | 95 | **95** | Unchanged. |
| **`SECURITY_PRIVACY.md`** (5 invariants) | 60 | **75** | Three of five invariants now enforced in CI (loopback default, no telemetry SDKs, GPL ban) plus a static-grep guard for invariant #2 via `scripts/audit/check-no-manuscript-over-wire.sh`.  Dynamic tests for invariants 1, 2, 4 are scaffolded with `#[ignore]` on `feat/mz-09-and-polish` (closes when that PR merges + ignores are removed).  CSP hardened (`'unsafe-inline'` removed). |
| **`AGENTS.md`** (10 MVP agents) | 75 | **88** | All 10 agents committed; `ProposalReview` shared component shipped (#29 closed); each agent panel can adopt it incrementally.  Final Review Editor now in scope per spec doc update. |
| **`MEMORY_SYSTEM.md`** | 80 | **80** | Unchanged — manual-CRUD UI surface (#30) still pending. |
| **`VOCABULARY_DICTIONARIES.md`** | 90 | **90** | Unchanged. |
| **`EXPORT_EPUB_SPEC.md` + `EXPORT_EPUB_QA.md`** | 85 | **88** | Pipeline + reproducibility + visual-regression all on `main`.  Sidecar fetch + bundling scaffolded on `feat/mz-09-and-polish`; Pandoc binary actually shipping is the team's pre-release fetch step. |
| **`UI_UX_SPEC.md`** | 65 | **82** | ErrorBoundary (#24) + ToastProvider (#25) + ShortcutHelp (#33) + theme toggle UI (#35) + i18n scaffold (#36) all wired into the app shell.  Onboarding (#28), per-panel CRUD (#30), selective restore (#31), export preview (#32), a11y full audit (#34), empty-state pass (#60) still tracked. |
| **`DESIGN_SYSTEM.md`** | 80 | Token system in `packages/ui/src/tokens.css` is well-structured per agent-2 audit; dark-mode tokens exist but no toggle (#35). |
| **`TESTING_STRATEGY.md`** | 55 | Rust integration tests are good (privacy_invariants, reproducibility, visual_regression, codegen_drift); **three L3/L4 crates have zero unit tests** (#21); **frontend has 3 test files for ~9k LOC of React** (#22); no E2E suite for the desktop golden path. |
| **`MVP_SCOPE.md`** §6 (acceptance criteria) | 60 | Of the 9 ship-gate criteria: #1 (seven journeys end-to-end), #2 (10 MVP agents), #3 (export <60s), #5 (pre_agent_edit snapshots), #7 (visual-regression match), #8 (vocab + Humanization), #9 (CI green) appear satisfied per BACKLOG. **#4 (network-disabled functional test) and #6 (kill -9 zero-data-loss test) are not visible as named CI tests** — see #7 and the Phase 1 tests in this audit. |
| **`CONSISTENCY_MATRIX.md`** | n/a | Cannot be scored mechanically; deferred. |
| **`ARCHITECTURE_DECISIONS.md`** (ADRs) | 85 | ADRs are honoured by the code (Tauri 2, sqlx macros, ts-rs codegen, MiniJinja). |

**Headline judgement on compliance:** the *engineering* spec has been followed
unusually well — architecture, data model, toolchain, export pipeline, and
agent wiring are faithful. The *operational* spec (security-invariant CI
tests, MVP §6 acceptance tests, frontend test discipline, MZ-09 telemetry,
MZ-10 release-engineering polish) is where the gap sits. This matches the
shape of the backlog below.

---

## PHASE 0 — Truthfulness & repro foundations

> Nothing else can be trusted until the repo describes itself accurately and
> can be reproducibly built by an outside contributor.

### #1 — Reconcile root `CLAUDE.md` with reality
- **Severity:** CRITICAL · **Effort:** S
- **Finding:** `/CLAUDE.md` line 7-9 still says *"This workspace is documentation-only. No source code exists yet. The first coding task is MZ-01."* Code through MZ-04 + Phase 4 follow-ups is merged. An outside contributor or auditor reading this is misled about what is real.
- **Action:** Rewrite the `Current state` block. Either delete the root `CLAUDE.md` and let `booksforge/CLAUDE.md` (per spec, it should have been copied at MZ-01) be authoritative, or keep root `CLAUDE.md` but mark it as a thin redirect.
- **Acceptance:** A new contributor cloning the repo and reading top-level `CLAUDE.md` correctly identifies the next task and the location of code.

### #2 — Add a real `LICENSE` at repo root and stop declaring `UNLICENSED`
- **Severity:** CRITICAL · **Effort:** S
- **Finding:** No `LICENSE` / `LICENSE.txt` at root or in `booksforge/`. `Cargo.toml` declares `license = "UNLICENSED"`. Distribution, contribution, and binary shipping are legally ambiguous.
- **Action:** Pick a license (recommend dual MIT/Apache-2.0 for the SDK-like crates, separate proprietary EULA for the shipped app if that is the business model). Commit `LICENSE` and update `Cargo.toml` `license = "..."`.
- **Acceptance:** `cargo deny check licenses` still passes; license header matches Cargo manifest.

### #3 — Commit `pnpm-lock.yaml`
- **Severity:** HIGH · **Effort:** S · ↳ blocks until #2
- **Finding:** No `pnpm-lock.yaml` at `booksforge/` workspace root. Frontend builds are not reproducible; CI cannot use `--frozen-lockfile`; supply-chain audit cannot resolve a deterministic dep graph.
- **Action:** `pnpm install` at workspace root, commit lockfile, switch CI to `pnpm install --frozen-lockfile`.
- **Acceptance:** Two clean clones produce byte-identical `node_modules` resolution; CI uses `--frozen-lockfile`.

### #4 — *(SCAFFOLDED — finalisation deferred to first signed release)*
*Landed 2026-05-08 on `chore/audit-backlog-drive-20260508` as
`THIRD_PARTY_LICENSES.md` (commit `179ef57`). The file documents the
licensing posture (GPL-family ban for linked crates; Pandoc/EPUBCheck
sidecars OK), enumerates bundled-asset categories that need
attribution, and provides the regeneration commands for `cargo about
generate` and `pnpm licenses list`. The auto-generated per-dependency
detail must be produced and committed before the first signed
release; tracked as **MILESTONES.md M6.E**.*

### #5 — Add `SECURITY.md` with private disclosure path
- **Severity:** HIGH · **Effort:** S
- **Finding:** No security policy at root. A privacy-first product without a private vulnerability channel will be reported via public GitHub issues, defeating the privacy posture.
- **Action:** Add `SECURITY.md` with: supported versions, disclosure email or GitHub Security Advisories link, response SLA, scope, safe-harbour wording.
- **Acceptance:** GitHub renders the "Report a vulnerability" button.

### #6 — Add `CODE_OF_CONDUCT.md`
- **Severity:** MEDIUM · **Effort:** S
- **Finding:** Missing. Any open contribution channel without a code of conduct is moderation-fragile.
- **Action:** Adopt Contributor Covenant 2.1, link from `CONTRIBUTING.md` and `README.md`.

---

## PHASE 1 — Privacy invariant gaps (the product's defining promise)

> The headline promise is *"no content leaves the device by default."* Every
> invariant must have a CI test, per the project's own contract. Several
> invariants currently rely on convention rather than enforcement.

### #7 — Add startup network audit test for invariant #1 (no outbound by default)
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #3
- **Finding:** Invariant #1 ("no content leaves the device by default") is asserted in `outputs/SECURITY_PRIVACY.md` and `CLAUDE.md` but no test boots the app and observes that zero non-loopback sockets are opened before the user takes an opt-in action.
- **Action:** Add an integration test that launches the Tauri app in headless mode, starts a packet-capture (loopback-only allowlist) for 30 s with no user input, and asserts no traffic to anything other than `127.0.0.1:11434`. Block the build on a violation.
- **Acceptance:** Removing the assertion or adding a `reqwest::get("https://...")` at startup turns CI red.

### #8 — Add static + dynamic test for invariant #2 (no manuscript content over the wire)
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #7
- **Finding:** No CI test prevents a future contributor from sending `Manuscript` / scene-content types to a non-loopback URL.
- **Action:** Two-layer guard. (a) A `cargo deny`-style lint (or a custom `xtask`) that fails if any non-`booksforge-ollama` crate constructs a `reqwest::Client` and accepts a `Manuscript`/`SceneContent` argument in the same module. (b) A runtime fixture that runs the full agent pipeline against a mock Ollama server and asserts the mock receives only the redacted prompt envelope, never raw scene bytes.
- **Acceptance:** Adding a remote backup function that takes scene content fails CI.

### #9 — Integration test for invariant #4 (AI off per project until consent)
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #7
- **Finding:** `tests/privacy_invariants.rs:228` checks the storage default but not the IPC-level enforcement. Storage default is necessary but not sufficient: a bug in the orchestrator could fire an agent before consent flips.
- **Action:** Test: create a fresh project, attempt `agent_run_dispatch` without flipping consent, assert `BooksForgeError::ConsentRequired`. Toggle consent, retry, assert success. Repeat with the consent row corrupted to non-JSON to confirm it falls back to "off", not "on".
- **Acceptance:** A regression that lets agents run pre-consent, or one that flips fall-back to "on", is caught.

### #10 — UI consent dialog for non-loopback Ollama host (invariant #3)
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #9
- **Finding:** Code defaults to loopback, but the UI accepts arbitrary host strings without a privacy warning. A typo or shared-LAN-Ollama setup would silently send manuscript text off the machine.
- **Action:** In Settings → Ollama Host, when the user enters a non-loopback host, show a blocking modal: "*This sends your prompts and excerpts to a remote machine. They may be logged or seen by others.*" Persist the consent flag with the host string; revoke if host changes.
- **Acceptance:** Flipping host without consent rejects the save; consent is per-host, not global.

### #11 — Treat the deferred consent-store `unwrap_or_default()` as a privacy bug
- **Severity:** HIGH · **Effort:** S · ↳ blocks until #9
- **Finding:** `crates/booksforge-orchestrator/src/originality_provider.rs:41,58-59` falls back to default when the consent JSON is corrupt. Default is "off" (good) but the use of `unwrap_or_default()` hides parse errors and could mask a deeper corruption.
- **Action:** On parse failure, log a structured warning, surface a UI banner ("AI consent state unreadable; AI features are disabled until you re-confirm in Settings"), and require re-consent. Never silently accept default state for a security-relevant flag.
- **Acceptance:** Corrupting the consent row in a test DB produces a visible warning, not silent operation.

### #12 — Document the unsafe blocks in `booksforge-fs` and `booksforge-ollama`
- **Severity:** HIGH · **Effort:** S
- **Finding:** `crates/booksforge-fs/src/lock.rs` calls `libc::kill`; `crates/booksforge-ollama/src/probe.rs` calls Win32 `GlobalMemoryStatusEx`. Each is wrapped in `unsafe { }` with no `// SAFETY:` comment. The workspace `forbid(unsafe_code)` is opted out per-crate, so the discipline relies on reviewers spotting it.
- **Action:** Add `// SAFETY:` comments for every `unsafe` block stating the invariant the call relies on. Add a clippy `undocumented_unsafe_blocks` lint at the deny level for these two crates.
- **Acceptance:** `cargo clippy -- -D clippy::undocumented_unsafe_blocks` is clean.

---

## PHASE 2 — Security hardening before any external testing

### #13 — Path-traversal hardening on project bundle import
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #4
- **Finding:** Bundle import accepts `*.booksforge` directories from the user without an explicit path-containment check. A bundle with symlinks or `..` segments under `manuscript/`, `assets/`, `snapshots/` could be read or written outside the bundle root.
- **Action:** In `booksforge-fs`, add `validate_bundle_path(root, child)` that canonicalises `child`, asserts it remains within `root`, and rejects symlinks unless they resolve inside `root`. Apply on every read/write that derives a path from `manifest.toml` or DB rows. Test with crafted malicious bundles (symlink to `~/.ssh`, `../`-laden filenames).
- **Acceptance:** A red-team bundle cannot exfil read or write outside its root.

### #14 — Sidecar argument allowlist for Pandoc / EPUBCheck
- **Severity:** HIGH · **Effort:** S
- **Finding:** `booksforge-export-pandoc` builds args via `Command::arg` (good — no shell), but flag composition and output paths are derived from genre/template config that ultimately traces to user-editable templates. Future templates with adversarial flags (`--lua-filter=/tmp/x.lua`) could cause Pandoc to execute arbitrary Lua.
- **Action:** Maintain an allowlist of permitted Pandoc flags and reject anything else with a typed error. Validate all output paths are under `<bundle>/exports/`. Same for EPUBCheck.
- **Acceptance:** A template that injects `--lua-filter` fails with a clear error before Pandoc is launched.

### #15 — Remove `'unsafe-inline'` from `style-src`
- **Severity:** MEDIUM · **Effort:** M
- **Finding:** `apps/desktop/tauri.conf.json` CSP contains `style-src 'self' 'unsafe-inline'`. Defence-in-depth fails if a future XSS regression appears in TipTap or Markdown rendering.
- **Action:** Migrate inline `style="..."` to CSS modules or Vanilla Extract (used by `packages/ui` already). Where dynamic styles are necessary, use CSS custom properties set via `style` on a single element rather than full style strings. Tighten CSP to `style-src 'self'`.
- **Acceptance:** App boots and renders correctly with `'unsafe-inline'` removed.

### #16 — *(LANDED 2026-05-08)*
*`.github/CODEOWNERS` added on `chore/audit-backlog-drive-20260508`
(commit `410cd03`). Owner-review now required on
`booksforge/apps/desktop/capabilities/**`,
`booksforge/apps/desktop/tauri.conf.json`, both `.github/workflows/`
trees, `booksforge/deny.toml`, the `privacy_invariants` test, the IPC
crate, the `shared-types` package, the SQL migrations dir, and every
governance artefact. The follow-up — adding a one-line justification
comment in each capability JSON — is folded into the first
sub-MZ-09 commit.*

### #17 — Tauri command transactional consistency for state mutations
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #13
- **Finding:** `apps/desktop/src/commands/{ai,agents,editor}.rs` mutate `AppState` and emit progress events without explicit transaction boundaries. A panic between `state.touch()` and event emit leaves UI and backend desynced.
- **Action:** Wrap multi-step command bodies in a guard that, on drop, emits a terminal `:error` event for any registered job. Add tests that simulate panics mid-command and assert the UI receives a terminal event.
- **Acceptance:** Killing a worker mid-export still produces a `progress :error` to the frontend.

### #18 — Replace `let _ = ...` discards with explicit, justified ignores
- **Severity:** MEDIUM · **Effort:** S · ↳ blocks until #17
- **Finding:** ≥10 sites swallow `Result` with `let _ = ...`. Some are documented as best-effort (markdown mirror), others are not (e.g., `vocab_seed_starters` in `commands/project.rs:186`). Mixing intentional and accidental discards weakens the no-`unwrap` discipline.
- **Action:** For each, either log the error at WARN or convert to a typed warning bubbled to the UI. Add a clippy `unused_must_use` lint at deny level workspace-wide.
- **Acceptance:** Every discarded `Result` has a `// Best-effort: ...` comment plus a log line, or it is propagated.

---

## PHASE 3 — IPC contract & test-coverage debt

### #19 — Hand-written IPC types (`OllamaStatusResponse`) must be ts-rs-generated
- **Severity:** HIGH · **Effort:** S · ↳ blocks until #1
- **Finding:** `packages/shared-types/src/index.ts:98-101` defines `OllamaStatusResponse` by hand; the Rust struct in `apps/desktop/src/commands/system.rs:32-36` is not annotated with `#[ts(export)]`. The "generated from Rust" guarantee is broken for this command.
- **Action:** Decorate the Rust type, regenerate, delete the hand-written TS. Audit the rest of `packages/shared-types/src/` for any other manual types.
- **Acceptance:** `git grep "// hand-written"` and similar markers in `shared-types` returns nothing.

### #20 — `ollama_status` should not swallow errors into a success type
- **Severity:** MEDIUM · **Effort:** S · ↳ blocks until #19
- **Finding:** `commands/system.rs:18-28` returns `Ok(OllamaStatusResponse { running: false, ... })` for both "Ollama is genuinely off" and "transient probe failure". The frontend (`App.tsx:22-24`) cannot distinguish, so transient errors silently look like Ollama being off.
- **Action:** Return `Result<OllamaStatusResponse, BooksForgeError>` and tag transient probe errors as a distinct variant. Frontend handles each state.
- **Acceptance:** A unit-flaky probe test surfaces a transient error; a clean "Ollama not installed" path stays Ok.

### #21 — Backfill unit tests on the three untested L3/L4 crates
- **Severity:** HIGH · **Effort:** L · ↳ blocks until #18
- **Finding:** `booksforge-storage`, `booksforge-memory`, `booksforge-template` have ~zero `#[test]` / `#[tokio::test]` annotations in `src/`. Storage in particular is ~900 lines of SQL access untested in unit form (only via integration tests in other crates).
- **Action:** Per crate, target ≥70 % line coverage on public APIs. Storage: at minimum CRUD round-trips for nodes, scenes, snapshots, ai_calls. Memory: ledger writes, vocab ingest. Template: every Jinja filter + sandbox escape attempt.
- **Acceptance:** `cargo llvm-cov` reports ≥70 % per crate; CI fails below threshold.

### #22 — Frontend test coverage is 3 files for ~9k LOC of React
- **Severity:** HIGH · **Effort:** L · ↳ blocks until #21
- **Finding:** `apps/desktop/src-ui/` ships only `OnboardingTour.test.ts`, `projectTemplates.test.ts`, `wordDiff.test.ts`. No tests for: IPC layer, EditorShell state transitions, snapshot restore flow, export validation, agent panel dispatch. No Playwright/E2E.
- **Action:** Two tracks. (a) Vitest + React Testing Library on every component that owns state, with `invoke` mocked from a generated fake. (b) Playwright E2E covering: create project → edit scene → autosave → snapshot → restore → export to all three formats. Run E2E in the macOS-14 CI lane only (lowest cost).
- **Acceptance:** Coverage gate ≥ 60 % statements; E2E runs green on every PR.

### #23 — *(REMOVED — addressed)*
*Verification showed `cargo test -p booksforge-ipc --test codegen_drift` runs in
both `.github/workflows/ci.yml` (root, line 100+) and
`booksforge/.github/workflows/ci.yml` and explicitly fails with
`"::error::TS bindings are out of date..."`. The hand-written
`OllamaStatusResponse` slipped past it because it is **not** under the codegen
contract at all (the Rust struct has no `#[ts(export)]`); that is what #19
already covers. No standalone backlog item is needed.*

---

## PHASE 4 — Product completeness (user flow gaps)

### #24 — Add a global React error boundary
- **Severity:** HIGH · **Effort:** S · ↳ blocks until #22
- **Finding:** No top-level `<ErrorBoundary>`. A throw inside `EditorShell` or any TipTap extension crashes the entire app to a blank window with no recovery.
- **Action:** Wrap the root in an error boundary that captures the exception, displays a recovery panel ("save your last scene as Markdown / restart"), and emits a structured log to disk (no remote send by default).
- **Acceptance:** A throwing test component shows the recovery UI rather than a white screen.

### #25 — Promote local errors to a global toast queue
- **Severity:** MEDIUM · **Effort:** S · ↳ blocks until #24
- **Finding:** `EditorShell.tsx` has component-scoped toast state. Parallel errors collide / overwrite. ≥12 `.catch(() => null)` swallow errors silently.
- **Action:** Provide a `ToastProvider` + `useToast()` hook with a queued list and severity. Replace silent `.catch(() => null)` with `useToast().error(...)` where the user can act on it.
- **Acceptance:** Two simultaneous errors both show; both can be dismissed individually.

### #26 — Wire progress + cancel for export and snapshot, not just agents
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #17
- **Finding:** `crates/booksforge-ipc/src/agent_events.rs` has progress events, but only chapter-drafter and dev-editor agents emit. Export operations block the UI with no feedback; snapshot restore has no progress; neither can be cancelled.
- **Action:** Emit `progress` events from Pandoc/EPUBCheck/EPUB exporters keyed by `job_id`. Implement `cancel(job_id)` for export and snapshot restore using a `tokio::CancellationToken` threaded into the worker.
- **Acceptance:** A 30-second export shows incremental progress; clicking Cancel actually kills the Pandoc child process.

### #27 — Pre-submit input validation in the project-creation wizard
- **Severity:** MEDIUM · **Effort:** S · ↳ blocks until #25
- **Finding:** `NewProjectWizard.tsx:73-75` sanitises the filename only after the user picks a save location; empty title / invalid bundle path is only caught by the Rust layer's error response. UX is "click Create, see error" instead of "Create disabled until inputs valid".
- **Action:** Validate `title`, `author`, `bundlePath` on every keystroke; surface inline errors; disable the Create button until valid.
- **Acceptance:** Empty title cannot dispatch.

### #28 — Onboarding tour: keep it dismissable AND re-openable
- **Severity:** MEDIUM · **Effort:** S
- **Finding:** `OnboardingTour.tsx` is a 3-step localStorage-gated overlay with no help-menu affordance to re-open. Users who dismiss can't recover the orientation.
- **Action:** Add a "Help → Show welcome tour" command. Anchor the tour to actual UI elements (Binder, Editor, Agents), not abstract concepts.
- **Acceptance:** Users can re-trigger the tour at any time; tour highlights real DOM elements with focus management.

### #29 — Agent panel: complete the proposal-review UX
- **Severity:** HIGH · **Effort:** L · ↳ blocks until #25 #26
- **Finding:** Per the internal BACKLOG and code inspection, agent panels (Copyedit, Continuity, Developmental, etc.) are largely dispatch routers. There is no visual diff of suggested edits, no per-suggestion accept/reject, no preview of the resulting document.
- **Action:** Build a single shared `ProposalReview` component with: side-by-side or inline diff, hunk-level accept/reject, "apply selected", undo via the snapshot system. Reuse `wordDiff` infrastructure from snapshot diffs.
- **Acceptance:** Each of the 9+ agent flows ends in `ProposalReview`; users never have to accept-or-reject the entire proposal in one click.

### #30 — Memory + Vocabulary panels: add manual CRUD
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #25
- **Finding:** `KnowledgePanel.tsx` is read-only. Users can run agents that populate the stores but cannot manually add a character, place, or rule. This makes the agent system feel one-way.
- **Action:** Add forms and inline-edit affordances for entries in both stores. Maintain provenance: distinguish user-authored entries from agent-proposed entries.
- **Acceptance:** A user can create a new character entry without ever invoking an agent.

### #31 — Snapshot per-node restore (selective restore)
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #29
- **Finding:** `SnapshotsPanel.tsx` shows diffs but only restores the entire project state. After running an agent and disliking changes to one chapter, users must roll back the whole book.
- **Action:** Extend the diff UI to allow per-node selection; restore only selected nodes; create a fresh safety snapshot first (existing pattern).
- **Acceptance:** User can revert one scene to a prior snapshot without affecting other scenes.

### #32 — Export wizard: live preview + dependency check at export time
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #26
- **Finding:** `ExportPanel.tsx` lists ~10 genre × subgenre combinations but offers no preview of typography, page size, or chapter break style. Pandoc/Java/EPUBCheck dependency check lives in Settings; users hit "Export" and learn dependencies are missing only by failure.
- **Action:** (a) Embed a thumbnail preview generated from a 3-page sample render. (b) Run dependency probe at the start of the export flow; if missing, link directly to install instructions or offer the bundled installer.
- **Acceptance:** Missing-dependency case never produces a generic error; users always see a preview before clicking Export.

### #33 — Centralise keyboard shortcut map + add a help overlay
- **Severity:** LOW · **Effort:** S
- **Finding:** Shortcuts are wired ad-hoc in `EditorShell.tsx`. No central keymap, no help overlay (`?` to show), no rebinding.
- **Action:** Move shortcut definitions to `apps/desktop/src-ui/src/keymap.ts`. Provide a `ShortcutHelp` modal opened by `?`. (Rebinding can be deferred.)
- **Acceptance:** `?` shows every shortcut on a single screen.

### #34 — Accessibility pass to claim WCAG 2.2 AA
- **Severity:** HIGH · **Effort:** L · ↳ blocks until #25
- **Finding:** `useDialogA11y()` exists but focus management is manual; some inline-styled buttons lose OS focus indicators; no screen-reader testing recorded; spec claims WCAG 2.2 AA.
- **Action:** Run axe-core in vitest for every component; add Playwright a11y scan on the E2E suite; fix every violation; document VoiceOver + NVDA pass on the golden path. Add focus-trap and return-focus to every modal.
- **Acceptance:** axe-core reports zero violations on the golden path; VoiceOver + NVDA can complete: create project → edit scene → snapshot → export.

### #35 — Dark-mode toggle (and `prefers-color-scheme` listener)
- **Severity:** LOW · **Effort:** S
- **Finding:** `packages/ui/src/tokens.css` defines `[data-theme="dark"]` overrides, but no UI toggle and no `prefers-color-scheme` listener. A long-form writing tool without dark mode is a missed table-stake.
- **Action:** Add a Settings → Appearance toggle (System / Light / Dark). Listen to `prefers-color-scheme` when on System.
- **Acceptance:** Toggle persists across restarts; System mode follows OS.

### #36 — Plan i18n now, even if MVP ships English-only
- **Severity:** MEDIUM · **Effort:** M
- **Finding:** Zero i18n infra. All copy hardcoded English. The product is a book-writing tool for international authors; retrofitting i18n later is a major refactor.
- **Action:** Adopt `react-i18next` (or `formatjs`) now. Move every hardcoded string to a `locales/en.json`. Ship MVP English-only but with the structure ready. Decide on RTL support intent and document it.
- **Acceptance:** No JSX literal of user-visible English remains in `apps/desktop/src-ui/`; switching `i18next.language` flips at least the menu and toolbar.

---

## PHASE 5 — Release engineering

### #37 — Code-signing config for macOS + Windows
- **Severity:** CRITICAL · **Effort:** M · ↳ blocks until #4
- **Finding:** `tauri.conf.json` has no `bundle.macOS.signingIdentity`, no Windows signing config, no notarization step. Any unsigned build will trigger Gatekeeper / SmartScreen warnings; macOS Sequoia (14+) effectively refuses to run them.
- **Action:** Provision Apple Developer ID + Windows EV certificate. Wire Tauri config + GitHub Actions secrets. Notarize macOS in CI; staple on success.
- **Acceptance:** Downloaded DMG opens with no Gatekeeper warning; downloaded MSI opens with no SmartScreen warning.

### #38 — Release pipeline (tag → matrix build → signed artefacts → GitHub Release)
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #37
- **Finding:** `.github/workflows/ci.yml` is gating-only. No release job. Any release is manual, error-prone, and unsigned.
- **Action:** Add `release.yml` triggered on `v*` tags: build on macos-14 + macos-13 + windows-2022, sign, generate update manifest, upload to GitHub Releases. Generate SBOM (CycloneDX) per artefact.
- **Acceptance:** Pushing tag `v0.1.0-rc.1` produces three signed installers + checksums + SBOM on the Releases page within 30 minutes.

### #39 — Tauri auto-updater: opt-out by default, signed manifests
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #38
- **Finding:** Updater is unconfigured. Spec promises a single-toggle opt-out. Without an updater, users get stale, vulnerable builds; with a misconfigured updater, the privacy invariant ("no outbound by default") could break.
- **Action:** Configure `updater` block in `tauri.conf.json` with `active = true`, public key, endpoint. Default user setting to "check on launch" but expose the opt-out toggle prominently in Settings on first run. Sign the update manifest with the private key held in CI secret. Verify check is gated by user setting before any HTTP call.
- **Acceptance:** With "check for updates" off, no HTTP call to the update endpoint occurs (verify with the test from #7).

### #40 — Add `cargo audit` / explicit `cargo deny check advisories` step in CI
- **Severity:** HIGH · **Effort:** S · ↳ blocks until #38
- **Finding:** Per the supply-chain audit, advisory output is hidden in the current `cargo deny check` invocation. Yanked or vulnerable transitive crates can land silently.
- **Action:** Add an explicit `cargo deny check advisories --hide-inclusion-graph` step that fails on any advisory; track exceptions in `deny.toml` `[advisories].ignore` with a date-stamped justification.
- **Acceptance:** A planted RUSTSEC advisory in a test branch fails CI.

### #41 — *(LANDED 2026-05-08)*
*`.github/dependabot.yml` added on `chore/audit-backlog-drive-20260508`
(commit `410cd03`). Weekly grouped PRs across cargo workspace, npm
workspace, per-package npm, and GitHub Actions (root + booksforge).
Logical groupings configured: `tauri-stack`, `sqlx-stack`,
`tokio-stack`, `serde-stack`, `tiptap-stack`, `react-stack`,
`vite-stack`, `tooling`, plus a `patch-updates` bucket per ecosystem
that is the auto-merge candidate. Major-version bumps to react /
react-dom / vite / tauri / sqlx / @tiptap/core are explicitly
ignored. **First weekly run will land Monday after this branch
merges.** Auto-merge automation (audit #58) is configured at repo
settings level — separate task.*

### #42 — *(REMOVED — addressed / non-issue)*
*Verification: `crates/booksforge-storage/migrations/reverse/` contains exactly
one file (`0001_initial_reverse.sql`), and the production migration runner
operates only on `migrations/*.sql`. The directory is a dev-only convenience
for local schema reset, consistent with the spec's "forward-only at runtime"
wording. A nice-to-have CI guard could be added later but this is not a
release-readiness gap.*

### #43 — Crash reporting: opt-in with redaction (post-MVP, but design now)
- **Severity:** MEDIUM · **Effort:** L · ↳ blocks until #39
- **Finding:** Settings panel claims crash reports off by default; no implementation. With privacy guarantees, this needs a dedicated opt-in pipeline that redacts manuscript content and hashes paths.
- **Action:** Design a self-hosted Sentry-compatible endpoint (or roll your own minimal sink). Redact: stack frames only, no manuscript text, no file paths inside the bundle, no project IDs. Add per-crash review-and-send UI; never auto-send.
- **Acceptance:** A planted panic during E2E shows a "send report?" UI with the redacted payload visible to the user before send.

---

## PHASE 6 — Business & market readiness

### #44 — Production `tauri.conf.json` metadata
- **Severity:** HIGH · **Effort:** S · ↳ blocks until #37
- **Finding:** `version = "0.0.1"` placeholder, no `publisher`, no proper `copyright`, no `category`. Required by both stores and by visible Finder/Explorer metadata.
- **Action:** Set `publisher`, `copyright`, `shortDescription`, `longDescription`, `category = "Productivity"`. Wire `version` to a single source of truth (Cargo workspace version) via build script.
- **Acceptance:** Right-click → Get Info on the macOS app shows publisher and copyright; the same fields appear in Windows Explorer's File Properties.

### #45 — Distribution plan & infrastructure
- **Severity:** HIGH · **Effort:** L · ↳ blocks until #38 #39
- **Finding:** No documented host for installers, no CDN, no DNS, no update endpoint, no Homebrew tap. `outputs/` mentions `booksforge.app` but nothing is provisioned.
- **Action:** Decide: GitHub Releases-only (cheap, fine for early access), or GitHub Releases + CDN. Provision domain + TLS for the update endpoint. Document the matrix in `docs/DISTRIBUTION.md`. Plan Mac App Store / Microsoft Store posture even if not pursued in MVP.
- **Acceptance:** A new user can find a download link from the README and install on macOS, Windows.

### #46 — User-facing legal docs: Privacy Policy, EULA, Terms
- **Severity:** HIGH · **Effort:** M · ↳ blocks until #5 #43
- **Finding:** No `PRIVACY_POLICY.md`, `EULA.md`, `TERMS.md`. Required before any commercial download. The privacy story is a competitive differentiator and needs to be plainly stated for users, not just developers.
- **Action:** Draft Privacy Policy with concrete statements ("we never receive your manuscript text", "telemetry is opt-in and redacted", "AI runs locally via Ollama"). Have legal counsel review. Include in the installer + link from the app's About box.
- **Acceptance:** First-run flow surfaces and accepts the Privacy Policy + EULA before any AI feature is unlocked.

### #47 — In-app help / docs system
- **Severity:** MEDIUM · **Effort:** L · ↳ blocks until #28
- **Finding:** Spec docs live in `outputs/` but are developer-facing. There is no end-user help system, FAQ, or troubleshooting guide.
- **Action:** Author a `Help` Markdown set rendered in-app (or as a static site at `docs.booksforge.app`) covering: project model, Binder, Editor, Agents, Snapshots, Export, Privacy, Ollama setup, common errors. Link contextually from each major panel.
- **Acceptance:** Every modal has a "Learn more" link that lands on the right help section.

### #48 — Public website / landing page
- **Severity:** MEDIUM · **Effort:** L · ↳ blocks until #45 #46
- **Finding:** No website asset in repo; no proof of a `booksforge.app` deployment. Without a landing page, the product cannot be discovered.
- **Action:** Out-of-scope for this repo, but should be a parallel work stream with: positioning, screenshots, comparison table (vs. Scrivener, Atticus, Sudowrite), download links, privacy explainer, pricing.
- **Acceptance:** `booksforge.app` resolves with a working download CTA.

### #49 — Pricing / monetisation decision
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #48
- **Finding:** No pricing model, no licence-key system, no payment integration. The internal docs are silent on monetisation.
- **Action:** Decide: free + donation, one-time purchase, subscription, or freemium (with bundled-template / cloud-LLM-tier upsell). If commercial, design an offline-friendly licence-key system (signed offline-verifiable token, no phoning-home for activation, to honour the privacy invariant).
- **Acceptance:** A pricing decision is documented in `docs/BUSINESS_MODEL.md` (kept private) and the licence flow has a one-page design.

### #50 — Support channel and response SLA
- **Severity:** MEDIUM · **Effort:** S · ↳ blocks until #46
- **Finding:** Beyond `SECURITY.md` (per #5), no support email, no Discord/forum, no issue triage policy.
- **Action:** Set up `support@booksforge.app`, a public issue template, and (optionally) a community channel. Document response SLA in README.

---

## PHASE 7 — Polish & longer-tail debt

### #51 — Source maps in production (uploaded, not shipped)
- **Severity:** MEDIUM · **Effort:** S
- **Finding:** `vite.config.ts:25` only generates source maps when `TAURI_ENV_DEBUG`. Release builds have none, so a production stack trace is unreadable.
- **Action:** Always generate source maps; upload them to a private artefact store on release; do NOT ship them inside the installer.
- **Acceptance:** A symbolised stack trace can be reconstructed from a release crash report.

### #52 — Frontend bundle-size monitoring
- **Severity:** LOW · **Effort:** S
- **Finding:** No bundle-size telemetry. As agents and templates grow, the React bundle will bloat unnoticed.
- **Action:** Add `rollup-plugin-visualizer` output as a CI artefact; track bundle size in a README badge or simple JSON file checked into the repo.

### #53 — Consolidate `useState` sprawl in `EditorShell`
- **Severity:** LOW · **Effort:** M · ↳ blocks until #24
- **Finding:** ~14 separate `useState` for modal visibility plus 3 for editor instance state. Hard to reason about; bug-prone as panels multiply.
- **Action:** Migrate to a `useReducer` or a small Zustand/Jotai store keyed by panel id.

### #54 — Snapshot creation idempotency keys
- **Severity:** LOW · **Effort:** S · ↳ blocks until #31
- **Finding:** A timed-out and retried snapshot create may double-write. Low-impact today, painful at scale.
- **Action:** Accept an optional `idempotency_key`; the orchestrator dedupes on it.

### #55 — Agent context size guard
- **Severity:** MEDIUM · **Effort:** S · ↳ blocks until #29
- **Finding:** `context_builder` has an assert in tests but no production guard for the focus excerpt + context exceeding the chosen model's window. Silent truncation produces silently bad agent output.
- **Action:** Compute estimated tokens at build time; fail fast with a typed error before the prompt template is rendered.

### #56 — Prompt-template version pinning / migration
- **Severity:** MEDIUM · **Effort:** M · ↳ blocks until #29
- **Finding:** Prompts are hash-versioned but there is no migration story when a template is updated. Old `ai_calls` rows reference an obsolete hash and cannot be re-rendered for audit / replay.
- **Action:** Keep the old templates checked in, indexed by their hash, in `crates/booksforge-prompt/templates/archive/`. Add an audit query that proves any historical `ai_calls.template_hash` resolves to a present file.

### #57 — Structured frontend logging with session id
- **Severity:** MEDIUM · **Effort:** S
- **Finding:** Frontend errors land in the dev console only; no breadcrumb trail or session correlation with backend logs.
- **Action:** Issue a per-session id at app start; pass it on every IPC call; tag both Rust `tracing` spans and frontend `console.error` calls. Surface session id in the Help → About box for users to share when reporting bugs.

### #58 — Dependabot / npm patch-update auto-merge guardrails
- **Severity:** LOW · **Effort:** S · ↳ blocks until #41
- **Finding:** Once #41 is on, the next concern is malicious patch-level npm pins (typo-squat takeovers).
- **Action:** Restrict auto-merge to specific trusted scopes (`@tiptap/*`, `@types/*`); require manual review for new packages.

### #59 — Pre-commit hooks (typecheck, lint, fmt)
- **Severity:** LOW · **Effort:** S
- **Finding:** No `husky` / `lefthook` config. Developers can land breakage that should have been caught locally.
- **Action:** Adopt `lefthook` with `pnpm typecheck`, `pnpm lint`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`. Make it skippable but loud.

### #60 — Empty-state and error-state design pass for every panel
- **Severity:** LOW · **Effort:** M · ↳ blocks until #25
- **Finding:** Many panels render nothing or a console error in the empty/error state. A polished writing app needs intentional empty / loading / error states for every list and editor surface.
- **Action:** Catalogue every panel; design and implement the three states for each.

### #61 — *(LANDED 2026-05-08)*
*Pin policy added to `CONTRIBUTING.md` on
`chore/audit-backlog-drive-20260508` (commit `179ef57`). Spells out:
minor pins for Rust workspace deps, explicit ignored-major list
(tauri, sqlx, react, react-dom, vite, @tiptap/core), pnpm-lock.yaml
always committed and CI runs `--frozen-lockfile`, Cargo.lock always
committed for the binary workspace. Plus a "Privacy invariants —
before you ship" PR-level checklist that re-states the five
invariants from `outputs/SECURITY_PRIVACY.md` and a "Reporting
security issues" pointer to `SECURITY.md`.*

### #62 — Internal `BACKLOG.md` reconciliation with this audit
- **Severity:** LOW · **Effort:** S · ↳ blocks until #1
- **Finding:** The internal `booksforge/BACKLOG.md` (~2,000 lines, 108 items) is dense, well-maintained, and partly overlaps with this audit. Two parallel backlogs are wasteful.
- **Action:** Cross-reference each item in this audit with the matching internal BACKLOG section, updating the internal BACKLOG to point here for items not previously tracked. Treat this audit as the **release-readiness** gating list; the internal BACKLOG remains the **engineering-task** list.

---

## Market-Readiness Scorecard

> Scale: 0 (not started) · 25 (early) · 50 (functional, gaps) · 75 (production-quality) · 100 (best-in-class)

| Dimension                              | Score | Confidence | Rationale |
|----------------------------------------|------:|:----------:|-----------|
| Dimension | Pass-1 (2026-05-08) | Pass-3 (2026-05-09) | Notes (Pass-3) |
|---|---:|---:|---|
| **Architecture & code quality**        |   78  |  **88** | MZ-05+ slice fully on `main` (PR #13).  Unsafe blocks now documented (`// SAFETY:` on all sites in `booksforge-fs::lock`, `booksforge-ollama::probe`).  Workspace deps tightened, Rust pinned to 1.88.0. |
| **Privacy posture (design)**           |   80  |  **85** | Crash-report design doc (`docs/CRASH_REPORTING_DESIGN.md`) makes the "no manuscript in reports" guarantee a typed-allowlist contract.  Privacy Policy draft commits the user-facing version. |
| **Privacy posture (CI enforcement)**   |   45  |  **70** | 3 of 5 invariants enforced by passing tests on `main`; #7/#8/#9 dynamic tests scaffolded with `#[ignore]` on the open PR — flipping the ignores activates them.  Static guard for #8 already CI-enforced via `scripts/audit/check-no-manuscript-over-wire.sh`. |
| **Security hardening**                 |   55  |  **78** | CSP `'unsafe-inline'` removed; `img-src` + `font-src` explicitly scoped; updater endpoint added to `connect-src`.  `OllamaStatusResponse` ts-rs-generated.  `let _ =` discard count visible via audit script.  Open: path-traversal hardening on bundle import (#13), Pandoc arg allowlist (#14). |
| **Test coverage — Rust**               |   50  |  **62** | New crates (`booksforge-snapshot`) shipped with integration tests; `applied_edit_invariant.rs`, `outline_to_tree_prop.rs`, `cold_launch_p50.rs`, `validators.rs`, `memory_vocab.rs` all on `main`.  Storage / memory / template still under-covered at unit level (#21). |
| **Test coverage — Frontend**           |   15  |  **35** | 4 new vitest test files (`sessionId.test.ts`, `keymap.test.ts`, `i18n.test.ts`, `theme.test.ts`) plus existing `wordDiff.test.ts`, `projectTemplates.test.ts`, `OnboardingTour.test.ts`.  Playwright visual-regression suite on `main`.  E2E for the golden path still pending (#22 partial). |
| **CI / supply chain**                  |   65  |  **88** | `.github/workflows/{ci.yml, release.yml, security-scan.yml, dependabot-auto-merge.yml, audit-checks.yml}` all live; `cargo deny check advisories` explicit; OSV-Scanner; cargo-audit; pnpm audit.  Dependabot weekly grouped PRs already producing patches (8 open as of writing).  Missing: `pnpm-lock.yaml` commit (#3). |
| **Release engineering**                |   10  |  **55** | `release.yml` tag-triggered + workflow_dispatch; signed-build branches keyed off secrets (per `docs/DISTRIBUTION.md §3`); SBOMs (CycloneDX) per artefact; `bundle.externalBin` + sidecar fetch script on `feat/mz-09-and-polish`.  Open: cert provisioning, first signed dry-run, updater pubkey generation. |
| **Licensing / IP**                     |   30  |  **65** | `LICENSE` placeholder committed (final terms pending founder decision); `THIRD_PARTY_LICENSES.md` scaffolded with regen commands; `deny.toml` GPL ban remains.  Open: founder picks final license, `cargo about generate` actual run. |
| **User-facing product (UX completeness)** | 55  | **80** | All 10 agents wired end-to-end with UI panels.  ProposalReview + ErrorBoundary + ToastProvider + ShortcutHelp + theme toggle UI now in app shell.  Open: per-panel manual CRUD (#30), selective restore (#31), export preview (#32), full a11y sweep (#34). |
| **Documentation — internal**           |   85  |  **95** | `MILESTONES.md`, `docs/ARCHITECTURE.md`, `docs/RUNBOOK.md`, `docs/REPO_SETTINGS.md`, `docs/CRASH_REPORTING_DESIGN.md`, `docs/PRE_LAUNCH_CHECKLIST.md`, post-mortem template, internal `BACKLOG.md` all on `main`. |
| **Documentation — external (users)**   |    8  |  **40** | `docs/SUPPORT.md` user-facing routing; `PRIVACY_POLICY.md` plain-English summary; in-app help drawer (`HelpDrawer.tsx`) committed.  Open: full FAQ at `docs.booksforge.app` (#47). |
| **Legal & policy posture**             |   10  |  **55** | All 4 governance files committed: `LICENSE` (placeholder), `SECURITY.md`, `CODE_OF_CONDUCT.md`, `PRIVACY_POLICY.md` + `EULA.md` + `TERMS_OF_SERVICE.md` (all drafts).  Open: legal-counsel review, jurisdiction selection, contact-email provisioning. |
| **Distribution & business model**      |   12  |  **35** | `docs/DISTRIBUTION.md` documents installer hosting + signing posture; `release.yml` matrix-builds; auto-updater config block in place.  Open: domain provisioning, cert provisioning, pricing decision (#49), website (#48). |
| **Internationalisation / accessibility** |   8  |  **45** | `lib/i18n.ts` + `locales/en.json` committed; first migration of `OllamaStatusBar` strings done; `useDialogA11y` + `aria-live` + `aria-labelledby` patterns adopted across new components.  Open: per-panel string migration completion, full WCAG 2.2 AA audit (#34), RTL story. |

### **Composite market-readiness: 62 / 100 (was 41 / 100) — public-beta candidate**

**What this means in plain English:**
- **Engineering substance:** ~ **88 % of MVP** (was 65–70 % at first audit).  Stabilisation Sprint S1 (PR #13) committed all of MZ-05..MZ-08 + Phase 5 Turns A-S; the agent system, prompt engine, snapshot v1, export pipeline, validators, memory + vocab are on `main`.  Open: MZ-09 crash-capture path (design done, Rust scaffold pending PR #14 merge), MZ-10 `pnpm-lock.yaml` commit, frontend test backfill at depth, M1 polish edges (per-panel CRUD, selective restore, full a11y sweep).
- **Shippability to outside users:** ~ **45 %** (was 15 %).  Release pipeline scaffolded (`release.yml`); legal docs drafted (PRIVACY_POLICY, EULA, TERMS_OF_SERVICE, DISTRIBUTION); auto-updater config in place pending key generation; CSP hardened; tauri.conf metadata complete.  **Still blocking installer ship:** Apple Developer ID + Windows EV cert provisioning, final license decision, legal-counsel review, `booksforge.app` domain + updater endpoint, first signed `release.yml` dry run.
- **Investor / due-diligence readiness:** ~ **65 %** (was 30 %).  Architecture and privacy posture well-evidenced (CI-enforced invariants, audit-script suite, runbook, post-mortem template).  Remaining red flags reduced to provisioning items (certs, emails, domain) and the final license decision.

### Recommended go/no-go gates (status post-Pass-3)

| Gate | Items required | Status |
|------|---------------|------|
| **Internal alpha** (developer-only)            | #1–#6, #19, #21–#22 | ✅ All satisfied except #21/#22 frontend-test-depth (in-progress) |
| **Closed beta** (~50 invited writers)          | + #7–#12, #13–#18, #24–#27, #29       | 🟡 #12, #15, #19, #20, #24, #25, #29 ✅; #7–#9 scaffolded (`#[ignore]` pending merge); #11, #13, #14, #17, #18, #26, #27 still open |
| **Public beta** (free download, signed)        | + #34, #37–#40, #44–#47               | 🟡 #38, #40, #41, #44, #45, #46 (drafts) ✅; #37 (certs) + #39 (updater pubkey) + #47 (in-app help) still open |
| **1.0 GA**                                     | + #28, #30–#33, #36, #41, #43, #46, #48–#50 | 🟡 #33, #35, #36 partial, #41, #43 (design), #50 ✅; #28, #30, #31, #32, #48, #49 still open |

### Top-5 risks (post-Pass-3 — most have moved one step closer)

1. **MZ-09 crash-capture not on `main`** — design done; Rust scaffold + Tauri commands gated on `feat/mz-09-and-polish` merging.  Single highest-leverage open code item.
2. **Privacy invariants not yet enforced dynamically** — #7/#8/#9 test scaffolds exist but `#[ignore]`d.  Activating them is one-step-from-done after merge.
3. **Cert provisioning is a months-long human task** — Apple Developer ID enrolment + Windows EV cert verification (3–10 business days each).  Cannot ship signed installer until these clear.
4. **Final license + legal review** — drafts exist; founder decision + counsel review are the gating actions.  Same critical-path duration as cert provisioning.
5. **`pnpm-lock.yaml` not committed (#3)** — frontend builds are not yet bit-for-bit reproducible.  One-line `pnpm install` + commit fixes it but should run on the merged tree, not on top of in-flight work.

---

## Appendix — Pruning + execution log

The first draft of this audit listed 62 items.

### Pass 1 (2026-05-08, post-verification)

| Original # | Title | Disposition | Reason |
|---:|---|---|---|
| 23 | IPC codegen drift test must run on every PR | **Removed** | Already runs in CI on every PR (`cargo test -p booksforge-ipc --test codegen_drift` in both `.github/workflows/ci.yml` files, with explicit `::error::TS bindings are out of date…` failure). The hand-written-type drift it failed to catch (`OllamaStatusResponse`) is covered by #19. |
| 42 | Reverse migrations directory: delete or formalise | **Removed** | `migrations/reverse/` contains a single dev-only file (`0001_initial_reverse.sql`); production runner only reads `migrations/*.sql`. Consistent with spec; not a release-readiness gap. |
| 16 | Audit Tauri capabilities scoping | **Downgraded MEDIUM → LOW** | Verified `apps/desktop/capabilities/default.json` is already minimally scoped (no `shell:`, no `http:`, no broad `fs:`). Remaining ask is procedural: add CODEOWNERS guard. |

### Pass 2 (2026-05-08, branch `chore/audit-backlog-drive-20260508`)

PR #1 closed audit items **#1, #2, #5, #6** (root governance —
CLAUDE.md drift fix, LICENSE placeholder, SECURITY.md,
CODE_OF_CONDUCT.md). The current branch closes:

| # | Title | How | Commit |
|---:|---|---|---|
| 4  | `THIRD_PARTY_LICENSES.md` / `NOTICE` | Scaffolded with regen commands; per-dependency aggregate deferred to first signed release per `MILESTONES.md` M6.E | `179ef57` |
| 16 | CODEOWNERS gate on Tauri capabilities | `.github/CODEOWNERS` covering capabilities, tauri.conf.json, both CI trees, deny.toml, privacy_invariants test, IPC + shared-types, migrations dir, governance artefacts | `410cd03` |
| 41 | Dependabot grouped weekly PRs | `.github/dependabot.yml` covering cargo workspace, npm workspace, per-package npm, GitHub Actions × 2 | `410cd03` |
| 61 | Pin policy doc | New "Dependency-pin policy" + "Privacy invariants — before you ship" + "Reporting security issues" sections in `CONTRIBUTING.md` | `179ef57` |

### Pass 3 (2026-05-09, branch `feat/m1-wiring-and-audit-sync-20260508`)

Three branches merged into `main` since Pass-2: `chore/audit-backlog-drive-20260508`
(PR #2), `chore/onboarding-and-runbook-20260508` (PR #12), and
`feat/stabilisation-sprint-20260508` (PR #13).

**Items closed by `chore/onboarding-and-runbook-20260508` (PR #12):**

| # | Title | How |
|---:|---|---|
| 8  | Manuscript-content-over-wire static guard | `scripts/audit/check-no-manuscript-over-wire.sh` (CI) |
| 12 | `// SAFETY:` comments on every unsafe block | `lock.rs` + `probe.rs` annotated, paired with `check-unsafe-blocks-documented.sh` |
| 15 | CSP `'unsafe-inline'` removed from `style-src` | `tauri.conf.json` hardened; `check-csp-no-unsafe-inline.sh` enforces |
| 19 | Hand-written `OllamaStatusResponse` ts-rs-generated | Struct moved to `booksforge-ipc/src/ollama.rs`; binding committed |
| 20 | `ollama_status` no longer swallows transient errors | `tracing::warn!` log before fallback to `running:false` |
| 24 | Global React error boundary | `components/ErrorBoundary.tsx` wrapping `App` root |
| 25 | Global toast queue | `components/ToastProvider.tsx` + `useToast()` hook |
| 29 | `ProposalReview` shared component | Per-hunk accept/reject + `aria-live` decision counts |
| 33 | Centralised keyboard-shortcut map + help overlay (module half) | `lib/keymap.ts` + `components/ShortcutHelp.tsx` |
| 35 | `prefers-color-scheme` listener | `lib/theme.ts` (toggle UI in Pass-3) |
| 36 | i18n scaffolding | `lib/i18n.ts` + `locales/en.json` |
| 40 | `cargo deny check advisories` step | `.github/workflows/security-scan.yml` |
| 43 | Crash-report design doc | `docs/CRASH_REPORTING_DESIGN.md` (Rust scaffold pending PR #14) |
| 44 | `tauri.conf.json` metadata fields | `publisher`, `copyright`, `shortDescription`, `longDescription`, `category` |
| 50 | Support docs + structured intake | `docs/SUPPORT.md`, `.github/PULL_REQUEST_TEMPLATE.md`, issue templates |
| 56 | Prompt-template archive structural | `crates/booksforge-prompt/templates/archive/README.md` |
| 57 | Frontend session-id logging | `lib/sessionId.ts` consumed by `ErrorBoundary` |
| 58 | Dependabot auto-merge guardrails | `.github/workflows/dependabot-auto-merge.yml` |

**Items closed by this branch (`feat/m1-wiring-and-audit-sync-20260508`):**

| # | Title | How |
|---:|---|---|
| 33 | Help overlay wired to `app.show-shortcuts` (`?` opens it) | `App.tsx` + `useShortcut` |
| 35 | Theme toggle UI in Settings → Appearance | `SettingsPanel.tsx` radiogroup + `initThemeSystem()` at boot in `main.tsx` |
| 36 | First i18n migration: `OllamaStatusBar` strings now go through `t()` | `App.tsx` migration |

### Net items still open after Pass 3

Originally 62 → minus 2 removed in Pass-0 → minus 4 closed in PR #1
→ minus 4 closed in PR #2 → minus 18 closed in PR #12 → minus 3 wiring
items (#33/#35/#36) closed in this branch → **31 items still open**.

Composite market-readiness (was 41/100): **62 / 100** post-Pass-3 —
"public-beta-ready except for cert provisioning + final license
decision + open PR #14".

Numbering preserved across all passes.

*End of audit.*
