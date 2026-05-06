# UI / UX Spec — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **MVP-scope screens only.** Post-MVP screens (collaboration, marketplace, sync, mobile) are out of scope here.

This document defines the screens Claude Code must build for the MVP and the user flows that thread them. Visual fidelity (typography, colour, spacing tokens) is not specified here; the design system lives in `packages/ui/`. This is the *behaviour* spec.

---

## 1. Information architecture

The application has six top-level surfaces. Only the first four ship in MVP.

| # | Surface | MVP | Description |
|---|---------|-----|-------------|
| 1 | Project Picker | ✅ | App launch screen; recent projects, new project, open existing |
| 2 | Project Workspace | ✅ | The main editor view with sidebars |
| 3 | Settings | ✅ | App and project preferences, model settings, style book |
| 4 | Onboarding / Setup | ✅ | First-launch wizard including Ollama setup |
| 5 | Marketplace | ❌ | Plugin browse and install — V1.5 |
| 6 | Account | ❌ | Studio sign-in, license — V1.0+ |

## 2. Project Picker

**Purpose.** The first screen on launch when no project is open.

**Layout.**

```
┌──────────────────────────────────────────────────────────────────┐
│  BooksForge                                                       │
│                                                                  │
│  [+ New project]   [Open project…]   [Import…]                  │
│                                                                  │
│  Recent                                                          │
│  ┌────────────────────────────────────────────┐                 │
│  │ The Cartographer's Daughter                │  yesterday      │
│  │ /Users/anya/Books/Cartographer.booksforge   │                 │
│  ├────────────────────────────────────────────┤                 │
│  │ Q3 Strategy Memo Book                      │  2 days ago     │
│  │ ...                                        │                 │
│  └────────────────────────────────────────────┘                 │
└──────────────────────────────────────────────────────────────────┘
```

**Behaviours.**

- Recent list shows pinned projects on top, then by `last_opened_at` descending. A "missing" pill appears when the bundle path no longer exists; click reveals "Locate…" or "Remove from list."
- "New project" launches the New Project Wizard (§3).
- "Open project…" opens an OS file picker filtered to `.booksforge` directories.
- "Import…" opens an OS file picker for `.docx`, `.md`, `.txt`. Followed by an Import Configuration modal (§7).
- Keyboard: `Cmd/Ctrl+N` new, `Cmd/Ctrl+O` open, `Cmd/Ctrl+I` import.
- Telemetry: nothing leaves the device.

## 3. New Project Wizard

**Purpose.** Create a project end-to-end with optional agent-driven outline generation.

**Steps.** Four steps. The user can back up at any step.

### Step 1 — Mode and template

The user picks a mode (Fiction / Non-fiction / Memoir / Academic), then a template. Each template card shows a one-line description, a thumbnail, and a "preview" link.

### Step 2 — Project metadata

Title, subtitle, author(s), language, target word count, manuscript folder location.

### Step 3 — Idea (optional, kicks off agents)

A multiline text area: "Tell us about the book in a paragraph or two." Below it, two options:

- **Skip — I'll start blank.** Creates the project from the template skeleton with placeholder chapters; closes the wizard.
- **Generate outline with AI.** Runs the `IntakeAndOutline` workflow. Requires Ollama setup (the wizard launches the Ollama Setup if not detected — §4).

If "Generate outline" is chosen:

1. The wizard transitions to a live progress view: "Project Intake Agent" → spinner → result preview, then "Outline Architect Agent" → spinner → result preview.
2. The user reviews the brief and outline, can edit any field inline, and clicks "Create project from outline."
3. The orchestrator creates the document tree from the outline and seeds entity stubs from the brief.

### Step 4 — Confirmation

A summary and a final "Create project" button. On success, the workspace opens to the first scene.

**Failure modes handled.**

- Folder path not writable: typed error, prompt for another folder.
- Ollama not detected and the user chose "Generate outline": offer the Ollama Setup wizard or fall back to "Skip — I'll start blank."
- Agent run errors: the user can retry, regenerate, or proceed with the partial output.

## 4. Ollama Setup Wizard

**Purpose.** Get the user from "no AI" to "AI ready" with a working model.

**Steps.**

1. **Detect.** Probe `127.0.0.1:11434/api/version`.
   - If reachable: jump to Step 3.
   - If not reachable but a binary is found on disk: offer "Launch Ollama" (one click).
   - If not installed: Step 2.
2. **Install.** Two options: "Guided install" (download official installer with pinned hash and run it), or "Manual install" (copyable URL and instructions).
3. **Pick a model.** Show the curated list filtered by detected RAM. Recommend a default. The user can pick, then "Pull model" with progress.
4. **Smoke test.** Run a tiny "Are you working?" generation against the chosen model. On success, finish.

**Failure modes handled.**

- Disk space insufficient for the chosen model: clear error, suggest a smaller model.
- Pull interrupted: resume on retry.
- Ollama starts but rejects the chosen model: suggest pulling another.

The wizard can be re-entered any time from Settings → AI / Models.

## 5. Project Workspace

**Layout.** A three-pane shell.

```
┌──────────┬──────────────────────────────────────┬───────────────┐
│ Sidebar  │            Editor                    │  Right Panel  │
│ (Binder) │  (TipTap, virtualised by chapter)    │  (Tabs)       │
│          │                                      │               │
│ Project  │  Chapter 1 — The Map Room            │  Notes        │
│  ▸ Part1 │                                      │  AI / Agents  │
│   ▸ Ch1  │  The brass key sat heavier than…    │  Validators   │
│     • S1 │                                      │  Bible        │
│     • S2 │                                      │  Snapshots    │
│   ▸ Ch2  │                                      │               │
│  ▸ Part2 │                                      │               │
└──────────┴──────────────────────────────────────┴───────────────┘
                            Status bar
   [Saved · 86,234 words · Today 1,240 · Snapshot 22m ago · Ollama: connected · qwen2.5:7b]
```

### 5.1 Left sidebar — Binder

A collapsible tree of nodes. Drag-reorder updates the document tree atomically. Right-click context menu: New Scene, New Chapter, Rename, Duplicate, Delete (soft-delete with undo).

Status colours per scene (planned / drafting / revised / final). Word count rollups per chapter and part.

A second tab in the sidebar: **Outline view** — flat list with synopsis, POV, beat, target word count, status. Editing inline updates the same `nodes` rows.

### 5.2 Centre — Editor

TipTap with the MVP's supported nodes (paragraph, H1–H4, bold/italic/underline, blockquote, lists, code block, image, footnote, comment anchor, link). Virtualised: only the current chapter mounts a full ProseMirror EditorView; adjacent chapters lazy-mount on scroll proximity.

Quick-action bar (`Cmd/Ctrl+K`): Sharpen, Continue, Rephrase, Shorten, Expand. Each is a single-shot call to Ollama using a versioned prompt template — **not** an agent. Suggestions appear in a side panel with a diff view; user accepts/rejects/regenerates. Pre-edit snapshot taken on accept.

### 5.3 Right panel — Tabs

Six tabs, each is a separate panel; only one active at a time.

- **Notes.** Free-form notes attached to the current scene or chapter.
- **AI / Agents.** The Agent Activity surface — see §6.
- **Validators.** Last validator run results with click-to-jump.
- **Bible / Memory.** Entity cards plus book / chapter / entity memory inspection. Inline edit; "Find scenes" filter on click. The **Memory Curator Agent** runs from this tab (per `MEMORY_SYSTEM.md`); proposals appear as inline diffs against current memory. In MVP this tab also hosts a **Vocabulary** sub-section where the **Vocabulary Dictionary Agent**'s pending proposals queue is shown (per `VOCABULARY_DICTIONARIES.md §9`); V1.0 promotes Vocabulary to its own tab.
- **Snapshots.** Timeline of snapshots with diff vs current and "Restore" actions.

The status colour of the active tab matches the current open run's status (running / awaiting_user / proposal_invalid).

### 5.4 Status bar

Always visible. Items: save state, project word count, today's word count, last snapshot age, Ollama status (connected / disconnected / pulling), current default model.

Clicking the Ollama status opens the Ollama Setup wizard or model picker.

## 6. AI / Agents panel

**Purpose.** Run agents, watch progress, review proposals.

### 6.1 Top of the panel — Run picker

A grouped action menu of the workflows available in the current scope:

- **From the project root:** `Intake & Outline` (only when project has a draft brief); `Continuity Check`; `Developmental Review` (full project — discouraged for >150k words; the UI warns); `Refresh Book Memory`.
- **From a chapter:** `Developmental Review` (this chapter); `Continuity Check` (this chapter); `Copyedit` (this chapter); `Humanize` (this chapter); `Refresh Memory for this chapter`. The Memory Curator runs automatically on chapter finalise — surfaced here as a confirmation dialog rather than a manual button.
- **From a scene:** `Copyedit` (this scene); `Humanize` (this scene); `Draft this scene` (off by default — opt-in toggle in scene settings).

Clicking a workflow opens the **Context Preview** (§6.2) before running.

A **Vocabulary Dictionary Agent** runs implicitly after every batch of 5 accepted Copyeditor / Humanization edits; its proposals appear in the Bible / Memory tab's Vocabulary sub-section and are not blocking — the user can review them at leisure.

### 6.2 Context Preview

Before any agent run, the user sees exactly what context will be sent:

- The selected scope text (collapsed by default; expandable).
- Entity cards selected (each can be deselected).
- Style book and tone preset.
- Token estimate vs. model context window — bar with green/yellow/red.
- Model selection (defaults from project settings; per-call override).

A "Run" button starts the workflow; "Cancel" returns to the panel root.

### 6.3 Live run view

While a workflow runs, the panel shows:

- Workflow name and elapsed time.
- Step list with status (pending / running / completed / failed).
- For the running step: streaming token preview (truncated to ~500 chars).
- Cancel button.

### 6.4 Proposal review

When a step completes and is awaiting user gate:

- Header: which agent, what scope, when run.
- Body: structured display of the proposal (e.g., for Outline: a tree; for Copyedit: a list of inline diffs; for Developmental: a list of issues).
- Per-item accept/reject; bulk accept/reject by category where it makes sense.
- "Regenerate" runs the same agent again with a counter that respects the run's retry budget.
- "Inspect raw output" reveals the JSON for power users.

### 6.5 Run history

A second tab inside the panel: a list of past runs for this project with status, duration, and "Open" action that re-opens the proposal review (read-only — applied edits are immutable, but the proposal can still be viewed).

## 7. Import flow

**Trigger.** "Import…" from Project Picker or "File → Import" inside a project.

**Steps.**

1. File picker (`.docx`, `.md`, `.txt`).
2. Configuration modal: target as new project (default) or merge into current project. For new project, choose mode and template; for merge, pick the parent node.
3. Preview: the importer shows the inferred structure (parts, chapters, scenes) and the user can toggle/edit before committing.
4. On commit: an atomic write. On failure (file unreadable, content too large, embedded scripts in DOCX): typed error.

Tracked changes from DOCX are **stored** in MVP but not displayed inline (V1.0 displays them); the user is informed.

## 8. Settings

**Per-app** (lives in `~/.booksforge/settings.toml`):

- Theme (system / light / dark / high-contrast).
- Editor font and size.
- Autosave interval (default 5s).
- Reduced-motion preference.
- Telemetry (off by default).
- Crash reports (off by default).
- Update channel (stable / beta — `beta` during MVP).

**Per-project** (lives in `manifest.toml` and `*_settings` tables):

- AI on/off (off by default; one-time consent prompt enables it).
- Default model and per-agent overrides.
- Ollama host (default `http://127.0.0.1:11434`).
- Style book.
- Snapshot policy (auto-snapshot interval, retention).
- Pre-export validator policy (block on errors / warn).

**Models page** (under Settings → AI / Models):

- Ollama status, version, host.
- Installed models list (read from Ollama at open and on Refresh).
- Curated model list with Pull buttons.
- Model picker for the project default + per-agent overrides.
- "Open Ollama Setup wizard" link.

## 9. Validators panel

A list view grouped by severity (errors / warnings / info). Each item shows: rule id, message, location hint, optional one-click fix. Click to jump to the location in the editor; the cursor lands at the issue range. The "Run all validators" button at the top runs the full suite for the current project.

Pre-export gate: on Export, validators run; the gate dialog shows summary; errors block (with override for advanced users); warnings prompt; info silent.

## 10. Snapshots panel

A reverse-chronological timeline. Each entry shows: timestamp, label (auto / pre-AI / pre-agent-edit / pre-export / manual), trigger reason, size delta, and actions: Diff vs current, Restore (whole or selective by node), Export this snapshot as a `.booksforge.zip` for transport.

A pre-restore confirmation dialog explains: "Restoring will create a new pre-restore snapshot first."

## 11. Bible panel

A grid of entity cards. Each card: name, type (character / location / item / organisation / theme), aliases, fields (typed per type), notes, and a "Scenes (n)" link to a filter view of scenes in which the entity appears. Add / edit / delete with undo.

Auto-extraction (regex + capitalisation heuristic) suggests entities at import; the user confirms each.

## 12. Export dialog

**Trigger.** File → Export, or `Cmd/Ctrl+E`.

**Layout.** A modal with tabs for each format (DOCX, PDF, EPUB-3) and a "Profile" picker per format. The active profile shows its options (page size, font, ToC depth) read from the template + user overrides.

**Action.** "Run pre-export check" → validators. If all green, "Export now" runs the pipeline. Progress bar; on success, the result is shown with a "Reveal in Finder/Explorer" button and entered into export history.

## 13. Empty states and errors

Every screen has an empty state:

- Project Picker with no recent: "Create your first book."
- Editor with no content: a short "Start with the first scene" with a one-click way to drop in a stub.
- Validators with nothing run: "No validators have been run yet."
- AI / Agents with no model: "Set up Ollama to use AI features."

Error UI is consistent: a non-blocking banner for recoverable errors, a modal for blocking errors. Errors include an action (Retry, Reconfigure, Cancel) and a short cause sentence. No raw stack traces.

## 14. Accessibility

Hard requirements for MVP:

- Every action is reachable by keyboard. There is a documented shortcut map.
- All actionable elements meet WCAG 2.2 AA contrast.
- Reduced-motion mode honours OS preference.
- Focus order is logical and visible.
- Form labels are associated with inputs.
- The editor has a minimum text-size and line-height pair set by the user, independent of export typography.

Screen reader basics work; full SR support across complex panels (live run streaming) is V1.0.

## 15. Performance UX

- Cold launch shows a splash within 200 ms; the picker is interactive within 1 s.
- Opening a 100k-word project shows the binder in <500 ms; the editor begins streaming the first chapter within 1 s; full readiness within 2 s.
- Agent runs show a token stream within 2 s of starting.
- Cancel always responds within 1 s.

## 16. Telemetry on UI

No UI event leaves the device by default. When telemetry is enabled, events are coarse and metadata-only: which screen was opened, which workflow was started, duration, whether it succeeded — never any content.

## 17. Out of scope for MVP UI

- Sign-in / accounts surfaces.
- Marketplace.
- Mobile companion.
- Live collaboration cursors / presence.
- Multi-window / tab-per-project (single window in MVP).
- Plugin panels.
