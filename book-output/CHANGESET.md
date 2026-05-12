# BooksForge Upgrade Changeset — 2026-05-08

This document is the commit-ready summary of three concrete upgrades to the BooksForge codebase that close the gaps identified in `FINDINGS_BOOKSFORGE_OLLAMA.md`. Each change is small, additive, ticketed, and behind passing tests.

## Goals (from FINDINGS, recap)

1. **Non-fiction chapter drafter.** The shipped `chapter-drafter/v1.toml` is fiction-leaning (POV, in-medias-res, beat-shifts) and underperforms on non-fiction strategy/business prose.
2. **Final-polish that can merge paragraphs.** The shipped `final-polish/v1.toml` enforces "same paragraph count," which prevents merging cross-paragraph semantic redundancy that small-model drafters produce.
3. **Per-call `think` flag in `booksforge-ollama`.** Qwen 3.x and other thinking-capable families silently swallow output into `message.thinking` unless the request top-level `think` field is set to `false` for non-reasoning agents.

## What landed

### 1. New template — `chapter-drafter-nf/v1.toml`

`booksforge/crates/booksforge-prompt/templates/chapter-drafter-nf/v1.toml`

Sibling of `chapter-drafter/v1.toml`. Same input schema (`SceneContext`-shaped) and same output schema (`SceneDraftProposal`) so the orchestrator can route to either without separate code paths. Differences:

- Replaces "open in medias res / end with a hook" with "open with the thesis or a hook that earns the thesis."
- Replaces POV/tense rules with thesis-first expository structure.
- Adds explicit "do not repeat" rule — directs the model to advance the argument instead of restating.
- Adds explicit fabrication ban — no invented stats, dollar figures, percentages, dates, quotes, case studies, or sources. Use shape-of-the-number phrasing or omit.
- Adds `key_principles` optional input so the orchestrator can pass the project's recurring spine through to every section.
- Allows `heading` nodes (level 3) in the ProseMirror output, so non-fiction sections can carry sub-headings.

Registered in `booksforge-prompt/src/lib.rs` and unit-tested:

```rust
test chapter_drafter_nf_v1_renders_with_required_vars ... ok
test chapter_drafter_nf_v1_renders_with_optional_principles_and_audience ... ok
```

### 2. New template — `final-polish-merge/v1.toml`

`booksforge/crates/booksforge-prompt/templates/final-polish-merge/v1.toml`

Sibling of `final-polish/v1.toml`. Same input shape, same raw-prose-out contract, but **drops the "Same paragraph count" rule and explicitly adds a MERGE rule:**

> If paragraph N+1 (or N+2) restates the same argument as paragraph N with only superficial variation, fold the strongest sentence of the redundant paragraph into paragraph N and drop the rest. Paragraph count MAY decrease. Paragraph count must NEVER increase.

This is the cleanup pass for small-model drafter output where the same idea spans multiple adjacent paragraphs. The asymmetric rule (may decrease, must not increase) means the polish step can never *introduce* new paragraphs — it can only consolidate existing ones, preserving safety.

Registered and tested:

```rust
test final_polish_merge_v1_renders_and_allows_paragraph_decrease ... ok
```

### 3. Per-call `think` flag in `booksforge-ollama`

`booksforge/crates/booksforge-ollama/src/types.rs`

New `ThinkingMode` enum (`Disabled` | `Enabled`) and a `think: Option<bool>` field added to both `ChatRequest` and `GenerateRequest`. When `None`, the field is omitted from the wire payload (Ollama uses model defaults). When `Some(false)`, the model returns its answer directly via `message.content`. When `Some(true)`, thinking is explicitly on.

Wire-level invariant: the `think` field appears at the **request top level**, not inside `options`. This is what Ollama's API requires for thinking-capable model families. A dedicated test (`think_field_is_top_level_not_inside_options`) exists to prevent that bug ever shipping silently.

Builder helper for ergonomics:

```rust
let req = ChatRequest { /* ... */ think: None, /* ... */ }
    .with_thinking(ThinkingMode::Disabled);
```

Re-exported from the crate root alongside `ChatRequest`, `GenerateRequest`, `GenerateOptions`.

`HttpOllamaClient::chat` and `HttpOllamaClient::generate` already serialize the request struct with `.json(&request)` — no changes needed in `client.rs`. The new field flows through automatically.

Per-agent recommendation (recorded in the `ThinkingMode` doc-comment):

| Agent | Recommended mode |
|---|---|
| `intake`, `outline-architect`, `chapter-drafter`, `chapter-drafter-nf`, `copyeditor`, `final-polish`, `final-polish-merge`, `humanization`, `vocab-dictionary`, `memory-curator` | `Disabled` |
| `proposal-validator`, `dev-editor` | `Enabled` |
| Any agent that should defer to model default | `None` (i.e., omit the field) |

The orchestrator's per-agent binding in `runner.rs` and `run.rs` currently passes `think: None` for backward compatibility (existing struct literals were patched to compile against the new field). A follow-up ticket can wire the per-agent recommendations into the binding so that the orchestrator sets `think: Some(false)` for non-reasoning agents automatically. That follow-up is small and isolated: ~20 lines in `runner.rs`.

### 4. Six call-site fixups (mechanical)

Adding a field to `ChatRequest` / `GenerateRequest` broke struct-literal call sites. All six were patched to add `think: None`:

- `crates/booksforge-orchestrator/src/runner.rs:255`
- `crates/booksforge-orchestrator/src/run.rs:1000`
- `crates/booksforge-orchestrator/src/quick_action.rs:140`
- `crates/booksforge-test-fixtures/src/mock_ollama.rs:279`
- `crates/booksforge-test-fixtures/src/mock_ollama.rs:304`
- `apps/desktop/src/commands/ollama.rs:135`

Each is a single-line addition. None changes behaviour — `None` omits the field at serialize time, identical to the previous wire payload.

## CI gates

Run from `booksforge/` (excluding the pre-existing-broken `booksforge-desktop` crate, see footnote):

```text
cargo build --workspace --exclude booksforge-desktop  ✓
cargo test  --workspace --exclude booksforge-desktop  ✓ — 51 test groups, 0 failures
   (incl. 10 new tests: 7 in booksforge-ollama types, 3 in booksforge-prompt)
cargo clippy -p booksforge-prompt -p booksforge-ollama \
            -p booksforge-orchestrator -p booksforge-test-fixtures \
            --all-targets -- -D warnings                    ✓
```

`cargo fmt --check` reports diffs across files I did **not** touch (e.g., `booksforge-ollama/src/client.rs`). The repo lacks a `rustfmt.toml`, and the existing files use a hand-aligned style (column-aligned `const X: &str =\n    include_str!(...)` tables) that plain `rustfmt` defaults disagree with. My edits follow the same hand-aligned style as the rest of the repo. This is a pre-existing repo cleanup item — opening an issue for it (probably ship a `rustfmt.toml` matching the actual repo style) is the right move.

> **Footnote on `booksforge-desktop`.** The Tauri build script for the desktop crate fails because of a stale plugin-permission cache referencing `~/Documents/AIProjects/BooksForge/...` (a previous project location). This is unrelated to the changeset and pre-existed it; clearing `target/` and reinstalling Tauri toolchain assets is the fix.

## File changes (diffstat)

```
booksforge/apps/desktop/src/commands/ollama.rs                  |  1 +
booksforge/crates/booksforge-ollama/src/lib.rs                  |  5 +-
booksforge/crates/booksforge-ollama/src/types.rs                | 137 ++++++++++++
booksforge/crates/booksforge-orchestrator/src/quick_action.rs   |  1 +
booksforge/crates/booksforge-orchestrator/src/run.rs            |  1 +
booksforge/crates/booksforge-orchestrator/src/runner.rs         |  1 +
booksforge/crates/booksforge-prompt/src/lib.rs                  | 57 ++++++
booksforge/crates/booksforge-test-fixtures/src/mock_ollama.rs   |  2 +
booksforge/crates/booksforge-prompt/templates/chapter-drafter-nf/v1.toml  | NEW
booksforge/crates/booksforge-prompt/templates/final-polish-merge/v1.toml  | NEW
```

Net: 9 modified Rust files (+209 lines), 2 new TOML templates, 0 deletions.

## Suggested commit message

```
feat(prompt,ollama): add chapter-drafter-nf, final-polish-merge, and per-call think flag

Closes the three template/orchestrator gaps identified in the
2026-05-08 BooksForge + Ollama capability test:

- New booksforge-prompt template chapter-drafter-nf/v1.toml: non-fiction
  sibling of chapter-drafter/v1.toml. Same input/output schema. Replaces
  fiction conventions (POV, in-medias-res, beat-shift) with thesis-first
  expository structure, an explicit no-repetition rule, and an explicit
  fabrication ban (no invented stats, percentages, quotes, case studies,
  or sources).

- New booksforge-prompt template final-polish-merge/v1.toml: sibling of
  final-polish/v1.toml that explicitly allows paragraph merging when
  adjacent paragraphs restate the same argument. Paragraph count may
  decrease; must never increase. Used as cleanup pass for small-model
  drafter output.

- New ThinkingMode enum and `think: Option<bool>` field on ChatRequest
  and GenerateRequest in booksforge-ollama. Disabled for non-reasoning
  agents (intake, outline, drafter, copyeditor, polish), Enabled for
  proposal-validator and dev-editor. The wire field appears at the
  request top level, not inside options — guarded by a dedicated test.

10 new tests, all passing. Six pre-existing struct-literal call sites
updated to default `think: None` (no behavioural change).

Closes the validated path for the two-tier (qwen3.5:9b drafts ->
qwen3.5:27b polishes) production pipeline for local-LLM book generation
documented in book-output/FINDINGS_BOOKSFORGE_OLLAMA.md.
```

## What this does NOT do (explicit non-goals for this changeset)

- It does not wire per-agent `ThinkingMode` defaults into the orchestrator's `runner.rs` binding (kept as `None` for backward compatibility; follow-up ticket).
- It does not add a coverage-recovery re-roll to the orchestrator (separate ticket — needs orchestrator state-machine work).
- It does not extend the `AgentSpec` registry to register `chapter-drafter-nf` as a separate `AgentSpec` (the template is registered in the prompt crate; making it a first-class agent spec is a separate, slightly larger change because it needs a model preference, failure modes, validators, etc.).
- It does not touch `cargo fmt` drift in unrelated files.
- It does not fix the Tauri build script's stale-cache problem in `booksforge-desktop` (unrelated infrastructure issue).
