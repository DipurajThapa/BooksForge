# Architecture — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06

This document is the implementation-ready architecture — the contract Claude Code will build against.

---

## 1. System overview

BooksForge is a desktop application with three runtime components:

1. The **BooksForge app** — Tauri v2 host (Rust) + React/TypeScript UI in a single signed binary.
2. The **Ollama runtime** — a separate, locally installed process the user controls, exposing an HTTP API on `127.0.0.1:11434`.
3. **External binaries** invoked by BooksForge: Pandoc (export), epubcheck (EPUB validation). These are bundled with the installer.

```
┌─────────────────────────────────────────────────────────────────┐
│ Desktop OS (macOS / Windows; Linux V1.0)                        │
│                                                                 │
│  ┌──────────────────────────────┐    ┌─────────────────────────┐│
│  │ BooksForge.app (signed)       │    │ Ollama (separate proc)  ││
│  │                              │    │   localhost:11434       ││
│  │  ┌────────────────────────┐  │    │                         ││
│  │  │ React/TypeScript UI    │  │    │  qwen2.5:7b ◄─┐         ││
│  │  │  (TipTap editor)       │  │    │  llama3.1:8b   │ models  ││
│  │  └────────┬───────────────┘  │    │  mistral:7b    │ pulled  ││
│  │           │ Tauri IPC         │    │  gemma2:9b    │ on user ││
│  │  ┌────────▼───────────────┐  │    │  phi3.5       ◄┘ request ││
│  │  │ Rust core (in-process) │  │    └────────────▲────────────┘│
│  │  │  • Project service     │  │                 │             │
│  │  │  • Storage (SQLite)    │  │                 │ HTTP/SSE    │
│  │  │  • Agent orchestrator  ├──┼─────────────────┘             │
│  │  │  • Ollama client       │  │                               │
│  │  │  • Validators          │  │                               │
│  │  │  • Export coordinator  ├──┼──────► Pandoc (sidecar)       │
│  │  └────────────────────────┘  │                               │
│  │                              ├──────► epubcheck (sidecar)    │
│  └──────────────────────────────┘                               │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Project bundle: MyBook.booksforge/                          │ │
│  │   manifest.toml, project.db, manuscript/, assets/,         │ │
│  │   snapshots/, exports/, agent_runs/                        │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

The diagram above is canonical for the MVP.

## 2. Layering

Four strict layers with single-direction dependencies. Lints enforce.

- **Layer 1 — Presentation (TypeScript / React).** UI components, editor host, view-model. Imports zero Rust except via typed IPC clients generated from `booksforge-ipc`.
- **Layer 2 — Application services (Rust, exposed as `tauri::command`s).** Project lifecycle, agent orchestration, validator runs, export jobs. Anti-corruption layer.
- **Layer 3 — Domain (Rust).** Pure-logic crates (`booksforge-domain`, `booksforge-template`, `booksforge-validator`, `booksforge-agents`, `booksforge-export`, `booksforge-prompt`). No I/O, no clocks, no randomness.
- **Layer 4 — Infrastructure (Rust).** SQLite, filesystem, Ollama HTTP client, Pandoc sidecar, epubcheck sidecar. Each is a trait at Layer 3; only Layer 4 implements.

Any cross-layer call goes through a trait boundary. This is what makes the agent orchestrator unit-testable without spinning up a real Ollama.

### 2.1 Layer boundary traits

The three stable trait boundaries are defined here so every crate knows which side of the boundary it lives on.

#### `StorageRepository` (defined in `booksforge-storage`, used by `apps/desktop`)

```rust
/// Layer 4 boundary — SQLite adapter.
/// Implemented by `SqliteStorage` in booksforge-storage.
/// Injected into Tauri command handlers as managed state.
#[async_trait]
pub trait StorageRepository: Send + Sync {
    /// Open (or create) the project database at the given path.
    /// Runs pending migrations automatically after taking a pre_migration snapshot.
    async fn open(&self, db_path: &Path) -> Result<(), StorageError>;

    /// Fetch all non-deleted nodes ordered by position.
    async fn list_nodes(&self) -> Result<Vec<Node>, StorageError>;

    /// Insert a new node row. Caller must ensure parent_id exists if set.
    async fn insert_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Update an existing node (title, status, position, pov, beat, target_words).
    async fn update_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Soft-delete a node by setting deleted_at = now().
    async fn delete_node(&self, id: &Ulid) -> Result<(), StorageError>;

    /// Load scene content for a node. Returns None if never saved.
    async fn load_scene(&self, node_id: &Ulid) -> Result<Option<SceneContent>, StorageError>;

    /// Upsert scene content (insert or replace).
    async fn save_scene(&self, content: &SceneContent) -> Result<(), StorageError>;

    /// Read the singleton style_book row. Returns default if no row yet.
    async fn load_style_book(&self) -> Result<StyleBook, StorageError>;

    /// Upsert the singleton style_book row.
    async fn save_style_book(&self, style: &StyleBook) -> Result<(), StorageError>;

    /// List all entities (non-deleted) with their aliases.
    async fn list_entities(&self) -> Result<Vec<Entity>, StorageError>;

    /// Insert a new entity and its aliases atomically.
    async fn insert_entity(&self, entity: &Entity) -> Result<(), StorageError>;

    /// Record an agent run row (status = running on insert).
    async fn insert_agent_run(&self, run: &AgentRun) -> Result<(), StorageError>;

    /// Update run status, completed_at, totals_json.
    async fn update_agent_run(&self, run: &AgentRun) -> Result<(), StorageError>;

    /// Insert a snapshot manifest row.
    async fn insert_snapshot(&self, snap: &SnapshotRecord) -> Result<(), StorageError>;
}
```

#### `BundleFilesystem` (defined in `booksforge-fs`, used by `apps/desktop`)

```rust
/// Layer 4 boundary — bundle filesystem adapter.
/// Implemented by the real OS filesystem; test implementations use tmp dirs.
#[async_trait]
pub trait BundleFilesystem: Send + Sync {
    /// Create a new empty bundle atomically.
    ///
    /// 1. Creates a temp directory under the system temp dir.
    /// 2. Writes `manifest.toml`, empty subdirectories, `.booksforge-version`.
    /// 3. Renames the temp dir to `final_path` atomically.
    /// 4. On any failure before rename: removes the temp dir.
    async fn create_bundle(
        &self,
        final_path: &Path,
        manifest: &str,              // TOML string
    ) -> Result<BundlePath, FsError>;

    /// Open an existing bundle. Validates presence of `manifest.toml`.
    /// Acquires the advisory lock file (fails if already locked by another PID).
    fn open_bundle(&self, path: &Path) -> Result<BundlePath, FsError>;

    /// Release the advisory lock and flush pending writes.
    async fn close_bundle(&self, path: &BundlePath) -> Result<(), FsError>;

    /// Write `manifest.toml` atomically (tmp + rename).
    async fn write_manifest(
        &self,
        bundle: &BundlePath,
        toml: &str,
    ) -> Result<(), FsError>;

    /// Write a chapter Markdown mirror file.
    async fn write_chapter_md(
        &self,
        bundle: &BundlePath,
        node_ulid: &str,
        markdown: &str,
    ) -> Result<(), FsError>;

    /// Write a content-addressed snapshot object.
    /// Returns the blake3 hex hash (which is also the filename).
    async fn write_snapshot_object(
        &self,
        bundle: &BundlePath,
        content: &[u8],
    ) -> Result<String, FsError>;
}
```

#### `OllamaClient` (defined in `booksforge-ollama` — see §5.4)

Already specified in §5.4. Repeated here for completeness of the boundary map.

| Trait | Defined in | Implemented by | Injected into |
|-------|-----------|----------------|---------------|
| `StorageRepository` | `booksforge-storage` | `SqliteStorage` | Tauri app state |
| `BundleFilesystem` | `booksforge-fs` | `OsFilesystem` | Tauri app state |
| `OllamaClient` | `booksforge-ollama` | `HttpOllamaClient` | Tauri app state |

## 3. Crate / module layout (MVP)

```
booksforge/
├── apps/
│   └── desktop/                       # Tauri app
│       ├── src/                        # Rust (Tauri host + L2 commands)
│       └── src-ui/                     # React + TS frontend (Vite)
├── crates/
│   ├── booksforge-domain/              # L3 — project model, document tree
│   ├── booksforge-template/            # L3 — template parsing/compile
│   ├── booksforge-validator/           # L3 — validator engine + built-ins
│   ├── booksforge-agents/              # L3 — agent definitions, schemas, prompts
│   ├── booksforge-prompt/              # L3 — prompt template engine (MiniJinja-based)
│   ├── booksforge-memory/              # L3 — book/chapter/entity/style memory
│   ├── booksforge-vocab/               # L3 — vocabulary dictionaries (layered)
│   ├── booksforge-export/              # L3 — export DOM + Pandoc-AST builder
│   ├── booksforge-ipc/                 # IPC types (codegen → TS via ts-rs)
│   ├── booksforge-storage/             # L4 — SQLite adapter
│   ├── booksforge-fs/                  # L4 — bundle filesystem adapter
│   ├── booksforge-ollama/              # L4 — Ollama HTTP client
│   ├── booksforge-orchestrator/        # L4 — agent orchestration (uses L3 agents + L4 ollama)
│   ├── booksforge-export-pandoc/       # L4 — Pandoc sidecar adapter
│   ├── booksforge-epubcheck/           # L4 — epubcheck sidecar adapter
│   └── booksforge-test-fixtures/       # Shared test fixtures
├── packages/
│   ├── ui/                             # Design system (React)
│   ├── editor/                         # TipTap wrapper
│   └── shared-types/                   # Generated TS types from booksforge-ipc
├── templates/                          # Built-in templates (TOML + assets)
├── docs/                               # Source docs (this folder is the spec)
└── .github/                            # CI workflows
```

`booksforge-orchestrator` is at Layer 4 because it composes Layer 3 (agent definitions, prompt rendering, validator) with Layer 4 adapters (Ollama, storage). This is the standard hexagonal pattern. Cloud-provider adapters slot into `booksforge-ollama`'s `LlmProvider` trait when they ship post-V1.0.

## 4. IPC contract

UI ↔ Rust over Tauri commands and events. `booksforge-ipc` defines all types. `ts-rs` generates TypeScript bindings on `cargo build`. Drift fails CI.

Every command has:

- A typed input (`#[derive(Deserialize, TS)]`).
- A typed output (`#[derive(Serialize, TS)]`).
- A typed error: a tagged enum (`BooksForgeError::Validation { … }`, `::NotFound { … }`, etc.). Never raw strings.

Long-running operations emit `progress` events keyed by a `job_id`. A `cancel(job_id)` command aborts.

## 5. Ollama integration — the primary local-LLM runtime

For MVP and V1.0, Ollama is the local-LLM runtime. Embedded llama.cpp is deferred to post-V1.0 behind a feature flag.

### 5.1 Why Ollama-first

- One integration that handles model download, version pinning, GPU detection, and a stable HTTP API.
- Cross-platform: macOS (Metal), Windows (CUDA + DirectML), Linux (CUDA + ROCm).
- Permissive licence (MIT) and active maintenance.
- Lets us ship BooksForge as a much smaller binary; no embedded inference engine, no per-platform GGUF mass.

### 5.2 Setup flow (`OllamaSetupWizard`)

On first launch and on demand from settings:

1. Probe `127.0.0.1:11434/api/version`. If reachable, show the detected version and proceed.
2. If not reachable, look for an installed Ollama binary on the platform-default path (`/Applications/Ollama.app`, `%LOCALAPPDATA%\Programs\Ollama`, `/usr/local/bin/ollama`). If found, offer to launch it.
3. If not installed, present an "Install Ollama" panel with two options:
   - **Guided install.** BooksForge downloads the official installer for the user's OS over HTTPS from `ollama.com/download/...`, verifies a pinned SHA-256, and launches it. The user grants installer privileges.
   - **Manual install.** A copy-pasteable URL and instructions; we re-probe after the user confirms.
4. Once Ollama is reachable, list installed models. If the user has none, offer to pull a recommended default based on their detected RAM.

This flow is implemented in `booksforge-ollama` with UI in `apps/desktop/src-ui/src/setup/`.

### 5.3 Curated model registry

`booksforge-ollama/models.toml` holds the curated list. Each entry:

```toml
[[model]]
id = "qwen2.5:7b-instruct-q4_K_M"
display_name = "Qwen 2.5 7B Instruct (Q4)"
family = "qwen"
size_bytes = 4_700_000_000
ram_min_gb = 16
context_window = 32768
chat_format = "chatml"
recommended_for = ["fiction", "non_fiction", "memoir"]
strengths = ["dialogue", "non-english"]
notes = "Strong default for multilingual fiction."

[[model]]
id = "llama3.1:8b-instruct-q4_K_M"
display_name = "Llama 3.1 8B Instruct (Q4)"
family = "llama"
size_bytes = 4_800_000_000
ram_min_gb = 16
context_window = 131072
chat_format = "llama3"
recommended_for = ["fiction", "non_fiction", "academic"]
strengths = ["long-context", "instruction-following"]

# Mistral 7B, Gemma 2 9B, Phi-3.5 Mini, TinyLlama, etc.
```

The user can use any model Ollama exposes, but the curated list is what powers default selection and the auto-pull UI. The list is updated each release.

### 5.4 Ollama HTTP client (`booksforge-ollama`)

A trait-fronted client used by the orchestrator and the inline-quick-action handlers.

```rust
#[async_trait]
pub trait OllamaClient: Send + Sync {
    async fn version(&self) -> Result<OllamaVersion, OllamaError>;
    async fn list_local_models(&self) -> Result<Vec<LocalModel>, OllamaError>;
    async fn pull(&self, model: &str, progress: ProgressSink) -> Result<(), OllamaError>;
    async fn generate(&self, req: GenerateRequest, sink: TokenSink, cancel: CancelToken)
        -> Result<GenerateOutcome, OllamaError>;
    async fn chat(&self, req: ChatRequest, sink: TokenSink, cancel: CancelToken)
        -> Result<ChatOutcome, OllamaError>;
}
```

`GenerateRequest` and `ChatRequest` carry: model id, messages or prompt, sampling parameters, optional grammar/JSON mode, context window override. Cancellation aborts the HTTP request and (where supported) sends an `OPTIONS` cancel.

A `MockOllamaClient` ships in `booksforge-test-fixtures` and is used everywhere in unit tests.

### 5.5 Model selection rules

When an agent needs a model and the user has not pinned one for the agent:

1. Look up the agent's `model_preference` (`AGENTS.md §3`).
2. Filter the user's installed models by family preference and `ram_min_gb` ≤ detected RAM.
3. Pick the largest model whose context window ≥ the agent's required context.
4. If no model qualifies, return a typed `OllamaError::NoSuitableModel { reason }` and surface a UI prompt to pull the recommended model.

The user can override per agent, per project, or per call.

### 5.6 Fallback behaviour

- **Ollama unreachable.** The orchestrator returns `OrchestratorError::RuntimeUnavailable`. UI offers to launch Ollama, re-probe, or cancel. No agent runs.
- **Model not pulled.** Same shape. UI offers to pull or pick another. Pull progress is shown.
- **OOM / context overflow.** Typed `OllamaError::ContextTooLong` or `::OutOfMemory`. The orchestrator's recovery options are: shrink the context (drop low-priority items per `AGENTS.md §5`) and retry once, or surface the error.
- **Output validation failure.** The orchestrator's validation step (per agent) rejects malformed output. Up to two retries with a "be more strict about the schema" reminder appended. After that, return the raw output as a `proposal_invalid` artifact for human inspection — never silently accept.

### 5.7 Future runtimes

Embedded llama.cpp, OpenAI-compatible local servers (e.g., LM Studio's API), and cloud providers are all post-V1.0 and slot in behind the same `LlmProvider` trait that wraps `OllamaClient`. They are not built in MVP.

## 6. Agent orchestrator

The orchestrator is the controller for the agent swarm. The agents themselves are pure: a prompt template, an input schema, an output schema, and a validation function.

### 6.1 Workflow types

A **workflow** is a named, hard-coded sequence of agent steps with optional branches. Workflows are not dynamic — Claude Code does not invent workflows at runtime; the available workflows are listed in `AGENTS.md §7` and registered in `booksforge-orchestrator/src/workflows.rs`.

MVP workflows:

- `IntakeAndOutline` — `ProjectIntakeAgent` → `OutlineArchitectAgent` → user gate.
- `DraftScene` — `ChapterDraftingAgent` → user gate (**off by default**; user must opt in).
- `DevelopmentalReview` — `DevelopmentalEditorAgent` (per chapter) → user gate.
- `ContinuityCheck` — `ContinuityAgent` (project-wide) → user gate.
- `Copyedit` — `CopyeditorAgent` (per chapter) → user gate.

### 6.2 Run lifecycle

```
1. UI calls workflow.start({workflow_id, scope, options})
2. Orchestrator creates `agent_runs` row (status=running)
3. For each step in the workflow:
   a. Render prompt with the step's prompt template + scoped context
   b. Validate token budget; if over, run context-shrinker
   c. Call OllamaClient.chat(..., cancel_token)
   d. Parse + validate output against the agent's schema
   e. If valid: persist `agent_outputs` row, emit progress event
   f. If invalid: retry up to 2x, then surface inspectable error
   g. If the next step requires user gate: pause, emit `awaiting_user`, wait for `workflow.continue` or `workflow.cancel`
4. On final step success, emit `completed`; UI displays the proposals
5. On user accept: take pre_agent_edit snapshot, apply changes, record the edit in `agent_applied_edits`
```

### 6.3 Bounded execution — the "no infinite loop" guarantee

The orchestrator hard-caps every workflow run:

- **≤8 agent calls** per run.
- **≤10 minutes** wall-clock.
- **≤200,000 generated tokens** total.
- **≤3 retries** per step.
- **No agent-spawned-agent.** Agents return data; only the orchestrator can call another agent. Recursion is structurally impossible.
- **No tool-use that mutates the manuscript.** Agents have **no tools** in MVP. They are prompt → text. (Tools are V1.5+, with capability tokens; see `_deep/07-plugin-architecture.md`.)

These caps are encoded in `OrchestratorConfig` and enforced both before each call and as overall budget tracking.

### 6.4 Approval gates and conflict resolution

Every workflow that produces a manuscript-mutating proposal ends in a user gate. The user reviews the proposal; only on accept does the orchestrator persist the change.

If two workflows produce conflicting proposals (e.g., Continuity Agent says rename "Aiden" → "Aidan" while Copyeditor proposes a deletion of the same paragraph), the orchestrator applies them in **dependency order**: structural-first (continuity), then mechanical (copyedit). Conflicts surface in the UI with a side-by-side view and the user picks.

### 6.5 Storage

Run state is in SQLite (`agent_runs`, `agent_tasks`, `agent_outputs`, `agent_applied_edits`). Generated text artifacts > 4 KB are written to `agent_runs/<run_id>/<task_id>.json` inside the bundle and the SQLite row stores a path + hash.

### 6.6 Cancellation

Every long-running operation (agent run, export, validator) registers a `CancellationToken` keyed by `job_id`. UI can call `cancel(job_id)`. The token signals the Ollama client to abort the HTTP call; the orchestrator marks the run `cancelled` and emits the partial outputs as inspectable artifacts.

## 7. Storage and project bundle

Source of truth: `_deep/04-data-model-and-project-format.md`. The bundle layout is preserved with two additions in MVP:

```
MyBook.booksforge/
├── manifest.toml
├── project.db
├── manuscript/                 # Markdown mirror, regenerated on save
├── assets/                     # Content-addressed asset store
├── snapshots/                  # Content-addressed snapshot store
├── exports/                    # Export history
├── agent_runs/                 # NEW: per-run agent artifacts (>4 KB outputs)
│   └── <run_id>/<task_id>.json
├── plugins/                    # Empty in MVP; populated post-MVP
├── .lock
└── .booksforge-version
```

`agent_runs/` is gitignored if the user puts the bundle in git, by default — they are large and redundant with `agent_outputs` rows. We provide a `.gitignore` shipped in the bundle.

SQLite migration policy is unchanged from `04-data-model § 5`. New tables and changes for the agent layer are documented in `DATA_MODEL.md`.

## 8. Concurrency model

Rust side: a single `tokio` multi-threaded runtime. SQLite uses one writer task fed by an mpsc channel; readers use a connection pool. Long-running operations (agent runs, exports, validators) are spawned as cancellable tasks. The orchestrator's run loop is single-threaded per run; multiple workflows can run in parallel up to a cap of 2 concurrent workflows per project (one is plenty for MVP — concurrent runs are a V1.0 nicety).

Frontend side: React 19 with `useSyncExternalStore` for editor state subscriptions and TanStack Query for IPC reads. State management: Zustand + Immer.

## 9. Export pipeline

Source of truth: `_deep/09-export-pipeline.md`. MVP scope:

- DOCX, PDF (Trade 5×8 and 6×9), EPUB-3 (Generic + KDP-eBook).
- Pandoc invoked as a sidecar process (no static linking — Pandoc is GPL).
- epubcheck embedded as a sidecar with a small JRE bundle.
- Reproducibility: same input + same template + same engine version = byte-identical output. Hash-checked in CI on a fixture.

The pipeline is **rule-based, not agent-driven** in MVP. An agent does not decide formatting. We add an Export Agent in V1.0 only if it earns its place.

## 10. Performance budgets

| Surface | Budget | How measured |
|---------|--------|--------------|
| Cold launch (no project) | p50 ≤ 1.0 s, p95 ≤ 2.0 s | startup probe in CI |
| Open 100k-word project | p50 ≤ 1.5 s, p95 ≤ 3.0 s | benchmark fixture |
| Editor keystroke latency | p95 ≤ 30 ms | dev-tools profiler |
| Scroll FPS (50k-word chapter) | ≥ 55 FPS | requestAnimationFrame probe |
| Validator full-project run | ≤ 10 s for 100k words | benchmark |
| Agent first-token (7B-Q4 on 16 GB Mac) | ≤ 2 s | benchmark |
| Sustained agent throughput (7B-Q4) | ≥ 25 tokens/s | benchmark |
| EPUB-3 export | ≤ 30 s for 100k words with images | benchmark |
| Memory steady-state, project open | ≤ 600 MB BooksForge process | OS metric |

CI fails any PR with a >10% regression on any budget without an explanation in the PR body. Budgets live in `benches/budgets.toml`.

## 11. Error handling

Every Layer-2 command returns `Result<Output, BooksForgeError>`. Categories:

- `Validation` — input failed schema or business rule.
- `NotFound` — resource missing.
- `Conflict` — concurrent modification.
- `IO` — disk/file failure.
- `Serialization` — codec failure.
- `External` — sidecar (Ollama, Pandoc, epubcheck) failed.
- `Plugin` — post-MVP.
- `Cancelled` — user cancelled.
- `Internal` — bug — never silently caught.

Each category gets category-appropriate UI. Stack traces never reach the user.

## 12. Logging and telemetry

`tracing` for structured logs with `tracing-appender` for rotating file output (5 MB × 5). Levels: error, warn, info (user-facing operations), debug (developer-only).

PII redaction at the sink: scrub paths under user home except project name; **always** scrub manuscript content; scrub email addresses and license keys.

Telemetry is **off by default**. When on, only event names + duration + non-PII metadata are sent. Manuscript content never leaves the device under any circumstance.

## 13. Build, packaging, signing, distribution (MVP)

- **Build.** Cargo workspaces + Vite + Tauri CLI.
- **CI matrix.** macos-14 (Apple Silicon), macos-13 (Intel), windows-2022. (Linux on V1.0.)
- **Signing.** Apple Developer ID + notarisation; Microsoft EV cert. Secrets via GitHub Actions OIDC + 1Password CLI.
- **Packaging.** macOS DMG (universal binary); Windows MSI (WiX) + portable EXE.
- **Distribution.** Direct download from `booksforge.app`. Tauri auto-updater on a `beta` channel during MVP.
- **Reproducibility.** Locked deps, fixed timestamps, pinned toolchains where Tauri allows.

## 14. Security architecture (high-level)

Source of truth: `SECURITY_PRIVACY.md`. Posture:

- Default-deny for plugins (post-MVP), network, and filesystem-outside-bundle.
- Capability tokens for any plugin → host call (post-MVP).
- Updates signed; Tauri updater verifies before applying.
- Crash dumps scrubbed of manuscript before optional upload; upload opt-in.
- Encryption at rest is **post-MVP** (V1.0). MVP relies on filesystem permissions.

## 15. Architecture review gates

Every milestone exits only after the tech lead signs off on:

1. No layer-violation imports (lint).
2. IPC types regenerated and committed (codegen check).
3. Performance budgets met (CI bench).
4. Test coverage targets met.
5. Security checklist for that milestone signed.
6. Docs updated for any user-visible change.

These gates are mechanical where possible, human-reviewed for the rest.

## 16. What this architecture is not

- It is not a microservices architecture. It is one binary plus one user-managed runtime (Ollama) plus two sidecars (Pandoc, epubcheck).
- It is not a self-driving book writer. The agent swarm is bounded and gated; the user remains in control.
- It is not "cloud-aware with offline mode". It is offline-first; cloud is post-MVP and additive.
- It is not premature in its abstractions. The trait boundaries exist where we measurably need them (Ollama, storage, export adapters); we did not abstract everything that moved.
