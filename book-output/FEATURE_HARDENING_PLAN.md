# Feature Hardening Plan — Voice / JSON Repair / Anti-Tells

**Date:** 2026-05-10
**Scope:** the three features shipped earlier today (`VoiceTarget`, Levenshtein
field-name self-healing, structural anti-AI-tells). For each, what's wrong,
what the literature recommends, and what to ship next.

This is a *critical review* — every section assumes the shipped code works
(it does; 42 net-new tests are green) and asks whether it's *good enough for
the manuscript-quality bar BooksForge promises in marketing*. Several places
the answer is "no, ship the harder fix."

---

## 1. `booksforge-voice` — six things to harden

What shipped: `VoiceTarget` with sentence-length bucket bands, TTR floor,
em-dash cap, repeated-opening cap, long-sentence-per-paragraph floor,
dialogue requirement. Three calibrated defaults (literary / commercial /
punchy). `directive_block()` for prompts. `score()` for verification.

### 1.1 Replace raw TTR with MATTR — the highest-leverage fix in this doc

**The problem.** Type-Token Ratio is *length-biased*: shorter text has
higher TTR by construction. The Run #5 → Run #11 comparison in
`quality-review.md` cited TTR 0.303 vs 0.45+ literary norm — but Run #5
was 965 words and Run #11 was 337 words, so the comparison overstates
the gap. The published literature is unambiguous on this:

> "MATTR is the only length-insensitive index that can compare texts
> of different sizes. Simple TTR, Root TTR, and Log TTR never level
> off at any point in the 50–200 token range."
> — *Investigating minimum text lengths for lexical diversity indices*

We're shipping cross-run comparisons today (the architecture
recommendations literally compare TTR across runs of different
lengths) on a metric the literature has flagged as broken for that
purpose since at least 2010.

**The fix.** Add three new fields to `VoiceProfile`:
- `mattr_50` — Moving-Average TTR with a 50-token window
- `mtld` — Measure of Textual Lexical Diversity
- `hdd_42` — HD-D probabilistic estimate at the 42-token threshold

`type_token_ratio_min` in `VoiceTarget` keeps working for back-compat
but we add `mattr_min` as the new authoritative floor and route
existing comparisons through it. Implementation is ~80 LOC of pure
loops over the existing `word_tokens` output.

**Effort:** half a day. **Impact:** every cross-run quality comparison
becomes valid; the literary floor becomes a defensible number.

### 1.2 Sentence splitter — close the 5% wrong-boundary gap

**The problem.** `split_sentences` in `lib.rs` is documented as "crude
but adequate" and uses a single regex (`.!?` followed by whitespace +
capital). The published baseline for naive rule-based SBD is ~95%
accuracy. The 5% it gets wrong is concentrated in cases that *matter
for fiction*:

| Edge case | Behavior today | What we want |
|---|---|---|
| `Dr. Smith said hello.` | splits at `Dr.` | one sentence |
| `She said "No." and left.` | splits at `No."` | one sentence |
| `D.H. Lawrence wrote.` | splits at `D.` and `H.` | one sentence |
| `She paused… then spoke.` | splits at `…` (sometimes) | one sentence |
| `Why? Why now?` | splits correctly | (works) |

Every metric we ship — sentence-length distribution, IQR, anaphora,
TTR — is built on top of this splitter. A 5% boundary error rate
becomes a 5% noise floor on every downstream score.

**The fix.** Two-stage:

1. **Abbreviation gazetteer.** Add a hardcoded set of period-ending
   tokens that do NOT end sentences: `Mr Mrs Ms Dr Prof Sr Jr St Mt
   Rev Capt Sgt Lt Col Gen Hon Sen Rep Inc Ltd Co Corp etc i.e e.g
   vs vs. cf p.m a.m`. ~50 entries. When the token preceding the
   period is in this set, do not break.
2. **Quote-aware breaking.** Track quote nesting depth as we walk; do
   not break inside `"..."` or `'...'` or `\u{201C}...\u{201D}`.

This gets us to ~99% per the SBD literature. The remaining 1% are
genuinely ambiguous (shortened names without context). Acceptable
floor.

**Effort:** 1 day. **Impact:** removes a 5% noise floor from every
voice metric.

### 1.3 Bucket scoring — replace pass/fail with KL divergence

**The problem.** Each sentence-length bucket is scored independently
with a tolerance band. A draft that matches every bucket within
tolerance passes; a draft that misses one bucket fails. But:

- A draft 5% over on the short bucket and 5% under on the medium
  bucket is "broken" today, even though the total distribution
  shape is closer to target than a draft that matches short
  perfectly and misses medium by 14%.
- Compensation isn't credited. The literary target wants 40/35/20;
  a draft at 50/30/20 is wrong on bucket 1, *but* the over-allocation
  came from the right place (the long bucket is intact).

**The fix.** Compute Kullback-Leibler divergence between the target
distribution and the observed distribution. Single number, 0 = perfect
match, monotonically increasing as the shape drifts. Keep the bucket
pass/fail too (it's useful for surfacing *where* the miss is) but
gate `overall_passes` on KL ≤ threshold rather than per-bucket.

**Effort:** 2 hours. **Impact:** the pass/fail signal correlates
better with what a human reader perceives as "wrong rhythm."

### 1.4 Anaphora — extend to 2-token and 4-token chains

**The problem.** `repeated_opening_max` only checks 3-token openings.
Real failure modes the detector misses:

- 2-token: `"She fell. She rose. She fell. She rose. She fell."` —
  five sentences, identical 2-token openings, distinct rhythm.
  Won't trip the 3-token check (third token differs each time).
- 4-token: `"The hand on the door. The hand on the lock. The hand on
  the chain."` — three sentences with identical 4-token openings;
  trips the 3-token check too, but then the chain-length count is
  capped by the 3-token grouping when in fact the 4-token chain is
  longer / more egregious.

**The fix.** Compute three chain lengths: `max_chain_2tok`,
`max_chain_3tok`, `max_chain_4tok`. Allow each its own cap in the
target. Default caps for literary: 4 / 3 / 2. The 2-token cap is
laxer because real prose legitimately uses 2-token repetition for
emphasis ("She walked. She paused.") within reasonable counts.

**Effort:** 2 hours. **Impact:** catches the second class of
mechanical-rhythm failure that Run #11 also exhibited but we missed.

### 1.5 Long-sentence floor — extend below the 4-sentence threshold

**The problem.** `passes_long_sentence_floor` exempts paragraphs with
< 4 sentences. A 3-sentence paragraph of 5/4/5 word lengths is also
mechanically uniform but the rule doesn't catch it.

**The fix.** Drop the floor to 3 sentences. The risk (false positive
on intentional minimalism — a Carver or Hemingway paragraph) is
addressed by item 1.7 below (deliberate-minimalism exception).

**Effort:** 30 minutes. **Impact:** small but tightens the contract.

### 1.6 Wire `VoiceTarget` into the bible TOML

**The problem.** `VoiceTarget` is serde-serializable and tested
round-trip but no bible TOML can yet declare one. The drafter
prompt template can't render `directive_block()` because the prompt
template doesn't have a slot for it. Until both gaps close, the
shipped code is purely defensive — the actual prose-shaping benefit
is zero.

**The fix.** Three changes:

1. New `[voice_target]` section in the bible TOML schema, parsed into
   `VoiceTarget`.
2. New `{{ voice_target_directive }}` slot in the
   `scene-drafter-fic/v1.toml` prompt template.
3. New default in the genre packs: every genre pack ships with one of
   the three calibrated targets pre-filled (literary cluster gets
   `literary_default()`, romantasy gets a custom blend, etc.).

**Effort:** 1 day end-to-end. **Impact:** turns the shipped code into
something that actually shapes prose.

### 1.7 Deliberate-minimalism exception (paragraph-level override)

**The problem.** A scene whose intent is mechanical uniformity —
Carver's terse minimalism, an interior-grief paragraph using
deliberate anaphora — should be allowed to override the contract on
a per-paragraph basis. Today it can't.

**The fix.** A new field on `VoiceTarget`:
`per_paragraph_override_marker: Option<String>` (default
`Some("<!-- voice-override: minimal -->")`). Any paragraph whose
preceding line contains this marker is excluded from voice scoring.
The marker is invisible in exports (HTML comment).

**Effort:** half a day. **Impact:** prevents the "voice contract
flagged my best paragraph" complaint that will otherwise come from
serious writers within the first week of shipping.

---

## 2. `booksforge-agents/json_repair` — five things to harden

What shipped: `levenshtein()`, `nearest_key()`, `repair_field_names()`,
`parse_and_repair_with_schema_keys()`. Default max distance 2.
Conservative tie-breaking. Audit trail for every rename.

### 2.1 Switch to Damerau-Levenshtein (transposition handling)

**The problem.** Plain Levenshtein treats character transpositions as
two edits (one delete + one insert). The most common keyboard typo
class is *adjacent transposition*:

| Typo | Plain Levenshtein | Damerau-Levenshtein |
|---|---|---|
| `vioce` → `voice` | 2 | 1 |
| `ohter` → `other` | 2 | 1 |
| `nmae` → `name` | 2 | 1 |

At our default `max_distance = 2`, plain Levenshtein still catches
these. But once we *raise* the cap to handle longer typos (item 2.3
below), the difference matters because plain Levenshtein then admits
spurious matches (`vioce`-distance-2 matches both `voice` and `voile`,
for example, while Damerau-Levenshtein cleanly says it's `voice`).

**The fix.** Replace the two-row DP with the standard Damerau-Levenshtein
implementation (one extra check per cell — see *Optimal String Alignment*
distance, the practical version). ~10 LOC delta.

**Effort:** 1 hour. **Impact:** more accurate matching, especially as
we expand the distance cap.

### 2.2 Add Jaro-Winkler as a prefix-weighted second opinion

**The problem.** Field names share prefixes by convention
(`character_name`, `character_role`, `character_arc`). When the model
typos the *suffix*, Jaro-Winkler handles it elegantly; when it typos
the *prefix*, plain edit distance is fine. The current implementation
treats prefix and suffix typos identically, which biases the
"unique nearest" result for field-name-shaped strings.

**The fix.** Compute both Damerau-Levenshtein and Jaro-Winkler
similarity. A candidate qualifies as a match when *both* signals
agree — DL distance ≤ max AND JW similarity ≥ 0.80. This eliminates
the largest class of false-positive renames (e.g.,
`internal_monologue` and `external_obstacle` are both DL distance 6
from `external_objective` — but JW similarity puts only the latter
above 0.80).

**Effort:** half a day. **Impact:** lower false-positive rename rate
when the distance cap is raised.

### 2.3 Length-normalized distance cap (the actual Run #11 fix)

**The problem.** The default `max_distance = 2` *does not* fix the
Run #11 case the test calls out:
`"external_object"` → `"external_objective"` is **distance 4** (suffix
deletion of `-ive`), so the test asserts the rename only happens at
`max_distance = 4`. In production with the default we'd still drop
this card.

**The fix.** Cap on *normalized* distance:
`distance / max(len_a, len_b) ≤ 0.25`. This admits 4-char typos in
18-char field names (4/18 = 22%) and 1-char typos in 4-char field
names (1/4 = 25%) — both reasonable. Hard rejects 5-char distance in
8-char field names (5/8 = 63%, almost certainly a different field).

**Effort:** 30 minutes. **Impact:** the headline Run #11 failure mode
is finally fixed in the default code path. This is the most important
single change in this doc.

### 2.4 snake_case ↔ camelCase normalization

**The problem.** A model trained on JS/TS examples will sometimes
emit `externalObjective` instead of `external_objective`. That's
distance ~10 in raw character form; 0 if you normalize case
conventions first.

**The fix.** Before computing distance, normalize both sides to a
canonical form: lowercase, all separators stripped, all alpha-only.
`external_objective` and `externalObjective` and `External-Objective`
all become `externalobjective`. Compute distance on the canonical
form; rename to the *original* allowed key (preserving the schema's
declared casing) when there's a hit.

**Effort:** 2 hours. **Impact:** removes a class of false negatives
that will start appearing the moment we add a model with
TypeScript-heavy training data.

### 2.5 Per-field re-ask (the second half of priority item #2)

**The problem.** When a required field is missing entirely (not a
typo — actually absent), the agent fails with `"semantic validation
failed: missing field 'X'"` and the orchestrator throws away the
whole proposal. This is wasteful for the same reason field-typo
discard was: 95% of the response was fine.

**The fix.** When validation fails with a missing-required-field error,
emit a one-shot follow-up call to the model: *"Your previous response
was valid except for the missing field `external_objective`. Reply
with ONLY a JSON object `{"external_objective": "..."}` containing
just that one field."* Merge the response into the original. Audit
trail records the patch.

This is harder than the typo fix because it touches the orchestrator
loop, not just the parser. But the failure-recovery payoff is
substantial — especially for the chunked-bible workflow where one
missing field today loses an entire character card.

**Effort:** 2–3 days (orchestrator changes + tests). **Impact:** the
chunked bible's "lenient retry" policy gets a second, much more
targeted recovery option before falling back to full retry.

---

## 3. `booksforge-anti-ai-tells/structural` — six things to harden

What shipped: `detect_anaphora_chain`, `detect_substitution_game`,
`detect_low_variance_paragraphs`, `detect_missing_concrete_noun`,
plus the `structural_tells()` aggregator. All wired into
`tells_per_1000_words` via `find_all_tells()`.

### 3.1 Add a paragraph-level burstiness score (the AI-detection industry standard)

**The problem.** The published AI-detection field is converging on
*burstiness* — the variance / entropy of sentence lengths within a
window — as the single most predictive structural signal. Per
Pangram Labs and GPTZero, modern detectors look at "syntactic tree
depth, discourse coherence patterns, lexical diversity curves, and
**paragraph-level structural signatures**." Our `low_variance`
detector approximates this with a single IQR threshold per
paragraph, but the literature uses a smoother metric.

**The fix.** Add `detect_low_burstiness_paragraphs(text)` —
`burstiness = variance(sentence_lengths) / mean(sentence_lengths)`.
A burstiness < 1.0 across a 5+ sentence paragraph is the same
signal a commercial AI detector would flag. Returns a `TellHit`
with `category = "structural:low_burstiness"`.

Keep the IQR detector (it's a coarse but useful signal). Add
burstiness as the second, finer signal.

**Effort:** 2 hours. **Impact:** brings our detector stack in line
with what the AI-detection industry has converged on, which is what
sceptical readers will use to assess our output quality.

### 3.2 Lemmatize before concrete-noun lookup

**The problem.** The 80-noun list in `CONCRETE_NOUNS` includes
`"hand"` but not `"hands"`, `"stone"` but not `"stones"`. The
tokenizer lowercases but does not lemmatize. So a paragraph using
`"hands"` and `"stones"` heavily registers as having zero concrete
nouns — false positive.

**The fix.** Two options:

1. **Manual plural inflation.** Pre-compute the plural for every
   entry at module init. ~2 hours. Misses irregular plurals
   (already covered in the list — `knife/knives` etc.).
2. **Lightweight stemming.** Trim `s/es/ies` before lookup. ~30
   minutes. False positives for words that legitimately end in `s`
   that aren't plurals, but harmless because the lookup direction
   only triggers on absence.

Take option 2.

**Effort:** 30 minutes. **Impact:** removes the false-positive class
that would surface every time prose uses plurals consistently.

### 3.3 Extend the concrete-noun list to ~250 entries

**The problem.** 80 nouns is too narrow. A scene set in a forest
won't share vocabulary with a coastal scene won't share with an
urban scene. Today the detector biases toward urban-domestic prose.

**The fix.** Extend the list with ~170 more concrete nouns spanning
seven domains: nature (twig, bark, fern, river, gravel, dust,
puddle, leaf), urban (curb, gutter, asphalt, brick, neon), workplace
(file, ledger, stapler, coffee, keyboard), kitchen (knife, salt,
flame, garlic, oil), garage (oil, grease, wrench, bolt), garden
(soil, root, mulch, thorn), weather (mist, frost, hail, sleet).

**Effort:** 2 hours of careful curation. **Impact:** removes most
false positives caused by setting-specific vocabulary.

### 3.4 Substitution game — extend to `[Pronoun] was [Adj]` template

**The problem.** Today's detector matches `[Det] [Noun]
(was|is|held|stood|carried) [Object]`. Misses the equally
mechanical:

- `She was hungry. She was tired. She was alone.`
- `It started to rain. It started to pour. It started to flood.`

These hit a different template (`[Pronoun] [Verb] [Modifier]`) but
exhibit the same slot-fill behavior.

**The fix.** Add a second template — `[Pronoun: she/he/they/it]
[Verb: was/felt/seemed/looked] [State]`. Trigger when the same
pronoun + verb appears 5+ times in the same paragraph with the
state-word varying.

**Effort:** 2 hours. **Impact:** catches the pronoun-driven slot-fill
class in addition to the determiner-driven one.

### 3.5 Discourse coherence detector (the next-bigger structural signal)

**The problem.** Run #11's monotony was visible at one more level our
detectors don't reach: *paragraph-to-paragraph entity continuity*.
The clean prose example in our tests carries a continuous referent
("she" → "the chair" → "Arthur" → "the name") across sentences.
The Run #11 collapsed prose pivoted between abstract concepts
("name", "scream", "stone", "scar") with no shared referent. A
human reader perceives that as "this prose is about nothing in
particular" — the most damning AI fingerprint of all.

**The fix.** Lightweight detector that:
1. Tokenizes each paragraph into nouns + named entities (proper
   nouns only — first-letter-capitalised words excluding sentence
   starts).
2. Computes Jaccard overlap between consecutive paragraphs' entity
   sets.
3. Flags paragraphs where overlap drops to zero AND neither
   paragraph contains a deliberate scene break marker.

Returns `TellHit` with `category = "structural:no_coherence"`.

**Effort:** 1 day. **Impact:** adds a structural signal that operates
above the sentence/paragraph level — the layer where AI-detection
research shows the strongest separator between human and model prose.

### 3.6 Calibration against a known-good corpus

**The problem.** All four current detectors have hand-tuned thresholds
(`chain_len ≥ 4`, `template_count ≥ 5 with ≥ 3 distinct nouns`,
`IQR < 3`, `paragraph ≥ 3 sentences with zero concrete nouns`). None
of these were calibrated against published prose. We don't actually
know whether `chain_len ≥ 4` is too strict for a Robinson paragraph
or too lax for a Lee Child paragraph.

**The fix.** Build a small calibration corpus — 20 published prose
samples spanning literary / commercial / thriller, public-domain or
fair-use excerpts only. Run all four detectors against each and
record hit counts. Adjust thresholds so that *no published-quality
paragraph trips more than one detector*. Save the calibration
results as a regression test.

This is the test that says "our detector doesn't false-positive on
Cormac McCarthy" — without it, the first complaint will be from a
writer whose deliberate style trips our checks.

**Effort:** 1 day (corpus assembly + threshold tuning + regression
test). **Impact:** durable confidence that the detectors aren't
crying wolf.

---

## Priority sequencing — what to do this week

If we ship in the order below, every change after the first is built
on top of an already-improved foundation.

| # | Item | Effort | Why now |
|---|---|---|---|
| 1 | **2.3 Length-normalized distance cap** | 30 min | Headline Run #11 fix — currently the priority #2 work doesn't actually catch its own example case |
| 2 | **1.6 Wire VoiceTarget into bibles + drafter prompt** | 1 day | Without this, all the voice work is dead code |
| 3 | **3.6 Calibration corpus + regression** | 1 day | We need to know we're not crying wolf BEFORE writers see the detector |
| 4 | **1.1 MATTR replacing TTR** | half day | Every comparison we ship today is methodologically wrong |
| 5 | **1.2 Sentence splitter abbreviation gazetteer** | 1 day | Removes 5% noise from every voice metric |
| 6 | **3.1 Burstiness detector** | 2 hours | Aligns with what AI-detection industry has converged on |
| 7 | **2.5 Per-field re-ask** | 2-3 days | Closes the chunked-bible failure-recovery gap |

Items 8+ (the rest of this doc) are real improvements but lower-leverage.
They should land between this list's items as gaps allow, not block them.

---

## What's NOT in this doc (and why)

Several things came up in research that I considered and ruled out:

- **Perplexity-based detection.** Requires running a local LLM at every
  scoring pass. Cost is too high for a real-time linting signal. The
  industry literature confirms perplexity is "easily defeated" by modern
  models anyway.
- **Deep-learning ensemble detector** (the Pangram / GPTZero approach).
  Requires training data we don't have and a model we'd need to ship.
  The structural detectors above approximate ~70% of the signal at
  ~5% of the implementation cost.
- **Contextual abbreviation disambiguation.** Real Punkt-style trainable
  SBD. Worth it later, but the gazetteer in 1.2 closes the gap to ~99%
  with two orders of magnitude less code.

---

## Sources

- *Investigating minimum text lengths for lexical diversity indices* — ScienceDirect ([link](https://www.sciencedirect.com/science/article/abs/pii/S1075293520300660))
- *Estimating lexical diversity using MATTR: Pros and cons* — ScienceDirect, 2024 ([link](https://www.sciencedirect.com/science/article/abs/pii/S2772766124000740))
- *Lexical diversity* — Wikipedia ([link](https://en.wikipedia.org/wiki/Lexical_diversity))
- *Jaro-Winkler vs. Levenshtein in AML Screening* — Flagright ([link](https://www.flagright.com/post/jaro-winkler-vs-levenshtein-choosing-the-right-algorithm-for-aml-screening))
- *Why Perplexity and Burstiness Fail to Detect AI* — Pangram Labs ([link](https://www.pangram.com/blog/why-perplexity-and-burstiness-fail-to-detect-ai))
- *AI Detection in 2026: What's Changed & What's Coming* — UndetectedGPT ([link](https://www.undetectedgpt.ai/blog/ai-detection-2026))
- *Sentence boundary disambiguation* — Wikipedia ([link](https://en.wikipedia.org/wiki/Sentence_boundary_disambiguation))
- *Detection of sentence boundaries and abbreviations in clinical narratives* — Springer ([link](https://link.springer.com/article/10.1186/1472-6947-15-S2-S4))
- *String Similarity Metrics – Edit Distance* — Baeldung ([link](https://www.baeldung.com/cs/string-similarity-edit-distance))
