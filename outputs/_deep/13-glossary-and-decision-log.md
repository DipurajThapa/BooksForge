# Glossary & Architecture Decision Log — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## A. Glossary

**BooksForge.** Working name for the platform; rename freely.

**Bundle.** The on-disk project as a directory (`*.booksforge/`) — see Data Model §2.

**Canonical AST.** The BooksForge intermediate representation of a project's content used as input to the export pipeline.

**Capability.** A named permission a plugin must request and the user must grant for the plugin to perform a privileged operation. See Plugin §5.

**CRDT.** Conflict-free replicated data type; basis for V1.5 collaboration.

**CSL.** Citation Style Language; the standard for citation formatting (Chicago, APA, MLA, etc.).

**Cold-open.** Time from launching the app to a project being interactive.

**Embedded LLM.** A model loaded inside the BooksForge process via llama.cpp. *Deferred to post-V1.0 per `[DECISION-006-revB]`.*

**External LLM.** A model running in a process BooksForge doesn't own. **Ollama** is the canonical external runtime and the **primary** local-LLM runtime for MVP and V1.0.

**Agent.** A specialised, bounded prompt → schema-typed output unit defined in `../AGENTS.md`. Distinct from a single-shot preset (Sharpen, Continue, etc.). Agents do not have tools and do not call other agents in MVP; the Orchestrator composes them.

**Orchestrator.** The Layer-4 component that runs workflows of agents under hard caps with approval gates and a pre-edit snapshot before any manuscript-mutating apply. Defined in `../ARCHITECTURE.md §6` and `../AGENTS.md`.

**Workflow.** A named, hard-coded sequence of agent steps (e.g., `IntakeAndOutline`, `Copyedit`). Workflows are not dynamic.

**Fenced content.** Untrusted content wrapped in `<<<USER_CONTENT>>> … <<<END_USER_CONTENT>>>` markers in AI prompts.

**Forward compatibility.** A newer BooksForge can open an older project; an older BooksForge refuses to open a newer one with a clear message.

**FR-XXX-NNN.** Functional requirement IDs in the FSD; stable and never renumbered.

**GGUF.** The model file format used by llama.cpp.

**IPC.** Inter-process communication; here, between the React UI and the Tauri/Rust host.

**LexoRank.** A string-based ordering scheme that allows insertion between any two adjacent items without renumbering.

**Manuscript mirror.** The Markdown copy of every scene, written alongside SQLite as a recovery surface. See Workflow §3.

**Pandoc-AST.** Pandoc's native JSON document representation.

**Profile.** A composition of (template, target format, post-processors, validators) — see Export Pipeline §4.

**Recovery log.** Append-only journal of unsaved edits used for crash recovery.

**Sidecar.** A separate executable spawned by BooksForge — Pandoc, epubcheck, optionally Ollama.

**Snapshot.** A point-in-time content-addressed copy of a node or the whole project.

**Storage layer.** The Layer-4 adapter wrapping SQLite plus the bundle filesystem.

**Studio tier.** Subscription-tier including cloud LLM credits, sync, and collaboration.

**TAD.** Technical Architecture Document.

**Template.** A versioned bundle of project skeleton + style rules + validator hints + AI prompt overrides; ships in-app or as a plugin.

**TipTap.** ProseMirror-based editor framework chosen for our UI.

**Validator.** A pure function from project state to a list of issues; runs on demand and pre-export.

**WAL.** Write-ahead log (SQLite mode).

**WIT.** WebAssembly Interface Types — the schema language for plugin host interfaces.

---

## B. Architecture decision log

Decisions are recorded in this format: **[ID]** Title — Date — Status — Context — Decision — Consequences.

### [DECISION-001] Pricing — perpetual + subscription

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Persona-A indie authors hate subscriptions; persona-B/C professionals expense them.

**Decision.** Offer both Pro perpetual ($129) and Pro monthly ($7), plus Studio subscription ($19/mo).

**Consequences.** Two SKUs to support; license-management complexity; clearer marketing story.

### [DECISION-002] Editor framework — TipTap

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Need a fast, extensible rich-text editor for novel-scale content.

**Decision.** TipTap (headless, ProseMirror-based) with custom UI; avoid TipTap Pro extensions in MVP.

**Consequences.** Saves 3–4 months of bootstrapping; ProseMirror community support; some custom work needed for tracked changes (no off-the-shelf TipTap solution we'd ship).

### [DECISION-003] Sidecar runtime — Rust

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Tauri allows Rust or external Node sidecars; we need to embed llama.cpp and SQLite.

**Decision.** Rust sidecar (in-process modules) for application services; external processes for Pandoc, epubcheck, optional Ollama.

**Consequences.** Single distribution binary; faster startup; simpler FFI to llama.cpp; Rust learning curve for the team.

### [DECISION-004] Project file format — directory bundle

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Need durability, recoverability, sync-friendliness, and inspectability.

**Decision.** `*.booksforge/` directory bundle with SQLite + Markdown mirror + content-addressed assets and snapshots.

**Consequences.** Excellent recovery story and git friendliness; macOS package-bit needed to keep one-icon UX; slightly more complex bundle integrity checking.

### [DECISION-005] Pandoc — sidecar process, not statically linked

**Date:** 2026-05-06. **Status:** Confirmed (subject to legal review per R-07).

**Context.** Pandoc is GPLv2+; we want our host code under a permissive license.

**Decision.** Spawn Pandoc as a separate process via stdin/stdout JSON. Bundle the Pandoc binary in installers with its license.

**Consequences.** GPL stays at process boundary; small IPC overhead per export; clean licensing story.

### [DECISION-006] Local LLM runtime — embedded llama.cpp + Ollama detection

**Date:** 2026-05-06. **Status:** Superseded by [DECISION-006-revB].

**Context.** Want zero-install AI for novices; want power-user choice.

**Decision.** Embed llama.cpp; auto-detect a running Ollama and offer its models as an additional provider.

**Consequences.** Single-binary install for AI; cooperate with users' existing setups.

### [DECISION-006-revB] Local LLM runtime — Ollama-first; embedded llama.cpp deferred

**Date:** 2026-05-06. **Status:** Confirmed for MVP and V1.0.

**Context.** Shipping our own llama.cpp Rust bindings is a multi-month engineering project with an ongoing maintenance tail (per-platform GGUF mass, quantisation drift, GPU offload, version pinning). Ollama gives us all of that for free behind a stable HTTP API on `127.0.0.1:11434`, with permissive licensing and active maintenance.

**Decision.** Ollama is the **primary** local-LLM runtime for MVP and V1.0. We talk to it over HTTP. Setup is guided: detect, install (with pinned hash), pull a curated default model. Embedded `llama.cpp` is deferred to post-V1.0 behind a feature flag, slotting in behind the same `LlmProvider` trait that wraps `OllamaClient`.

**Consequences.** One extra installation step for the user (mitigated by guided setup). A much smaller BooksForge installer. We rely on Ollama's stability — risk MVP-R1 in `../IMPLEMENTATION_PLAN.md §6` tracks this. Power users with existing Ollama installs get an optimal experience on day one. The architecture is unchanged: providers live behind a trait; switching back to embedded llama.cpp later is a config change.

### [DECISION-016] Bounded agent swarm replaces ad-hoc multi-step prompting

**Date:** 2026-05-06. **Status:** Confirmed for MVP and V1.0.

**Context.** Earlier docs described AI features as single-shot prompt presets (Sharpen, Continue, Rephrase, Summarise, Beta-read, Series consistency check). The user-product brief calls for "an automated agentic swarm architecture" with specialised roles spanning intake, outlining, drafting, editing, continuity, copyediting, and review. Without bounding, agentic systems are easy to over-promise and dangerous to under-implement.

**Decision.** We add a **bounded agent swarm** alongside the existing prompt-preset surface. The swarm is defined in `../AGENTS.md` with: a hard-coded agent registry; per-agent input/output JSON schemas; a versioned prompt template per agent with hash pinning; cross-cutting validators; an orchestrator with hard caps (≤8 agent calls, ≤10 minutes wall clock, ≤200k tokens, ≤3 retries per agent; no agent-spawned-agent; no manuscript-mutating tools); approval gates before any manuscript-mutating apply; a pre-edit snapshot before any apply. MVP includes **nine LLM agents** (Project Intake, Outline Architect, Memory Curator, Vocabulary Dictionary, Chapter Drafting [opt-in], Developmental Editor, Continuity, Copyeditor, Humanization) plus the always-present Orchestrator (controller); V1.0+ adds another nine: Book Strategy, Research Organizer, Chapter Planning, Line Editor, Style Guide, Fact-Check, Formatting, ePUB Export QA, Final Review. Refined further by `[DECISION-018]` (Memory + Vocabulary as first-class subsystems).

**Consequences.** A clearer mental model for users and reviewers ("six named agents do specific things") and a clearer engineering target. The orchestrator is non-trivial to implement correctly but is the only place agent risk is concentrated. We must resist over-applying the agent pattern — formatting and export remain deterministic and rule-based.

### [DECISION-007] PDF engine — Typst preferred, LaTeX fallback

**Date:** 2026-05-06. **Status:** Provisional.

**Context.** Pandoc supports both. Typst is faster and easier; LaTeX is what academic presses expect for source.

**Decision.** Default profiles use Typst when supported; academic profiles default to LaTeX.

**Consequences.** Two engines to test; Typst maturity is improving but younger; users who edit `.tex` files won't be confused.

### [DECISION-008] Plugin sandbox — WASM compute + isolated WebView UI

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Want third-party extensibility without compromising trust.

**Decision.** Compute plugins in `wasmtime` with WIT-typed host API and capability tokens. UI plugins in isolated Tauri WebViews with strict CSP.

**Consequences.** Strong sandbox; learning curve for plugin authors; reasonably ergonomic SDK.

### [DECISION-009] State management (frontend) — Zustand + Immer + TanStack Query

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Need predictable UI state and IPC caching.

**Decision.** Zustand for app state (with Immer middleware for ergonomics); TanStack Query for IPC reads with cache invalidation.

**Consequences.** Lightweight; well-supported; team avoids Redux ceremony.

### [DECISION-010] Localization library — ICU MessageFormat (via `intl-messageformat` and `icu4x` for Rust)

**Date:** 2026-05-06. **Status:** Provisional.

**Context.** Need full i18n with plurals, gender, RTL.

**Decision.** ICU MessageFormat as the source of truth; tooling to extract and validate keys.

**Consequences.** Slightly heavier than `react-intl`; correct support for languages we will need; consistent across UI and Rust.

### [DECISION-011] Telemetry vendor — self-hosted

**Date:** 2026-05-06. **Status:** Provisional.

**Context.** Privacy posture forbids third-party trackers; we want product analytics.

**Decision.** Self-hosted PostHog for telemetry; self-hosted Sentry-compatible service for crash reports; both opt-in.

**Consequences.** Operational cost of self-hosting; tighter privacy story; vendor-lock minimised.

### [DECISION-012] Linux package format priority

**Date:** 2026-05-06. **Status:** Provisional.

**Context.** Multiple package formats exist; we cannot ship all.

**Decision.** AppImage and Flatpak first; deb/rpm only on community demand.

**Consequences.** Wide reach (AppImage), modern sandboxing (Flatpak); some traditional Linux users underserved initially.

### [DECISION-013] Marketplace revenue split

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Need a sustainable plugin ecosystem.

**Decision.** 80/20 split (publisher 80, BooksForge 20). Stripe Connect for payouts.

**Consequences.** Aligns with App Store norms while being more publisher-friendly; covers infrastructure cost.

### [DECISION-014] Authentication — magic link primary, password optional

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Need account auth for Pro/Studio without imposing SSO complexity.

**Decision.** Magic link (email) primary; password (Argon2id) optional; 2FA (TOTP) optional for Studio.

**Consequences.** Lower friction; no SSO complexity in V1; SSO is V2 enterprise concern.

### [DECISION-015] License validation cadence

**Date:** 2026-05-06. **Status:** Confirmed.

**Context.** Privacy-friendly licensing without giving away the store.

**Decision.** Online re-validate every 30 days; offline grace 60 days; honour-system free tier.

**Consequences.** Some piracy is possible; the target market is paying customers anyway; clean offline-first UX.

### [DECISION-016] Snapshot retention defaults

**Date:** 2026-05-06. **Status:** Provisional.

**Context.** Snapshots can grow unboundedly.

**Decision.** Keep all manual snapshots forever; keep last 30 auto-snapshots; keep monthly archives.

**Consequences.** Predictable disk growth; easy mental model; user can override.

---

## C. Decision change procedure

A decision changes by adding a new entry **[DECISION-NNN-revB]** that supersedes the prior, with date, rationale, and migration plan. The prior is marked **Superseded by NNN-revB**. The change log in the README is updated. Any document that references the old decision is updated in the same PR.
