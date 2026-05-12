# Out-of-the-Box Ideas for Pushing BooksForge Output Quality

**Audience:** the BooksForge team. Companion to [`RCA_QUALITY_6_TO_9.md`](RCA_QUALITY_6_TO_9.md), which covers the conventional architecture lifts (fiction agents, per-scene critique-revise, specialist polish stack, multi-specialist scoring). This document is for the *non-obvious* moves — the ones a normal product roadmap would not contain.

Every idea is ranked by an honest **leverage / cost / risk** rubric. I have flagged the ones I would actually bet money on with **[BET]**.

---

## Tier 1 — High leverage, low-to-medium cost (BET)

### 1. **[BET] Ensemble drafting + critic-as-selector** (already in `ghostwriter/pipeline.py`)
Generate the same scene N=2-4 times at different temperatures, let the per-scene critic pick the best. Costs N× compute per scene — for ghostwriters charging $0.10–$1.00 per word, N=3 is trivial; for free local-only users, expose the knob.

**Why it works:** local 27B at temperature 0.55 vs 0.70 vs 0.85 produces materially different drafts. The critic, scoring on the genre's craft axes, can recognise which one is best. This compounds: a critic-selected scene is ~0.5 pts better on the rubric than a single-shot scene of the same model.

**Cost:** N× draft tokens per scene; one extra critic call per scene. For an 8-chapter book at 27 scenes, that's ~50 extra LLM calls, ~25 minutes wall-clock on this hardware. Real money for free users; nothing for ghostwriters.

**Risk:** the critic sometimes picks the most *correct* draft over the most *interesting* — if the critic prompt is mediocre, ensemble degenerates to averaging. Mitigated by tuning the critic prompt per genre.

### 2. **[BET] Voice fingerprint as numeric constraint** (already in `ghostwriter/voice_fingerprint.py`)
Instead of "write in the style of X," give the drafter a numeric profile — *median sentence length 11 words, IQR 6-22, dialogue share 18%, em-dash density 1.2 per 1000 words, type-token ratio 0.78*. LLMs respect numeric constraints surprisingly well; they ignore vibes-prompts.

**Why it works:** "literary voice" is unmeasurable; "median sentence length 11 with 22% short sentences" is measurable. The drafter can self-check against the constraint. The scorer can verify post-hoc using stylometric distance.

**Cost:** ~50 lines of code + one extra prompt block. **Free.**

**Risk:** the user has to paste 3-5 paragraphs of comp samples to bootstrap. If they paste their own bad first-draft prose, the system will faithfully reproduce it. Mitigated by an explicit warning + a curated default for genres without samples.

### 3. **[BET] Anti-AI-tells dictionary + targeted rewrite** (already in `ghostwriter/anti_ai_tells.py`)
Don't ask the polisher to "make it sound less like AI" (it can't — AI cannot reliably introspect). Instead, run a regex pass over the draft, find the actual fingerprints (delve, tapestry, "It's important to note that," em-dash overuse, "blood ran cold," marketing triplet adjectives), and feed them to the polisher as a list of *spans to rewrite, leaving everything else alone*.

**Why it works:** AI fingerprints are concentrated in a small, measurable set of patterns. Targeted rewrite preserves the rest of the prose; vague "remove AI-isms" prompts almost always make things blander.

**Cost:** ~150 lines of regex + scoring. **Free.**

**Risk:** false positives (legitimate use of "delve" in a digging-the-garden scene). Mitigated by severity tiers + a context-aware whitelist hook.

### 4. **[BET] One accepted scene becomes the next scene's voice anchor**
After the user accepts chapter 1, automatically use it as a few-shot example in chapter 2's drafter prompt. After 3 accepted chapters, use a sliding window of the most recent 1-2 as the live voice anchor — the comp samples become the *initialisation*, the accepted prose becomes the *continued anchoring*.

**Why it works:** voice consistency across a 50k-word book is the #1 thing local LLMs lose. Re-anchoring on every chapter prevents drift. Compounds with idea #2.

**Cost:** modify the drafter-prompt assembler. ~30 lines. **Free.**

**Risk:** if chapter 1 was a stinker the user accepted under duress, chapter 2 inherits the badness. Mitigated by: only use accepted prose with a critic score ≥7.

---

## Tier 2 — Medium leverage, higher complexity

### 5. Stylometric distance as a *gate*, not just a score
Today the system reports "manuscript scores 6.8 on rubric." Add: "manuscript is at stylometric distance 4.2/10 from your comps." Then refuse to ship the chapter unless distance ≥7. This forces the polish loop to keep working until the prose actually resembles the target.

**Cost:** wire `stylometric_distance` into the orchestrator's accept gate. ~50 lines. **Free.**

### 6. Per-character voice dictionaries (different from author voice)
The author has one voice. Each character has another. Build per-character vocabulary lists (preferred words, banned words, sentence-length distribution) from accepted dialogue lines. Inject into the dialogue-polish stage so the dialogue editor enforces them.

**Why it works:** the #1 dialogue tell — every character sounding like the same author talking through different masks — falls apart immediately when each character's lines are forced into a different numeric profile.

**Cost:** new module + table + dialogue-polish template extension. ~1 day.

### 7. Reverse-engineer the genre comp's beat sheet, then write to it
Don't only fingerprint comp prose — fingerprint comp **structure**. From a comp blurb / outline / first-chapter sample, extract: chapter-level beat (setup / catalyst / etc.), pacing rhythm, hook style, opening-line type, ending-line type. Feed this as a target structure into the orchestrator.

**Cost:** new "comp structure analyser" agent. ~3 days. Tier 2 because it requires careful prompt engineering.

### 8. Adversarial drafting (model A drafts; model B critiques as if it were a hostile review-bot)
The "hostile critic" prompt — *"You are a prickly Goodreads reviewer who hates AI-generated books. Find every reason to one-star this."* — produces a critique very different from a craft critic. The two together cover much more ground than either alone.

**Cost:** add a second critic role. **Free.** Tier 2 only because the orchestrator needs to merge two critique streams.

### 9. Sentence-class diversity enforcer
LLMs love patterns. Within 2k words they will repeat the same sentence shape (subject-verb-object, length 12-14, no clauses) until it is the only shape in the chapter. Add a sentence-class diversity scorer (count: simple / compound / complex / fragment, plus length-bucket distribution). Reject drafts whose distribution is degenerate.

**Cost:** ~80 lines of analysis + a regen trigger. **Free.**

### 10. "Did the protagonist do anything?" check
For genre fiction especially, scene-level agency is the #1 silent failure. After each scene, ask: *did the protagonist make a choice that mattered?* If no, regen the scene with explicit instruction to insert a choice + consequence.

**Cost:** one extra critic call per scene. ~30 lines.

---

## Tier 3 — High leverage, high cost or high risk

### 11. Multi-author ensemble (different LLM families)
Today everything runs on Qwen. A local Llama-3.1-storm-8B + Qwen-3.5-27B + DeepSeek-r1-distill ensemble would produce more diverse drafts → critic has better candidates to pick from. The 8B is already installed on this machine.

**Cost:** 2× compute on draft passes; orchestration complexity. **Tier 3** because the ROI vs single-family ensemble is uncertain — we should A/B test before adopting.

### 12. Fine-tuned local model on accepted human-edited prose
Once a ghostwriter has accepted ~50,000 words of polished prose, fine-tune a small LoRA on it. The next book in the same author's voice gets the LoRA loaded. **This is the closest local LLMs can come to actually learning a voice.**

**Cost:** real engineering — LoRA training pipeline, model versioning, per-project model storage, hardware time. Medium-to-large project. **Tier 3** because most ghostwriters won't write enough in one voice to benefit; for those who do (series writers, house pseudonyms), the lift is substantial.

### 13. RAG over the author's prior published work
If the author has prior published novels, embed and retrieve the closest 2-3 paragraphs as voice anchors per scene. Better than static comp samples because it scales with the corpus.

**Cost:** local embedding model + vector store + retriever. ~1 week.

### 14. Counter-AI dataset distillation
Maintain a small curated dataset of "what AI does badly vs how humans actually write," and use it as a few-shot prefix in every drafter prompt. **The dataset is the moat** — once curated, every BooksForge user benefits, and the curation can be community-driven.

**Cost:** ongoing curation work, plus prompt-context budget. **Tier 3** because it requires sustained editorial work, not a one-shot build.

### 15. Reading-level + cadence reject loop
Compute Flesch-Kincaid, average sentence length, syllable-per-word, and lexical diversity per chapter. Compare to genre baselines (literary fiction has FK ~9-11, YA ~6-8, business non-fiction ~10-12). Reject drafts whose metrics miss the genre target by more than 1.5 SD and regen with explicit metric targets in the prompt.

**Cost:** ~50 lines + a textstat-equivalent. **Free.** Tier 3 only because the genre baselines need careful calibration.

---

## Tier 4 — Speculative / R&D bets

### 16. Reflection prompts in two voices
Have the drafter write the scene, then reflect *as the author* ("what was I trying to do here? what did I miss?"), then reflect *as the protagonist* ("what was I really feeling? what did I leave unsaid?"). Both reflections feed the revision pass. Often surfaces interiority that the draft missed.

### 17. Scene-card → scene-card chess
Treat the outline as a game tree: for each scene, generate 3 alternative scene-card variants (different reveals, different conflicts), simulate one paragraph of each, let the critic pick the variant that best advances the chapter. Then commit to that variant.

### 18. Anti-pattern lottery
Maintain a banned-pattern list per project. Every accepted scene contributes to the list (n-grams the reader actually noticed, words that turned out to be tics). Future scenes are auto-rejected if they introduce tokens from the list at >2× baseline rate.

### 19. Voice-DNA splice
Take the user's comp sample and the project's accepted prose, splice them at the sentence level into a "DNA strip" (alternating one sentence each), and feed THAT as the few-shot example to the drafter. Forces the drafter to reconcile two voices into one, which often produces something fresher than either alone.

### 20. The "Ghost of Hemingway" pass — explicit anti-style transfer
Final-stage pass: rewrite the chapter in the style of a deliberately-different author (Hemingway, McCarthy, Ishiguro), then port the prose-level moves back to the author's voice. The dual translation often discovers fresh sentences.

---

## What I would NOT recommend

- **A vague "make it more literary" polish prompt.** "Literary" isn't a measurable target. The polisher will respond with longer sentences and more adjectives, both of which usually make things worse.
- **Chain-of-thought "let me think step by step" wrappers in the drafter.** Helps for analysis tasks; flattens voice in prose. Use it in critics, not in drafters.
- **Bigger models alone.** 70B+ helps at the margin, but doesn't fix the architecture problem. A well-orchestrated 27B beats a poorly-orchestrated 70B for fiction at this scale.
- **More polish passes past 4-5.** Diminishing returns kick in fast. By the 6th polish pass, you are sanding off voice.
- **"AI detector evasion" optimisation.** AI detectors are noisy and vendor-specific; chasing their scores doesn't make prose better, just makes it different.

---

## What "the right experiment" looks like

For each idea above ranked Tier 1 or Tier 2, the proof-of-value experiment is the same shape:

1. Pick a 2-3 chapter scope from a real book project.
2. Run the existing pipeline → record honest score + stylometric distance.
3. Run the proposed pipeline (with one new idea added) → record honest score + stylometric distance.
4. Show the diff to a real human reader (the author, ideally) and ask: *which would you ship?*

The reader's ship-vote, not the rubric score, is the ground truth. Rubric scores are a proxy; the proxy is calibrated against the ship-vote, not the other way around.

This is also why I would build the **author-feedback capture** loop early: every accept / reject / "regen with notes" event is a data point that calibrates the rubric and identifies where the architecture is hallucinating quality.
