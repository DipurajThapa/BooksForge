# BooksForge — Visual System Proposal (v1.1 extension)

Visual / IxD extension to `outputs/DESIGN_SYSTEM.md` and `packages/ui/src/tokens.css`. Audience: frontend engineers under `apps/desktop/src-ui/` and `packages/ui/`. 2026-05-08.

---

## A. Brand voice and visual character

BooksForge feels like a small, well-lit room above a bookshop — a desktop tool, not a browser tab, not a SaaS dashboard, not a code editor in a costume. Type sits on warm, near-white paper instead of cold #FFFFFF; ink is deep walnut-black, never pure black. The accent is a single amber the product earns rather than splashes — used for the active binder node, focus ring, primary button, nothing else. Chrome is recessive: thin dividers, no cards-on-cards, no gradients, no glass. The editor is the hero and everything else apologises for being on screen. Motion is short and only ever used to explain causation. `DESIGN_SYSTEM.md` already names this character ("warm, not clinical"); this proposal makes the numbers honour it.

---

## B. Color system extension

### B.1 Reconciliation

The repo ships two color systems that disagree: the doc names `--color-surface-0..4`; the code exports `--color-neutral-50..950` plus a thinner surface set. This proposal keeps the implemented palette as the primitive layer and adds a semantic layer on top. No file rewrites.

### B.2 Primitive palette (locked — already in `tokens.css`)

Warm-grey neutrals: `n-50 #FAFAF9` · `100 #F5F5F4` · `200 #E7E5E4` · `300 #D6D3D1` · `400 #A8A29E` · `500 #78716C` · `600 #57534E` · `700 #44403C` · `800 #292524` · `900 #1C1917` · `950 #0C0A09`.

Amber: `a-50 #FFFBEB` · `100 #FEF3C7` · `200 #FDE68A` · `300 #FCD34D` · `400 #FBBF24` · `500 #F59E0B` · `600 #D97706` · `700 #B45309` · `800 #92400E` · `900 #78350F`.

### B.3 New semantic tokens

Contrast ratios vs. `--ink` (text-primary). "UI" items are gated against the WCAG 2.2 3:1 minimum for non-text UI components.

| Token | Light | Dark | L/D vs ink | Role |
|---|---|---|---|---|
| `--surface` | `#FAFAF9` | `#1C1917` | 16.2 / 16.2 | App background |
| `--surface-alt` | `#F5F5F4` | `#292524` | 14.8 / 13.6 | Shelf, inspector, panels |
| `--surface-sunken` | `#E7E5E4` | `#0C0A09` | 12.5 / 18.7 | Empty wells, snapshot rail |
| `--surface-raised` | `#FFFFFF` | `#322E2C` | 17.0 / 11.4 | Popovers, dropdowns |
| `--surface-paper` | `#FBFAF5` | `#16140F` | 16.4 / 17.6 | Editor canvas only |
| `--ink` | `#1C1917` | `#FAFAF9` | — | Body copy, headings |
| `--ink-strong` | `#0C0A09` | `#FFFFFF` | 18.7 / 21 | Display, emphasis |
| `--ink-muted` | `#57534E` | `#A8A29E` | 6.8 / 7.1 | Labels, metadata |
| `--ink-subtle` | `#78716C` | `#A8A29E` | 4.5 / 7.1 | Hints, placeholders (≥14 px) |
| `--ink-disabled` | `#A8A29E` | `#57534E` | 2.6 / 2.4 | Disabled (exempt per SC 1.4.3) |
| `--ink-inverse` | `#FAFAF9` | `#1C1917` | — | Text on `--accent` fill |
| `--accent` | `#B45309` (a-700) | `#FBBF24` (a-400) | 4.5 / 9.5 (UI ✅) | Primary action, focus ring |
| `--accent-hover` | `#92400E` | `#FCD34D` | 6.4 / 12.0 | Hover on accent |
| `--accent-pressed` | `#78350F` | `#F59E0B` | 8.6 / 7.6 | Active/pressed |
| `--accent-muted` | `#FEF3C7` | `#451A03` | bg only | Tinted callouts, badges |
| `--accent-ink` | `#78350F` | `#FCD34D` | 8.6 / 12.0 | Text on `--accent-muted` |
| `--danger` | `#B91C1C` | `#FCA5A5` | 6.1 / 7.4 | Errors, destructive |
| `--danger-muted` | `#FEE2E2` | `#450A0A` | — | Error pill bg |
| `--success` | `#15803D` | `#86EFAC` | 5.6 / 9.8 | Validation passed, saved |
| `--success-muted` | `#DCFCE7` | `#052E16` | — | Success pill bg |
| `--warning` | `#B45309` | `#FCD34D` | 4.5 / 12.0 | Validator warnings |
| `--warning-muted` | `#FEF3C7` | `#451A03` | — | Warning pill bg |
| `--info` | `#1D4ED8` | `#93C5FD` | 6.0 / 8.6 | Status messages |
| `--info-muted` | `#DBEAFE` | `#0C1B3D` | — | Info pill bg |
| `--status-local` | `#15803D` | `#86EFAC` | 5.6 / 9.8 | "All local" / on-device badge |
| `--status-network` | `#0E7490` (cyan-700) | `#67E8F9` | 4.6 / 11.7 | Outbound network active (pull, update check) |
| `--focus-ring` | `#B45309` | `#FBBF24` | 4.5 / 9.5 (UI ✅) | `:focus-visible` outline |
| `--divider` | `#E7E5E4` | `#44403C` | — | Hairline rule |
| `--divider-strong` | `#D6D3D1` | `#57534E` | — | Section break |
| `--code-bg` | `#F5F5F4` | `#0C0A09` | — | Inline code, code block |
| `--code-ink` | `#44403C` | `#E7E5E4` | 8.5 / 12.0 | Code foreground |
| `--selection` | `rgba(180,83,9,0.18)` | `rgba(251,191,36,0.28)` | — | Editor + UI text selection |
| `--marker-add` | `#DCFCE7` | `#052E16` | bg | Diff additions |
| `--marker-del` | `#FEE2E2` | `#450A0A` | bg | Diff deletions |

### B.4 What changed

- `--accent` locks to amber-700 light / amber-400 dark — the original amber-500 failed the 3:1 UI minimum and must not be used for actionable elements in light mode.
- `--surface-paper` is new, valid only inside the editor canvas — two ticks warmer than `--surface` so the paper feel is unmistakable when side panels collapse.
- `--status-local` / `--status-network` are new privacy tokens. Every Ollama, update, or pull surface uses one of them — never generic `--success`/`--info` — making the privacy invariant visible at a glance.

---

## C. Type system

### C.1 Faces (locked)

| Token | Family | Why |
|---|---|---|
| `--font-prose` | **Source Serif 4** (variable, OFL) | Designed for long-form on-screen reading; generous x-height, real italic, OpenType figures. Replaces Lora as the editor face — Lora has weak italic and loose rhythm at 18 px; it stays as a fallback. |
| `--font-ui` | **Inter** (variable, OFL) | Already in the repo. Excellent at 13–15 px. Keep. |
| `--font-mono` | **JetBrains Mono** (OFL) | Already in the repo. IDs, paths, hashes only — not editor code blocks (those use `--font-prose`; see §J). |
| `--font-display` | **Source Serif 4** (weight 600) | Same family as prose, used at large sizes. Avoids a third typeface. |

Self-host under `apps/desktop/src-ui/src/assets/fonts/` (offline-first; CDN forbidden). Latin Extended subset for v1; Cyrillic and Greek loaded on demand by project language (§I).

### C.2 OS fallback stacks

```
--font-prose: "Source Serif 4 Variable", "Source Serif 4", "Lora", "Iowan Old Style",
              "Charter", "Georgia", "Times New Roman", serif;
--font-ui:    "Inter Variable", "Inter", -apple-system, "SF Pro Text",
              "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
--font-mono:  "JetBrains Mono Variable", "JetBrains Mono", ui-monospace,
              "SF Mono", "Cascadia Mono", "Consolas", monospace;
```

### C.3 Variable-font axes

- Inter Variable: `wght 100..900`, `slnt 0..-10`. Use 400 / 500 / 600 only.
- Source Serif 4 Variable: `wght 200..900`, `opsz 8..60`. Set `font-optical-sizing: auto` on the editor root so 14 px captions and 36 px titles use the right master.
- JetBrains Mono: not variable; ship Regular + Bold only.

### C.4 Type ramp

Sizes in rem (root = 16 px); `lh` unitless line-height; `tr` letter-spacing; weights numeric.

| Token | Family | Size | lh | tr | wght | Role |
|---|---|---|---|---|---|---|
| `--type-editor-body` | prose | 1.125rem (18 px) | 1.75 | 0 | 400 | Default manuscript body |
| `--type-editor-body-comfortable` | prose | 1.1875rem (19 px) | 1.8 | 0 | 400 | Density = comfortable override |
| `--type-editor-h1` | display | 2.25rem (36 px) | 1.15 | -0.01em | 600 | Chapter title |
| `--type-editor-h2` | prose | 1.625rem (26 px) | 1.25 | -0.005em | 600 | Section / scene break |
| `--type-editor-h3` | prose | 1.25rem (20 px) | 1.4 | 0 | 600 | Sub-section |
| `--type-editor-blockquote` | prose | 1.0625rem (17 px) | 1.7 | 0 | 400 italic | Pull quote / epigraph |
| `--type-editor-caption` | ui | 0.8125rem (13 px) | 1.45 | 0.005em | 400 | Figure caption, footnote ref |
| `--type-editor-footnote` | prose | 0.9375rem (15 px) | 1.55 | 0 | 400 | Footnote body |
| `--type-editor-drop-cap` | display | 4.25rem (68 px) | 1.0 | 0 | 700 | Optional ornamental opener |
| `--type-ui-display` | display | 1.75rem (28 px) | 1.2 | -0.01em | 600 | Picker hero, modal title |
| `--type-ui-title` | ui | 1.0625rem (17 px) | 1.35 | 0 | 600 | Panel header |
| `--type-ui-body` | ui | 0.9375rem (15 px) | 1.5 | 0 | 400 | Default UI text |
| `--type-ui-label` | ui | 0.8125rem (13 px) | 1.4 | 0.01em | 500 | Form label, inspector label |
| `--type-ui-caption` | ui | 0.75rem (12 px) | 1.4 | 0.015em | 400 | Tooltip, status bar metadata |
| `--type-ui-mono` | mono | 0.8125rem (13 px) | 1.5 | 0 | 400 | IDs, paths, hashes |
| `--type-ui-button` | ui | 0.9375rem (15 px) | 1 | 0.005em | 500 | Button label |
| `--type-ui-kbd` | mono | 0.75rem (12 px) | 1 | 0 | 500 | Keyboard chord pill |

OpenType features: editor face uses `'liga' 1, 'kern' 1, 'onum' 1, 'pnum' 1` (oldstyle proportional figures sit better in prose); UI face uses `'tnum' 1` for word counts and timers so digits don't dance.

### C.5 What changed

`tokens.css` exposes only sizes, not roles — every component picks a size and the UI drifts. The proposal binds size + family + weight + line-height + tracking + role into one token. The size primitives stay as low-level escape hatches.

---

## D. Spacing, radius, shadow

### D.1 Spacing — keep the existing scale

Keep `--space-1..24` (4-px base). No half-steps. Opinionated rules:

- Editor measure: `68ch` at 18 px (≈ 612 px), tunable 55–90ch.
- Inspector: min 320 px, max 480 px. Shelf: min 240 px, max 360 px.
- Top bar: 44 px comfortable / 36 px compact. Status bar: 28 px.

### D.2 Radius

Trim from five tokens to four. Software-soft, not mobile-app-bubbly.

| Token | Value | Use |
|---|---|---|
| `--radius-xs` | 2 px | Tags, kbd pills, validator markers |
| `--radius-sm` | 4 px | Buttons, inputs, chips, binder rows |
| `--radius-md` | 8 px | Cards, popovers, dropdowns, agent cards |
| `--radius-lg` | 12 px | Modals, full-screen wizards |

Drop `--radius-xl`. The window itself does not get a radius — Tauri handles the OS frame.

### D.3 Shadow — three elevations

| Token | Light | Dark | Use |
|---|---|---|---|
| `--shadow-1` | `0 1px 0 rgb(0 0 0 / 0.04), 0 1px 2px rgb(0 0 0 / 0.05)` | `0 1px 0 rgb(0 0 0 / 0.5)` | Resting card, popover stem |
| `--shadow-2` | `0 6px 16px -8px rgb(0 0 0 / 0.16), 0 2px 4px rgb(0 0 0 / 0.06)` | `0 6px 18px -6px rgb(0 0 0 / 0.6)` | Dropdowns, hover-lifted card |
| `--shadow-3` | `0 24px 48px -12px rgb(0 0 0 / 0.22)` | `0 24px 60px -12px rgb(0 0 0 / 0.7)` | Modal, full-screen wizard |
| `--shadow-focus` | `0 0 0 2px var(--surface), 0 0 0 4px var(--focus-ring)` | same | Keyboard focus |

Replaces the existing four-tier scale. Modals don't stack their own shadow on top of `--shadow-3`. Focus mode zeroes all shadows by setting `--shadow-1..3: none`.

---

## E. Component patterns

### E.1 App shell — top bar, left shelf, center room, right inspector

Three-column desktop shell, no floating panels. Top bar is a 44-px strip — project title centered in `--type-ui-title`, leading shelf-toggle and project menu, trailing inspector-toggle and command-palette button; `--surface-alt` background, bottom hairline `--divider`. **No traffic-light spacers, no breadcrumbs, no logo.** Left shelf and right inspector each collapse to 0 px with a 180 ms `--ease-room` slide; toggle buttons hold position. The center "room" stays centered on a `--surface-paper` canvas at `68ch` measure regardless of window width — extra space becomes margin. When both side panels collapse, the gutter darkens to `--surface-sunken` so the page still reads as a page on a desk, not a floating div. Status bar (28 px) is pinned to the window bottom across all three columns and never overlays content.

### E.2 Binder item

A 28-px row with a 4-px leading status bar (color = scene status: planned/drafting/revised/final), a 6-px drag handle that appears only on row hover or keyboard focus, an 8-px folder caret for parent nodes, the title in `--type-ui-body`, and a trailing word-count badge in `--type-ui-caption` `--ink-muted`. Selection swaps the 4-px status bar for a 2-px `--accent` rule (status color persists in the title weight). Hover fills with `--surface-alt`. Drag-over shows a 2-px `--accent` insertion line above or below the row, never inside; reorder commits with a 100 ms ease-in flash. Right-click and `Shift+F10` open the context menu. Multi-select via `Shift+Up/Down` and `Cmd/Ctrl+Click`. Deleted nodes get a 4-second undo toast; no exit animation — the row vanishes immediately so the writer can keep moving.

### E.3 Editor surface

Canvas is `--surface-paper`, fixed `68ch` column with `padding: 96px 64px 192px`. Body uses `--type-editor-body`; line-height 1.75; first-line indent 1.5em **except** after a heading or a scene break. Paragraph spacing 0 — indent does the work — toggleable to block mode (0.6em margin, no indent) per project setting. Drop caps off by default; when on, the first chapter paragraph takes `--type-editor-drop-cap`, floats left, drops three lines, 8-px right margin. Footnote markers are superscript `--type-editor-caption` numerals; clicking opens a popover with a "Reveal in margin" toggle. Citations are inline tokens with a 1-px `--divider` underline and `--ink-muted` color. Scene breaks render in-flow as centered `* * *` in `--ink-muted` with 64 px breathing room — never a horizontal rule. **Focus mode** (`Cmd/Ctrl+Shift+.`) collapses both side panels, hides the top bar to a 4-px hot-zone, dims everything outside the active paragraph to `--ink-muted`, zeroes shadows. Cursor is a 1.5-px `--ink-strong` block; selection uses `--selection`.

### E.4 Inspector tab strip

Single 36-px-tall row with five tabs (Notes, Agents, Validators, Bible+Memory, Snapshots — one open at a time per `UI_UX_SPEC.md`). Active tab is marked by a 2-px `--accent` rule along its **bottom** edge and `--ink-strong` weight 600 label; inactive tabs are `--ink-muted` weight 500. **No pill backgrounds, no rounded cards behind tabs.** Active panel surface is `--surface-alt`. Labels are nouns, never abbreviated below 5 characters. Each tab can carry a numeric badge ("Validators · 3") as a 16-px-min `--accent-muted` / `--accent-ink` chip with `--type-ui-caption tnum`. Keyboard: `Cmd/Ctrl+1..5` opens a tab; `Cmd/Ctrl+0` toggles the inspector. Switching tabs is instant — no slide, no fade. The active tab persists per project.

### E.5 Agent invocation card

A `--surface-raised` card, `--radius-md`, `--shadow-1`. Header: 24-px circular avatar (mono initial on `--accent-muted`), agent name in `--type-ui-title`, state chip in `--type-ui-caption` (Idle, Running, Streaming, Proposal ready, Applied, Rejected, Errored; colors from §B; Idle is `--ink-muted` with no fill). Body: scope summary in `--type-ui-body` ("Chapter 4 · 2,841 words"), token-budget bar in `--accent` on a `--surface-sunken` track. Footer is contextual: Idle → `Run`; Running → `Cancel` + elapsed; Streaming → live preview (E.6) + `Cancel`; Proposal ready → `Review` + `Discard`; Applied → check, `Open diff`, snapshot id in `--type-ui-mono`; Rejected → reason; Errored → error code, `Retry`, `Show details`. Card never exceeds 240 px tall. State changes pulse the chip once at `--motion-base` `--ease-pulse`. Reduced-motion: opacity flip instead of pulse.

### E.6 Streaming token preview

A fixed-height region (8 lines × 1.55 lh ≈ 188 px) inside the agent card or live-run overlay, using `--type-editor-footnote` (15 px serif) on `--surface-sunken`. Tokens append at the **bottom** of a soft-clipped buffer; on overflow, content scrolls up by exactly one line-height per event, 90 ms `--ease-out`. We never re-flow, never wrap mid-word, never animate individual characters — token chunks land as whole-line groups. A 1-px `--ink-muted` caret blinks at 1 Hz at the trailing edge. The buffer's top is masked with a 24-px fade to `--surface-sunken` so older content recedes rather than clips. Reduced-motion: scroll instant, caret stops blinking. Buffer caps at ~500 visible chars per `UI_UX_SPEC.md §6.3`. Region is `aria-live="polite"` and announces only the final settled text, never every token.

### E.7 Diff / proposal viewer

Default to **inline** diff for prose (one column, additions in `--marker-add` with a leading `+` in `--success`, deletions struck through in `--marker-del` with a leading `−` in `--danger`); switch to **side-by-side** when > 30% lines change or the user toggles. Both modes use 17 px serif so the writer reads diffs near the editor face. Word-level granularity is default; line-level is a toggle. Each hunk has a header row with location ("Chapter 4 · paragraph 12"), `Accept`, `Reject`, and a `…` overflow for `Accept all in chapter`. Partial accept is supported: clicking a single inserted span accepts just that span. The viewer renders the snapshot id and timestamp at top. Keyboard: `J/K` next/prev hunk; `A` accept; `R` reject; `Cmd/Ctrl+Enter` applies all accepted hunks. Conflicts (paragraph changed since the proposal was generated) get a `--warning` banner that disables accept until the writer merges or regenerates.

### E.8 Snapshot timeline

A horizontal rail at the top of the Snapshots tab, 64 px tall, mapping snapshot history left-to-right. Each snapshot is a 6-px-wide tick on a 1-px `--divider` baseline; tick color encodes trigger (`--accent` manual, `--info` pre-AI, `--warning` pre-export, `--ink-muted` auto). Hover any tick: a `--shadow-2` popover appears above with label, timestamp in `--type-ui-mono`, word-count delta, and a 200×120 px paper-thumbnail of the first changed paragraph in editor type. Popover is sticky on focus. Below the rail, a paginated list view with `Diff vs current`, `Restore`, and `Export as bundle`. Selecting a tick scrolls the list to that snapshot — instant, no animation. When > 200 snapshots, ticks bin into 3-px columns with a small counter glyph above each bin. Reduced-motion: popover appears without slide.

### E.9 Pre-flight validators panel

Three groups in one scroll: **Errors** (open by default), **Warnings** (open), **Info** (closed). Group headers carry a count chip in the group's semantic muted/ink pair. Each row is a `--surface-raised` card with a 16-px severity icon (Lucide `alert-octagon` / `alert-triangle` / `info`), rule id in `--type-ui-mono` `--ink-muted`, message in `--type-ui-body`, a clickable location chip ("Chapter 4 · ¶12") that jumps the editor cursor, and an optional `Apply fix` button when the validator has a one-shot remedy. Errors **block** export — the export button is disabled with a tooltip naming the count; warnings **inform** via a confirm-on-export modal; info is silent. Above the groups: a primary `Run all validators` and a model-of-record badge ("Validated against KDP Paperback v1.2"). Empty state is the standard one-icon block (E.14), never a hero illustration.

### E.10 Export target picker

A modal at `--shadow-3` with three horizontal tiles: **KDP Paperback (PDF)**, **KDP Kindle (EPUB-3)**, **Google Books (EPUB-3)**. Each tile is 240×200 px, `--radius-md`, `--surface-raised`, with a 32-px line icon (book / e-reader / globe), target name in `--type-ui-title`, format pill in `--type-ui-caption`, and a bottom-right status chip: `Ready`, `Last exported 2h ago`, `Validators failing (3)`, `Profile out of date`. Selected tile gets a 2-px `--accent` border and `--shadow-2`. Below the tiles, a profile picker (page size / font / ToC depth) from template + user overrides; below that, a primary `Run pre-export check` that transitions to `Export now` once validators pass. After export, the tile flips to a success face with a `--status-local` check, file path in `--type-ui-mono`, and a `Reveal in Finder/Explorer` button. Keyboard-first: `1/2/3` selects, `Enter` runs.

### E.11 Status bar

28 px tall, `--surface-alt`, top hairline `--divider`. Left cluster: save state (`Saved · 2s ago` / `Saving…` / `Unsaved changes` with `--success` / `--ink-muted` / `--warning` dots), project word count, today's word count (`+1,240` in `--success` when positive), last snapshot age. Right cluster: Ollama state pill (`Connected · qwen2.5:7b` with `--status-local` dot; `Pulling 32%` with animated bar; `Disconnected` with `--danger` dot), an **All local** badge (always present in MVP — 2-px-bordered pill with `--status-local` text and a lock glyph) making the privacy invariant visible, and app version in `--type-ui-mono` `--ink-subtle`. Every right-cluster item is clickable: Ollama opens the wizard or model picker; All-local opens a privacy popover with the long form ("Ollama 0.5.4 on 127.0.0.1:11434 — loopback only, never network").

### E.12 Command palette (⌘K)

Centered modal, 640 px wide, 480 px max height, `--shadow-3`, `--radius-lg`, `--surface-raised`. Top: a `--type-ui-body` input with a leading 16-px search icon and a trailing kbd hint (`Esc to close`). Below: results grouped by section (Navigate, Actions, Settings, Files, Agents, Help) with `--type-ui-label` `--ink-muted` headers. Each row is 36 px tall with a 16-px leading icon, primary label, optional secondary path in `--type-ui-caption` `--ink-muted`, and a trailing kbd-chord pill. Fuzzy matching: subsequence with positional weighting and recent-use boost; matched characters wrap in `--accent-ink` weight 600. Up/Down moves selection; Enter executes; Tab cycles section; `?` on empty input shows top-level shortcut help. Opens in 120 ms with a 4-px upward translate `--ease-room`; closes in 80 ms. Reduced-motion: instant. The palette is the canonical entry point — every menu item must register an entry.

### E.13 Toast / banner

**Toast** (transient): bottom-right stack, 8-px gap, slides in 16 px from the right at `--motion-base` `--ease-room`. Each toast is `--surface-raised`, `--shadow-2`, 320 px wide, with a 4-px leading rule in the semantic color, an icon, title in `--type-ui-title`, body in `--type-ui-body`, optional action link, close ×. Auto-dismiss after 5 s (info/success), 8 s (warning), never (error — manual only). Hover/focus pauses the timer. Max 3 visible; older queue. **Banner** (persistent): full-width strip beneath the top bar, 40 px tall, semantic-muted background with semantic-ink text, only for project-wide state ("AI is off for this project — Enable in settings"). Not stackable; severity-prioritized. Reduced-motion: toasts fade in (no slide) at 80 ms.

### E.14 Empty states

Always: a 48-px Lucide line icon in `--ink-muted`, a one-line headline in `--type-ui-display` `--ink`, a two-line body in `--type-ui-body` `--ink-muted`, one primary CTA. Centered with 96-px top padding and 480 px max width. **No illustrations, no mascots, no decorative imagery** — for binary size and tone. Shape only: Project Picker — `book-open`, "Begin a book." / "Create a new project, or open one you already have." / `[+ New project]`. Binder — `list`, "This project is empty." / "Add a chapter to start writing." / `[+ New chapter]`. Agents (no model) — `cpu`, "AI is off for this project." / "Set up Ollama and pick a model to enable agents." / `[Set up Ollama]`. Validators — `check-circle`, "Nothing to report yet." / "Run validators to see issues here." / `[Run all]`.

### E.15 Settings (one room, sectioned)

A single full-window surface with a 240-px left rail and a content column — not a modal stack, not nested dialogs. Rail sections: `Application`, `Editor`, `AI / Models`, `Project`, `Privacy`, `Keyboard`, `About`; active section is `--type-ui-title` `--ink-strong` with a 2-px `--accent` leading rule. Content column max-width 720 px. Each row: label-on-left in `--type-ui-label`, `--type-ui-caption` `--ink-muted` help directly below, control aligned right. Group rows under sub-headings in `--type-ui-title`. **No tabs inside settings, no accordions** — the rail is the navigation. Save is implicit (autosave on blur with a `Saved` toast); destructive items (reset Ollama, delete project) sit in a `Danger zone` at the section bottom with `--danger`-bordered cards. Reachable from `Cmd/Ctrl+,` and the project menu — never a status-bar gear.

---

## F. Motion system

### F.1 Curves

| Token | Value | Use |
|---|---|---|
| `--ease-room` | `cubic-bezier(0.32, 0.72, 0, 1)` | Panel slide, modal open, palette enter. Drawer feel. |
| `--ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | Hover, focus, button press, toast slide. Quick start, soft land. |
| `--ease-pulse` | `cubic-bezier(0.4, 0, 0.6, 1)` | Agent state pulse, save flash, snapshot taken. Symmetric. |

Replaces the previous four-easing set; old `--ease-default`/`--ease-in`/`--ease-out` aliases stay for compatibility.

### F.2 Durations

| Token | Value | Use |
|---|---|---|
| `--motion-fast` | 90 ms | Hover, focus, button press, palette match, token-buffer scroll. |
| `--motion-base` | 180 ms | Panel collapse/expand, modal open, toast/banner slide-in. |
| `--motion-slow` | 320 ms | Settings rail content swap, full-screen wizard transitions. |

No `--motion-agent: 400ms` — agent pulses use `--motion-base`; 400 ms felt slow on the Tauri WebView.

### F.3 Reduced-motion fallbacks

`@media (prefers-reduced-motion: reduce)` collapses every motion duration to 0 ms. **Exceptions** (functional, not decorative): the streaming-token caret keeps blinking at 1 Hz so the writer sees the cursor is alive; the Ollama-pulling progress bar stays animated since it conveys download motion. Translation transitions become instant opacity transitions. Pulsing state chips (E.5) become an opacity flip 0.5 → 1, instant. **No `transform: scale()` ever** — scale biases people with vestibular sensitivity.

---

## G. Iconography

**Pick: Lucide React.** Already pinned (`^0.447.0`). Stroke-based and geometrically restrained — fits the recessive-chrome principle better than Phosphor's Bold/Fill weights, which tempt inconsistency, or Material's filled set. Heroicons is too narrow; Tabler's strokes are too heavy. Keep Lucide.

**Stroke weight.** 1.5 px at 16 px, 1.75 px at 20 px+. Set via `strokeWidth={1.5}` on a `LucideIcon` wrapper in `packages/ui/src/Icon.tsx`. **Color** always `currentColor`. **Sizing tiers** (replace `--icon-sm/base/lg/xl`):

| Token | Size | Use |
|---|---|---|
| `--icon-xs` | 12 px | Inline kbd pill, validator-row severity in compact density |
| `--icon-sm` | 14 px | Inline with body text |
| `--icon-md` | 16 px | Toolbar, binder rows, default UI icon |
| `--icon-lg` | 20 px | Tabs, panel headers, action buttons |
| `--icon-xl` | 32 px | Empty states, target picker tiles |
| `--icon-2xl` | 48 px | Onboarding hero |

**Custom marks** (cover, e-reader, KDP/Google badge) live in `packages/ui/src/icons/brand/` as inline SVGs, same stroke weight, 24×24 grid. No PNG/JPG. No emoji — emoji reads as a chat product.

---

## H. Density modes

Two modes on `<html data-density>`. Comfortable is default; compact auto-applies to the binder/outline panel and is globally toggleable in Settings → Application.

| Surface | Comfortable | Compact |
|---|---|---|
| App row height | 40 px | 32 px |
| Top bar height | 44 px | 36 px |
| Status bar height | 28 px | 24 px |
| UI body size | 15 px | 13 px |
| UI label size | 13 px | 12 px |
| Spacing scale | base ×1.0 | base ×0.875 (rounded to 4-px grid) |
| Inspector tab height | 36 px | 32 px |
| Binder row height | 32 px | 24 px |
| Editor body size | 18 px (override min 14, max 24) | unchanged — editor never compacts |
| Touch target min | 32 px | 28 px |

The editor never enters compact mode — long-form reading doesn't benefit from denser type. Compact compresses chrome only.

---

## I. Internationalization

**Text expansion.** Plan for **+35%** (German, Finnish) and **+20%** baseline. Buttons and tab labels stay flexible-width; menu items wrap to two lines rather than truncate. When truncation is unavoidable, use `text-overflow: ellipsis` with a tooltip showing the full text. Never abbreviate a translated label in code.

**RTL.** v1 LTR only; design as if RTL ships v1.5. Use logical CSS properties (`margin-inline-start`, `padding-inline-end`, `border-inline-start`) everywhere. Direction-implying icons (chevrons, send arrows) take a `direction` prop; use `<bdi>` for mixed runs. Status-bar order does not flip in RTL — the privacy badge stays trailing. The editor mirrors paragraph direction per chapter via TipTap's `dir="auto"`.

**Font subsets.** Source Serif 4 and Inter ship variable WOFF2 Latin-Extended (~80 KB each). Cyrillic, Greek, Vietnamese subsets load conditionally on project language via a `packages/ui/src/fonts/` loader module. CJK is **out of scope for v1**: the editor falls back to system serif with a banner "Editor typography is system default for this language" rather than half-rendered glyphs. JetBrains Mono ships Latin only.

**Dates / numbers.** Use `Intl.DateTimeFormat` and `Intl.NumberFormat` with the OS locale. Word counts use the writing-system definition (CJK counts characters, not whitespace tokens) — kept behind a flag in v1 so imported manuscripts still report meaningful counts.

---

## J. Print / manuscript surface

The editor canvas is a deliberately under-stated mirror of the published page — close enough that the writer reads "this is a book," not so close that it pretends to be the export.

**Margins.** Canvas padding 96 px top, 64 px sides, 192 px bottom (bottom buffer keeps the cursor off the status bar in short chapters).

**Measure.** `68ch` at 18 px (≈ 612 px), tunable 55–90ch. Wider is forbidden — long lines kill prose readability.

**Type rhythm.** Body 18 px / 1.75 lh / 0 paragraph margin / 1.5em first-line indent. Indent suppressed after headings, scene breaks, and at the start of a blockquote. **Block mode** swaps indent for 0.6em paragraph margin; default for non-fiction.

**Headings.** Chapter `H1` 96 px below canvas top, centered, `--type-editor-h1`; optional 1-px `--divider` 32 px below. Scene breaks (`H2.scene-break`) render as centered `* * *` with 64 px breathing — manuscript convention; horizontal rules look like a web page. `H3` flush-left, weight 600, 2.0em top / 0.5em bottom.

**Drop caps.** Off by default. When on, the first chapter paragraph takes a 3-line drop cap (4.25 rem, 700, optical size 60). Suppressed for punctuation or single-letter words ("A", "I").

**Footnotes.** Superscript `--type-editor-caption` numerals; click opens a popover. A `Show in margin` toggle lifts them into a 240-px right-margin column when the window is > 1280 px (reading only).

**Chapter break.** Each chapter is a virtualised ProseMirror EditorView (`UI_UX_SPEC.md §5.2`). The visual boundary is a 192-px gap with the next title 96 px in — turning a section break, not crossing a page break. No faux page breaks; pages belong to the export.

**Ornaments.** Smart quotes via TipTap's `Typography` extension (locale-aware). `--` → em-dash, `-` → en-dash, `...` → ellipsis. Old-style numerals on (`'onum' 1`).

---

## K. What `outputs/DESIGN_SYSTEM.md` gets right and what it lacks

**Right (keep).** Principles are compatible ("warm, not clinical," OS theme preference, offline-first fonts). User-adjustable prose settings (0.875–1.5 rem, lh 1.5–2.2, 55–90 ch) are well-bounded. axe-core contrast in Vitest is the right CI line. Lucide React `^0.447.0` is the right pick.

**Wrong / incomplete.** (1) Doc and ship code disagree: doc names `--color-surface-0..4`, `tokens.css` exports `--color-neutral-50..950` — picked the implemented set, added a semantic layer. (2) Original `--color-accent-500` failed the 3:1 UI minimum; the audit patched it but the doc never reflected the patch — **lock amber-700/400**. (3) Type tokens are size-only, no role binding, so components freelance. (4) Four shadow elevations — the brief asks three. (5) No `--status-local`/`--status-network` despite privacy invariants demanding a visual hook. (6) Focus mode mentioned but no zero-shadow focus profile in tokens. (7) No motion curves named. (8) No icon stroke-weight rule. (9) Density hand-rolled per component. (10) No print/manuscript-surface section. This proposal fixes all ten.

---

## L. Component naming and file conventions

The repo lays things out reasonably (`apps/desktop/src-ui/src/components/`, `packages/ui/src/`, `packages/editor/src/`, `packages/preview/src/`). What's missing is discipline about what goes where. The split below is the locked convention.

### L.1 Where a component lives

| Concern | Home | Examples |
|---|---|---|
| Generic primitives | `packages/ui/src/primitives/` | `Button`, `Input`, `Select`, `Dialog`, `Toast`, `Tooltip`, `Badge`, `Kbd`, `Icon` |
| App shell scaffolding | `apps/desktop/src-ui/src/components/shell/` | `AppShell`, `TopBar`, `LeftShelf`, `Inspector`, `StatusBar` |
| Editor-bound surfaces | `packages/editor/src/surfaces/` | `EditorCanvas`, `FocusMode`, `DropCap`, `FootnoteMarker`, `SceneBreak` |
| Inspector tab panels | `apps/desktop/src-ui/src/components/inspector/` | `NotesPanel`, `AgentsPanel`, `ValidatorsPanel`, `BiblePanel`, `SnapshotsPanel` |
| Agent sub-cards | `apps/desktop/src-ui/src/components/agents/` (exists) | `IntakeAndOutlinePanel`, `CopyeditPanel`, … |
| Agent visual blocks | `packages/ui/src/agents/` | `AgentCard`, `StateChip`, `TokenBudgetBar`, `StreamingPreview`, `ProposalDiff` |
| Modals and wizards | `apps/desktop/src-ui/src/components/wizards/` | `NewProjectWizard`, `OllamaWizard`, `ExportWizard` |
| Tokens, theme, fonts | `packages/ui/src/{tokens.css,fonts/,themes/}` | — |

### L.2 File naming

- Components: `PascalCase.tsx`, one per file. Co-locate `.test.tsx` and `.module.css`: `AgentCard.tsx`, `AgentCard.module.css`, `AgentCard.test.tsx`.
- Hooks: `useCamelCase.ts` under `packages/ui/src/hooks/` or `apps/desktop/src-ui/src/lib/hooks/`.
- Utility modules: `camelCase.ts` (already the convention).
- Token CSS in `packages/ui/src/tokens.css`; per-component CSS uses **CSS Modules** (already the rule).
- No inline `style={{...}}` except (a) user-adjustable prose settings on `#editor-root` and (b) computed positions for popovers/cursors. The current `App.tsx` `OllamaStatusBar` uses inline styles — flagged for migration to a CSS Module, not blocking.

### L.3 Component contract

Every primitive in `packages/ui/src/primitives/` exports a named TypeScript props type (never default), an explicit `displayName`, an honored `data-testid` (no auto ids), a forwarded `ref` to the root DOM element, and a `className` merged via a single `clsx` call. App-specific components in `apps/desktop/src-ui/src/components/` may use default exports per the existing `tsconfig` rule.

### L.4 Theme / dark-mode wiring

Theme is `data-theme="light"|"dark"|"high-contrast"` on `<html>`, set by `lib/theme.ts` (already exists). Density is a sibling `data-density="comfortable"|"compact"` attribute. CSS Modules query both via `:root[data-theme="dark"] .x { … }` for rare per-component overrides. No runtime `<ThemeProvider>` — the CSS-variable model handles every case and survives Tauri WebView reloads cleanly.

---

## Closing note

Bind type tokens to **roles**, not just sizes; add **semantic surface tokens** over the existing primitives; add **`--status-local` / `--status-network`** so privacy is a visible token; lock **`--accent`** to amber-700/400; trim shadows to **three** and motion to **three curves × three durations**; add the missing **manuscript-surface ruleset** and **density-mode token surface**. If only one lands first, make it **§C.4 (role-bound type ramp)** — every other downstream drift lives there.
