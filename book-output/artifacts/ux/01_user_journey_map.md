# 01 — User Journey Map (BooksForge MVP)

_Audit by UX Design Agent against the actual React/Tauri source under
`booksforge/apps/desktop/src-ui/src/components/`._

## The first-time author's path, screen by screen

| # | Screen / surface | Component | Required user input | Click count |
|---|---|---|---|---|
| 1 | Project picker | `ProjectPicker.tsx` | none — see recents or click "New project" | 1 click |
| 2 | New Project Wizard, Step 1 | `NewProjectWizard.tsx` | **title**, **author** | 2 inputs + 1 click |
| 3 | Step 2 — save location | same | bundle path (folder picker) | 1 click |
| 4 | Step 4 — template + AI toggle | same | template (defaults to "blank"), AI on/off | 0–2 clicks |
| 5a | Skip AI → editor opens with empty Binder | `EditorShell.tsx` | start typing | n/a |
| 5b | Use AI → AI brief screen | same | premise (no default); genre/audience/tone/length/chapters/model all default | 1 input + click "Generate outline" |
| 6 | Outline preview | `OutlinePreview.tsx` | accept or reject the proposed outline | 1 click |
| 7 | Editor + Binder | `EditorShell.tsx` + `Binder.tsx` | open a scene, draft via AgentsPanel | many |
| 8 | AgentsPanel switchboard | `agents/AgentsPanel.tsx` | pick one of **14 agents** | 1 click → opens the agent's panel |
| 9 | Agent panel (e.g. Chapter Drafter) | `agents/GenericAgentForm.tsx` | scene synopsis + chapter purpose + (POV default) | 2 inputs + 1 click |
| 10 | **Edit/review of generated prose** | same panel — fix landed in this session | — generated prose now renders as readable preview with **Apply to scene** | 1 click |
| 11 | Validator | `ValidatorPanel.tsx` | review issues, fix or override | n |
| 12 | Snapshots | `SnapshotsPanel.tsx` | optional — manual snapshot label | 1 click |
| 13 | Export | `ExportPanel.tsx` | choose format(s) (DOCX / EPUB / PDF) | 1–3 clicks |
| 14 | Marketplace submission | **NOT IN PRODUCT YET** — HUMAN_REQUIRED | upload to KDP / Apple / Google manually | external |

## Required-input footprint to reach "first generated outline"

- **Without AI:** 3 required inputs (title, author, save location) → 4 clicks.
- **With AI:** 4 required inputs (title, author, save location, premise) → ≈6 clicks.

Both within the brief's ≤5-input PASS gate.

## Friction-bearing transitions

1. **Switchboard tax** — picking the right agent out of 14 is intimidating for a first-time author.
2. **No book-type branching** — fiction and non-fiction users see the exact same surface; the same form fields; the same agent set.
3. **No marketplace path** — the user reaches an export bundle, then is dropped at the OS file picker. There is no "Submit to KDP / Apple / Google" surface.
4. **Manual scene drafting per scene** — the user must invoke Chapter Drafter once per scene; there is no "draft all scenes in this chapter" sweep button.
5. **Typing scene synopses by hand** — even though the outline already has scene goals, the Chapter Drafter form asks the user to retype them. A pre-fill from the outline node would remove this.

(Source citations for every line above are in the per-check evidence in `07_ux_scorecard.md`.)
