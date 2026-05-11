# 07 — UX Scorecard

**Overall weighted score: 6.84/10**

_Per-check breakdown (weights in parentheses):_

| Check | Score | Weight | Status | Summary |
|---|---|---|---|---|
| 01_first_time_inputs | 10.0/10 | 1.5 | PASS | Beginner path: 4 required inputs (title, author, save-location). AI-brief adds 7 fields, 6 have non-empty defaults — premise alone has no default and effectivel… |
| 02_defaults_coverage | 4.7/10 | 1.5 | FAIL | 9/19 major workflow categories have defaults that the user can rely on. Big gaps: structure model, editorial strictness, originality level, KDP trim/bleed/inter… |
| 03_dynamic_branching | 6.0/10 | 1.5 | WARN | 3/5 dynamic branches detected. The UI does NOT adapt to fiction vs non-fiction, beginner vs advanced, or KDP-print vs ebook-only. Every user sees the same field… |
| 04_automation | 7.0/10 | 1.0 | PASS | 7/20 fully-automated, 7 AI-assisted, 6 user-required → 70% automated-or-assisted (brief gate: ≥70%). Marketplace + cover-art are necessary human steps. |
| 05_speed_complexity | 8.0/10 | 1.0 | PASS | Wizard has 3 step screens. ≈6 clicks and 4 text inputs to reach the first generated outline (with AI on). Brief target ≤3 setup screens met; ≤5 inputs met. |
| 06_progressive_disclosure | 7.5/10 | 1.0 | WARN | AI brief fields are hidden behind a `useAi` toggle (correct). However the main editor surface exposes ALL 14 agents in the AgentsPanel switchboard with no begin… |
| 07_approval_gates | 3.3/10 | 1.5 | WARN | 3/9 required approval gates exist. Outline-accept, per-edit-accept, export review, and pre-AI snapshot are present. The big missing gates: approve final topic, … |
| 08_failure_recovery | 7.5/10 | 1.0 | WARN | 6/8 recovery primitives exist (ErrorBoundary, RecoveryDialog, snapshot restore, pre-AI snapshot, cancel mid-job, EPUB validation). Missing: plain-English error … |
| 09_copy_microcopy | 4.5/10 | 0.8 | WARN | ~6 friendly action labels detected (Generate/Choose/Approve/Fix/Prepare/Preview/Export). ~37 jargon strings present in 19 files: ['BISAC', 'EPUBCheck', 'trim', … |
| 10_edit_review_bug | 9.0/10 | 2.0 | PASS | User-reported bug: AI output buried in collapsed JSON view with no path to the editor. UI-only fix landed in this session — generated prose now renders as a rea… |

## Per-check evidence

### 01_first_time_inputs — PASS 10.0/10

- `booksforge/apps/desktop/src-ui/src/components/NewProjectWizard.tsx:87` — required: ['Title', 'Author', 'Save location', 'Premise']
- `booksforge/apps/desktop/src-ui/src/components/NewProjectWizard.tsx:47` — AI-branch defaults populated for: ['genre', 'audience', 'tone', 'targetWordCount', 'targetChapterCount', 'model']
- Required-to-start (non-AI path): 4 (['Title', 'Author', 'Save location', 'Premise'])
- AI-brief fields with sensible defaults: 6/7

### 02_defaults_coverage — FAIL 4.7/10

- `booksforge/apps/desktop/src-ui/src/components/NewProjectWizard.tsx:47` — EMPTY FormState — defaults block
- 9/19 categories have defaults exposed in the wizard or export panel
- - **book_type**: project template → ✓
- - **genre**: default 'fantasy' → ✓
- - **audience**: default 'adult' → ✓
- - **tone**: default 'adventurous' → ✓
- - **length**: default 80,000 words → ✓
- - **chapter_count**: default 12 → ✓
- - **pov**: no default in wizard; user types in chapter-drafter form → ✗
- - **structure_model**: no explicit structure-model picker → ✗
- - **editorial_strictness**: not exposed → ✗
- - **originality_check**: not exposed → ✗
- - **keyword_optimization**: not exposed → ✗
- - **trim_size**: ExportPanel exposes trim → ✓
- - **interior_type**: not exposed in wizard → ✗
- - **bleed**: not exposed → ✗
- - **ebook_format**: ExportPanel offers epub → ✓
- - **marketplace_targets**: no explicit picker → ✗
- - **metadata_generation**: agent-driven, not in wizard → ✗
- - **cover_brief_style**: not exposed → ✗
- - **preview_mode**: ValidatorPanel + previews → ✓

### 03_dynamic_branching — WARN 6.0/10

- - fiction_vs_nonfiction: ✓
- - kdp_vs_ebook_only: ✓
- - beginner_vs_advanced: ✗
- - childrens_book_layout: ✗
- - chained_intake_outline: ✓
- `booksforge/apps/desktop/src-ui/src/components/agents/IntakeAndOutlinePanel.tsx:25` — chained intake → outline panel exists

### 04_automation — PASS 7.0/10

- - create project: user-required
- - pick template: AI-assisted
- - AI brief (genre/audience/tone): AI-assisted
- - outline generation: fully-automated (one click)
- - outline accept/reject: user-required
- - scene drafting: AI-assisted (per-scene apply, post-fix)
- - memory refresh: fully-automated
- - vocabulary updates: AI-assisted
- - copyedit: AI-assisted (per-edit accept)
- - humanization: AI-assisted
- - continuity: AI-assisted
- - validators: fully-automated (gate)
- - snapshots (auto-hourly): fully-automated
- - manual snapshot: user-required
- - DOCX export: fully-automated
- - EPUB export: fully-automated
- - PDF export: fully-automated (engine permitting)
- - metadata + KDP submission: user-required (NOT automated)
- - cover art: user-required (brief generated, art HUMAN_REQUIRED)
- - marketplace upload: user-required

### 05_speed_complexity — PASS 8.0/10

- `booksforge/apps/desktop/src-ui/src/components/NewProjectWizard.tsx:28` — type Step = 1 | 2 | 4
- steps: 3; clicks-to-first-concept (estimated): 6

### 06_progressive_disclosure — WARN 7.5/10

- `booksforge/apps/desktop/src-ui/src/components/NewProjectWizard.tsx:36` — useAi toggle on FormState
- `booksforge/apps/desktop/src-ui/src/components/agents/AgentsPanel.tsx:54` — AGENTS list — all 14 visible at once, no progressive split
- <details> blocks in wizard: 1
- 'advanced' label in ExportPanel: False
- 'advanced' label in SettingsPanel: False

### 07_approval_gates — WARN 3.3/10

- - approve_topic: not present — topic is implicit in brief → ✗
- - approve_book_plan: not present → ✗
- - approve_character_world: not present (no fiction agents) → ✗
- - approve_chapter_outline: OutlinePreview accept exists → ✗
- - approve_manuscript_pre_polish: not present → ✗
- - approve_export_settings: ExportPanel review screen exists → ✓
- - approve_metadata_marketplace: no marketplace submission flow yet → ✗
- - approve_individual_agent_edits: Copyedit/Continuity/Humanization per-edit accept → ✓
- - snapshot_before_ai_apply: snapshot trigger=pre_ai exists per IPC → ✓

### 08_failure_recovery — WARN 7.5/10

- - ErrorBoundary: ✓
- - RecoveryDialog: ✓
- - Snapshot restore: ✓
- - Pre-AI snapshot: ✓
- - Cancel mid-job: ✓
- - Plain-English errors: ✗
- - Suggested fix in errors: ✗
- - EPUB validation surface: ✓

### 09_copy_microcopy — WARN 4.5/10

- jargon-bearing components: ['booksforge/apps/desktop/src-ui/src/components/AgentDebugForm.tsx', 'booksforge/apps/desktop/src-ui/src/components/EditorShell.tsx', 'booksforge/apps/desktop/src-ui/src/components/ErrorBoundary.tsx', 'booksforge/apps/desktop/src-ui/src/components/ExportPanel.tsx', 'booksforge/apps/desktop/src-ui/src/components/FindReplaceBar.tsx', 'booksforge/apps/desktop/src-ui/src/components/InspectorPanel.tsx', 'booksforge/apps/desktop/src-ui/src/components/NewProjectWizard.tsx', 'booksforge/apps/desktop/src-ui/src/components/OutlinePreview.tsx']
- friendly action-verb count across all components: ~6

### 10_edit_review_bug — PASS 9.0/10

- `booksforge/apps/desktop/src-ui/src/components/agents/GenericAgentForm.tsx:75` — handleApplyToScene + pmDocToPlainText helpers added
- `booksforge/apps/desktop/src-ui/src/components/agents/AgentsPanel.tsx:110` — onApplied prop threaded to GenericAgentForm
- `booksforge/apps/desktop/src-ui/src/components/EditorShell.tsx:440` — EditorShell passes onApplied → ipc.sceneLoad → setSceneContent


## Verdict

# **FAIL** — overall 6.84/10

The brief's PASS gate is **9.0/10**. Current surface scores below that. The largest contributors to the gap are: missing dynamic branching (book-type), missing 'Prepare for Publishing' single action, missing approval gates (topic / plan / bibles / manuscript), and incomplete progressive disclosure in the AgentsPanel switchboard. The user-reported 'AI output not in editor' bug was real and was fixed in this session (UI-only patch); follow-up to route through the Orchestrator + audit ledger is tracked under BACKLOG §A9.