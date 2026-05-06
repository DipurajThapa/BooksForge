# AI Integration — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

> **Status note (2026-05-06):** The MVP/V1.0 scope of this document has been **partially superseded** by the implementation pack — see `../ARCHITECTURE.md §5` and `../AGENTS.md`. Specifically: (1) Ollama is now the **primary** local-LLM runtime for MVP and V1.0 (embedded llama.cpp moves to post-V1.0 behind a feature flag); (2) the AI surface includes a bounded agent swarm in addition to the single-shot prompt presets described here. The principles in §1, the prompt-template format in §5, the audit log in §12, and most of the rest of this document remain authoritative. Where this document and the implementation pack differ, the implementation pack wins for MVP. The pivot is captured as `[DECISION-006-revB]` in `13-glossary-and-decision-log.md`.

---

## 1. Principles

The AI layer follows five non-negotiables. **Local-first**: the default and well-tested path uses on-device inference; cloud is opt-in. **Provider-agnostic**: no surface in the application code knows whether the model is local or cloud, llama.cpp or Ollama, Anthropic or OpenAI — only a `LlmProvider` trait. **Auditable**: every call is recorded, every prompt template is versioned and hashed, every AI-applied edit produces a snapshot. **Cancellable**: any inference can be aborted; partial output is preserved or discarded per user choice. **Safe by default**: AI capability is off per project until enabled with a one-time consent prompt, and outputs never auto-apply.

## 2. Architecture

```
┌───────────────────────────────────────────────────────────────┐
│ React UI                                                      │
│   AI sidebar  ──► invoke `ai.suggest({...})` (typed IPC)      │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│ Layer 2: AI Orchestrator (Rust)                               │
│   • capability check                                          │
│   • context selection (entities, scope text, tone)            │
│   • prompt template resolution + hash                         │
│   • call LlmProvider                                          │
│   • stream tokens to UI                                       │
│   • record `ai_calls` audit row                               │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│ Layer 3: booksforge-ai (pure logic)                            │
│   PromptTemplate, ContextSelector, RedactionPolicy            │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│ Layer 4: booksforge-ai-runtime (adapters)                      │
│  ┌──────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐  │
│  │ Embedded     │ │ External   │ │ Cloud      │ │ Mock     │  │
│  │ llama.cpp    │ │ Ollama     │ │ providers  │ │ (tests)  │  │
│  └──────────────┘ └────────────┘ └────────────┘ └──────────┘  │
└───────────────────────────────────────────────────────────────┘
```

## 3. Local LLM runtime — [DECISION-006]

**Choice: embedded llama.cpp via Rust bindings, with Ollama supported as an external provider.**

Reasoning:

llama.cpp is the de facto open-source CPU/GPU inference engine for GGUF models, supports Metal on Mac, CUDA on NVIDIA, Vulkan as a fallback, and CPU on everything. Rust bindings (`llama-cpp-rs`, `llm`) are mature enough for production. Embedding it gives us a single-binary install: the user does not need to install Ollama separately to get AI. Hardware floor for a usable experience is an 8 GB RAM laptop with a 3B-Q4 model; 16 GB unlocks 7B-Q4 which is the recommended config; 32 GB allows 13B. CPU-only fallback is supported but flagged.

Ollama is also supported as an *external* runtime: if the user has Ollama running (on `localhost:11434`), BooksForge auto-detects and offers to use its already-pulled models. This gives power users their existing setup without us shipping duplicate weights.

Why not transformers/Python? Distribution friction is huge, Python startup is slow, and we would inherit a fat dependency tree. Our Rust binding avoids this.

## 4. Curated model catalogue

A bundled JSON file lists models, with **hash pinning**, that we have tested for prose-quality and footprint. Not exhaustive — users can sideload any GGUF — but the catalogue is what powers the auto-download UI:

| Tier | Model | Size (Q4_K_M) | RAM target | Use |
|------|-------|---------------|------------|-----|
| Tiny | TinyLlama 1.1B | ~700 MB | 4 GB | Quick sanity, summary |
| Small | Llama 3.2 3B Instruct | ~2.0 GB | 8 GB | Default for low-RAM |
| Medium | Llama 3.1 8B Instruct | ~4.8 GB | 16 GB | Recommended default |
| Medium | Mistral 7B Instruct v0.3 | ~4.4 GB | 16 GB | Alternative |
| Medium | Qwen 2.5 7B Instruct | ~4.7 GB | 16 GB | Stronger non-English |
| Large | Llama 3.1 70B Instruct | ~40 GB Q4 | 64 GB | Power users |
| Code | Qwen 2.5 Coder 7B | ~4.7 GB | 16 GB | Technical writers |

The catalogue is updated with each app release. Each entry has SHA-256 pinned, license, recommended sampling parameters, and the prompt format ("chat-ml", "llama-3", etc.) so the runtime knows how to format input.

The user picks a default model in settings. Per project they can override.

## 5. Prompt templates

Prompts are **structured**, not strings. A template is a declarative document:

```toml
[template]
id = "sharpen-prose.v3"
preset = "sharpen"
description = "Tighten language, reduce filler, preserve voice."
schema_version = 1

[input.required]
scope_text = { kind = "text", max_tokens = 2000 }

[input.optional]
tone_preset = { kind = "string", default = "literary" }
character_voice = { kind = "string", default = "" }

[render.system]
text = """
You are an expert prose editor. Tighten language while preserving the
author's voice. Do not add or remove story beats. Output the revised
paragraph and a one-sentence rationale.
"""

[render.user]
text = """
TONE: {{tone_preset}}
{% if character_voice %}CHARACTER VOICE: {{character_voice}}{% endif %}

ORIGINAL:
<<<USER_CONTENT>>>
{{scope_text}}
<<<END_USER_CONTENT>>>

Return revised paragraph then a `Rationale:` line.
"""
```

Key properties:

- The template is a versioned artifact. Hash is recorded on every `ai_calls` row so we can reproduce.
- Rendering uses a strict templating engine (e.g., MiniJinja). No arbitrary expressions; only declared variables.
- Untrusted content is fenced with `<<<USER_CONTENT>>>` markers and the system prompt instructs the model to ignore embedded instructions inside fences.
- The same template renders to the appropriate format for chat or completion APIs depending on the provider.

Built-in presets (FR-AI-005): Sharpen, Shorten, Expand, Continue, Rewrite-tone, Rephrase, Summarise, Brainstorm, Critique, Beta-read. Users add custom presets as templates; plugins ship presets in prompt packs.

## 6. Context selection

A core differentiator is **transparent context selection**. Before any AI call, the orchestrator builds a context bundle from:

The selected scope text (paragraph, scene, or chapter). Optionally, entities mentioned in the scope (characters, locations) — their card content is included. Optionally, the immediately preceding scene as continuity. The project's tone preset. Plugin-provided overlays (e.g., a romance plugin injects "this is a Black Moment beat" if the scene is tagged).

The user sees the assembled context in a preview UI before sending. They can deselect items. The send is the *exact* preview content — no hidden additions.

Token budget management: the orchestrator targets the configured model's context window (e.g., 8K, 32K, 128K). It greedily fills with highest-priority items (scope first, entities next, continuity last) and drops lower-priority items if over budget.

## 7. Cloud provider integration (V1.5)

### 7.1 Providers supported

Anthropic (Claude family), OpenAI (GPT family), OpenRouter (routes to many), Mistral La Plateforme, Cohere. Each is implemented as `LlmProvider`. Adding a new provider is one file in `booksforge-ai-runtime/cloud/`.

### 7.2 Bring-your-own-key vs Studio credits

Two modes. **BYO key**: user enters their API key in settings; key stored in OS keyring; calls go directly from the user's device to the provider. **Studio credits**: subscription includes a credit pool; calls are proxied through BooksForge's backend (which is a thin auth layer that records usage and forwards to the provider with a BooksForge-side enterprise key). The user toggles per-call which mode to use.

### 7.3 Privacy on cloud calls

For both modes, the same content-preview UI shows what will be sent. We use providers' enterprise endpoints with **no-train** terms wherever available (Anthropic's API, OpenAI Enterprise, etc.). For BYO key the user is responsible for their account's terms. For Studio credits BooksForge ensures no-train terms in our DPAs with each provider.

### 7.4 Cost control

Per-call estimate displayed before send (FR-AI-010). Per-day budget cap configurable. Hard rate-limit per minute to avoid runaway. Cancellation (FR-AI-015) aborts mid-call.

### 7.5 Error handling

Typed errors: `RateLimited`, `ContextTooLong`, `Unauthenticated`, `NetworkUnreachable`, `ProviderInternal`. UI guides the user to a fix (e.g., "context too long — try selecting a smaller scope"). On `NetworkUnreachable`, offer to retry with the local model.

## 8. AI as a plugin overlay

Prompt packs (plugin type) can register prompt overlays for specific contexts. Example: a "Mystery Pack" injects a fragment "remember that this is a fair-play mystery and clues must be present before deductions" into every Continue and Sharpen call when the project's template is `mystery-cosy`. Overlays are visible in the context preview. Overlay capability is rate-limited to prevent prompt-bloat by a single plugin.

## 9. Special features

### 9.1 Beta-reader (FR-AI-013)

Long-form critique. Inputs: a chapter. Output: a structured Markdown report (pacing, character clarity, scene goal achievement, prose-level notes). Implemented as a multi-pass: a first pass produces an outline of the chapter; a second pass critiques against rubric items. Each pass is an `ai_calls` row. Local-model usable on 8B class.

### 9.2 Series consistency check (FR-AI-014)

Hybrid: a deterministic linter scans for spelling drift in tagged entities; an LLM pass adjudicates ambiguous matches and surfaces likely issues. The deterministic part runs without AI; AI is only used for fuzzy adjudication.

### 9.3 Continue / Brainstorm

Continuations are explicitly bounded to the next paragraph or scene with the model encouraged to stop at natural breaks. Brainstorm produces N options the user can drag into notes — never directly into the manuscript without an accept step.

## 10. Inference performance

Targets (FR-AI-001 reference hardware: 16 GB Apple Silicon Mac, Llama 3.1 8B Q4_K_M, Metal):

- TTFT (time to first token) ≤ 500 ms
- Sustained throughput ≥ 30 tokens/sec
- 200-word rewrite end-to-end ≤ 6 seconds

On 8 GB RAM with Llama 3.2 3B Q4_K_M:

- TTFT ≤ 800 ms
- Throughput ≥ 40 tokens/sec (smaller model)
- 200-word rewrite ≤ 7 seconds

On CPU-only (no GPU/Metal): warned in UI, expect 5–20× slower; users get a "Slow inference" badge.

## 11. Streaming and cancellation

Tokens stream from the provider through the orchestrator to the UI as `ai.token` events. The UI accumulates and shows them live. A cancellation token is held by the orchestrator; on `ai.cancel(jobId)` the streaming task is dropped, the provider is signalled to stop where supported (HTTP cancel for cloud; abort generation for local), and the UI is asked to keep-or-discard the partial output.

## 12. Audit log

Schema in Data Model §4. Per call: provider, model, preset, prompt template hash, context tokens, output tokens, duration, cost estimate, status, error if any. The user can browse the project's AI history, replay a call (regenerate with the same parameters), and export the log to CSV for compliance.

## 13. Test fixtures

A "deterministic mock" provider returns canned outputs for unit/integration tests; this is what the editor's E2E tests use. A "live local" provider in CI loads a small fixture model (TinyLlama) to smoke-test the runtime path on every PR. Cloud providers are not hit in CI; we use VCR-style recordings for their adapters.

## 14. Failure modes and recovery

If the local model fails to load (corrupt download, missing weights), the UI shows a clear error and offers to re-download. Hash mismatch on download → reject and re-fetch. Inference timeout → surface error with "try a smaller scope" suggestion. Out-of-memory → kill inference task, free model, surface a polite error pointing to the lower-RAM model option. Cloud auth fail → prompt for new key; never use a partially-valid key state.

## 15. Roadmap-relevant features

V2.0 brings: voice-style models trained on user corpus *(opt-in, on-device only — no cloud training)*; multi-document long-context features (full-novel critique using long-context-window models); RAG over the user's own bibliography for fact-grounded research suggestions; translator pack with terminology preservation. None of these change the architecture in this document — they slot into existing surfaces.
