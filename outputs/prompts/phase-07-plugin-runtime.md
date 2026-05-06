# Phase 07 — Plugin runtime

## Goal

Ship the plugin runtime: WASM compute plugin host (wasmtime), isolated UI plugin WebViews, capability prompt UX, plugin SDK in Rust + TypeScript, plugin CLI tool, sideload UX. Ship three first-party plugins as proof: Romance Trope Validator, Save the Cat panel, Mystery Pack.

## Pre-conditions

Phase 05 (validators API stable) and Phase 06 (entity bible stable) merged.

## Inputs

1. `../_deep/07-plugin-architecture.md` — entire document.
2. `../_deep/06-security-privacy-compliance.md` — section 5 (sandbox).
3. `../_deep/02-FSD-functional-specifications.md` — section 10 (FR-PLUG).

## Deliverables

### 1. `booksforge-plugin-host`

`wasmtime` + WASI-preview2. Capability tokens. Resource caps (256 MB, 30 s wall, 10 s CPU). WIT-typed host interface. Capability-gated host-call dispatcher.

### 2. UI plugin runtime

Tauri isolated WebView per UI plugin with its own origin (`booksforge://plugin/<id>/`). Strict CSP. Postmessage IPC mediated by host with capability checks.

### 3. Plugin SDKs

Rust SDK as a published crate `booksforge-plugin-sdk` (dependency of compute plugins). TypeScript SDK as `@booksforge/plugin-sdk` for UI plugins. Both wrap the WIT-defined host API.

### 4. Plugin CLI

`booksforge plugin new --type validator|template|prompt-pack|ui-panel --name x` scaffolds. `plugin lint`, `plugin test` (against fixture projects), `plugin pack`, `plugin publish`. CLI lives in `tools/plugin-cli/` (Rust binary).

### 5. Capability prompt UX

On install: list every requested capability with the plugin's rationale. Approve / Cancel. On per-project enable: a smaller per-capability re-prompt only if the user has previously revoked.

### 6. Sideload flow

Install from a `.booksforge-plugin` file. Show "Unsigned plugin" warning when not marketplace-signed. Refuse to install plugins whose `compatibility.booksforge_min/max` does not match the running version.

### 7. Three first-party plugins

- **Romance Trope Validator** (compute / validator): scans for missing romance beats per the project's Romance template; capabilities: `read-manuscript`, `write-validator-issue`.
- **Save the Cat Panel** (UI panel): renders the 15-beat sheet over the outline view, lets the user tag scenes against beats, shows word-count rollups; capabilities: `ui-panel-side`, `read-manuscript`, `write-suggestions`.
- **Mystery Pack** (template + validator + prompt-pack): a Cosy Mystery template, a "fair-play clue" validator, and a Mystery Sharpen prompt overlay.

These prove all four current plugin types work end-to-end and serve as the SDK example library.

### 8. Tests

- Sandbox adversarial fixtures (allocator bombs, infinite loops, attempted host-call from outside capabilities, large-result attacks). All result in killed plugin task and typed error.
- Capability prompt UX flow E2E.
- Sideload flow with signed and unsigned packages.
- Each first-party plugin runs end-to-end.
- WASM resource caps verified (over-allocation killed).

### 9. Documentation

- `docs/plugin-sdk/getting-started.md` — your-first-plugin tutorial (validator).
- `docs/plugin-sdk/capability-list.md` — reference.
- `docs/plugin-sdk/manifest.md` — `plugin.toml` reference.
- In-app help: "Plugins overview", "Installing plugins safely", "Managing plugin permissions".

## Guard-rails specific to this phase

**[GUARD-P7-1]** Default-deny on every plugin → host call.

**[GUARD-P7-2]** No host call exists that doesn't go through the capability dispatcher.

**[GUARD-P7-3]** UI plugins cannot directly access the editor's ProseMirror state — they emit suggestions / comments via host API.

**[GUARD-P7-4]** Resource caps are configurable but cannot be raised by a plugin's own manifest.

**[GUARD-P7-5]** The capability prompt is mandatory; bypassing it (e.g., a "developer mode skip") is forbidden in release builds.

## Acceptance criteria

1. The three first-party plugins ship and work.
2. Adversarial fixture suite passes — sandbox holds.
3. Capability prompt is shown and grants are persisted.
4. Sideload an unsigned plugin shows the "Unsigned" warning.
5. Plugin CLI scaffolds, tests, and packs a new validator end-to-end.

## Review gate

- `cargo deny` and dependency review for new deps (wasmtime, wit-bindgen).
- The host API is fully typed (no `any` / `Box<dyn Any>`).
- A new capability requires updating: capability list, prompt UX, docs, test fixtures.

## Out of scope

- Marketplace (Phase 10).
- Importer/exporter plugin types (Phase 13).
- Plugin update flow (Phase 10).

## When you finish

PR title `Phase 07: Plugin runtime`.
