# Phase 00 — Bootstrap

> **Status note (2026-05-06):** This phase prompt is **superseded** by MZ-01 in `IMPLEMENTATION_PLAN.md`. The procedures below are preserved for historical context; use the implementation pack for the MVP build.

## Goal

Stand up a Cargo workspace + Tauri v2 + React + TypeScript monorepo that boots a "Hello, manuscript" window and round-trips a typed IPC call. CI runs on Windows, macOS, and Linux. Pre-commit hooks and lints are wired. Signing-cert procurement is documented (procurement itself is a human task, but the integration points are stubbed).

## Pre-conditions

- Empty Git repo, default branch `main`.
- Decision-makers available for signing-cert procurement (out of band).
- Node ≥ 20, Rust ≥ 1.79, pnpm ≥ 9 installed locally.

## Inputs (read these first)

Read in this order before writing any code. Do not skim.

1. `outputs/README.md`
2. `../_deep/03-TAD-technical-architecture.md` — sections 1, 2, 3, 4, 5, 22.
3. `../_deep/06-security-privacy-compliance.md` — sections 4.5 (signing) and 9 (supply-chain).
4. `../_deep/11-test-and-validation-strategy.md` — sections 3 and 7.
5. `outputs/prompts/README.md` — the universal guard-rails (G1–G18).

## Deliverables

### 1. Workspace layout

Initialise the layout described in TAD §4 *exactly*:

```
booksforge/
├── apps/desktop/
│   ├── src/                    # Tauri Rust host (one Tauri command, one event)
│   ├── src-ui/                 # React + TS frontend (Vite)
│   ├── tauri.conf.json
│   └── Cargo.toml
├── crates/
│   ├── booksforge-domain/        (cargo new --lib, with placeholder type)
│   ├── booksforge-ipc/           (cargo new --lib, with one IPC type pair)
│   ├── booksforge-storage/       (cargo new --lib, empty for now)
│   ├── booksforge-fs/            (cargo new --lib, empty)
│   ├── booksforge-template/      (empty)
│   ├── booksforge-validator/     (empty)
│   ├── booksforge-export/        (empty)
│   ├── booksforge-ai/            (empty)
│   ├── booksforge-ai-runtime/    (empty)
│   ├── booksforge-export-pandoc/ (empty)
│   ├── booksforge-plugin-host/   (empty)
│   └── booksforge-test-fixtures/ (empty)
├── packages/
│   ├── ui/                      (pnpm workspace package, empty)
│   ├── editor/                  (empty)
│   ├── plugin-sdk/              (empty)
│   └── shared-types/            (generated TS types target dir)
├── plugins/                     (empty; first-party plugin source later)
├── docs/                        (mdBook scaffold; not built yet)
├── tools/                       (codegen, migration scripts; empty)
└── .github/workflows/           (CI; see below)
```

### 2. Cargo workspace

Top-level `Cargo.toml` declares all crates as workspace members. Set `resolver = "2"`. Pin `rust-toolchain.toml` to a specific stable.

### 3. pnpm workspace

`pnpm-workspace.yaml` registers `apps/desktop/src-ui`, `packages/*`, and `plugins/*`. Pin `package.json` `engines.node` and add `engines.pnpm`.

### 4. Tauri v2 app

Initialise `apps/desktop` with Tauri v2 stable. The app must:

- Open one window titled "BooksForge — Hello, manuscript".
- Expose one Tauri command `app.echo(input: { text: string }) -> { text: string, server_received_at: string (ISO-8601 UTC) }`.
- The command lives in `apps/desktop/src/commands/app.rs` and the input/output types are imported from `booksforge-ipc`.
- Frontend calls the command via the typed client at app start and renders the round-tripped string.

### 5. `booksforge-ipc` typed IPC

In `crates/booksforge-ipc/src/lib.rs`, define `EchoRequest`, `EchoResponse`, and `BooksForgeError` with the categories listed in TAD §15. Use `serde` + `ts-rs`. A build script (or `tools/codegen-ts.sh`) regenerates TypeScript into `packages/shared-types/src/generated/`. Run codegen and commit the result.

### 6. Frontend scaffold

Vite + React 19 + TypeScript. State via `zustand` + `immer`. Routing not needed yet. Render a single page with a "Hello" message and an echo round-trip indicator. Tailwind not required yet — keep dependencies minimal.

### 7. Lints, formatters, hooks

- Rust: `clippy` configured with `-D warnings` for the workspace. `rustfmt.toml` with team style.
- TS: ESLint + Prettier; flat-config; strict TS.
- Pre-commit hooks via `lefthook` or `husky`: format, lint, typecheck on staged files.
- A `cargo deny` config forbidding GPL/AGPL/LGPL crates as static deps.

### 8. CI workflows

`.github/workflows/ci.yml` runs on PR and push:

- Matrix: `ubuntu-22.04`, `macos-13`, `macos-14`, `windows-2022`.
- Steps per OS: install toolchains, cache, `cargo fmt --check`, `cargo clippy -- -D warnings`, `pnpm i`, `pnpm typecheck`, `pnpm lint`, `cargo test`, `pnpm test`, `cargo deny check`, `pnpm tauri build --debug` (smoke).
- Artifact upload: debug build per OS retained 14 days.

`.github/workflows/release.yml` (skeleton, not active yet) for signed release builds.

### 9. Documentation

- `README.md` (top-level): one paragraph plus "How to develop" with the `pnpm tauri dev` instructions.
- `CONTRIBUTING.md`: code style, commit message convention (Conventional Commits), how to run tests.
- `docs/open-questions.md`: empty file ready for guard-rail-driven questions.
- `docs/runbooks/signing.md`: skeleton with the steps for procuring and integrating Microsoft EV, Apple Developer ID, and Linux signing.

### 10. License and governance

- `LICENSE` — pick a permissive license (MIT / Apache-2.0 dual-license is conventional). **Do not** pick GPL.
- `CODE_OF_CONDUCT.md`.
- `SECURITY.md` with reporting email and PGP key placeholder.
- `CODEOWNERS` listing the tech lead for `crates/`, frontend lead for `apps/desktop/src-ui` and `packages/`.

## Guard-rails specific to this phase

In addition to the universal G1–G18:

**[GUARD-P0-1]** Do not write any business logic in this phase. Crates beyond `booksforge-ipc` and `booksforge-domain` (placeholder type only) are *empty stubs*. The point of Phase 00 is the runway, not the application.

**[GUARD-P0-2]** Pin Tauri v2 to an exact released version. Do not float to `latest`.

**[GUARD-P0-3]** Do not install dependencies you don't use. Vite, React, ts-rs, zustand, immer — that's the frontend dep list. Nothing else.

**[GUARD-P0-4]** The `app.echo` command is the only IPC. Resist the temptation to add commands you'll need later — they belong to Phase 01.

## Acceptance criteria (must all pass)

1. `pnpm tauri dev` opens the window on macOS, Windows, and Linux.
2. The window shows the round-tripped echo string within 1 second of launch.
3. `pnpm typecheck` passes with zero errors.
4. `cargo clippy --all-targets -- -D warnings` passes with zero warnings.
5. `cargo test --workspace` passes (will be near-empty but green).
6. `cargo deny check` passes.
7. `pnpm tauri build` produces a packaged artifact on each OS in CI.
8. Generated TS types exist in `packages/shared-types/src/generated/` and are referenced from the frontend.
9. Pre-commit hooks run on a `git commit` and reject a deliberately bad style.
10. CI is green on `main` after merging this PR.

## Review gate (human inspection)

The tech lead checks:

- The directory layout is **exactly** the one in TAD §4. No deviations.
- No GPL deps anywhere (verify `cargo deny`).
- Tauri config does not enable any allowlist beyond what's needed for window mgmt.
- The IPC echo round-trips and the type is shared, not duplicated.
- README is accurate.
- Signing runbook draft is in `docs/runbooks/signing.md` even if procurement isn't done.

## Out of scope (do not do these in Phase 00)

- Project bundle creation, SQLite, file-system adapters.
- Editor scaffolding.
- AI runtime.
- Any business logic.
- Any UI beyond the placeholder hello screen.
- Any installer signing (procurement is human, integration is later).

## When you finish

Open the PR with title `Phase 00: Bootstrap`. In the body, list deliverables 1–10 with check marks. Ping the tech lead for review-gate sign-off. After merge, update `outputs/prompts/STATUS.md` to mark Phase 00 complete and Phase 01 ready.
