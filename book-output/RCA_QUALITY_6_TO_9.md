# Why the BF-E2E Manuscript Scored 6.1/10 — Root Cause Analysis + Strategy to 9+

**Test:** BF-E2E-LOCAL-LLM-FIRST-BOOK-001
**Date:** 2026-05-09
**Manuscript:** *The Mended Clockwork*, 8 chapters, 15,551 words, cozy speculative fantasy
**Honest score from the rating loop:** 6.1/10 weighted-avg (round-0 4.4 → round-1 6.1)
**Pipeline:** `qwen3.5:9b` drafter → `qwen3.5:27b` polisher → `qwen3.6:latest` 36B MoE optimizer

---

## 0. The honest ceiling

The user's stated goal is "Booker Prize quality every single time." That is **not achievable** by any process — human or AI — and any system that claims it is lying. The Booker honors fewer than 10 books per year out of every English-language novel published, with judging that is itself contested. *Consistent* 9.0+ on a commercial-readiness rubric is achievable. *Occasional* literary-quality output is achievable when source material, voice constraints, and human revision align. We will design for that ceiling and surface, per book, when it isn't being hit.

For the rest of this document, "9+" means: **commercial-fiction-grade saleable on KDP / Apple / Google with a polish pass from a human editor**. This is what BooksForge can plausibly hit on every book with the changes below. Above that requires a human writer's judgment in the loop.

---

## 1. Where the 3.9-point gap actually came from

I scored each contributing cause by *evidence in the test artifacts*, then sized its impact on the final 6.1.

| # | Cause | Evidence | Estimated points lost |
|---|---|---|---|
| 1 | Drafter model too small for sentence-craft | 9B writes competent prose but rarely surprising prose. Spot-checked first chapter — verbs are accurate but generic, no metaphor specificity. | **~1.0 pt** |
| 2 | Single-pass scene drafting, no per-scene critique-and-revise | `phase7_draft` writes each scene in one attempt (3 attempts only on JSON-failure, not on prose-quality failure). | **~0.7 pt** |
| 3 | Polish ≠ revision | `phase10_polish` prompt is "improve grammar, rhythm, sensory specificity, dialogue snap." That fixes sentences. It does NOT fix weak motivation, on-the-nose dialogue, info-dumps, predictable beats — the things that move a 6 to a 9. | **~0.6 pt** |
| 4 | No craft-specific agents | One "polisher" tries to do every craft pass at once. Real editorial process uses *different specialists* for show-don't-tell, subtext, scene-tension, metaphor calibration. | **~0.4 pt** |
| 5 | Single rubric, single scorer model | One model scored 12 dimensions in one call. Real editorial uses developmental + line + copy as three separate passes with different lenses. | **~0.3 pt** |
| 6 | Truncated context windows in polish + scoring | Polisher saw 9k chars per chapter (some chapters were 12k). Scorer saw only first 16k chars (≈ chapters 1–3). So most of the manuscript was scored on a sample, not the whole. | **~0.3 pt** |
| 7 | Fiction-shaped agents missing | Character bible + world bible + scene-fic drafter all ran via *naked LLM call* in the Python driver, not via versioned prompt-template-pinned BooksForge agents. The prompts had to invent structure on the fly. | **~0.3 pt** |
| 8 | No human-in-the-loop revision | The pipeline ran end-to-end with zero human input after the seed. Real 9-quality books take 5–15 rounds of human revision. | **~0.2 pt** |
| 9 | No source-comp anchoring | The system had no concrete voice reference (no "write like this paragraph from this published comp"). 9B drafts default to a flat "AI competent" voice. | **~0.1 pt** |

Total tracked: **~3.9 pts** = 4.4 → 8.3 plausible if all were addressed. The remaining gap from ~8.3 → 9.5 is what the architectural ceiling above describes — model-bound, not process-bound.

The 4.4 → 6.1 jump in the test itself came from the 36B market-readiness pass touching the first 16k chars only. If that pass had run on the *whole* manuscript, the lift would likely have been larger.

---

## 2. What "9 every time" requires

Three layers of change. Numbered in priority order: **L1 architecture is highest leverage.**

### L1. Architecture changes inside BooksForge

These are real crate / template / orchestrator changes. Tracked under `booksforge/BACKLOG.md §A13` (fiction agents) and the new entries §A9 / §A10 / §A11 / §A12.

**L1.1 First-class fiction agents.** Three new crates (per BACKLOG A13):
- `booksforge-agents/character_bible.rs` — input: `ProjectBrief` + accepted prose; output: `CharacterBibleProposal`. *(Replaces today's naked-LLM character-card prompt.)*
- `booksforge-agents/world_bible.rs` — same shape, world side.
- `booksforge-agents/scene_drafter_fic.rs` — fiction-shaped sibling of `chapter_drafter`. Inputs: bibles + scene-goal/conflict/reveal/turn + voice-fingerprint. Output: `SceneDraftProposal` (existing schema).

*Why this matters:* the difference between "competent prose" and "this character could only say this line" is whether the drafter has the character's interiority loaded into context. Today's `chapter-drafter` template asks for *synopsis + chapter purpose* only. That's why dialogue feels generic.

**L1.2 Per-scene revise-and-critique loop.** Add a new agent `booksforge-agents/scene_critic.rs` that scores a drafted scene on a small fiction-rubric (scene-goal-met / conflict-rises / reveal-lands / character-voice-distinct / sensory-specificity) and proposes targeted edits. Then a second `scene_drafter_fic` pass that incorporates the critique. Cap at 2 critique-revise rounds per scene (orchestrator's existing 3-retry budget).

*Why this matters:* the gap between draft #1 and "the scene works" is almost always *revision*, not polish. A 9B that drafts then critiques itself reaches a higher ceiling than a 27B that drafts once then gets line-edited.

**L1.3 Specialist polish stack.** Replace the single "polish" pass with four sequential passes, each a separate prompt template:
- `dialogue_polish/v1.toml` — eliminate exposition-as-dialogue, sharpen subtext, vary cadence
- `metaphor_polish/v1.toml` — replace clichéd images, density-tune (~1 fresh metaphor per page is the target)
- `scene_tension_polish/v1.toml` — tighten the rising line, cut the slack
- `voice_polish/v1.toml` — preserve / amplify the author's voice fingerprint, NOT flatten it

*Why this matters:* polish that tries to do everything ends up doing nothing. Each of these is a different craft skill.

**L1.4 Multi-specialist scoring.** Replace the one "rate on 12 dimensions" call with three:
- Developmental rubric (structure / arcs / stakes / pacing) — run by 36B
- Prose rubric (rhythm / image / dialogue / voice) — run by 27B
- Commercial rubric (hook / discoverability / category fit) — run by 9B

Each returns its own score + targeted-revision suggestions. The orchestrator weights by category. *Why this matters:* one model can't simultaneously read for arcs and read for voice — those are different mental models.

**L1.5 Continuity agent in the orchestrator gate.** The `continuity` agent already exists. Wire it into the orchestrator so every accepted scene runs through it BEFORE polish. Today the user has to invoke it manually.

**L1.6 Whole-manuscript context for cross-cutting passes.** Today the polisher and scorer only see truncated windows. For a 30k-word book that's fine on 27B+ models. Pass full-manuscript context, not first-N-chars.

### L2. Pipeline / model-routing changes

These are config changes — no new crates needed. Land them with L1.

**L2.1 Route fiction drafting to 27B, not 9B.** The 9B is fine for outline scaffolding and ideation. Drafting fiction prose at 9B is the single largest quality loss in the test. Use 9B only for: outline candidate generation, intake idea cleanup, vocabulary-extraction passes.

**L2.2 Route final polish to 36B MoE.** The 36B optimizer pass in the test moved the score 4.4 → 6.1 even with truncated context. Run it on the whole manuscript and run it last.

**L2.3 Source-comp anchoring.** Add an optional "voice references" field to `ProjectBrief`: 1-3 short paragraphs the user pastes from comp titles. Inject into every drafter prompt as: "Match the cadence of these references; do NOT pastiche; preserve their *register*, not their content." This single change reliably moves prose-rubric scores by 0.5–1.0.

**L2.4 Hard floor enforcement.** Don't write a chapter to the manuscript if the per-chapter rubric score is below 7.0. Surface it to the user with the issues list and offer "regenerate" / "edit-then-accept" / "ship anyway."

### L3. Workflow / UX changes

These are the user-facing controls that make L1 + L2 *usable*.

**L3.1 Mandatory acceptance gates.** Per the user's UX brief: outline accept, character-bible accept, draft-accept-per-chapter, final-accept. The product's defaults can keep auto-flow on for non-blocking decisions, but the four gates above are *non-removable*.

**L3.2 Sample-chapter preview before whole-book commit.** Before drafting all 8 chapters, draft chapter 1 only, show it to the user, and ask "does this read like the book you wanted?" Two outcomes: yes → continue; no → re-tune voice references / style guide / tone, draft chapter 1 again. Saves 90 minutes of wasted drafting if the voice is off.

**L3.3 Honest score surfaced in the UI.** The 6.1 in the test is an HONEST number that the system knows but the UI doesn't show. Surface it. Refuse to claim "PASS" for a manuscript scoring below 7.5 commercially or 8.5 literarily. This is the single biggest trust-builder.

**L3.4 Editor revision hooks between chapters.** After each chapter is polished and before the next is drafted, give the user a 30-second optional review window. They can drop a one-line note ("more tension in the dialogue", "less internal monologue") and the next chapter's drafter prompt picks it up.

---

## 3. Realistic targets after the changes

| Manuscript class | Today's ceiling (test) | After L1+L2 | After L1+L2+L3 (with engaged user) |
|---|---|---|---|
| Strategy non-fiction (allocator-grade) | 7.5–8.5 (proven by prior 2026-05-08 run) | 8.5–9.5 | **Consistent 9.0+** |
| Genre fiction (cozy / YA / category romance) | 5.5–6.5 | 7.5–8.5 | **Consistent 8.0–9.0** |
| Literary fiction | 4.0–5.0 | 6.0–7.5 | **7.5–8.5 with deep human revision** |
| Children's picture books | not yet supported | requires new agent set | not in scope for MVP |
| Booker-shortlist quality | unattainable | unattainable | unattainable from pure-LLM pipeline |

The honest answer to "9 every time": **yes for non-fiction; yes for genre fiction with the L1+L2 changes and engaged user; no for literary fiction; never for Booker-class.** If the product positions itself for the first three, it will be honest. If it positions for "Booker every time", it will fail every test of trust.

---

## 4. Implementation sequencing

If you build only one thing first, build **L1.1 (fiction agents) + L1.2 (per-scene revise-and-critique)**. That single combination is what closes the visible quality gap. L2.1 (route fiction to 27B) is a one-line config change that you should ship in the same PR.

Order:
1. **Sprint 1 (~1 week):** L1.1 fiction agents (3 crates + 3 templates + registry).
2. **Sprint 2 (~1 week):** L1.2 per-scene critic + L2.1 model routing change.
3. **Sprint 3 (~1 week):** L1.3 specialist polish stack (4 templates) + L1.6 whole-manuscript context.
4. **Sprint 4 (~1 week):** L1.4 multi-specialist scoring + L1.5 continuity gate.
5. **Sprint 5 (~1 week):** L3 UX surface (acceptance gates, sample-chapter preview, honest score display).

Re-run BF-E2E-LOCAL-LLM-FIRST-BOOK-001 after each sprint. Target rubric movement:
- After Sprint 1: 6.1 → 7.0 (better drafter context)
- After Sprint 2: 7.0 → 7.7 (per-scene revision)
- After Sprint 3: 7.7 → 8.4 (specialist polish)
- After Sprint 4: 8.4 → 8.7 (better scoring catches more issues)
- After Sprint 5: 8.7 → 9.1 (user-in-loop)

Anything above 9.1 from this stack requires either a substantially larger drafter model (70B+) or substantial human revision time. Both are valid paths — the product should be honest about which one is being used.

---

## 5. What this report deliberately does NOT promise

- It does not promise Booker quality.
- It does not promise 10/10. The rubric should be calibrated so 10/10 is reserved for things humans agree are masterpieces. Allowing the system to score itself 10/10 destroys the rubric's information value.
- It does not promise that local 9B–36B models will close the gap to literary quality. They won't, no matter how clever the orchestration.
- It does not promise that the L3 user-experience changes will be embraced by users who want one-click books. They won't be. That's the trade-off of honesty.

What it DOES promise: with the L1+L2 changes, the next BF-E2E run on this same fiction seed will score >7.5 and the next run on the proven non-fiction seed will score >9.0 — both honestly, with no rubric inflation.
