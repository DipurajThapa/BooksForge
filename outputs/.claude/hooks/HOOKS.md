# Claude Code Hooks — BooksForge

This file is the declarative list of hooks Claude Code respects in this repository. The full spec (procedure, inputs, outputs, failure behaviour, performance budgets) is in `outputs/CLAUDE_CODE_HOOKS_SPEC.md`.

The `.git/hooks/` scripts implement hooks 4, 5, 7, 8, 10. The hooks below are declarative and used by Claude Code at edit/read time.

## Active hooks (Claude Code-side)

### 1. `pre-edit-docs-consistency`

Trigger: before editing any `.md` in `outputs/`.
Action: scan for stale BooksForge name variants and verify the file is in `DOCS_INVENTORY.md`.
Failure: warning (non-blocking) by default; blocking if `BOOKSFORGE_HOOK_STRICT=1`.

### 2. `post-edit-markdown-lint`

Trigger: after editing any `.md`.
Action: run `markdownlint-cli2`; resolve broken links; check heading hierarchy.
Failure: blocking. Fix or skip with explicit reason.

### 3. `post-edit-booksforge-naming`

Trigger: after any file edit.
Action: grep for `Bookforge|BookForge|Book Forge` and `\bbookforge[A-Z]`.
Failure: blocking. Correct before edit finalises.

### 6. `post-export-epub-validation`

Trigger: after any export completes (dev, test, release).
Action: run `epubcheck`; render preview vs. EPUB content with same WebView; pixel-diff.
Failure: blocking in CI; surfacing in dev.

### 9. `pre-edit-large-file-token-guard`

Trigger: before reading a file >300 lines.
Action: warn if file is on the "do not read by default" list in `CLAUDE_CODE_CONTEXT_HARNESS.md §14`. After 3 large reads in one session, propose updating the harness.
Failure: non-blocking; awareness only.

## Git-side hooks (referenced for completeness)

These run from `.git/hooks/` — not from `.claude/`. Documented here so Claude Code knows they exist.

- `pre-commit-test-runner` (4)
- `pre-commit-typecheck-lint` (5)
- `pre-commit-prompt-library-schema` (7)
- `pre-commit-diagram-reference-validation` (8)
- `pre-commit-secret-privacy-guard` (10)

## Override

Use `git commit --no-verify` only with a written reason in the commit message. CI re-runs every blocking hook regardless. The privacy-guard hook (10) cannot be bypassed.

## Performance budgets

See `CLAUDE_CODE_HOOKS_SPEC.md` performance-budgets section. If a hook exceeds its budget, the user is warned but the hook is not bypassed.
