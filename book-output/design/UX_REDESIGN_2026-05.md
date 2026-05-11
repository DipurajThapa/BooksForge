# Editor UX redesign — 2026-05

## The problem

The current editor toolbar surfaces **14 equal-weight buttons** in one row:

```
AI Setup · Debug AI · Focus · Snapshots · Check · Knowledge · Brief ·
Workflow · Agents · Export · Publish · Help · Settings · Close
```

None are visually weighted by importance. A writer opening the app for
the first time has no idea which button starts them writing the book.
"Workflow" sounds important but is a passive checklist; "Generate Book"
(the action that actually drives the AI pipeline) was buried as one of
the equally-weighted buttons. "Debug AI" sits next to "AI Setup" with
the same visual treatment, even though one is a developer escape hatch
and the other is a one-time onboarding step.

The writer is not told *how to use the app*. They have to discover the
flow by clicking each button.

## What writing-app UIs converge on (2024-26 reference set)

Patterns observable across **Scrivener** (manual), **Atticus** (manual
+ formatting), **Sudowrite** (AI-first), **NovelCrafter** (AI + binder),
**Plottr** (outliner-led):

1. **One mode commitment up front.** "Are you writing this yourself or
   with AI?" The whole shell reshapes around that decision. Mixing
   modes invisibly is what made Word's outline view fail — too much
   surface area for too little intent.

2. **One primary action.** Not 14. The primary action is contextual:
   on an empty project it's "Start writing" / "Generate first chapter";
   on an open scene it's "Continue" / "Refine"; on a complete book
   it's "Export".

3. **Everything else lives behind a `⋯` menu.** Menus are scanned
   linearly; toolbars compete for attention spatially. Settings, Snapshots,
   Validators, Help, Debug — all of these are "I need this every few
   sessions" actions, not "I need this every time I sit down to write."

4. **Mode is permanent and visible.** A top-bar pill ("Manual mode" /
   "AI Writer") tells the writer what shell they're in and lets them
   swap with one click. Same shape as the GitHub branch pill or the
   Notion workspace switcher.

5. **The empty state IS the CTA.** When a writer clicks an empty scene
   in AI mode, they don't see "Select a scene to start writing" —
   they see a large "Generate this scene" button. The empty state is
   the highest-conversion teaching surface in the app.

## The redesign

### Two modes, picked once, swappable from the top bar

* **📝 Manual writing mode** — the writer types. AI assistance is
  passive: spell-check, autocomplete on `Tab`, the validator panel,
  vocab. No agent runs unless the writer explicitly opens a panel.
* **🤖 AI Writer mode** — the writer is the editor. The agents draft;
  the writer reviews, accepts, refines. Every empty scene shows a
  "Generate this scene" CTA. Every drafted scene shows a "Refine /
  Polish" CTA.

The mode is stored in user settings (`ui.app_mode`) and applied per
session. A first-time-open prompt asks for the choice; the user can
switch from the toolbar pill at any time.

### Toolbar reduced to 4 visible elements

```
[BooksForge] [Project title]                [Mode pill] [Primary CTA] [⋯] [Close]
```

* **Mode pill** — `📝 Manual` or `🤖 AI Writer`, click to switch.
* **Primary CTA** — context-bound:
  | Mode | Selected node | Primary CTA |
  |---|---|---|
  | AI Writer | nothing / project root | **Generate Book** |
  | AI Writer | empty scene | **Generate this scene** |
  | AI Writer | drafted scene | **Refine scene** |
  | Manual | any | **Word count** indicator (no CTA) |
* **`⋯` overflow menu** — Brief, Workflow, Agents, Knowledge, AI Setup,
  Snapshots, Check, Export, Publish, Settings, Debug AI, Help.
* **Close** — exits the project to the picker.

### Editor empty state in AI mode

Replace:
> *"Select a scene to start writing."*

with a hero panel:

```
   ╔══════════════════════════════════════════════════╗
   ║   This scene hasn't been drafted yet.            ║
   ║                                                  ║
   ║   ┌────────────────────────────┐                 ║
   ║   │  ✨  Generate this scene   │                 ║
   ║   └────────────────────────────┘                 ║
   ║                                                  ║
   ║   …or write the first paragraph yourself; the   ║
   ║   AI will continue from where you stop.          ║
   ╚══════════════════════════════════════════════════╝
```

The first-paragraph-then-AI flow is where Sudowrite and NovelCrafter
have converged — it gives writers control over voice without forcing
them to either fully write or fully delegate.

### What we keep

* The binder (left). Already correct — chapters and scenes, drag/drop
  reorder, status dots.
* The inspector (right). Already correct — POV, beat, target words.
  Hidden in Focus mode; visible by default.
* The status bar (bottom). Word count + saving state.

### What we remove from the chrome

Nothing is *deleted* — every existing panel still works. They move
behind the `⋯` menu. The Workflow checklist (which is a passive
approval-gate UI, not an agent runner) becomes one item in the menu
labelled "Approval gates" so its purpose is unambiguous.

## Implementation scope (this commit)

1. New top-bar component with mode pill + primary CTA + `⋯` menu.
2. Mode state in `ui.app_mode` (localStorage for now; to migrate to
   `~/.booksforge/settings.toml` once the field is added there).
3. Empty-scene CTA in AI mode → wires to existing
   `agent_run_full_scene_pipeline` for the selected scene.
4. `Generate Book` panel (already shipped) becomes the AI-mode
   primary CTA when no scene is selected.
5. First-time mode picker overlay on first project open after this
   ships (one prompt, then the choice persists).

## Out of scope (deliberately deferred)

* The "first paragraph then AI continues" mode — needs a streaming
  continuation agent that doesn't exist yet.
* Per-scene mode override (so writer can be Manual on Chapter 1 and
  AI on Chapter 2). Adds a per-node config that's not justified
  until users ask for it.
* Renaming `Workflow` to `Approval gates` — pure cosmetic; rolled
  in if no other change is touching the menu.

## Why this is reversible

Every existing panel still mounts via the same component; only the
trigger location moves. If a user prefers the old toolbar, a single
boolean in settings (`ui.legacy_toolbar`) restores it — out of scope
this commit but trivial to add.
