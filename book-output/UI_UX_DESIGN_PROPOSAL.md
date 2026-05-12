# BooksForge — UI/UX Design Proposal

**Version:** 0.9 (proposal) • **Date:** 2026-05-08 • **Author:** Senior product design • **Status:** For team review

This document proposes the desktop UI/UX for BooksForge MVP. It is a deliberate reconciliation and tightening of `outputs/UI_UX_SPEC.md` against the seven non-negotiables in the brief: minimal-but-not-empty, writer-first, local-first/privacy-loud, one-bar-one-shelf-one-room, keyboardable, KDP/Google-Books-ready in one click, no-model-fiddling-theater.

The document is opinionated. Where it disagrees with the existing spec, it says so and why (§J). Where it punts, it says so (§K, §L).

---

## A. Information architecture

BooksForge is a **single-window, single-document desktop app**. There are no tabs, no multi-project shells, no detachable panels in MVP. The entire application is one of seven screens at a time. Modals are reserved for actions that genuinely block (export, destructive confirms, setup wizards).

### A.1 Screen inventory (MVP)

| # | Screen | Modality | Reach |
|---|--------|----------|-------|
| 1 | First-launch / Ollama Setup | Full-window wizard | First run only; re-enterable from Settings |
| 2 | Project Picker | Full-window | Default when no project is open |
| 3 | New-Project Wizard | Full-window wizard | From Picker `+ New` |
| 4 | Workspace | Full-window, four-region | The room where writing happens |
| 5 | AI Proposal Review | **Right inspector** in Workspace | Triggered by an agent finishing |
| 6 | Memory / Bible | **Right inspector** in Workspace | `Cmd/Ctrl+B` |
| 7 | Validators / Pre-export QA | **Right inspector** + modal gate | `Cmd/Ctrl+'` and on Export |
| 8 | Export | Modal | `Cmd/Ctrl+E` |
| — | Settings | Modal | `Cmd/Ctrl+,` |
| — | Shortcut Cheat Sheet | Modal overlay | `?` |

The Workspace is the **primary surface**. All "screens" 5–7 are inspector tabs inside it. Memory, Validators, Snapshots, Agents, Notes are not separate destinations — they are right-inspector contexts. This is the "one room" rule.

### A.2 First-run journey ("idea → KDP-ready files")

```
Launch → Ollama Setup (4 steps, ~3 min)
       → Project Picker (empty)
       → New-Project Wizard
            • Step 1: Mode + template
            • Step 2: Title, target words, location
            • Step 3: Idea (optional → IntakeAndOutline workflow)
            • Step 4: Confirm
       → Workspace opens to Chapter 1 / Scene 1
            • Writer drafts; quick-actions on demand (Cmd+K)
            • On chapter finalise → Memory Curator confirmation
            • On demand → Copyedit / Continuity / Humanize
            • On demand → Final Review Editor (last pass)
       → Export (Cmd+E)
            • Pick target: KDP Paperback | KDP Kindle | Google Books
            • Pre-flight validators run automatically
            • If green → Export now → Reveal in Finder
            • If red → "Fix and retry" with one-click jumps
```

Eight steps, three of them invisible after first run. Median time-to-first-export on a 60k-word draft should be under 30 seconds once validators pass.

### A.3 Navigation model

- No persistent navigation rail. The user is always inside one of the seven screens; transitions are explicit (a button, a shortcut, or a workflow event).
- The **status bar at the bottom of Workspace** is the only persistent navigator: clicking the project name returns to Picker; clicking the Ollama dot opens Setup; clicking the model name opens the Models page.
- The **right-inspector** is context-driven: it never shows two contexts at once. Switching contexts is cheap (a single key) and stateful (returning re-opens the last sub-tab).

---

## B. Layout system — one bar, one shelf, one room, one inspector

The Workspace is divided into four regions. **Three of them collapse**.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ TOP BAR · 36 px                              [project] [⌘ saved] [● ai]  │
├────────────┬─────────────────────────────────────────────┬───────────────┤
│            │                                             │               │
│ LEFT SHELF │              CENTER ROOM                    │   INSPECTOR   │
│  240 px    │              fluid, max 72ch                │     360 px    │
│ (binder)   │              (TipTap editor)                │ (context)     │
│            │                                             │               │
│            │                                             │               │
│            │                                             │               │
├────────────┴─────────────────────────────────────────────┴───────────────┤
│ STATUS BAR · 24 px  [save] [words] [today] [snapshot] [ollama] [model]   │
└──────────────────────────────────────────────────────────────────────────┘
```

### B.1 Top bar (36 px)

Always visible, one row. **Left:** project title (clickable → Picker with confirm-if-dirty). **Center:** breadcrumb (Part › Chapter › Scene), click-to-jump, truncates middle. **Right:** save state pill, AI consent dot (warm grey off / accent on, never red), `⌘K` palette opener. **Not** in top bar: agent triggers, export, settings, model selector, search. Those live in palette, inspector, or shortcuts.

### B.2 Left shelf — Binder (240 px)

Visible by default. Collapses to 32 px rail showing chapter word-count percent (`Cmd/Ctrl+\`). Two sub-modes (segmented control): **Tree** (Parts → Chapters → Scenes with status dots and word rollups) and **Outline** (flat list with synopsis/POV/beat/target, inline editable). Right-click context menu: New / Rename / Duplicate / Delete (soft, with `Cmd+Z` undo). Drag-reorder is atomic (one transaction, one snapshot). No third sub-mode — search and filtering happen via the command palette.

### B.3 Center room — Editor

The only thing on screen by default is prose, centered, max 72ch (user-adjustable). No toolbar, no floating buttons, no status pills. Formatting is keyboard-first; a **bubble menu** on selection exposes bold, italic, headings, blockquote, list, link, comment. Markdown shortcuts (`**bold**`, `# h1`) work too. **No floating AI button.** Quick actions live behind `Cmd/Ctrl+K`. Focus mode (`Cmd/Ctrl+.`): hides shelf, inspector, top bar, status bar; typewriter scroll keeps the active line at ~40% from top; `Esc` exits.

### B.4 Right inspector — context room (360 px, hidden by default after first 5 minutes)

The inspector holds **one** of the following contexts at a time. Never two. Never more.

| Context | Trigger | What it shows |
|---------|---------|---------------|
| Outline metadata | Click a node in binder | Title, status, POV, beat, target words (this is the existing `InspectorPanel`) |
| Agents | `Cmd/Ctrl+J` | Run picker, live run, history |
| Proposal Review | An agent completes | Diff/accept/reject surface (§E) |
| Memory / Bible | `Cmd/Ctrl+B` | Entity cards, vocab, chapter memory |
| Validators | `Cmd/Ctrl+'` | Last run grouped by severity |
| Snapshots | `Cmd/Ctrl+Shift+H` | Timeline + diff vs current |
| Notes | `Cmd/Ctrl+'` (toggle in shelf header) | Free-form notes per scene |

The inspector is **closed by default after first launch** for a returning user. The editor expands to fill the room. The writer brings the inspector back with one keystroke. This is a deliberate departure from the current spec, which always shows the inspector.

**Empty inspector rule.** If no context is selected (the writer just hit `Cmd/Ctrl+\`, but no context), the inspector simply does not render. The editor takes the full width. The writer never sees an empty 360 px panel.

### B.5 Status bar (24 px)

- Always visible. Tabular numerals.
- Items, left to right: save state (icon + relative time), project word count, today's word count, last snapshot age, **Ollama dot + model tag**, **AI consent state**, focus-mode toggle.
- Clicking the Ollama dot opens the Models page modal. Clicking the model tag opens the per-agent override sheet. Clicking the AI consent dot is a no-op when on; when off it opens the consent prompt.

---

## C. The eight screens

### C.1 First-launch / Ollama Setup

**Purpose.** Get the writer from "no AI" to "AI ready" in under five minutes.

**Default state.** Four-step linear wizard. Step 1 carries the privacy promise: *"BooksForge is local-first. AI runs on your machine via Ollama. Nothing leaves your computer."*

**Steps.** (1) **Detect** — auto-probe; Continue or Install. (2) **Install** — Guided (pinned-hash installer) or Manual (copy URL). (3) **Pick a model** — three cards driven by RAM detection: *Recommended*, *Smaller / faster*, *Largest that fits*. Other models behind "See all" collapsed. (4) **Smoke test** — 12-token hello against the chosen model.

**Secondary.** *Continue without AI* (sets project AI off; banner in Workspace prompts later). *Show advanced* — host override, custom model tag.

**Errors.** Disk too low → size-ranked alternatives. Pull interrupted → resume. Model rejected → pick another. No stack traces.

**Microcopy.** Plain second-person. *"This downloads about 4.7 GB."* Never *"initializing inference engine."*

### C.2 Project Picker

**Purpose.** Open or start a book.

**Default state.** Recent projects (max 8 visible without scroll); buttons `+ New project` and `Open existing`. Footer privacy line: *"All projects live on your computer."*

**Primary.** Open recent, New, Open existing. **Secondary.** Right-click row → Pin / Show in Finder / Remove. Import (`Cmd/Ctrl+I`).

**Empty.** No recents → single-line *"Start your first book."* + `+ New project` button. No illustration. **Missing bundle** → row pill → `Locate` or `Remove`.

Project rows show only title, date, directory. **No word count.** Word count is irrelevant for opening.

### C.3 New-Project Wizard

**Purpose.** Create a project. Optionally seed it with an AI-generated outline.

**Steps.** Linear, no skipping. Breadcrumb: `Mode → Details → Idea → Confirm`. (1) Mode + template card. (2) Title, author, target words, save location. (3) **Idea (optional)** — textarea *"In a paragraph, what's this book?"* with `Skip — start blank` and `Outline this with AI` (disabled if Ollama down — replaced by `Set up AI first`). (4) Review & Create.

On `Outline this with AI`: step 3 transitions in place to a live agent run — Intake spinner → editable brief preview → Outline Architect spinner → editable outline tree → accept/regenerate.

**Errors.** Folder not writable → in-line error, suggest desktop. Agent fails after retries → surface raw output, offer Retry/Skip/Use partial. `not_a_book` flag → warn in line, allow proceed.

**Microcopy.** *"This stays on your machine."* under location. *"You can change all of this later."* under title.

### C.4 Workspace — the editor experience

**Purpose.** A calm room where the writer writes. Everything else is one keystroke away.

**Default state.** Top bar (project, save state). Binder open (240 px). Editor centered with current chapter mounted and cursor at the last edit. Inspector closed. Status bar. Total chrome ≤ 60 px; the rest is prose.

**The editor (TipTap, virtualised by chapter).** Body in `--font-prose` (Lora) at the user's size. Chapter title above prose, centered, larger of the same face. Scene breaks render as the template ornament (`❦` default). A thin **drift indicator** — a soft amber dot in the gutter — surfaces when typed text contains flagged AI-tells (`delve`, `tapestry`, `intricate`, etc.), dismissible on click; never modifies prose. Live word count lives in the status bar, not in the editor. OS-native spell check; grammar is off (post-MVP).

**Primary actions (keyboard or palette).** `Cmd/Ctrl+K` quick actions on selection (Sharpen / Continue / Rephrase / Shorten / Expand — single-shot prompts, not agents, with inline diff). `Cmd/Ctrl+J` Agents · `Cmd/Ctrl+B` Memory · `Cmd/Ctrl+'` Validators · `Cmd/Ctrl+P` palette · `Cmd/Ctrl+S` force save (autosave 5 s).

**Secondary.** Snapshot now, find/replace, focus mode, Markdown shortcuts.

**Bubble menu on selection.** Seven affordances — bold, italic, heading, blockquote, list, link, comment. **No AI in the bubble menu.** AI is the palette, not a hover.

**Empty / errors.** Empty chapter placeholder: *"Start writing. Cmd+K opens AI quick actions when you need them."* Save fails → non-blocking banner with retry. Ollama disconnects mid-edit → status dot warm grey, no modal; quick actions show *"AI is unavailable"* with re-probe link.

**Microcopy.** Save indicator says *"Saved 4s ago"* — not *"All changes saved automatically to the cloud"* — because it's the desktop and there is no cloud.

### C.5 AI Proposal Review — the heart of the trust model

**Purpose.** This is where the writer decides whether to accept what the machine wrote. It must feel like reviewing a colleague's red pen — clear, granular, reversible.

**Default state.** When an agent completes, the inspector switches to **Proposal Review** automatically (with a 200ms delay so the writer isn't yanked). A toast also appears in the editor: *"Copyedit ready — 14 changes."* with `Review` and `Dismiss` buttons. If the writer dismisses, the proposal sits in Agents → History, never lost.

**Layout.**

```
┌─ INSPECTOR (360 px) ────────────────────────────────┐
│ COPYEDITOR · Chapter 3                              │
│ 14 proposed edits · 2m ago · qwen2.5:7b             │
│ [VerificationReport: passed · 2 reviewers]          │
│─────────────────────────────────────────────────────│
│ Filter: [ all ] punctuation casing dashes spelling  │
│─────────────────────────────────────────────────────│
│ ▸ Edit 1 / 14 · punctuation                         │
│   "she said,"  →  "she said:"                       │
│   Rationale: dialog tag introduces a list.          │
│   [Accept] [Reject] [Skip]                          │
│─────────────────────────────────────────────────────│
│ ▸ Edit 2 / 14 · spacing                             │
│   ...                                                │
│─────────────────────────────────────────────────────│
│ Bulk:  [Accept all punctuation] [Reject all]        │
│ [Accept all] [Reject all] [Snapshot first ✓]        │
└─────────────────────────────────────────────────────┘
```

In the editor, each proposed change is also marked in the gutter as a colored bar. Clicking the gutter mark scrolls the inspector to that edit and vice versa. The selected edit highlights its before-text in the editor with a soft amber background.

**Three flows in detail.**

**(a) Copyedit on a single paragraph.** Writer hits `Cmd/Ctrl+K` → "Copyedit this paragraph". Live run streams; on completion the inspector shows N edits and gutter marks appear in the editor. Writer reviews each — `Y` accept and advance, `N` reject and advance, `Space` skip. **Pre-edit snapshot is taken once on the first accept** with a confirmation in the inspector header: *"Snapshot taken at 14:22 · Restorable from Snapshots."* Edits apply atomically per accept. Bulk-by-category accepts only that category. `Reject all` closes the proposal; raw output remains in `agent_runs/`.

**(b) Developmental review across many chapters.** Writer runs `DevelopmentalReview` on project (warned on >150k words; "this chapter" offered instead). Batch of per-chapter sub-runs streams (`Chapter 1 · done · 3 issues; Chapter 2 · running…`). Cancel-batch preserves completed sub-runs. The proposal is a **review report**, not edits: aggregate summary (≤120 words) + severity bars + issue list grouped by chapter. Each issue clicks through to editor location. Per-issue actions: `Convert to TODO` (writes a scene-level note tied to the issue location) · `Dismiss` · `Mark resolved`. **Dev-edit issues never auto-edit prose.** "Apply nothing" is the default outcome; the report is filed in History.

**(c) Continuity rename touching three scenes.** Writer runs `ContinuityCheck`; deterministic linter first, LLM adjudicator on ambiguous findings. Proposal groups findings by kind (`name_drift`, `pov_drift`, `timeline`). A `name_drift` rename shows scope (scene / chapter / project) and an expanded mention list — *"Affects 3 scenes, 7 mentions."* Per-mention checkboxes let writers skip a deliberate variant (a nickname); the Apply button counts checked items dynamically. Accept takes one pre-edit snapshot labeled `pre-continuity-rename · Aiden→Aidan` covering all three scenes, applies in document order, updates entity_memory aliases, queues a Memory Curator run. Toast: *"Renamed 7 mentions across 3 scenes. Restore from snapshot if needed."*

**Pre-edit snapshot reassurance.** Always in the proposal header: *"A snapshot will be taken before any change."* After accept: *"Snapshot taken — `2026-05-08 14:22 · pre-continuity-rename`."* Ships in every proposal.

**Microcopy.** Never *"AI suggests"* (passive). *"Copyeditor proposes"*, *"Continuity flagged"*, *"You haven't decided yet."* Verbs over nouns. Schema-invalid output → *"We couldn't parse this. Open raw output."* with artifact link. Cancellation → partial output preserved, no proposal applied.

### C.6 Memory / Bible

**Purpose.** The single source of truth for "what is true in this book": entities, vocabulary, chapter summaries.

**Default state.** Three sub-tabs (segmented control): **Entities** (default — card grid filterable by Characters / Locations / Items / Organisations / Themes), **Vocabulary** (pending proposals top with gold "n new" pill; accepted below grouped `prefer / avoid / replace`), **Chapter memory** (finalised chapters with summaries; click to expand with `Refresh memory`).

**Primary.** Edit card inline. Add entity. Click "Scenes (n)" → editor filters to those scenes.

**Secondary.** Run Memory Curator on demand. Promote a `prefer` entry. Merge entities. Soft-delete with snapshot.

**Empty.** No entities → two-line state + *"Auto-extract from manuscript"* (regex+capitalisation, not LLM). No chapter memories → *"Memory builds as you finalise chapters."* Card placeholders are concrete: *"What does she want?"*, *"What's at stake?"* — writer fills in human language.

### C.7 Validators / Pre-export QA

**Purpose.** Tell the writer what's wrong before they publish. Block on hard errors.

**Default state.** Three groups: **Errors (n)** · **Warnings (n)** · **Info (n)**, each collapsed unless n>0. `Run all` button.

**Primary.** Click an issue → editor scrolls, cursor lands at range. One-click fix where validator declares one. Suppress with typed reason (logged in `validator_suppressions`).

**Secondary.** Configure rules. Show resolved.

**Empty.** No run yet → *"Run all (Cmd+Shift+V)."* All passed → green check + *"Ready to export."*

**Microcopy.** Issue line: rule id + plain English + exact location. *"R-EPB-12 · Image at line 412 lacks alt text · Fix: open the image and add alt text"* — never a stack trace.

### C.8 Export — opinionated about KDP and Google Books

**Purpose.** Get the writer from manuscript to a publishable file with no fiddling. The export screen leads with the **destination**, not the format.

**Default state.** A modal (`Cmd/Ctrl+E`). Top of modal asks **"Where are you publishing?"** with three large cards:

| Card | Outputs |
|------|---------|
| **KDP Paperback** | Print-ready PDF (PDF/X-1a:2001, embedded fonts, bleed, gutter) |
| **KDP Kindle eBook** | EPUB-3 with KDP-friendly metadata + EPUBCheck pass |
| **Google Books** | EPUB-3 with Google's stricter metadata + EPUBCheck pass |

Plus a dim *"Other"* row that expands to: Generic EPUB-3, DOCX, Markdown, Trade PDF (5×8 / 6×9). These are for sharing-not-publishing; they bypass the platform pre-flight gate.

**Per-target details — see §H.** The card expands inline to show the few choices that matter:
- KDP Paperback: trim size (5×8, 5.25×8, 5.5×8.5, **6×9** default), front-matter included (yes/no), embedded fonts (read-only — "All fonts will be embedded ✓"), output path.
- KDP Kindle: ASIN (optional), language, output path.
- Google Books: ISBN (required), language, output path.

**Primary actions.** `Run pre-flight` → validators run inline with a progress strip. Errors block; warnings prompt; info silent. On green, `Export now` runs the pipeline, shows progress with elapsed time, ends with a `Reveal in Finder` button and a one-line provenance sentence: *"manuscript.epub · 4.1 MB · sha256 7f3a… · EPUBCheck v5.1.0: passed."*

**Secondary actions.** "Save profile" — remembers per-project per-target choices. "Open EPUBCheck report" — full XML view in a sub-modal. "Inspect package" — opens the unzipped EPUB in Finder/Explorer. Compare to last export (size, hash diff).

**Empty/error states.**
- No pre-flight yet: orange chip *"Pre-flight not run yet."*
- Pre-flight blocked: a clear list with click-to-fix; the `Export now` button is disabled with explanatory copy.
- EPUBCheck fail: surface the EPUBCheck JSON's first three errors in plain English (via the `epub-export-qa` agent in V1.0; in MVP, by message-ID lookup). One-click `Open in Validators inspector`.
- Pandoc missing for paperback PDF: clear install instructions, `Open Settings → Sidecars`.

**Microcopy.** Use platform words. *"KDP requires PDF/X-1a for paperbacks."* not *"PDF/X-1a:2001 compliance."* (Both are true; one talks to writers.) *"Your file is ready to upload to KDP."* on success.

---

## D. Agent UX

### D.1 The trigger surface

Agents are run from one of three places:

1. **Command palette** (`Cmd/Ctrl+P`) — the canonical surface. Searchable: `Copyedit chapter`, `Continuity check`, `Final review scene`. The palette shows the agent's scope, expected duration, and a context preview *before* running.
2. **Right-click in the binder** — for chapter / scene scope (`Copyedit this chapter`, `Refresh memory`).
3. **Agents inspector** (`Cmd/Ctrl+J`) — for batch jobs and scope-aware launches with the full Context Preview.

**No "run last agent" button hovering near the cursor.** Power users use the palette.

### D.2 Context Preview (mandatory before any agent run)

When the writer triggers an agent, a compact preview appears in the inspector:

- Scope (collapsed by default, expandable).
- Entity cards being sent (each can be deselected).
- Style book + tone.
- Token estimate vs. context window — a single bar with green/amber/red.
- Model selection (defaults from project; per-call override behind `Show advanced`).
- A `What this agent does` line in plain English.

`Run` starts; `Cancel` returns. **Sending equals previewing — there are no hidden additions.** This is the orchestrator's invariant.

### D.3 Live run experience

In the Agents inspector during a run:

- Workflow name, scope, and elapsed time at top.
- Step list with per-step status (`pending / running / completed / failed`).
- For the running step: a **streaming preview** showing the last ~500 chars of the model output. This is read-only and truncated; full output goes to the proposal.
- A single `Cancel` button that aborts within 1 second.
- A privacy chip at the bottom: *"Local — qwen2.5:7b · 127.0.0.1"*.

For batches, a sub-run progress list ("3 of 24 done · 2 running") with per-sub-run cancel.

### D.4 Run states & visual language

Five states, five colors (using the design system tokens):

| State | Color | Where |
|-------|-------|-------|
| `in flight` | `--color-info` (blue) with a calm pulse | Inspector header, status bar mini-dot |
| `awaiting review` | `--color-accent-500` (amber) | Inspector header, toast in editor |
| `applied` | `--color-success` (green) | Snapshot row, history entry |
| `rejected` | `--color-text-secondary` (neutral) | History entry |
| `proposal_invalid` | `--color-error` (red, but used sparingly) | Inspector with "Inspect raw" link |

The Active tab badge of the agents inspector mirrors the live state. Runs animate through the `--duration-agent` token (400ms). Reduced-motion drops the pulse to opacity-only.

### D.5 Cancellation

- Always reachable. `Esc` cancels the active run.
- Cancelling within the first 2 seconds aborts before any token is generated.
- After tokens, cancellation aborts the HTTP request and marks the run `cancelled`. Partial output is preserved as an artifact under `agent_runs/<run_id>/<task_id>.json`. The writer can inspect or discard.
- Cancellation never partially applies. Only fully-accepted proposals mutate the manuscript.

### D.6 Per-agent UX notes (deltas only)

- **Intake / Outline Architect** live inside the New-Project Wizard, plus a `Refine brief` / `Regenerate outline` palette entry.
- **Chapter Drafter** is opt-in per scene; its output lands in a working buffer the writer must merge.
- **Developmental Editor** outputs a *report*, not edits — issues become TODOs, never auto-prose.
- **Continuity** runs the deterministic linter first; LLM only adjudicates ambiguous findings.
- **Memory Curator** runs automatically on chapter finalise (proposal surfaces in inspector + toast — never a blocking modal).
- **Vocabulary Dictionary** runs after every 5 accepted edits; non-blocking pile-up in Memory → Vocabulary.
- **Final Review Editor** is reachable **only** from the palette, never the binder right-click. It is the last pass; the writer must opt in consciously. If `qwen3.6:latest` isn't pulled, the inspector shows a model-pinning warning before run.

---

## E. The proposal/diff surface — detailed

The proposal surface is the most important interaction in the app. It must:

1. Show the change clearly enough to decide in seconds.
2. Make pre-edit snapshots impossible to miss.
3. Make accept and reject equally easy.
4. Survive the writer changing their mind (snapshots, history).
5. Never partially apply.

### E.1 Inline diff (Copyedit, single paragraph)

```
Edit 4 of 14 · spacing                       [⌘↑ prev] [⌘↓ next]
"He turned. The doorway  was empty."     ← before (struck, soft red)
"He turned. The doorway was empty."      ← after  (added, soft green)
Rationale: Removed double space.
[ Accept (Y) ]   [ Reject (N) ]   [ Skip (Space) ]
```

Selection highlights in the editor and scrolls into view. Accepting takes a snapshot (only on first accept), then applies the edit, advances. "Bulk-accept by category" is always a separate action — never automatic.

### E.2 Structural review (Developmental)

The proposal surface is **a report, not a diff list**. Each issue: severity + category + diagnosis + optional advisory suggestion. Per-issue actions: `Convert to TODO` (writes a scene-level note + binder marginal mark) · `Mark resolved` · `Dismiss`. **No prose is changed by this surface.**

### E.3 Continuity rename (3 scenes)

A summary header (`"Aiden" → "Aidan" · Affects 3 scenes · 7 mentions`) plus an expandable per-scene mention list with 40-char context. Per-mention checkboxes; the Apply button counts checked items dynamically. Accept fires one transaction: snapshot → apply mentions in document order → update entity_memory aliases → queue Memory Curator → success toast. Pre-edit snapshot is mandatory and visible.

### E.4 Partial accepts

- Copyedit / Humanization: per-edit accept/reject; pending edits sit in the inspector until `Done` or `Run again`.
- Continuity rename: per-mention checkboxes; partial accept warns *"This will leave 2 mentions un-renamed."*
- Final Review Editor: change-by-change review with rationale; accept applies the whole revised paragraph atomically. The FRE is a holistic pass — sub-fragment accept would be incoherent. Revert from snapshots.

Proposals are kept in Agents → History indefinitely (read-only after edits diverge); applied edits link back via `agent_applied_edits`.

---

## F. Keyboard model — 22 shortcuts, ruthlessly chosen

### F.1 Global

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+N` | New project |
| `Cmd/Ctrl+O` | Open project |
| `Cmd/Ctrl+,` | Settings |
| `?` | Cheat sheet |
| `Cmd/Ctrl+Q` | Quit (macOS native; on Windows: Alt+F4) |

### F.2 Workspace navigation

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+P` | Command palette (search, jump, run) |
| `Cmd/Ctrl+\` | Toggle binder |
| `Cmd/Ctrl+]` | Toggle inspector |
| `Cmd/Ctrl+.` | Focus mode |
| `Cmd/Ctrl+B` | Memory inspector |
| `Cmd/Ctrl+J` | Agents inspector |
| `Cmd/Ctrl+'` | Validators inspector |

### F.3 Editing

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+S` | Force save |
| `Cmd/Ctrl+Z` / `Shift+Z` | Undo / redo |
| `Cmd/Ctrl+F` | Find |
| `Cmd/Ctrl+K` | Quick AI actions on selection |

### F.4 Agents & exports

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+E` | Export |
| `Cmd/Ctrl+Shift+H` | Snapshot now |
| `Cmd/Ctrl+Shift+V` | Run all validators |
| `Esc` | Cancel active agent run / exit focus mode / close modal |

### F.5 Proposal review

| Shortcut | Action |
|----------|--------|
| `Y` | Accept current proposal item |
| `N` | Reject current proposal item |
| `Space` | Skip |

That's 22. Anything else lives in the palette. The cheat sheet (`?`) renders this entire table grouped, with platform-aware modifier symbols.

---

## G. Privacy + status surface

The privacy state must be **calm, persistent, and verifiable**. Never alarming, never dismissable.

### G.1 Three privacy signals

1. **Ollama dot (status bar).** Warm grey = not running. Solid success = running. Solid pulsing = generating. **Never red** — Ollama-down is a state, not an error.
2. **Model tag (status bar).** Plain text: `qwen2.5:7b`. Click → per-agent model sheet. The only place the model name lives in the chrome.
3. **AI consent dot (top bar).** Warm grey off / amber on. Off-click → one-time consent prompt per project.

### G.2 The "nothing has left this device" line

A live line in Settings → Privacy, driven by actual network counters: *"This device · No outbound network in this session except: Ollama loopback (127.0.0.1:11434) · Update check (last: 12 minutes ago)."* A subtler version appears in the Project Picker footer, prominently in Ollama Setup step 1, and next to the AI button in the New-Project Wizard.

### G.3 What we never do

Toast "AI is processing locally" (patronising). Cookie banner consent. Sound effects. Countdowns.

---

## H. KDP & Google Books readiness — ticket-shaped

This is the most opinionated part of the document because the brief asks for it. Each target produces files the writer can upload **without further fiddling**.

### H.1 KDP Paperback

**Output:** PDF/X-1a:2001 compliant interior, embedded fonts.

**Writer supplies:** trim size (`5×8`, `5.25×8`, `5.5×8.5`, `6×9` default), bleed yes/no (default no), front-matter toggles (title, copyright, dedication, ToC depth 1, acks), ISBN (optional).

**We compute (writer cannot fiddle):** margins per KDP table — outer 0.5″, inner 0.875″ gutter for >150 pages else 0.625″, top/bottom 0.75″; +0.125″ all sides on bleed; ALL fonts subset-embedded (refuse if missing); page number outer corner + chapter title running header (suppressed on chapter starts); chapter on recto with auto blank verso; ICC US Web Coated (SWOP) v2; no transparency, layers, or JS.

**Pipeline.** TipTap → canonical HTML → Pandoc with pinned LaTeX template → xelatex → PDF. PDF/X-1a final conversion is **manual via Acrobat in MVP** (Ghostscript sidecar in V1.0). The export shows: *"PDF/X-1a conversion is manual in MVP. Open the exported PDF in Acrobat → Print Production → Convert to PDF/X-1a."*

**Pre-flight blockers.** `R-EXP-FNT-01` fonts embeddable · `R-EXP-IMG-01` images ≥300 DPI (warn 200–299, block <200) · `R-EXP-TRM-01` trim matches · `R-EXP-PGN-01` page count within KDP min/max · `R-EXP-BLD-01` bleed consistent.

### H.2 KDP Kindle eBook

**Output:** EPUB-3 with KDP-friendly metadata, EPUBCheck-passing.

**Writer supplies:** title, subtitle, author, language, optional series + number, description (≤4000 chars), optional ASIN, cover (≥2560×1600, 1.6:1).

**We compute:** `<dc:identifier>` = `urn:bf:project:<ULID>` if no ASIN/ISBN; `dcterms:modified` from manifest; `rendition:layout=reflowable`; ToC depth 2 (KDP allows 3); page-list omitted (not required for reflowable fiction); KDP-safe CSS — no fixed widths, `position:fixed`, JS, or `<iframe>`.

**Pre-flight blockers.** `R-EPB-CHK-01` EPUBCheck (errors block, warnings prompt) · `R-EPB-COV-01` cover dimensions · `R-EPB-TOC-01` ToC depth ≤3 · `R-EPB-IMG-01` all images have `alt` · `R-EPB-FNT-01` embedded fonts have valid Adobe Embed Permission Bit.

**Success copy.** *"manuscript.epub is ready for KDP upload. Cover, metadata, and EPUBCheck passed."*

### H.3 Google Books EPUB

**Differences from KDP Kindle.** ISBN is **required** (Export disabled until filled); ToC depth ≤3 enforced; figure images need `<figcaption>` in addition to `alt`; page-list recommended when a print edition exists (warn if absent); `<dc:identifier>` must be `urn:isbn:NNN-NNNNNNNNNN`; `dcterms:modified` required.

**Pre-flight blockers.** All Kindle blockers, plus `R-EPB-ISBN-01` ISBN-13 in `urn:isbn:` form · `R-EPB-IMG-02` figure captions.

### H.4 The pre-flight gate

When the writer clicks `Run pre-flight`:

1. Validators run in parallel (with a single progress strip) for the selected target.
2. Errors block; the `Export now` button is disabled with a specific reason.
3. Each error has `Fix` (when validator declares) or `Open in Validators inspector` (always).
4. Warnings appear in line with `Continue anyway` and `Fix`.
5. Info silent.

**Override.** A `Force export` toggle exists behind `Show advanced` (per-project setting). It is NOT exposed by default. It writes a `validator_override` row with the writer's typed reason. We do not silently let writers bypass. This is the privacy-loud-equivalent for export integrity.

### H.5 Tickets implied

`[MZ-EXP] Platform-target picker (KDP Paperback / KDP Kindle / Google Books / Other)` · `KDP Paperback per-trim margin/gutter/bleed rules` · `KDP Kindle metadata defaults + EPUBCheck wiring` · `Google Books ISBN-required gate + figure caption validator` · `Pre-flight gate: parallel run, block-on-error` · `Validator rules R-EXP-FNT-01, R-EXP-IMG-01, R-EXP-TRM-01, R-EXP-PGN-01, R-EPB-CHK-01, R-EPB-COV-01, R-EPB-TOC-01, R-EPB-IMG-01/02, R-EPB-ISBN-01, R-EPB-FNT-01` · `PDF/X-1a fallback warning + Acrobat link.`

---

## I. Empty states, error states, and recovery

Every error has a path forward. Stack traces never reach the writer.

| Failure | UX |
|---|---|
| Ollama not running | Status dot warm grey; agent palette items disabled with hover *"Start Ollama (Cmd+,)"* |
| Model not pulled | Inspector: *"This agent prefers qwen2.5:7b. Pull now? (4.7 GB)"* with one button |
| Agent fails after retries | `proposal_invalid` artifact preserved; inspector offers `Inspect` and `Retry` |
| EPUBCheck fails | Export modal lists errors with plain-English remediation; one-click jump to Validators |
| Pandoc missing | Export shows install instructions; never silently skip |
| Disk full | Non-blocking banner; in-memory state retained; `Retry` / `Save elsewhere` |
| Bundle missing on launch | Picker row pill → `Locate` or `Remove from list` |
| Migration fails | Modal: *"A backup snapshot was made at \<path\>. Restore from there if anything's wrong."* — never silent |
| Snapshot restore | Restore creates a pre-restore snapshot first; explicit confirm with label + timestamp |
| Cancel mid-run | Within 1s; partial outputs preserved; toast *"Cancelled. Saved to history."* |

**Empty state grammar.** All empty states use the same shape: small icon, one-line headline, two-line body, one primary CTA. No illustrations. No empty-state heroes. No "Pro tip:".

---

## J. Reconciliation with `outputs/UI_UX_SPEC.md`

| Existing | Decision | Reason |
|---|---|---|
| Three-pane workspace | Keep structure; default inspector-closed for returning users | Minimal-but-not-empty |
| Right-panel six persistent tabs | Collapse into context switching via shortcuts | Tabs imply persistence; we want summoning |
| Format-first Export dialog | Replace with platform-target-first (KDP/Google/Other) | Brief asks for one-click KDP-ready |
| Quick-action bar (`Cmd+K`) | Keep, specify five actions + inline diff trust model | Was underspecified |
| Bubble menu in editor | Add explicit bubble menu | Editor lacked a discoverable formatting affordance for non-Markdown writers |
| Agents inspector run picker | Replace with command palette as canonical surface | Single search beats nested menus |
| Final Review Editor | Add to UX (was in AGENTS.md, missing from UI spec) | Completes the publish flow |
| Per-edit `Y/N/Space` shortcuts | Add | Highest-frequency interaction |
| Focus mode (`Cmd+.`) | Add explicit shortcut + behavior | Was implied not specified |
| Drift indicator (AI-tell gutter dots) | Add | Connects Voice Fingerprint to a passive UI affordance |
| Cheat sheet (`?`) | Add | Keyboardable principle requires discoverability |
| Per-agent model override | Hide behind `Show advanced` everywhere | "No model-fiddling theater" |
| AI consent dot in top bar | Add | Privacy-loud; was Settings-only |
| Validator pre-export gate | Keep; specify parallel run + per-platform rule sets | Was generic |

Net effect: same surfaces, fewer ambient affordances, sharper opinion on platform targets and the proposal interaction.

---

## K. What we will explicitly NOT build for v1

- **Find-and-replace inside the right inspector.** Editor `Cmd+F` is enough.
- **Multi-window or tab-per-project.** One window. The spec already excludes this; we reaffirm.
- **A built-in dictionary / thesaurus UI.** OS-native dictionaries are fine.
- **Inline AI suggestions while typing (autocomplete).** No ghost text. Ever.
- **Image generation.** Not even a button. Cover image is user-supplied.
- **Theme builder / custom design system.** System / light / dark / high-contrast only.
- **Rich onboarding tour.** A small `Cmd+P` palette demo is plenty. No carousel, no checklist.
- **Per-agent system prompt editing in UI.** Prompt versions are code-pinned; users cannot edit prompts.
- **Drag-and-drop into the editor from external apps.** Copy-paste-as-Markdown is the path.
- **Comments / suggestions Word-style.** TipTap supports comments; we wire only the comment anchor in MVP, not a comment thread UI. V1.0.
- **Streaming preview of an agent's output into the editor in place.** All agent output goes through the proposal surface.
- **Plugin panels.** Post-MVP, gated by capability tokens.
- **Mobile / web companion.** Out of scope.
- **A first-run welcome video or hero screen.** First run goes straight to Ollama Setup.
- **PDF/X-1a auto-conversion in MVP.** We ship the warning + Acrobat instructions; auto-conversion is V1.0.

---

## L. Open questions for the team

1. **Inspector default.** Inspector-closed (proposal) vs. remember-last-state for returning users? Affects minimalism vs. "where did my notes go?"
2. **KDP Paperback output.** PDF only, or also DOCX for IngramSpark-style services in "Other"?
3. **PDF/X-1a in MVP.** Ship the manual-Acrobat warning (proposal), or block KDP Paperback export until V1.0 bundles Ghostscript?
4. **Quick-action diff surface.** Same proposal surface as agent runs (proposal — consistency), or a lighter inline pop-up (less friction)?
5. **Drift indicator.** Does the AI-tell gutter dot feel surveillance-y, even though detection is purely local regex?
6. **Final Review Editor model pin.** Soft-fall-back when `qwen3.6` isn't pulled with `low_confidence` warning (proposal), or hard-block with "you need a larger model"?
7. **Memory Curator on chapter finalise gate.** Modal (interrupts), toast (easy to miss), or inspector-switch + toast (proposal)?
8. **Continuity rename default scope.** Project is usually right but is the most destructive — should we default to project, or chapter?
9. **Snapshots top-bar pill.** Promote snapshots into the top bar ("12 snapshots, last 22m ago") to reinforce trust, or keep it inspector-only?
10. **Telemetry preview pane.** Surface what *would* be sent if telemetry were enabled, so users can verify before opting in?

---

**End of proposal.**
