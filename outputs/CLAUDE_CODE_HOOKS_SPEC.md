# Claude Code Hooks — Spec

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Defines the ten hooks BooksForge uses for reliable development.** Companion to `CLAUDE_CODE_SKILLS_SPEC.md`, `CLAUDE_CODE_SUBAGENTS_SPEC.md`.

A "hook" is an event-triggered automation. We use hooks to catch problems **before** they land — naming drift, lint failures, doc inconsistency, accidental secrets, slow tests, and ePUB QA regressions. Hooks complement CI gates but fire **earlier** in the loop, ideally on every save or every commit.

---

## How hooks are wired

For BooksForge:

- **Pre-commit hooks** are in `.git/hooks/` (installed by `make hooks` or the bootstrap script).
- **Pre-edit and post-edit hooks** for Claude Code live under `.claude/hooks/HOOKS.md`.
- **CI gates** live in `.github/workflows/`.

This spec defines each hook's name, trigger, command intent, inputs, outputs, and failure behaviour. The actual scripts ship with the repo at MZ-01.

---

## 1. `pre-edit-docs-consistency`

**Trigger.** Before Claude Code edits a `.md` file in `outputs/` or any spec doc.

**Intent.** Quick scan to flag obvious problems before the edit lands.

**Inputs.** The file about to be edited; the staged diff.

**Procedure.**

1. Run `grep -E 'Bookforge|BookForge|Book Forge'` on the file. Any match is a pre-edit warning (the rename should be settled).
2. Verify the file is in `DOCS_INVENTORY.md`. If not, prompt to add a row.
3. If the file has a "Status note" superseding it, remind Claude Code to either preserve the note or write a justification for removing it.

**Outputs.** A short report. If failures, the edit is paused and Claude Code is asked to confirm.

**Failure behaviour.** Non-blocking by default — Claude Code can proceed with awareness. Set `BOOKSFORGE_HOOK_STRICT=1` to make it blocking.

---

## 2. `post-edit-markdown-lint`

**Trigger.** After Claude Code writes or edits a `.md` file.

**Intent.** Catch malformed Markdown (broken tables, unclosed code blocks, trailing whitespace, inconsistent heading levels).

**Procedure.**

1. Run `markdownlint-cli2` (or `mdl`) against the file with the project's `.markdownlint.json` config.
2. Run a custom check: every `[link](path)` in the doc resolves to an actual file on disk.
3. Run a heading-level check: no skips (h1 → h3 missing h2).
4. Run a TOC check: if the file has a TOC, it matches the headings.

**Outputs.** Either PASS or a list of findings.

**Failure behaviour.** Blocking. Claude Code must fix or explicitly skip with a one-line reason.

---

## 3. `post-edit-booksforge-naming`

**Trigger.** After any file edit (any extension).

**Intent.** Enforce the `BooksForge` / `booksforge` rename — catch accidental introduction of stale variants.

**Procedure.**

1. Run `grep -E 'Bookforge|BookForge|Book Forge'` on the changed file.
2. Run `grep -nE '\bbookforge[A-Z]'` (lowercase followed by capital — likely a typo of the new name).

**Outputs.** PASS or list of offending lines.

**Failure behaviour.** Blocking. Claude Code corrects before the edit is finalised.

---

## 4. `pre-commit-test-runner`

**Trigger.** Before `git commit`.

**Intent.** Run fast tests so failures surface before the PR.

**Procedure.**

1. `cargo test --workspace --no-fail-fast` for changed Rust crates.
2. `pnpm vitest run --changed` for changed TS files.
3. If `prompts/` changed, run prompt-render snapshot tests for the affected agents.
4. If `EXPORT_EPUB_SPEC.md` or `EXPORT_EPUB_QA.md` or `booksforge-export*` changed, run the medium-fixture ePUB QA suite (S1–S8 + EPUBCheck).

**Outputs.** Test pass/fail.

**Failure behaviour.** Blocking. The commit is rejected until tests pass.

**Performance budget.** Total runtime ≤ 60 s for a typical change.

---

## 5. `pre-commit-typecheck-lint`

**Trigger.** Before `git commit`.

**Intent.** Catch lint and type errors.

**Procedure.**

1. `cargo fmt --check`.
2. `cargo clippy --all-targets -- -D warnings`.
3. `cargo deny check licenses` and `cargo deny check bans` (layered-imports).
4. `pnpm typecheck` for changed packages.
5. `pnpm lint` for changed packages.

**Outputs.** Pass/fail.

**Failure behaviour.** Blocking.

---

## 6. `post-export-epub-validation`

**Trigger.** After any export completes (during dev, test, or release).

**Intent.** Run EPUBCheck against the produced EPUB and visual regression against the in-app preview.

**Procedure.**

1. Run `epubcheck` on the EPUB; parse JSON output.
2. If errors, surface and refuse the export (in dev) or fail the test (in CI).
3. Render the editor preview and the unzipped EPUB content with the same WebView; compare via `pixelmatch`.
4. Pixel diff above tolerance fails.

**Outputs.** Validation report + visual diff image (on failure).

**Failure behaviour.** Blocking for CI. In dev, surfaces the issue but allows the developer to inspect the artifact.

---

## 7. `pre-commit-prompt-library-schema`

**Trigger.** Before `git commit` if `prompts/` or `templates/prompts/` changed.

**Intent.** Verify every prompt declares the required metadata (purpose, agent, inputs, outputs, memory reads, memory writes, failure handling, validation checklist, example output).

**Procedure.**

1. Parse every changed `*.prompt.toml` or `prompt-*.md`.
2. Verify the required sections are present.
3. Verify the `[render.json_schema]` ref points at a real schema in `booksforge-agents`.
4. Verify the prompt is hash-pinned (the file's blake3 matches the `prompt_template_hash` in tests).

**Outputs.** Pass/fail per file.

**Failure behaviour.** Blocking.

---

## 8. `pre-commit-diagram-reference-validation`

**Trigger.** Before `git commit` if any `.md` file mentions a diagram.

**Intent.** Ensure `diagrams/*.svg` references in prose actually point at existing files; ensure the `diagrams/README.md` index lists every SVG.

**Procedure.**

1. Grep for `diagrams/[^)]+` in the changed `.md` files.
2. Verify every reference is a real path under `outputs/diagrams/`.
3. Verify `diagrams/README.md` lists every SVG file under `outputs/diagrams/`.

**Outputs.** Pass/fail.

**Failure behaviour.** Blocking on dangling references; warning on README drift.

---

## 9. `pre-edit-large-file-token-guard`

**Trigger.** Before Claude Code reads a file that would consume substantial context.

**Intent.** Prevent unnecessary token use by encouraging the harness path. Files >300 lines that aren't on the "always read" list get a soft warning.

**Procedure.**

1. Compute the file's line count.
2. If >300 lines AND the file is in the "do not read by default" list in `CLAUDE_CODE_CONTEXT_HARNESS.md §14`, emit a warning naming the harness as the alternative.
3. If the file is on the "read first" list, proceed silently.
4. If reading >3 large deep specs in one session, ask Claude Code whether to update the harness instead.

**Outputs.** A warning or silent proceed.

**Failure behaviour.** Non-blocking. Awareness only.

---

## 10. `pre-commit-secret-privacy-guard`

**Trigger.** Before `git commit`.

**Intent.** Catch accidental commits of secrets, API keys, or private fixtures.

**Procedure.**

1. Run `gitleaks` (or equivalent) over the staged diff.
2. Run a custom regex: AWS keys, private RSA keys, Anthropic / OpenAI / OpenRouter API keys, `.env` contents.
3. Reject commits that include any file under `private/`, `secrets/`, `.env*`, `*.key`, `*.pem`.
4. Reject any file that looks like a manuscript fixture not in `crates/booksforge-test-fixtures/`.

**Outputs.** Pass/fail per match.

**Failure behaviour.** Blocking. The user must explicitly add a `.gitignore` or remove the file.

---

## Hook lifecycle and overrides

- Hooks are installed by `make hooks` at MZ-01.
- A hook can be skipped for a single commit with `git commit --no-verify` and a reason in the commit message — but `pre-commit-secret-privacy-guard` cannot be skipped (it re-runs in CI).
- CI re-runs every blocking hook so a `--no-verify` cannot bypass the gate.

## Performance budgets

| Hook | Budget |
|------|--------|
| `post-edit-markdown-lint` | ≤ 2 s per file |
| `post-edit-booksforge-naming` | ≤ 1 s per file |
| `pre-commit-test-runner` (fast path) | ≤ 60 s |
| `pre-commit-typecheck-lint` | ≤ 30 s |
| `post-export-epub-validation` | ≤ 10 s |
| `pre-commit-prompt-library-schema` | ≤ 5 s |
| `pre-commit-secret-privacy-guard` | ≤ 5 s |

If a hook exceeds its budget, the user is warned but the hook is not bypassed.

## Where the actual hooks live

- `.git/hooks/pre-commit` — runs hooks 4, 5, 7, 8, 10.
- `.git/hooks/post-merge` — re-installs hooks if `make hooks` script changed.
- `.claude/hooks/HOOKS.md` — declarative list for Claude Code (hooks 1, 2, 3, 6, 9).
- `.github/workflows/ci.yml` — re-runs every blocking hook.

## What hooks do not do

- They do not replace human review.
- They do not enforce "good taste" — only mechanical correctness.
- They do not run agents.
- They do not block on warnings; only on errors.
- They do not write code on the user's behalf.
