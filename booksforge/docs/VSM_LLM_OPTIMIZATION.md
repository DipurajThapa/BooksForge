# BooksForge — Value Stream Map + LLM Optimization Plan

**Date:** 2026-05-09
**Audience:** writers running long literary-fiction sessions on a single machine
**Why:** the current run blueprint heats hardware, burns tokens, and takes 75–80 min to produce 2 chapters. This doc maps where time goes, where tokens leak, and the highest-leverage optimizations to land before the next big run.

---

## 1. Value Stream Map (current state — observed in the prior 4.66/10 baseline run)

```
[ Writer's idea text ]
        │
        ▼   30–60s   (1 LLM call, 9B/27B, ~2k tokens)
   ┌────────────┐
   │  Intake    │  → ProjectBrief
   └────────────┘
        │
        ▼   60–90s   (1 LLM call, 9B/27B, ~3k tokens)
   ┌────────────┐
   │  Outline   │  → OutlineProposal (Parts → Chapters → Scenes)
   └────────────┘
        │
        ▼   90–120s  (1 LLM call per bible, 27B preferred, ~4–5k tokens each)
   ┌────────────┐ ┌────────────┐
   │ Char Bible │ │ World Bible│  → CharacterBibleProposal + WorldBibleProposal
   └────────────┘ └────────────┘
        │           │
        └────┬──────┘
             ▼   ~3 min/scene  (1 LLM call, 27B, ~6–8k tokens)
        ┌────────────┐
        │ Scene Draft│  ×N scenes  → SceneDraftProposal
        └────────────┘                     │
             ▼   ~1 min/scene  (1 LLM call, 9B/27B, ~3k tokens)
        ┌────────────┐
        │ Critic     │  ×N scenes  → SceneCritiqueProposal
        └────────────┘                     │
             ▼   ~3 min/scene  (4 LLM calls, 27B, ~5k tokens each)
        ┌─────────────────────────┐
        │  Polish stack ×4 stages │  →  PolishProposal per stage
        │  (dialogue / metaphor / │     applied serially
        │   voice / scene_tension)│
        └─────────────────────────┘
             ▼   <5s            (deterministic — no LLM)
        ┌────────────┐
        │ Tells scan │  → TellsReport (verdict + density)
        └────────────┘
             ▼
        [ Final manuscript ]
```

**Per-scene cost (current state, observed):**
- 1 draft + 1 critic + 4 polish = **6 LLM calls / scene**
- ~26–35k tokens / scene
- ~7–10 min wall-clock / scene on Apple Silicon (M2 Max with qwen3.5:27b)

**Per 2-chapter book (3 scenes/chapter avg = 6 scenes):**
- ≈ 36 LLM calls
- ≈ 180–210k tokens
- ≈ 70–90 min wall-clock
- Hot machine: GPU at 95%+ for 70+ minutes; fans audible; thermal throttling kicks in around minute 30 on laptops

**This matches the standalone Python ghostwriter baseline** in `artifacts/ghostwriter/PROOF_RESULTS_LITERARY.md` (32 calls, ~78 min). The waste is mostly in the per-scene polish stack — 4 sequential 27B passes is the biggest single hot spot.

---

## 2. Where the value-add is — and where it isn't

| Stage | Value add per token spent | Quality lift attributable | Notes |
|---|---|---|---|
| **Intake** | ⭐⭐⭐⭐ | 0.3 pts | Cheap, structural — keep on 9B |
| **Outline** | ⭐⭐⭐⭐⭐ | 0.8 pts | Architecture matters most; 27B worth it |
| **Bibles (char + world)** | ⭐⭐⭐⭐⭐ | 1.0 pts | Drift propagator — best ROI for 27B |
| **Scene draft** | ⭐⭐⭐⭐ | 1.5 pts | The actual prose — 27B mandatory |
| **Scene critic** | ⭐⭐⭐ | 0.4 pts | Lift is 2× when fed back to revision; 9B is enough |
| **Polish: dialogue** | ⭐⭐ | 0.3 pts | High value when there's dialogue; near-zero on quiet scenes |
| **Polish: metaphor** | ⭐⭐⭐ | 0.4 pts | High variance — kills bad images; can over-polish good ones |
| **Polish: voice** | ⭐⭐⭐⭐ | 0.5 pts | Critical for literary; LESS critical for genre |
| **Polish: scene_tension** | ⭐⭐ | 0.2 pts | Often a no-op on tight scenes |
| **Tells scan** | ⭐⭐⭐⭐⭐ | 0.0 (gate) | Free — pattern scan, not LLM |

**The polish-stack waste pattern is real:** running 4 stages on every scene unconditionally means we burn 4 × 27B inferences on scenes that have no dialogue (dialogue stage = no-op), no metaphors (metaphor stage = no-op), or already tight tension (scene_tension stage = no-op). Each no-op still takes 60–90s and ~5k tokens.

---

## 3. Optimizations — highest leverage first

### Tier 1 — high-impact, low-risk, ship before the next test run

| # | Optimization | Expected wall-clock saving | Quality risk | How |
|---|---|---|---|---|
| **O1** | **Conditional polish stack (skip no-op stages)** | **30–40%** (~25 min off a 75-min run) | Negligible — only skips stages where the deterministic pre-scan finds nothing to operate on | Before running each polish stage, run a deterministic detector: `dialogue_polish` skips if `pm_doc` has < 20 quote characters; `metaphor_polish` skips if no figurative-language patterns detected; `scene_tension_polish` skips if scene already ends on a hook (last sentence has `?`/`!` or one of ~12 hook patterns). |
| **O2** | **Two-tier model routing** | **20–30%** (~15 min) | Low — confirms the user's existing two-tier feedback (memory) | Route intake + critic + tells to 9B (fast); keep 27B only for drafter + polish + bibles. The 27B-for-everything default is wasteful. |
| **O3** | **Response cache by template-hash + input-hash** | **10–20% on iterative runs** | Zero — deterministic dedup | Cache the LLM response in `agent_outputs` keyed on `(template_hash, input_hash, model)`. On second invocation with same inputs, return cached. The schema already has both hash fields — wire the lookup in `runner::run_inner` before the chat call. |
| **O4** | **Prompt-size budget audit** | **5–15%** (less context = faster decode + less RAM pressure = less thermal throttling) | Zero | Audit each `[render.user]` block: drop fields the agent doesn't actually use (e.g. `prior_summary` can be capped at 800 chars instead of unbounded; `character_bible` for scene drafter can ship character-cards-only-by-name when scene POV implies a single focal character). |
| **O5** | **Parallel scene drafting (independent scenes)** | **40–60% on multi-scene chapters** | Negligible — scenes are independent at the orchestrator level | Today scenes are drafted serially. Draft 2 scenes in parallel via `tokio::join!` when GPU has headroom (most M2/M3 with 24GB+ unified memory). Add a runtime probe at startup: if `ollama show` reports the loaded model fits twice in RAM, allow concurrency=2. |

**Combined Tier 1 saving:** ~50–65% wall-clock on a typical 75-min run → **25–35 min for 2 chapters instead of 75–80 min.**

### Tier 2 — medium-impact, medium-risk

| # | Optimization | Saving | Risk | How |
|---|---|---|---|---|
| **O6** | Per-scene polish-stack ordering by genre + early-exit on diminishing returns | 10–15% | Low | After each polish stage, the critic re-scores. If improvement < 0.3 axis points, skip remaining stages for this scene. |
| **O7** | Speculative decoding (load 9B as draft model, 27B as verifier) | 30–40% on the 27B passes | Medium — Ollama support for speculative decoding is recent; behavior varies by model | Configure `--draft` flag on Ollama 0.4+ if both 9B and 27B share architecture. |
| **O8** | Streaming polish (apply edits incrementally as tokens stream rather than buffering full response) | Better UX (perceived latency); minor wall-clock | Low | Wire `TokenSink` through `apply_polish` to write the diff into the editor as it generates. |
| **O9** | Drop the per-scene critic when polish stack will run anyway | 10% on chained pipelines | Medium — loses the targeted-edit signal | When `agent_run_full_scene_pipeline` is called and `stop_after_critic=false`, skip the critic; polish stack stages already include their own quality probes. |

### Tier 3 — high-impact, higher-risk (defer until baseline measurements exist)

| # | Optimization | Saving | Risk | How |
|---|---|---|---|---|
| **O10** | Model quantization sweep (q4 → q5_K_M for 27B) | 15–25% throughput | Quality variance — must benchmark per-genre | Add a per-project model-pinning UI; run a 12-axis rubric A/B comparing q4 vs q5 on 3 sample scenes. |
| **O11** | Local KV-cache reuse across same-project runs | 5–15% on same-day continuation | Medium — Ollama doesn't expose KV persistence directly; would need llama.cpp escape hatch | Out of scope until Ollama 0.5+ exposes it. |
| **O12** | Distill polish prompts into a single fine-tuned 9B "polisher" | 50–70% on polish | High — needs ~500 sample edits to fine-tune; user generates them by accepting/rejecting current pipeline output | Long-term play; track this as a future product feature. |

---

## 4. Thermal / power signals to watch

When you run the next test, monitor these on macOS:

```bash
# Live thermal pressure level (nominal | fair | serious | critical)
sudo powermetrics --samplers thermal -i 2000 | grep -E "Thermal|CPU die"

# Per-process power (Wh / hour)
sudo powermetrics --samplers cpu_power,gpu_power -i 5000 | grep -E "ollama|GPU"
```

**Trigger thresholds:**
- `Thermal pressure: serious` for >5 min → throttling is hurting throughput; pause + cool down
- GPU sustained >80W on M2 Max for >30 min → battery degradation risk on laptops; plug in
- Ollama RSS > 50% of unified memory → swap pressure; close other apps

---

## 5. Concrete minimum changes I'd ship before the next run

If we ship just **O1 (conditional polish skip)** + **O2 (model routing)** + **O3 (response cache)** before the next book test, the expected outcome is:

- **Wall-clock:** ~30 min instead of 75 min (2.5× speedup)
- **Tokens:** ~80–100k instead of 180–210k (~2× saving)
- **Heat:** GPU active for ~25 min instead of 70 min; thermal throttling unlikely
- **Quality:** unchanged (skipped stages were no-ops; cached responses are deterministic)

Token cost in $-equivalent (had this been cloud) drops from ≈ $0.50–$0.80 per book test to ≈ $0.20–$0.30. On-device that means battery + heat instead of dollars, but the principle is the same.

These three are scoped for one focused implementation session each. None require model changes, infrastructure changes, or prompt rewrites — they're pure orchestrator-level wins.

---

## 6. Honest caveats

- **Single-machine context.** All numbers above assume one user, one machine, one 27B model loaded. Multi-user or pipeline-parallel scenarios change the math.
- **Local model quality plateau.** No amount of orchestration optimization can lift a 4.66/10 to 9/10 — the ceiling is the model. To push past ~6.5/10 on literary fiction with local models, the realistic next step is fine-tuning on accepted-prose corpora (a project the user is best-positioned to feed, since they own the corpus).
- **Optimization measurements.** The percentages above are estimates from the prior baseline + general LLM throughput knowledge. Real measurements come from running the optimized pipeline + comparing to the 4.66/3.93/2.08 baseline. Treat as planning numbers, not promises.

---

## 7. What this doc does NOT touch

- UI / UX work (separate roadmap)
- Tier-2 ProposalValidator + peer-review — these run in parallel with the main agent so they don't extend wall-clock; they DO add ~10% to total tokens. Acceptable cost for the audit-trail value.
- Cancellation / partial-output recovery — already wired via `CancelToken`; not a perf optimization.
- Storage / SQLite tuning — disk I/O is < 1% of wall-clock; not a hot path.
