# UI archive — 2026-05-11

Snapshot of `apps/desktop/src-ui/` as it stood the moment we paused to
fix the apply_outline duplicate-root bug + decide whether to rewrite
the UI.

## What's preserved

- 129 files, 1.1 MB (no `node_modules`, no `.vite/`, no `dist/`).
- 56 vitest test files (181 passing tests at backup time).
- All components shipped through 2026-05-11:
  - The 4-element redesigned toolbar (mode pill + primary CTA + ⋯ + Close)
  - `ModePicker.tsx`, `MoreMenu.tsx`, `BiblesPanel.tsx`, `BookGenerationPanel.tsx`
  - `NewProjectWizard.tsx` (with installed-models dropdown + spinner)
  - `BriefEditorPanel.tsx` (with provenance UX)
  - `ProjectPicker.tsx` (with Remove + hover affordance)
  - `EditorShell.tsx` (with mode-aware empty hero + ⋯ menu)
  - TipTap editor wiring (`@booksforge/editor` package — separate)
  - All quality / agents / snapshots / export / publishing panels

## How to restore

```sh
# Undo the archive (overwrite live src-ui from this snapshot):
rsync -a apps/desktop/src-ui-archive-2026-05-11/ apps/desktop/src-ui/

# Or pull a single file back:
cp apps/desktop/src-ui-archive-2026-05-11/src/components/BiblesPanel.tsx \
   apps/desktop/src-ui/src/components/
```

This directory is intentionally outside the pnpm workspace glob
(`packages: - "apps/desktop/src-ui"` only — sibling dirs aren't
matched), so it doesn't participate in builds or tests.

## Why the snapshot was taken

User accumulated multiple UI ↔ backend mismatches:
- Wizard collected brief but didn't persist it (fixed pre-snapshot)
- `apply_outline` creates a 2nd project root instead of merging
  (open at snapshot time)
- Generic Novel template + outline-architect both seed nodes →
  duplicate trees with 45 scenes total (open at snapshot time)
- Pipeline used non-chunked character-bible (fixed pre-snapshot)
- 14-button toolbar with no hierarchy (redesigned pre-snapshot)

Decision was: backup current UI as a safe rollback target, fix the
backend bugs first, then choose whether to keep evolving the current
UI or build a fresh one. This snapshot is the rollback target.
