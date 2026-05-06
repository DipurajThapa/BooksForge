# Functional Specifications Document — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 0. How this document is structured

Each functional area is presented as a numbered module. Inside each module you will find: a short description; user stories with acceptance criteria; an ID-tagged requirements table where every row is testable; explicit non-requirements; phase mapping. Requirement IDs are stable across versions (`FR-PROJ-001` etc.); deletions become tombstones, not renumbers. This stability is what makes the FSD usable as a regression suite.

Phases are: **M** (MVP), **1.0**, **1.5**, **2.0**. Phase tags appear in each requirement.

## 1. Project & manuscript management

### 1.1 Description

The project module is the root of every other feature. A *project* is a folder-based bundle (`MyBook.booksforge/`) containing a SQLite database, a manuscript directory, an assets directory, a snapshots directory, and a manifest. Users create, open, import, export, archive, and delete projects through this module. The folder format is a deliberate choice — see TAD §8 and Data Model §2 — so that projects are git-friendly, partially recoverable on corruption, and inspectable on disk by power users.

### 1.2 User stories

**US-PROJ-01.** As Anya, I create a new project from a "Romance Novel" template and immediately land on a blank scene with a series-bible sidebar already populated with empty character cards. **Acceptance:** project visible in recent-projects list within 2 seconds of creation; manifest pinned to template version; all template scaffolding written atomically (no half-created project on crash).

**US-PROJ-02.** As Theo, I open my publisher's manuscript-format DOCX and BooksForge imports it into a structured project preserving headings, tracked changes, comments, and footnotes. **Acceptance:** round-trip diff (export back to DOCX, compare with original via `pandoc-diff`) shows zero structural loss for a 50k-word document; tracked changes preserved with author attribution; image fidelity preserved.

**US-PROJ-03.** As Aisha, I move a project from my Linux laptop to my Mac by copying the `.booksforge` folder. **Acceptance:** project opens with no migration prompts; all paths resolve; assets render; SQLite WAL handled cleanly across filesystem boundaries.

**US-PROJ-04.** As any user, I never lose more than 5 seconds of work to a crash. **Acceptance:** autosave fires at most every 5 seconds when the document is dirty; crash recovery offers the unsaved buffer on next launch; recovery test: kill -9 the process mid-edit, relaunch, recover within one prompt.

### 1.3 Functional requirements

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-PROJ-001 | Create a new project from a named template; project is a directory bundle (`*.booksforge`) | M | MUST |
| FR-PROJ-002 | Open an existing project by selecting the bundle directory or its top-level manifest | M | MUST |
| FR-PROJ-003 | Recent-projects list with pinning, last-opened timestamp, and "missing" state when bundle moved | M | MUST |
| FR-PROJ-004 | Atomic project creation (temp dir + rename) so no partial bundles on crash | M | MUST |
| FR-PROJ-005 | Project metadata: title, subtitle, authors, ISBN, series, language, genre, target word count | M | MUST |
| FR-PROJ-006 | Multi-author metadata with role (author / co-author / contributor / editor / translator) | 1.0 | SHOULD |
| FR-PROJ-007 | Import from DOCX with structure preservation (headings, tracked changes, footnotes, images) | 1.0 | MUST |
| FR-PROJ-008 | Import from Markdown / plain text with optional structure inference | M | SHOULD |
| FR-PROJ-009 | Import from Scrivener (`.scriv`) preserving binder hierarchy and snapshots | 1.5 | SHOULD |
| FR-PROJ-010 | Import from EPUB (re-importing your own published book for a 2nd edition) | 1.0 | MAY |
| FR-PROJ-011 | Archive project (zip and move out of recent list) and restore from archive | 1.0 | SHOULD |
| FR-PROJ-012 | Encrypted-at-rest project (AES-256-GCM, password-derived key via Argon2id) | 1.0 | MUST |
| FR-PROJ-013 | Autosave with configurable interval (default 5s after last keystroke) and configurable max-loss budget | M | MUST |
| FR-PROJ-014 | Crash recovery: replay autosave-WAL into the project on next launch with explicit user confirmation | M | MUST |
| FR-PROJ-015 | Project-level word count target with daily progress tracking | M | SHOULD |
| FR-PROJ-016 | Project locking (advisory) — warn if same bundle opened by another instance | M | MUST |
| FR-PROJ-017 | "Self-contained" export: bundle into a single `.booksforge.zip` for transport | M | SHOULD |

### 1.4 Non-requirements

We do **not** support multi-project workspaces in V1.0 (one project per window). We do **not** auto-import from a vendor cloud (Dropbox/iCloud) — users may put projects in any folder including a synced one but BooksForge does not orchestrate sync. We do **not** offer collaborative project ownership in V1.0 (single owner per project).

## 2. Editor and document tree

### 2.1 Description

The editor is where 90% of user time is spent. It must be **fast**, **reliable**, and **boring** — innovation in the editor is a liability. We use **TipTap** (ProseMirror-based) headless framework with a custom UI **[DECISION-002]**. The document is modelled as a tree of typed nodes (Part → Chapter → Scene → Block) persisted in SQLite plus content stored as ProseMirror JSON. The left sidebar shows a binder/outline; the right sidebar is a context panel (notes, AI, validators, characters, etc.) that swaps purpose by tab.

### 2.2 User stories

**US-EDIT-01.** As Anya, I drag a scene from chapter 3 to chapter 5 and the scene moves with all its notes, characters-in-scene tags, and AI history intact. **Acceptance:** drag completes in <100 ms perceived; references update; undo restores original position.

**US-EDIT-02.** As Theo, I see tracked changes from my editor as inline marks I can accept or reject one at a time or in bulk. **Acceptance:** matches Word's behaviour for accept/reject all; per-change attribution shown on hover; round-trip preserves all rejected changes for re-export.

**US-EDIT-03.** As Aisha, I insert a footnote with a CSL citation that auto-formats per the project's chosen style (Chicago author-date). **Acceptance:** citation key autocomplete from imported BibTeX; footnote numbers re-flow on insert/delete; export preserves CSL fidelity.

**US-EDIT-04.** As any user, my 200,000-word manuscript opens to my last cursor position in under 1.5 seconds on a 2024-class laptop. **Acceptance:** measured cold-open p50 ≤1.5s, p95 ≤3.0s on reference hardware (defined in 11-Test §4).

### 2.3 Functional requirements

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-EDIT-001 | Rich text editing: bold, italic, underline, strikethrough, sub/superscript, code, highlight | M | MUST |
| FR-EDIT-002 | Block types: paragraph, H1–H6, blockquote, code block, list (bulleted/numbered/checklist), figure, equation | M | MUST |
| FR-EDIT-003 | Inline elements: link, footnote, endnote, citation, cross-reference, comment anchor | M | MUST |
| FR-EDIT-004 | Document tree: Project → Part → Chapter → Scene → Block; each node has a stable ID | M | MUST |
| FR-EDIT-005 | Drag-reorder of nodes in the tree with reference integrity | M | MUST |
| FR-EDIT-006 | Find / replace with regex, scope (selection / scene / project), case-aware replace | M | MUST |
| FR-EDIT-007 | Multi-cursor and column selection | 1.0 | SHOULD |
| FR-EDIT-008 | Tracked changes (insert / delete / format) with author attribution | 1.0 | MUST |
| FR-EDIT-009 | Inline comments anchored to a range, threadable | 1.0 | MUST |
| FR-EDIT-010 | Footnotes / endnotes with auto-numbering and re-flow | M | MUST |
| FR-EDIT-011 | Citation insert with CSL style preview | 1.0 | MUST |
| FR-EDIT-012 | Cross-references (figure, equation, section) that survive renumbering | 1.0 | MUST |
| FR-EDIT-013 | Math: inline `$...$` and display LaTeX, rendered via KaTeX | 1.0 | SHOULD |
| FR-EDIT-014 | Code blocks with language hint and syntax highlighting (technical writing) | M | SHOULD |
| FR-EDIT-015 | Image / figure with caption, alt text, target width, target placement hint | M | MUST |
| FR-EDIT-016 | Tables with header rows, alignment, merged cells | 1.0 | MUST |
| FR-EDIT-017 | Distraction-free / typewriter / focus modes | M | SHOULD |
| FR-EDIT-018 | Spell-check using OS or Hunspell with project-level custom dictionary | M | MUST |
| FR-EDIT-019 | Word count: project, chapter, scene, selection, today's session, by author (for tracked-changes) | M | MUST |
| FR-EDIT-020 | Markdown shortcuts (`# ` → H1, `> ` → blockquote) and full Markdown paste | M | SHOULD |
| FR-EDIT-021 | Paste from Word/Pages/Google Docs preserving meaningful formatting only | M | MUST |
| FR-EDIT-022 | Snapshots — manual point-in-time captures of any node, browseable and restorable | 1.0 | MUST |

### 2.4 Editor performance requirements

The editor is bound by ProseMirror's performance envelope. We require: opening a 200k-word manuscript in ≤1.5 s (p50) on reference hardware; typing latency p95 ≤30 ms; scroll FPS ≥55 fps on a 144Hz display when scrolling through a 50k-word chapter. Achieving this requires virtualised rendering of the document (don't mount all blocks); see TAD §6.4.

## 3. Templates and formatting engine

### 3.1 Description

Templates are how we serve three personas with one product. A *template* is a versioned bundle of: a project skeleton (parts/chapters/scenes scaffolding), a set of style rules (typography, spacing, page setup), validator hints (which validators apply), and AI prompt overrides. Templates ship in the app and as plugins. The formatting engine is rule-based and produces the source for the export pipeline (§9). It is **not** WYSIWYG-only — the user can override at any level, but defaults always come from the template.

### 3.2 User stories

**US-TPL-01.** As Anya, I switch my project from "Trade Paperback (5×8)" to "Mass Market (4.25×6.87)" and the entire formatting reflows correctly with no manual tweaking. **Acceptance:** export renders correctly under both; no broken images or runaway captions; user-level overrides are preserved.

**US-TPL-02.** As Aisha, I apply the "Cambridge University Press monograph" template-pack and my export now satisfies the press's submitted-format checklist. **Acceptance:** every item on the press checklist (margins, font, footnote style, heading hierarchy, copyright placement) is correct in the exported DOCX.

### 3.3 Functional requirements

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-TPL-001 | Template manifest format (TOML) with versioning, name, description, applies-to (mode), required validators | M | MUST |
| FR-TPL-002 | Style rules expressed as a typed declarative document (not raw CSS) compiled per export target | M | MUST |
| FR-TPL-003 | Built-in templates: Generic Novel, Romance, Mystery, Sci-Fi/Fantasy, Memoir, Business Non-Fiction, Cookbook, Textbook, Academic Monograph | 1.0 | MUST (M=Generic+Romance+Sci-Fi) |
| FR-TPL-004 | Page-size profiles: A5, B5, US Trade 5×8, 5.5×8.5, 6×9, Mass Market, US Letter, A4, custom | M | MUST |
| FR-TPL-005 | Typography presets: serif/sans pairs with documented fallbacks, license-clear fonts only | M | MUST |
| FR-TPL-006 | Per-element style overrides (chapter title, scene break, drop cap, ornament) | M | SHOULD |
| FR-TPL-007 | Front-matter and back-matter scaffolding (title page, copyright, dedication, also-by, about-the-author, acknowledgements) | M | MUST |
| FR-TPL-008 | Template hot-swap with diff preview before apply | 1.0 | SHOULD |
| FR-TPL-009 | Export-target-specific overrides (e.g., this template renders differently for KDP vs IngramSpark) | 1.0 | MUST |
| FR-TPL-010 | Template signing (cryptographic) for marketplace plugin templates | 1.5 | MUST |

## 4. AI assistance

See `08-ai-integration.md` for the integration architecture. This section covers the *user-visible* AI features.

### 4.1 User stories

**US-AI-01.** As Anya, I select a paragraph that feels weak, hit Cmd+K, choose "Sharpen prose", and a side panel offers three rewrites with explanations. The model runs entirely on my laptop. **Acceptance:** for a 200-word paragraph, first suggestion arrives in ≤6 s on a 16 GB M1 with a 7B Q4 model; suggestions are typed locally; nothing leaves the device; user can accept, reject, or regenerate.

**US-AI-02.** As Theo, I ask the AI to "summarise chapter 3 as a 200-word back-cover blurb" and it delivers. The summary is auditable — I can see exactly what context it used. **Acceptance:** context window is shown ("used: 4,200 tokens of chapter 3, character bible row 'Theo Voss', tone preset 'literary'"); audit log entry created.

**US-AI-03.** As any user, I have AI off by default and turn it on per project. **Acceptance:** new project's AI capability is disabled until explicitly enabled with a one-time consent prompt; consent recorded in project manifest.

### 4.2 Functional requirements

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-AI-001 | Embedded local LLM runtime (llama.cpp) with model auto-download from a curated, hash-pinned catalogue | M | MUST |
| FR-AI-002 | External Ollama support — detect a running Ollama on `localhost:11434` and offer its models | M | SHOULD |
| FR-AI-003 | Cloud LLM support (Anthropic, OpenAI, OpenRouter) gated behind Studio tier or BYO API key | 1.5 | SHOULD |
| FR-AI-004 | AI capability is disabled per-project by default; explicit enable required | M | MUST |
| FR-AI-005 | AI menu presets: Sharpen, Shorten, Expand, Continue, Rewrite-tone, Rephrase, Summarise, Brainstorm, Critique, Beta-read | M | MUST |
| FR-AI-006 | Custom prompt presets per project / per template / per plugin | 1.0 | MUST |
| FR-AI-007 | Inline diff view of suggestion vs original; accept partial / accept all / reject | M | MUST |
| FR-AI-008 | Context selection UI — user can see and edit which scenes / character cards / notes are sent into the prompt | M | MUST |
| FR-AI-009 | Audit log: every AI call records prompt template, context size, model, timing; viewable per project | M | MUST |
| FR-AI-010 | Cost estimate before sending a cloud request (token estimate × model price) | 1.5 | MUST |
| FR-AI-011 | Rate limit / safety: cloud requests throttled per minute; long context warned | 1.5 | MUST |
| FR-AI-012 | Prompt-injection mitigations: untrusted content (imported docs, web research) is fenced with explicit markers in prompts; see 08-AI §6 | 1.0 | MUST |
| FR-AI-013 | Beta-reader mode — runs a long-form critique against a chapter; output is a structured report | 1.0 | SHOULD |
| FR-AI-014 | Series consistency check — flag character/place name spelling drift across the project | 1.0 | SHOULD |
| FR-AI-015 | AI generation can be cancelled at any time; partial output is preserved or discarded per user choice | M | MUST |

### 4.3 Non-requirements

We do **not** auto-apply AI suggestions. We do **not** train models on user content under any circumstance. We do **not** generate cover-art images in V1.x. We do **not** generate audio in V1.x. We do **not** "agentically" edit the manuscript without per-step user approval.

## 5. Validators (publishing readiness)

### 5.1 Description

A validator is a deterministic rule that inspects the project and reports issues classified as Error / Warning / Info. Validators run on demand and on pre-export. The validator engine is a pure-function pipeline: project state in, report out. New validators ship as plugins. Validators must explain *why* and *how to fix*, not just flag.

### 5.2 Validator categories

**Manuscript** — heading hierarchy gaps, orphaned paragraphs, broken cross-references, missing alt text, oversized images, unmatched quote marks, double spaces, em-dash inconsistencies. **Series & character** — character-name spelling drift, location-name drift, point-of-view discipline within a scene, tense drift. **Genre** — romance "Black Moment present?", mystery "every clue introduced before red herring?", thriller "ticking-clock established by chapter X". **Publishing platform** — KDP file-size limits, IngramSpark cover dimensions, Apple Books EPUB validation, font embedding licences, ISBN format, copyright placement, ToC depth limits. **Accessibility** — alt text completeness, language tagging, semantic heading structure for EPUB-3.

### 5.3 Functional requirements

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-VAL-001 | Validator API — pure function `(project) → Issue[]`, declared category and severity | M | MUST |
| FR-VAL-002 | Built-in manuscript validators (≥20) | M | MUST |
| FR-VAL-003 | Built-in KDP and IngramSpark profile validators | M | MUST |
| FR-VAL-004 | Built-in EPUB-3 validator (epubcheck embedded) | M | MUST |
| FR-VAL-005 | Built-in accessibility validator (alt text, language, headings) | 1.0 | MUST |
| FR-VAL-006 | Genre / series validators (per template) | 1.0 | SHOULD |
| FR-VAL-007 | Pre-export gate: warns or blocks based on user setting | M | MUST |
| FR-VAL-008 | Validator results panel with click-to-jump-to-source and one-click fix where deterministic | M | MUST |
| FR-VAL-009 | Validator marketplace ingestion (plugin packs) | 1.5 | MUST |
| FR-VAL-010 | Performance: full-project validation of a 100k-word project completes in ≤10 s on reference hardware | M | MUST |

## 6. Series bible / structured project metadata

### 6.1 Description

A *series bible* is the structured-data sidecar to the manuscript: characters, locations, items, magic systems, organisations, timelines, themes. Each entity has a card with fields, free-form notes, and links to the scenes where it appears. The bible auto-suggests entries by extracting capitalised entities from the manuscript (with confirmation) and tags scenes with which entities appear. This module is what enables the series-consistency validators (§5).

### 6.2 Functional requirements

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-BIBLE-001 | Entity types: Character, Location, Item, Organisation, Theme, Custom | M | MUST |
| FR-BIBLE-002 | Each entity has structured fields per type (e.g., Character: aliases, age, role, voice) and a notes field | M | MUST |
| FR-BIBLE-003 | Entity → Scene link with auto-suggestion from text mentions | M | SHOULD |
| FR-BIBLE-004 | Custom entity types via plugins | 1.5 | SHOULD |
| FR-BIBLE-005 | Timeline view (entities × scenes × in-story date) | 1.0 | SHOULD |
| FR-BIBLE-006 | Relationship graph (character-to-character, with edge labels) | 1.0 | MAY |
| FR-BIBLE-007 | Bible export to CSV / JSON for external tools | M | SHOULD |

## 7. Outline and corkboard

A binder/outline view lists every scene with synopsis, status (planned/draft/revised/final), POV, scene goal, conflict, outcome. A corkboard view shows scenes as draggable cards. Both views read/write the same underlying scene metadata. (Familiar to Scrivener users.)

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-OUTLINE-001 | Scene cards with title, synopsis, status, POV, beat, target word count | M | MUST |
| FR-OUTLINE-002 | Corkboard with drag-reorder reflected in document tree | M | SHOULD |
| FR-OUTLINE-003 | Outliner view (hierarchical list) with collapse/expand | M | MUST |
| FR-OUTLINE-004 | Status colour-coding and per-status word-count rollups | M | SHOULD |
| FR-OUTLINE-005 | Story-structure overlays (Save the Cat beats, Three-Act, Hero's Journey) — apply a beat-sheet template and tag scenes against beats | 1.0 | SHOULD |

## 8. Snapshot history & version control

Snapshots are point-in-time captures of project state at any granularity (whole project, chapter, scene). They are content-addressed in `snapshots/` inside the bundle and indexed by SQLite. Snapshots are independent of the user's git workflow but the bundle itself is git-friendly.

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-SNAP-001 | Manual snapshot of any node (or whole project) with a label | 1.0 | MUST |
| FR-SNAP-002 | Auto-snapshot on schedule (default: hourly during active sessions) | 1.0 | MUST |
| FR-SNAP-003 | Pre-AI-edit auto-snapshot (any AI-applied edit creates a snapshot of the affected node first) | M | MUST |
| FR-SNAP-004 | Browse snapshots with diff vs current; restore whole or selective | 1.0 | MUST |
| FR-SNAP-005 | Snapshot retention policy: keep all manual + last 30 auto + monthly archives | 1.0 | SHOULD |
| FR-SNAP-006 | Storage-efficient delta (content-addressed dedupe; only changed nodes stored) | 1.0 | MUST |

## 9. Export pipeline (user-facing)

See `09-export-pipeline.md` for engine details. User-facing requirements:

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-EXP-001 | Export profiles: Manuscript-DOCX (industry standard), Trade-PDF, EPUB-3, Mobi-via-EPUB, LaTeX, Markdown bundle, HTML | M | MUST (LaTeX=1.0) |
| FR-EXP-002 | Store-targeted profiles: KDP-Print, KDP-eBook, IngramSpark-Print, Apple-Books-EPUB, Kobo-EPUB, Google-Play-EPUB | M | MUST (subset; full=1.0) |
| FR-EXP-003 | Export pre-flight runs validators and shows a go/no-go summary | M | MUST |
| FR-EXP-004 | Export completes in ≤30 s for a 100k-word EPUB-3 with images on reference hardware | M | MUST |
| FR-EXP-005 | Export pipeline is reproducible — same input + same template + same engine version = byte-identical output | M | MUST |
| FR-EXP-006 | Export log preserved per export with every parameter and validator result | M | MUST |
| FR-EXP-007 | Custom export profile editor (advanced users) | 1.5 | SHOULD |

## 10. Plugin system (user-facing)

See `07-plugin-architecture.md` for technical model. User-facing requirements:

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-PLUG-001 | Plugin install from local file (sideload) and from marketplace | 1.0 (sideload), 1.5 (mkt) | MUST |
| FR-PLUG-002 | Capability prompt on install — user sees and approves what plugin can read/write/network | 1.0 | MUST |
| FR-PLUG-003 | Plugin types: template, validator, AI prompt pack, importer, exporter, UI panel | 1.0 (template/validator/prompt), 1.5 (importer/exporter/UI) | MUST |
| FR-PLUG-004 | Per-project plugin enable/disable | 1.0 | MUST |
| FR-PLUG-005 | Plugin updates with delta and rollback | 1.5 | MUST |
| FR-PLUG-006 | Plugin developer mode (load from folder, hot reload) | 1.0 | SHOULD |
| FR-PLUG-007 | Marketplace browse, search, install, ratings, paid plugin checkout | 1.5 | MUST |

## 11. Settings, accounts, licensing, sync

### 11.1 Settings

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-SET-001 | Per-app settings (theme, font, autosave interval, AI on/off) and per-project settings | M | MUST |
| FR-SET-002 | Settings exportable / importable to share machine setup | 1.0 | SHOULD |
| FR-SET-003 | Telemetry is **off by default**; opt-in toggle with a clear "what is sent" panel | M | MUST |
| FR-SET-004 | Crash reports opt-in separately from telemetry | M | MUST |

### 11.2 Account & licensing

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-LIC-001 | License activation by key; offline activation supported (signed token) | M | MUST |
| FR-LIC-002 | Free tier requires no account | M | MUST |
| FR-LIC-003 | Pro / Studio account uses email + magic link or password (no third-party SSO required) | 1.0 | MUST |
| FR-LIC-004 | License is per-user, multi-machine (concurrent up to 3 devices) | 1.0 | SHOULD |
| FR-LIC-005 | Periodic re-validation (every 30 days online; offline grace 60 days) | 1.0 | MUST |

### 11.3 Sync (Studio tier)

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-SYNC-001 | Encrypted cloud sync of project bundles using user-derived key | 1.5 | SHOULD |
| FR-SYNC-002 | Conflict detection on multi-device edits with three-way merge UI | 1.5 | MUST |
| FR-SYNC-003 | Selective sync (don't sync large assets / snapshots) | 1.5 | SHOULD |

## 12. Accessibility

The platform must be usable by writers with disabilities. Accessibility is a first-class requirement.

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-A11Y-001 | Keyboard-only operation for every feature; documented shortcut map | M | MUST |
| FR-A11Y-002 | Screen-reader compatibility (NVDA, JAWS, VoiceOver, Orca) | 1.0 | MUST |
| FR-A11Y-003 | High-contrast and dark themes; user-defined colour overrides | M | MUST |
| FR-A11Y-004 | Font size and line-height controls in editor independent of export | M | MUST |
| FR-A11Y-005 | Reduced-motion mode honours OS preference | M | MUST |
| FR-A11Y-006 | All actionable elements meet WCAG 2.2 AA contrast and target size | M | MUST |
| FR-A11Y-007 | Dyslexia-friendly font option (e.g., OpenDyslexic) for editor | 1.0 | SHOULD |

## 13. Internationalisation (i18n) and localisation (l10n)

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-I18N-001 | UI strings extracted; one source of truth (ICU MessageFormat) | M | MUST |
| FR-I18N-002 | Bidirectional text (RTL: Arabic, Hebrew) supported in UI and editor | 1.0 | MUST |
| FR-I18N-003 | Per-project manuscript language (BCP-47) drives spell-check and AI prompt language | M | MUST |
| FR-I18N-004 | Localised UI: en at MVP; +es, fr, de, pt-BR, ja, zh-CN at V1.0; community translations onboarding via Crowdin or Weblate | 1.0 | SHOULD |

## 14. Logging, observability, support

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-OBS-001 | Local rotating log (5 MB × 5 files) with PII-redaction by default | M | MUST |
| FR-OBS-002 | "Save diagnostic bundle" command produces a redacted ZIP for support | M | MUST |
| FR-OBS-003 | Crash reporter (opt-in) with symbolicated stacks; hosted on a privacy-friendly service or self-hosted | 1.0 | SHOULD |
| FR-OBS-004 | In-app help with offline content; "open documentation" links default to bundled docs | M | MUST |
| FR-OBS-005 | In-app feedback / bug-report form (opt-in network) | 1.0 | SHOULD |

## 15. Update mechanism

| ID | Requirement | Phase | Priority |
|----|-------------|-------|----------|
| FR-UPD-001 | Auto-update via Tauri updater with signed packages; user can defer | M | MUST |
| FR-UPD-002 | Channel selection: stable, beta, nightly | 1.0 | SHOULD |
| FR-UPD-003 | Rollback to previous version | 1.0 | SHOULD |
| FR-UPD-004 | Plugin updates separate from app updates | 1.5 | MUST |
| FR-UPD-005 | Air-gapped update via downloaded installer | M | MUST |

## 16. Cross-cutting acceptance criteria

These apply to every feature and are part of the definition of done:

A feature is not done until it has unit tests for pure logic, a Playwright E2E for the happy path, an accessibility audit pass, an i18n key extraction pass, a performance budget assertion (where relevant), an entry in the user-facing changelog, and an update to the in-app help if it affects user behaviour.
