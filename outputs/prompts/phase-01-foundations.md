# Phase 01 — Foundations

> **Status note (2026-05-06):** This phase prompt is **superseded** by MZ-02 and MZ-03 in `IMPLEMENTATION_PLAN.md`. The procedures below are preserved for historical context; use the implementation pack for the MVP build.

## Goal

Implement project lifecycle end-to-end: create, open, close, save, autosave, crash recovery, lock file, and recent-projects list. Land the SQLite schema v1, the bundle filesystem adapter, and the typed IPC commands the UI uses for these flows. After this phase, a user can create a project from a stub template, write a few words, kill -9 the process, relaunch, and recover.

## Pre-conditions

- Phase 00 merged; CI green.
- `booksforge-ipc` and `booksforge-domain` exist with placeholders.
- Empty `booksforge-storage` and `booksforge-fs` crates ready to fill.

## Inputs (read these first)

1. `../_deep/03-TAD-technical-architecture.md` — sections 3, 4, 7, 9, 14, 15, 18.
2. `../_deep/04-data-model-and-project-format.md` — entire document.
3. `../_deep/05-workflow-and-dataflow.md` — sections 1, 2, 3, 8, 14.
4. `../_deep/02-FSD-functional-specifications.md` — section 1 (FR-PROJ-001 through FR-PROJ-017) and section 8 (FR-SNAP-001…006 — at least the manual snapshot path).
5. `../_deep/06-security-privacy-compliance.md` — section 4 (controls).

## Deliverables

### 1. `booksforge-domain` model

Define the typed model: `Project`, `Manifest`, `Node` (with `kind` enum: `Project|Part|Chapter|Scene|FrontMatter|BackMatter`), `SceneContent`, `Note`, `Entity`, `EntityScene`. Use ULIDs for IDs (crate: `ulid`). LexoRank-style ordering (start with the simpler approach: f64 fractional positions, document the limitation in code).

Pure functions live here: `Project::scaffold_from_template(template, meta) -> ProjectShape`. No I/O. Property tests for ordering and tree integrity.

### 2. `booksforge-storage` SQLite adapter

Use `sqlx` (with the SQLite feature, async). Implement schema migration v1 from Data Model §4. Migration files in `crates/booksforge-storage/migrations/V001__initial.sql`. Migrations run in a transaction. Pre-migration snapshot hook (calls into `booksforge-fs` to take a snapshot before any future migration ≥ V002).

Public API (excerpt):

```rust
pub trait Storage {
    async fn open(path: &Path) -> Result<Self>;
    async fn close(self) -> Result<()>;
    async fn create_node(&self, node: NodeInsert) -> Result<NodeId>;
    async fn update_scene(&self, node_id: NodeId, content: SceneContent) -> Result<()>;
    async fn read_scene(&self, node_id: NodeId) -> Result<SceneContent>;
    async fn list_children(&self, parent: NodeId) -> Result<Vec<Node>>;
    async fn move_node(&self, node_id: NodeId, new_parent: NodeId, new_position: f64) -> Result<()>;
    // ... see Data Model
}
```

Single-writer-task pattern (TAD §14): all writes go through an mpsc channel into a dedicated task that owns the SQLite write connection. Reads use a connection pool.

WAL mode on. Foreign keys on. `synchronous=NORMAL` default; expose a setting for `FULL`.

### 3. `booksforge-fs` bundle filesystem adapter

Public API:

```rust
pub trait BundleFs {
    async fn create_atomic(path: &Path, manifest: &Manifest) -> Result<BundleHandle>;
    async fn open(path: &Path) -> Result<BundleHandle>;
    async fn read_manifest(handle: &BundleHandle) -> Result<Manifest>;
    async fn write_manifest(handle: &BundleHandle, manifest: &Manifest) -> Result<()>;
    async fn write_manuscript_mirror(handle: &BundleHandle, node_id: NodeId, md: &str) -> Result<()>;
    async fn acquire_lock(handle: &BundleHandle) -> Result<LockGuard>;
    // ...
}
```

Atomic create: temp dir under the parent directory, populate, rename. On collision, fail clearly.

The Markdown mirror writes are atomic per file: write to `*.tmp` then rename. Mirror layout matches the structure in Data Model §2.

The lock file uses platform-appropriate primitives (`fs2::FileExt` on UNIX, `LockFileEx` on Windows).

### 4. Crash recovery

Implement the recovery log described in Data Model §9: `.recovery.log` is append-only with framed records `{node_id, pm_doc_json_blob_compressed, hash, timestamp}`. The autosave path writes to the log first, then commits to SQLite, then truncates the log entry. On open, if the log has un-truncated entries newer than `manifest.integrity.project_db_hash`, the UI is told to surface a recovery dialog.

A recovery test in `booksforge-storage/tests/crash_recovery.rs` does the following: create a project, write content, *do not* close cleanly, simulate a crash by corrupting the WAL, reopen, expect recovery prompt, accept recovery, verify content is restored.

### 5. Layer-2 IPC commands

Add Tauri commands in `apps/desktop/src/commands/project.rs`:

- `project.create({ path, template_id, meta }) -> ProjectHandle`
- `project.open({ path }) -> ProjectHandle`
- `project.close({ handle_id }) -> ()`
- `project.recent() -> RecentProject[]`
- `project.set_pinned({ handle_id, pinned: bool }) -> ()`
- `project.import_docx({ src_path, dst_path }) -> ProjectHandle` *(stub: error "not implemented in Phase 01" — formal in Phase 06 — but the IPC type exists)*
- `node.list_children({ parent_id }) -> Node[]`
- `scene.read({ node_id }) -> SceneContent`
- `scene.save({ node_id, pm_doc_json, hash }) -> SaveAck`

All input/output types in `booksforge-ipc`. TS types regenerated and committed.

### 6. Stub template for "Generic Novel"

A templates directory bundled with the app: `apps/desktop/templates/generic-novel/template.toml` plus a `scaffold.json`. The scaffold lists Front Matter (Title Page, Copyright, Dedication), three Chapters with one empty Scene each, and Back Matter (About the Author).

Template loader implementation in `booksforge-template`. Public API: `Template::load(id) -> Result<Template>`, `Template::scaffold(meta) -> ProjectShape`. The `Template` struct deserialises from `template.toml`. Validators-required and AI-prompt-overrides fields are present but unused in this phase.

### 7. UI screens

- **Welcome / project picker**: lists recent projects, "New project" button, "Open project" button.
- **New project wizard**: choose template (one option for now), set title, author, language, target word count.
- **Project window**: minimal — a left binder showing the tree from `node.list_children`, a center pane with a textarea bound to one scene's `pm_doc_json` (raw JSON for now; the editor lands in Phase 02), word count.
- **Recovery dialog** when the recovery log has entries.

State held in Zustand. Reads via TanStack Query.

### 8. Autosave

Debounced 5 s after last keystroke (§3 of Workflow doc). Write order: recovery log → SQLite → Markdown mirror. Status bar reflects state: Editing → Saving → Saved.

### 9. Tests

- Unit tests in `booksforge-domain` for tree operations.
- Integration tests in `booksforge-storage` against a temp SQLite DB: create/read/update/delete nodes, scene content round-trip, schema migration.
- Integration test in `booksforge-fs` for atomic create, lock acquisition, mirror write.
- Crash recovery test (above).
- Property test: create N nodes with random parents and positions, list-children returns them in position order with no duplicates.
- E2E (Playwright) on built Tauri app: create project from wizard, write text, close, reopen, content present.

### 10. Performance probe

Add `crates/booksforge-storage/benches/open_project.rs` (criterion). Fixture: a 200k-word generated project. Assert open time p50 ≤ 1.5 s on the reference Mac. Wire into the nightly CI pipeline; fail nightly if regressed.

## Guard-rails specific to this phase

**[GUARD-P1-1]** All SQLite writes go through the single-writer task. Search for any `sqlx` query call from outside the writer task; if found, refactor.

**[GUARD-P1-2]** The Markdown mirror write must not block the SQLite save acknowledgement to UI. SQLite confirms first; mirror writes asynchronously.

**[GUARD-P1-3]** No unwrap. No expect. Typed errors only.

**[GUARD-P1-4]** Schema migration must be transactional and snapshotted (snapshot infra is stubbed here; the migrator calls a no-op snapshot hook that will be implemented in Phase 09 — but the call site exists).

**[GUARD-P1-5]** `.booksforge` is the bundle extension. macOS UTI registration in `tauri.conf.json` and `Info.plist`.

**[GUARD-P1-6]** Atomic creation. Tests assert that interrupting a `project.create` mid-flight leaves no half-bundle on disk.

**[GUARD-P1-7]** No editor work. We are wiring plumbing; the editor lives in Phase 02. A textarea is a placeholder.

## Acceptance criteria

1. Create a new "Generic Novel" project from the wizard; landing on the project window in <2 s.
2. Type into the placeholder scene textarea; status bar shows Saved within 5 s.
3. Close the app, reopen — the project is in recents and content is preserved.
4. Kill the app while editing (`kill -9` / Activity Monitor); reopen — recovery dialog appears, accepting recovers content.
5. Move the project bundle directory to a different folder; "missing" badge appears in recents.
6. `cargo test --workspace` and Playwright E2E pass on all three OSes in CI.
7. Performance probe meets the 1.5 s p50 budget on reference hardware.
8. The Markdown mirror exists at the documented path and matches the SQLite content for the test scenes.

## Review gate

The tech lead inspects:

- Layering: no `sqlx` import outside `booksforge-storage`; no `booksforge-storage` import in `booksforge-domain`. Lints enforce.
- Single-writer task: only one `WriteConn` exists; reads go through the pool.
- Recovery test asserts content recovery and not just "no crash".
- Atomic create test asserts no half-bundle on disk after a simulated mid-flight failure.
- IPC types are generated and committed; no untyped strings.
- E2E covers the happy path on all three OS matrix entries.

## Out of scope

- Tracked changes, citations, footnotes, math (Phase 02 / 06).
- The TipTap editor (Phase 02).
- Validators, templates beyond the one stub (Phase 05).
- AI of any kind (Phase 03).
- Encryption (Phase 09).
- DOCX import — the IPC stub exists; the implementation lands in Phase 06.

## When you finish

Open the PR. Update `outputs/prompts/STATUS.md`. Ensure `docs/api/` reflects every new IPC command. Close the phase only after the review gate is signed.
