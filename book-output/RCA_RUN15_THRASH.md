# RCA — Run #15 multi-chapter timeout (the model-context-thrash problem)

**Date:** 2026-05-10
**Trigger:** the multi-chapter run hit the 30-min orchestrator wall-clock cap on its very first drafter call. Twice in a row.

This document is a *systemic* RCA, not a per-symptom patch list. It traces every recent failure to one root cause, and prescribes the fix sequence that actually addresses it.

---

## 1. The symptom log (what we kept seeing)

| Run | Symptom | Surface "fix" applied |
|---|---|---|
| #11 | drafter returned empty (0 words) | raised `num_predict` 6k → 16k |
| #12 | bimodal collapse (84% short / 0% medium / 16% long) | added `MANDATORY INTERLEAVING` directive |
| #13 | drafter produced 28 min of bare prose, no JSON wrapper | forced `format: "json"` decoder mode |
| #14 | **success** — 18 min total, 4.4 min drafter, real literary prose | (compounding wins) |
| #15a | drafter timed out at 30 min cap (128k context) | reverted spec to 64k |
| #15b | drafter timed out at 30 min cap **again** at 64k | (this RCA) |

The pattern: **every fix solved the immediate failure but exposed the next layer of an underlying problem we hadn't named.**

---

## 2. The single underlying root cause

> **Model context-size thrashing.** Every time `num_ctx` changes between successive Ollama calls, qwen3.6 36B reloads the model and reinitialises its KV cache for the new context size. On Apple Silicon unified memory with a 36B Q4_K_M model, that reload costs **5–15 minutes of wall-clock** that shows up as "the drafter is slow" rather than "the model is reloading."

Direct evidence from Run #15b:

| Stage | num_ctx | Wall-clock | Cold/warm? |
|---|---|---|---|
| intake (qwen3.5:9b) | ~6k | 21.2s | (different model entirely; cheap) |
| chunked bibles (qwen3.6) | ~5k | 63.0s | warm at small ctx |
| world bible (qwen3.6) | **24k** | **462s = 7.7 min** | first reload |
| scene drafter (qwen3.6) | **64k** | **timed out at 30 min** | second reload + first generation at 64k ctx |

The world bible at 7.7 min vs Run #14's 2.8 min is the smoking gun: **the wall-clock cost of a single num_ctx-driven model reload is on the order of 5 minutes.** The drafter call paid the same reload cost AGAIN going from 24k → 64k. Five minutes of reload + 25 min of thinking-mode generation = right at the 30-min cap.

The "fix" of reverting from 128k to 64k did not address the root cause. It only changed how many minutes of the cap were eaten by reload vs generation. **The number of reloads stayed the same; the per-reload cost shrank slightly.**

---

## 3. Why every prior fix was a symptom-patch, not a root-cause fix

Each prior fix solved the immediate failure mode AT a particular num_ctx, but none addressed the fact that we keep changing num_ctx between calls within a single pipeline run. The arc:

- **Run #11 fix** (raise `num_predict` to 16k): made one call work at one context size.
- **Run #12 fix** (mandatory interleaving): changed prompt, did not touch num_ctx.
- **Run #13 fix** (force JSON mode): saved an entire failure class but left num_ctx variability untouched.
- **Run #14 success**: succeeded by luck — the model happened to be loaded at the same num_ctx the drafter used because earlier runs had warmed it there. We mistook luck for craft.
- **Run #15 spec bump to 128k**: changed num_ctx in a way no prior call had warmed. First-call cold-load tax = 30+ min.
- **Run #15 revert to 64k**: still a num_ctx the freshly-loaded model didn't have. Reload again.

Until we stop **changing** num_ctx between agent calls inside a single run, we will keep paying the cold-load tax somewhere in the pipeline.

---

## 4. The five things we should have measured but did not

This is the secondary failure: we have no telemetry that would have surfaced the root cause earlier.

| Signal | Where it would have shown up | Why we missed it |
|---|---|---|
| `num_ctx` actually requested per call | Ollama API request body | not logged anywhere |
| Whether Ollama performed a model reload | `/api/ps` `expires_at` change | never queried |
| First-call vs steady-state per-call duration | per-call timing | logged total, not first-vs-steady-state |
| Actual prompt size in tokens | runner.rs request build | never measured against `max_context_tokens` budget |
| KV-cache memory pressure at chosen num_ctx | macOS memory pressure | not measured |

The orchestrator's `max_duration_secs` cap is the only "telemetry" — and it's a binary signal that fires *after* the time is already burned.

---

## 5. The fix sequence (in order of root-cause proximity)

Each fix below addresses the root cause more directly than its predecessor. **Ship them in this order**; do not skip the early ones to get to the later ones.

### Fix 1 — Pin a single `num_ctx` for the entire pipeline run

The cleanest answer to context thrashing is **don't thrash**. Pick the maximum num_ctx any agent in the run will need (currently the drafter's 64k), and use that num_ctx for every Ollama call in the run, including ones that would have used 6k or 24k.

Cost: bibles + world bible run at 64k instead of 5k–24k. KV cache for those calls is bigger than they need (~9 GiB instead of ~1–4 GiB). Wasted VRAM during those calls.

Benefit: model loads ONCE at 64k at the start of the run and stays there for every subsequent call. Zero reloads.

Net: Run #15 wall-clock drops from "30 min cap on first drafter" + 7.7 min world bible to ~5 min model warm-up + 2 min world bible + 5 min drafter. **Saves ~25 min** on the first scene alone, plus all subsequent scenes get instant model availability.

This is the single most impactful change we can make.

### Fix 2 — Per-call telemetry: log `num_ctx`, prompt size, cold-vs-warm

We cannot tune what we cannot measure. Three new pieces of structured logging at every Ollama call:

1. `num_ctx` requested
2. prompt token count (estimated from `chars / 4`)
3. wall-clock from request-send to first-token-received (= cold-load latency proxy)

This turns "the drafter is slow" into "the drafter spent 4 min waiting for first token, then 20 min generating" — which is the difference between needing a context-pinning fix and needing a thinking-mode budget fix.

### Fix 3 — Explicit pre-warm at run start

After Fix 1 the model only needs to load once per run; Fix 3 makes that load happen at a *predictable* moment (before any LLM work) rather than disguised as part of the first agent call.

A single throwaway request at the run's start: `chat(model, "ping", num_ctx=PIPELINE_NUM_CTX, num_predict=4)`. Costs ~30s. Pays back the first-agent-call cold-load tax in advance, and gives us a clean "warm-up done" log line so subsequent slowness is unambiguously model-side, not system-side.

### Fix 4 — Reduce drafter `num_ctx` to actual usage

Today's drafter spec is 32k input + 32k output = 64k. Measured prompt size per Run #14 telemetry (now that Fix 2 makes this measurable): the drafter actually uses ~10–12k input. Output uses ~8–16k.

So 24k is enough total, not 64k. Halve the per-run KV-cache cost without affecting any current workload. Reinstate higher num_ctx only when item-5 exemplar-memory or item-4 planner-with-prior-corpus actually push the input past 12k.

### Fix 5 — A "smoke test" that runs after every spec change

The Run #15 failure was foreseeable. We changed the drafter spec and the model thrash arrangement without re-running the multi-chapter scenario. The smoke test:

- One scene through the full pipeline at the post-change spec
- Asserts wall-clock under a sensible cap (~10 min)
- Asserts non-empty manuscript output
- Asserts `tells_verdict != "AI_SMELL_HIGH"`

A green smoke test gates the spec change. A red smoke test reverts the change automatically.

---

## 6. The bigger product lesson

**Spec changes that look additive (add a directive, raise a budget, add a slot) are not actually additive when they shift Ollama runtime behaviour.** The headroom bump from 64k → 128k looked like "more room for future work" but actually was "force model reload at runtime cost we never measured."

Every spec change should answer:
- Does this change the `num_ctx` Ollama loads with?
- Does this change the prompt size by more than 1k tokens?
- Did we run a smoke test after the change?

Until those three questions are part of every PR description, we will keep finding another layer.

---

## 7. The order of operations from here

1. **Apply Fix 1** (pin pipeline num_ctx) — single biggest lever
2. **Apply Fix 2** (telemetry) — so Fix 1's effect is measurable
3. **Apply Fix 3** (pre-warm) — clean cold-load semantics
4. **Re-run the multi-chapter scenario** — proves Fixes 1+2+3 work
5. **Apply Fix 4** (right-size drafter num_ctx) once telemetry confirms current usage
6. **Apply Fix 5** (smoke test gate) — locks the win in for future spec changes

Fixes 4 and 5 are deferred to a follow-up; this session ships 1+2+3 and re-validates.
