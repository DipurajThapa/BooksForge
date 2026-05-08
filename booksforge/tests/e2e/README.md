# BooksForge — End-to-End Test Suite (Playwright)

> Golden-path E2E for the desktop app.  Closes part of
> EXTERNAL_AUDIT_BACKLOG.md #22.  Distinct from the
> *visual-regression* suite next door (`tests/visual-regression/`)
> which only checks rendering parity between the in-app preview
> and the unzipped EPUB content HTML.
>
> This suite drives the **whole user journey** — open project →
> create scene → autosave → snapshot → restore → run an agent →
> apply proposals → export.  It runs against a real Tauri build,
> which means it's slower than vitest unit tests but exercises
> code paths nothing else can.

---

## Status: SCAFFOLDED

The harness, fixtures, and one (`#test.skip`) skeleton are
committed.  Activation requires:

1. A Tauri-friendly E2E driver (we're targeting **WebDriver via
   `tauri-driver`** which Tauri 2 supports natively).
2. A "test mode" boot for the app that uses a temp `~/.booksforge/`
   so concurrent tests don't trample one another's settings.
3. CI matrix entry: `release.yml` already runs visual-regression
   on `macos-14`; this E2E suite belongs alongside it but is
   currently un-wired pending implementation of items 1–2.

Until then, every test in this folder uses `test.skip` so the
suite is harmless to the gating CI.

---

## Layout

```
tests/e2e/
├── README.md          (this file)
├── package.json       (Playwright deps; pinned to match the
│                       visual-regression suite)
├── playwright.config.ts
├── tsconfig.json
├── fixtures/
│   └── small-fiction-project.json   (a known-good test project bundle
│                                       seed; reused across specs)
└── src/
    ├── golden-path.spec.ts          (the headline 7-journey walk)
    ├── snapshot-restore.spec.ts     (snapshot + per-node restore)
    └── agent-dispatch.spec.ts       (Copyedit + Continuity end-to-end)
```

---

## Running

```bash
cd booksforge/tests/e2e
pnpm install
pnpm playwright install chromium

# Headless (CI)
pnpm test

# Headed (debugging)
pnpm test:headed

# Specific spec
pnpm test src/golden-path.spec.ts
```

---

## Adding a new spec

1. Drop the file in `src/<feature>.spec.ts`.
2. Use `test.skip()` until the harness is ready.
3. When activating, switch `test.skip` → `test()` and add a
   one-line entry to the catalogue table below.

---

## Catalogue

| Spec | Status | Coverage |
|------|:------:|----------|
| `golden-path.spec.ts` | ⏸ scaffolded | open project → edit scene → autosave → snapshot → export |
| `snapshot-restore.spec.ts` | ⏸ scaffolded | full + per-node selective restore (closes audit #31 once activated) |
| `agent-dispatch.spec.ts` | ⏸ scaffolded | Copyedit dispatch + ProposalReview accept/reject + apply (closes audit #29 in the integration sense) |

---

*Refs: `tests/visual-regression/README.md` (sibling suite),
`outputs/MVP_SCOPE.md §6` (the 7 user journeys this suite walks),
`MILESTONES.md M6.J`.*
