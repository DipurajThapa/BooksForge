# Phase 02 — Editor core

> **Status note (2026-05-06):** This phase prompt is **superseded** by Milestone 1 (M1) in `IMPLEMENTATION_PLAN.md`. The procedures below are preserved for historical context; use the implementation pack for the MVP build.

## Goal

Replace the placeholder textarea with a production-grade TipTap editor that meets the editor performance budgets and supports the FSD §2 baseline: rich text, document tree (binder/outline/corkboard), drag-reorder, find/replace, footnotes (placeholder rendering until Phase 06 wires the citation engine), word count, focus modes, basic snapshot UI.

## Pre-conditions

- Phase 01 merged. CI green. Performance probe passing.
- A textarea-bound scene flow is functional and will be replaced.

## Inputs

1. `../_deep/02-FSD-functional-specifications.md` — section 2 (FR-EDIT-001 through FR-EDIT-022) and section 7 (outline/corkboard).
2. `../_deep/03-TAD-technical-architecture.md` — section 6 (editor framework decision and §6.4 performance plan).
3. `../_deep/05-workflow-and-dataflow.md` — section 3 (edit loop hot path).
4. `../_deep/04-data-model-and-project-format.md` — section 4 (`scene_content`, `nodes`).
5. `../_deep/11-test-and-validation-strategy.md` — section 6.2.

## Deliverables

### 1. `packages/editor`

A React package wrapping TipTap with the BooksForge schema.

Schema includes: paragraph, heading (1–6), blockquote, codeBlock (with language hint), list (bullet/ordered/checklist), table, horizontalRule, image (with caption, alt text), figure, footnote (anchor + content), endnote, citation (anchor referencing `references` table), crossRef, comment (range mark), trackedChange (insert / delete / format marks — phase-06 fully wires it; mark types declared now).

Marks: bold, italic, underline, strike, code, sub, sup, highlight, link.

Custom node views: figure, footnote, equation (placeholder), citation (placeholder until Phase 06).

### 2. Per-chapter editor instances + virtualisation

Implement TAD §6.4: each chapter gets its own `EditorView`. The chapter list is virtualised via `react-virtual`. Only the viewport chapters mount editor instances; off-screen chapters render a static skeleton until scrolled into proximity. State for unmounted chapters lives in SQLite — we don't keep all of them in memory.

Cursor and selection meta synced into Zustand for the status bar; the document state stays in ProseMirror.

### 3. Document tree (binder)

Left sidebar lists Project → Part → Chapter → Scene with collapse/expand. Drag-reorder using `@dnd-kit/core` with the LexoRank-style position update (call `node.move`).

### 4. Outline view + corkboard

Right-side panel switchable to Outline (hierarchical list with synopsis/status/POV columns) or Corkboard (drag-arrangeable cards) — both read/write the same underlying scene metadata (`nodes` table fields `status`, `pov`, `beat`, `target_words` and a synopsis stored on `notes` keyed `synopsis`).

### 5. Find/replace

Project-wide find with regex toggle. Replace one / replace all with confirmation. Scope: selection, scene, chapter, project. Implementation note: scene-level find can search the loaded ProseMirror doc; project-wide find queries the SQLite `scene_content` table over a Markdown-mirror-derived index for speed (or, simpler in MVP: scan all scenes — accept the perf cost and revisit if needed).

### 6. Word count + status bar

Word/char counts at selection, scene, chapter, project. Today-session word count. ICU break iterator for non-space-separated languages (CJK).

### 7. Spell-check

Hunspell via Rust binding; bundled US-English dictionary; user dictionary stored in app data dir. Other languages ship as optional dictionary downloads.

### 8. Snapshot UI (manual only in this phase)

A "Take snapshot" button on any node opens a label dialog and persists into `snapshots` table. A "Snapshots" panel lists snapshots with diff-vs-current preview and a "Restore" action. Storage layer for snapshot objects lands in `booksforge-fs` (content-addressed `snapshots/objects/`).

### 9. Tests

- Unit: ProseMirror schema serialise/deserialise round-trips for every node and mark type (property test with `proptest`-generated documents).
- Integration: `scene.save` from the editor reaches storage; reopen reproduces document state byte-identically.
- E2E: open a 100k-word fixture project; type at the end of a chapter; assert keystroke-latency p95 ≤ 30 ms (Playwright + DevTools profile probe).
- Performance: cold-open of 200k-word fixture ≤ 1.5 s p50 on reference Mac (criterion benchmark).
- Visual: snapshot of editor with footnote, image, table.
- Accessibility: axe pass on editor surface; keyboard-only test traverses tree, edits, saves.

### 10. Documentation

- `docs/editor/architecture.md` explaining per-chapter EditorViews and virtualisation.
- In-app help: "Editor basics", "Find and replace", "Outline and corkboard", "Snapshots".

## Guard-rails specific to this phase

**[GUARD-P2-1]** Do not load all scenes into memory. The number of mounted EditorViews is bounded by the viewport.

**[GUARD-P2-2]** No double-source for document state. ProseMirror is the source of truth in the editor; Zustand mirrors only what the UI needs.

**[GUARD-P2-3]** Drag-reorder updates persist transactionally and update Markdown mirror.

**[GUARD-P2-4]** Citation/footnote/equation placeholders render but are non-functional until Phase 06. They store payload faithfully so Phase 06 implementations can interpret them — a `citation` node serialised in Phase 02 and read in Phase 06 must round-trip.

**[GUARD-P2-5]** Editor performance budgets are CI-enforced this phase. A regression > 10% blocks merge.

## Acceptance criteria

1. Open a 200k-word fixture in p50 ≤ 1.5 s on reference Mac.
2. Keystroke latency p95 ≤ 30 ms on a 50k-word chapter.
3. Drag a scene from chapter 3 to chapter 5; references persist; undo restores; save acknowledged.
4. Find/replace with regex over a 50k-word project completes in ≤ 2 s.
5. Manual snapshot of a chapter, restore from snapshot, content matches.
6. Outline drag-reorder persists same as binder.
7. Accessibility audit: zero axe violations on editor surfaces.
8. All Phase 01 tests still pass.

## Review gate

- The single-EditorView-per-chapter pattern is implemented; not "one EditorView for the whole manuscript".
- Property tests cover every node type.
- Hunspell dictionary is loaded lazily, not on app start (cold-open budget).
- Snapshot UI persists into `snapshots` and `snapshots/objects/`.
- Citation/footnote/equation node payloads are stable across save/reload.

## Out of scope

- Tracked-changes acceptance flow (Phase 06; mark types defined now).
- Citation engine resolution to CSL output (Phase 06).
- Math rendering (KaTeX wiring is Phase 06).
- AI assistance (Phase 03).
- Plugin-provided UI (Phase 07).

## When you finish

Update `STATUS.md`. PR title `Phase 02: Editor core`. Phase 03 may begin in parallel with Phase 04 once 02 merges.
