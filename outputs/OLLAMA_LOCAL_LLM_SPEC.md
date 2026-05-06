# Ollama / Local LLM Spec — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative Ollama integration spec.** Companion to `ARCHITECTURE.md §5` and `AGENTS.md`.

This document is the contract for the **Ollama HTTP client**, the **OllamaSetupWizard**, the **curated model registry**, the **model selection rules**, and the **failure-recovery behaviour**. Everything here is MVP-ready.

---

## 1. Why Ollama-first

Per `[DECISION-006-revB]`. Summary:

- One integration that handles model download, version pinning, GPU detection, and a stable HTTP API.
- Cross-platform (macOS / Windows / Linux).
- Permissive licence (MIT) and active maintenance.
- Lets us ship BooksForge as a much smaller binary; no embedded inference engine or per-platform GGUF mass.

The cost is one extra installation step for the user; the OllamaSetupWizard mitigates that.

## 2. Communication protocol

BooksForge talks to Ollama over HTTP on `127.0.0.1:11434`.

Endpoints we use:

- `GET /api/version` — probe (200 ms timeout).
- `GET /api/tags` — list installed models.
- `POST /api/pull` (streaming) — pull a model with progress.
- `POST /api/show` — get model digest + metadata for audit.
- `POST /api/generate` (streaming) — completion API.
- `POST /api/chat` (streaming) — chat-format API (preferred for instruct models).
- `DELETE /api/delete` — V1.0 (model management UI).

**No other endpoint is hit.**

The HTTP client is `booksforge-ollama` (Layer 4 Rust crate) wrapping `reqwest`. All calls are async with `tokio::CancellationToken` support.

## 3. The OllamaClient trait

```rust
#[async_trait]
pub trait OllamaClient: Send + Sync {
    async fn version(&self) -> Result<OllamaVersion, OllamaError>;
    async fn list_local_models(&self) -> Result<Vec<LocalModel>, OllamaError>;
    async fn pull(&self, model: &str, progress: ProgressSink) -> Result<(), OllamaError>;
    async fn show(&self, model: &str) -> Result<ModelInfo, OllamaError>;
    async fn generate(&self, req: GenerateRequest, sink: TokenSink, cancel: CancelToken)
        -> Result<GenerateOutcome, OllamaError>;
    async fn chat(&self, req: ChatRequest, sink: TokenSink, cancel: CancelToken)
        -> Result<ChatOutcome, OllamaError>;
}

pub enum OllamaError {
    Unreachable { detail: String },
    NotFound { model: String },
    AuthRequired,                          // future
    OutOfMemory,
    ContextTooLong { tokens: u32, model_window: u32 },
    InvalidRequest { detail: String },
    InternalError { detail: String },
    Cancelled,
    NoSuitableModel { reason: String },
}
```

A `MockOllamaClient` ships in `booksforge-test-fixtures` and is used in unit tests; the real client is integration-tested in CI with a tinyllama smoke job.

## 4. Curated model registry

`booksforge-ollama/models.toml`:

```toml
[[model]]
id = "qwen2.5:7b-instruct-q4_K_M"
display_name = "Qwen 2.5 7B Instruct (Q4_K_M)"
family = "qwen"
size_bytes = 4_700_000_000
ram_min_gb = 16
context_window = 32768
chat_format = "chatml"
recommended_for = ["fiction", "non_fiction", "memoir", "academic"]
strengths = ["dialogue", "non-english", "instruction-following"]
default_for_modes = ["fiction", "memoir"]   # chosen if the user has not pinned a model
notes = "Strong default for multilingual fiction; recommended MVP default."
official = true

[[model]]
id = "llama3.1:8b-instruct-q4_K_M"
display_name = "Llama 3.1 8B Instruct (Q4_K_M)"
family = "llama"
size_bytes = 4_800_000_000
ram_min_gb = 16
context_window = 131072
chat_format = "llama3"
recommended_for = ["fiction", "non_fiction", "academic"]
strengths = ["long-context", "instruction-following", "english-prose"]
default_for_modes = ["non_fiction", "academic"]
official = true

[[model]]
id = "mistral:7b-instruct-q4_K_M"
display_name = "Mistral 7B Instruct (Q4_K_M)"
family = "mistral"
size_bytes = 4_400_000_000
ram_min_gb = 16
context_window = 32768
chat_format = "mistral"
recommended_for = ["fiction", "non_fiction"]
strengths = ["english-prose", "fast"]
official = true

[[model]]
id = "gemma2:9b-instruct-q4_K_M"
display_name = "Gemma 2 9B Instruct (Q4_K_M)"
family = "gemma"
size_bytes = 5_400_000_000
ram_min_gb = 24
context_window = 8192
chat_format = "gemma2"
recommended_for = ["fiction", "non_fiction"]
strengths = ["multilingual"]
official = true

[[model]]
id = "phi3.5:latest"
display_name = "Phi-3.5 Mini (Q4)"
family = "phi"
size_bytes = 2_400_000_000
ram_min_gb = 8
context_window = 131072
chat_format = "phi"
recommended_for = ["non_fiction"]
strengths = ["small", "long-context", "low-RAM"]
default_for_modes = []                      # not a default; available for low-RAM users
official = true

[[model]]
id = "tinyllama:1.1b-chat-q4_K_M"
display_name = "TinyLlama 1.1B Chat (Q4)"
family = "tinyllama"
size_bytes = 700_000_000
ram_min_gb = 4
context_window = 2048
chat_format = "chatml-tiny"
recommended_for = []                        # not user-recommended; used for nightly smoke tests
strengths = ["fastest", "smallest"]
official = true
```

The registry is data, not code. Updated each release. The user can use any model Ollama exposes; the registry is what powers the auto-pull UI and default selection.

## 5. The OllamaSetupWizard

Per `UI_UX_SPEC.md §4`. Steps:

1. **Detect.** Probe `127.0.0.1:11434/api/version`.
   - Reachable: jump to step 3.
   - Not reachable but a binary is found on disk (platform-default install path): offer "Launch Ollama".
   - Not installed: step 2.

2. **Install.** Two options:
   - **Guided install.** Download the official installer over HTTPS from `https://ollama.com/download/{platform}`, verify a pinned SHA-256, run it (the user grants installer privileges).
   - **Manual install.** Show the URL and instructions; re-probe after the user confirms.

3. **Pick a model.** Show the curated list filtered by detected RAM. Default highlighted based on detected hardware:
   - 8 GB: Phi-3.5 Mini.
   - 16 GB: Qwen 2.5 7B (or Llama 3.1 8B if non-fiction mode).
   - 24+ GB: Llama 3.1 8B; offer Gemma 2 9B for multilingual.
   - 32+ GB: Llama 3.1 8B + suggest Llama 3.1 70B for power users (post-MVP UI).
   - User can pick any model.

4. **Pull.** `POST /api/pull` with progress events. The UI shows MB downloaded, ETA, speed.

5. **Smoke test.** Run a tiny "Are you working?" generation against the chosen model; on non-empty output, finish.

The wizard can be re-entered anytime from Settings → AI / Models.

### 5.1 Pinned installer hashes

The `booksforge-ollama/installer-pins.toml` file:

```toml
[macos.universal]
url = "https://ollama.com/download/Ollama-darwin.zip"
sha256 = "<pinned at release tag>"
size_bytes = "<approximate>"

[windows.x64]
url = "https://ollama.com/download/OllamaSetup.exe"
sha256 = "<pinned at release tag>"

[linux.x64]                                 # V1.0
url = "..."
sha256 = "..."
```

A hash mismatch fails the install with a typed error. The user can choose "Try again later" or "Manual install."

## 6. Model selection rules

When an agent needs a model and the user has not pinned one for the agent:

1. Look up the agent's `model_preference` in `AGENTS.md`.
2. Filter installed models by family preference and `ram_min_gb` ≤ detected RAM.
3. Prefer models whose `context_window` ≥ the agent's required context.
4. If multiple match, prefer the one in `default_for_modes` for the project's mode.
5. If none match, return `OllamaError::NoSuitableModel { reason }`.

The user can override at three scopes: per-agent (per project), per-project, or globally in settings.

## 7. Generation parameters

Defaults per agent's prompt template:

```toml
[render.sampling]
temperature = 0.7                          # default; overridable per agent
top_p = 0.9
top_k = 40
repeat_penalty = 1.1
num_predict = -1                           # use the agent's context_budget output cap
stop = []                                  # agent-specific stop tokens
```

JSON-mode (where Ollama supports it via the `format: "json"` parameter):

- The Outline Architect, Memory Curator, Vocabulary Dictionary, Copyeditor, Continuity, Humanization agents request JSON-mode where the model supports it. Falls back to ungrammared output with stricter validation when not.

## 8. Streaming and cancellation

Tokens stream from Ollama via the streaming HTTP API; the `booksforge-ollama` client emits them through a `TokenSink` trait. The orchestrator forwards to UI as `ai.token` events.

Cancellation: the orchestrator's `CancelToken` aborts the HTTP request. Ollama stops generation server-side on connection close. The orchestrator marks the run `cancelled` and preserves any partial output.

## 9. Audit fields recorded per call

For every Ollama call, BooksForge records on the `agent_tasks` row (per `DATA_MODEL.md §5`):

- `model` — the model id (e.g., `qwen2.5:7b-instruct-q4_K_M`).
- `model_digest` — from `/api/show` (a content hash of the model). If unavailable, `unknown` rather than empty.
- `prompt_template_id` and `prompt_template_hash`.
- `input_hash` — blake3 of the rendered prompt + context bundle.
- `output_hash` — blake3 of the parsed output.
- `context_tokens`, `output_tokens`, `duration_ms`.
- `retries`.
- `status` (`completed | invalid | cancelled | error`).
- `error_category` if applicable.
- The `agent_runs.ollama_version` is captured once per run.

This makes any past run reproducible by replaying with the same model+digest+template+input.

## 10. Failure modes and recovery

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Ollama unreachable | `/api/version` timeout (200 ms) | UI offers Launch / Re-probe / OllamaSetupWizard / Cancel. No agent runs. |
| Model not pulled | `/api/show` returns 404 | UI offers Pull / Pick another / Cancel. |
| OOM during generation | error in stream | Typed `OllamaError::OutOfMemory`. UI suggests a smaller model. Run marked `error`. |
| Context too long | error in stream | Typed `OllamaError::ContextTooLong { tokens, model_window }`. Orchestrator's `ContextBuilder` shrinks context (drops low-priority items per `MEMORY_SYSTEM.md §9`) and retries once. If still over, run marked `error`. |
| Invalid request | parse error | Typed `OllamaError::InvalidRequest { detail }`. Should not happen in production; bug if it does. Tested. |
| Network jitter / partial stream | stream truncated | Orchestrator validates whatever was streamed; if invalid, falls back to a retry. |
| Mid-call cancellation | `CancelToken` fires | HTTP request aborted; partial output preserved as `agent_runs/<run_id>/<task_id>.json`; row marked `cancelled`. |
| Non-loopback host configured | startup check | Consent dialog warning the user about privacy; configurable but logged in `model_settings`. |

## 11. Performance budgets

| Surface | Budget | Measured |
|---------|--------|----------|
| Probe (`/api/version`) | ≤ 200 ms | startup probe |
| List models (`/api/tags`) | ≤ 500 ms | dev probe |
| First token (7B-Q4 on 16 GB Mac) | ≤ 2 s | nightly bench |
| Sustained throughput (7B-Q4) | ≥ 25 tokens/s | nightly bench |
| 200-word rewrite end-to-end | ≤ 6 s | nightly bench |

CI fails the PR on >10% regressions without a justification block.

## 12. Hardware tiers and CPU-only fallback

Detected via `sysinfo` (RAM) and OS APIs (GPU presence).

- **8 GB RAM, no GPU.** Phi-3.5 Mini Q4 is usable. CPU-only flagged with a "Slow inference" badge; expect 5–20× slower than the GPU path.
- **16 GB RAM, integrated GPU or Metal.** 7B-Q4 models fit and run well. Default tier.
- **16 GB RAM, dedicated GPU (NVIDIA / AMD on Linux V1.0).** 7B-Q4 plus 8B comfortably; Gemma 2 9B usable.
- **32+ GB RAM, dedicated GPU.** 13B and above. Power-user tier.
- **CPU-only (no GPU).** Hard-flagged; users see a warning before pulling a model.

## 13. Privacy

- Loopback-only by default. Non-loopback configuration is gated by an explicit consent dialog and recorded in `model_settings`.
- `Ollama.pull` triggers a download from Ollama's CDN — this is the only pull-time outbound network call we make on the user's behalf, and it is user-initiated.
- BooksForge never proxies prompts through any third party.
- No telemetry from the Ollama integration; events stay local in the `agent_runs` ledger.

## 14. Privileged operations

The OllamaSetupWizard's "Guided install" requires installer privileges (admin on Windows, sudo on macOS). The wizard explains this clearly and the user grants per OS conventions. Failure to grant falls back to "Manual install" instructions.

## 15. Acceptance criteria

The Ollama integration is acceptable when:

1. On a clean macOS or Windows machine without Ollama, the OllamaSetupWizard completes guided install and a model pull within 10 minutes (network-dependent).
2. With Ollama running, the curated model list reflects installed models within 500 ms of the wizard opening.
3. A pull of `qwen2.5:7b-instruct-q4_K_M` completes with progress; failure is recoverable.
4. The smoke test returns a non-empty completion; the model digest is captured for audit.
5. Killing Ollama mid-run leaves the run marked `external_error`; partial outputs preserved; the user is offered "retry when Ollama is back."
6. With network mocked-fail, every BooksForge feature except Ollama install / pull works.
7. With telemetry off, no outbound packets after the initial Ollama probe (pcap assertion).
8. The `MockOllamaClient` exercises every code path in unit tests; nightly smoke against tinyllama exercises the real HTTP path.

## 16. Out of scope

- Cloud LLM providers — V1.0 (`_deep/08-ai-integration.md §7`).
- Embedded llama.cpp — post-V1.0.
- Bring-your-own LLM gateway (LM Studio, OpenAI-compatible local servers) — V1.0.
- Model fine-tuning UI — V2.0.
- Voice models / Whisper — V2.0.
