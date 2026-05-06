# Technical Architecture Document — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 1. Architectural goals (in priority order)

1. **Local-first correctness.** No feature requires a network call. Every cloud feature degrades cleanly to offline.
2. **Data durability.** A user's manuscript must survive crashes, OS-level kill, disk-full, and bad-shutdown. Corruption is unacceptable.
3. **Privacy by construction.** PII never leaves the device unless the user explicitly enables a network feature. Plugins cannot exfiltrate without capability grants.
4. **Performance envelope.** Editor latency p95 ≤30 ms; cold-open p50 ≤1.5 s for 200k words; full project validate ≤10 s for 100k words.
5. **Cross-platform parity.** Identical UX on Windows, macOS, and Linux; no per-OS feature drift.
6. **Extensibility without break.** Plugin API has a 6-month-minimum compatibility commitment from V1.0 onward.

When goals conflict, **correctness > durability > privacy > performance > parity > extensibility** — durability never loses to performance; performance never loses to a slick UX trick.

## 2. Top-level component map

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         BooksForge Application                           │
│                                                                         │
│  ┌─────────────────────────────┐    ┌────────────────────────────────┐  │
│  │   Tauri host (Rust core)    │◄───┤   Frontend: React + TS + Vite  │  │
│  │   - Window mgmt              │    │   - Editor (TipTap)            │  │
│  │   - IPC bridge               │    │   - Design system              │  │
│  │   - Filesystem access        │    │   - State (Zustand + Immer)    │  │
│  │   - Updater, deep links      │    │   - Renderer of validator UI   │  │
│  └─────────────┬───────────────┘    └────────────────────────────────┘  │
│                │ tauri::command + events                                 │
│  ┌─────────────▼─────────────────────────────────────────────────────┐  │
│  │              Rust Sidecar (in-process modules)                     │  │
│  │  ┌─────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐  │  │
│  │  │ Project │ │ Storage  │ │ AI Bridge│ │ Export  │ │ Validator   │  │  │
│  │  │ Service │ │ (SQLite) │ │ (gRPC?)  │ │ Pipeline│ │ Engine      │  │  │
│  │  └─────────┘ └──────────┘ └────┬─────┘ └────┬────┘ └─────────────┘  │  │
│  └─────────────────────────────────┼────────────┼─────────────────────┘  │
│                                    │            │                       │
│  ┌────────────────────────────┐    │            │                       │
│  │ External sidecars (spawned)│    │            │                       │
│  │  ┌─────────┐ ┌──────────┐  │◄───┘            │                       │
│  │  │llama.cpp│ │  Ollama  │  │                 │                       │
│  │  └─────────┘ │(external)│  │                 │                       │
│  │              └──────────┘  │                 │                       │
│  │  ┌─────────┐ ┌──────────┐  │                 │                       │
│  │  │ Pandoc  │ │epubcheck │  │◄────────────────┘                       │
│  │  └─────────┘ └──────────┘  │                                         │
│  │  ┌─────────────────────┐   │                                         │
│  │  │ Plugin host (WASM   │   │                                         │
│  │  │ runtime: wasmtime)  │   │                                         │
│  │  └─────────────────────┘   │                                         │
│  └────────────────────────────┘                                         │
│                                                                         │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Project bundle on disk: MyBook.booksforge/                          │ │
│  │   manifest.toml, project.db (SQLite), manuscript/, assets/,        │ │
│  │   snapshots/, exports/, .lock                                      │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

See `diagrams/component-architecture.svg` for the rendered version.

## 3. Layering and boundaries

The application has four layers with strict directional dependencies — upper layers may call lower, never the reverse.

**Layer 1 — Presentation (TypeScript / React).** UI components, editor host, view-model. Owns no business rules; calls into Layer 2 via a typed IPC client. Imports zero Rust.

**Layer 2 — Application services (Rust, exposed as `tauri::command`s).** Project lifecycle, document operations, validator runs, export jobs, AI request orchestration, plugin orchestration. This is the *anti-corruption layer* between UI and storage.

**Layer 3 — Domain (Rust).** Pure-logic crates: `booksforge-domain` (project model, document tree, citations), `booksforge-validator` (validator API and built-ins), `booksforge-template` (template parsing and compilation), `booksforge-export` (DOM → target serialisation). Pure functions, no I/O, no clocks, no randomness — all injected.

**Layer 4 — Infrastructure (Rust).** SQLite adapter, filesystem adapter, plugin host adapter, AI runtime adapter, network clients. Each is a trait at Layer 3; only Layer 4 implements.

This is hexagonal/ports-and-adapters. The reason: it lets us unit-test the domain without spinning up SQLite, swap llama.cpp for a mock in tests, and replace the editor without touching domain logic.

## 4. Crate / module layout

```
booksforge/
├── apps/
│   └── desktop/                  # Tauri app
│       ├── src/                   # Rust (Tauri host + Layer 2 commands)
│       └── src-ui/                # React + TS frontend
├── crates/
│   ├── booksforge-domain/          # Layer 3: pure model
│   ├── booksforge-template/        # Layer 3: template parsing/compile
│   ├── booksforge-validator/       # Layer 3: validator engine + built-ins
│   ├── booksforge-export/          # Layer 3: export DOM and serialisers
│   ├── booksforge-ai/              # Layer 3: prompt builder, context selector
│   ├── booksforge-storage/         # Layer 4: SQLite adapter
│   ├── booksforge-fs/              # Layer 4: bundle filesystem adapter
│   ├── booksforge-plugin-host/     # Layer 4: WASM plugin runtime
│   ├── booksforge-ai-runtime/      # Layer 4: llama.cpp / Ollama / cloud adapters
│   ├── booksforge-export-pandoc/   # Layer 4: Pandoc sidecar adapter
│   ├── booksforge-ipc/             # IPC types (shared with TS via codegen)
│   └── booksforge-test-fixtures/   # Shared test fixtures
├── packages/
│   ├── ui/                        # Design system (React)
│   ├── editor/                    # TipTap editor wrapper
│   ├── plugin-sdk/                # TS plugin SDK
│   └── shared-types/              # TS types generated from `booksforge-ipc`
├── plugins/                       # First-party plugins (template packs, validators)
├── docs/                          # End-user docs (mdBook)
├── tools/                         # Code-gen, migration scripts
└── .github/                       # CI workflows
```

## 5. IPC and type sharing

UI ↔ Rust over Tauri IPC. We use **`ts-rs`** to generate TypeScript types from Rust structs in `booksforge-ipc`. Every command has a typed input, typed output, and typed error. Errors are tagged unions, never strings — UI can pattern-match. Long-running operations (export, AI, full-project validate) emit progress events keyed by a job-id; UI tracks jobs in a global jobs store.

**[GUARD]** No untyped strings across the IPC boundary. Every command and event has a generated TS type. Build fails if types drift.

## 6. Editor framework — [DECISION-002]

**Choice: TipTap (ProseMirror-based) with custom UI.**

Considered: TipTap, Lexical, custom ProseMirror, Slate, CodeMirror-as-prose-host.

| Option | Pros | Cons |
|--------|------|------|
| TipTap | Mature, ProseMirror under the hood, well-typed, extensible, large community | Some marketing-leaning APIs; Pro features are paid (optional) |
| Lexical (Meta) | Modern, framework-agnostic, fast | Younger ecosystem, fewer book-grade extensions, breaking changes more frequent |
| Custom ProseMirror | Maximum control | Multi-month effort to reach feature parity with TipTap baseline |
| Slate | React-native model | Performance issues with large documents; slow with footnotes/tracked-changes |
| CodeMirror | Excellent perf | Source-code editor; rich-text via overlays is fragile |

**Why TipTap:** the document model we need (block tree, marks, footnotes, tracked changes) is exactly ProseMirror's wheelhouse, and TipTap saves us 3–4 months of bootstrapping. We avoid TipTap Pro paid extensions in MVP and write our own where needed.

### 6.4 Editor performance plan

A 200k-word manuscript renders a single chapter at a time, lazy-mounting the next chapter on scroll proximity. Document state is held in ProseMirror but persisted scene-by-scene to SQLite. We never serialise the whole manuscript into a single ProseMirror state — chapters are independent EditorView instances stitched into a virtualised list.

## 7. Sidecar runtime — [DECISION-003]

**Choice: Rust sidecar (in-process modules) for the application services.**

Rust gives us deterministic memory, easy embedding of llama.cpp and SQLite, ts-rs codegen, and a single distribution binary. Node was considered for the convenience of npm libraries but loses on (a) startup latency, (b) memory footprint, (c) FFI complexity for llama.cpp, (d) two runtimes to ship.

External processes that *do* run as separate binaries are: Pandoc (license isolation), llama.cpp (optional swap to user-installed Ollama), epubcheck (Java), and plugin WASM modules (sandbox). Everything else is in-process Rust.

## 8. Project bundle format — [DECISION-004]

**Choice: directory bundle (`*.booksforge/`).**

Considered: directory bundle, single SQLite file, single zip-bundle, custom binary container.

A directory bundle wins because: (a) it is git-friendly so users get free version control, (b) on corruption, partial recovery is possible (lose one snapshot, not the manuscript), (c) on-disk inspection is allowed (advanced users debug their projects), (d) sync tools (Dropbox, iCloud, Syncthing) handle directories well, (e) snapshot dedupe via content-addressed storage is natural.

Disadvantages and mitigations: bundle integrity is harder to enforce — we use a `manifest.toml` with a content checksum and a tamper-resistant signature for marketplace bundles; "single file" UX is preserved by macOS package bit (`com.booksforge.project` UTI) so the bundle appears as one icon.

The schema, manifest format, and on-disk layout are in `04-data-model-and-project-format.md`.

## 9. SQLite usage

One SQLite database per project, file `project.db` inside the bundle. WAL mode. Synchronous=NORMAL (with a documented trade-off; FULL is the privacy-grade option). Foreign keys on. 64 KiB page size. Connection pooling via `sqlx` (one writer, multiple readers).

Schema migrations are versioned with `refinery` or a hand-rolled migrator embedded in the binary. **Never** auto-migrate without taking a snapshot first.

**[GUARD]** Every schema change ships with: forward migration, reverse migration (or documented irreversibility), test fixtures from previous version, snapshot taken before run.

## 10. AI runtime

See `08-ai-integration.md` for the full picture. Architecturally:

A trait `LlmProvider` in `booksforge-ai` is implemented in `booksforge-ai-runtime` for `Embedded(LlamaCpp)`, `External(Ollama)`, `Cloud(Anthropic | OpenAI | OpenRouter | …)`. The application layer never imports a concrete provider — only the trait. Switching providers is a config change.

Embedded llama.cpp is loaded via `llm` or `llama-cpp-rs` Rust bindings. Models are GGUF, downloaded from a curated catalogue with hash pinning. We never download arbitrary URLs. Models live in a per-user data directory outside the bundle so they aren't duplicated per project.

## 11. Pandoc and the export pipeline — [DECISION-005]

**Choice: Pandoc as a spawned sidecar binary, not statically linked.**

Pandoc is **GPLv2+**. Statically linking it taints the host binary with GPL. Spawning Pandoc as a separate process invoked over its CLI/JSON API is the standard way to use GPL tooling from non-GPL hosts (this is widely accepted; see the GNU FAQ on "mere aggregation"). We ship Pandoc inside the installer as a sidecar binary the app spawns.

The export pipeline pre-processes the document into Pandoc's native JSON AST, hands it to Pandoc, post-processes the result, and finalises (font embedding, cover insertion, EPUB validation). Details in `09-export-pipeline.md`. **[RISK R-07]** is the legal review of this approach.

## 12. Plugin runtime

UI plugins run in a Tauri **isolated WebView** (separate origin from the host). Compute plugins run as **WASM** modules under `wasmtime` with WASI-preview2 syscalls denied by default; capabilities are added per-plugin. Capability tokens are passed by the host on each call.

See `07-plugin-architecture.md` for the manifest format, capability list, and packaging.

## 13. State management (frontend)

**Zustand + Immer** for app state with **TanStack Query** for IPC-cached reads. Rationale: Redux is overkill, Recoil is dormant, MobX hides re-render reasons, signals are still maturing in React 19. Zustand is small, ergonomic, devtools-supported.

Editor state is **owned by ProseMirror** and synced into Zustand only for parts the UI needs (cursor, selection meta, dirty flag). We do not double-source the document.

## 14. Concurrency model

Rust side: a `tokio` multi-threaded runtime. Long-running jobs (export, validation, AI) are spawned tasks; cancellation via `CancellationToken`. The SQLite layer uses a single writer task fed by an mpsc channel — UI commands enqueue, never write directly — to keep the on-disk WAL tidy and avoid SQLITE_BUSY.

Frontend side: React 19 concurrent features for non-urgent state; `useSyncExternalStore` for editor state subscriptions.

## 15. Error handling

Every Rust command returns `Result<Output, BooksForgeError>`. `BooksForgeError` is a tagged enum with categories: `Validation`, `NotFound`, `Conflict`, `IO`, `Serialization`, `External`, `Plugin`, `Cancelled`, `Internal`. UI shows category-appropriate UI (toast, modal, blocking dialog).

**[GUARD]** No `unwrap()` in production paths outside test code and `main()`. Lints enforce.

## 16. Logging and telemetry

`tracing` for structured logs with `tracing-appender` for rotating file output. Levels: `error` always; `warn` always; `info` user-facing operations; `debug` developer-only. PII redaction filter applied at sink: scrub paths under user home except for project name; scrub manuscript content always; scrub email addresses.

Telemetry is **off by default**. When enabled, only event names + duration + non-PII metadata are sent — never content.

## 17. Build, packaging, signing, distribution

**Build:** Cargo workspaces + Vite + Tauri CLI. Reproducible builds where feasible (locked deps, fixed timestamps, pinned toolchains). Trunk-based development with feature flags for in-flight features.

**CI:** GitHub Actions matrix on ubuntu-22.04, macos-13 (Intel), macos-14 (Apple Silicon), windows-2022. Per-PR: lint, typecheck, unit tests, smoke-build. Per-merge to main: full build, integration tests, packaged artifact upload.

**Signing:** Microsoft EV signing certificate (Windows), Apple Developer ID (macOS, notarisation), GPG signing for Linux packages. Secrets in GitHub Actions OIDC + AWS KMS or 1Password CLI.

**Packaging:**
- Windows: MSI (WiX) and portable EXE
- macOS: DMG with notarisation, ARM + Intel universal binary
- Linux: AppImage and Flatpak (V1.0); deb/rpm only on community demand

**Distribution:** Direct download from `booksforge.app`. Auto-update via Tauri updater. Optionally Microsoft Store / Mac App Store later (constraints differ).

## 18. Performance budgets

| Surface | Budget | Measurement |
|---------|--------|-------------|
| Cold launch (no project) | p50 ≤ 1.0 s, p95 ≤ 2.0 s | startup probe in CI |
| Open 200k-word project | p50 ≤ 1.5 s, p95 ≤ 3.0 s | benchmark fixture |
| Editor keystroke latency | p95 ≤ 30 ms | dev-tools profiler |
| Scroll FPS (50k-word chapter) | ≥ 55 FPS | `requestAnimationFrame` probe |
| Validator full-project run | ≤ 10 s for 100k words | benchmark |
| AI 200-word rewrite (7B Q4 local) | ≤ 6 s on 16 GB Apple-Silicon Mac | benchmark |
| EPUB-3 export | ≤ 30 s for 100k words with images | benchmark |
| Memory (steady state, project open) | ≤ 600 MB | OS metric |

Budgets are enforced in CI as fail-on-regression. **[GUARD]** A PR that regresses any budget by >10% must include a benchmark explanation in its description.

## 19. Security architecture

See `06-security-privacy-compliance.md` for the threat model. Architectural posture:

- **Default-deny** for plugins, network, and filesystem-outside-bundle.
- **Capability tokens** required for any plugin → host call beyond pure compute.
- **Encrypted at rest** as user opt-in: Argon2id-derived key, AES-256-GCM, envelope encryption per node.
- **Updates** are signed; Tauri updater verifies before applying.
- **Crash dumps** scrubbed of manuscript content before optional upload.

## 20. Observability

Local: rotating logs, in-app diagnostic bundle export. Cloud (opt-in): Sentry-style crash reporting, anonymous-event metrics — both pluggable; no vendor lock-in. Self-hosted option for enterprise.

## 21. Testing strategy

See `11-test-and-validation-strategy.md`. Architectural commitments:

The domain crates target ≥90% line coverage with property tests for ProseMirror→DOM round-trips, snapshot diff/restore, and validator idempotence. Integration tests run against a real SQLite. E2E uses Playwright against a built Tauri app on each CI matrix entry. Performance benchmarks gate merges. Accessibility audits via axe-core in E2E.

## 22. Dependency policy

Every direct dependency is reviewed for: license compatibility, maintenance status (last commit ≤ 6 months unless mature), security history, transitive footprint. Renovate runs weekly. **[GUARD]** No GPL/AGPL Rust crate is statically linked into the binary. Pandoc, epubcheck, and any GPL tooling are sidecars.

## 23. Deprecation and migration

Plugin API: 6-month minimum compatibility commitment from V1.0. Deprecations announced one minor version before removal. Project file format: forward-compatible reads — newer versions read older formats; older versions refuse to open newer formats with a clear message.

## 24. Architecture review gates

Every phase exits only after the tech lead signs off on the following: (a) no Layer-violation imports, checked by lint, (b) IPC types regenerated and committed, (c) performance budgets met, (d) test coverage targets met, (e) security checklist for that phase signed, (f) docs updated for any user-visible change. Gates are mechanically enforced where possible (lints, CI checks) and human-reviewed for the rest.
