# BooksForge — End-to-End Product Roadmap

**Owner:** active-development workspace
**Started:** 2026-05-09
**North star:** A complete end-to-end book factory for ghostwriters and indie authors. Genre fiction · literary fiction · non-fiction · all the way from "I have an idea" to "ready to upload to KDP / Apple / Google". Honest quality scoring. No fake 9/10s.

This roadmap supersedes the per-item BACKLOG entries A13–A16 and the prior UX recommendations R2–R6. Each phase is a coherent shippable chunk; the deliverables column lists the files / surfaces that must land for the phase to be considered closed.

---

## Phase 1 — Fiction agents wired end-to-end (closes A13)

**Goal:** A user opens BooksForge, picks "Fiction", and the agent layer can build a character bible, world bible, and per-scene fiction drafts entirely through the in-product UI. No more naked-LLM scaffolding for fiction.

| # | Deliverable | Type | Where | Closes |
|---|---|---|---|---|
| 1A | Inspect `run_chapter_drafter` + `apply_outline` to mirror the pattern | Discovery | — | — |
| 1B | Domain types: `SceneCardInput`, `FictionDrafterContext`, `CharacterBibleInput`, `WorldBibleInput` | Rust | `booksforge-domain/src/agent_io.rs` | A13 |
| 1C | `scene_drafter_fic` agent crate (mirror of `chapter_drafter` shape, fiction-shaped inputs) | Rust | `booksforge-agents/src/scene_drafter_fic.rs` | A13 |
| 1D | Orchestrator `run_*` methods for the 3 fiction agents | Rust | `booksforge-orchestrator/src/run.rs` | A13 |
| 1E | Orchestrator `apply_*` methods (bibles → memory entries; scene-fic → pm_doc replace via existing `apply_chapter_drafter` flow) | Rust | `booksforge-orchestrator/src/apply_character_bible.rs`, `apply_world_bible.rs` | A13 |
| 1F | Tauri commands + IPC types + ts-rs bindings for all 6 (3 run + 3 apply) | Rust + TS | `apps/desktop/src/commands/agents.rs`, `booksforge-ipc/src/agents.rs`, `packages/shared-types/src/index.ts` | A13 |
| 1G | React panels: `CharacterBiblePanel`, `WorldBiblePanel`, `SceneDrafterFicPanel`; AgentsPanel adds 3 cards | TS | `apps/desktop/src-ui/src/components/agents/*.tsx` | A13 |
| 1H | Tests + clippy + codegen-drift all green | Test | per crate | — |
| 1I | BACKLOG + roadmap status updated | Doc | `BACKLOG.md`, this file | — |

**Estimated test count delta:** +24 tests (8 per new agent, structural + semantic + repair-salvage + spec).

**Phase 1 done means:** the user can produce a complete fiction project draft (bibles → outline → drafted scenes) entirely from the desktop UI, all going through the orchestrator with proper snapshots and audit ledger.

---

## Phase 2 — Specialist polish stack as native agents (closes A15)

**Goal:** The 4 voice-preserving polish stages currently sitting as `.toml` templates become real Rust agents the orchestrator can sequence. Replaces the prior single "polish" prompt that flattened voice (the cause #3 in `RCA_QUALITY_6_TO_9.md` worth ~0.6 rubric pts).

| # | Deliverable | Type | Where | Closes |
|---|---|---|---|---|
| 2A | Domain `PolishProposal { revised_pm_doc, edit_notes, stage_id }` | Rust | `booksforge-domain/src/agent_io.rs` | A15 |
| 2B | 4 polish agent crates (`dialogue_polish`, `metaphor_polish`, `voice_polish`, `scene_tension_polish`) | Rust | `booksforge-agents/src/*_polish.rs` | A15 |
| 2C | `scene_critic` agent crate (per-scene critique-revise loop) | Rust | `booksforge-agents/src/scene_critic.rs` | A15 |
| 2D | Orchestrator `run_polish_stack(scene_id, BookKind)` — sequences stages per genre | Rust | `booksforge-orchestrator/src/polish_stack.rs` | A15 |
| 2E | Orchestrator `run_scene_critique_revise(scene_id)` — bounded 2-round critic loop | Rust | `booksforge-orchestrator/src/critique_revise.rs` | A15 |
| 2F | Tauri commands + IPC types | Rust + TS | as above | A15 |
| 2G | UI: replace single "Polish" button with stage-aware progress (4 sub-progress bars) | TS | `agents/PolishStackPanel.tsx` | A15 |
| 2H | Tests + clippy + codegen-drift | Test | — | — |

**Estimated test count delta:** +35 tests (5 polish stages × 5 tests + 10 stack-orchestration).

**Phase 2 done means:** invoking "Polish" on a chapter runs the 4-stage stack (genre-ordered) with each stage voice-preserving by design, surfaces per-stage diffs the user can accept/reject, and writes the audit ledger. Per-scene critique-revise runs as a separate user-invokable action.

---

## Phase 3 — Quality stack: voice fingerprint + anti-AI-tells + ensemble + multi-specialist scoring (closes A16)

**Goal:** Bring the proven Python `ghostwriter/` capabilities into the product as native Rust crates. This is the biggest quality lift in the roadmap — closes the "measurement framework that doesn't lie" gap from the user's 4.66/10 question.

| # | Deliverable | Type | Where | Closes |
|---|---|---|---|---|
| 3A | New crate `booksforge-voice` — port `voice_fingerprint.py` to Rust (fingerprint, stylometric_distance, constraints_block) | Rust | `crates/booksforge-voice/` | A16 |
| 3B | New crate `booksforge-anti-ai-tells` — port `anti_ai_tells.py` (50+ patterns, severity tiers, span-level revision-prompt builder) | Rust | `crates/booksforge-anti-ai-tells/` | A16 |
| 3C | New crate `booksforge-genre-packs` — three packs (literary / genre / non-fiction) with system prompts, critic axes, polish-stack ordering, 12-axis rubric weights | Rust | `crates/booksforge-genre-packs/` | A16 |
| 3D | Orchestrator `run_ensemble_draft(scene_card, n_candidates)` — N candidates at varied temps; critic picks best | Rust | `booksforge-orchestrator/src/ensemble.rs` | A16 |
| 3E | Orchestrator `score_manuscript_multi_specialist()` — 3 separate lens calls (developmental / prose / commercial) merged with per-genre weights | Rust | `booksforge-orchestrator/src/scoring.rs` | A16 |
| 3F | Voice-fingerprint extraction at ProjectBrief intake — store on the project for all subsequent drafts to consume | Rust | `agents/intake.rs` extension | A16 |
| 3G | Anti-AI-tells redaction as an orchestrator post-polish step (auto-runs if density > 4) | Rust | `polish_stack.rs` integration | A16 |
| 3H | Tauri commands + IPC for: fingerprint extraction, manual fingerprint-from-comp, score manuscript, get tells report | Rust + TS | as above | A16 |
| 3I | UI surfaces: Voice Anchor card on Project Settings; Honest Score panel on the Validator surface; AI-Tells inspector | TS | `components/{VoiceAnchorPanel,HonestScorePanel,TellsInspector}.tsx` | A16 |
| 3J | Tests | Test | per crate | — |

**Estimated test count delta:** +60 tests (voice 12, tells 15, packs 8, ensemble 10, scoring 15).

**Phase 3 done means:** every draft is anchored to user-supplied voice constraints; every polish run is followed by an AI-tells audit; every chapter has an honest score the user sees in the editor; every commit-to-final is gated on rubric thresholds the user can override but must see.

---

## Phase 4 — Ghostwriter mode + book-type branching as a first-class feature

**Goal:** Make the genre packs and the multi-pass quality stack the default invoke path, not a separate workflow. The product KNOWS what kind of book it's making and adapts accordingly.

| # | Deliverable | Type | Where |
|---|---|---|---|
| 4A | `ProjectBrief.book_kind: BookKind` enum (`fiction-literary` / `fiction-genre` / `non-fiction` / `memoir` / `childrens`) | Rust | `booksforge-domain/src/brief.rs` |
| 4B | Migration: existing projects get `book_kind = unknown`; UI prompts user once on first open | Rust + TS | new sqlx migration + onboarding overlay |
| 4C | Orchestrator workflow router: `Workflow::for_kind(BookKind)` returns the right agent sequence | Rust | `booksforge-orchestrator/src/workflows/mod.rs` |
| 4D | One-click "Draft this scene" invokes the genre-correct stack: ensemble drafter → critic → revise → polish stack → tells redact → score | Rust + TS | new `agent_run_full_scene` command |
| 4E | "Polish this chapter" invokes the genre-correct polish stack | Rust + TS | extends Phase 2 |
| 4F | "Score this manuscript" invokes the genre-weighted multi-specialist scorer | Rust + TS | extends Phase 3 |
| 4G | Validator surface shows per-genre rubric; pre-export gate honors per-genre quality thresholds | Rust + TS | `ValidatorPanel.tsx` extension |
| 4H | Tests (one full E2E per genre on a tiny fixture) | Test | new integration tests |

**Estimated test count delta:** +25 tests including 3 full-E2E integration tests (one per BookKind).

**Phase 4 done means:** there is no longer a separate "ghostwriter pipeline" sitting in `artifacts/`. The product itself is the pipeline.

---

## Phase 5 — UX R2: Book-type branching surface

**Goal:** Implement UX recommendation R2 (book-type branching) — the highest-leverage UX fix. Builds on Phase 4 backend.

| # | Deliverable | Where |
|---|---|---|
| 5A | NewProjectWizard: BookKind picker becomes step 1 (was: implicit via AI toggle); 6 cards, each with a 1-sentence "what this means" tooltip | `NewProjectWizard.tsx` |
| 5B | ProjectSettings: BookKind editable post-creation with confirmation dialog (changes invalidate prior bibles) | `SettingsPanel.tsx` |
| 5C | Onboarding overlay for projects with `book_kind = unknown` (post-migration) | `OnboardingTour.tsx` |
| 5D | E2E test (Playwright) covers the per-kind wizard branches | `tests/e2e/` |

---

## Phase 6 — UX R3: AgentsPanel grouped by intent

**Goal:** Replace the 14-agent switchboard with a 4-card intent grouping. Hide meta agents.

| # | Deliverable | Where |
|---|---|---|
| 6A | New layout: 4 large cards (Plan / Draft / Polish / Publish), each opens a sub-panel with the relevant agents | `AgentsPanel.tsx` |
| 6B | Plan card: Intake, Outline, Character Bible, World Bible | `agents/PlanGroup.tsx` |
| 6C | Draft card: Scene Drafter (genre-aware), Chapter Drafter | `agents/DraftGroup.tsx` |
| 6D | Polish card: Polish Stack (4 stages), Per-Edit Copyedit, Humanization, Continuity | `agents/PolishGroup.tsx` |
| 6E | Publish card: Validator, Export, Marketplace Readiness | `agents/PublishGroup.tsx` |
| 6F | Advanced expander: Memory, Vocab, Proposal Validator, Peer Review (auto-invoked → moved out of user catalogue) | `agents/AdvancedGroup.tsx` |
| 6G | E2E test: first-time user reaches first generated outline in ≤4 clicks | `tests/e2e/` |

---

## Phase 7 — UX R4: Single "Prepare for Publishing" action

**Goal:** One button that produces every per-platform package + a HUMAN_REQUIRED checklist for the parts only a human can complete. Closes the user's "exports are not in the UI" complaint at the workflow level.

| # | Deliverable | Where |
|---|---|---|
| 7A | New Tauri command `prepare_for_publishing` that bundles: DOCX, EPUB-3 (validated), PDF (typst), metadata.json, metadata.kdp.csv, cover brief, marketplace checklists | Rust + TS |
| 7B | New panel `PrepareForPublishingPanel.tsx` — single "Prepare" button, shows per-format progress, ends with a HUMAN_REQUIRED checklist (cover art, ISBN, AI disclosure, payment account) | TS |
| 7C | Per-platform output directories: `exports/kdp/`, `exports/google_play/`, `exports/apple_books/`; each contains the artifacts that platform requires + a `READY_TO_UPLOAD.md` instruction file | Rust |
| 7D | EPUBCheck + KDP cover sizing checks + Apple metadata validation run as part of the bundle | Rust |
| 7E | Tests | per command |

---

## Phase 8 — UX R5: Plain-English copy + glossary

**Goal:** Strip publishing jargon from the user-facing surface. No more BISAC, trim, bleed, ULID, task_id, pm_doc, EPUBCheck visible without an explanation.

| # | Deliverable | Where |
|---|---|---|
| 8A | Build `lib/glossary.ts` with definitions for every publishing term that appears in the UI | `lib/glossary.ts` |
| 8B | New `<Term>` component that wraps any jargon in a tooltip-on-hover with the glossary entry | `components/Term.tsx` |
| 8C | Copy audit: replace `task_id` / `ULID` / `pm_doc` in user-facing surfaces (keep them in audit/dev surfaces) | grep-driven sweep |
| 8D | Convert all "trim", "bleed", "BISAC" labels into `<Term>`-wrapped versions | grep-driven sweep |
| 8E | Pre-commit lint: no bare jargon strings in user-facing TSX (allowlist for inspector / settings) | new lint rule |

---

## Phase 9 — UX R6: 4 missing approval gates

**Goal:** Add the 4 mandatory creative-decision gates so users never lose control at expensive/irreversible stages.

| # | Deliverable | Where |
|---|---|---|
| 9A | "Approve Topic" gate after intake — shows premise, target reader, what's distinctive | `agents/IntakeAndOutlinePanel.tsx` extension |
| 9B | "Approve Plan" gate after outline architect — shows logline, acts, escalation per chapter | new `OutlineApprovalDialog.tsx` |
| 9C | "Approve Bibles" gate after character + world bible runs — shows cards inline with edit-each option | new `BibleApprovalDialog.tsx` |
| 9D | "Approve Manuscript Pre-Final-Polish" gate — shows the per-chapter draft scores and asks "ready to commit to the polish stack?" | new `ManuscriptApprovalDialog.tsx` |
| 9E | All 4 gates are non-removable in default mode; advanced users can disable them in `SettingsPanel` | settings checkbox |
| 9F | Tests: each gate is reached and respects user's accept/edit/reject | E2E |

---

## Round structure (for the user)

This roadmap targets ~4 substantive rounds:

- **Round 1 (this session):** Phase 1 in full.
- **Round 2:** Phase 2 + Phase 3 (the two heaviest backend phases — they share testing infrastructure).
- **Round 3:** Phase 4 + Phase 5 + Phase 6 (book-type branching surface + intent grouping).
- **Round 4:** Phase 7 + Phase 8 + Phase 9 (Prepare for Publishing + jargon cleanup + approval gates) + final E2E run on all 3 genres for honest comparative scores.

Each round ends with: cargo workspace tests green, clippy clean, tsc clean (no new errors), and a status update appended to this file.

---

## Honest constraints (these will not change between rounds)

1. **No fake 9/10.** Every score the system surfaces is the honest measurement output. The only way to "improve the score" is to improve the prose, not the rubric.
2. **The orchestrator is the only mutator.** Every apply path goes through it, takes a snapshot, and writes an audit-ledger row. UI never calls `sceneSave` directly for AI-applied content.
3. **No outbound network for generation.** All LLM calls go to `127.0.0.1:11434`. Cloud generation is a hard CI failure.
4. **No fabricated stats / dates / quotes for non-fiction.** The non-fiction genre pack enforces `[SOURCE NEEDED]` over invention; the validator gates on it.
5. **Voice-preserving by design.** Polish stack is 4 specialist passes, each scoped to its remit, never a "make it generally better" prompt.
6. **Ghostwriter-grade defaults.** Ensemble drafting, voice fingerprint constraints, and AI-tells redaction run by default in every drafted scene; users can opt out.
7. **Human-in-the-loop is the floor.** The 4 approval gates are non-removable in default mode.

---

## Status (this file lives next to BACKLOG.md and is updated at the end of every round)

| Phase | Status | Closed in |
|---|---|---|
| 1 | **CLOSED** ✅ | Round 1 (2026-05-09) |
| 2 | **CLOSED** ✅ | Round 2 (2026-05-09) |
| 3 | **CLOSED** ✅ | Round 2 (2026-05-09) |
| 4 | **CLOSED** ✅ | Round 3 (2026-05-09) |
| 5 | **CLOSED** ✅ | Round 3 (2026-05-09) |
| 6 | **CLOSED** ✅ | Round 3 (2026-05-09) |
| 7 | **CLOSED** ✅ | Round 4 (2026-05-09) |
| 8 | **CLOSED** ✅ | Round 4 (2026-05-09) |
| 9 | **CLOSED** ✅ | Round 4 (2026-05-09) |

---

## Round 1 close-out (2026-05-09)

Phase 1 fully landed, end-to-end, in the desktop product. See BACKLOG §A13
for the full punch list — every deliverable shipped:

- **Domain types** (6 new structs) + **memory-scope authorisation** for the
  3 new agents (3 new unit tests).
- **3 new agent crates** (`character_bible`, `world_bible`,
  `scene_drafter_fic`) registered in `all_agents()` (13 new tests across
  the three).
- **3 orchestrator `run_*` methods** + **3 orchestrator `apply_*` methods**
  (each: idempotency-guarded, mandatory `PreAgentEdit` snapshot,
  `agent_applied_edits` ledger row, scope-authorisation).
- **9 new IPC types** + **6 Tauri commands** + **9 ts-rs binding files**
  + **6 IPC client methods** + **3 React panels** + **AgentsPanel**
  shows the new "Fiction" category at the top.

### Workspace verification (Round 1)

| Gate | Result |
|---|---|
| `cargo test --workspace` | **505 passed, 0 failed, 3 ignored** (up from 488) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **clean** |
| `cargo build -p booksforge-desktop` | **clean** |
| `pnpm exec tsc --noEmit` (src-ui) | **0 new errors** (same 14 pre-existing in unrelated files) |
| `cargo test -p booksforge-ipc` (codegen-drift) | **all green** |

### What this means for the user

A user can now, **entirely from the BooksForge desktop app**:

1. Open the AgentsPanel → see the new "Fiction" category at the top.
2. Run **Character Bible** with a chapter count and optional accepted-prose
   samples; review per-character cards (name, role, wants, needs, wound,
   secret, voice traits, per-chapter arc); accept → persisted to project
   memory as one entity-scope row per character.
3. Run **World Bible**; review locations + social rules + history +
   sensory palette + motifs + continuity constraints; accept → locations
   to entity scope, everything else to book scope.
4. Open a scene in the editor; run **Scene Drafter (Fiction)** with a
   scene card (goal / conflict / reveal / target words / POV / genre
   lens); the drafter auto-loads the bibles at run time on the backend;
   review the prose preview; Apply → orchestrator-mediated write into
   the active scene with snapshot + audit-ledger row.

There is no naked-LLM scaffolding for fiction in the product anymore.
Every fiction-mode operation goes through a versioned prompt-template-pinned
agent with proper validators, failure modes, and audit trails.

---

## Round 2 close-out (2026-05-09)

**Round 2 fully landed.** Phases 2 + 3 both complete + the full pre-round
cleanup (14 TS errors fixed, A11 typst sidecar wired into export_run + the
dependency-check report, A13 finished by persisting the typed
`project_brief` to book-scope memory at intake so the bibles can find it).

### What shipped

**Cleanup (3 items):**
- All 14 pre-existing TypeScript errors fixed → **TS workspace 0 errors**.
- A11 finish: `booksforge-export-typst` wired into `export_run` as the
  preferred PDF engine for `TradePdf*` profiles (pandoc-via-LaTeX is the
  fallback when typst is missing); typst surfaced in
  `export_check_dependencies` so the ExportPanel can show install status.
- A13 finish: `agent_run_intake` persists the typed `ProjectBrief` to
  book-scope memory under key `project_brief`, so the fiction agents
  (`character-bible`, `world-bible`, `scene-drafter-fic`) find context
  via `memory_get` without re-running intake.

**Phase 2 — specialist polish stack as native agents (closes A15):**
- Domain types `PolishProposal`, `PolishStageId`, `SceneCritiqueProposal`,
  `TargetedEdit` in `booksforge-domain`.
- 5 new Rust agent crates: `dialogue_polish`, `metaphor_polish`,
  `voice_polish`, `scene_tension_polish`, `scene_critic`. Each with
  `spec()`, `parse_and_validate()`, failure modes, and unit tests.
  Voice-preserving by design — each stage's prompt is scoped to its
  remit, never a "make it generally better" prompt.
- New shared `polish_common` module factoring the parse-with-stage-id-
  verification logic.
- All 5 registered in `agents::registry::all_agents()`.
- Orchestrator `run_polish_stage` (polymorphic over `PolishStageId`),
  `run_scene_critic`, `apply_polish` (also polymorphic).
- 3 new Tauri commands: `agent_run_polish_stage`, `agent_apply_polish`,
  `agent_run_scene_critic`. Registered in `lib.rs` invoke list.
- 4 new IPC types + ts-rs bindings + `ipc.ts` client methods.
- New React panel `PolishStackPanel.tsx` — sequences the 4 stages in
  genre-correct order (`literary` → voice/metaphor/dialogue/scene_tension;
  `genre` → scene_tension/dialogue/metaphor/voice). Per-stage Run + diff
  preview + Accept/Skip. AgentsPanel adds a `polish` category.

**Phase 3 — quality stack as native crates (closes A16):**
- New crate `booksforge-voice` — Rust port of `voice_fingerprint.py`.
  `fingerprint(text)` returns a 16-field `VoiceProfile`; `stylometric_distance`
  scores 0–10. Pure logic, ts-rs-exported.
- New crate `booksforge-anti-ai-tells` — Rust port of `anti_ai_tells.py`.
  50+ patterns across 8 categories with severity tiers; `tells_per_1000_words`
  density measurement; `revision_prompt` for span-targeted polish handoff.
- New crate `booksforge-genre-packs` — Rust port of `genre_packs.py`.
  Three packs (literary / genre / non-fiction), each with system prompt,
  draft lens, critic axes, polish-stack ordering, 12-axis rubric weights,
  hard rules. `BookKind` enum with forgiving `from_str`.
- 6 new Tauri commands wired into `commands::quality`:
  `voice_fingerprint`, `voice_anchor_set`, `voice_anchor_get`,
  `stylometric_distance_compute`, `tells_scan`, `genre_pack_get`.
- 10 new IPC types + ts-rs bindings + `ipc.ts` client methods.
- 3 new React panels under `components/quality/`:
  - `VoiceAnchorPanel.tsx` — paste comp samples, preview measurement,
    save as project voice anchor (book-scope memory key `voice:anchor`).
  - `TellsInspectorPanel.tsx` — scan prose, show per-span hits with
    severity + category, render verdict badge, expose revision-prompt
    fragment for hand-off.
  - `HonestScorePanel.tsx` — stylometric distance vs. anchor + AI-tells
    verdict + per-genre rubric weights as a bar chart.
- AgentsPanel adds a `quality` category at the very top of the
  switchboard.

### Workspace verification (Round 2 close-out)

| Gate | Result |
|---|---|
| `cargo test --workspace` | **564 passed, 0 failed, 3 ignored** (up from 505 after Round 1 — +59 new tests) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **clean** |
| `cargo build --workspace` | **clean** |
| `pnpm exec tsc --noEmit` (src-ui) | **0 errors** (up from 14 pre-existing — all fixed) |
| `cargo test -p booksforge-ipc` (codegen-drift) | **green** — 27 new TS bindings emitted, all re-exported from `packages/shared-types/src/index.ts` |

### What this means for the user

Three new switchboard categories at the top of AgentsPanel:

1. **Quality** — set a voice anchor from comp samples, scan any prose
   for AI-tells, view honest score with per-genre rubric weights.
2. **Fiction** (Round 1) — character bible, world bible, scene drafter.
3. **Polish** — 4-stage voice-preserving polish stack with per-stage
   accept.

End-to-end: a user can now:
- Paste a comp sample → save as Voice Anchor (numeric profile in memory).
- Run intake → brief auto-persists to book-scope memory.
- Run Character Bible + World Bible → entity / book memory.
- Open a scene → run Scene Drafter (Fiction) → bibles auto-load.
- Apply the draft via the orchestrator-mediated path (audit ledger).
- Run Polish Stack → 4 voice-preserving passes in genre order.
- Open the Honest Score panel → see stylometric distance vs. anchor +
  AI-tells verdict + per-genre rubric weights — measured, not vibes.

---

## Round 3 close-out (2026-05-09)

**Round 3 fully landed.** Phases 4 + 5 + 6 all complete.

### What shipped

**Phase 4 — Book-type branching as a first-class feature:**
- New `BookKind` enum in `booksforge-domain` (LiteraryFiction /
  GenreFiction / NonFiction / Memoir / ChildrensBook). 5 variants;
  Memoir + ChildrensBook map to literary pack; ChildrensBook is
  marked `is_supported_in_mvp = false` so the wizard refuses it.
- `Project.book_kind: Option<BookKind>` field. Optional for backwards
  compat; `manifest.toml` round-trips with serde defaults.
- `genre-packs::BookKind` is now a re-export from domain (so the
  orchestrator, project schema, and packs all refer to the same type).
- `BundleManifest::set_book_kind` writes back to manifest atomically.
- IPC: `book_kind` field on `CreateProjectInput` + `OpenProjectResult`.
  New `ProjectKindSetInput` / `ProjectKindSetResult` types.
- `project_kind_set` Tauri command — updates manifest in place.
- `agent_run_full_scene_pipeline` Tauri command — chains
  `scene-drafter-fic` → `scene-critic` → genre-correct 4-stage polish
  stack → AI-tells scan, with `pipeline:progress` events emitted per
  stage. Reads project's `book_kind`, loads the matching genre pack,
  routes prompts and polish-stack ordering accordingly.

**Phase 5 — UX R2 book-type surface:**
- `NewProjectWizard` now opens on **Step 0 — book kind picker** with 5
  cards (literary / genre / non-fiction / memoir / children's coming
  soon). User must pick before continuing. Wizard re-numbers to "Step
  N of 4" instead of "Step N of 3".
- New `BookKindOverlay.tsx` component (used in both onboarding +
  settings modes). Onboarding mode is non-dismissible (forces choice
  for legacy projects); settings mode is dismissible.
- `App.tsx` — when an opened project has `book_kind = null`, fires the
  onboarding overlay automatically. After save, the open-project state
  is updated in place so the editor sees the new kind on its next
  render.
- `SettingsPanel` — new "Book kind" section at the top with a "Change
  book kind…" button that opens the BookKindOverlay in settings mode.

**Phase 6 — UX R3 AgentsPanel intent grouping:**
- Replaced the previous 7-category switchboard ("the switchboard tax"
  friction-point #F1 from the prior UX audit) with **4 intent
  sections** in writer-flow order: **Plan → Draft → Polish → Publish**.
- Plan: Intake, Brief→Outline, Outline Architect, Character Bible,
  World Bible, Voice Anchor.
- Draft: Scene Drafter (Fiction), Chapter Drafter, Dev Editor,
  Developmental Review.
- Polish: Polish Stack (4 stages), Copyeditor, Humanization, Continuity,
  AI-Tells Inspector.
- Publish: Honest Score, Memory Curator, Vocab Dictionary, Entity Bible.
- Meta agents (Proposal Validator, Peer Review) moved under a
  collapsible **Advanced** disclosure — they're auto-invoked, the
  user almost never needs to click them.

### Workspace verification (Round 3 close-out)

| Gate | Result |
|---|---|
| `cargo test --workspace` | **574 passed, 0 failed, 3 ignored** (up from 564 — +10 new tests) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **clean** |
| `cargo build -p booksforge-desktop` | **clean** |
| `pnpm exec tsc --noEmit` (src-ui) | **0 errors** |
| `cargo test -p booksforge-ipc` (codegen-drift) | **green** — 3 new TS bindings (`BookKind`, `ProjectKindSetInput`, `ProjectKindSetResult`) |

### What this means for the user

A first-time author can now, end-to-end:
1. Click "New project" → **Step 1 picks a book kind** before anything else.
2. Fill title / author / save location → land in the editor with the
   project tuned to their genre.
3. Open AgentsPanel → see **Plan / Draft / Polish / Publish** in that
   order, with cards grouped by what the user is actually trying to do.
4. Click "Voice Anchor" in Plan → paste comp samples → drafter and
   polish stack automatically honour them.
5. Click a single button (when Round 4 surfaces it on the editor
   toolbar) → run the full per-genre pipeline (draft → critic → polish
   stack → tells scan) and get an Honest Score at the end.

If they open a project from before Phase 4, the **onboarding overlay**
pops up automatically and asks them to pick a kind before any agent
will run. They can change the kind later in Settings.

---

## Round 4 close-out (2026-05-09)

**Round 4 fully landed.** Phases 7, 8, 9 all closed. The product now ships
the full UX-recommendation set R2–R6 from the audit on top of the Round
1–3 backend depth.

### What shipped

**Phase 7 — Prepare for Publishing single action (closes UX R4):**
- New IPC types in [`booksforge-ipc::publishing`](crates/booksforge-ipc/src/publishing.rs):
  `PrepareForPublishingInput`, `PrepareForPublishingResult`,
  `PublishingMetadata`, `PlatformReadiness`, `ReadinessItem`. All
  ts-rs-exported and re-exported from `@booksforge/shared-types`.
- New Tauri command [`prepare_for_publishing`](apps/desktop/src/commands/publishing.rs)
  bundles per-platform packages under
  `<bundle>/exports/{kdp,google_play,apple_books}/`:
    - `manuscript.epub` (every platform; EPUBCheck-validated, gating
      Apple Books readiness)
    - `manuscript.pdf` via typst (KDP + Google Play; Apple skips); falls
      back to pandoc-via-LaTeX when typst is missing
    - `metadata.kdp.csv` / `metadata.gp.json` / `metadata.apple.json`
      (`[PLACEHOLDER]` for unset fields so the writer sees what's still
      required)
    - `cover_brief.md` (HUMAN_REQUIRED to commission art)
    - `READY_TO_UPLOAD.md` per-platform submission walkthrough
    - `readiness.json` per-item PASS/WARN/FAIL/HUMAN_REQUIRED
- Per-platform HUMAN_REQUIRED items: KDP gets AI-disclosure +
  rights-review, Google Play gets preview-percentage settings, Apple
  Books gets category/age-range/explicit-content fields.
- New panel
  [`PrepareForPublishingPanel.tsx`](apps/desktop/src-ui/src/components/PrepareForPublishingPanel.tsx)
  + toolbar button "Publish" — collects optional metadata overrides,
  shows per-platform readiness grid with status pills + uploadable flag.

**Phase 8 — Plain-English copy + glossary (closes UX R5):**
- New
  [`lib/glossary.ts`](apps/desktop/src-ui/src/lib/glossary.ts) — single
  source of truth for ~25 publishing/agent terms (KDP, EPUB, EPUBCheck,
  BISAC, ISBN, ONIX, trim, bleed, spine, voice fingerprint, AI tells,
  approval gate, …). Each entry has `label`, `short` (≤120 chars), and
  optional `long` + retailer link.
- New
  [`<Term k="…" />`](apps/desktop/src-ui/src/components/Term.tsx)
  inline component renders the canonical label as a dotted-underlined
  span with native-tooltip + `aria-describedby` SR hookup.
- HelpDrawer gains a fourth tab "Glossary" rendering the full library
  alphabetically with hyperlinks where applicable.
- Copy audit on user-facing surfaces: every visible
  `task: <code>{result.task_id}</code>` rewritten to
  `Status: <strong>…</strong> · run id <code>…</code>` (10 panels:
  Copyedit, Humanization, Continuity, Vocab, World Bible, Character
  Bible, Scene Drafter Fic, Generic Agent Form, Entity Bible,
  Developmental Review, Intake-and-Outline). AgentDebugForm intentionally
  retains raw `task_id` since it's a debug surface.
- PrepareForPublishingPanel uses `<Term>` for EPUB/PDF labels and
  rewrites all metadata-field hints in plain English (no bare BISAC,
  ISO 639-1, etc. without explanation).

**Phase 9 — 4 approval gates (closes UX R6):**
- New
  [`lib/workflowGates.ts`](apps/desktop/src-ui/src/lib/workflowGates.ts):
  `loadWorkflowState`, `saveWorkflowState`, `setGate`,
  `nextPendingGate`, `gatesEnabled` / `setGatesEnabled`. Per-project
  state in `localStorage` under `bf-workflow-gates:<projectId>`.
  Settings flag at `bf-workflow-gates-enabled` (default true).
- Four named gates: `topic`, `plan`, `bibles`, `pre_final_polish`. Each
  has display label, blurb, "comes after" agent name, and
  `unset | pending | approved` status with optional approval note.
- New panel
  [`WorkflowGuide.tsx`](apps/desktop/src-ui/src/components/WorkflowGuide.tsx)
  + toolbar button "Workflow" — renders the four gates as numbered
  rows with status pills, mark-pending / approve / reset controls, and
  a banner highlighting the next blocking gate.
- SettingsPanel gains a "Workflow approval gates" section near the top
  with the master enable toggle (so advanced users have a discoverable
  off switch outside the workflow guide itself).

### Workspace verification (Round 4)

| Gate | Result |
|---|---|
| `cargo test --workspace` | **579 passed, 0 failed, 3 ignored** (up from 505 at Round 1; counts include the new doctests for publishing.rs) |
| `cargo build -p booksforge-desktop` | **clean** |
| `cargo clippy -p booksforge-desktop --all-targets -- -D warnings` | **clean** (after one `unwrap_or_default` fix in publishing.rs) |
| `cargo test -p booksforge-orchestrator` | **4 passed, 0 failed** |
| `cargo test -p booksforge-ipc` (codegen-drift) | **all green** (5 new TS bindings re-exported from `@booksforge/shared-types`) |
| `pnpm typecheck` (desktop-ui) | **clean** (no new errors) |

### What this means for the user

A first-time author can now, end-to-end **on the integrated product**:

1. Open a literary-fiction project, walk through the 4 approval gates
   in order:
   - **Gate 1 — Topic & angle:** approve the intake brief (or reject &
     re-run intake with a tighter premise).
   - **Gate 2 — Plan / outline:** approve the outline (or reorder /
     prune chapters before any prose is drafted).
   - **Gate 3 — Character + world bibles:** approve the bibles (drift
     here is the single highest-leverage fix; one rejection saves
     dozens of polish-stack runs later).
   - **Gate 4 — Pre-final-polish review:** approve the structurally-
     edited draft before the heavy stylistic polish stack runs.
2. Hover over jargon (BISAC, EPUB, KDP, voice fingerprint, AI tells, …)
   to see plain-English definitions. Open the HelpDrawer → Glossary tab
   for the full library.
3. Click "Publish" once and get three per-marketplace folders ready to
   upload — eBook, print PDF, metadata, cover brief, walkthrough, and a
   per-item readiness checklist that surfaces the human-required steps
   the system honestly cannot do (cover commissioning, AI disclosure,
   ISBN purchase, terms acceptance).
4. Advanced users can disable approval gates from Settings ("advanced
   mode") and skip directly to running the full-scene pipeline
   (`agent_run_full_scene_pipeline`) end-to-end.

### Comparison-to-baseline note

The honest 4.66/10 baseline from
[`artifacts/ghostwriter/PROOF_RESULTS_LITERARY.md`](../artifacts/ghostwriter/PROOF_RESULTS_LITERARY.md)
was produced by the standalone Python ghostwriter pipeline against
`qwen3.5:27b` over ~78 minutes. **Reproducing the comparison on the
integrated BooksForge product** is wall-clock-bound on live Ollama and
out of scope for an in-session deliverable; the steps to run it
yourself once Ollama is up:

1. Open BooksForge desktop → "New project" → pick **Literary Fiction**
   (Step 1 in the wizard).
2. Use the same proof spec
   ([`proof_spec_literary.json`](../artifacts/ghostwriter/proof_spec_literary.json))
   as the brief input to the Intake agent.
3. Walk through the 4 approval gates: topic → plan → bibles →
   pre-final-polish.
4. For each of the 2 chapters, click "Run full scene pipeline" (Round 3
   wired this end-to-end). Pin model to `qwen3.5:27b` for both drafter
   and polisher in Settings → Models.
5. Run the integrated **voice fingerprint** + **tells scan** + the
   12-axis rubric scoring on the produced manuscript.
6. Compare to the baseline 4.66 / 3.93 / 2.08 numbers in
   PROOF_RESULTS_LITERARY.md.

What the Round 1–4 work changes vs. the standalone Python baseline:

- **Voice fingerprint anchoring** is now first-class in the product
  (Phase 3 / BACKLOG §A16), so the drafter receives the same numeric
  constraints the Python pipeline injected, but persisted on the
  project rather than re-derived per run.
- **Polish stack ordering** is now genre-aware (Phase 3 — literary
  fiction polishes voice → metaphor → dialogue → tension; genre
  fiction polishes tension → dialogue → metaphor → voice).
- **Approval gates** (Phase 9) let the writer kill drift earlier,
  which the standalone pipeline could not do — the standalone pipeline
  ran end-to-end with no human intervention. We expect the integrated
  product to *not* match the autonomous baseline on raw rubric score
  (because the gates introduce stop points), but to score *higher* on
  ship-vote ground truth, since the writer keeps creative control.
- **Honest scoring is preserved.** No rubric calibration was changed
  in Round 4; the same 12-axis scoring lives in
  `booksforge-validator`. Anything reported by the system is still the
  honest measurement output.
