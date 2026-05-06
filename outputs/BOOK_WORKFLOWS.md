# Book Workflows — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for the end-to-end book-production pipeline.** Companion to `AGENTS.md`, `UI_UX_SPEC.md`, `MEMORY_SYSTEM.md`, `VOCABULARY_DICTIONARIES.md`, `EXPORT_EPUB_SPEC.md`.

This is the workflow contract — every stage from idea to publication-ready files, with **user action**, **agent action**, **inputs**, **outputs**, **UI state**, **data saved**, **error cases**, and **acceptance criteria** for each.

---

## Stage 0 — Discovery & intake

**User action.** From the Project Picker, click "New project" and pick a mode + template. Type a free-text idea (1–2 paragraphs) **or** skip.

**Agent action.** If the user typed an idea: run `IntakeAndOutline` workflow.

- **Project Intake Agent** turns the idea into a `ProjectBrief`. User reviews/edits.

**Inputs.** Mode, template id, free-text idea (≤4,000 chars), preferred language.

**Outputs.** A `ProjectBrief` JSON.

**UI state.** New Project Wizard step 3 shows the brief inline; user can edit any field.

**Data saved.** Nothing yet — the project is created at the end of the wizard.

**Error cases.**

- Idea is empty: agent runs only if "Generate outline" was chosen; otherwise skipped.
- Idea is non-book content (a poem, a recipe): agent flags `not_a_book = true`; user is asked to clarify.
- Ollama not running: wizard offers to launch the OllamaSetupWizard or fall back to "Skip — start blank."

**Acceptance criteria.**

1. A user with a 200-character idea sees a complete `ProjectBrief` within 8 seconds on the reference hardware.
2. A user can edit any brief field; the next agent (Outline Architect) sees the edited brief.

---

## Stage 1 — Book concept development

**User action.** Confirm or refine the brief produced by the Project Intake Agent.

**Agent action.** None at this stage; the user holds the pen.

**Inputs.** The brief from Stage 0.

**Outputs.** A confirmed brief, used as input to the Outline Architect.

**UI state.** Wizard step 3, brief edited.

**Data saved.** Nothing — still in the wizard.

**Acceptance criteria.** The user can override every field, including book mode (which propagates the right starter vocabulary dictionaries to Stage 4 onwards).

---

## Stage 2 — Audience & genre analysis

**User action.** Implicit; the brief contains audience, genre, sub-genre, tone.

**Agent action (V1.0).** The **Book Strategy Agent** produces audience analysis, comp titles, positioning notes, risks. **Not in MVP.**

**Inputs.** The brief.

**Outputs.** (V1.0) `BookStrategyReport` — audience analysis, comp titles, positioning notes.

**UI state.** Bookself / Strategy tab (V1.0).

**Data saved.** (V1.0) `book_strategy_report` table.

**Acceptance criteria (V1.0).** Comp titles are real titles (named explicitly to avoid hallucination); strategy report ≤ 1,500 words.

---

## Stage 3 — Outline creation

**User action.** Click "Generate outline" in the wizard.

**Agent action.** **Outline Architect Agent** proposes parts → chapters → scenes from the brief.

**Inputs.** Confirmed brief; target chapter count; optional genre overlay.

**Outputs.** `OutlineProposal` JSON (per `AGENTS.md §4.2`).

**UI state.** Wizard step 3 transitions to a live progress view, then a tree view of the outline. User can edit chapter titles, synopses, target word counts inline.

**Data saved.** Nothing yet — accepted in Stage 4.

**Error cases.**

- Outline collapses to "beginning, middle, end": semantic validator rejects; orchestrator retries once with a "be more specific" reminder.
- User wants different chapter count: regenerate with new `target_chapter_count`.

**Acceptance criteria.**

1. For a brief targeting 90,000 words, the outline proposes a chapter count within ±20% of the user's target.
2. No two scenes have identical synopses (semantic validator).
3. Total target word count of all scenes is within ±20% of the brief's `target_word_count`.

---

## Stage 4 — Chapter planning

**User action.** Accept the outline. The wizard creates the project bundle and the document tree.

**Agent action.**

- **Memory Curator Agent** creates initial `book_memory` from the brief.
- **Vocabulary Dictionary Agent** seeds the project-layer dict by merging the genre + sub-genre + audience starter dictionaries.
- (V1.0) **Chapter Planning Agent** can produce per-chapter scene plans from the outline.

**Inputs.** The accepted `OutlineProposal`, the brief.

**Outputs.** A `*.booksforge/` bundle with: `manifest.toml`, `project.db`, `manuscript/` skeleton, `assets/`, empty `snapshots/`. Plus initial `book_memory`, seeded vocab dictionaries, document tree (Parts → Chapters → empty Scenes).

**UI state.** Workspace opens at the first scene. Binder shows the full tree.

**Data saved.** All of the above committed atomically (temp dir + rename).

**Error cases.**

- Folder path not writable → typed error, prompt for another folder.
- Crash during atomic creation → no half-bundle on disk (temp dir abandoned).

**Acceptance criteria.**

1. New project visible in recent-projects list within 2 seconds.
2. Killing the app mid-create leaves no half-bundle.
3. Initial `book_memory.last_writer = 'memory-curator'` row exists.
4. Vocabulary dictionaries for the active template are populated.

---

## Stage 5 — Drafting (the writer's main loop)

**User action.** Type into scenes. Use quick-action presets (Sharpen, Continue, Rephrase, Shorten, Expand) for inline help.

**Agent action.**

- **Quick-action presets** are not agents — they are single-shot prompt presets through Ollama. Pre-edit snapshot before any apply.
- **Chapter Drafting Agent** is opt-in (off by default). When invoked, it drafts a scene from the synopsis. The draft lands in a working buffer; the user merges manually.
- **Memory Curator Agent** is **off** during active drafting (no per-keystroke noise). Runs on chapter finalise.
- **Vocabulary Dictionary Agent** is **off** during active drafting. Observes accepted edits in batches.

**Inputs.** The user's typing; Ollama for presets.

**Outputs.** Scene content saved to `scene_content` (ProseMirror JSON + Markdown mirror).

**UI state.** Editor centre pane. Status bar: word count + autosave indicator + Ollama status.

**Data saved.** Autosave at 5s after last keystroke or on blur.

**Error cases.**

- Ollama unreachable mid-preset: typed error, recovery actions.
- Disk full: typed error, no half-write.
- `kill -9` mid-edit: recovery prompt on next launch restores buffer.

**Acceptance criteria.**

1. Typing latency p95 ≤ 30 ms.
2. Autosave fires within 5s of last keystroke.
3. Crash recovery restores within a single prompt.
4. A 200-word Sharpen preset on the reference hardware completes in ≤ 6 s.

---

## Stage 6 — Developmental editing

**User action.** Click "Developmental review" on a chapter or the project. (Whole-project on a >150k-word manuscript is allowed but the UI warns about cost.)

**Agent action.** **Developmental Editor Agent** produces structural notes.

**Inputs.** Chapter text, outline context, character cards (entity_memory).

**Outputs.** `DevelopmentalNotes` (per `AGENTS.md §4.4`) — pacing, structure, characterization, stakes, scene-goal issues.

**UI state.** Right panel → AI / Agents → Developmental Review tab. Each issue is clickable to jump to the location.

**Data saved.** `agent_runs`, `agent_tasks`, `agent_outputs` rows. Notes saved as project review (not applied to manuscript).

**Error cases.**

- Hallucinated character names: `EntitySanityCheck` flags them; surfaced in UI.
- Schema-invalid output: orchestrator retries up to 2x; if still invalid, surfaces a `proposal_invalid` artifact.
- Run cancelled mid-stream: partial outputs preserved as inspectable artifacts.

**Acceptance criteria.**

1. Per-chapter run completes in ≤ 30 s on the reference hardware.
2. No proposal references a proper noun absent from the bible without a warning.
3. Severity distribution is sensible (not all "high"; semantic validator).

---

## Stage 7 — Structural revision

**User action.** Convert any developmental note into a TODO; revise affected scenes; reorder scenes via the binder.

**Agent action.** None directly — this is the writer's revision pass.

**Inputs.** The user's edits, drag-and-drop reorders.

**Outputs.** Updated scene content, possibly new chapter structure.

**UI state.** Editor + binder.

**Data saved.** `nodes` (on reorder), `scene_content` (on edit), Markdown mirror.

**Acceptance criteria.** Drag-reorder of a scene completes in <100 ms perceived; references update; undo restores original.

---

## Stage 8 — Continuity check

**User action.** Click "Continuity check" on a chapter or the whole project.

**Agent action.**

- The deterministic continuity linter (in `booksforge-validator`) runs first.
- The **Continuity Agent** adjudicates ambiguous findings.

**Inputs.** Project view; deterministic findings; `entity_memory`.

**Outputs.** `ContinuityReport` (per `AGENTS.md §4.5`).

**UI state.** Right panel → Validators → Continuity Findings.

**Data saved.** `agent_runs`, `agent_tasks`, `agent_outputs`. Renames are user-gated; on accept, `entity_memory` is updated and a pre-edit snapshot is taken.

**Error cases.** False positives on intentional aliases — the deterministic linter is given the alias list explicitly.

**Acceptance criteria.**

1. The linter completes a 100k-word project in ≤ 5 s.
2. The agent adjudication completes in ≤ 30 s for ≤ 20 ambiguous findings.

---

## Stage 9 — Line editing (V1.0)

**User action (V1.0).** "Line edit this passage" command.

**Agent action (V1.0).** **Line Editor Agent** proposes passage-level rewrites with rationales.

**Outputs.** Inline diff per proposal.

**MVP equivalent.** The Sharpen / Rephrase quick-action presets and the Humanization Agent (which proposes human alternatives, not arbitrary rewrites). Line Editor is V1.0.

---

## Stage 10 — Copyediting

**User action.** Click "Copyedit" on a scene, chapter, or project.

**Agent action.** **Copyeditor Agent** proposes mechanical fixes (per `AGENTS.md §4.6`).

**Inputs.** Scene text + the project's `style_book`.

**Outputs.** `CopyeditProposals` — per-edit diffs with category and rationale.

**UI state.** Inline diff in the editor with Accept / Reject / Accept-by-category affordances.

**Data saved.** `agent_runs`, `agent_tasks`, `agent_outputs`. Accepted edits trigger a `pre_agent_edit` snapshot, then update `scene_content`. The Vocabulary Dictionary Agent observes the accepted batch.

**Error cases.** Range mismatch (model fabricates positions) → rejected at validation; retry once.

**Acceptance criteria.** A 2,000-word scene's copyedit completes in ≤ 20 s on the reference hardware.

---

## Stage 11 — Style consistency check (V1.0)

**User action (V1.0).** "Style review" command.

**Agent action (V1.0).** **Style Guide Agent** detects voice/tone drift; proposes questions for the Line Editor.

**MVP equivalent.** The Humanization Agent surfaces robotic prose in the same scope. Style Guide is V1.0.

---

## Stage 12 — Humanization (anti-robotic prose)

**User action.** Click "Humanize this scene/chapter."

**Agent action.** **Humanization Agent** surfaces AI-tells and robotic constructions; proposes human alternatives using the merged vocab dictionaries plus style memory.

**Inputs.** Scope text; merged vocab decisions; style memory.

**Outputs.** `HumanizationProposals` — per-passage diffs with category and rationale citing vocab entries.

**UI state.** Inline diff with category labels (`ai-tell`, `rhythm`, `register`, `repetition`, `filler`).

**Data saved.** `agent_runs`, `agent_tasks`, `agent_outputs`. Accepted proposals trigger a snapshot and reinforce the vocab entries; rejected ones with "this is my voice" add `prefer` entries to the project-layer dict.

**Error cases.** Voice flattening — mitigated by the user's "this is my voice" affordance.

**Acceptance criteria.**

1. On a fixture passage that contains "in today's world", the agent surfaces a proposal citing the `audience.builtin.ai-tells.in-todays-world` entry.
2. Rejecting a proposal three times for the same vocab entry downgrades the entry's confidence (next run is less likely to surface it).

---

## Stage 13 — Fact-checking & citation review (V1.0)

**Not in MVP.** V1.0 adds the Fact-Check Agent (bibliography-grounded), CSL citation engine, and BibTeX/Zotero import.

---

## Stage 14 — Style guide enforcement

**User action.** Set or revise the project's `style_book` (em-dash style, Oxford comma, quote style, etc.) from project settings.

**Agent action.** None — the style book is the authority. The Copyeditor Agent reads from it.

**Inputs.** User edits in the Style Book settings page.

**Outputs.** Updated `style_book` table; `manifest.toml.[style_book]` round-tripped.

**UI state.** Settings → Project → Style Book.

**Data saved.** `style_book` row 1; `manifest.toml`.

**Acceptance criteria.** Editing a style book field and immediately running Copyeditor on a fixture scene reflects the change.

---

## Stage 15 — Memory curation (continuous, but explicit on finalise)

**User action.** Mark a chapter as "final" from the binder.

**Agent action.** **Memory Curator Agent** runs a full pass on the chapter: refreshes summaries, key events, introduced/reintroduced entities, open/resolved loops.

**Inputs.** Chapter text; current chapter_memory; relevant entity_memory.

**Outputs.** `MemoryRefreshProposals` (per `AGENTS.md §4.7`).

**UI state.** Memory tab. Each proposal is an inline diff against the current memory.

**Data saved.** `chapter_memory`, `entity_memory`, `book_memory` (only if book-level fields shift). Pre-edit snapshot before any accepted write.

**Acceptance criteria.** Per-chapter finalise completes in ≤ 45 s on the reference hardware for a 5,000-word chapter.

---

## Stage 16 — Vocabulary dictionary maintenance (continuous)

**User action.** Implicit during drafting + revision; explicit when reviewing the Vocabulary tab.

**Agent action.** **Vocabulary Dictionary Agent** runs after every batch of 5 accepted Copyeditor / Humanization edits and on chapter finalise.

**Inputs.** Recent accepted edits; current project-layer dict summary.

**Outputs.** `VocabUpdateProposals` (per `VOCABULARY_DICTIONARIES.md §6`).

**UI state.** Vocabulary tab → Pending proposals inbox.

**Data saved.** `vocab_entries`, `vocab_updates`. Each accepted proposal becomes a `vocab_updates` row.

**Acceptance criteria.**

1. After accepting 5 user edits that replace "delve" with "look at", the agent proposes a `replace` entry.
2. The user can revert any vocab update from the audit trail.

---

## Stage 17 — Formatting

**User action.** Choose a template profile (e.g., "Trade Paperback 5×8" or "KDP-eBook EPUB-3") in the Export dialog.

**Agent action.** None in MVP. (V1.0 Formatting Agent handles only template-vs-override conflicts.)

**Inputs.** Document tree + template + style book + project overrides.

**Outputs.** Canonical Document → Canonical HTML / Pandoc-AST.

**UI state.** Export dialog showing the active profile and its options.

**Data saved.** Nothing yet — committed only on actual export.

**Acceptance criteria.** Template hot-swap with diff preview completes in ≤ 1 s.

---

## Stage 18 — Pre-export validation gate

**User action.** Click "Run pre-export check."

**Agent action.** None directly. The validator engine runs all manuscript validators + the format-specific validator (KDP-eBook). EPUBCheck runs if EPUB profile is selected.

**Inputs.** Project view + selected profile.

**Outputs.** Validator report: errors / warnings / info.

**UI state.** Pre-export gate dialog with summary; clickable issues that jump to source.

**Data saved.** `validator_runs`, `validator_issues`.

**Acceptance criteria.**

1. Errors block the export until resolved (override allowed for advanced users with explicit confirmation).
2. A 100k-word project completes the validator suite in ≤ 10 s.

---

## Stage 19 — Export

**User action.** Click "Export now" after the gate passes.

**Agent action.** None in MVP. (V1.0 ePUB Export QA Agent reads results and proposes user-friendly fixes.)

**Inputs.** Canonical Document + asset list + format/profile.

**Outputs.** Exported file at the user's chosen path; an `exports` row.

**UI state.** Progress bar; on success, "Reveal in Finder/Explorer" + entry in export history.

**Data saved.** `exports` row + the output file. A copy is kept under `exports/<timestamp>-<format>` inside the bundle (last 10 retained).

**Error cases.**

- EPUBCheck error: surfaced; export refused.
- Pandoc crash: typed error; partial files cleaned.
- Disk full: typed error; partial files cleaned (atomic rename).

**Acceptance criteria.**

1. A 100k-word project exports to all four MVP profiles in ≤ 60 s.
2. Reproducibility test passes (byte-identical output across runs).
3. Visual regression test passes (preview vs. EPUB content under tolerance).

---

## Stage 20 — Final QA (V1.0)

**User action (V1.0).** Click "Final review."

**Agent action (V1.0).** **Final Review Agent** gathers issues from prior agent runs, validators, and memory; produces a go/no-go report with a confidence rating per chapter.

**MVP equivalent.** The pre-export validator gate plus the manual pre-release checklist (`EXPORT_EPUB_QA.md §11`).

---

## Stage 21 — Version management (continuous)

**User action.** Click "Snapshot now" or rely on automatic snapshots.

**Agent action.** None — the snapshot system is rule-based.

**Inputs.** Current project state.

**Outputs.** Content-addressed objects under `snapshots/objects/` and a row in `snapshots`.

**UI state.** Snapshots tab → timeline. Diff vs current; Restore (whole or selective).

**Data saved.** Snapshots are immutable until garbage-collected.

**Acceptance criteria.** Restore round-trip preserves prior state byte-for-byte (modulo timestamps).

---

## Cross-stage acceptance: "the writer's day"

Anya can:

1. Open her project (`*.booksforge/`) — workspace ready in ≤ 2 s.
2. Type for an hour with autosave + crash recovery; ≥ 0 lost keystrokes after a `kill -9` test.
3. Run "Developmental review" on chapter 12; review notes; convert two into TODOs.
4. Run "Continuity check" on the project; rename a character via the Continuity Agent's proposal (pre-edit snapshot taken; entity_memory updated).
5. Run "Copyedit" on chapters 11–12 (batch); accept all by category for "punctuation" and "spacing".
6. Run "Humanize" on chapter 12; accept three proposals citing AI-tells.
7. Mark chapter 12 final; the Memory Curator refreshes summaries.
8. Export to KDP-eBook EPUB-3; gate passes; export completes in ≤ 30 s; downloaded file matches preview.

That sequence is the MVP's daily-use acceptance test.
