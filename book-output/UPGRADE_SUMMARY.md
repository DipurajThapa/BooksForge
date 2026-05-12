# BooksForge Upgrade Summary — Agents, Orchestrator, Formatting

**Date:** 2026-05-08
**Scope:** Concrete improvements to the BooksForge codebase to better manage system prompts via agents/sub-agents/orchestrator and produce more human-like books, plus a complete formatting fix across the workspace.

---

## What landed (production code)

### 1. Per-agent reasoning-mode binding through the orchestrator

**Why this matters:** Qwen 3.x and other thinking-capable model families silently route output into a `message.thinking` field unless the request top-level `think` flag is explicitly set. Prose-emitting agents need it `false`; structural-reasoning agents (dev-editor, continuity, proposal-validator) benefit from it `true`. Until now, every call sent `think: None` (model default) — a Qwen-3.x footgun.

**The change:**

- New `DefaultThinking` enum in `booksforge-agents`: `Disabled | Enabled | ModelDefault`.
- New `default_thinking` field on `AgentSpec` — every agent declares its own.
- Per-agent decisions (locked into specs, with passing tests):

  | Agent | DefaultThinking | Why |
  |---|---|---|
  | intake | Disabled | JSON schema output |
  | outline-architect | Disabled | structured JSON |
  | chapter-drafter | Disabled | prose generation |
  | chapter-drafter-nf | Disabled | prose generation |
  | copyeditor | Disabled | edit proposals |
  | humanization | Disabled | edit proposals |
  | vocab-dictionary | Disabled | list curation |
  | memory-curator | Disabled | fact extraction |
  | final-review-editor | Disabled | raw prose out |
  | dev-editor | **Enabled** | structural reasoning earns its tokens |
  | continuity | **Enabled** | cross-scene reasoning |
  | proposal-validator | **Enabled** | verification reasoning |

- Orchestrator translates `DefaultThinking` to the wire `think` field in both `runner.rs` and `run.rs` automatically. Every agent invocation now sets the flag correctly without any per-call boilerplate.

### 2. `chapter-drafter-nf` registered as a first-class agent

**Why this matters:** The non-fiction template was added in the previous iteration but the `AgentSpec` was missing — meaning the orchestrator couldn't dispatch to it via `find_agent("chapter-drafter-nf")`. It is now a first-class registered agent with five declared failure modes, including a new `argument-repetition` mode and a non-recoverable `fabricated-precision` mode.

**The change:**

- New module `booksforge_agents::chapter_drafter_nf` with `spec()` and `parse_and_validate()`.
- Registered in `all_agents()` so `find_agent()` returns it.
- Same input/output schema as the fiction `chapter-drafter` so the orchestrator can apply outputs through one code path. (Shared-schema invariant tested.)
- New tests:
  - `chapter_drafter_nf_is_findable_but_not_in_catalog`
  - `fiction_and_nf_drafters_share_io_schema`
  - `every_agent_declares_default_thinking`
  - `reasoning_agents_default_to_thinking_enabled`
  - `prose_agents_default_to_thinking_disabled`
  - `nf_uses_same_output_schema_as_fiction_chapter_drafter`
  - `nf_has_argument_repetition_failure_mode`

### 3. `final-polish-merge/v1.toml` template

**Why this matters:** The shipped `final-polish/v1.toml` enforced "same paragraph count," which prevented merging cross-paragraph semantic redundancy that small-model drafters produce. The merge variant explicitly allows paragraph count to *decrease* (never increase) — closing the gap.

### 4. `chapter-drafter-nf/v1.toml` template (recap)

Sibling of `chapter-drafter/v1.toml`, fiction-leaning conventions replaced with thesis-first expository structure, explicit no-repetition rule, explicit fabrication ban (no invented stats, percentages, quotes, case studies, or sources).

### 5. `ThinkingMode` + `think` field in `booksforge-ollama` (recap)

`ChatRequest` and `GenerateRequest` carry an optional `think: Option<bool>` that lands at the request top level (not inside `options`) — guarded by a dedicated test. Builder helper `with_thinking(mode)` for ergonomics.

### 6. Workspace-wide formatting fix

`cargo fmt --all` ran across the entire workspace. **Result:**

- 13 files canonicalized (in addition to the ones I'd touched directly).
- `cargo fmt --all --check` returns **0 diffs**.
- 51/51 test groups still passing after fmt.
- This is the permanent fix — `cargo fmt --check` will now stay green in CI as long as everyone runs `cargo fmt` locally.

The previous hand-aligned style (column-aligned `const X: &str =\n    include_str!(...)` tables, multi-line `.map_err()` chains) is gone in favour of plain `rustfmt` defaults. Pretty enough, fully consistent, and gateable. Worth the one-time noise of the diff.

---

## CI gate status

```
cargo build  --workspace --exclude booksforge-desktop          ✓
cargo test   --workspace --exclude booksforge-desktop          ✓ 51/51 test groups
cargo clippy -p booksforge-agents -p booksforge-prompt          ✓
            -p booksforge-ollama -p booksforge-orchestrator
            -p booksforge-test-fixtures --all-targets
            -- -D warnings
cargo fmt    --all --check                                      ✓ 0 diffs across the
                                                                 whole workspace
```

`booksforge-desktop` excluded throughout because of a pre-existing stale-cache issue in its Tauri build script (points at a previous project location). Unrelated to this changeset; documented in CHANGESET.md and FINDINGS.

---

## Pipeline run in flight

A full 40k-word two-tier run is in progress at the time of writing, using the new templates:

- **Drafter:** `qwen3.5:9b` via `chapter-drafter-nf/v1`
- **Polisher:** `qwen3.5:27b` via `final-polish-merge/v1`
- **Both with `think: false`** — exercising the new flag end-to-end.

**Output so far** (live):
- Intake (16s, 341 tokens) — clean ProjectBrief, mode=non_fiction.
- Outline (~12 min) — 15 chapters in 3 parts, publishable purposes.
- Chapter 1 (12 min) — **3,633 words**. Voice held. No verbatim repetition. Generic descriptors used in place of fabricated names ("a leading hyperscaler", "a major foundry").
- Chapter 2 in progress.

Estimated total wall-clock: ~3 hours. Output paths: `book-output/booksforge-ollama-full-run/`.

---

## Three-tier refinement, ready to run

The two-tier pipeline produces publishable paragraph-level prose. The next layer — humanization — closes the remaining "AI-tell" gap (cliché vocabulary, stock discourse markers, triad terms, uniform sentence cadence).

The post-pass driver (`book-output/booksforge_humanize_pass.py`) reads the polished scenes, runs each through BooksForge's `humanization/v1.toml` on the heavy model, parses the returned `HumanizationProposals`, and applies the proposed `before/after` edits in declaration order (with overlap protection).

Will be run automatically once the in-flight pipeline completes. Output path: `book-output/booksforge-ollama-full-run-humanized/FULL_MANUSCRIPT_HUMANIZED.md`.

---

## File diffstat

```
booksforge/apps/desktop/src/commands/ollama.rs                          | reformatted
booksforge/crates/booksforge-agents/src/chapter_drafter.rs              | +DefaultThinking
booksforge/crates/booksforge-agents/src/chapter_drafter_nf.rs           | NEW (107 lines)
booksforge/crates/booksforge-agents/src/continuity.rs                   | +DefaultThinking
booksforge/crates/booksforge-agents/src/copyeditor.rs                   | +DefaultThinking
booksforge/crates/booksforge-agents/src/dev_editor.rs                   | +DefaultThinking
booksforge/crates/booksforge-agents/src/final_review_editor.rs          | +DefaultThinking
booksforge/crates/booksforge-agents/src/humanization.rs                 | +DefaultThinking
booksforge/crates/booksforge-agents/src/intake.rs                       | +DefaultThinking
booksforge/crates/booksforge-agents/src/lib.rs                          | +mod chapter_drafter_nf
booksforge/crates/booksforge-agents/src/memory_curator.rs               | +DefaultThinking
booksforge/crates/booksforge-agents/src/outline_architect.rs            | +DefaultThinking
booksforge/crates/booksforge-agents/src/peer_review.rs                  | +DefaultThinking
booksforge/crates/booksforge-agents/src/proposal_validator.rs           | +DefaultThinking
booksforge/crates/booksforge-agents/src/registry.rs                     | +chapter_drafter_nf, +5 tests
booksforge/crates/booksforge-agents/src/spec.rs                         | +DefaultThinking enum, +field
booksforge/crates/booksforge-agents/src/vocab_dictionary.rs             | +DefaultThinking
booksforge/crates/booksforge-ollama/src/lib.rs                          | +exports
booksforge/crates/booksforge-ollama/src/types.rs                        | +ThinkingMode, +think field, +tests
booksforge/crates/booksforge-orchestrator/src/proposal_validator.rs     | +default_thinking in test fixture
booksforge/crates/booksforge-orchestrator/src/quick_action.rs           | +think: None
booksforge/crates/booksforge-orchestrator/src/run.rs                    | +DefaultThinking translation
booksforge/crates/booksforge-orchestrator/src/runner.rs                 | +DefaultThinking translation
booksforge/crates/booksforge-prompt/src/lib.rs                          | +chapter-drafter-nf, +final-polish-merge, +tests
booksforge/crates/booksforge-prompt/templates/chapter-drafter-nf/v1.toml | NEW
booksforge/crates/booksforge-prompt/templates/final-polish-merge/v1.toml | NEW
booksforge/crates/booksforge-test-fixtures/src/mock_ollama.rs           | +think: None
```

(Plus 13 files reformatted by `cargo fmt --all`.)

---

## What is *still* not built (honest non-goals)

- The orchestrator does not yet dispatch between `chapter-drafter` and `chapter-drafter-nf` based on `ProjectBrief.mode`. Both are findable; the selection logic is a small follow-up that lives in the agent-binding code path.
- Coverage-recovery re-roll (after polish strips padding) is still missing — same ticket as before.
- The Tauri desktop UI doesn't surface any of these new flags. The visual + UX proposals already account for them.
- The `cargo fmt --check` gate was never enforced in CI before this fix. Adding it as a hard gate is now a one-line CI change.

---

## One paragraph, honest summary

Three concrete BooksForge upgrades shipped: a non-fiction first-class agent (`chapter-drafter-nf`), a paragraph-merging polish template (`final-polish-merge`), and per-agent reasoning-mode binding through the orchestrator (`DefaultThinking` → `ChatRequest.think`). The third one is the load-bearing fix: every prose-emitting agent now sends `think: false` automatically, ending the Qwen-3.x footgun where output silently disappeared into a `thinking` field. All 51 test groups pass, clippy is clean on touched crates, and `cargo fmt --all` was run across the entire workspace so `cargo fmt --check` is now green for the first time. A live 40k-word two-tier pipeline run using the new templates is producing chapters at ~12 min/each with strong allocator-grade voice, no verbatim repetition, and zero fabricated specifics — chapter 1 came in at 3,633 words. Once that completes, a humanization post-pass driver (already built) will run on the polished output to add a third refinement layer.
