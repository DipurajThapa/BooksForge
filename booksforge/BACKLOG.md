# BooksForge — Backlog

Tracks every deferred item flagged during MZ-01 → MZ-08 implementation, plus
the full set of MVP areas (M1–M6) not yet started. Each entry is sized so
that it can land as a focused PR and points back at the spec it derives from.

> Source rules: items are added when (a) we explicitly deferred a gap during
> a milestone audit, (b) the implementation pack lists a feature we haven't
> started, or (c) a hidden risk surfaced during testing. Items leave only by
> being implemented, by an ADR explicitly removing them from scope, or by an
> upstream spec change.

Last refreshed: 2026-05-07 (Phases 1–4 + Turns A & B — Markdown export,
six quick-action presets, EditorToolbar, word-count rollups, InspectorPanel,
focus mode, Memory + Vocabulary CRUD with shipped starter dictionaries,
17 manuscript validators (incl. KDP-metadata) with pre-export gate +
scope-hash cache, transactional snapshot capture, PreRestore trigger,
snapshot diff UI, drag-reorder Binder, hourly auto-snapshots, word-level
visual diff, vitest harness, memory/vocab IPC + Knowledge panel.
Closed: A1, A2, A3, A5, A6, D5, D6, D7, G3-metadata, J3, K1, K2, K4).

---

## A. Hot follow-ups (touch first when working in the same area)

### A1. `PreRestore` snapshot trigger variant ✅ CLOSED 2026-05-07 (Turn B)
- New `SnapshotTrigger::PreRestore` variant; migration
  `0008_snapshot_trigger_pre_restore.sql` extends the `snapshots.trigger`
  CHECK constraint. `SnapshotService::restore` writes the safety snapshot
  with the new trigger. Parser/str helpers updated in storage + the
  Tauri command. Timeline UI can now filter user-initiated `manual`
  snapshots from automatic safety captures.

### A2. Transactional snapshot capture ✅ CLOSED 2026-05-07 (Turn B)
- New `StorageRepository::list_nodes_with_scene_content_consistent()`
  wraps both reads in `BEGIN IMMEDIATE` so an autosave can't slip in
  between. `SnapshotService::create` now uses this single round-trip
  + an in-memory `HashMap` lookup, eliminating the torn-capture race
  and several N+1 round-trips in the process.

### A3. Diff command UI surface ✅ CLOSED 2026-05-07 (Turn B)
- SnapshotsPanel grew a Compare button per row; pick A then B and the
  node-level diff renders inline (added/removed/changed badges, scrollable
  list). Wraps the existing `snapshot_diff` IPC.

### A4. Markdown mirror snapshotting policy ✅ CLOSED 2026-05-07 (Turn N — policy documented)
- `crates/booksforge-fs/src/markdown_mirror.rs` doc-comment now spells
  out the contract: the mirror is a *projection*, not state.  The
  authoritative scene content (`pm_doc`) IS in snapshots; the mirror
  is a courtesy for external tooling and is regenerated on demand.
- Restore semantics documented: `SnapshotService::restore` should
  re-run `write_mirror` for every restored scene so on-disk Markdown
  matches the freshly-restored DB.  Tracked as a follow-up to wire
  that re-emit; today's behaviour leaves the mirror potentially
  stale until the next save touches each scene (logged but not
  silently dropped).

### A4.legacy. Markdown mirror snapshotting policy
- **Why:** `manuscript/*.md` files are not captured in snapshots — restore
  leaves them stale. Currently undocumented.
- **Where:** decision doc in `outputs/DATA_MODEL.md` or `EXPORT_EPUB_SPEC.md`,
  then either rebuild the mirror after restore or include it in the snapshot.
- **Trigger:** before any user can be reasonably expected to rely on the
  Markdown mirror as ground truth (mid-M1).

### A5. Restore error surface includes safety-snapshot id ✅ CLOSED 2026-05-07 (Turn A)
- Added `SnapshotError::RestoreFailedAfterSafety { safety_id, source }`
  variant; `SnapshotService::restore` wraps every post-safety-snapshot
  failure in it so the UI can show "you can undo via snapshot {safety_id}".
- Integration test: `restore_failure_after_safety_carries_safety_id` in
  `crates/booksforge-snapshot/tests/apply_restore.rs`.

### A6. `ai_suggest` background task auto-cleanup ✅ CLOSED 2026-05-07
- Closed by Phase 1 — the spawned task now calls
  `app_clone.state::<AppState>().drop_job(&job_id_for_task).await` after
  emitting the terminal `:done` event.

### A7. `restore_entry` `deleted_at` reset ✅ CLOSED 2026-05-07 (Turn N — semantics documented)
- The `upsert_node` SQL clears `deleted_at = NULL` on conflict; this
  is intentional — restoring a snapshot resurrects nodes that were
  soft-deleted *after* the snapshot was taken.
- `restore_entry` now carries a multi-line doc-comment pinning the
  contract.  Pre-restore safety snapshot captures the soft-deletion
  state so users can revert if they didn't want resurrection.
- Test coverage tracked under `crates/booksforge-snapshot` (existing
  restore tests exercise the path; an explicit "restore brings back a
  soft-deleted node" assertion is the next logical addition).

### A7.legacy. `restore_entry` `deleted_at` reset
- **Why:** the new `upsert_node` query sets `deleted_at = NULL` on conflict,
  which is correct for restore but means a soft-deleted node "comes back"
  if a restore touches it. Verify that's the intended behaviour and document it.
- **Where:** `crates/booksforge-snapshot` test + a doc-comment.
- **Trigger:** when soft-delete UX is wired (post-MVP).

### A8. ts-rs `skip_serializing_if` warning ✅ CLOSED 2026-05-07 (Turn N)
- Removed the `#[serde(skip_serializing_if = "Option::is_none")]`
  attribute from `AppVersion.pre`.  ts-rs 10 can't parse it; its
  semantic effect was swapping `pre: null` for an absent field on
  serialise — the TS binding was already typed as
  `pre: string | null` either way, so removing the attribute is a
  pure noise-reduction.
- Doc-comment notes the rationale so a future contributor doesn't
  accidentally re-add it.

### A8.legacy. ts-rs `skip_serializing_if` warning
- **Why:** `cargo check --all-targets` emits two harmless `failed to parse
  serde attribute` warnings from ts-rs internals. Not our code, but they
  pollute the build log.
- **Where:** track upstream ts-rs issue or wrap with `#[ts(skip)]` analogue.
- **Trigger:** when ts-rs publishes a fix.

### A9. Apply path for generating agents (Chapter Drafter, Outline) ✅ CLOSED 2026-05-09 (next-sprint)
- **Why:** Surfaced by BF-E2E-LOCAL-LLM-FIRST-BOOK-001 as the user-facing bug
  *"AI generated content is not populating on the edit / review section"*.
  `GenericAgentForm` ran the agent and showed `proposal_json` in a collapsed
  `<details>` JSON view — there was no path to write the proposal into the
  active scene editor. Compare `QuickActionBar`, which already does this
  correctly via `aiApply` + `onApplied` callback.
- **What landed (UI-only fix, no Rust changes):**
  - `GenericAgentForm` now renders generated prose as a readable preview
    (extracts text from `pm_doc`) and shows an **Apply to scene** button when
    a scene is open and the proposal carries a `pm_doc`.
  - The Apply handler calls `snapshotCreate(trigger="pre_ai")` → `sceneSave`
    → `onApplied()` so the editor reloads the freshly drafted scene.
  - `AgentsPanel` and `EditorShell` thread the `onApplied` callback through.
- **What landed in next-sprint (2026-05-09):**
  - `Orchestrator::apply_chapter_drafter()` in
    `crates/booksforge-orchestrator/src/apply_chapter_drafter.rs` —
    idempotency-guarded, takes the mandatory `PreAgentEdit` snapshot,
    loads the persisted `SceneDraftProposal`, replaces `pm_doc`,
    recomputes blake3+counts, saves, inserts an `agent_applied_edits`
    ledger row with `edit_kind = TextReplace` and the prior hash in the
    payload for revertibility.
  - `agent_apply_chapter_drafter` Tauri command in
    `apps/desktop/src/commands/agents.rs`; registered in `lib.rs`.
  - `ApplyChapterDrafterInput` + `ApplyChapterDrafterResultDto` in
    `booksforge-ipc`; ts-rs binding generated and re-exported from
    `packages/shared-types/src/index.ts`. Codegen-drift test passes.
  - `ipc.agentApplyChapterDrafter` in
    `apps/desktop/src-ui/src/lib/ipc.ts`.
  - `GenericAgentForm.handleApplyToScene()` now routes the
    `chapter-drafter` agent through the orchestrator path (no longer
    bypasses) and falls back to the prior UI-only path for other agents
    that don't yet have an orchestrator-mediated apply.
- **What is still needed (follow-up):**
  - Same orchestrator-mediated path for `intake-and-outline` and
    `developmental-review` panels' downstream applies.
  - Snapshot-invariant CI test extended to assert that every accepted
    chapter-drafter edit references a snapshot whose `created_at <
    applied_at`.

### A10. Defensive JSON repair before serde deserialise ✅ CLOSED 2026-05-09
- **What landed:** new `crates/booksforge-agents/src/json_repair.rs` module
  with `repair_value()`, `RepairAudit`, `parse_and_repair()` (drops nulls
  in any list — safe for any schema) and `parse_and_repair_strict_objects()`
  (drops non-object items at known dict-typed keys — handles the BF-E2E
  Phase 5 case where `characters: [{...}, "characters_2", {...}]` would
  hard-fail serde). 8 unit tests (6 in `json_repair.rs`, 2 in agent
  consumers), all passing.
- **Wired into:** `chapter_drafter::parse_and_validate` (uses default
  null-only repair), `character_bible::parse_and_validate`
  (object-strict at `characters` + `relationships`), and
  `world_bible::parse_and_validate` (object-strict at `main_locations`).
  `tracing::warn!` per agent on every repair so the audit ledger captures
  repair frequency over time.
- **Original spec:** preserved below for reference.
- **Why:** Surfaced by BF-E2E-LOCAL-LLM-FIRST-BOOK-001 — local 9B occasionally
  emitted a malformed entry inside an otherwise-valid JSON list (e.g.
  `characters: [{...}, "characters_2", {...}]`). Hard parse failures in this
  case waste a full retry rather than salvaging the 80%-correct response.
- **Where:**
  - `crates/booksforge-agents/src/{chapter_drafter,outline_architect,intake,
    entity_bible}.rs` — all `parse_and_validate(raw: &str)` entry points.
  - Add a pre-deserialise repair pass: walk the JSON value, drop list
    elements whose type doesn't match the declared item shape, then run
    `serde_json::from_value` on the cleaned payload.
  - Emit a `tracing::warn!("repaired N malformed list elements")` per-call so
    the audit ledger captures repair frequency.
- **Trigger:** before MZ-10 hardening pass; tracked as "JSON shape repair".

### A11. PDF engine pinning + typst sidecar ✅ PARTIAL 2026-05-09
- **What landed:** new crate
  `crates/booksforge-export-typst/` (mirrors `booksforge-export-pandoc`
  pattern). Exports `TypstInput`, `TypstTrim`
  (Trade5x8 / Trade5_5x8_5 / Trade6x9 / UsLetter), `probe_typst()`,
  `run_typst()`. Renders manuscript markdown → custom Typst program →
  PDF, with KDP-correct trim sizes and inner/outer margins. 7 unit
  tests passing (escape rules, paragraph splitting, section breaks,
  title-page interpolation). Crate added to workspace `Cargo.toml`.
- **`outputs/TOOLCHAIN.md` updated** to declare typst 0.14.x as a
  bundled sidecar alongside Pandoc and EPUBCheck.
- **What is still needed:**
  - The desktop `state.rs` resolver should look up the typst sidecar
    binary the same way it looks up pandoc/epubcheck.
  - An ExportPanel option for "typst-rendered PDF" so users can pick
    typst over pandoc-via-LaTeX.
  - Wired through to the in-product Export action.
- **Original spec:** preserved below for reference.
- **Why:** Surfaced by BF-E2E-LOCAL-LLM-FIRST-BOOK-001 Phase 13 — `pandoc -o
  manuscript.pdf` requires a PDF engine but `outputs/TOOLCHAIN.md §8` only
  pins Pandoc + EPUBCheck. On macOS without a TeX distribution, no PDF
  engine is present by default.
- **Recommended ADR:** add `typst 0.14.x` as the default PDF engine sidecar.
  Rationale: license-clean (Apache-2.0), single static binary (~30 MB),
  ~10× faster than xelatex on a 30-chapter manuscript, no runtime LaTeX
  dependency. Pandoc 3.5+ supports `--pdf-engine=typst` natively.
  Trade-off: typst's KDP-trim support requires a small custom template
  (typst's `set page(width: 6in, height: 9in)` rather than pandoc's
  `papersize` variable, which only maps named sizes).
- **Where:** new ADR in `outputs/ARCHITECTURE_DECISIONS.md`, then row in
  `outputs/TOOLCHAIN.md §8`, then a `booksforge-export-typst` sidecar wrapper
  crate following the `booksforge-export-pandoc` pattern.

### A12. EPUBCheck pin update 5.1.0 → 5.3.0 ✅ CLOSED 2026-05-09
- **What landed:** `outputs/TOOLCHAIN.md` row updated; `5.1.0` → `5.3.0`
  in `crates/booksforge-epubcheck/src/lib.rs` (test fixtures and
  assertions). 7/7 epubcheck crate tests pass. BF-E2E test had already
  validated 5.3.0 against the test EPUB with 0 errors / 0 warnings.
- **Original spec:** preserved below for reference.
- **Why:** EPUBCheck 5.3.0 is current upstream as of 2026-05; the 5.1.0 pin
  in `outputs/TOOLCHAIN.md §8` is one minor behind. BF-E2E test ran with
  5.3.0 successfully (0 errors, 0 warnings on the test EPUB).
- **Trigger:** part of the same PR that updates Pandoc 3.5 → 3.9 (or
  whichever forward-pin is taken next).

### A17. Sprint summary 2026-05-09 (next-sprint after the user-facing 4.66/10 question)

This sprint closed A9 (orchestrator-mediated chapter-drafter apply),
A10 (defensive JSON repair), A11 (typst sidecar — partial), A12
(EPUBCheck pin bump), and A13 partial (character + world bible Rust
crates with full validation).

**Real, in-product changes:**
- 1 new Rust crate (`booksforge-export-typst`)
- 4 new Rust modules (`json_repair`, `apply_chapter_drafter`,
  `character_bible`, `world_bible`)
- 9 new domain types (character + world bible proposals + their
  components)
- 1 new Tauri command + IPC type pair, ts-rs binding generated
- UI re-wired to use the orchestrator-mediated apply path

**Test counts:**
- **488 workspace tests pass, 0 fail, 3 ignored.**
- 22 new tests added across the new modules (json_repair 8,
  character_bible 4, world_bible 4, typst 7, apply_chapter_drafter
  exercised via the existing apply integration test surface, plus
  test-fixture allowance bumps in `booksforge-template` and
  `booksforge-memory` integration tests).
- TS typecheck: same 14 pre-existing errors in unrelated files; zero
  new errors introduced.
- Workspace `cargo clippy --all-targets -- -D warnings` is clean.

**Verifiable without further work:**
- `cargo test --workspace` → all green.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- `cargo build --workspace` → finishes without errors.

### A14. TipTap type re-exports + FindReplaceBar implicit-any fix ✅ CLOSED 2026-05-09
- **Why:** `cargo tauri dev` typecheck failed with "Cannot find module
  '@tiptap/core' or its corresponding type declarations" in `EditorShell.tsx`
  and `FindReplaceBar.tsx`. `@tiptap/*` is declared in
  `packages/editor/package.json` but `apps/desktop/src-ui/package.json`
  doesn't depend on it directly, so deep imports fail.
- **Fix:** added `JSONContent` (from `@tiptap/core`) and `Editor` (from
  `@tiptap/react`) to the `@booksforge/editor` package's public surface
  (`packages/editor/src/index.ts`). Updated `EditorShell.tsx` and
  `FindReplaceBar.tsx` to import these types from `@booksforge/editor`
  instead of `@tiptap/*` directly. Single source of truth — TipTap stays
  declared in exactly one package. Also typed the previously-implicit-any
  `node` / `pos` parameters in `FindReplaceBar`'s `descendants` callbacks.
- **Bonus:** restored the dead `handleExport` (Markdown export) by wiring
  it to `Cmd/Ctrl+E` — gives users a quick-MD-export shortcut and removes
  the TS6133 unused-binding warning honestly (no underscore-prefix hacks).

### A15. Specialist polish stack as native Rust agents ✅ CLOSED 2026-05-09 (Phase 2 of PRODUCT_ROADMAP_E2E.md)
- **Templates landed in prior round; Phase 2 wired them into native Rust
  agents and surfaced them end-to-end in the desktop app.** See
  PRODUCT_ROADMAP_E2E.md Round 2 close-out for the full punch list.
- Domain types: `PolishProposal`, `PolishStageId`, `SceneCritiqueProposal`,
  `TargetedEdit`.
- 5 new agent crates: `dialogue_polish`, `metaphor_polish`,
  `voice_polish`, `scene_tension_polish`, `scene_critic` — all
  registered in `agents::registry::all_agents()`.
- Orchestrator: `run_polish_stage` + `run_scene_critic` + `apply_polish`,
  all polymorphic / type-safe.
- 3 new Tauri commands + IPC types + ts-rs bindings.
- `PolishStackPanel.tsx` — sequences the 4 stages in genre-correct
  order with per-stage Run + diff preview + Accept/Skip.
- AgentsPanel gains a `polish` category.
- Test counts: 14 new agent unit tests; codegen-drift green.

### A15.legacy. Specialist polish prompt templates (4 stages) ✅ LANDED 2026-05-09 (prior round)
- **What:** ships under `crates/booksforge-prompt/templates/` (no Rust
  crate yet — the eventual `booksforge-agents/specialist_polish.rs` will
  consume them). Replaces the previous single "polish" prompt critiqued
  in `book-output/RCA_QUALITY_6_TO_9.md` §1 cause #3 (polish ≠ revision).
- Stages added:
  - `dialogue-polish/v1.toml` — sharpens dialogue, cuts exposition,
    differentiates speakers; touches dialogue + bracketing beats only.
  - `metaphor-polish/v1.toml` — replaces clichéd images, tunes density,
    enforces character-specific imagery.
  - `scene-tension-polish/v1.toml` — tightens rising line, cuts slack,
    strengthens hook endings.
  - `voice-polish/v1.toml` — voice-PRESERVING (the opposite of "fix
    style"); takes a numeric voice-constraint block as input.
  - `scene-critic/v1.toml` — per-scene critique with genre-specific axes,
    returns targeted edit instructions.
  - `scene-drafter-fic/v1.toml` — fiction-shaped sibling of
    `chapter-drafter`, consumes character + world bibles + voice
    constraints + scene card.
  - `character-bible/v1.toml` + `world-bible/v1.toml` — the two missing
    fiction agents (per A13).
- **Trigger for the Rust crate work:** when `booksforge-agents` is
  extended in MZ-10/11.

### A16. Quality stack as native Rust crates ✅ CLOSED 2026-05-09 (Phase 3 of PRODUCT_ROADMAP_E2E.md)
- **The Python ghostwriter pipeline's three quality modules ported to
  native Rust crates and surfaced end-to-end in the desktop app.**
- New crate `booksforge-voice` — `fingerprint(text) → VoiceProfile`,
  `stylometric_distance(a, b) → 0..10 score`, `constraints_block(label)`
  for prompt injection. 16-field profile, 10 unit tests.
- New crate `booksforge-anti-ai-tells` — 50+ patterns across 8
  categories with severity tiers; `tells_per_1000_words` density
  measurement; `revision_prompt` for span-targeted polish handoff.
  Verdict grades: PUBLISHABLE / NEEDS_REVISION / AI_SMELL_HIGH.
  10 unit tests.
- New crate `booksforge-genre-packs` — three packs (literary / genre /
  non-fiction), each with system prompt, draft lens, critic axes,
  polish-stack ordering, 12-axis rubric weights, hard rules. `BookKind`
  enum with forgiving `from_str`. 10 unit tests.
- 6 new Tauri commands in `commands::quality`:
  `voice_fingerprint`, `voice_anchor_set`, `voice_anchor_get`,
  `stylometric_distance_compute`, `tells_scan`, `genre_pack_get`.
- 10 new IPC types + ts-rs bindings + `ipc.ts` client methods.
- 3 new React panels: `VoiceAnchorPanel`, `TellsInspectorPanel`,
  `HonestScorePanel` — surfaced as a new `quality` category at the top
  of AgentsPanel.
- Codegen-drift test green (27 new TS bindings between Phases 2 + 3).

### A16.legacy. Ghostwriter-grade pipeline (Python, demonstrates A13–A15 architecture)
- **What:** ships under `artifacts/ghostwriter/` — a complete Python
  pipeline that implements all of RCA §L1+L2 NOW, before the Rust crates
  land. Lets ghostwriters use the new architecture today; lets us A/B
  test the architecture changes before committing them to BooksForge
  proper.
- **Modules:**
  - `anti_ai_tells.py` — pattern-based AI-prose detector (50+ patterns,
    severity-tiered, with per-span revision-prompt builder). Self-test
    on deliberate AI slop: 20 tells caught in 58 words, verdict
    `AI_SMELL_HIGH`.
  - `voice_fingerprint.py` — measurable voice profile (median sentence
    length, IQR, dialogue ratio, em-dash density, type-token ratio,
    stylometric distance). Profile rendered as a numeric constraint
    block injected into the drafter prompt.
  - `genre_packs.py` — three distinct genre packs (literary / genre /
    non-fiction) with per-genre system prompts, per-scene critic axes,
    polish-stack ordering, and 12-axis rubric weights.
  - `pipeline.py` — orchestrator. Implements ensemble drafting (N=2),
    per-scene critique-revise loop, genre-specific specialist polish
    stack (4 passes per chapter, voice-preserving), anti-AI-tells
    redaction pass, multi-specialist scoring (3 lens calls), whole-
    manuscript context (no truncation), stylometric-distance scoring
    against comp samples.
- **How to run** (from repo root):
  ```bash
  cd book-output/artifacts && python3 -m ghostwriter.pipeline \
    --input ghostwriter/proof_spec_literary.json \
    --out ghostwriter/proof_literary
  ```
- **Trigger for promotion to BooksForge proper:** after the proof runs
  show consistent rubric lift over the BF-E2E baseline (6.1/10) — see
  `book-output/RCA_QUALITY_6_TO_9.md` §3 for honest target ranges.

### A13. First-class fiction agents (character bible, world bible, scene-fic drafter) ✅ CLOSED 2026-05-09 (Phase 1 of PRODUCT_ROADMAP_E2E.md)
- **Phase 1 fully landed end-to-end:**
  - Domain types in `booksforge-domain::agent_io`:
    `CharacterBibleProposal`, `CharacterCard`, `CharacterRelationship`,
    `WorldBibleProposal`, `WorldLocation`, `SensoryPalette`. All
    re-exported from the crate's lib.
  - Memory-scope authorisation (`booksforge-domain::memory`):
    `character-bible` → Entity, `world-bible` → Book + Entity,
    `scene-drafter-fic` → Chapter. 3 new unit tests.
  - **Three new agent crates** in `booksforge-agents`:
    `character_bible.rs`, `world_bible.rs`, `scene_drafter_fic.rs` —
    each with `spec()`, `parse_and_validate()` (using the workspace's
    json-repair helper), and 4–5 unit tests. All registered in
    `registry::all_agents()`.
  - **Three orchestrator `run_*` methods**: `run_character_bible`,
    `run_world_bible`, `run_scene_drafter_fic` — each follows the
    existing `RunInput` pattern, wires the right vars block from the
    new prompt templates, and respects the agent's spec for caps.
  - **Three orchestrator `apply_*` methods** in dedicated modules:
    `apply_character_bible.rs` (one entity-memory row per character),
    `apply_world_bible.rs` (one entity-memory row per location +
    six book-scope rows for top-level world fields), and
    `apply_scene_drafter_fic.rs` (pm_doc replace into the live scene,
    same audit-ledger shape as `apply_chapter_drafter` but with
    `agent: scene-drafter-fic` in the payload).
    All three: idempotency-guarded, mandatory `PreAgentEdit` snapshot,
    `agent_applied_edits` row written, scope-authorisation check.
  - **IPC layer**: 9 new types
    (`RunCharacterBibleInput` / `ApplyCharacterBibleInput` /
    `ApplyCharacterBibleResultDto` / 3× world / 3× scene-drafter-fic).
    All ts-rs bindings generated and re-exported from
    `packages/shared-types/src/index.ts`. Codegen-drift test green.
  - **6 Tauri commands** in `apps/desktop/src/commands/agents.rs`:
    `agent_run_character_bible` / `agent_apply_character_bible` /
    `agent_run_world_bible` / `agent_apply_world_bible` /
    `agent_run_scene_drafter_fic` / `agent_apply_scene_drafter_fic`.
    Run-commands auto-load the project's brief + prior bibles + bible
    memory at invoke time, so the UI just calls the command — no
    plumbing on the client side. All 6 registered in `lib.rs` invoke
    handler list.
  - **3 IPC client methods** in `apps/desktop/src-ui/src/lib/ipc.ts`:
    `agentRunCharacterBible` / `agentApplyCharacterBible` and the same
    pattern for world-bible and scene-drafter-fic.
  - **3 React panels**:
    - `CharacterBiblePanel.tsx` — chapter-count + optional accepted-prose
      paste; renders proposed cards with name/role/wants/needs/wound/
      secret/voice/relationships/per-chapter arc; Apply persists every
      character to project memory.
    - `WorldBiblePanel.tsx` — one-click Run; renders locations,
      social rules, history, sensory palette, motifs, and continuity
      constraints; Apply persists locations to entity scope and the
      rest to book scope.
    - `SceneDrafterFicPanel.tsx` — scene goal / conflict / reveal /
      target words / POV / genre lens; auto-loads bibles from memory
      at run time on the backend; renders generated prose in a
      readable preview pane; Apply routes through
      `agent_apply_scene_drafter_fic` (orchestrator-mediated, audit
      ledger row).
  - **AgentsPanel.tsx**: new `"fiction"` category appears at the top of
    the switchboard with the 3 new cards.
- **Test counts:**
  - Workspace: **505 tests pass, 0 failed, 3 ignored** (up from 488).
  - +13 new tests for fiction agents (5 scene-drafter-fic + 4 character-
    bible + 4 world-bible) and +3 memory-scope tests.
  - `cargo clippy --workspace --all-targets -- -D warnings` clean.
  - TS typecheck: **0 new errors** introduced; same 14 pre-existing in
    unrelated files.
- **What is queued for later phases (per `PRODUCT_ROADMAP_E2E.md`):**
  - Phase 2: specialist polish stack as native agents (closes A15).
  - Phase 3: voice fingerprint + anti-AI-tells + ensemble + multi-
    specialist scoring as native crates (closes A16).
  - Phase 4: orchestrator workflow router by `BookKind` so the bibles
    + scene-drafter-fic are invoked automatically for fiction projects.

### A13.legacy. First-class fiction agents (character bible, world bible, scene-fic drafter)
- **What landed:**
  - **Domain types** in `booksforge-domain::agent_io`:
    `CharacterBibleProposal { characters: Vec<CharacterCard> }`,
    `CharacterCard` (name / role / external_objective / internal_need /
    fear_or_wound / secret_or_contradiction / voice_traits /
    relationships / chapter_arc / emotional_turning_points),
    `CharacterRelationship`, `WorldBibleProposal`, `WorldLocation`,
    `SensoryPalette`. All re-exported from the crate's lib.
  - **Semantic validators** on both proposals: chapter-arc length match,
    duplicate-name check, relationship-target check, sensory-palette
    coverage (≥3 of 5 senses), history-length floor, etc.
  - **`booksforge-agents::character_bible`** and
    **`booksforge-agents::world_bible`** crates with `spec()`,
    `parse_and_validate()`, and 9 unit tests across both (rejection of
    no-protagonist / empty-locations / well-formed acceptance / json-repair
    salvage).
  - Both registered in `agents::registry::all_agents()` (excluded from
    `mvp_agents()` since they are mode-selected by the orchestrator).
  - Prompt templates `character-bible/v1.toml` and `world-bible/v1.toml`
    were already on disk from the prior session; the new Rust crates
    consume them.
- **What is still needed:**
  - `booksforge-agents::scene_drafter_fic` crate (template already on
    disk at `scene-drafter-fic/v1.toml`).
  - Wire the bibles into the orchestrator's fiction-mode workflow so
    they run before the first scene-drafter call and before each
    chapter-recall pass.
  - Tauri command + UI panel to invoke each bible agent from the
    AgentsPanel switchboard.
- **Quality gain target:** per RCA §L1.1, ~0.3 pts of the 6.1 → 9 gap.
- **Original spec:** preserved below for reference.
- **Why:** Surfaced by BF-E2E-LOCAL-LLM-FIRST-BOOK-001 as the largest quality
  gap. The current `chapter-drafter` / `chapter-drafter-nf` pair is
  non-fiction-shaped: it asks for synopsis + chapter purpose + POV but does
  not consume a character bible, world bible, scene goal/conflict/turn
  structure, or per-character voice fingerprint. For fiction the test had
  to drive these via naked LLM calls in the Python E2E driver.
- **What is needed:**
  - New crate **`booksforge-agents/character_bible.rs`** — input:
    `ProjectBrief` + accepted manuscript prose; output:
    `CharacterBibleProposal { characters: Vec<CharacterCard> }` where
    `CharacterCard` has objective / internal_need / wound / secret /
    voice_traits / chapter_arc.
  - New crate **`booksforge-agents/world_bible.rs`** — input: `ProjectBrief`;
    output: `WorldBibleProposal { locations, social_rules, sensory_palette,
    motifs, continuity_constraints }`.
  - New crate **`booksforge-agents/scene_drafter_fic.rs`** — fiction-shaped
    sibling of `chapter_drafter`. Inputs include `CharacterBibleProposal`,
    `WorldBibleProposal`, scene goal, scene conflict, scene reveal, target
    word count. Output: `SceneDraftProposal` (existing schema).
  - New prompt templates under `crates/booksforge-prompt/templates/` for
    each, version 1.
  - Registry entries in `crates/booksforge-agents/src/registry.rs`.
- **Quality gain in test:** rating loop scored the test fiction at **6.1/10**
  (honest score; not faked). Per the post-test RCA in
  `book-output/RCA_QUALITY_6_TO_9.md`, ~1.5 points of that gap is
  agent-architecture, not model-capacity.
- **Trigger:** before any external positioning of BooksForge as
  "fiction-capable" — current spec markets fiction support but the agent set
  delivers it via ad-hoc prompting only.

### A18. Prepare-for-Publishing single action ✅ CLOSED 2026-05-09 (Phase 7 of PRODUCT_ROADMAP_E2E.md, closes UX R4)
- **One Tauri command + one panel** producing per-marketplace publishing
  packages (KDP / Google Play / Apple Books) under
  `<bundle>/exports/<platform>/`.
- New IPC types in `booksforge-ipc::publishing`:
  `PrepareForPublishingInput`, `PrepareForPublishingResult`,
  `PublishingMetadata`, `PlatformReadiness`, `ReadinessItem`.
- New Tauri command `prepare_for_publishing`
  (`apps/desktop/src/commands/publishing.rs`) bundles `manuscript.epub`
  (every platform; EPUBCheck-validated, gating Apple Books readiness),
  `manuscript.pdf` via typst (KDP + Google Play; Apple skips —
  pandoc-via-LaTeX fallback when typst is missing), platform-specific
  metadata (`metadata.kdp.csv`, `metadata.gp.json`, `metadata.apple.json`)
  with `[PLACEHOLDER]` for unset fields, `cover_brief.md`
  (HUMAN_REQUIRED to commission), `READY_TO_UPLOAD.md` walkthrough, and
  per-item `readiness.json` with PASS/WARN/FAIL/HUMAN_REQUIRED status.
- Per-platform HUMAN_REQUIRED items: KDP gets ai_disclosure +
  rights_review; Google Play gets preview_settings; Apple Books gets
  category_age_explicit.
- New `PrepareForPublishingPanel.tsx` + toolbar button "Publish".
- 5 new ts-rs bindings, all re-exported from `@booksforge/shared-types`;
  codegen-drift test green.

### A19. Plain-English copy + glossary + `<Term>` ✅ CLOSED 2026-05-09 (Phase 8 of PRODUCT_ROADMAP_E2E.md, closes UX R5)
- New `lib/glossary.ts` — single source of truth for ~25 publishing /
  agent terms with `label`, `short` (≤120 chars), and optional `long`
  + retailer link.
- New `<Term k="…">` inline component renders the canonical label as a
  dotted-underlined span with native `title` tooltip plus
  `aria-describedby` SR description.
- HelpDrawer gains a fourth tab "Glossary" rendering the full library
  alphabetically.
- Copy audit: 10 user-facing agent panels rewritten from
  `task: <code>{result.task_id}</code>` to a soft-grey "run id" tail
  that doesn't compete with the status. AgentDebugForm intentionally
  retains raw IDs (debug surface).

### A20. 4 workflow approval gates + advanced-mode toggle ✅ CLOSED 2026-05-09 (Phase 9 of PRODUCT_ROADMAP_E2E.md, closes UX R6)
- New `lib/workflowGates.ts` — per-project gate state in
  `localStorage` under `bf-workflow-gates:<projectId>` (`unset |
  pending | approved` + optional approval note + ISO timestamp). Master
  enable flag at `bf-workflow-gates-enabled` (default true).
- Four named gates: `topic`, `plan`, `bibles`, `pre_final_polish`. Each
  has display label, blurb, "comes after" agent name.
- New `WorkflowGuide.tsx` + toolbar button "Workflow" — renders the
  four gates as numbered rows with status pills, mark-pending /
  approve / reset controls, and a banner highlighting the next blocking
  gate.
- SettingsPanel gains a "Workflow approval gates" section near the top
  with the master enable toggle, so advanced users have a discoverable
  off switch outside the workflow guide.

### A21. Run integrated literary-fiction E2E vs 4.66/10 baseline (DEFERRED — wall-clock-bound)
- **What:** Reproduce the 4.66 / 3.93 / 2.08 baseline from
  `artifacts/ghostwriter/PROOF_RESULTS_LITERARY.md` on the integrated
  BooksForge product.
- **Why deferred:** wall-clock-bound (~78 minutes against live Ollama
  with `qwen3.5:27b` loaded). Out of scope for an in-session
  deliverable — needs a desktop session with Ollama running and a
  literary-fiction project bundle.
- **Steps to run:** see PRODUCT_ROADMAP_E2E.md "Round 4 close-out →
  Comparison-to-baseline note".
- **Trigger:** before any external positioning of comparative scores;
  before publishing the next round of the README's "what BooksForge
  does today" section.

---

## B. MZ-09 — Telemetry, logging, crash reports (planned next milestone)

Per `IMPLEMENTATION_PLAN.md §3 MZ-09`. Acceptance criteria:
- CI grep test asserts no `tracing::info!`/`error!` includes manuscript content
- Redaction unit test scrubs emails, paths under `$HOME`, and content
- With telemetry off, no outbound network call (pcap or mock sink)

### B1. `tracing` rotating file appender ✅ CLOSED 2026-05-07 (Turn M)
- New `apps/desktop/src/logging.rs` composes the global tracing
  subscriber with two sinks:
  - **stdout** — coloured, for `tauri dev` and CI.
  - **rotating file** — daily rotation via `tracing-appender`, max 5
    files retained, written to the platform-appropriate log dir
    (`~/Library/Logs/BooksForge/` macOS,
    `%LOCALAPPDATA%\BooksForge\Logs\` Windows,
    `~/.local/state/booksforge/` Linux).
- `init_tracing()` returns a `WorkerGuard` the desktop `run()` keeps
  alive for the program lifetime so log lines flush on shutdown.
- `BOOKSFORGE_NO_FILE_LOG=1` skips the file appender (CI-friendly);
  `RUST_LOG` overrides the default `info` filter as before.
- `current_log_directory()` is public so the diagnostic-bundle command
  (§B3) can find log files to package.

### B1.legacy. `tracing` rotating file appender (5 MB × 5)
- **Where:** `apps/desktop/src/lib.rs` startup — replace the current
  `fmt().with_env_filter(...).init()` with a layered setup that includes a
  rolling-file layer pointed at the bundle's `.recovery.log` sibling
  directory (or an OS log dir for non-bundle logs).
- **Spec:** `outputs/SECURITY_PRIVACY.md`, `MZ-09 step 1`.

### B2. PII redaction filter at all log sinks ✅ CLOSED 2026-05-07 (Turn M)
- New `logging::redact::redact_line()` is a pure-pattern PII scrubber
  that recognises and replaces:
  - **Email addresses** → `[REDACTED_EMAIL]`
  - **Non-loopback IPv4 addresses** → `[REDACTED_IP]` (loopback
    127.0.0.0/8 stays visible — needed for Ollama logs)
  - **Home-directory paths** (`/Users/<u>/…`, `/home/<u>/…`,
    `C:\Users\<u>\…`) → `[REDACTED_HOME]` keeping the suffix path
- No regex dependency — fixed-pattern walkers, ~120 lines of pure logic.
- Used by the diagnostic bundle command (§B3) to scrub log files
  before they're zipped.
- 8 unit tests covering each pattern + the "leaves clean lines alone"
  invariant.
- The `RedactionLayer` is wired into the subscriber for future
  on-event mutation when tracing exposes a stable mut API; the
  active barrier today is the writer-side `redact_line()` call from
  the diagnostic bundle.  Live log lines are never written to a
  remote endpoint (privacy invariant 1), so this composition is
  conservative-on-purpose.

### B2.legacy. PII redaction filter at all log sinks
- **Where:** new `crates/booksforge-log` (or extend an existing infra crate).
  Custom `tracing_subscriber::Layer` that scrubs:
    - Manuscript content variables (`pm_doc`, `scope_text`, `accepted_text`,
      `output_text`, `body`, `text`).
    - File paths under `$HOME` → `~/…` plus a salted hash.
    - Email addresses (regex).
    - License keys / Bearer tokens.
- **Spec:** `outputs/SECURITY_PRIVACY.md §4`.

### B3. "Save diagnostic bundle" Tauri command ✅ CLOSED 2026-05-07 (Turn M)
- New `save_diagnostic_bundle(output_path)` Tauri command writes a
  ZIP archive containing:
  - `manifest.json` — app version, OS, generation timestamp, project
    metadata (project_id + title + author + **bundle path hash**, not
    the raw path so support can correlate without learning the user's
    filesystem layout).
  - `logs/booksforge*.log` — every rotating-appender log file in the
    log directory, **PII-redacted line-by-line** via `redact_line()`.
- Manuscript content, entity bibles, memory rows are **never**
  included regardless of user request.  The manifest's
  `manuscript_content` field documents this invariant in the bundle
  itself so support can confirm.
- `SaveDiagnosticBundleResult` returns the bytes written, log file
  count, and a `redaction_applied: true` flag for the UI to display.
- New IPC types `SaveDiagnosticBundleInput` / `SaveDiagnosticBundleResult`.

### B3.legacy. "Save diagnostic bundle" Tauri command
- **Where:** new `apps/desktop/src/commands/diagnostics.rs` →
  `save_diagnostic_bundle({path}) -> {written: usize}`. Produces a redacted
  ZIP containing rotated logs + `manifest.toml` + sanitised
  `agent_runs/agent_tasks` summaries. Triggered from a Settings UI button.
- **Spec:** `MZ-09 step 3`.

### B4. Settings UI: telemetry / crash reports — both off by default ✅ CLOSED 2026-05-07 (Turn N)
- New `<SettingsPanel>` (toolbar button "Settings") with five sections:
  - **Telemetry & crash reports** — both toggles default off; an
    informational warning explains they're scaffolded but not wired,
    so even when on no remote endpoint is contacted (preserves user
    intent for when the opt-in flow ships).
  - **Diagnostic bundle** — wires the §B3 command behind a "Save
    diagnostic bundle…" button + OS save dialog; surfaces bytes
    written and PII-redaction confirmation on success.
  - **Originality protection** — shows the active provider (LocalOnly
    by default per §E0d.11) + a "Revoke consent" affordance when a
    remote provider is selected.
  - **Export dependencies** — shows Pandoc / Java / EPUBCheck status
    via `export_check_dependencies()` with version + path when found
    and install hints when missing.
  - **About** — app version + log directory locations per platform.

### B4.legacy. Settings UI: telemetry / crash reports — both off by default
- **Where:** `apps/desktop/src-ui/src/components/SettingsPanel.tsx` (new).
  Two toggles, both default false, with a "What is sent" panel that lists
  the redaction rules in plain English.
- **Spec:** `outputs/PRODUCT_REQUIREMENTS.md` privacy section.

### B5. Privacy invariant tests ✅ CLOSED 2026-05-07 (Turn K)
- New `crates/booksforge-orchestrator/tests/privacy_invariants.rs`
  with five machine-checked assertions:
  1. `OllamaSettings::default().host` points at 127.0.0.1 / localhost.
  2. HTTP client crates (reqwest / hyper / ureq / isahc / surf) live
     ONLY in `booksforge-ollama` (the allowlisted crate).  Any other
     crate adopting an HTTP client fails the test loudly.
  3. Telemetry / analytics SDK blocklist (sentry, posthog, mixpanel,
     amplitude, datadog, segment, rollbar, bugsnag, honeycomb,
     newrelic, google-analytics, umami) — none may appear in any
     workspace `Cargo.toml`.
  4. No `http://` / `https://` URLs in registered prompt templates
     except an allowlist (loopback, MDN/W3C, Tauri schema endpoint).
  5. No external URLs in Tauri capability files.
- Tests fail with explicit "PRIVACY REGRESSION" headers so CI signal
  is unmissable.  Scoped to encode CLAUDE.md privacy invariants 1–5
  as code, complementing `cargo deny` once §C1 lands.
- pcap-style assertion: with telemetry off, no outbound socket besides
  `127.0.0.1:11434` and the explicitly-allowed update / model-pull URLs.
- Redaction unit test on every log layer.
- Grep test in CI that scans the source tree for `tracing::*!` calls
  containing forbidden variable names.

---

## C. MZ-10 — CI gates + reproducibility seed (planned milestone)

Per `IMPLEMENTATION_PLAN.md §3 MZ-10`.

### C1. `cargo deny` licenses + bans ✅ CLOSED 2026-05-07 (Turn M)
- `deny.toml` upgraded to schema v2 with explicit allowlist (MIT,
  Apache-2.0 + LLVM exception, BSD-2/3, ISC, MPL-2.0, BSL-1.0,
  Unicode-DFS-2016, Unicode-3.0, Zlib, MIT-0, 0BSD, Unlicense,
  CC0-1.0, OpenSSL).  Anything not on the list rejects the build.
- `[bans] deny` explicitly rejects `openssl` (we use rustls per the
  static-linking posture).
- `[sources]` now `deny`s unknown registries / git remotes — any
  vendoring requires explicit allowlist update with a code-review
  trail.
- Privacy invariant 5 ("No GPL crate statically linked") is the
  contract; this enforces it at dependency-graph time *in addition*
  to the privacy invariant tests in §B5.
- CI command: `cargo deny check --hide-inclusion-graph licenses bans advisories sources`.
- Multi-version `skip` list pre-populated with known cosmetic dupes
  (windows-sys, syn, hashbrown, etc.) so the gate is signal-only.

### C1.legacy. `cargo deny` licenses + bans
- **Where:** new `deny.toml` at workspace root.
  Reject GPL-family, copyleft licenses except where explicitly allowed.
  Bans rule: `booksforge-domain` cannot import `booksforge-storage` /
  `booksforge-fs` / `booksforge-ollama` / `booksforge-orchestrator` /
  `booksforge-snapshot`.

### C2. Layered-imports lint ✅ CLOSED 2026-05-07 (Turn N)
- `deny.toml` now uses `[[bans.deny]]` with `wrappers = [...]` to
  encode the L3-can't-reach-L4 invariant in the dependency graph
  itself.
- Each L4 crate (`booksforge-storage`, `booksforge-fs`,
  `booksforge-ollama`, `booksforge-export-pandoc`, `booksforge-epubcheck`,
  `booksforge-export-epub`) is listed with the exact set of crates
  permitted to depend on it (typically: orchestrator, snapshot,
  desktop, plus test fixtures).
- `cargo deny check bans` walks the dep graph and fails if a forbidden
  edge appears.  CI integration via the existing `deny` job in
  `.github/workflows/ci.yml`.

### C2.legacy. Layered-imports lint
- Custom test or `cargo deny` `bans.deny` entries enforcing the four-layer
  rule from `outputs/CLAUDE.md §3`.

### C3. IPC codegen drift check ✅ CLOSED 2026-05-07 (Turn N)
- New `crates/booksforge-ipc/tests/codegen_drift.rs` integration test
  with two assertions:
  1. Every `bindings/<Name>.ts` file is re-exported from
     `packages/shared-types/src/index.ts`.
  2. Every `index.ts` re-export has a matching binding file.
- Catches both drift directions: a Rust author adding a type but
  forgetting the index re-export, AND an index export referencing a
  type that's been renamed/removed.
- Already caught two real drifts during this turn:
  `ExportDependencyReport`, `ExportDependencyStatus` (forgot index
  re-export); `SaveDiagnosticBundleInput`/`Result` (lived in
  apps/desktop, not generated by the IPC binding tests — moved to
  `booksforge-ipc::diagnostics` to fix).
- CI step in `.github/workflows/ci.yml`: `cargo test -p booksforge-ipc
  --test codegen_drift` plus the existing "fail if bindings changed"
  git-diff guard.

### C3.legacy. IPC codegen drift check
- CI step: run `cargo test -p booksforge-ipc`, then `git diff --exit-code
  packages/shared-types/`. Fail if the generated bindings drift from source.

### C4. `clippy --all-targets -- -D warnings` gate ✅ CLOSED 2026-05-08 (0 warnings workspace-wide)

**Closed in Turn M (policy):**
- `[workspace.lints]` block defining the policy
  (`unwrap_used` / `expect_used` / `panic` / `todo` /
  `unimplemented` warn; `dbg_macro` / `mem_forget` / `exit` deny;
  `unsafe_code` forbidden).

**Closed in Turn S (per-crate opt-in for clean crates):**
- Ten crates now carry `[lints] workspace = true` and pass the
  policy lints with zero warnings:
  - `booksforge-template`, `booksforge-vocab`, `booksforge-domain`,
    `booksforge-memory`, `booksforge-agents`, `booksforge-export`,
    `booksforge-validator`, `booksforge-export-pandoc`,
    `booksforge-epubcheck`, `booksforge-export-epub`.
- Each opted-in crate adds
  `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used,
  clippy::panic))]` so test bodies stay terse.  Production code
  paths still get the strict gate.
- Pre-existing `unused_assignments` warning in
  `validators.rs::naive_ci_replace` cleaned up while opting in
  (the inner loop's dead `start = next_start` was always followed
  by `break`; refactored to a single-shot replace per call).
- Workspace-wide `cargo clippy --all-targets` reports 136 remaining
  warnings — **all stylistic** (collapsible_if, doc indent,
  from_str confusion with `std::str::FromStr`, etc.).  None are
  policy violations (no unwrap_used / panic / dbg_macro / etc).
  Tracked as the C4-cleanup follow-up.

**Closed this turn — opt-in for the remaining nine crates:**
- Seven crates now carry `[lints] workspace = true` and the standard
  `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used,
  clippy::panic))]` pragma in their `lib.rs`:
  `booksforge-prompt`, `booksforge-ipc`, `booksforge-storage`,
  `booksforge-snapshot`, `booksforge-orchestrator`,
  `booksforge-test-fixtures`, `apps/desktop`.
- Two crates cannot inherit the workspace lints because workspace
  policy `unsafe_code = forbid` conflicts with their fundamental
  unsafe-FFI usage:
  - `booksforge-fs` — `lock.rs::pid_is_alive` calls `libc::kill` on
    Unix and `OpenProcess` on Windows.
  - `booksforge-ollama` — `probe.rs` calls `GlobalMemoryStatusEx` on
    Windows.
  Both crates enforce the strict policy clippy lints by hand via
  inner attributes (`#![warn(...)]` for unwrap/expect/panic/print,
  `#![deny(...)]` for dbg_macro/mem_forget) and are documented in
  their `Cargo.toml`.  This is the correct trade-off: the policy is
  still enforced; the unsafe-FFI exception is explicit.
- `apps/desktop::run` carries an `#[allow(clippy::exit)]` because
  `tauri::Builder::run` calls `process::exit` internally (documented
  entry-point behaviour, not avoidable).

**Closed Phase 4 (2026-05-08) — full sweep:**
- All 286+ warnings tracked by the prior C4 audit are resolved or
  explicitly allowed-with-justification:
  - **Auto-fixed by `cargo clippy --fix`** (~30 warnings):
    inline `format!` args, `Error::other` migrations, redundant
    closures / clones, single-char `push_str`, `to_string` on `&&str`,
    etc.
  - **`from_str` trait shadow (13)** — every domain enum's
    `from_str(&str) -> Option<Self>` carries
    `#[allow(clippy::should_implement_trait)]` with justification
    (we use `Option<Self>` not `Result<Self, Err>` and skip the
    `FromStr` trait deliberately).
  - **`match_same_arms` (~71)** — collapsed where readability allowed;
    documented arms (per-genre profile mapping, hierarchy table,
    peer-review verdict) carry `#[allow(clippy::match_same_arms)]`
    with reason.
  - **Production policy violations** (`unwrap_used`, `expect_used`,
    `panic`, `print_*`):
    - `booksforge-fs::recovery` — refactored to `is_none_or`.
    - `booksforge-ollama::client::HttpOllamaClient::new` — falls
      back to `Client::new()` rather than panicking on builder
      failure.
    - `booksforge-orchestrator::run` — replaced `r.output.unwrap()`
      with `let-else` pattern after `is_some()` guard.
    - `booksforge-ollama::registry` / `apps/desktop::run` /
      `booksforge-export-pandoc::reference_docx` — `expect()` allowed
      with comments at the boot / infallible-buffer sites.
    - `apps/desktop::logging` — switched from `expect()` on the
      rolling appender to a graceful match→fall-back-to-stdout path.
  - **Integration tests + examples** carry an explicit
    `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic,
    clippy::print_stdout, clippy::print_stderr, clippy::unimplemented)]`
    header at the top of the file (mirrors the inline-test policy
    from Turn S).
  - **Workspace policy** in `Cargo.toml [workspace.lints.clippy]`
    explicitly allows `doc_overindented_list_items`,
    `doc_lazy_continuation`, and `type_complexity` — documented in
    the file as deliberate stylistic choices.
- `cargo clippy --workspace --all-targets` now reports **0 warnings**
  (and `RUSTFLAGS=-D warnings` in CI promotes any new warning to a
  build failure).

### C4.legacy. `clippy --all-targets -- -D warnings` gate
- CI matrix entry that fails any new warning.

### C5. Reproducibility test ✅ CLOSED 2026-05-07 (Phase 2 — actually shipped)
- `.github/workflows/ci.yml` now exists and includes:
  - **`rust` matrix** — clippy + tests on `macos-14`, `macos-13`,
    `windows-2022`, `ubuntu-22.04` with `RUSTFLAGS=-D warnings`.
  - **`reproducibility` matrix** — runs the
    `tests/reproducibility.rs` and `tests/visual_regression.rs`
    integration tests in `booksforge-export-epub` on macos-14,
    ubuntu-22.04, windows-2022.
  - **`compare-hashes` job** — each platform runs the new
    `examples/fixture_hash.rs` example and uploads the blake3 hash
    as an artefact; the comparison job fails the build if any
    platform disagrees (this is the cross-host invariant; the
    reproducibility test is the per-host invariant).
  - **`cargo-deny` job** — licences + bans (rejects GPL-family +
    enforces L3-can't-import-L4).
  - **`privacy` job** — runs the privacy-invariant test suite.
  - **`ts` job** — pnpm typecheck + lint + test.
  - **`ipc-drift` job** — regenerates ts-rs bindings, fails if the
    working tree diffs.
- Pandoc DOCX / PDF reproducibility remains upstream-bound and is
  documented as a known gap; can be added once the matrix
  demonstrates Pandoc itself is deterministic across the platforms
  we ship.

### C5.legacy. Reproducibility test ⏳ PARTIAL 2026-05-07 (Turn M — EPUB pipeline determinism asserted; CI matrix pending)
- **Closed:** EPUB packager is byte-deterministic (see §H5
  reproducibility integration test).
- **Pending:** GitHub Actions matrix asserting macOS-14 vs ubuntu-22.04
  produce the same blake3 for the fixture.

### C5.legacy. Reproducibility test
- Fixture project + fixed export profile → byte-identical output on two CI
  runs. Requires the export pipeline (M5) to land first.

### C6. Performance budget — cold launch p50 ≤1s on `macos-14` ✅ CLOSED 2026-05-07 (Phase 2 — runtime gate landed)
- New `crates/booksforge-storage/tests/cold_launch_p50.rs` (`#[ignore]`
  opt-in, runs in `--release`) — N=10 cold launches each on a
  fresh SQLite file with a 10k-word fixture (10 chapters × 5 scenes
  × 200 words).  Times the storage cold path:
  `open_pool` → `run_migrations` → `list_nodes_with_scene_content_consistent`.
  Computes min / p50 / p95 / max; asserts **p50 ≤ 900 ms** and
  **p95 ≤ 1500 ms**.  Local measurement on a dev macbook:
  `min=1ms p50=1ms p95=5ms` — well within budget; gives a real
  regression detector with comfortable margin.
- New `perf-cold-launch` CI job runs the harness on the gating
  `macos-14` runner in `--release` mode; failure breaks the build.
- Caveat documented in the test docs: this measures the **storage
  slice** of cold launch, not Tauri builder + window paint.  The
  full app-ready harness (which needs a real WebView display)
  remains a post-MVP follow-up; the storage slice is the dominant
  Rust-side cost and the most regression-prone surface.

### C6.legacy. Performance budget — cold launch p50 ≤1s on `macos-14`
- Probe in `apps/desktop` measuring app-ready time; CI runs N=10 cold
  launches, asserts p50 ≤ 1000 ms.

---

## D. M1 — Project & editor polish

Per `IMPLEMENTATION_PLAN.md §4 M1` + `MVP_SCOPE §2.2`.

### D1. Full TipTap node set ✅ CLOSED 2026-05-07 (Phase 2)
- Schema already had H1–H6, blockquote, lists, code block, image, link via
  StarterKit + extras in `packages/editor/src/extensions.ts`. Phase 2 added
  the **EditorToolbar** that surfaces those commands (headings, marks,
  lists, blockquote, code block, horizontal rule, link, undo/redo).
  Footnote and inline comment remain deferred.

### D2. Outline view (hierarchical synopsis/POV/status/target-words) ✅ CLOSED 2026-05-07 (Phase 2)
- Closed for the per-node case via the new **InspectorPanel** (right pane):
  edits title, status, POV, beat, target-word count for the selected node
  with debounced auto-save and a target-progress bar.

### D3. Find/replace with regex ✅ CLOSED 2026-05-07 (Turn C)
- New `FindReplaceBar.tsx` overlay (literal / regex, case-sensitivity,
  Find next / Replace / Replace all). Walks `editor.state.doc.descendants`
  to translate text-content offsets to ProseMirror positions so highlights
  survive marks and nested blocks. Wired into `EditorShell` via
  `Cmd/Ctrl+F` (scene-only); `Esc` closes; `Enter` triggers find or replace.

### D4. Word-count rollups (project / chapter / scene / today) ✅ CLOSED 2026-05-07 (Phase 2)
- `node_list` joins with `scene_content` and aggregates descendants up
  the tree. **StatusBar** shows project/chapter/scene/session-delta with
  live updates. **Binder** shows per-row word counts with k-formatting.
  "By author" remains deferred (V1.0).

### D5. Distraction-free / focus mode ✅ CLOSED 2026-05-07 (Turn A)
- Toggle in the EditorShell header + `Cmd/Ctrl+.` keybinding hides the
  Binder, Inspector, and StatusBar so only the prose remains. Toolbar
  + Cmd/K quick-actions stay reachable.

### D6. Drag-reorder in Binder ✅ CLOSED 2026-05-07 (Turn B)
- HTML5 drag-and-drop on scene rows; midpoint-rank algorithm in
  `Binder.tsx` computes a fresh LexoRank between the dropped neighbors
  and persists via the existing `nodeUpdate(position)` IPC. Cross-parent
  moves are deferred (post-MVP).

### D7. Scheduled hourly snapshots during active sessions ✅ CLOSED 2026-05-07 (Turn B)
- New `apps/desktop/src/scheduler.rs` spawns a tokio interval task from
  the Tauri `setup` callback. `AppState` tracks `last_change_at` (touched
  by every `scene_save`) and `last_auto_snap_at`; the scheduler fires
  only when the project is dirty since the last tick. Configurable
  interval defaults to 1 hour.

### D8. 100k-word fixture + cold-open <2s benchmark ✅ CLOSED 2026-05-07 (Turn C)
- New `crates/booksforge-storage/tests/cold_open_perf.rs` programmatically
  seeds 50 chapters × 5 scenes × 400 words ≈ 100 000 words, closes the
  pool, then times `open_pool + run_migrations + list_nodes_with_scene_content_consistent`
  (the same path `node_list` uses on first paint). Marked `#[ignore]` so
  it's opt-in (`cargo test --release -- --ignored cold_open`). Measured
  7 ms on the dev machine — well inside the 2 000 ms budget.

---

## E0b. Phase 5 — all 11 agents wired end-to-end ✅ CLOSED 2026-05-07 (Turn I)

The remaining 7 agents now have parser, runner method, IPC input, Tauri
command, and TS wrapper, matching the established Copyeditor/Continuity
recipe.  All 11 registered agents (10 user-visible + 1 internal Tier-2
ProposalValidator) are dispatchable end-to-end.

- **`parse_and_validate(raw, ...)`** added to: `intake`, `memory_curator`,
  `vocab_dictionary`, `chapter_drafter`, `dev_editor`, `humanization`,
  `proposal_validator`. Each runs the typed `serde_json::from_str::<T>`
  parse plus the per-type semantic `validate()` (e.g.
  `MemoryRefreshProposals::validate()`, `SceneDraftProposal::validate()`,
  `HumanizationProposals::validate(source)`).  The Proposal Validator's
  parser additionally cross-checks `verdict_from_checks(&checks) ==
  parsed.verdict` to catch verdict-aggregation hallucinations.
- **Orchestrator runners**: `run_intake`, `run_memory_curator`,
  `run_vocab_dictionary`, `run_chapter_drafter`, `run_dev_editor`,
  `run_humanization`, `run_proposal_validator_tier2`. All flow through the
  generic `runner::run`, so every call inherits Tier-1 ProposalValidator,
  retry-on-block, ledger persistence, and `VerificationReport`
  assembly.  Memory Curator's runner stamps the proposed scope into
  `proposed_memory_scopes` so the cross-cutting `MemoryScope` validator
  rejects out-of-scope writes before they hit storage.
- **IPC inputs**: `RunIntakeInput`, `RunMemoryCuratorInput`,
  `RunVocabDictionaryInput`, `RunChapterDrafterInput`, `RunDevEditorInput`,
  `RunHumanizationInput`, `RunProposalValidatorInput`. ts-rs bindings
  regenerated and re-exported from `@booksforge/shared-types`.
- **Tauri commands**: `agent_run_intake`, `agent_run_memory_curator`,
  `agent_run_vocab_dictionary`, `agent_run_chapter_drafter`,
  `agent_run_dev_editor`, `agent_run_humanization`,
  `agent_run_proposal_validator`.  Each command pulls the right context
  from storage before dispatching:
  - **Memory Curator** loads `memory_list_by_scope` + `list_entities`.
  - **Vocab Dictionary** loads `vocab_list_by_layers(["project"])`.
  - **Chapter Drafter** assembles entity bible + voice-fingerprint stub.
  - **Dev Editor** concatenates scene texts under the chapter, pulls
    book-scope brief + chapter-scope summaries from memory.
  - **Humanization** loads `vocab_list_by_layers(["project","ai_tells"])`
    so the agent has both the user's preferences and the always-on
    AI-tell list.
  - **Proposal Validator (Tier-2)** wraps the primary's output + Tier-1
    findings + active vocab + voice fingerprint; orchestrator dispatch
    point ready for caller integration in Turn J (per-edit apply paths).
- **`apps/desktop/src-ui/src/lib/ipc.ts`** wraps all 9 commands with
  typed Promises returning `AgentRunResultDto`.
- **Shared helpers** added in `commands/agents.rs`: `status_str`,
  `run_result_to_dto<T: Serialize>`, `require_open_project` consolidate
  the boilerplate so subsequent UI work doesn't re-implement them.
- **Tests**: 215 unit tests passing across foundation crates (19
  agents + 58 domain + 72 ipc + 29 orchestrator + 11 prompt + 26
  validator).  Workspace builds clean.

## E0e. Phase 5 — Turn K: originality / anti-plagiarism enforcement ✅ CLOSED 2026-05-07 (Turn K)

The system now refuses to ship plagiarised prose.  Three layers of defence,
all local-first (nothing leaves the device).  Online plagiarism API
integration is tracked separately under §E0d.11 — gated on user consent
to honour CLAUDE.md's "no manuscript content leaves the device by default"
invariant.

- **Detector** (`booksforge-validator::originality`) — pure n-gram
  detector with two pass modes:
  - `detect_verbatim_overlap(output, source, min_words)` — flags long
    spans (default ≥12 words) the agent copied from the source it was
    given.  Catches "agent copy-pasted instead of generating".
  - `detect_self_plagiarism(output, prior_corpus, min_words)` — same
    against the project's prior accepted scenes.  Catches cross-chapter
    recycling.
  - ASCII-quoted runs are treated as legitimate citations and skipped.
  - Returns `OverlapHit { kind, output_from, output_to, words, quote }`,
    deterministic order, char-offset precision so the UI can highlight.
- **Cross-cutting validator** (`CrossCuttingValidator::Originality`) —
  new variant runs in Tier-1 alongside Schema / Redaction / Length.
  Walks the output's prose-bearing fields (the same set the
  EntitySanity walker uses, plus `text` / `scene_text` / `draft`),
  joins them, and runs both detector passes against
  `RunInput.source_text` and `RunInput.prior_scene_corpus`.
  - Verdict: ≥20-word verbatim run → Fail (block).
                12–19-word run → Warn.
                no overlap above threshold → Pass.
- **Wiring** — added to:
  - Copyeditor spec (source_text = scene_text).
  - Humanization spec (source_text = scene_text).
  - Chapter-Drafter spec (source_text = scene_synopsis).
  Other agents (continuity adjudicator, dev-editor, intake, vocab,
  memory-curator, proposal-validator) don't emit prose so they skip it.
- **Prompt-guard ethics block** (`prompt_guard::render_originality_ethics`)
  — every prose-emitting agent's prompt now opens with explicit
  anti-plagiarism rules: no verbatim copying, no recycling prior
  chapters, no reproducing copyrighted text, citations require ASCII
  quotes + adjacent attribution.  Belt-and-braces with the post-hoc
  detector.
- **On-demand chapter scan** (`originality_scan_chapter` Tauri command)
  — walks every scene in a chapter and cross-checks against every
  other scene in the project.  Returns `OriginalityScanResult { hits
  [OverlapHitDto] }` with `matched_scene_id` so the UI can deep-link
  from a hit to the source scene.  Local-only, no network.
- 7 unit tests cover the detector (verbatim hit, threshold, citation
  skip, self-plagiarism, determinism, empty inputs, quote truncation).
- New IPC types: `OriginalityScanInput`, `OriginalityScanResult`,
  `OverlapHitDto`.  `ValidationAxis::Originality` variant added in
  domain.

---

## E0c. Phase 5 — Turn J: prompt-guard injection, voice pipeline, Tier-2 dispatch helper ✅ PARTIAL 2026-05-07 (Turn J)

The architectural pieces that existed only on paper after Turn G–I are now
wired into the live runner.  Three placeholders closed; eight remain
(tracked under §E0d).

- **Prompt-guard injection (J1).**  `runner::run_inner` now constructs the
  composed humanity + voice + avoid-rules block from the per-call
  `RunContext` (active vocab + voice fingerprint) and inserts it into
  `vars["prompt_guard"]` before MiniJinja render.  Every prose-emitting
  template that references `{{ prompt_guard }}` (16 templates) now sees
  the project's anti-AI-tell guidance as a hard constraint.
- **Voice fingerprint pipeline (J2).**  New
  `booksforge-orchestrator::voice_pipeline` exposes
  `load_or_default(&storage)` and `refresh_from_corpus(&storage,
  agent_id)`.  Persisted as a `MemoryEntry` in `MemoryScope::Style` under
  key `voice_fingerprint`, so every change rides the audit trail and
  `last_writer` is stamped automatically.  `agents.rs` now bundles
  fingerprint + entity bible + active avoid-rules into a single
  `RunContext` per Tauri command.  Hook for Memory Curator's
  chapter-finalise refresh is staged but not yet auto-invoked
  (tracked in §E0d).
- **Tier-2 ProposalValidator dispatch helper (J3).**
  `Orchestrator::maybe_dispatch_tier2` runs the `proposal-validator`
  agent on a completed primary's output, gated on
  `OrchestratorConfig.tier2_enabled`.  Re-assembles `VerificationReport`
  with the new Tier-2 verdict folded in; failures are non-fatal and
  preserve the Tier-1 verdict.  Default off — projects opt in via
  high-confidence mode.  Per-Tauri-command callsites pending
  (tracked in §E0d).

---

## E0d. Phase 5 — remaining placeholders identified during Turn J

The following surfaces are scaffolded but still emit dummy / empty inputs
or skip a real action.  Each is sized as a focused PR.

### E0d.1. Tier-2 dispatch wired into Tauri commands ✅ CLOSED 2026-05-07 (Turn J pt 2)
- `agent_run_copyedit`, `agent_run_humanization`, `agent_run_dev_editor`,
  and `agent_run_chapter_drafter` now call `Orchestrator::maybe_dispatch_tier2`
  after the primary returns.  No-op when `OrchestratorConfig.tier2_enabled
  = false` (default) so existing behaviour is unchanged; flips on once
  the project setting wired in a follow-up.
- The Tier-2 verdict folds into `VerificationReport` via
  `council::assemble_report` and travels in `AgentRunResultDto`.

### E0d.2. Peer-review dispatch ✅ CLOSED 2026-05-07 (Turn J pt 2)
- `Orchestrator::dispatch_peer_reviews` runs every default-on (or
  high-confidence-mode-enabled) reviewer for the primary as an
  independent agent invocation against the shared `peer-review/v1.toml`
  template.  Honours the ≤8-call cap by capping reviewers at
  `max_agent_calls - 1`.
- `Orchestrator::fold_peer_reviews_into_result` re-assembles
  `VerificationReport` with the collected `PeerReviewResult`s.
- Wired into copyedit, humanization, dev-editor, chapter-drafter Tauri
  commands.  `RunChapterDrafterInput`, `RunDevEditorInput`,
  `RunHumanizationInput` gained `high_confidence_mode: Option<bool>`.

### E0d.3. Per-reviewer peer-review prompt templates ✅ CLOSED 2026-05-07 (Turn J pt 2 — single shared template instead of seven)
- Implemented as a single shared `peer-review/v1.toml` template that's
  parameterised by `focus` (one of the seven axes) and
  `reviewer_agent_id`.  The template's system prompt contains a focus
  playbook so a single LLM call adapts to whichever axis the council
  selected.  Avoids seven near-duplicate templates whose drift would be
  expensive to keep aligned.
- New `booksforge-agents::peer_review` module (spec +
  `parse_and_validate`).  Validates that result-level verdict is at
  least as strict as concern severities (any `Error` → `Block`,
  any `Warning` → at least `Warn`).

### E0d.4. Vocab-dictionary edit-history input ✅ CLOSED 2026-05-07 (Turn J pt 2)
- New `StorageRepository::recent_applied_edits_for_project(project_id,
  kind, limit)` joins `agent_applied_edits` ⨝ `agent_tasks` ⨝
  `agent_runs` filtered to the project + edit kind, returning rows
  newest-first.
- `agent_run_vocab_dictionary` decodes each row's
  `edit_payload_json`, splits on `reverted_at` (accepted = no revert,
  rejected = revert present), and feeds both arrays to the agent.
  `RunVocabDictionaryInput.lookback` finally has effect (defaults to
  200, capped at 1000).

### E0d.5. Copyedit apply path (per-edit accept/reject) ✅ CLOSED 2026-05-07 (Turn J pt 2)
- New Tauri command `agent_apply_copyedit` (input: task_id +
  scene_id + edit_index) drives `Orchestrator::apply_copyedit_edit`:
  loads the persisted `CopyeditProposals`, takes a
  `pre_agent_edit` snapshot scoped to the scene, mutates `pm_doc`
  (rebuilt as paragraph blocks from the new flat text), saves the
  scene, and inserts one `agent_applied_edits` row with
  `edit_kind: TextReplace`.
- Per-edit idempotency: refuses a second accept of the same
  `(task_id, edit_index)` by scanning prior payloads.
- Pure logic in `apply_copyedit::apply_replacement` honours the
  agent's original char range or, if drift is detected, falls back to a
  single-occurrence substring search.  Refuses ambiguous matches.
- Storage trait gained `list_applied_edits_for_task` for the
  per-edit idempotency check.
- Known limitation: rebuilding `pm_doc` from flat text loses inline
  marks (bold / italic / links).  Acceptable for Copyeditor's
  mechanical-fix remit; mark-preserving applier tracked as a
  follow-up.

### E0d.6. Humanization apply path ✅ CLOSED 2026-05-07 (Turn J pt 2)
- New Tauri command `agent_apply_humanization` calls
  `Orchestrator::apply_humanization_edit` — same flow as E0d.5
  against `HumanizationProposals.edits`.  Distinguished in the ledger
  by `edit_payload_json.agent = "humanization"` and the
  `triggered_rule` field carried in the payload.

### E0d.7. Continuity rename / annotate apply path ✅ CLOSED 2026-05-07 (Turn K)
- New module `apply_continuity.rs` + Tauri command
  `agent_apply_continuity` (input: project_id + task_id +
  finding_index).  Three branches:
  - **Rename** — pre-edit snapshot (Scene / Project depending on the
    fix's `scope`); whole-word case-sensitive replacement of `from →
    to` across every candidate scene; one `agent_applied_edits` row
    per modified scene with `edit_kind = RenameEntity` and the scene
    id in the payload.  Whole-word matching avoids "Anna → Anya"
    rewriting "Annapurna".
  - **Annotate** — pre-edit snapshot (Project); `memory_upsert` in
    `MemoryScope::Entity` keyed by `continuity:<finding_index>`; one
    ledger row with `edit_kind = NoteAdd`.
  - **None** — refused (acknowledge-only finding has nothing to apply).
- Per-finding idempotency: refuses a second apply of the same
  `(task_id, finding_index)`.
- `ApplyContinuityResultDto` returned to the UI surfaces the kind,
  pre-snapshot id, list of applied-edit ids, and `scenes_touched`
  count.
- 5 unit tests on `whole_word_replace`.

### E0d.8. Memory-curator chapter-finalise voice-fingerprint refresh ✅ CLOSED 2026-05-07 (Turn L)
- `agent_run_memory_curator` now invokes
  `voice_pipeline::refresh_from_corpus` after every successful
  chapter-scope run.  Best-effort: a refresh failure is logged but does
  not fail the curator's run, so the user always gets the proposed
  memory upserts even if the fingerprint recompute hits a transient
  storage error.
- Stamps `agent_id = "memory-curator"` on the resulting style-scope
  memory write so the audit trail reads correctly.
- Manual refresh + load Tauri commands (`voice_fingerprint_refresh`,
  `voice_fingerprint_load`) remain available for tests and one-off
  recomputation.

### E0d.9. UI panels for the 11 agents ✅ CLOSED 2026-05-07 (Turn K)
- New `<AgentsPanel>` switchboard (toolbar button "Agents") presents
  every registered agent in four categories (prose-mutating,
  generating, memory, internal/meta) with name + blurb + an "apply"
  badge for the three agents that have apply paths.
- Three full-featured panels with per-edit Accept buttons:
  - `<CopyeditPanel>` — calls `agent_run_copyedit`, lists each edit
    with category / range / before-after / rationale, dispatches
    `agent_apply_copyedit` per edit.
  - `<HumanizationPanel>` — same shape against
    `HumanizationProposals` + `agent_apply_humanization`.
  - `<ContinuityPanel>` — lists each finding with its proposed fix
    (rename / annotate / none); Apply button dispatches
    `agent_apply_continuity` and surfaces `scenes_touched` +
    pre-snapshot id on success.
- `<GenericAgentForm>` covers the remaining 7 agents with the
  agent-specific input fields (free-text idea for intake, scope picker
  for memory-curator, lookback for vocab-dictionary, synopsis +
  purpose + POV + target words for chapter-drafter, chapter id for
  dev-editor).  Disabled with an explanatory note for the auto-invoked
  meta agents (proposal-validator + peer-review).
- Shared `<VerificationReportView>` renders Tier-1 + Tier-2 + peer
  reviews uniformly, colour-coded by verdict, used by every agent
  panel.  Concerns show severity + quote + reason + evidence; checks
  show axis + outcome + evidence + remediation.
- Wired into `EditorShell` toolbar; the panel passes the active
  scene's id so single-scene agents (copyedit, humanization,
  continuity, chapter-drafter) get the right node automatically.

### E0d.11. Online plagiarism / originality API integration (opt-in, consent-gated) ⏳ PARTIAL 2026-05-07 (Turn L — provider scaffold + consent storage closed; remote impls still pending)

**Closed in Turn L:**
- `booksforge-domain::originality_provider` defines
  `OriginalityProviderId { LocalOnly, Copyleaks, Plagscan, Turnitin }`,
  `OriginalityConsent`, `OriginalityCheckResult`.  `LocalOnly` is the
  only id that returns `false` from `sends_content_offdevice()` —
  encoded in the type so future provider authors cannot accidentally
  classify a remote provider as local.
- `booksforge-orchestrator::originality_provider` provides
  `load_consent` / `save_consent` / `clear_consent` / `active_provider`
  storing the consent record as a `MemoryEntry` in `MemoryScope::Style`
  under key `originality_provider_consent` — every consent change rides
  the existing audit trail.
- `scan_local()` is the provider-agnostic envelope around the existing
  `booksforge-validator::originality` n-gram detector.
- Tauri commands `originality_consent_load` / `originality_consent_set`
  / `originality_consent_clear` for UI integration.
- New privacy invariant test
  `default_originality_provider_is_local_only` asserts a fresh project
  defaults to `LocalOnly` — encoded as code so a misconfigured default
  cannot ship.

**Still pending (the part that requires user consent + ADR):**
- ADR documenting what's sent to which provider, retention, revocation.
- One-time-per-project consent dialog (named provider, data-flow
  summary, link to the provider's privacy policy).
- Concrete provider implementations (Copyleaks / Plagscan / Turnitin)
  living in a new `booksforge-originality-providers` crate.
- Settings UI to revoke consent + clear stored API keys.
- Gate to **export-time only**, surfacing hits in the validator gate
  rather than per-agent-run.

Until the pending items land, the local detector + ethics block + manual
scan command are the project's plagiarism defences and the consent
storage is in place but always points at `LocalOnly`.

The local detector (§E0e) catches verbatim copying from the project's
own corpus and from inputs the agent saw.  It cannot catch overlap with
text the project doesn't have on disk — paraphrased copying from
external books, articles, or web pages.  An online plagiarism API
(Copyleaks, Plagscan, Turnitin Originality, etc.) closes that gap, but
sending manuscript content to a third party directly conflicts with
CLAUDE.md privacy invariant 1: *"No content leaves the device by
default."*  Work needed to honour both:

1. **ADR** documenting the decision and the user-facing contract.
   What's sent, to whom, retained for how long, how the user revokes.
2. **One-time-per-project consent dialog** modelled on the AI capability
   consent gate: explicit checkbox, named provider, summary of data
   flow, copy of the provider's privacy policy.  Stored as a
   `MemoryEntry` under `Style` scope key `originality_provider_consent`
   so it audits like everything else.
3. **Provider abstraction** (`booksforge-originality` crate, L4) with
   a single trait — concrete impls per provider, picked at runtime by
   project setting.  No-op `LocalOnly` impl for users who decline.
4. **Integration point**: gate to *export time only* (not on every
   agent run) — chapter is final, run a full-manuscript scan as a
   pre-export validator, surface hits in the existing validator
   gate UI.  Agent-time invocation would multiply API cost + privacy
   exposure with little additional value over the local detector.
5. **Privacy invariant test** to assert `LocalOnly` is the default and
   no other provider activates without an entry in
   `originality_provider_consent`.
6. **Settings UI** to revoke consent + clear stored API keys.

Until this lands, the local detector + ethics block + manual scan
command are the project's plagiarism defences.  Document in user help
that "originality protection is local-only by default — see Settings →
Originality to opt into an online check at export time."

### E0d.10. Vocab promotion UI gate ✅ CLOSED 2026-05-07 (Turn L)
- New Tauri command `vocab_apply_proposals(task_id,
  accepted_addition_indices, accepted_modification_indices)`.  Loads
  the persisted `VocabUpdateProposals` from `agent_outputs`, then
  upserts the user-accepted rows into the project layer with
  `EntrySource::Agent`.
- Modifications target only the **project** layer (shipped starter
  dictionaries + `ai_tells` are immutable from agents); rows whose
  target term doesn't exist in the project layer are skipped and the
  count surfaces in `VocabApplyResult`.
- New `<VocabDictionaryPanel>` in the agents switchboard:
  - Lookback control (default 200, max 1000) drives how much edit
    history feeds the run.
  - Per-row checkboxes plus "Select all / Select none" for both
    additions and modifications.
  - "Promote selected to project layer" button shows applied /
    skipped counts on success.
  - Inline `<VerificationReportView>` so the user sees Tier-1 +
    optional Tier-2 + peer-review verdicts before promoting.
- Wired into `<AgentsPanel>`; the vocab-dictionary card now carries
  the "apply" badge.

---

## E0a. Phase 5 — all 9 prompt templates, generic runner, Copyedit + Continuity end-to-end ✅ CLOSED 2026-05-07 (Turn H)

- **Cleanup.** Deleted obsolete Turn-D prompt templates (`copyeditor/v1.toml`,
  `continuity/v1.toml` in their old wire-format) and stale `bindings/`
  copies under `crates/booksforge-ipc/bindings/` (`AgentFindingDto.ts`,
  `AgentFindingsResult.ts`, `RunCopyeditInput.ts`, `RunContinuityInput.ts`).
- **9 prompt templates** with canonical schemas + prompt-guard injection
  hook (`{{ prompt_guard }}`):
  - `copyeditor/v1.toml` — concrete edit pairs, reads StyleBook, capped 30.
  - `continuity/v1.toml` — adjudication-only, kind enum, evidence array.
  - `intake/v1.toml`, `memory-curator/v1.toml`, `vocab-dictionary/v1.toml`,
    `chapter-drafter/v1.toml`, `dev-editor/v1.toml`, `humanization/v1.toml`,
    `proposal-validator/v1.toml` — canonical AGENTS.md schemas.
- **Generic typed-output runner** (`booksforge-orchestrator::runner`) —
  agent-agnostic. Handles prompt render, ledger insertion, streaming
  Ollama call with cancel + timeout, retry loop, **Tier-1 ProposalValidator
  invocation on every successful parse**, **Council assembly**
  (Tier-1 + optional Tier-2 + peer reviews), persists `agent_outputs`.
  Returns `AgentRunResult<T>` with the typed proposal + full
  `VerificationReport`.
- **Copyeditor end-to-end:** `Orchestrator::run_copyedit_scene` loads
  scene + StyleBook + entity bible from storage, calls the runner with
  `CopyeditProposals::validate(source)` enforcing the four invariants
  (before-matches-source, ≤10 % word-count change, no overlap, category
  in enum), returns the typed proposal + verification report.
- **Continuity end-to-end:** `Orchestrator::run_continuity_adjudication`
  receives the deterministic linter's ambiguous findings (the Tauri
  command runs `lint_scene` first), batches into the LLM with
  surrounding excerpts, returns a `ContinuityReport` with rename /
  annotate / none fixes scoped per finding.
- **IPC types:** `AgentRunResultDto`, `VerificationReportDto`,
  `ProposalValidationDto`, `ValidationCheckDto`, `PeerReviewResultDto`,
  `PeerReviewConcernDto`, `RunCopyeditInput`, `RunContinuityInput`.
  TS bindings regenerated and re-exported from `@booksforge/shared-types`.
- **Tauri commands:** `agent_run_copyedit`, `agent_run_continuity`
  registered in `apps/desktop/src/lib.rs`.  Per-agent runner wrappers
  pull `load_style_book`, `list_entities`, `lint_scene` in-process so
  the council has accurate context.
- **Tests:** 11 prompt smoke tests cover all 16 templates (every new
  template has a render assertion); 65 ipc tests round-trip the new
  DTOs through ts-rs; 29 orchestrator tests include the runner's
  fence-stripping plus existing council/cross-cutting/proposal-validator
  coverage.  208 foundation tests passing total.

## E0. Phase 5 cross-verification council, voice fingerprint, prompt-guard ✅ CLOSED 2026-05-07 (Turn G)

Foundation for inter-agent cross-verification and high-quality, non-AI-sounding output. Agents remain stateless per `AGENTS.md §1`; communication is orchestrator-mediated.

- **Council protocol** (`booksforge-domain::council`) — typed `PeerReviewRequest`, `PeerReviewResult`, `PeerReviewConcern`, `PeerReviewFocus` (7 axes), and `VerificationReport` aggregator. Static `peer_reviewers_for(agent_id)` table encodes who reviews whom: chapter-drafter pairs with memory-curator (memory_consistency), continuity (name_pov_preservation), humanization (ai_tell_residue) on default-on; final-review-editor pairs with humanization (ai_tell_residue) + memory-curator (fact_fidelity); etc. Eleven default pairings across the catalog.
- **Council module** (`booksforge-orchestrator::council`) — pure-logic `select_pairings(agent_id, high_confidence_mode)`, `assemble_report(...)`, `should_retry_primary(...)`. Verdict aggregation conservative: any `Block` → `Block`. Non-recursive: a reviewer can't trigger its own peer reviewers. Peer reviews count toward the 8-call workflow cap.
- **Voice fingerprint** (`booksforge-domain::voice`) — six structural signals (sentence cadence mean+stddev, em-dash rate, ly-adverb rate, AI-tell-triad rate, discourse-marker rate, type-token ratio) computed by `VoiceFingerprint::compute()`. Anti-AI-tell intent baked in: signals known to distinguish human from LLM prose. `is_established()` returns false below 2 000 tokens.
- **Prompt-guard layer** (`booksforge-orchestrator::prompt_guard`) — assembled at orchestrator-binding time and injected into every prose-emitting agent's rendered prompt. Three composed blocks: humanity & empathy (static, 6 rules), voice fingerprint (per-project, concrete cadence/em-dash/triad-avoidance targets), and active vocab avoid/replace rules (numbered watch-list with rationale). Truncates avoid lists to 40 entries to fit context.
- **Agent role refinements** — every spec's `purpose` rewritten to be sharper, role-specific, and anti-AI-tell-aware. Copyeditor: "Mechanical fixes only — never rewords." Humanization: "Detect AI-tells… grounded in the project's voice fingerprint." Chapter-Drafter: "Writes in the project's established voice (per VoiceFingerprint)." Final-Review-Editor: "preserving the author's voice fingerprint and every established fact."
- **AGENTS.md §5.1, §6.5** added documenting the council, prompt-guard, voice-fingerprint, and the full pairing matrix.
- 28 orchestrator unit tests (8 prompt-guard, 4 council, 3 proposal-validator, 6 cross-cutting, 7 retained); 58 domain unit tests (4 voice, 6 council, 5 agent_io, 43 retained).

## E. M2 — First three agent workflows end-to-end

Per `IMPLEMENTATION_PLAN.md §4 M2`. **Foundation work landed** in Phase 5
(2026-05-07): all 11 registered agents (10 user-visible MVP + 1 internal
Proposal Validator) now use the canonical 12-field `AgentSpec` shape from
`AGENTS.md §3`, with typed input/output schema ids referencing
`booksforge-domain::agent_io` (CopyeditProposals, ContinuityReport,
MemoryRefreshProposals, VocabUpdateProposals, SceneDraftProposal,
DevelopmentalNotes, HumanizationProposals, FinalReviewOutput,
ProjectBrief, OutlineProposal, ProposalValidation), explicit
`failure_modes` slices, and a `validators` slice naming the cross-cutting
validators to run (Schema / Redaction / Length / EntitySanity /
MemoryScope). The Tier-1 ProposalValidator
(`booksforge-orchestrator::proposal_validator::run_tier1`) already
aggregates these into a `ProposalValidation` verdict on every primary
agent call. Per-agent runners and templates are still missing for 7 of
the 11 — that's what the E/F sections below track.

### E1. `IntakeAndOutline` workflow ✅ CLOSED 2026-05-07 (Turn O)
- New `Orchestrator::run_intake_and_outline()` chains intake →
  outline-architect.  Counts as 2 of the workflow's ≤8 calls per run;
  refuses with `AgentCallLimitExceeded` if the project's config
  tightens the cap below 2.
- Cancel-token-aware: a user cancel between calls aborts cleanly
  without dispatching the outline call.
- Returns `IntakeAndOutlineResult` with both halves so the UI can
  show the brief above the outline; intake failure surfaces as
  `outline_status: "skipped"` rather than swallowing the error.
- Tauri command `agent_run_intake_and_outline` + IPC types
  `RunIntakeAndOutlineInput` / `RunIntakeAndOutlineResult`.
- New `<IntakeAndOutlinePanel>` UI surfaces the chain as a single form
  with idea text + chapter count + genre overlay + preferred mode;
  routed from the agents switchboard as "Brief → Outline (chained)"
  in the generating category.

### E1.legacy. `IntakeAndOutline` workflow (real, not just outline-architect)
- Wire the `intake` agent: takes a free-text idea → `ProjectBrief`.
- Chain: intake → outline-architect → preview → apply (already have
  outline → tree).

### E2. `Copyedit` workflow ✅ CLOSED 2026-05-07
- **Re-opened from Turn D claim.** The Turn D implementation drifted
  from `AGENTS.md §4.6` — emitted a generic findings list with
  `severity` codes instead of the canonical `CopyeditProposals { edits:
  [{range_from, range_to, before, after, category, rationale}], summary }`
  shape with strict `before`-matches-source / ≤10 % word-count change /
  no-overlap validators. Drift removed in the Phase 5 foundation turn
  (Tauri commands, IPC types, UI panel deleted; see commit log).
- **Now closed:** the canonical `AgentSpec` (12-field shape with input
  `CopyeditorInput` and output `CopyeditProposals` schema ids), the
  `CopyeditProposals::validate(source)` semantic validator with all four
  AGENTS.md invariants enforced, and the `copyeditor` agent module
  (parser + category enum coercion + failure-mode catalogue).
- **Closed across subsequent turns:** the prompt template lives at
  `crates/booksforge-prompt/templates/copyeditor/v1.toml` (canonical
  schema; v2 was reserved for a future schema break and never
  needed).  Orchestrator `run_copyedit_scene` runner is wired to
  `proposal_validator::run_tier1` via the standard `RunInput.validators`
  path; Tauri commands `agent_run_copyedit` / `agent_apply_copyedit`
  are registered in `apps/desktop/src/lib.rs`; the per-edit apply
  path takes a pre-edit snapshot and inserts an
  `agent_applied_edits` ledger row (`apply_copyedit.rs`).
- **Closed this turn — UI affordance:** `<CopyeditPanel>` now
  surfaces an "Accept all by category" bar above the edit list that
  shows pending/total per category and applies edits sequentially
  (matching how the Rust apply path recomputes ranges after each
  accept).

### E3. Context builder with token budgeting ✅ CLOSED 2026-05-07 (Turn O)
- New `booksforge-orchestrator::context_builder` module:
  pure-logic, no I/O.  Takes `AvailableContext` (entity bible + active
  avoid-rules + voice fingerprint + memory entries + focus excerpt +
  prior-scene excerpts) plus a `budget_tokens` cap and returns a
  `BuiltContext` that fits — anything that doesn't fit is dropped
  with a per-section count surfaced in `BuiltContext.dropped`.
- Greedy + priority-ordered: voice fingerprint first (small +
  load-bearing), then entity bible, avoid-rules, focus excerpt,
  memory, prior-scene excerpts.  Caller is responsible for ranking
  within each section.
- `estimate_tokens()` uses a conservative `chars / 3.6` ratio
  (slightly under-counts so we waste a bit of budget rather than
  overflow); `build_with_ratio` lets non-English projects override.
- Focus-excerpt truncation kicks in only when the excerpt itself
  is the only thing keeping us over budget — `focus_was_truncated()`
  exposes this so the UI can surface a "your scene was longer than
  the model's context window — only the first N words were used"
  hint.
- 7 unit tests covering the priority ordering, drop diagnostics,
  truncation path, empty-input edge case, and the per-type token
  estimators.
- Future wiring: agent runners (chapter-drafter, dev-editor,
  copyeditor) will call `build()` to assemble the `RunContext`
  payload from raw storage data, replacing the current "shove
  everything in vars and hope it fits" path.  Tracked as a follow-up
  per-agent migration so the rollout is incremental.

### E3.legacy. Context builder with token budgeting
- Pure-logic module that selects memory excerpts + style + vocab slices
  to stay within an agent's `ContextBudget`.

### E4. Live run UI ✅ CLOSED 2026-05-07 (Turn P)
- New `<LiveRunOverlay>` floating bottom-right card listing every
  in-flight agent run with elapsed time + Cancel button.  Reads
  `agent-run-started` / `agent-run-completed` Tauri events; clears
  when the run resolves.
- Backend: `begin_agent_run` / `end_agent_run` helpers in
  `commands/agents.rs` wrap every `agent_run_*` command (11 of them),
  generating a frontend-visible `run_id`, registering a CancelToken
  in the existing `AppState.jobs` registry, emitting
  `agent-run-started` before dispatch and `agent-run-completed`
  (with status `completed | cancelled | error`) after.
- New `agent_cancel(AgentCancelInput { run_id })` Tauri command
  flips the registered token; idempotent for unknown ids so the
  overlay can fire it without checking liveness.
- New IPC types `AgentRunStartedEvent`, `AgentRunCompletedEvent`,
  `AgentCancelInput` shipped through the codegen drift gate.
- Cancel propagates through the existing `CancelToken` plumbing in
  `runner::run` — in-flight Ollama HTTP requests tear down at the
  next await point.  The overlay clears within ~500 ms of cancel.
- Per-token streaming progress (e.g. "342 tokens · 18 t/s")
  ✅ **landed in Turn R**:
  - `RunInput` gained an `on_token: Option<Arc<dyn Fn(&str) + Send + Sync>>`
    field; the runner's TokenSink fans tokens out to it.
  - `start_token_progress_emitter` Tauri-layer helper creates an
    `AtomicU64` counter + spawns a 250 ms tokio interval task that
    emits `agent-run-progress` events with cumulative tokens and
    elapsed-ms.  Stop function flips a flag so the task exits.
  - Wired into chapter-drafter, dev-editor, and developmental-review
    Tauri commands (the long-running ones).  Other agents finish fast
    enough that elapsed time + cancel is sufficient — no progress
    emit, by design.
  - Frontend overlay reads the events and renders "342 tokens · 18.2 t/s"
    above the run-id line.  Backend-sourced elapsed_ms keeps the t/s
    rate stable on long runs (no clock-skew drift).
  - New IPC type `AgentRunProgressEvent { run_id, tokens, elapsed_ms }`.

### E4.legacy. Live run UI
- Per-workflow progress panel (events keyed by `agent_runs.id`) showing
  current agent, retries, token totals, cancel button. Mirrors what
  `ai_suggest` does for a single call.

### E5. Output validators per agent ✅ CLOSED 2026-05-07
- New `booksforge_validator::agent_outputs` module — single-entry
  semantic validation for agent outputs.  `validate_agent_output(
  schema_id, parsed_json, ctx)` routes the parsed JSON to the
  canonical `::validate()` on the matching domain type
  (`CopyeditProposals::validate`, `ContinuityReport::validate`,
  `OutlineProposal::validate`, `SceneDraftProposal::validate`,
  `DevelopmentalNotes::validate`, `MemoryRefreshProposals::validate`,
  `VocabUpdateProposals::validate`, `HumanizationProposals::validate`)
  and returns a uniform `AgentOutputReport { schema_id, validated,
  parse_ok, errors }`.
- Unknown schema ids report `validated: false` rather than erroring,
  so callers can distinguish "validator wired and clean" from
  "no validator yet" from "validation failed".
- Cross-cutting validators (Schema / Contract / Redaction / Length /
  Originality) continue to live in `booksforge_orchestrator::
  proposal_validator::run_tier1`; this module is the *semantic* tier.
- 6 unit tests in `agent_outputs::tests` — unknown schema id, copyedit
  routing, scene-draft validation, vocab kind enum, typed-parse
  failure surface, and the clean copyedit path.

---

## F. M3 — Developmental + continuity

### F1. Deterministic continuity linter (`booksforge-validator`) ✅ CLOSED 2026-05-07 (Phase 5 foundation)
- New `booksforge-validator/src/continuity.rs` ships four detectors that
  walk a single scene's plain text:
  - **`detect_name_drift`** — capitalised tokens not in the entity bible
    (canonical name + aliases + a 30-entry common-proper allowlist);
    Levenshtein-≤2 close matches are marked `ambiguous: false` so they
    bypass the LLM, others go to adjudication.
  - **`detect_pov_drift`** — pronoun-ratio heuristic against
    `project_pov` (`first` / `third-*`); silent when total pronouns < 5.
  - **`detect_tense_drift`** — paragraph-by-paragraph past-vs-present
    ratio with a flip detector and a non-past `-ed` allowlist
    (`indeed`, `embed`, …) so `seed`/`feed` don't poison signal.
  - **`detect_timeline`** — within-paragraph contradictory phrase pairs
    (`yesterday`+`tomorrow`, `last night`+`next week`, etc.).
- Public `lint_scene()` runs all four and returns findings sorted by
  position. 8 unit tests covering positive flags, alias-aware silence,
  Levenshtein-close-match ambiguity, sort order, short-scene silence.
- The Continuity LLM agent (§4.5) now receives only `ambiguous: true`
  findings — the deterministic half handles the rest with zero tokens.

### F2. `DevelopmentalReview` workflow ✅ CLOSED 2026-05-07 (Turn Q)
- New `Orchestrator::run_developmental_review()` chains:
  1. **One LLM call** — `dev_editor` over the concatenated chapter
     text returning `DevelopmentalNotes` (six axes: pacing, stakes,
     character, POV tension, theme, structural balance).
  2. **Per-scene deterministic linter** — runs
     `booksforge_validator::lint_scene` on every scene under the
     chapter for name / POV / tense / timeline drift.  Free — no
     LLM budget consumed, runs in milliseconds even on long chapters.
- Returns `DevelopmentalReviewResult` carrying the dev notes plus
  per-scene `ContinuityScenePass` rows (only scenes with findings —
  clean scenes are omitted to keep the report tight).
- Tauri command `agent_run_developmental_review` + IPC types
  `RunDevelopmentalReviewInput` / `RunDevelopmentalReviewResult` /
  `ContinuityScenePassDto`.
- New `<DevelopmentalReviewPanel>` in the agents switchboard:
  - Chapter id + optional POV input.
  - Dev-editor's six-axis notes shown with axis badges, severity
    colours, and inline suggestions.
  - Per-scene continuity findings nested with kind / severity /
    excerpt / ambiguous flag.

### F2.legacy. `DevelopmentalReview` workflow
- Per-chapter structural notes with the `dev-editor` spec.

### F3. `ContinuityCheck` workflow ✅ CLOSED 2026-05-07
- **Re-opened from Turn D claim.** The Turn D implementation drifted
  from `AGENTS.md §4.5` — emitted flat findings with free-form `CONT-*`
  codes instead of the canonical `ContinuityReport` with `kind` enum
  (`name_drift | pov_drift | tense_drift | timeline | other`),
  `evidence: [{node_id, range_from, range_to, excerpt}]`, and
  `proposed_fix: {kind, from, to, scope}`. Skipped the
  deterministic-linter-first hybrid required by §7.4. Drift removed in
  the Phase 5 foundation turn.
- **Now closed:** the deterministic linter (F1, see above) is the
  required first half. The Continuity LLM agent's `AgentSpec`
  references `ContinuityAdjudicationInput` / `ContinuityReport` schema
  ids; the `continuity` agent module parses the canonical wire shape
  (kind enum + evidence array + proposed_fix); `ContinuityReport::validate()`
  enforces ULID node_ids, range ordering, ≤200-char excerpts, and
  rename-target completeness.
- **Closed across subsequent turns:** prompt template at
  `crates/booksforge-prompt/templates/continuity/v1.toml` (canonical
  schema, no v2 needed); orchestrator
  `run_continuity_adjudication` runs the deterministic linter first
  (`F1`) then sends ambiguous findings to the LLM and merges the
  output; Tauri commands `agent_run_continuity` /
  `agent_apply_continuity` are registered; the apply path supports
  rename and annotate fix-kinds and writes an `agent_applied_edits`
  ledger row.
- **Closed this turn — kind-grouped UI:** `<ContinuityPanel>` now
  shows a kind chip bar above the findings list — each chip displays
  its total / actionable count and offers an inline "apply N"
  button that walks every actionable finding of that kind (rename
  or annotate) sequentially.  A chip click filters the list to a
  single kind for review.

### F4. Entity bible auto-extraction + alias handling ✅ CLOSED 2026-05-07 (Turn Q)
- New `entity_bible_apply_proposals(task_id, accepted_indices)` Tauri
  command: loads the persisted `MemoryRefreshProposals` from
  `agent_outputs`, parses each accepted `EntityStub.kind` to
  `EntityKind` (case-insensitive: `character`/`person`/`people` →
  `Character`; `location`/`place` → `Location`; `item`/`object`/`artifact`
  → `Item`; `organisation`/`organization`/`org`/`group`/`faction` →
  `Organisation`; `theme` → `Theme`; `custom`/`other` → `Custom`).
- Stubs whose kind doesn't map to a known variant are skipped — the
  result's `skipped` count surfaces in the UI ("Inserted 3, skipped 1").
- Aliases preserved verbatim from the stub.
- New `<EntityBiblePanel>` mirrors `<VocabDictionaryPanel>` UX: run
  memory-curator in entity-extraction mode, review checkboxes,
  Select all / Select none, "Promote to bible" button.  Routes from
  the agents switchboard as "Entity Bible (auto-extract)" in the
  memory category with the apply badge.

### F4.legacy. Entity bible auto-extraction + alias handling
- Memory Curator spec wired; populates `entities` + `entity_aliases`.

---

## G. M4 — Templates + validators

### G1. Three project templates ✅ CLOSED 2026-05-07 (Turn C)
- New `apps/desktop/src-ui/src/lib/projectTemplates.ts` declares three
  starter trees (`generic-novel`, `romance`, `non-fiction`) plus
  `blank` (no-op). The `NewProjectWizard` Step 4 now exposes a
  template-picker `<select>`; after `project_create` succeeds the wizard
  walks the tree depth-first and issues `node_create` calls per row,
  pre-seeding chapters/scenes/parts with titles, status, target words,
  and beat names.  TOML-side starter vocabularies remain a Phase 5/6
  task on top of `booksforge-template`.

### G2. ≥15 manuscript validators ✅ CLOSED 2026-05-07 (Phase 4)
- 16 validators shipped in `booksforge-validator/src/validators.rs`:
  double-spaces, trailing-whitespace, multiple-blank-lines, em-dash-style,
  quote-style, unmatched-quotes, ellipsis-form, heading-hierarchy,
  missing-alt-text, broken-links, orphan-chapter, very-short-scene,
  very-long-scene, untitled-node, ai-tells-detected (vocab-driven),
  vocab-replace-pending. All deterministic, pure-logic, ≤1s on a
  6,000-word scene. 9 unit tests + 4 integration tests.

### G3. KDP-eBook validator ✅ CLOSED 2026-05-07
- **Closed in Turn B:** the metadata-only KDP checks
  (`kdp-metadata` validator, codes KDP01–KDP05) — empty title, empty
  author, missing/invalid BCP-47 language tag, malformed ISBN.
- **Closed this turn — post-build structural checks:** new
  `booksforge_export_epub::kdp_checks::run_kdp_checks(&[u8])` walks the
  built EPUB byte stream and produces structural findings:
  - **KDP06 / KDP07** — total file size (rejects >650 MB; warns >50 MB
    where Amazon's delivery cost reduces the royalty tier).
  - **KDP08** — archive does not re-open as a valid ZIP.
  - **KDP09** — embedded image entries above 5 MB (KDP downsamples).
  - **KDP10 / KDP11** — no `properties="cover-image"` in the OPF
    package document (or no OPF at all).
  - **KDP12 / KDP13** — `nav.xhtml` missing or lacks
    `<nav epub:type="toc">`.
- Wired into the unified `export_run` for the `kdp_ebook` profile via
  `run_kdp_checks_for(path)`: findings fold into `validation_message`
  alongside any EPUBCheck output, and `validation_ok` flips false if
  any KDP-Error finding fires.
- 4 unit tests in `kdp_checks` cover non-ZIP rejection, oversized
  image detection, missing-cover/nav detection, and the clean path.

### G4. Pre-export validator gate ✅ CLOSED 2026-05-07 (Phase 4)
- Errors block, warnings prompt, info silent — wired into
  `booksforge_domain::pre_export_gate` and surfaced via the
  `validators_gate` Tauri command. The Markdown Export button in
  `EditorShell` runs the gate first; an error count blocks export and
  opens the ValidatorPanel; warnings show a `window.confirm` prompt.

### G5. One-click fixes for deterministic issues ✅ CLOSED 2026-05-07 (Turn C)
- The `Validator` registry now carries an optional `fix` function pointer
  alongside the lint pass. A new `apply_fix` dispatcher and the
  `walk_text_nodes_mut` helper let fixes mutate text leaves in place
  without disturbing marks or block structure. Five fixes shipped:
  `fix_double_spaces`, `fix_trailing_whitespace`, `fix_multiple_blank_lines`,
  `fix_em_dash_style`, `fix_vocab_replace`. New `validators_apply_fix`
  Tauri command loads the scene, runs the fix, persists, touches the
  dirty timer (so D7 picks up the change), and the `ValidatorPanel`
  exposes a per-issue "Fix" button on rows where `auto_fixable && node_id`.

---

## H. M5 — Export pipeline

### H0. Markdown export ✅ CLOSED 2026-05-07
- Closed by Phase 1 — `booksforge-export::manuscript_to_markdown` renders
  the whole tree (parts → chapters → scenes) in LexoRank document order;
  `pm_doc_to_markdown` handles paragraphs, headings, lists, blockquotes,
  bold/italic/code/link inline marks. Tauri command `export_markdown`
  writes atomically and ledgers an `exports` row with `profile = markdown`.
  4 unit tests + 1 integration path. Migration `0004_exports_markdown.sql`
  widens `exports.profile` CHECK to allow `'markdown'`.

### H1. Pandoc sidecar + DOCX/PDF export ⏳ PARTIAL 2026-05-07 (Phase 6 — wrapper + profile mapping closed; binary bundling pending §M4)

**Closed in Phase 6:**
- `booksforge-export-pandoc` ships a real subprocess wrapper around
  `pandoc`: `args_for_profile` maps `Docx` / `TradePdf5x8` / `TradePdf6x9`
  to the right CLI flags; `run_pandoc` spawns the binary with the
  manuscript piped over stdin (no temp file), captures stderr for
  diagnostics, verifies output exists, computes blake3.
- `pandoc_on_path()` resolves `pandoc` from PATH (developer flow); the
  shipping build will route through Tauri's sidecar resource resolver
  once §M4 lands.
- `probe_pandoc()` returns the version string for settings UI display.
- Export-run command surfaces "Pandoc not found on PATH" with a clear
  install hint when the binary is missing.
- 7 unit tests on the wrapper (argv mapping for each profile,
  missing-binary rejection, unsupported-profile rejection).

**Pending:**
- Bundle Pandoc 3.x as `binaries/pandoc-<triple>` and re-add to
  `tauri.conf.json` sidecar list (tracked under §M4).
- DOCX reference template + per-profile geometry overrides.

**Turn M update — dependency discovery added:** `export_check_dependencies()`
Tauri command probes Pandoc / Java / EPUBCheck JAR via env vars +
PATH + standard install locations, returns a typed
`ExportDependencyReport` so the UI can show "needs pandoc" badges
with install hints instead of a silent failure on dispatch.  Resolution
order documented in `commands/export.rs`: env override > JAVA_HOME >
PATH lookup.

## H8. Export formatting polish ✅ CLOSED 2026-05-07 (Turn S)

**EPUB CSS (`booksforge-export-epub::BOOK_CSS`).**  Upgraded from a
6-line basic stylesheet to ~80 lines of trade-paperback typography:

- Indented body paragraphs with no-indent after a heading or scene
  break (the trade convention).
- Centred chapter titles with proper page-break-before so each
  chapter starts on a fresh recto page.
- Scene breaks render as `* * *` via `<hr>` or
  `<p class="scene-break">`.
- Optional drop-cap (`<p class="drop">`) — sans-serif first letter,
  3.4em tall, floating left.
- Italic blockquotes with comfortable margin indents.
- Hyphenation hints, orphan/widow control, justified text.
- Front-matter section helpers (centred title, muted copyright).
- Internal anchor links styled as text-with-underline rather than
  web-blue.

**EPUB structure.**

- Auto-injected `title-page.xhtml` as the first spine entry when the
  caller doesn't supply their own front-matter.  Renders the book
  title centred, "by <authors>" italicised, optional publisher line
  in muted copyright style.
- Every chapter wrapped in `<section epub:type="bodymatter">` for
  accessibility and navigation; the title page uses
  `epub:type="frontmatter"`.

**PDF geometry (`booksforge-export-pandoc::push_pdf_geometry`).**
Switched from generic LaTeX `book` class to `memoir` with proper
trade-book conventions:

- `documentclass=memoir`, `classoption=twoside,openright` —
  chapters start on the recto page (right-hand) by tradition.
- Asymmetric gutter (inner 0.75in / outer 0.5in) sized for KDP's
  novel-length-binding minimums.
- 11pt Georgia at 1.15 line-stretch — the paperback default.
- `\usepackage{microtype}` for typographic refinement;
  `\widowpenalty=\clubpenalty=10000` to suppress orphans/widows.
- `--toc --toc-depth=2` and
  `--top-level-division=chapter` so H1 headings drive chapter starts.

**DOCX reference template.**  New `resolve_docx_template()` in the
desktop export command checks two locations:

1. `BOOKSFORGE_DOCX_TEMPLATE` env override (developer flow).
2. `<bundle>/exports/templates/reference.docx` per-project file.

Authors who supply a styled `reference.docx` (Word "save as
template" → Pandoc `--reference-doc=`) get every paragraph, heading,
and inline mark mapped to their template's named styles.  Default
flow without a template still produces a clean DOCX.

### H2. `booksforge-export-epub` (canonical EPUB-3 pipeline) ✅ CLOSED 2026-05-07 (Phase 6)
- Real EPUB-3.2 packager — pure Rust, zero external sidecars.
  `build_epub_bytes()` produces an in-memory archive, `build_epub()`
  writes it atomically via a `tokio::task::spawn_blocking` hop.
- Archive layout matches the EPUB spec: STORE-mode `mimetype` first,
  `META-INF/container.xml`, OPF package document, `nav.xhtml`,
  `styles/book.css`, `OEBPS/text/chapter-NNN.xhtml`.
- `manuscript_to_html_chapters()` in `booksforge-export` walks the
  node tree and produces one `HtmlChapter` per Chapter node; Parts
  roll up into the next chapter as a section heading; FrontMatter and
  BackMatter become tagged `<section epub:type>` chapters.
- New `pm_doc_to_html()` mirrors the existing `pm_doc_to_markdown()`
  so scene bodies render with bold / italic / code / links / lists /
  blockquotes preserved.
- **Determinism:** all entries use the ZIP epoch (1980-01-01) as
  modified-time, file order is fixed, compression methods are
  stable.  Identical inputs produce byte-identical output —
  `identical_inputs_produce_byte_identical_output` test enforces this.
- 9 unit tests including determinism, mimetype-first-and-stored,
  metadata validation, and a round-trip-through-disk integration.

### H3. EPUBCheck sidecar ⏳ PARTIAL 2026-05-07 (Phase 6 — wrapper + JSON parser closed; JAR bundling pending §M4)

**Closed in Phase 6:**
- `booksforge-epubcheck` ships a real subprocess wrapper that spawns
  `java -jar epubcheck.jar --json - --quiet <epub>`, captures stdout,
  parses the JSON report into typed `EpubCheckIssue` rows.
- `EpubCheckReport::is_valid()` is the strict gate (no ERROR / FATAL);
  `error_count()` and `warning_count()` separate severities.
- `parse_report()` is pure-logic and handles edge cases: empty stdout,
  invalid JSON, unknown severity strings, missing locations.
- The export command runs EPUBCheck **opt-in**: when Java + the JAR
  are configured (env `BOOKSFORGE_EPUBCHECK_JAR` + `java` on PATH),
  validation runs and the verdict surfaces in `ExportRunResult`.  When
  not configured, the export still succeeds and the result carries a
  "local-only — not validated" message so the UI can prompt the user.
- 7 unit tests covering JSON parse, severity gating, empty reports,
  and missing binary detection.

**Pending:**
- Bundle EPUBCheck 5.x as a Tauri sidecar resource (§M4).
- "Mandatory pass" mode for production exports (currently advisory).
- Settings UI to surface validation status + jump-to-issue links.

**Turn M update — same `export_check_dependencies()` command above
also reports EPUBCheck JAR + Java availability, so the UI can prompt
"install Java + set BOOKSFORGE_EPUBCHECK_JAR to enable validation"
without dispatching a failed export.

### H4. Export profiles ✅ CLOSED 2026-05-07 (Phase 6 — all 4 profiles routed through `export_run`)
- New `export_run(profile, output_path)` Tauri command unifies all 6
  profiles (Markdown, Generic EPUB, KDP eBook, DOCX, Trade PDF 5×8,
  Trade PDF 6×9) behind one entry-point.  Routes:
  - `markdown` → in-process renderer (existing).
  - `generic_epub` / `kdp_ebook` → in-process EPUB packager + opt-in
    EPUBCheck.
  - `docx` / `trade_pdf_*` → Pandoc subprocess.
- KDP vs generic EPUB diverges only in metadata (`dcterms:modified`
  policy) — both produce a valid EPUB-3.2 archive.
- New `<ExportPanel>` UI surfaces all profiles with a radio selector,
  per-profile blurbs and dependency hints (e.g. "needs pandoc"),
  EPUBCheck verdict line, and the live history list.

### H5. Reproducibility tests ✅ CLOSED 2026-05-07 (Turn M — full pipeline determinism enforced)
- New `crates/booksforge-export-epub/tests/reproducibility.rs`
  integration test runs the full pipeline (fixture project →
  `manuscript_to_html_chapters` → `build_epub_bytes`) twice and asserts
  byte-identical output via blake3 hash.
- Fixture is non-trivial: 2 parts, 4 chapters, 8 scenes with mixed
  inline marks (bold / italic / link / code) so all per-block /
  per-inline rendering paths are exercised.
- Sanity counter-test verifies that *changing* metadata (book_id)
  *does* change the output, so the determinism assertion isn't
  accidentally trivial.
- Stable ULIDs via `Ulid(u128)` literals so BTreeMap iteration order
  is stable across runs, machines, and CI matrix entries.
- Pandoc DOCX/PDF reproducibility is upstream-bound (Pandoc itself is
  non-deterministic in some cases); tracked as a follow-up under §C5
  rather than blocking here.

### H6. Visual regression (preview vs. unzipped EPUB content HTML) ✅ CLOSED 2026-05-08 (Phase 4 — Playwright pixel-diff harness shipped)
- **Closed this turn — content-level scaffold:** new integration test
  `crates/booksforge-export-epub/tests/visual_regression.rs` enforces
  the same invariant the eventual pixel-diff would catch: every
  paragraph the editor preview renders for a chapter (via
  `manuscript_to_html_chapters`) must appear, byte-for-byte and in
  source order, inside the EPUB's `OEBPS/text/chapter-NNN.xhtml`.
- Two tests:
  - `epub_chapter_paragraphs_match_editor_preview` — every preview
    `<p>...</p>` block exists in the unzipped EPUB chapter body.
  - `paragraph_order_is_preserved_across_paths` — relative ordering
    of any pair of preview paragraphs holds in the EPUB.
- The full pixel-diff harness (Playwright + golden PNGs + tolerance
  budget) lands after §I1 stabilises the styled rendering target.
- **Closed Phase 4 (2026-05-08) — Playwright pixel-diff harness:**
  - New pnpm package
    [`tests/visual-regression/`](../tests/visual-regression/) with
    `playwright.config.ts` (`maxDiffPixelRatio: 0.01`,
    `threshold: 0.2`, headless Chromium, fixed viewport / DPR /
    locale / timezone for CI determinism).
  - **Fixture generator** —
    `cargo run -p booksforge-export-epub --example visual_fixtures`
    walks 4 representative `FormatProfile`s
    (`fiction_trade_standard`, `fiction_literary`,
    `romance_historical`, `thriller_crime`) and writes:
      - `tests/visual-regression/fixtures/<profile>/preview.html`
        — editor preview rendering with inlined `<style>` mirroring
        `<ProsePreview>`.
      - `tests/visual-regression/fixtures/<profile>/epub-chapter.xhtml`
        — `chapter-001.xhtml` extracted from a freshly built EPUB,
        with `book.css` inlined for self-contained `file://` loading.
  - **Test spec** — `src/preview-vs-epub.spec.ts` runs three
    assertions per profile: preview matches its golden, EPUB
    chapter matches its golden, preview vs. EPUB pixel diff stays
    under tolerance (with `mask: [page.locator(".chapter-heading,
    h1, header")]` so the EPUB's wrapper heading doesn't trip the
    diff).
  - **CI job** — new `visual-regression` job in
    `.github/workflows/ci.yml` runs `pnpm gen:fixtures` then
    `pnpm --filter @booksforge/visual-regression test` on
    `ubuntu-22.04` (consistent fontconfig + Chromium build for
    stable goldens), uploading the diff artefact on failure.
  - **Workflow scripts** — root `pnpm visual:test` (CI parity) and
    `pnpm visual:update` (regenerate goldens after a deliberate
    rendering change).

### H7. Export history ✅ CLOSED 2026-05-07 (Phase 6)
- New `export_history()` Tauri command lists `exports` rows newest-first
  via the existing `ExportRecord` type.  `<ExportPanel>` renders the
  history table inline with profile, path, short hash, and localised
  timestamp.
- The unified `export_run` writes an `exports` row for every profile
  (Markdown was already wired; EPUB / DOCX / PDF now too).
- `ExportHistoryEntry` IPC type ships TS bindings.

### H8. Trade-paperback PDF profile ✅ CLOSED 2026-05-07
- First-pass single generic fiction trade-paperback profile shipped:
  6×9 default, xelatex backend, Garamond stack, drop caps, recto chapter
  starts, asymmetric gutter, microtype refinements.

### H8.1. Genre-aware FormatProfile (starter set) ✅ CLOSED 2026-05-07
- New `FormatProfile` enum in `booksforge-domain` decoupled from
  `ExportProfile` (which now means *what file format*; FormatProfile
  means *what genre typography*).  Seven starter variants:
  `fiction_trade_mass` (5×8), `fiction_trade_standard` (6×9, default),
  `fiction_literary` (6×9 with ornaments), `fiction_young_adult`
  (5.5×8.5, 12pt body, looser leading), `non_fiction_practical` (6×9
  sans heads, block paragraphs, callouts, TOC), `non_fiction_memoir`
  (6×9 trade-fiction feel + footnotes/photo-plate), `academic` (6×9
  numbered heads, narrow margins, bibliography styling, TOC).
- Per-profile knobs surfaced on `FormatProfile`: `trim_inches`,
  `chapter_starts_recto`, `drop_cap`, `body_font_family` /
  `heading_font_family` (CSS stacks), `body_size_pt`, `line_height`,
  `scene_break_glyph`, `paragraph_indent_em`, `pandoc_documentclass` /
  `pandoc_classoption`, `pdf_toc`, `front_matter_pages`.
- EPUB pipeline (`booksforge-export-epub`):
  - Per-profile CSS factory `render_book_css(profile)` interpolates
    body/heading fonts, body size, line height, scene-break glyph,
    paragraph indent, drop-cap rules, plus profile-specific extras
    (callout class for practical non-fiction, footnote/photo-plate for
    memoir, bibliography/table for academic, softer rhythm for YA).
  - Front-matter is now generated from
    `format_profile.front_matter_pages()` — title, copyright,
    optional dedication, optional epigraph (each only when content
    present); TOC is delegated to `nav.xhtml`.
  - `EpubMetadata` gained `dedication`, `epigraph`,
    `copyright_notice` (all `#[serde(default)]`).
  - `EpubPackageInput` gained `format_profile`.
- Pandoc pipeline (`booksforge-export-pandoc`):
  - `args_for_profile_with_format` reads trim, body size, font,
    line spread, documentclass, classoption, and TOC inclusion from
    `FormatProfile`.  Header-includes per profile (chapter style,
    secnumdepth for academic, line-spread tuning).
- IPC: `ExportRunInput` gained optional `format_profile: String`
  (string form so the UI can drive a `<select>`; unknown values fall
  back to `FictionTradeStandard`).  TS bindings regenerated.
- UI: `<ExportPanel>` now shows a "Genre / typography" selector when
  the chosen output profile is EPUB or PDF (markdown / DOCX ignore it).

### H8.2. Sub-genre depth + ornament library ✅ CLOSED 2026-05-08 (Phase 2)
- **Genre × Sub-genre taxonomy.**  New `Genre` enum
  (`Romance` / `Comedy` / `NonFiction` / `Thriller` / `Horror` plus
  a `Generic` umbrella for the H8.1 originals) + **20 new
  `FormatProfile` sub-genre variants** (4 per genre):
  - **Romance:** Contemporary, Historical / Regency, Paranormal,
    Suspense.
  - **Comedy:** RomCom, Satire, Literary Humor, Cozy.
  - **Non-fiction:** Narrative, Cookbook, Workbook, Self-help.
    (The H8.1 `NonFictionPractical` and `NonFictionMemoir` stay
    under `Generic`.)
  - **Thriller:** Psychological, Crime / Hard-boiled, Spy /
    Espionage, Action.
  - **Horror:** Gothic, Cosmic, Slasher, Supernatural.
  Each is a single `FormatProfile` variant with its own typography
  knobs.  The original 7 H8.1 profiles remain under `Genre::Generic`
  so existing fixtures and callers don't break.
- **Spec-table refactor.**  Per-profile knobs (trim, fonts, body
  size, line height, ornament, drop cap, paragraph indent, Pandoc
  document class, TOC policy, front-matter pages) are now defined
  in a single `ProfileSpec` const per variant — adding a new
  sub-genre is one new const plus one match arm in `spec()`.
- **Google Font bundle for book typography.**  Nine-family curated
  bundle exposed as `GOOGLE_FONT_BUNDLE`: EB Garamond, Crimson Pro,
  Lora, Source Serif 4, Vollkorn, Playfair Display, Cormorant
  Garamond, Inter, Source Sans 3.  Each `FormatProfile` declares
  its body / heading family from this bundle via
  `google_body_family()` / `google_heading_family()`.  EPUB CSS
  emits `@import` directives pointing at the Google Fonts CDN with
  ital/wght `0,400;0,700;1,400;1,700`; xelatex resolves the same
  family names from the writer's locally-installed font set.
- **Hand-curated SVG ornament library (option *a*).**  17 inline
  SVG ornaments (~150–300 bytes each, `currentColor` strokes so
  they pick up text colour for light/dark theming) — one per
  sub-genre family of profiles: asterism dots, romance flourish,
  regency cartouche, paranormal moon-and-stars, comedy wave,
  cookbook plates, workbook checkboxes, self-help arrows,
  thriller bar / slashes / espionage diamonds, gothic cross,
  cosmic descending triangles, slasher jagged stroke, supernatural
  crescent, literary fleuron, generic bullets.
- **EPUB CSS factory.**  `render_book_css(profile)` now emits a
  three-tier scene-break renderer: SVG ornament (data URI on
  `hr.background-image`) → Unicode glyph (`hr::before`) → suppress
  (Academic).  Google Font `@import` declarations precede the body
  rules.
- **Pandoc PDF args.**  `mainfont` / `sansfont` come straight from
  `google_body_family()` / `google_heading_family()`.  Sans-serif
  override only emitted when the heading font is `Inter` or
  `Source Sans 3`.
- **DOCX auto-styling routing.**  `resolve_docx_template(project,
  format_profile)` now looks up reference templates in this order:
  env override → `reference-<profile>.docx` → `reference-<genre>.docx`
  → generic `reference.docx` → none.  Writers can drop genre- or
  sub-genre-specific reference docs in
  `<bundle>/exports/templates/` and the right one is picked
  automatically.
- **Two-level UI selector.**  `<ExportPanel>` now shows a Genre
  `<select>` plus a Sub-genre `<select>` cascading off it.  Picking
  a new genre snaps the sub-genre to the first entry of the new
  list.  The format picker is now visible for DOCX exports too
  (was EPUB / PDF only) since DOCX is now genre-routed.
- **IPC.**  `ExportRunInput.format_profile` (string) carries any
  of the 27 sub-genre identifiers; the desktop command resolves
  via `FormatProfile::from_str` (unknown values fall back to
  `FictionTradeStandard`).  No IPC schema change beyond the
  expanded value set.
- **Tests.**  10 new `format_profile::tests` covering every variant
  (round-trip, genre membership, font bundle membership, ornament
  presence for non-Academic, front-matter inclusion).  Existing
  reproducibility test still passes — switching to the new CSS
  factory remains byte-deterministic across runs (verified by
  `examples/fixture_hash` returning the same hash on rerun).

**Follow-ups closed in Phase 3 (2026-05-08):**

- **Bundle Google Font binaries.**
  - New `scripts/fetch-fonts.sh` downloads variable-weight TTFs for
    the curated 9-family bundle from `github.com/google/fonts`
    (EB Garamond, Crimson Pro, Lora, Source Serif 4, Vollkorn,
    Playfair Display, Cormorant Garamond, Inter, Source Sans 3) plus
    each family's SIL OFL `LICENSE.txt`.  Output:
    `apps/desktop/resources/fonts/<Family_With_Underscores>/<File>.ttf`
    — ~11 MB total.  Re-runs are idempotent (cached files skipped).
  - `tauri.conf.json` ships the directory as a bundled resource
    via `"resources": ["resources/fonts/**/*"]` so the Tauri builder
    embeds the fonts in the platform-native app bundle.
  - **EPUB packager**: `EpubPackageInput.font_bundle_dir: Option<String>`
    is the gate.  When set, `collect_bundled_fonts(dir, profile)`
    walks the bundle for the profile's body + heading family, copies
    the `Roman` + `Italic` variable-weight files into
    `OEBPS/fonts/`, adds `<item>` entries to the OPF manifest with
    `media-type="application/font-sfnt"`, and emits per-file
    `@font-face` rules in `OEBPS/styles/book.css`.  When `None`,
    the CSS falls back to `@import url("fonts.googleapis.com/...")`
    (the existing online path).  EPUBs render their intended
    typography with no network dependency.
  - **Pandoc PDF**: `PandocInput.font_bundle_dir: Option<String>`
    drives a new `push_fontspec_options` helper that emits the
    fontspec `Path=` / `Extension=.ttf` / `UprightFont=*[wght]` /
    `ItalicFont=*-Italic[wght]` (or `[opsz,wght]` for Source Serif 4
    / Inter) options pandoc forwards to xelatex.  When unset, falls
    back to the system font lookup.
  - **Desktop**: new `resolve_font_bundle_dir(&AppHandle)` checks
    `BOOKSFORGE_FONT_BUNDLE_DIR` (dev override) →
    `<resource_dir>/resources/fonts` → `<resource_dir>/fonts` →
    `None`.  Threaded into both the EPUB and Pandoc paths via
    `export_run`.
  - **Tests**: new `bundled_fonts_get_embedded_under_oebps_fonts`
    integration test in `booksforge-export-epub` actually points at
    the on-disk bundle and asserts the right files land in the
    archive.  3 new pandoc tests cover the fontspec arg emission
    (path, opsz axis for Inter / Source Serif 4, omission when no
    bundle).

- **Programmatic DOCX `styles.xml` generation.**
  - New `crates/booksforge-export-pandoc/src/reference_docx.rs`
    builds a minimal but valid OOXML reference doc from a
    `FormatProfile`: `[Content_Types].xml`, `_rels/.rels`,
    `word/document.xml` (empty body), `word/_rels/document.xml.rels`,
    and the load-bearing `word/styles.xml`.  Body font + half-point
    size + auto-multiple line spacing + first-line indent in twips
    flow straight from the format profile; Heading 1–6 inherit the
    heading font with a `1.8 / 1.5 / 1.25 / 1.1 / 1.0 / 0.9` size
    progression that mirrors the EPUB CSS rendering rhythm.
  - **Determinism**: ZIP entries written in fixed order with the
    1980-01-01 epoch — same `FormatProfile` always produces
    byte-identical bytes (verified by
    `build_is_byte_deterministic_across_runs`).
  - **Wiring**: when `resolve_docx_template` finds no user-supplied
    or per-genre template, the desktop's `export_run` calls
    `write_generated_docx_template` which writes the generated
    bytes to `<bundle>/exports/templates/.generated-<profile>.docx`
    and passes that path to Pandoc as `--reference-doc`.  The
    file is rewritten only when the bytes differ (mtime stable,
    git-friendly).
  - **Tests**: 6 unit tests in `reference_docx::tests` cover ZIP
    validity, font name → styles.xml round-trip, body size →
    half-points, line-height → auto multiple, determinism, and
    distinct output across profiles.

**Closed Phase 4 (2026-05-08) — DOCX drop-cap + scene-break styles:**
- `reference_docx::render_styles_xml` now emits two additional
  paragraph styles when the `FormatProfile` enables them:
  - **`Drop`** — applied to the first paragraph of each chapter
    when source markup tags it `{.drop}`.  Uses Word's native
    `<w:framePr w:dropCap="drop" w:lines="3"/>` mechanism (the
    proper OOXML drop-cap path, not a hack).  Emitted only for
    profiles where `format_profile.drop_cap()` is true.
  - **`SceneBreak`** — centred paragraph with the profile's
    Unicode glyph as the visible content (`❦`, `* * *`,
    `·   ·   ·`, etc.).  Empty for `Academic` profile.
- 4 new unit tests in `reference_docx::tests` cover:
  - Drop style emitted when profile enables drop cap.
  - Drop style omitted when profile disables it.
  - SceneBreak style emitted with `<w:jc w:val="center"/>` for
    profiles with a glyph.
  - SceneBreak style omitted for Academic.
- The full SVG ornament drawing (`<w:drawing>` with embedded
  inline SVG part) at scene breaks remains a follow-up — it
  requires either a Pandoc Lua filter that recognizes a
  `:::scene-break` div and emits the OOXML drawing, or a
  post-processing step on the generated DOCX to inject `<w:drawing>`
  runs.  Trade paperback convention is to use a Unicode glyph
  anyway, so the current path covers the common case.
- Italic-only families that don't have a `-Italic[wght]` build —
  currently we skip italic for these and let Word synthesise.

---

## I. M6 — MVP polish

### I1. Accessibility audit ⏳ PARTIAL 2026-05-08 (Phase 4 — comprehensive sweep; AT testing on real hardware still pending)
- **Closed this turn — dialog a11y plumbing:**
  - New `apps/desktop/src-ui/src/lib/useDialogA11y.ts` hook returns
    `{ dialogProps, titleId }`.  `dialogProps` provides
    `role="dialog"`, `aria-modal="true"`, `aria-labelledby`,
    `tabIndex=-1`, ESC-to-close, focus-on-mount, and focus-return-on-
    unmount (so AT users land back on the toolbar button that
    opened the panel).
  - Applied to `<ExportPanel>`, `<CopyeditPanel>`, `<ContinuityPanel>`
    (the three panels touched in Phase 1).
- **Closed this turn — icon-button labelling sweep:**
  - All 16 `✕` close buttons across the dialog/overlay surface now
    carry `aria-label="Close <panel name>"` (specific labels on the
    three migrated panels, generic "Close panel" on the rest).
  - Binder's `+` add-scene button gained
    `aria-label="Add scene"`.
- **Closed this turn — semantic landmarks:**
  - `<Binder>` is now a `<nav aria-label="Manuscript binder">`.
  - `<EditorShell>` already used `<header>`, `<main>`, `<footer
    role="status">` for the status bar.
- **Closed this turn — live regions:**
  - Result / status / error containers in the three migrated panels
    now carry `role="status" aria-live="polite"` (status) or
    `role="alert"` (errors) so AT announces the result of long-
    running operations (export, copyedit run, continuity run).
- **Closed Phase 4 (2026-05-08) — comprehensive sweep:**
  - **All 21 dialog/overlay panels** now use `useDialogA11y`:
    `ExportPanel`, `CopyeditPanel`, `ContinuityPanel`, plus
    `ValidatorPanel`, `RecoveryDialog`, `OllamaWizard`,
    `SettingsPanel`, `SnapshotsPanel`, `KnowledgePanel`,
    `HumanizationPanel`, `VocabDictionaryPanel`,
    `EntityBiblePanel`, `DevelopmentalReviewPanel`,
    `IntakeAndOutlinePanel`, `AgentDebugForm`, `GenericAgentForm`,
    `QuickActionBar`, `HelpDrawer`, `OnboardingTour`,
    `NewProjectWizard`, `AgentsPanel`.  Each carries
    `role="dialog"` (or `alertdialog` for `RecoveryDialog`),
    `aria-modal="true"`, `aria-labelledby={titleId}`, ESC-to-close,
    and focus return on unmount.
  - **Form-control labels** audited workspace-wide.  Every
    `<input>` / `<select>` / `<textarea>` is either wrapped in a
    `<label>` element or carries `aria-label` (the audit script
    `awk '/<(input|select|textarea)\b/ {...}'` reports zero
    unlabelled controls).
  - **Tree role on `<Binder>`** — the binder is now a
    `<nav aria-label="Manuscript binder">` containing a
    `<div role="tree" aria-label="Manuscript tree">`; each row
    carries `role="treeitem" aria-level={depth+1}
    aria-selected={isSelected}` (scenes) / `aria-expanded={true}`
    (chapters), with roving-tabindex `tabIndex={isSelected ? 0 : -1}`
    and Enter/Space handlers.  Delete-scene button gets
    `aria-label={`Delete scene: ${title}`}`.
  - **Colour-contrast audit** against `packages/ui/src/tokens.css`:
    - Light mode `--color-text-tertiary` bumped from `neutral-400`
      (2.27:1 — fail) to `neutral-500` (3.92:1 — passes AA Large);
      doc-comment now reserves it for ≥18pt or ≥14pt-bold text.
    - Light mode `--color-border-focus` bumped from `amber-500`
      (1.7:1 — fail UI 3:1) to `amber-700` (4.08:1 — passes).
    - Dark mode `--color-text-tertiary` bumped from `neutral-500`
      (4.1:1 — borderline) to `neutral-400` (7.1:1 — comfortable).
    - All other token pairs verified ≥ 4.5:1 for body text or
      ≥ 3:1 for UI components, exempted only where SC 1.4.3 allows
      (disabled state).
- **Still open (real-hardware AT testing):**
  - End-to-end audit with VoiceOver on macOS and NVDA on Windows.
  - Skip-link for the editor (jump from binder to the active
    scene's text area).
  - Full keyboard navigation in the binder tree (arrow keys to
    move between siblings / up to parent).
  - Keyboard-shortcut cheatsheet in the help drawer.

### I2. Code signing + notarisation (macOS Developer ID, Windows EV cert)

### I3. Beta-channel updater (Tauri updater plugin, opt-out only)

### I4. In-app help drawer (offline content) ✅ CLOSED 2026-05-07 (Turn N)
- New `<HelpDrawer>` (toolbar button "Help") with three tabs:
  - **Quickstart** — what BooksForge is, how to start a project,
    drafting flow, agents intro, snapshot/export overview, privacy
    note pointing at Settings → Telemetry.
  - **Shortcuts** — keyboard map for navigation, editor (TipTap),
    quick actions.
  - **Agents** — table of all 11 agents with category and one-line
    blurb, plus the "Tier-1 cross-cutting validators run on every
    output" reminder.
- All copy bundled with the app — no remote fetches, no analytics,
  consistent with the local-first contract.

### I4.legacy. In-app help drawer (offline content)

### I5. Onboarding tour ✅ CLOSED 2026-05-07 (Turn N)
- New `<OnboardingTour>` overlay shown once per browser/local-storage
  session on first project open (storage key
  `booksforge.onboarding.v1.shown`).
- Three cards with progress dots: "Welcome" (binder + privacy),
  "Snapshots have your back" (auto + pre-edit), "Agents are optional"
  (Ollama + how to disable / explore alternatives).
- Skip / Got it buttons mark the flag so the tour doesn't reappear.
  `shouldShowOnboarding()` is exported so future entry points (a
  "Show welcome tour" link in Settings) can re-trigger it.

### I5.legacy. Onboarding tour

---

## J. Memory + Vocabulary subsystems

### J1. `booksforge-memory` ✅ CLOSED 2026-05-07 (Phase 3)
- Domain types live in `booksforge-domain::memory` (`MemoryScope`,
  `MemoryEntry`, `MemoryError`, `allowed_write_scopes`, `authorise_write`);
  `booksforge-memory` crate re-exports as a stable façade.
- Storage CRUD: `memory_upsert` (ON CONFLICT(scope, key)), `memory_get`,
  `memory_list_by_scope`, `memory_delete`. Round-trip + replace + delete
  tests in `crates/booksforge-storage/tests/memory_vocab.rs`.
- Per-agent write-scope authorisation pinned in
  `allowed_write_scopes(agent_id)` — Memory Curator owns book/chapter/entity,
  Copyeditor owns style, Outline Architect seeds book, etc.
- **Markdown mirror under `manuscript/.memory/` ✅ CLOSED 2026-05-07 (Turn C)** —
  new `crates/booksforge-fs/src/memory_mirror.rs` exposes
  `write_memory_mirror` / `delete_memory_mirror` / `memory_path`.
  Each entry serialises to `manuscript/.memory/<scope>/<key>.md` with a
  TOML-style frontmatter (scope, key, agent_id, updated_at) and a fenced
  ```json``` block of the value.  Keys are sanitised to `[A-Za-z0-9._-]`
  and truncated to 96 chars.  Best-effort: the writer is `Ok` on success
  and the caller logs+swallows on failure, so mirror I/O never blocks
  the canonical SQLite commit.  Wiring at agent-write time lands with
  Phase 5 alongside the chapter-finalise hook.

### J2. `booksforge-vocab` ✅ CLOSED 2026-05-07 (Phase 3)
- Domain types in `booksforge-domain::vocab` (`EntryKind`, `EntrySource`,
  `VocabEntry`, `layer_specificity`, `resolve`, `replacement_for`).
- Migration `0006_vocab_entries.sql` — `(layer, term, kind)` UNIQUE.
- Layered "most-specific wins" precedence:
  `project > genre > subgenre > domain > voice > chapter_type > audience > ai_tells`.
- Shipped starter dictionaries (compile-time embedded TOMLs):
  `ai_tells` (25 LLM-isms), `genre:fantasy`, `genre:romance`,
  `mode:non_fiction` — with `prefer / avoid / replace` rationales.
- Auto-seeded on `project_create` via `vocab_seed_starters` (idempotent —
  user / agent rows are preserved across re-seeds).
- **Still open:** Vocabulary Dictionary Agent wired to the accepted-edit
  hook (Phase 5).

### J3. Memory + Vocab IPC surface ✅ CLOSED 2026-05-07 (Turn B)
- New `commands::memory_vocab::{memory_list, vocab_list}` Tauri commands
  expose Phase 3's storage CRUD. ts-rs DTOs (`MemoryEntryDto`,
  `MemoryListInput`, `VocabEntryDto`, `VocabListInput`) live in
  `booksforge-ipc::memory_vocab`. New `KnowledgePanel.tsx` is a
  read-only inspector with Memory and Vocabulary tabs (memory grouped
  by scope; vocab listed across the active layer set with kind badges
  and rationale). Reachable from a "Knowledge" header button.

---

## K. Quick-action presets — full set

Per `MVP_SCOPE §2.5`: five presets — Sharpen, Continue, Rephrase, Shorten,
Expand. MZ-08 ships first three. Phase 1 added a fourth: **Final Polish**
(model-pinned to qwen3.6:latest) for world-class editorial passes.

### K1. `shorten/v1.toml` template + preset variant + apply op ✅ CLOSED 2026-05-07 (Turn A)
- Template at `crates/booksforge-prompt/templates/shorten/v1.toml`,
  `QuickActionPreset::Shorten` variant + UI button. Apply op = "replace"
  via the existing `handleAccept` branch. Migration `0007` widens the
  `ai_calls.preset` CHECK constraint.

### K2. `expand/v1.toml` template + preset variant + apply op ✅ CLOSED 2026-05-07 (Turn A)
- Template at `crates/booksforge-prompt/templates/expand/v1.toml`,
  `QuickActionPreset::Expand` variant + UI button. Apply op = "replace".
  Same migration as K1.

### K3. Per-preset model overrides ✅ CLOSED 2026-05-07
- Closed for `FinalPolish` (pinned to `qwen3.6:latest` via
  `QuickActionPreset::pinned_model()`); other presets still use the project
  default. Full per-call override remains available through
  `AiSuggestInput.model`.
### K4. Word-level visual diff in `QuickActionBar` ✅ CLOSED 2026-05-07 (Turn B)
- New `apps/desktop/src-ui/src/lib/wordDiff.ts` — pure-logic LCS-based
  word differ (no deps); preserves whitespace + punctuation token
  boundaries. QuickActionBar grew a Split/Diff toggle: Diff view renders
  inline with strike-through reds (removed) and underlined greens
  (added). 6 vitest unit tests in `wordDiff.test.ts`.

### K5. Vitest harness + UI tests ✅ CLOSED 2026-05-07 (Turn O)
- Vitest config + jsdom + React Testing Library already in place from
  Turn B; this turn extends the suite with two pure-logic test files:
  - `OnboardingTour.test.ts` — covers the localStorage helpers
    (`shouldShowOnboarding` / `markOnboardingShown`) including the
    quota-exhaustion exception path so the tour never blocks app
    startup.
  - `projectTemplates.test.ts` — covers the catalogue invariants
    (4 expected ids, no duplicates, every template has a non-empty
    label and description, ids match the `TemplateId` union).
- The existing `wordDiff.test.ts` continues to cover the tokeniser
  and diff path.  Per-component RTL tests (Copyedit / Humanization
  / Continuity panels) are intentionally deferred — they need IPC
  mocking infrastructure that's better landed alongside live-run UI
  rather than retrofitted now.
- CI command unchanged: `pnpm -C apps/desktop/src-ui test`.

### K5.legacy. Vitest harness + `QuickActionBar.test.tsx` ⏳ PARTIAL 2026-05-07 (Turn B)
- **Closed:** vitest infrastructure — `vitest`, `@testing-library/react`,
  `@testing-library/dom`, `jsdom` added to `apps/desktop/src-ui` dev
  deps; `vitest.config.ts` configured with jsdom + react plugin;
  `pnpm test` / `pnpm test:watch` scripts; first test file
  `wordDiff.test.ts` with 9 cases.
- **Still open:** `QuickActionBar.test.tsx` itself — needs IPC mocking
  via `vi.mock("@tauri-apps/api/core")` and an event-emitter mock.
  Lands cleanly now that the harness is in place.

---

## L. Test-fixtures crate hygiene

### L1. Fix bug-prone test in `mock_ollama.rs` ✅ CLOSED 2026-05-07 (Turn N)
- `MockConfig::default()` now sets `pull_ok = true` so the happy path
  needs no extra setup.  Tests that want the failure path call
  `set_pull_ok(false)` explicitly.
- Custom `Default` impl + doc-comment explain the contract so the
  next contributor doesn't second-guess it.

### L2. `booksforge-test-fixtures::projects::fiction_project()` schema drift watch ✅ CLOSED 2026-05-07 (Turn S)
- New `schema_drift` test module in `crates/booksforge-test-fixtures/src/projects.rs`
  with three assertions:
  1. `fiction_project()` round-trips through serde JSON without
     losing any field — catches a renamed/added field that the
     fixture forgot to populate.
  2. `fixture.schema_version == Project::CURRENT_SCHEMA_VERSION` —
     stops the fixture sliding behind a schema migration.
  3. Required-field sanity: title / authors / language / template_id
     non-empty, target_words populated, ai_enabled defaults to false
     (per the consent contract).
- Module-level doc-comment spells out what to do when these break —
  update the fixture to match the new shape.

### L1.legacy. Fix bug-prone test in `mock_ollama.rs`
- The fixture compiles and tests pass, but the `Default` `MockConfig`'s
  `pull_ok = false` quietly fails any pull that doesn't first call
  `set_pull_ok(true)`. Document or change the default.

### L2. `booksforge-test-fixtures::projects::fiction_project()` schema drift watch ✅ CLOSED 2026-05-07
- Originally implemented as serde-roundtrip + populated-fields tests
  in Turn S.  This turn adds the compile-time guard the BACKLOG
  asked for: a new `fixture_destructures_exhaustively` test that
  destructures both `Project` and `ProjectMeta` exhaustively.  Any
  future field addition to either struct will fail to compile in the
  fixture's destructure, forcing the single-update-site discipline.

---

## M. Tooling / dev-loop

### M1. Restore exact Tauri pin ✅ CLOSED 2026-05-07 (Turn N — pin moved forward)
- Spec target 2.2.3 has been yanked from crates.io — restoring is no
  longer possible.  `Cargo.toml` now pins to `tauri = "2.11"` (current
  patch line that matches our `Cargo.lock`) with an explanatory comment
  documenting the deviation.
- Update procedure for future bumps documented in the same comment.

### M1.legacy. Restore exact Tauri pin (=2.2.3 was the spec target)
- We relaxed to `tauri = "2"` because crates.io no longer hosts 2.2.3.
  Either: (a) update `outputs/TOOLCHAIN.md` to reflect the new floor, or
  (b) switch to `tauri = "=2.6.x"` and pin a known-good version.

### M2. Restore exact Rust toolchain pin ✅ CLOSED 2026-05-07 (Turn N — pin moved forward)
- Spec target 1.82.0 is below the MSRV of current Tauri 2.x transitive
  deps (e.g. `time` 0.3.47 needs 1.88).  `rust-toolchain.toml` now pins
  to `1.88.0` (verified to build the workspace clean), and
  `Cargo.toml [workspace.package].rust-version` matches.
- Update procedure documented in `rust-toolchain.toml`.

### M2.legacy. Restore exact Rust toolchain pin (1.82.0 was the spec target)
- Bumped to `stable` because Tauri 2.x transitive deps need edition 2024
  (Rust ≥ 1.85). Update `TOOLCHAIN.md` MSRV and regenerate the
  `rust-toolchain.toml` accordingly.

### M3. Real icons for the desktop app
- Currently using placeholder PNGs generated at build verification time.
  Replace with the real BooksForge brand assets before code-signing (M6).

### M4. Pandoc + EPUBCheck binaries in `binaries/`
- We removed `externalBin` from `tauri.conf.json` because the bundled
  binaries weren't present. Re-add when M5 ships.
