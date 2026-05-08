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
- [ ] **New React component?** add an `axe` test in `src/a11y.test.tsx` (see "Accessibility testing" below)
- [ ] **New panel / dialog?** uses `useDialogA11y()` for focus management + `aria-labelledby` + `role="dialog"` (or `alertdialog` for errors)

## Accessibility testing

Every new React component should be exercised by `axe-core` in
`apps/desktop/src-ui/src/a11y.test.tsx`.  The setup file
(`src/test-setup.ts`) registers the `toHaveNoViolations` matcher
globally so any vitest test can:

```typescript
import { axe } from "vitest-axe";

it("MyComponent is accessible", async () => {
  const { container } = render(<MyComponent />);
  const result = await axe(container);
  expect(result).toHaveNoViolations();
});
```

`axe` automatically detects ~57 % of WCAG 2.2 violations.  The
remaining issues require manual / AT testing on real hardware
(VoiceOver, NVDA, JAWS); that is a pre-release task in
[`MILESTONES.md`](MILESTONES.md) M6 §I1, not a per-PR gate.

Running `pnpm test` runs the a11y suite alongside everything else.
A failed `toHaveNoViolations` prints the rule, the offending
selector, and the recommended fix — *don't suppress violations*,
fix the component or file an issue.

## Running a single test

```bash
# Rust
cargo test -p booksforge-domain entity_matches_name

# TypeScript
cd booksforge && pnpm --filter @booksforge/shared-types typecheck
```

## Pre-commit / pre-push hooks (audit #59)

We use [lefthook](https://github.com/evilmartians/lefthook) to run the
cheapest CI gates locally before they reach CI.  The config lives at
`booksforge/lefthook.yml`.

```bash
cd booksforge
pnpm install        # `prepare` script auto-installs the hooks
# or, manually:
pnpm dlx lefthook install
```

Hooks run:

| Stage      | Command                                        | When it fires |
|------------|------------------------------------------------|---------------|
| pre-commit | `cargo fmt --all -- --check`                   | any `*.rs` staged |
| pre-commit | `pnpm -r typecheck`                            | any `*.ts*` staged |
| pre-commit | `pnpm -r lint`                                 | any `*.ts*` staged |
| pre-push   | `cargo clippy --workspace --all-targets -- -D warnings` | every push |
| pre-push   | `cargo test --workspace --lib`                 | every push |

Bypass with `LEFTHOOK=0 git commit ...` for genuine WIP work — the
bypass is loud in the diff so accidents are easy to spot.

## Frontend bundle-size reports (audit #52)

The Vite build emits a treemap to `apps/desktop/src-ui/dist/bundle-report.html`
when `BOOKSFORGE_BUNDLE_REPORT=1` is set or when running under CI.
Local dev builds skip the visualiser to keep HMR cheap.  Open the HTML
file in any browser to see gzipped/brotli-compressed module sizes.

```bash
BOOKSFORGE_BUNDLE_REPORT=1 pnpm --filter @booksforge/desktop-ui build
open booksforge/apps/desktop/src-ui/dist/bundle-report.html
```

## Dependency-pin policy

| Layer | Pin specificity | Update path |
|-------|-----------------|-------------|
| Rust workspace deps in `booksforge/Cargo.toml` | Pin to **minor** (e.g. `= "1.5"`, never `= "1"`). | Patch: Dependabot auto-PR, auto-merge if CI green. Minor: Dependabot PR, manual review. Major: human-authored PR with justification + spec / ADR update if it crosses the layer boundary. |
| `tauri`, `sqlx`, `react`, `react-dom`, `vite`, `@tiptap/core` | Major bumps explicitly **ignored** by Dependabot — they are deliberate human decisions. | Open a tracking issue, then a feature branch named `feat/upgrade-<dep>-<major>`. Update the spec entry in `outputs/TOOLCHAIN.md` in the same PR. |
| `pnpm-lock.yaml` | Always committed. CI runs `pnpm install --frozen-lockfile`. | Re-run `pnpm install` locally, commit the lockfile change in the same commit as the `package.json` change. |
| `Cargo.lock` | Always committed (binary workspace). | Touch only when a `Cargo.toml` dep changes; never edit by hand. |

Dependabot configuration lives at `.github/dependabot.yml`. CODEOWNERS
review is required for any change to that file.

## Privacy invariants — "before you ship"

These are checked in CI by `cargo test -p booksforge-orchestrator --test
privacy_invariants`. Before opening a PR that touches networking, IPC,
agent dispatch, or settings, sanity-check yourself:

- [ ] No outbound network call at app startup. Only `OllamaSetup → Install`,
      `Ollama.pull`, and the opt-out `Update.check` may make outbound calls.
- [ ] No manuscript content ever reaches a remote endpoint. Scene text,
      outlines, memory entries, and vocabulary entries all stay local.
- [ ] Ollama traffic stays on `127.0.0.1`. A non-loopback host requires
      an explicit user-facing consent dialog (`OllamaSetup`).
- [ ] AI capability is **off per project** until the user enables it
      with an explicit one-time consent prompt. Default state on parse
      failure is "off", not "on".
- [ ] No GPL / AGPL / LGPL crate is statically linked. `cargo deny`
      enforces; Pandoc and EPUBCheck run as external sidecar
      processes, not linked in.

## Reporting security issues

See `SECURITY.md`. Do **not** open a public GitHub issue for security
or privacy bugs.
