# Workflows & Dataflows — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

This document captures the high-value end-to-end flows. Each flow is described in prose, with a sequence-diagram-style step list, and the data shapes involved. Visual diagrams are in `diagrams/`.

---

## 1. Project lifecycle (the macro flow)

A project moves through six conceptual states: **Created → Drafting → Revising → Validated → Exported → Archived**. State is implicit (derived from data) rather than stored as an enum, because users routinely loop back (e.g., revise after a failed validator pass).

### 1.1 Create
The user picks a template, names the project, and chooses a folder. BooksForge writes the bundle atomically: temp directory → populate manifest, scaffold tree, scaffold front-matter and back-matter from template, initialise SQLite — then `rename` to final path. Reference: FR-PROJ-001/004.

### 1.2 Draft and revise
The user opens scenes, types, organises, snapshots, and uses AI assistance. Each commit-of-content path: keystrokes → ProseMirror state → debounced commit (5s default) → IPC `scene.save` → Rust storage layer → SQLite write + Markdown mirror update + `scene_content.hash` recompute + manuscript-tree-hash invalidation. See §3 below.

### 1.3 Validate
Before export (and on demand) the validator engine runs. It produces an Issue list. Errors block export; warnings prompt; info is silent. See §6.

### 1.4 Export
The user picks a target (DOCX manuscript, EPUB-3, KDP-print PDF, etc.). BooksForge: takes a snapshot tagged `pre_export`, runs validators that apply to the target, builds a Pandoc-AST representation of the project, runs the Pandoc sidecar, applies post-processors, writes to `exports/`, and records a row in `exports`. See §7.

### 1.5 Archive
Optional. The bundle is zipped and metadata moved to an "archive" view. Recoverable.

---

## 2. Workflow: starting a new project (sequence)

```
User                  UI (React)            Tauri host         Storage          Template engine
 │  click "New"         │                     │                   │                  │
 ├─►choose template ───►│                     │                   │                  │
 │                      ├─► invoke `project.create({path,template,meta})`           │
 │                      │                     ├──► resolve template (built-in or plugin)──►│
 │                      │                     │                   │                  │
 │                      │                     ├──► template.scaffold(meta) ◄─────────┤
 │                      │                     │      returns { tree, manifest_seed, scenes[] }
 │                      │                     ├──► fs.create_bundle_atomic(path)    │
 │                      │                     │     (tmp dir → write → rename)      │
 │                      │                     ├──► storage.initialize_db(path)─────►│
 │                      │                     ├──► storage.populate_tree(tree)─────►│
 │                      │                     ├──► storage.populate_scenes(scenes)─►│
 │                      │                     ├──► fs.write_manuscript_mirror()    │
 │                      │   ProjectCreated    │                   │                  │
 │                      │◄────────────────────┤                   │                  │
 │   project opens      │                     │                   │                  │
 │◄─────────────────────┤                     │                   │                  │
```

**Failure modes handled at each step:** template not found → typed error, prompt user; path not writable → typed error; partial write → temp dir abandoned, no half-state on disk.

---

## 3. Workflow: edit loop (the hot path)

This loop runs thousands of times per session. Latency budget: 30 ms p95 from keystroke to next keystroke being acceptable.

```
keystroke → ProseMirror state → React re-render of changed blocks only (virtualised list)
        ↓ (debounced 250 ms for live word count)
        word-count + dirty flag → Zustand → status bar updates
        ↓ (debounced 5 s after last keystroke OR on blur)
        commit → IPC `scene.save` →
            Rust:
              • acquire writer lock for project
              • compress pm_doc_json (zstd)
              • compute new content hash (blake3)
              • SQLite UPDATE scene_content SET pm_doc_json=?, hash=?, updated_at=?
              • UPDATE nodes SET updated_at=? WHERE id=?
              • write Markdown mirror file (atomic: tmp + rename)
              • emit `scene.saved` event with new hash
            ←
        UI updates "saved" indicator
```

Notes: the Markdown mirror is written *after* the SQLite commit (best-effort durability). If the mirror write fails, we log and re-queue; the SQLite write is the source of truth. The mirror is rebuildable from SQLite at any time.

---

## 4. Workflow: AI request (local model, default path)

```
1. User selects text and chooses preset "Sharpen prose"
2. UI calls `ai.suggest({ scope, preset, options })`
3. Rust orchestrator:
   a. Verify project AI capability is enabled (FR-AI-004)
   b. Build prompt context: scope text + entity bible rows referenced + tone preset
   c. Apply prompt template (versioned, see 08-AI §5)
   d. Call LlmProvider (resolved from settings: embedded llama.cpp by default)
   e. Stream tokens back via event `ai.token`
   f. On completion: record `ai_calls` row; close stream
4. UI shows suggestion in side panel with diff view
5. User accepts/rejects/regenerates
   - On accept: pre-AI-edit auto-snapshot taken (FR-SNAP-003), then content updated
   - On reject: nothing persists except the ai_calls audit row
```

**Cancellation:** UI sends `ai.cancel(jobId)`; the orchestrator drops the cancellation token; the streaming task aborts. Any partial token output is held in UI until the user dismisses or pins it.

**Privacy invariants:** for the local path, no network call ever happens. Asserted in tests by mocking the network layer to fail and verifying the local path completes.

---

## 5. Workflow: AI request (cloud model, opt-in)

Identical to §4 except the LlmProvider is a Cloud variant. Additional steps:

The orchestrator first computes a token-cost estimate; if it exceeds the user's per-call budget the UI prompts. The orchestrator strips PII from the prompt according to the configured redaction policy (default: scrub paths and email addresses; never scrubs manuscript content because that's the point — but the user is shown exactly what is sent in a "context preview" UI per FR-AI-008). The HTTPS request goes to the configured provider with timeout, retry-with-jitter on transient errors, and a hard stop on 4xx. The response is cached locally for the session if the user wants to revisit; nothing is written to a vendor-side cache.

**Provider abstraction:** the `LlmProvider` trait erases vendor differences. Anthropic, OpenAI, OpenRouter, and Mistral implementations live in `booksforge-ai-runtime/cloud/`. Each implementation is rate-limit-aware and emits a typed error.

---

## 6. Workflow: validation run

```
User triggers full validation OR pre-export gate
 → Orchestrator collects applicable validators (built-in + project-enabled plugin validators)
 → For each validator (parallel where pure):
     • Check cache: scope_hash unchanged → reuse last run
     • Else: spawn task with read-only ProjectView
     • Validator returns Issue[]
     • Persist `validator_runs` + `validator_issues`
 → UI receives stream of issues; renders panel with counts and click-to-source
 → If any error issues exist and user is in pre-export gate → block, surface fix actions
```

Validators **must be pure functions of inputs**. The validator engine asserts determinism by hashing inputs and outputs in test fixtures. Non-deterministic validators are rejected at plugin install time (capability `nondeterministic` required if needed; not granted to marketplace plugins by default).

---

## 7. Workflow: export pipeline

```
User → Export → choose profile (e.g., "KDP-eBook EPUB-3")
 → Orchestrator:
   1. Take snapshot tagged `pre_export`
   2. Resolve profile (template + target + post-processors)
   3. Run profile-required validators
   4. If validator errors AND user setting "block on errors" → abort, show panel
   5. Build BooksForge-AST (canonical doc tree with resolved cross-refs and citations)
   6. Transform BooksForge-AST → Pandoc-AST JSON
   7. Spawn pandoc sidecar:
        pandoc --from=json --to=epub3 --output=… --resource-path=… --css=… --epub-cover-image=…
   8. Receive output
   9. Post-processors (per format):
        • EPUB: epubcheck pass → fix common issues → re-validate
        • PDF: font subsetting if not embedded; bleed-and-trim if KDP
        • DOCX: tracked-changes preserved; comments preserved
   10. Write final file to `exports/<timestamp>-<profile>.<ext>`
   11. Record `exports` row with template version, validators run, and output hash
 → UI: show success with file link and validator summary
```

**Reproducibility:** the same inputs (project hash + template version + app version + profile) must produce a byte-identical output. We pin Pandoc version per app release and record it in the export row. (Caveat: PDF generators sometimes embed timestamps; we use Pandoc's `--metadata-file` with a fixed date when reproducibility mode is on.)

---

## 8. Workflow: snapshot create / restore

**Create.** Triggered manually, on schedule, before AI-applied edit, or before migration. The engine walks the affected scope, hashes each node's canonical form, dedupes against `snapshots/objects/` (only writes objects whose hash is new), writes a tree object listing `(node_id → object_hash)`, and inserts a `snapshots` row. Time complexity O(N) in nodes; storage typically O(M) in *changed* nodes after dedupe.

**Restore.** User picks a snapshot, scope, and mode (replace whole / replace selected nodes / restore alongside as a draft). Engine reads the tree object, reconstructs nodes, and applies a transactional diff to SQLite. A second snapshot tagged `pre_restore` is automatically taken first.

---

## 9. Workflow: plugin install (sideload)

```
User → Install plugin → choose .booksforge-plugin file
 → Plugin host:
   1. Verify package signature if from marketplace; warn if sideload (still allowed)
   2. Parse manifest; extract requested capabilities (e.g., read-manuscript, network-domain:zotero.org)
   3. Show capability prompt with explanation of each capability
   4. On approve: copy plugin to user data dir, register in plugin_installs
   5. On per-project enable: add to `plugins/enabled.toml`
   6. Plugin runtime loads WASM module / UI WebView lazily on first use
```

Capability prompt includes a "Why does this plugin need this?" link to the manifest's per-capability rationale.

---

## 10. Dataflow: write → save → mirror → snapshot

This is the canonical hot path for manuscript durability:

```
ProseMirror state (in-memory)
        ↓  (debounced 5s)
Layer 2: scene.save command
        ↓
Layer 3: ContentService.update_scene(node_id, pm_json, hash)
        ↓
Layer 4: Storage adapter (SQLite UPDATE) ──► commit
        ↓                                 ↘
        ↓                                  scene_saved event ─► UI status
        ↓
Layer 4: FS adapter (Markdown mirror, atomic rename)
        ↓
        (every N saves OR every M minutes)
Snapshot scheduler: take auto snapshot, dedupe writes
```

If the device powers off between SQLite commit and Markdown mirror, the mirror is reconstructed at next launch.

If the device powers off mid-SQLite commit, WAL replay handles it.

If both fail catastrophically, the most recent snapshot (≤ snapshot interval old) is the recovery surface.

---

## 11. Dataflow: AI prompt assembly

```
[scope text]            ─┐
[entity bible matches]  ─┤
[tone preset]           ─┼─► Context selector ─► Prompt template ─► LlmProvider ─► Tokens
[user instruction]      ─┤                              ▲
[plugin prompt overlay] ─┘                              │
                                                  Provider-specific
                                                  formatting (chat, completion)
```

The prompt template is a typed structure with named slots — never string interpolation. Templates are versioned and hashed; the hash is stored on every `ai_calls` row so we can reproduce a call later for audit or A/B testing.

---

## 12. Dataflow: import (DOCX example)

```
DOCX file
   ↓
pandoc --from=docx --to=json (run as sidecar)
   ↓
Pandoc-AST JSON
   ↓
BooksForge importer:
   • Map Pandoc nodes to BooksForge nodes (Heading lvl 1 → Chapter, lvl 2 → Scene break, etc.)
   • Extract footnotes to `references` table
   • Extract images to `assets/` (deduped by hash)
   • Extract tracked changes (if present in DOCX)
   • Extract comments
   • Infer parts/chapters from heading hierarchy
   ↓
Project structure proposal → user reviews and confirms
   ↓
Atomic write into project bundle
```

**Safety:** an import never overwrites an existing project. It always creates a new bundle or a new branch within the bundle (V1.5 feature).

---

## 13. Dataflow: encryption (when enabled)

```
Plain content ─► (per-blob random nonce) ─► AES-256-GCM(key, nonce, plaintext) ─► ciphertext + tag
                              ▲
                              │
        master_key = Argon2id(passphrase, salt, params)
        passphrase entered at project open; held in OS keyring (optional) or memory only
```

The SQLite database is encrypted at the page level via SQLCipher. Asset files and snapshot objects are individually encrypted with the same master key.

---

## 14. Cross-flow invariants

These hold across every flow above and are asserted by tests:

A user-initiated change is durable within `autosave_interval` of the change. Every AI-applied edit produces a snapshot. Every export produces an immutable record. Validators are pure of inputs. Plugins cannot read or write outside their capability set. Network calls only happen from explicitly user-enabled features. PII redaction is always applied to logs at the sink, not the call site.
