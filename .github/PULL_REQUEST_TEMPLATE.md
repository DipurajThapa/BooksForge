<!--
Thanks for opening a PR.  Please fill in the sections below.
-->

## Summary

<!-- 1-3 sentences. WHAT changes and WHY. -->

## Linked

<!-- Issue numbers, audit-backlog items, milestone tasks. -->

- Closes #
- Refs `EXTERNAL_AUDIT_BACKLOG.md` #
- `MILESTONES.md` row:

## Changes

<!-- Bulleted list. Group by file or by concern. -->

-

## CI checklist

> Adapted from `CONTRIBUTING.md`.  Tick what you have run locally.
> CI will run them all again on this PR.

- [ ] `cd booksforge && cargo fmt --all -- --check` — clean
- [ ] `cd booksforge && cargo clippy --workspace --all-targets -- -D warnings` — clean
- [ ] `cd booksforge && cargo test --workspace --all-targets` — green
- [ ] `cd booksforge && cargo deny check licenses bans advisories sources` — clean
- [ ] `cd booksforge && cargo test -p booksforge-orchestrator --test privacy_invariants` — green (if anything privacy-touching changed)
- [ ] `cd booksforge && cargo test -p booksforge-ipc` — green (if any IPC type changed; bindings committed)
- [ ] `cd booksforge && pnpm typecheck && pnpm lint && pnpm test` — green

## Privacy invariants — before you ship

> If this PR touches networking, IPC, agent dispatch, settings, or
> Tauri capabilities, sanity-check yourself.  Skip if N/A.

- [ ] No new outbound network call at app startup.
- [ ] No path that sends manuscript content to a remote endpoint.
- [ ] Ollama traffic still pinned to `127.0.0.1` by default; non-loopback hosts still require explicit consent.
- [ ] AI features still off-by-default per project.
- [ ] No GPL/AGPL/LGPL crate added (`cargo deny check licenses` will fail otherwise).
- [ ] Tauri capabilities not widened without justification (CODEOWNERS will block this).

## Layer-boundary check (if Rust changed)

- [ ] L3 crates (`domain`, `template`, `validator`, `agents`, `prompt`, `memory`, `vocab`, `export`) have **no** L4 imports in `Cargo.toml` (`storage`, `fs`, `ollama`, `orchestrator`, `export-epub`, `export-pandoc`, `epubcheck`).
- [ ] No production `unwrap()` / `expect()` / `panic!()` introduced.
- [ ] All SQL goes through `sqlx::query!` / `query_as!` (compile-time checked).
- [ ] New migrations under `crates/booksforge-storage/migrations/` are *additive* (forward-only).

## Risks & rollback

<!-- What might break? How would we revert? -->

-

## `[ASSUMED]` notes

<!-- Per CLAUDE.md: any undocumented choice you made.  Mark them in
     code as well as here. -->

-

## Screenshots / recordings (UI changes only)

<!-- For React / TipTap / panel changes.  Drag-drop here. -->

-
