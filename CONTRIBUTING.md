# Contributing to BooksForge

## Branch convention

```
main              ← always releasable
milestone/mz-XX   ← milestone branch, squash-merged into main
feat/<ticket>     ← feature branches, rebased onto milestone branch
fix/<ticket>      ← bug fixes
```

## Before you code

1. Read `CLAUDE.md` — it is the coding contract.
2. Check `outputs/CONSISTENCY_MATRIX.md` for known open questions.
3. All decisions that affect public API, data model, or IPC surface must be recorded in `docs/open-questions.md` (or resolved there) before implementation begins.

## Commit style

```
<type>(<scope>): <short imperative summary>

Body explaining WHY, not WHAT.  Reference the spec doc if applicable.
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`

## PR checklist

- [ ] `cargo fmt --all` — no diff
- [ ] `cargo clippy --workspace -- -D warnings` — clean
- [ ] `cargo test --workspace` — green
- [ ] `cargo test -p booksforge-ipc` run if IPC types changed; bindings committed
- [ ] `cargo deny check` — clean
- [ ] New public Rust types have `#[derive(Debug, Clone, Serialize, Deserialize)]`
- [ ] No `unwrap()` / `expect()` in non-test code
- [ ] Layer boundaries respected: L3 crates have no L4 imports in `Cargo.toml`
- [ ] Privacy invariant: Ollama calls only go to `127.0.0.1:11434`

## Running a single test

```bash
# Rust
cargo test -p booksforge-domain entity_matches_name

# TypeScript
cd booksforge && pnpm --filter @booksforge/shared-types typecheck
```
