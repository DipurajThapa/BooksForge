# BooksForge — Writer Journey Redesign (2026-05)

Author: product strategy pass against the existing codebase.
Status: implementation-ready proposal. Section 18 maps every item to current code.

---

## 1. Executive Summary

BooksForge already has **most of the engine** the spec asks for: 25 agents
(intake, outline-architect, character-bible-chunked, world-bible, scene-drafter-fic
+ critic + 4-stage polish, copyedit, continuity, humanization, dev-editor, final-review-editor,
proposal-validator, memory-curator, vocab-dictionary), 4 export crates
(EPUB, Pandoc PDF/DOCX, Typst, EPUBCheck), 16 deterministic validators
(HRC hierarchy, KDP metadata, AI-tells density, originality n-gram), 5 book
kinds, publishing-target profiles (KDP/Apple/Google/IngramSpark), and a
per-paragraph 6-axis quality scorer.

**What's missing is not the engine, it's the chassis.** Agents fire from a 14-button
toolbar with no enforced sequence; quality scoring exists but doesn't gate progression;
two book categories (poetry, technical) have no path; market analysis isn't built;
cover assembly is a folder of files. The wizard collects ~12 fields when the spec calls
for ~40+, so downstream agents work with thin context.

This document redesigns the **chassis** — a 14-stage linear journey with 8.5/10
quality gates between stages, three writing modes (Manual / Hybrid / AI Writer),
explicit human approval points, and a publishing-ready exit ramp — built on top of
the existing agent + validator + export stack. **Aim:** the writer never sees the
agent inventory; they see a journey. Failures route back to the right stage with a
specific fix instruction.

---

## 2. Problems in the Current Journey

Audited against today's code and yesterday's bugs:

| # | Problem | Root cause |
|---|---|---|
| 1 | Wizard collects ~12 fields; spec needs 40+ | `ProjectBrief` schema is fiction-shaped; no formatting/printing preferences, no audience map, no sub-genre |
| 2 | After outline applies, user lands in a toolbar of 14 buttons with no obvious next step | No journey state machine. Each panel exists independently. |
| 3 | Generic Novel template + outline-architect both seed nodes → duplicate tree | `apply_outline` always emitted a 2nd project root (FIXED 2026-05-11). Templates still seed placeholder scenes that aren't reconciled. |
| 4 | Quality scores compute but never block progression | `paragraph_quality` returns a 0-10 number; no agent in the pipeline reads it and reroutes for revision. |
| 5 | Poetry / Technical books have no path | `BookKind` enum has 5 entries; no agent prompts target poetry or pedagogy. |
| 6 | Market analysis is missing entirely | Conflicts with local-first invariant (no outbound network by default). Opt-in toggle + clear consent needed. |
| 7 | Cover / boilerplate / front+back matter assembly is a folder of files, not a workflow | `assets/` exists in the bundle but no panel walks the writer through importing/positioning. |
| 8 | "Humanization" is one agent, not a stage with verification | We have the agent and an AI-tells scanner. We don't loop until score ≥ 8.5. |
| 9 | No sensitivity / risk review pass | No agent exists. Spec requires it. |
| 10 | No hybrid-mode attribution tracking | Each scene's pm_doc is opaque; we don't track which paragraphs are AI / human / edited. |

---

## 3. Redesigned End-to-End Journey (14 stages)

Each stage has: **inputs**, **AI behavior**, **manual fallback**, **approval gate**, **next stage trigger**. The journey is a state machine; you can't progress past a gate without either passing it or explicitly overriding.

```
   ┌──────────────────────────────────────────────────────────────────┐
   │  STAGE 1   Book Setup                  → quality ≥ 8.5/10 ──┐   │
   │  STAGE 2   Audience Map                → quality ≥ 8.5/10   │   │
   │  STAGE 3   Market & Originality        → originality risk < │   │
   │            (opt-in network)              0.20               │   │
   │  STAGE 4   Mode Choice                 (Manual / Hybrid /   │   │
   │                                          AI Writer)         │   │
   │  STAGE 5   Characters (fiction-only)   → each ≥ 8.5/10      │   │
   │  STAGE 6   World / Setting             → ≥ 8.5/10           │   │
   │            (optional for poetry)                            │   │
   │  STAGE 7   Outline & Structure         → flow ≥ 8.5/10      │   │
   │  STAGE 8   Drafting                    → per-scene ≥ 8.0    │   │
   │                                          (revision threshold)│   │
   │  STAGE 9   Flow / Coherence Pass       → ≥ 8.5/10           │   │
   │  STAGE 10  Content Quality Pass        → ≥ 8.5/10           │   │
   │  STAGE 11  Humanization Pass           → ai-tells < 6/1000  │   │
   │  STAGE 12  Editorial Pipeline          (5 sub-passes, each  │   │
   │                                          ≥ 8.5/10)          │   │
   │  STAGE 13  Formatting & Cover Assembly                      │   │
   │  STAGE 14  Final Audit & Export        → all validators pass│   │
   └──────────────────────────────────────────────────────────────────┘
```

**Override mechanism:** any gate can be skipped by clicking "Publish anyway"
which records `override_reason` in the audit ledger. The export still includes
the failing-gate report so the writer knows what they shipped.

---

## 4. User Flow by Stage

### Stage 1 — Book Setup

**Input fields** (all stored to `manifest.toml` + `book:project_brief`):

| Field | Type | Required | Source |
|---|---|---|---|
| Title | text | yes | exists |
| Subtitle | text | no | **new** |
| Author name | text | yes | exists |
| Book kind | enum | yes | exists; add `Poetry`, `Technical`, `Academic` |
| Genre | enum-or-text | yes | exists |
| Sub-genre | enum-or-text | no | **new** |
| Estimated chapter count | int | yes | exists |
| Estimated word count | int | yes | exists |
| Premise / thesis | textarea (1-3 sentences) | yes | exists |
| Background / concept setup | textarea | no | **new** |
| Tone | text | yes | exists |
| Writing style | enum (spare / propulsive / wry / academic / playful) | yes | **new** |
| Target book size (trim) | enum (5×8, 6×9, 7×10 …) | yes | exists in `TargetSpec`, hoist to setup |
| Publishing format | multi-checkbox (epub/paperback/hardcover/PDF/DOCX) | yes | **new** |
| Printing preferences | nested form (paper, color, margins) | yes for print | **new** |
| Formatting preferences | nested form (font, size, drop-caps, chapter style) | yes | partial — extend |

**AI action: "Refine my book concept"** (8.5/10 gate)

- Calls existing `intake` agent → returns refined `ProjectBrief`.
- Runs new **`concept_scorer`** agent on the refined brief with rubric:
  - Clarity (does the premise read in one breath?)
  - Originality (vs. an embedded corpus of public-domain comps)
  - Emotional pull (does the premise have a wound or a wonder?)
  - Market fit (kind/genre/sub-genre × target word count realism)
  - Execution potential (can a 25k-word book actually deliver this?)
- Score < 8.5 → returns numbered weaknesses + a revised premise + `next_action: "regenerate"`.
- Score ≥ 8.5 → unlocks Stage 2.

**Manual fallback:** writer types everything themselves, clicks "I've checked these" → unlocks Stage 2. Concept scorer still runs in the background and surfaces score in the corner ("Concept: 7.4/10 — consider tightening the antagonistic force").

**Approval gate:** explicit "Lock in setup" button. Writes a `setup_locked: true` flag to `book:project_brief`. Stages 5–14 read this; if not locked, they show a banner "Setup not locked yet — improvements may force re-runs."

### Stage 2 — Audience Map

**Why this is a separate stage:** today's `ProjectBrief.audience` is one text field. The spec wants a map. We promote it.

| Field | Type | Default |
|---|---|---|
| Primary audience | text | from setup |
| Secondary audience | text | empty |
| Age range | range slider (8-80) | derived from book_kind |
| Reader interests | tag input | empty |
| Reader pain points | tag input | empty |
| Reader expectations | tag input | empty |
| Emotional outcome desired | text | empty |
| Practical outcome desired (non-fiction) | text | empty |
| Comparable books | tag input | from setup `comp_titles_or_authors` |
| Reading level | enum (middle-school / YA / adult-general / academic) | derived |
| Preferred tone | text | from setup |
| Cultural / regional considerations | textarea | empty |
| Sensitivity considerations | tag input | empty |

**AI action: "Generate Reader Expectation Map"**

New agent **`audience_mapper`** (Light tier, ~30 s):

- Input: setup brief + audience fields.
- Output: a structured `ReaderExpectationMap`:
  - `genre_expectations: string[]` — what readers expect.
  - `genre_anti_patterns: string[]` — what they may dislike.
  - `emotional_promises: string[]` — what the book should fulfil.
  - `recommended_themes: string[]`
  - `recommended_tropes: string[]`
  - `tropes_to_avoid: string[]`
  - `pacing_expectation: "slow-build" | "page-turner" | "episodic" | "lyrical"`
- Persisted to `book:audience_map` memory.
- Read by every downstream prose agent in their `creative_profile` block (same wiring the Brief uses today).

**Approval gate:** writer clicks "Accept map" or edits + saves.

### Stage 3 — Market & Originality

**Opt-in, network-gated.** Default: this stage runs offline using only the embedded corpus (public-domain comps). With explicit consent the system can fetch market signals.

**Offline path (default, no consent prompt):**

- Run existing **originality scanner** (n-gram against project's prior scenes + the public-domain corpus shipped with `booksforge-originality`).
- Output an `OriginalityRiskScore` (0–1; gate threshold: < 0.20).
- Compute a `comp_titles_similarity` map by Jaccard on premise tokens vs the user-supplied `comp_titles_or_authors`.

**Online path (consent required — explicit "Allow market research" toggle):**

- New module **`booksforge-market`** with adapters for at-rest sources (Goodreads RSS, GBooks API, Amazon ASIN lookup). Cached locally.
- Calls visit `127.0.0.1` only by default; outbound calls require **per-call** consent surfaced as a system toast ("BooksForge wants to query Goodreads for 3 comps. Allow?").
- Output `MarketOpportunity` with: comparable_titles_table, what_is_selling, what_to_avoid, differentiation_strategy, positioning_statement.

**Honest framing:** plagiarism cannot be guaranteed by AI alone. The originality risk score is a *signal* (low score = lots of unique n-grams; high score = phrases that overlap with the comp corpus). Human review still required. We surface the risk number, the matching spans, and the source; we do not claim "100% original."

**Approval gate:** `originality_risk < 0.20` AND writer reviewed the positioning statement.

### Stage 4 — Writing Mode

Three modes, picked at this stage and persisted in `ui.app_mode`. Already shipped (`ModePicker.tsx`) but with two modes; this stage adds **Hybrid**:

| Mode | Empty-scene CTA | Per-paragraph attribution |
|---|---|---|
| **Manual** | "Start writing" | All paragraphs tagged `human` |
| **AI Writer** | "Generate this scene" | All paragraphs tagged `ai_drafted` then re-tagged after edits |
| **Hybrid** | Cursor + sidebar with "Expand", "Continue", "Rewrite" inline AI actions | Per-paragraph attribution: `human`, `ai_drafted`, `ai_edited`, `human_edited_ai` |

**New domain field:** `SceneContent.paragraph_attribution: Vec<Attribution>` (one per paragraph, indexed by paragraph ordinal). Pure addition; backward-compatible (`#[serde(default)]`).

**UI consequence:** the Inspector panel gains a "% human / AI" pie per scene + per book.

### Stage 5 — Characters (fiction / memoir only; skipped for non-fiction, poetry, technical)

Existing `CharacterCard` schema has 11 fields. Spec adds 3:

| Field | New? |
|---|---|
| Name | exists |
| Role | exists |
| **% coverage in book** | **new** (0-100 int) |
| Background | exists (as `internal_need` + `fear_or_wound`) |
| Personality, intent, objective, motivation, conflict | exists (as `external_objective` + `internal_need` + `fear_or_wound`) |
| Emotional arc | exists (as `chapter_arc` + `emotional_turning_points`) |
| Relationships | exists |
| Secrets / backstory | exists (as `secret_or_contradiction`) |
| Voice & dialogue style | exists (as `voice_traits`) |
| **First appearance** (chapter number) | **new** |
| **Final state by end** | **new** |

**AI action: "Optimize characters"** — runs existing `character-bible-chunked` (4-char default), then a new **`character_critic`** agent that:

- Cross-checks: any duplicate-name relationships? Coverage % sums to ≤ 105%? Each major character has a non-empty conflict?
- Scores each card 0–10 on: depth, consistency, uniqueness, narrative usefulness, emotional impact.
- Any card < 8.5 → returns specific edit suggestions and re-runs the per-character chunked agent for just that role.

**Approval gate:** all cards ≥ 8.5/10, no duplicate names, coverage sum ≤ 105%.

### Stage 6 — World / Setting (optional for poetry; required for fiction/memoir/technical)

Existing `WorldBibleProposal` has 7 sections. Spec maps cleanly. **No new fields**, but we add:

- **`Locations.first_appearance`** (chapter number) — new optional field on `WorldLocation`.
- World critic agent (similar to character_critic): checks `continuity_constraints` is non-empty for fiction, `sensory_palette` has at least 3/5 senses filled, `history` ≥ 30 words.

For **technical / academic** books, "world" is renamed "**Subject domain**" in the UI and the schema is reused for: `locations` → "concept areas", `social_rules` → "domain conventions", `continuity_constraints` → "non-negotiable facts the book must respect."

### Stage 7 — Outline & Structure

Existing `outline-architect` produces `OutlineProposal` (parts → chapters → scenes). We add:

**For Fiction:** scene cards now persist `scene_goal`, `scene_conflict`, `scene_reveal`, `emotional_beat`, `hook_into_next` (today these live only in the example runner). New domain type `SceneCard` stored in `scene:<id>:card` memory; scene-drafter-fic reads them per-scene (today it gets thin titles).

**For Non-Fiction:** outline-architect emits a **`KnowledgeStructure`** instead of `OutlineProposal`: chapters carry `thesis`, `reader_problem`, `key_claims[]`, `supporting_evidence[]`, `examples[]`, `citation_requirements[]`. Existing `chapter_drafter_nf` already accepts these; we just need to surface them in the outline-architect template for the non-fiction path.

**For Poetry:** new agent **`poetry_sequencer`** emits a `PoemSequence` with: theme_clusters, emotional_arc, voice_target, per-poem cards. Replaces the chapter/scene tree with `collection → cluster → poem` hierarchy. (New `NodeKind::Cluster`, `NodeKind::Poem`.)

**For Technical / Study:** outline emits a **`PedagogyStructure`**: `chapter.objective`, `definitions[]`, `examples[]`, `exercises[]`, `summaries[]`. New agent **`pedagogy_architect`** + new node kinds `Lesson`, `Exercise`.

**Quality gate:** new agent **`structure_critic`** scores the outline on:
- Promise-payoff (every key_promise from setup is delivered by ≥ 1 chapter)
- Flow (chapter purposes don't repeat; transitions imply causation)
- Reader satisfaction (final chapter resolves the central tension)
- Length realism (chapter word totals × per-scene plausibility ≈ target word count)

Score < 8.5 → outline-architect re-runs with the critic's edits as additional input.

### Stage 8 — Drafting

Existing flow: `agent_run_book_pipeline` (already shipped). The redesign:

- Per-scene drafter takes the **full `SceneCard`** as input (today it takes title + thin defaults — quality lift is meaningful here).
- Per-scene critic + 4-stage polish runs by default in AI Writer mode; opt-in in Hybrid; off in Manual.
- **Per-scene quality threshold: 8.0** (more permissive than the final 8.5 because polish runs after this).
- Failed scenes route to **revision queue** — a panel showing scenes < 8.0 with the weakest axis named ("Scene 7: rhythm 4.2/10 — too many short sentences"). One-click "Re-draft with these notes" sends the critic's notes back to the drafter.

### Stage 9 — Flow / Coherence / Consistency

Runs the existing **`continuity`** agent across all drafted scenes + a new **`flow_auditor`** that checks:

- Chapter-to-chapter transitions (does ch5 open from where ch4 left off?)
- Timeline consistency (POV character dates / ages / season cues line up)
- World-building consistency (locations described identically across appearances)
- Pacing curve (act-2 sag detector — flat tension across 5+ scenes)

Output: a list of `ContinuityIssue { scene_id, severity, description, suggested_fix }`. Issues ≥ "warning" must be addressed (or explicitly overridden) before Stage 10.

### Stage 10 — Content Quality

Runs `paragraph_quality` scorer (already shipped: 6 axes, weights, returns 0–10) over every scene. The new wrapper **`scene_quality_aggregator`** rolls per-paragraph scores up to per-scene with a per-axis breakdown.

For scenes scoring < 8.5 on any axis, route to the appropriate polish agent:
- `rhythm < 1.5/2.0` → rhythm-expansion polish (already shipped in `multi_chapter_run`)
- `figurative < 1.0/1.5` → metaphor-polish
- `sensory < 1.5/2.0` → sensory-grounding polish (new variant of metaphor-polish)
- `mattr < 1.0/1.5` → vocab-diversity polish (new — re-runs with explicit "avoid these 20 most-repeated words" instruction)
- `low_token_tells < 0.7/1.0` → AI-tells humanization
- `no_structural < 1.5/2.0` → dialogue or scene-tension polish

This is essentially the **adaptive polish planner** I drafted earlier (FEATURE_HARDENING_PLAN.md Item 4) — surface the existing infrastructure as a stage with a clear UI.

### Stage 11 — Humanization

Existing `humanization` agent + AI-tells scanner. The redesign:

- Run AI-tells scan; if `weighted_density_per_1000 > 6.0`, send to humanization with explicit "these are your worst phrases" prompt.
- Run **voice consistency check**: stylometric distance between every scene and the user-anchored voice fingerprint (existing `voice_fingerprint` + `stylometric_distance` modules).
- Re-score; loop until tells < 6.0 AND voice distance < threshold OR 3 iterations elapsed.
- If still failing after 3 iterations, route to the writer with "These 4 paragraphs need your eye" rather than spinning forever.

### Stage 12 — Editorial Pipeline

5 sub-passes, each its own agent + each its own 8.5/10 gate:

| Pass | Agent | Checks | Pass criterion |
|---|---|---|---|
| Developmental | `dev_editor` (exists) | Structure, plot/argument, pacing, character arcs, reader promise | ≥ 8.5 + writer accepts top 3 recommendations |
| Line | `line_editor` (**new**, derived from `copyeditor`) | Sentence quality, style, flow, voice, emotional impact | rhythm + clarity ≥ 8.5 |
| Copy | `copyeditor` (exists) | Grammar, punctuation, usage, consistency | 0 errors at "error" severity |
| Proofread | `proofreader` (**new** — light pass) | Typos, formatting mistakes, final errors | 0 errors at "error" severity |
| Sensitivity / Risk | `sensitivity_review` (**new**) | Harmful stereotypes, cultural issues, legal risks, defamation (memoirs) | 0 critical-risk flags; human approves all warnings |
| Market-readiness | `market_review` (**new**) | Title strength, hook strength, genre fit, commercial clarity | composite ≥ 8.5 |

Each pass writes its findings to `book:editorial_pass:<name>` memory. The writer sees a checklist; each pass has its own "Accept changes" / "Reject" / "Edit manually" controls.

### Stage 13 — Formatting & Cover Assembly

Two parts:

**Interior formatting** (existing infrastructure, surface in stage):

- Trim size (from `TargetSpec.allowed_trims`)
- Font pair (heading + body) — preset menu by genre
- Font size, line spacing, margins, gutter (defaults from `TargetSpec`)
- Chapter title design (3-4 presets per genre)
- Drop caps yes/no
- Header / footer + page number style
- Scene break symbol
- Widow / orphan control toggle
- Image placement preferences

**Cover & boilerplate assembly** (**new flow**):

- Front cover upload (validates against `TargetSpec.cover_min_px`, `cover_aspect_x100`)
- Spine + back cover OR full wraparound
- ISBN / barcode
- Author photo
- Interior images (with auto-placement preview)
- Front matter editor: title page, copyright, dedication, epigraph, ToC, foreword
- Back matter editor: epilogue, acknowledgments, about the author, also by, bibliography, index
- Compatibility report: each platform (KDP, IngramSpark, Apple, Google) shows pass/fail per requirement

**New domain types:** `CoverSet`, `BoilerplatePage` (kind + content + position).
**New IPC:** `cover_import`, `cover_validate`, `boilerplate_save`, `boilerplate_list`.

### Stage 14 — Final Audit & Export

Existing infrastructure: 16 validators + `prepare_for_publishing` + 4 export crates. The redesign:

- **Audit board** — single panel showing all 16 validators + 3 custom audits (coverage of stages, attribution check for hybrid mode, formatting compatibility).
- Severity-grouped: errors block, warnings prompt, info silent.
- Each issue has: code, message, jump-to-node link, auto-fix where available, human-approve-required flag.
- **Export bundle** — single click writes EPUB + print-PDF + DOCX + metadata sheet + audit report + per-platform compatibility report into `<bundle>/exports/<timestamp>/`.

---

## 5. AI vs Manual vs Hybrid — Behavior Matrix

| Action | Manual | Hybrid | AI Writer |
|---|---|---|---|
| Stage 1 setup | Writer fills all fields | Writer fills, AI suggests on demand | AI drafts a setup from a one-paragraph idea, writer reviews |
| Stage 5 characters | Writer types cards | Per-card: writer types OR AI generates | All 4 cards auto-generated, writer reviews |
| Stage 7 outline | Writer types chapter/scene cards | AI proposes, writer accepts/edits per chapter | Full outline auto |
| Stage 8 drafting | Writer types prose | "Continue from here" / "Expand this" inline AI | Full per-scene auto |
| Stage 9 continuity | Writer self-checks | AI surfaces issues, writer decides | AI surfaces + auto-fixes minor, surfaces major |
| Stage 12 editorial | Writer self-edits or sends to a human | AI suggests, writer accepts | AI applies, writer reviews top-5 changes per chapter |
| Stage 14 export | Writer ships when ready | Writer ships when audits pass | AI proposes ship date, writer confirms |

**Per-paragraph attribution** is tracked in all three modes. The export bundle's metadata sheet states the percentages.

---

## 6. Required Screens / Pages

```
Project picker (existing — keep)
   ├─ Recent projects
   ├─ New project          → wizard
   └─ Open                 → editor shell

Wizard (existing — extend to 7 steps from current 4)
   1. Book kind            (existing; add Poetry, Technical, Academic)
   2. Title / author       (existing)
   3. Save location        (existing)
   4. Concept              (new — premise + background + tone + style)
   5. Audience             (new — Stage 2 fields)
   6. Format & printing    (new — trim, fonts, formats)
   7. Mode + AI consent    (existing ModePicker)

Editor shell (existing — keep redesigned toolbar)
   ├─ Stage rail (NEW)     left-side vertical strip showing all 14 stages
   │                       with traffic-light status; click to jump
   ├─ Binder               (existing)
   ├─ Editor center        (existing)
   ├─ Inspector            (existing; add attribution pie chart)
   └─ Stage-specific panels  (mostly exist; some new):
      ├─ Brief panel              (existing, extend)
      ├─ Audience map panel       (NEW)
      ├─ Market & originality     (NEW for online; offline OK today)
      ├─ Bibles panel             (existing)
      ├─ Outline panel            (existing OutlinePreview, extend with SceneCard editor)
      ├─ Book generation panel    (existing; now bound to Stage 8)
      ├─ Revision queue panel     (NEW — Stage 8 failed scenes)
      ├─ Flow report panel        (NEW — Stage 9 findings)
      ├─ Quality dashboard        (NEW — Stage 10 axes per scene)
      ├─ Editorial board          (NEW — Stage 12 pass status)
      ├─ Formatting panel         (NEW — Stage 13 layout)
      ├─ Cover assembly panel     (NEW — Stage 13 cover/boilerplate)
      └─ Audit & export panel     (existing prepare-for-publishing; extend)
```

---

## 7. Quality Gate System

Each gate has the same shape:

```rust
pub struct QualityGate {
    pub stage_id: StageId,
    pub axes: Vec<QualityAxis>,           // e.g. ["clarity", "originality", "emotional_pull"]
    pub min_score_per_axis: f32,          // 8.5
    pub min_composite: f32,               // 8.5
    pub blocking: bool,                   // true by default; "Publish anyway" sets false
    pub max_revision_iterations: u32,     // 3 — after which we surface to the writer
}
```

**Scoring rubric per axis** (consistent across all gates):

| Score | Meaning |
|---|---|
| 9.5 – 10.0 | Publishable as-is; no revision recommended |
| 8.5 – 9.4 | Passes the gate; minor polish optional |
| 7.0 – 8.4 | Needs targeted revision; agent runs again with named weakness |
| 5.0 – 6.9 | Significant rewrite needed; agent runs with broader edit instructions |
| < 5.0 | Stage failure; surface to writer with full diagnostic — no auto-retry |

**Loop behavior:** on score < 8.5, the gate runs the corresponding fix-agent up to 3 times. If still < 8.5 after 3 tries, surface to the writer with the diagnostic and let them either edit manually, lower the threshold for this stage, or skip the gate.

---

## 8. Originality & Plagiarism Framework

We do not promise "no plagiarism." We promise:

1. **Boundaries:** the AI is forbidden, via prompt-level instruction, from naming or quoting any work by title outside the user's own `comp_titles_or_authors` list.
2. **Detection:** every drafted scene is scanned for n-gram overlap (≥ 6-gram exact match) against:
   - The project's own prior scenes (anti-self-plagiarism for serial writers).
   - The shipped public-domain corpus (`booksforge-originality`).
   - The user-supplied comp_titles_or_authors (n-gram match only on any user-uploaded sample text).
3. **Risk score** is published as a number, not "passed/failed." Threshold 0.20 is the gate but the writer can override.
4. **Source attribution:** every claim the AI makes in a non-fiction draft has a `[needs-citation]` tag if it can't be verified against the user-supplied source corpus.
5. **Human review required:** before publishing the export bundle includes a "Originality Reviewed by Human" checkbox the writer must tick. We log the timestamp.

**Online research consent:** any Stage 3 online call shows a system toast naming the domain and the query. User clicks Allow / Deny. Refused calls don't fail the stage — Stage 3 degrades to offline.

---

## 9. Human-in-the-Loop Approval Points

Every stage produces a Tauri command + a "Reviewed and approved" record:

| Approval | Stored as |
|---|---|
| Setup locked | `book:setup_locked: true` + timestamp |
| Audience map accepted | `book:audience_map_accepted_at: <iso>` |
| Originality reviewed | `book:originality_reviewed_at: <iso>` |
| Characters approved | `book:characters_approved_at: <iso>` |
| World approved | `book:world_approved_at: <iso>` |
| Outline approved | `book:outline_approved_at: <iso>` |
| Drafting frozen | `book:drafting_frozen_at: <iso>` |
| Flow pass accepted | `book:flow_pass_accepted_at: <iso>` |
| Quality pass accepted | `book:quality_pass_accepted_at: <iso>` |
| Humanization accepted | `book:humanization_accepted_at: <iso>` |
| Editorial pass [name] accepted | `book:editorial_<name>_accepted_at: <iso>` |
| Cover validated | `book:cover_validated_at: <iso>` |
| Final audit signed off | `book:final_audit_signed_off_at: <iso>` + writer's review notes |

The export bundle's metadata sheet lists every approval timestamp + the agent runs that fed each stage. This is the audit trail.

---

## 10. Failure Cases & Recovery Flows

| Failure | Recovery |
|---|---|
| Agent fails 3× with parse error | Surface raw output to writer; offer "Lower JSON-mode strictness" toggle; offer manual entry of the missing structure |
| Quality gate fails 3× | Surface diagnostic; offer manual edit, lower threshold, or skip with override reason recorded |
| User cancels mid-pipeline | Already-applied stages preserved; in-flight stage rolls back via snapshot |
| Outbound network refused mid-Stage 3 | Stage 3 degrades to offline-only output; banner explains what's missing |
| Project bundle has duplicate roots | `apply_outline` refuses with explicit message (shipped 2026-05-11) |
| Cover file fails platform validation | Stage 13 panel shows per-platform pass/fail; writer can re-upload or accept some platforms only |
| Export EPUBCheck fails | Audit board lists every EPUBCheck error with severity + auto-fix where possible |

---

## 11. Recommended MVP

Six-stage subset that delivers a publishable book end-to-end on the existing
infrastructure. Aim: **shippable in 2–3 weeks of focused frontend work + the new
agents specified below.**

| Stage | Backend status | Frontend status (post-archive) |
|---|---|---|
| 1. Book Setup (extended) | brief schema needs 3 new fields | new `BookSetupWizard.tsx` (replaces current wizard) |
| 2. Audience Map | new `audience_mapper` agent | new `AudienceMapPanel.tsx` |
| 5. Characters | existing chunked + new `character_critic` | extend `BiblesPanel.tsx` |
| 7. Outline | existing + new `structure_critic` + extended `SceneCard` storage | extend `OutlinePreview.tsx`, new `SceneCardEditor.tsx` |
| 8. Drafting | existing book pipeline | existing `BookGenerationPanel.tsx` (good as-is) |
| 13–14. Format & Export | existing | new `FormattingPanel.tsx` + extend `PrepareForPublishingPanel.tsx` |

Skipped in MVP: Market & Originality online, Sensitivity review, Hybrid attribution
tracking, Poetry / Technical paths. All have agent shells but aren't on the critical
path for a fiction novel ship.

---

## 12. Recommended Advanced Version

Adds everything not in MVP:

- Stage 3 with online market research (consent-gated).
- Hybrid mode with per-paragraph attribution + percentage UI.
- Stages 9, 10, 11, 12 as full gated pipelines (today they're agents that run but don't gate).
- Editorial board panel.
- Sensitivity + market-readiness reviews.
- Poetry pipeline (poetry_sequencer + new node kinds).
- Technical / Study pipeline (pedagogy_architect + new node kinds).
- Per-platform compatibility report.
- Custom font upload + custom chapter title design.
- Audit board with auto-fix for 80%+ of validator findings.

---

## 13. Data Model — Key Entities (current → needed)

```rust
// ─── EXISTING (keep, extend) ───────────────────────────────────────

pub struct ProjectBrief { /* 14 fields today; ADD: */
    pub subtitle: Option<String>,                        // NEW
    pub sub_genre: Option<String>,                       // NEW
    pub background: Option<String>,                      // NEW
    pub writing_style: Option<WritingStyle>,             // NEW enum
    pub publishing_formats: Vec<PublishingFormat>,       // NEW
    pub printing_preferences: Option<PrintingPrefs>,     // NEW
    pub formatting_preferences: Option<FormattingPrefs>, // NEW
    pub setup_locked_at: Option<DateTime<Utc>>,          // NEW gate flag
}

pub struct CharacterCard { /* 11 fields today; ADD: */
    pub coverage_pct: Option<u32>,        // NEW
    pub first_appearance: Option<u32>,    // NEW (chapter number)
    pub final_state: Option<String>,      // NEW
}

pub struct SceneContent { /* existing; ADD: */
    pub paragraph_attribution: Vec<Attribution>,  // NEW: one per paragraph
}

pub enum Attribution { Human, AiDrafted, AiEdited, HumanEditedAi }

// ─── NEW DOMAIN TYPES ──────────────────────────────────────────────

pub struct ReaderExpectationMap {
    pub genre_expectations:    Vec<String>,
    pub genre_anti_patterns:   Vec<String>,
    pub emotional_promises:    Vec<String>,
    pub recommended_themes:    Vec<String>,
    pub recommended_tropes:    Vec<String>,
    pub tropes_to_avoid:       Vec<String>,
    pub pacing_expectation:    PacingExpectation,
}

pub struct SceneCard {
    pub scene_id:        Ulid,
    pub goal:            String,
    pub conflict:        String,
    pub reveal:          String,
    pub emotional_beat:  String,
    pub hook_into_next:  Option<String>,
    pub pov:             Option<String>,
}

pub struct OriginalityRisk {
    pub overall_score:           f32,    // 0–1
    pub matches:                 Vec<NgramMatch>,
    pub corpus_sources_checked:  Vec<String>,
    pub reviewed_by_human_at:    Option<DateTime<Utc>>,
}

pub struct MarketOpportunity {
    pub comparable_titles:        Vec<CompTitle>,
    pub what_is_selling:          Vec<String>,
    pub what_to_avoid:            Vec<String>,
    pub differentiation_strategy: String,
    pub positioning_statement:    String,
    pub sources_queried:          Vec<NetworkQuery>,  // for audit trail
}

pub struct CoverSet {
    pub front:      Option<AssetRef>,
    pub back:       Option<AssetRef>,
    pub spine:      Option<AssetRef>,
    pub wraparound: Option<AssetRef>,
}

pub struct BoilerplatePage {
    pub kind:    BoilerplateKind,  // TitlePage, Copyright, Dedication, Foreword, …
    pub content: String,
    pub position: BoilerplatePosition,  // FrontMatter | BackMatter
}

pub enum BookKind {  // existing; ADD:
    LiteraryFiction, GenreFiction, NonFiction, Memoir, ChildrensBook,
    Poetry,       // NEW
    Technical,    // NEW
    Academic,     // NEW
}

pub enum NodeKind {  // existing; ADD for poetry + technical:
    Project, Part, Chapter, Scene, FrontMatter, BackMatter,
    Cluster,   // NEW — poetry
    Poem,      // NEW — poetry
    Lesson,    // NEW — technical
    Exercise,  // NEW — technical
}
```

---

## 14. Roadmap (4 phases)

### Phase A — Chassis (1 week)
Reuse all existing agents; build the journey shell.
1. Stage rail in editor shell — vertical strip with 14 lights (red/amber/green/grey).
2. Wizard expansion to 7 steps (concept → audience → format).
3. Audience map agent + panel.
4. Setup-locked flag enforcement in downstream agents.
5. Hybrid mode + per-paragraph attribution domain type (UI in Phase B).

### Phase B — Quality gates (1 week)
Convert existing agents into gated stages.
6. Quality dashboard panel reading `paragraph_quality` scores per scene.
7. Revision queue panel for scenes < 8.0.
8. Flow report panel using existing `continuity` + new `flow_auditor` agent.
9. Editorial board reading existing dev_editor / copyedit results.
10. New `concept_scorer`, `character_critic`, `structure_critic` agents.

### Phase C — Polish + Cover (1 week)
11. Cover assembly panel.
12. Boilerplate editor.
13. Formatting panel surfacing existing `TargetSpec` choices.
14. New `sensitivity_review`, `market_review`, `proofreader`, `line_editor` agents.

### Phase D — Genre expansion + market (2 weeks)
15. Poetry pipeline: `poetry_sequencer`, `cluster` + `poem` node kinds.
16. Technical pipeline: `pedagogy_architect`, `lesson` + `exercise` node kinds.
17. `booksforge-market` crate with consent-gated network adapters.
18. Per-platform compatibility report.
19. Audit board auto-fix wiring.

Stretch (Phase E):
- Voice anchoring from writer's prior books (upload, fingerprint, lock).
- Co-author collaboration via shared snapshots.
- Print-cost estimator per `TargetSpec`.

---

## 15. Verify-and-Revise — Self-Audit of This Plan

Issues caught after first draft, fixed in this version:

| Issue | Fix |
|---|---|
| Stage 5 (characters) was mandatory in v1 — bad for non-fiction | Now optional; gated on `book_kind` |
| 8.5/10 gate was hard-block — would frustrate writers with strong drafts that the scorer dislikes | "Publish anyway" override with `override_reason` written to audit |
| Market analysis violated local-first invariant | Made network calls per-call consent-gated; offline degradation default |
| Originality framework claimed "no plagiarism" | Reframed as risk score with mandatory human review checkbox |
| Hybrid mode had no concrete state model | Added `Attribution` enum + per-paragraph storage |
| Editorial pipeline lumped 5 passes into 1 stage | Split into 5 sub-passes each with its own pass criterion |
| Poetry and technical were named but not designed | Added new node kinds + new agents + outline-architect variant |
| Quality scoring loop could spin forever | 3-iteration cap, then surface to writer |

Open questions (assumptions stated):

1. **Web research opt-in cost** — assumed users will accept per-call consent prompts. If too intrusive, fall back to a single per-project "Market research mode: on/off" with full audit log.
2. **Per-platform formatting** — assumed each platform's `TargetSpec` (KDP, IngramSpark, Apple, Google) already encodes its requirements correctly. Worth verifying when building the compatibility report.
3. **8.5/10 threshold** — chosen as the spec asked. Consider exposing as user-tunable (per-stage slider) for experienced writers who know their book is unusual.
4. **Custom corpus uploads** — out of scope here; advanced writers may want to upload their own backlist as a voice corpus. Plumb the storage now, surface UI later.

---

## 16. What This Doc Is NOT

- A throwaway brainstorm — it's a buildable spec; every entity / agent / panel either exists or is named with a concrete signature.
- A commitment to ship all 14 stages — the MVP is 6 stages.
- A claim that all current code is right — Phase A explicitly reuses code; bugs will be fixed inline as we hit them (e.g. the apply_outline duplicate-root, fixed 2026-05-11).
- A UI mock — visual design is downstream of journey design. The component list is binding; their look is open.

---

**Next step:** confirm MVP scope (six stages above? different?) and which phase to start. The 14-stage end-state is the destination; we should ship phases A→B→C→D rather than try-to-build-all-at-once.
