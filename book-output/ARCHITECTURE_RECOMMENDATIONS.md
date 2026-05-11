# BooksForge — Architecture Recommendations

**Date:** 2026-05-10
**Scope:** changes that make the product (a) faster, (b) more robust, (c) self-learning, (d) measurably less robotic
**Source evidence:** Round 7's 11 integrated runs (Run #1 → Run #11), plus the Run #11 quality review

---

## TL;DR — five bets, ranked by leverage

| # | Bet | Effort | Time-to-value | Quality lift |
|---|---|---|---|---|
| 1 | **Voice as a measured contract** (numeric profile, not prose description) | M | 1 week | Largest single quality lift — directly fixes the 1.4/5 monotony score |
| 2 | **Streaming + parallel pipeline** (chunked agents in flight, polish on partial draft) | M | 1–2 weeks | 25 min → 10–12 min wall clock per scene |
| 3 | **Self-learning exemplar memory** (store best paragraphs, few-shot the next run) | L | 3–4 weeks | Compounding: each successful scene improves the next |
| 4 | **Schema-aware self-healing** (Levenshtein field-typo correction, not just null-repair) | S | 2–3 days | Removes the "one bad field kills the card" failure (Run #11 card #2) |
| 5 | **Adaptive planner** replacing the linear `draft → critic → polish` chain | L | 4 weeks | Stops polish stages from running on inputs they can't fix (Run #11: metaphor SKIPPED on input with zero metaphors) |

Bets 1 and 4 are this-week work. Bet 2 is the next sprint. Bets 3 and 5 are the structural changes that make BooksForge a product, not a pipeline.

---

## 1. The empirical baseline (what 11 runs actually proved)

| Claim | Evidence |
|---|---|
| MoE 36B beats dense 27B on Apple Silicon for fiction prose | Run #2 dense-27B stalled 23+ min; Run #5 MoE-36B finished 39 min, Run #11 finished 26 min |
| 9B can't hold the full character-bible schema | Run #3 returned an empty bible array; per-character chunked agent restored 3-of-4 success |
| Thinking-mode + full bibles overflow the default 6k output cap | Run #9 returned 0 words (truncated mid-string); raising to 16k fixed it (Run #11) |
| **Bibles produce LESS rhythmically varied prose than empty bibles** | Run #5 (empty bibles): IQR=4 words. Run #11 (full bibles): IQR=1 word. Counter-intuitive — bibles enforce uniformity rather than range |
| Polish stack runs blind | `polish:metaphor` SKIPPED on Run #11 because there were no metaphors to operate on. The stage that should have *added* metaphor only knows how to *fix* metaphor |
| One field typo kills a chunked card | Run #11 card #2 emitted `"external_object"` (vs. `"external_objective"`) and was rejected wholesale; lenient retry kept the run alive |

These six findings are what the architecture changes below are designed to address. None are speculative — each maps to a numbered run.

---

## 2. The five biggest leverage points

### 2.1 Voice as a measured contract

**Today.** `voice_traits` in the character bible is a freeform string: *"Sentences alternate between short, staccato fragments (under 8 words) and long, winding observations (20+ words)."* The Run #11 model implemented the first half and dropped the second. Result: median sentence length 5, IQR 1 word.

**Change.** Make voice a *numeric profile* with measurable bands, computed from comp titles and stored as a first-class entity. Schema:

```rust
pub struct VoiceProfile {
    sentence_length_buckets: [(Range<u32>, f32); 3],  // e.g. (0..8, 0.60), (12..18, 0.25), (20..50, 0.15)
    type_token_ratio_min:    f32,                     // e.g. 0.42
    em_dash_density_per_1k:  Range<f32>,              // e.g. 0.5..3.0
    dialogue_ratio:          Range<f32>,
    repeated_opening_max:    u8,                      // hard cap on consecutive `[The X]` openings
    figurative_density_min:  f32,                     // similes + metaphors per 1000 words
}
```

The drafter prompt embeds the profile as targets (*"At least 15% of your sentences must be 20+ words. Maximum 3 consecutive sentences may share an opening clause."*). A new deterministic stage — `rhythm_polish` — scores the draft against the profile and forces a re-write of any paragraph that misses by more than one bucket.

**Why it works.** Quantitative is harder to ignore than qualitative. The Run #11 monotony was not because the model rejected the spec; it was because *"alternate between short and long"* admits a degenerate solution (all short). Bands of 60/25/15 do not.

**Code surface.** New crate `booksforge-voice` (already scaffolded per the existing CLAUDE.md mention of *"voice constraints (numeric profile from `booksforge-voice`, when wired in Phase 3)"*), one new prompt template `rhythm-polish/v1.toml`, one new agent module, one orchestrator wiring.

---

### 2.2 Self-learning exemplar memory

**Today.** Every run starts from zero. The 965 well-rhythmed words from Run #5 and the 337 collapsed words from Run #11 leave no trace on Run #12. The system has no memory.

**Change.** Two new SQLite tables:

```sql
CREATE TABLE agent_exemplars (
    id              BLOB PRIMARY KEY,        -- ULID
    project_id      BLOB NOT NULL,
    agent_id        TEXT NOT NULL,           -- 'scene-drafter-fic'
    snippet         TEXT NOT NULL,           -- 1–3 paragraph chunk
    quality_score   REAL NOT NULL,           -- critic score 0..5
    voice_profile_match REAL NOT NULL,       -- 0..1
    tags            TEXT NOT NULL,           -- JSON: ["dialogue-heavy", "interior", "action"]
    created_at      TEXT NOT NULL
);

CREATE TABLE agent_anti_patterns (
    id              BLOB PRIMARY KEY,
    pattern_regex   TEXT NOT NULL,           -- e.g. '^The (\w+) was a (\w+)\.( The \1 was a \w+\.){4,}'
    description     TEXT NOT NULL,           -- 'recursive [The X] was [Y] anaphora'
    severity        INTEGER NOT NULL,        -- 1..5
    discovered_in   TEXT NOT NULL,           -- run_id where this was first detected
    fix_hint        TEXT NOT NULL            -- prompt fragment to suppress it
);
```

Every successful run promotes its top-scored paragraphs to `agent_exemplars`. Every quality review (like the Run #11 one) adds rows to `agent_anti_patterns`. The drafter prompt is then templated with up to 3 in-context exemplars matched by tag and an anti-pattern guard list.

This is the compounding mechanism. After 50 runs the drafter has its own internalised house style derived from its own best work. After 500 runs the anti-pattern bank is doing more work than the bibles.

**Code surface.** Two migrations in `crates/booksforge-storage/migrations/`, one new memory module, prompt-template extension to inject `{exemplars}` and `{anti_patterns}` blocks, retrieval ranking by tag + recency. ~600 LOC.

---

### 2.3 Streaming and parallel pipeline

**Today.** The pipeline is strictly sequential: `intake → bibles (chunked, but sequential) → world → drafter → critic → polish×4 → tells_scan`. Run #11 wall clock is 26.6 min. The MoE drafter alone is ~10 min of that, during which every downstream agent is idle.

**Change.** Three structural moves:

1. **Parallel chunked bibles** — Run #11 runs the four character cards sequentially (each ~20s on 9B). Move them to `tokio::join!`, with a single Ollama keep-alive holding the model resident. Saves ~60s.
2. **Stream the drafter and start critique on completed paragraphs.** The Ollama API supports streaming. The critic does not need the whole scene — its axes (POV consistency, scene-goal advancement) work paragraph-by-paragraph. A streaming critic reduces critique wall clock from 4.6 min to ~30s tail.
3. **Run polish stages in parallel where the O1 detector permits.** `polish:voice` and `polish:scene_tension` are read-only-then-rewrite — they don't conflict. Today they're serial. Saves ~3 min.

Combined: 26 min → 10–12 min per scene. Below 10 min, BooksForge stops feeling like a batch system and starts feeling interactive.

**Code surface.** Modify `crates/booksforge-orchestrator/src/run.rs` (the well-tested orchestrator we just got to 79.35% coverage). The streaming critic needs a new `CriticIncremental` trait. The keep-alive change is one HTTP header in `crates/booksforge-ollama`.

---

### 2.4 Schema-aware self-healing

**Today.** `json_repair` handles null-in-list. Field name typos (`external_object` vs `external_objective` in Run #11 card #2) are hard failures. One typo loses a card.

**Change.** Add a Levenshtein-based field-name corrector in the parse path. For every JSON object key not in the target schema, find the nearest schema key with edit distance ≤ 2 and rewrite it. Log the rewrite in the audit trail so a human can review.

Stretch: when a *value* fails type validation (e.g. integer expected, string returned), attempt a type coercion before failing. If a required string field is missing entirely, prompt the model with a one-line targeted re-ask (*"Add the missing field 'external_objective' — one sentence, the character's stated goal in the world."*) rather than rejecting the whole card.

**Code surface.** Extend `crates/booksforge-agents/src/json_repair.rs`. Each agent's `parse_and_validate` opts in to schema-aware repair. ~150 LOC + tests.

---

### 2.5 Adaptive planner replacing the linear chain

**Today.** Every scene runs the same stages in the same order. The O1 detector lets stages skip themselves, but it's reactive. `polish:metaphor` skipping on Run #11 is the architectural failure: the stage that should have *introduced* metaphor only knows how to *fix* it, so it sees the absence-of-target as success.

**Change.** Replace the static stage list with a *planner* — a small agent that runs after the first draft, examines it, and produces a polish-stack DAG.

Pseudocode:

```
plan = scene_planner.examine(draft, voice_profile, scene_card)
// returns: [
//   { stage: "rhythm_polish", reason: "median 5w, IQR 1w, target IQR ≥ 4w" },
//   { stage: "figurative_inject", reason: "0 metaphors detected; profile requires ≥ 1.5 / 1k words" },
//   { stage: "polish:dialogue", reason: "0 dialogue; scene_card.beats[2] requires confrontation" },
// ]
orchestrator.execute(plan)
```

The planner is not a heavy LLM call (qwen3.5:9b is fine). It returns a small JSON. The orchestrator executes the DAG. Stages that aren't needed don't run; stages that *should* exist but don't yet (`figurative_inject`) become the obvious next thing to build, driven by real planner output rather than guesswork.

**Why this matters most long-term.** The planner is the place where "self-growing" actually happens. It learns which stage combinations correlate with high critic scores, and it deprecates polish stages that consistently no-op. Without a planner, every new polish stage is dead weight on draft text that doesn't need it.

**Code surface.** New agent `crates/booksforge-agents/src/scene_planner.rs`, new prompt template, orchestrator change to consume a `PolishPlan` instead of a hardcoded vec. ~400 LOC.

---

## 3. Naturalness — five fixes for AI-tells

These are smaller than the structural bets above but each one removes a recognisable AI fingerprint. All five are cheap.

1. **Anti-anaphora regex guard** — runtime check for `^(The \w+) (was|held|stood) ... (\1 \2 ...){4,}` patterns; force re-write. (Run #11 had two of these.)
2. **Sentence-length variance assertion in the critic** — fail the run with status NEEDS_REWORK when IQR < 3 words.
3. **"Substitution-game" detector** — when `[X] was [Y]` appears more than 5× per 1000 words with X varying and Y varying, that's the model treating the sentence as a slot-fill template. Flag and re-write.
4. **Concrete-noun budget** — every paragraph should contain ≥ 1 sensory concrete noun (touch, smell, sound, weight). Detector + re-prompt if a paragraph is all abstract.
5. **Banned-phrase list in the prompt guard** — there's already `prompt_guard`; add the AI-tell phrases (*"in the grand tapestry"*, *"speaks volumes"*, *"a testament to"*, *"navigate the complexities"*). Cheap, instant.

---

## 4. Speed — five fixes for the 25-min wall clock

1. **Parallel chunked bibles** (above): -60s.
2. **Streaming drafter + incremental critic** (above): -3 min.
3. **Parallel non-conflicting polish stages**: -3 min.
4. **Cached prompt prefixes** — the bible block is identical across all polish stages of one scene. Ollama's prompt cache has ~5-min TTL. Reorder so all stages on one scene run within the TTL. -30s.
5. **Skip the `tells_scan` on first draft** — only run after the polish stack settles. The scan on raw draft is wasted work (it always returns NEEDS_REVISION pre-polish). -45s.

Combined target: **10–12 min per scene** from today's 26.

---

## 5. Error robustness — five fixes

1. **Schema-aware field-name repair** (Levenshtein, above).
2. **Per-field re-ask** when one field fails — don't discard the whole proposal.
3. **Quarantine + downgrade** — if the heavy MoE drafter fails twice, downgrade to qwen3.5:9b for that scene with a `degraded=true` flag in the audit ledger. Better a B+ scene than no scene.
4. **Idempotent retry with progressive prompt clarification** — every retry adds one sentence pointing at the prior failure. We do this informally now; make it a structured retry policy.
5. **Lenient parse mode for chunked agents** — already shipped (Run #11). Generalise the pattern to a `LenientChunkedAgent` trait so the world bible and any future chunked agent inherits it.

---

## 6. The three things to build this week

In priority order:

1. **`booksforge-voice` crate scaffolding + numeric `VoiceProfile` schema** (bet 2.1, item 1). This unblocks bet 5 (the planner needs a profile to plan against) and is the precondition for fixing monotony at the root.
2. **Schema-aware repair + per-field re-ask** (bet 2.4). This is 2-3 days, removes a class of failure, no new architecture.
3. **The five anti-tell guards in §3.** Each is < 50 LOC. Five of them total an afternoon. They are the highest quality-per-line ratio of anything on this list.

Skip everything else for a week — these three are independently shippable, none need each other, and they cover the three loudest complaints from the 11-run campaign (monotony, fragility, AI smell).

---

## 7. The three things explicitly NOT to build (yet)

1. **The `BibleEditorPanel` UI** — proposed as a fallback when chunked bibles failed. They no longer fail. Building the UI now is solving last week's problem; the writers' workflow needs the *generated* bibles to be good before it needs an editor.
2. **A second tier-1 model swap.** We already paid the experimental cost of dense-27B → MoE-36B. The next swap (e.g. to a 70B) needs benchmark evidence, not a hunch. Hold the model fixed for at least 50 more runs.
3. **Microservices split.** It came up in conversation. The orchestrator is 79.35% covered, runs in one process, and the bottlenecks are LLM latency, not orchestration latency. Splitting now adds operational surface for zero throughput gain. Revisit only if the wall clock target after bet 2.3 still misses.

---

## 8. The single most important point

The Run #5 → Run #11 comparison is the most expensive lesson this campaign produced: **a more specific bible produced less interesting prose**. That is not a model bug. It is a spec-design bug. Constraints expressed as descriptive prose are read by the model as ceilings, not as ranges. Every architectural change in this document follows from that one observation:

- **Voice profile as numeric bands** (bet 2.1) — replaces ceilings with ranges.
- **Exemplar memory** (bet 2.2) — replaces description with example.
- **Adaptive planner** (bet 2.5) — replaces fixed sequence with conditional structure.
- **Anti-tell regex guards** (§3) — replaces "try not to be repetitive" with "you may not be repetitive."

If only one of these ships, ship voice profile.
