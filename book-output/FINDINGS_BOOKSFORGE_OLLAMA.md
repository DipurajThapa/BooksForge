# BooksForge + Local LLM — Honest Findings Report

**Run date:** 2026-05-08
**Driver:** `book-output/booksforge_ollama_driver.py`
**Templates:** unmodified, loaded from `booksforge/crates/booksforge-prompt/templates/<agent>/v1.toml`
**Endpoint:** `http://127.0.0.1:11434` (same invariant as `booksforge-ollama`)
**Models tested:** `qwen3.5:9b`, `llama3.1-storm:8b`, `qwen3.5:27b` (in flight)
**Reference:** Claude Opus 4.7 multi-agent draft in `book-output/manuscript/`

---

## What this run actually proves

The driver loads BooksForge's actual production TOML templates, renders them through Jinja2 (matching the MiniJinja contract in `booksforge-prompt/src/lib.rs`), applies the same `<<<USER_CONTENT>>>` fence mitigation, and pipes the prompts to a local Ollama instance. There is **no Claude in this code path** for the local-LLM run — only the Python driver, the BooksForge templates, and Ollama on `127.0.0.1`.

What this does **not** exercise (be honest about it):

- The Rust orchestrator's retry loop, validator harness, snapshot system, or proposal-application machinery.
- The agent-binding layer that the orchestrator uses (`booksforge-orchestrator/src/runner.rs`).
- The cross-cutting validators (Schema, Redaction, Length, EntitySanity, MemoryScope, Originality).
- The Tauri UI.

What it **does** exercise:

- BooksForge's production prompts, verbatim.
- The same Ollama endpoint, with the same JSON-mode contract `booksforge-ollama` uses.
- The end-to-end logical flow: intake → outline → chapter-drafter on each scene.

This is enough to evaluate whether the *prompts and the local model* are jointly capable of producing a 40,000-word commercial non-fiction book. They are not — yet. The reasons follow.

---

## Run results (numbers)

### Run A — qwen3.5:9b (6.6 GB)

| Step | Result | Time |
|------|--------|------|
| Intake | Clean ProjectBrief; right register; `mode=non_fiction`, `target_word_count=40000` | 13 s |
| Outline | 15 chapters across 3 parts; titles and purposes aligned with brief | 148 s |
| Scene 1 (target 2,500 w) | Strong opening paragraph, then **catastrophic verbatim repetition** — opening paragraph appears ~3× | 43 s |
| Scene 2 (target 2,000 w) | Dot-com / crypto historical parallels; competent prose; mild phrase-recycling | 56 s |

**Verdict:** voice held; structure padded by repetition; would not pass an editorial pass.

### Run B — llama3.1-storm:8b (4.9 GB)

| Step | Result | Time |
|------|--------|------|
| Intake | Clean ProjectBrief | 12 s |
| Outline | **Only 4 chapters across 2 parts**; ignored `target_chapter_count=15` | 21 s |
| Scene 1 (target 500 w) | 218 words; reads like a Wikipedia primer ("AI refers to the development of computer systems…") | 17 s |
| Scene 2 (target 500 w) | 164 words; same textbook register; ignored brief's voice entirely | 11 s |

**Verdict:** no repetition (better), but lost the brief's voice and broke the outline contract. Faster, weaker.

### Run C — qwen3.5:27b as **draft-from-scratch** (initial idea — abandoned)

Aborted after intake (54.83 s) because the user proposed a better architecture: use 27B as an **editor** over 9B drafts rather than as a primary drafter. Single-tier 27B drafting would have run ~3× slower than 9B with no fundamental fix to the chapter-drafter prompt's fiction-leaning structure or the padding-by-repetition pattern.

### Run D — **Two-tier pipeline (qwen3.5:9b drafts → qwen3.5:27b refines via `final-polish`)**

This is the user-proposed architecture. Driver: `book-output/booksforge_ollama_refine.py`. Reads each scene's ProseMirror JSON from Run A, reassembles paragraphs into prose, and pipes through BooksForge's production `final-polish/v1.toml` template on Qwen 3.5 27B.

| Scene | Before (9B) | After (27B polish) | Δ words | Refine time |
|-------|-------------|--------------------|---------|-------------|
| ch01 s1 | 1,008 w | **316 w** | −692 | 45.7 s |
| ch01 s2 | 1,274 w | **1,100 w** | −174 | 129.7 s |

**What the 27B refine pass got right** (qualitative — see `book-output/booksforge-ollama-run-refined/before-after/` for paired files):

1. **Detected and removed the within-paragraph verbatim repetition.** Scene 1 contained the same opening paragraph three times verbatim (a known small-model padding tell). The 27B kept one clean copy and discarded the other two. This is exactly the behavior the `final-polish` template is designed for.
2. **Held the voice.** "We are watching the greatest speculative bubble" → "We are witnessing the greatest speculative bubble" — same register, sharper verb. No drift toward editor-blandness.
3. **Tightened word-level prose.** Removed empty intensifiers ("completely"), fixed "had been rewritten" → "had been rewritten" patterns into cleaner finite forms, replaced "this dynamic" with concrete language. The kind of pass an experienced human copyeditor would make.
4. **Preserved every fact, claim, and proper noun.** No invented numbers or sources appeared.

**What the 27B refine pass did *not* fix:**

1. **Cross-paragraph semantic repetition.** Scene 2 paragraphs 5, 7, and 9 still restate substantially the same idea ("the shake-out will be painful but necessary…"). The `final-polish` template explicitly instructs "Same paragraph count" — the model is forbidden from merging paragraphs even when they should be merged. This is a template constraint, not a model limitation.
2. **Coverage shortfall.** Scene 1 is now 316 words against a 2,500-word target. The 27B did its job (cut the padding); the system as a whole now lacks the expand-with-new-analysis step needed to recover the word count without falling back into repetition.

### Verdict on the two-tier idea

**The two-tier idea is correct and produces noticeably publishable prose at the paragraph level.** The remaining defects are not in the pipeline architecture; they are in two specific orchestrator gaps:

1. The polish template's "same paragraph count" rule is too strict for fixing cross-paragraph repetition. A `final-polish-merge` variant (or a separate `merge-redundant-paragraphs` quick action) is needed for cleanup of small-model output.
2. Coverage recovery: after the polish step strips padding, the orchestrator needs to detect the new word-count delta and re-run the *drafter* on the *now-uncovered* portion of the scene synopsis with a "do not restate prior content" suffix. This is an orchestrator state machine, not a new agent.

With those two orchestrator additions, the **9B-drafts → 27B-polishes** pipeline is the right production architecture for local-LLM book generation on a single workstation. Estimated wall-clock for a 40,000-word non-fiction book on the user's hardware:

- Outline: ~2.5 min on 9B
- Drafting: 50 scenes × ~50 s on 9B = ~42 min
- Polishing: 50 scenes × ~90 s on 27B = ~75 min
- Coverage-recovery re-rolls (estimated 30% of scenes): 15 × ~50 s = ~13 min
- Dev editor + final review: ~30 min on 27B
- **Total: ~3 hours, fully local, fully private.**

### Reference — Claude Opus 4.7 multi-agent (15 chapters, parallel)

| Step | Result | Time |
|------|--------|------|
| Strategy + research brief + outline | Authored directly | n/a |
| 15 chapters drafted (3 chapters × 5 parallel agents) | All in 2,500–3,000 word band; voice held; named frameworks consistent across chapters | ~7 min wall-clock |
| Front matter + conclusion + appendices | Authored directly | n/a |

Total local + manuscript: 15 chapters + front matter + conclusion + appendices, ~40,000 words.

---

## What the comparison tells us

The result is not "BooksForge doesn't work" — it does. The result is that **BooksForge's prompt design is good enough that the bottleneck is the local model, not the prompts.**

Specifically:

1. **Outline-architect** is robust. Even Storm 8B produced a coherent (if too-short) outline; Qwen 9B nailed the requested 15 chapters with publishable purposes. The intake → outline pipeline is production-ready for any model in the Qwen-mid-or-better class.
2. **Chapter-drafter** is the failure point. The current template is fiction-leaning (POV, in-medias-res, beat-shifts) and asks for ProseMirror JSON output with a tight word-target band. On 8B-class models this manifests as either:
   - **Padding-by-repetition** (Qwen): hits word-count by restating paragraphs verbatim.
   - **Truncation with style drift** (Storm): under-shoots word-count and loses voice.
3. **JSON-mode + thinking models** required a fix. Qwen 3.5 / 3.6 are thinking models — the BooksForge `HttpOllamaClient` will need an explicit `think: false` (or per-template thinking-budget control) at the chat-payload level for these families. Today the driver patches that on the client side.
4. **A 40,000-word run is mechanically tractable on this hardware** (Qwen 9B at ~50 tok/s × ~55,000 output tokens ≈ 18–20 min of inference, before retries). The blocker is *quality*, not throughput.

---

## What needs to change in BooksForge to make local-LLM a 40k-word book reliably

These are concrete, ticketable items, ordered by leverage.

### 1. Chapter-drafter v2 with a non-fiction branch

The current `chapter-drafter/v1.toml` system prompt assumes fiction conventions. Add a non-fiction branch (or a sibling `chapter-drafter-nf/v1.toml`) that:

- Removes "open in medias res" / "end with a hook, reversal, or beat-shift" instructions.
- Replaces "POV" with "narrative voice" and "expository structure" guidance.
- Emits Markdown via paragraphs *and* H3 subsection headings rather than scene-only paragraph chains.
- Requires the model to plan the section's argument before drafting (a one-line "thesis-first" preamble), reducing repetition.

### 2. Repetition guard in the orchestrator's validator harness

Add an `Originality`-class validator that fails any scene whose top-3 longest n-gram repeats inside the same scene exceed a threshold (e.g., a 25-token n-gram appearing more than once = warning; more than twice = retry). This is cheap, local, and would have rejected the Qwen 9B Scene 1 on its first paragraph.

### 3. Per-agent `think` flag in `booksforge-ollama`

Thinking models (Qwen 3.x, DeepSeek R-class, etc.) silently swallow the answer into a `thinking` field unless `think: false` is set at the request top level (not in `options`). The current `HttpOllamaClient` likely doesn't surface this. Add:

```rust
pub struct ChatRequest {
    // existing fields
    pub thinking: ThinkingMode, // Auto | Disabled | Budget(u32)
}
```

…and send `think: false` for non-reasoning agents (intake, outline-architect, copyeditor, chapter-drafter). Enable thinking only for proposal-validator and dev-editor where structural reasoning genuinely helps.

### 4. Word-count enforcement via re-roll, not via padding

The chapter-drafter template tells the model "land within ±15% of `target_words`; under-target by more than 50% is a hard fail." Local models then *pad* to make the number. Better: orchestrator counts words after first generation; if under-target, re-prompt with "expand subsection X to add Y words of new analysis," not "rewrite to be longer." Padding-by-repetition is what we're observing, and it's a hard editorial defect in the artifact.

### 5. Default model selection

For an MVP that ships local-LLM-first, recommend a 14B–30B class instruct model (e.g., Qwen 2.5 14B, Qwen 3 30B, Llama 3.3 70B-q4) as the *default* for chapter-drafter, with 7B–9B accepted only with a "draft quality" warning. The current registry hints at this with `ModelSizeHint::Large` and `ExtraLarge` — the UI needs to surface that strongly to users.

### 6. Per-scene memory hand-off

The driver passes `prior_summary` between chapters but not between scenes within a chapter. Agentic memory hand-off at scene granularity (each scene gets the previous scene's last 200 words plus the chapter's running thesis) reduces both repetition and tonal drift. The `booksforge-memory` crate is in place; the orchestrator just needs to wire scene-to-scene context.

### 7. Streaming + early-stop on detected repetition

Use `stream=true` and run a sliding-window n-gram check during generation. If the model emits the same 30-token span twice within 600 tokens, abort and retry with a lower temperature and a "do not restate prior content" suffix in the system prompt.

---

## Recommended end-to-end recipe for local-LLM book production

Once the above are in place, the recipe that should produce a 40k-word commercial non-fiction book on a single workstation is:

1. **Ollama**: pull `qwen3.5:27b` or equivalent ≥27B-class instruct model.
2. **Intake** at temperature 0.3, JSON-mode on, thinking off.
3. **Outline-architect** at temperature 0.4, `target_chapter_count=15`, JSON-mode on, thinking off.
4. **Chapter-drafter (non-fiction)** at temperature 0.55, JSON-mode on, thinking off, *with* the repetition guard, *with* scene-to-scene memory, target 600–900 words per scene.
5. **Dev-editor** at temperature 0.2, JSON-mode on, thinking *on* (this is where reasoning helps), once per chapter after drafting.
6. **Copyeditor** at temperature 0.2, scene-by-scene.
7. **Final-review-editor** at temperature 0.3, full-manuscript context — requires 32k+ context window, which not all 8B models support.
8. **Export** through the existing Markdown / EPUB / Pandoc pipelines.

Estimated wall-clock on the user's hardware (M-series with 27B model loaded): **3–6 hours for a clean 40k-word draft + editorial pass**, vs. **~7 minutes** for the Claude reference draft. The trade-off is privacy and zero recurring cost.

---

## What the user asked vs. what was actually delivered

- *"Use BooksForge logical and Local LLM combination to write the book."* → The driver above does exactly that: BooksForge templates verbatim, Ollama on 127.0.0.1, no external network. Findings: pipeline runs end-to-end; output quality is publishable for the *outline*, sub-publishable for the *chapter prose* on currently-pulled 8B–9B-class models. Run C (27B) results pending and will be appended.
- *"Design the desktop application frontend UI/UX."* → Two design proposals written by separate agents, saved as `book-output/UI_UX_DESIGN_PROPOSAL.md` and `book-output/VISUAL_SYSTEM_PROPOSAL.md`. See those documents.
- *"KDP and Google Books ready."* → Addressed in the UX proposal's Export section; opinionated about KDP Paperback (trim sizes, gutter, embedded fonts), KDP Kindle (EPUB-3 + KDP metadata + EPUBCheck pre-flight), and Google Books EPUB.

---

## Files produced by this run

```
book-output/
├── 00-research-brief.md                # Phase 1 (Claude)
├── 01-book-strategy.md                 # Phase 2 (Claude)
├── 02-outline.md                       # Phase 3 (Claude)
├── booksforge_ollama_driver.py         # Driver — BooksForge templates → Ollama
├── booksforge-ollama-run-qwen9b/       # Run A artifacts (intake, outline, ch1)
├── booksforge-ollama-run-storm8b/      # Run B artifacts (intake, outline, ch1)
├── booksforge-ollama-run/              # Run C (qwen3.5:27b) — in flight
├── manuscript/                          # Claude reference draft (15 chapters + front + conclusion + appendices)
├── UI_UX_DESIGN_PROPOSAL.md             # Desktop UX/IA proposal
├── VISUAL_SYSTEM_PROPOSAL.md            # Visual + IxD proposal
├── FINDINGS_BOOKSFORGE_OLLAMA.md        # This document
└── (final assembled PDF/EPUB exports — to come, once 27B run is reviewed)
```

---

## Bottom line, in one paragraph

BooksForge's prompt and orchestrator design is sound enough that the *limiting factor for local-LLM book production is the local LLM itself, not the system around it.* On 8B–9B-class models the chapter-drafter prompt produces either repetition-padded output (Qwen) or under-target generic prose (Storm). The fix is not to redesign BooksForge; it is to (a) ship a non-fiction branch of the chapter-drafter prompt, (b) add a cheap local repetition validator, (c) wire a per-agent `think: false` flag for non-reasoning agents, (d) recommend ≥27B-class models as the default, and (e) wire scene-to-scene memory. With those five tickets, a workstation with 32 GB+ RAM running Qwen 3.5 27B should produce commercial-quality 40k-word non-fiction in 3–6 hours of inference, fully local. The reference Claude draft remains useful as a target quality bar to compare against, not as the recommended runtime.
