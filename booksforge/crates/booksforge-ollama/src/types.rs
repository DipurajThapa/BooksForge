//! Request/response types for the Ollama REST API.

use serde::{Deserialize, Serialize};

/// Ollama server version reported by `GET /api/version`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaVersion {
    pub version: String,
}

/// A model available on the local Ollama instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub name: String,
    pub modified_at: String,
    /// Size in bytes.
    pub size: u64,
    pub digest: String,
}

/// A single chat turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `"system"` | `"user"` | `"assistant"`
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

/// Reasoning-mode control for thinking-capable models (Qwen 3.x, DeepSeek R-class, etc.).
///
/// Newer Ollama families surface a top-level `think` flag on the request payload.
/// When the flag is `false`, the model returns its answer directly via the
/// `message.content` field. When the flag is `true` (or absent on a thinking
/// model), the model may swallow the answer into a separate `message.thinking`
/// field, which downstream parsers won't see — a known footgun.
///
/// Per-agent guidance:
///   - `Disabled` for non-reasoning agents (intake, outline-architect,
///     chapter-drafter, copyeditor, final-polish, humanization).
///   - `Enabled` only for agents where structural reasoning earns its
///     tokens (proposal-validator, dev-editor).
///   - `Auto` (i.e. `None`) if the agent is family-agnostic; the wire field
///     is omitted and the model uses its default behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode {
    /// Force thinking off — the model must produce its answer directly.
    Disabled,
    /// Force thinking on.
    Enabled,
}

impl ThinkingMode {
    /// Wire value sent on the request payload's top-level `think` field.
    pub fn as_bool(self) -> bool {
        match self {
            ThinkingMode::Disabled => false,
            ThinkingMode::Enabled => true,
        }
    }
}

/// Parameters for `POST /api/generate`.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub stream: bool,
    /// Top-level `think` flag for thinking-capable model families.
    /// `None` omits the field from the wire payload; `Some(false)` disables
    /// thinking; `Some(true)` enables it. See [`ThinkingMode`].
    #[serde(rename = "think", skip_serializing_if = "Option::is_none")]
    pub think: Option<bool>,
    /// Force structured-output mode — see `ChatRequest::format`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

/// Parameters for `POST /api/chat`.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    /// Top-level `think` flag for thinking-capable model families.
    /// `None` omits the field from the wire payload; `Some(false)` disables
    /// thinking; `Some(true)` enables it. See [`ThinkingMode`].
    #[serde(rename = "think", skip_serializing_if = "Option::is_none")]
    pub think: Option<bool>,
    /// Force structured-output mode. `Some("json".to_owned())` constrains
    /// the model to produce valid JSON. Run #13 fix — under heavy
    /// prose-shaping system prompts (e.g. the MANDATORY INTERLEAVING
    /// voice contract directive), Qwen 3.5/3.6 occasionally drops the
    /// "Return JSON: ..." instruction and emits bare prose, wasting
    /// 28+ minutes of generation time. Setting `format: "json"` makes
    /// Ollama enforce JSON-shaped output at the decoder level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

impl ChatRequest {
    /// Builder helper: set the `think` flag from a `ThinkingMode`.
    pub fn with_thinking(mut self, mode: ThinkingMode) -> Self {
        self.think = Some(mode.as_bool());
        self
    }
}

impl GenerateRequest {
    /// Builder helper: set the `think` flag from a `ThinkingMode`.
    pub fn with_thinking(mut self, mode: ThinkingMode) -> Self {
        self.think = Some(mode.as_bool());
        self
    }
}

/// Shared model-level sampling options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerateOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,
    /// Hard token budget enforced by the orchestrator — passed through to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
}

/// Completed generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOutcome {
    pub model: String,
    pub response: String,
    pub prompt_eval_count: u32,
    pub eval_count: u32,
    /// Nanoseconds of total duration reported by Ollama.
    pub total_duration_ns: u64,
}

/// Completed chat result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOutcome {
    pub model: String,
    pub message: ChatMessage,
    pub prompt_eval_count: u32,
    pub eval_count: u32,
    pub total_duration_ns: u64,
}

/// Detailed model metadata returned by `POST /api/show`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Ollama model tag.
    pub name: String,
    /// Digest hash (sha256:…) for audit logging.
    pub digest: Option<String>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

/// Progress event emitted during `pull()` — mirrors Ollama's NDJSON stream.
#[derive(Debug, Clone)]
pub struct PullProgress {
    pub status: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
}

/// Callback type used by `OllamaClient::pull` to report download progress.
/// Each call corresponds to one NDJSON line from the Ollama stream.
pub type ProgressSink = Box<dyn Fn(PullProgress) + Send>;

/// Callback type used by `OllamaClient::generate` / `chat` to stream tokens.
/// Called once per token fragment as Ollama streams the response.
/// Uses `FnMut` so callers can accumulate tokens into a local buffer.
pub type TokenSink = Box<dyn FnMut(&str) + Send>;

/// Cooperative cancellation token shared between the orchestrator and an
/// in-flight Ollama request.  `cancel()` is idempotent.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    inner: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Signal cancellation.
    pub fn cancel(&self) {
        self.inner.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// `true` if `cancel()` has been called.
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_chat() -> ChatRequest {
        ChatRequest {
            model: "qwen3.5:9b".into(),
            messages: vec![ChatMessage::user("hi")],
            stream: false,
            think: None,
            format: None,
            options: None,
        }
    }

    fn base_gen() -> GenerateRequest {
        GenerateRequest {
            model: "qwen3.5:9b".into(),
            prompt: "hi".into(),
            system: None,
            stream: false,
            think: None,
            format: None,
            options: None,
        }
    }

    #[test]
    fn chat_request_omits_think_field_by_default() {
        let req = base_chat();
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            !json.contains("\"think\""),
            "default chat must not emit think: {json}"
        );
    }

    #[test]
    fn chat_request_emits_think_false_when_disabled() {
        let req = base_chat().with_thinking(ThinkingMode::Disabled);
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            json.contains("\"think\":false"),
            "expected think:false in: {json}"
        );
    }

    #[test]
    fn chat_request_emits_think_true_when_enabled() {
        let req = base_chat().with_thinking(ThinkingMode::Enabled);
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            json.contains("\"think\":true"),
            "expected think:true in: {json}"
        );
    }

    #[test]
    fn generate_request_omits_think_field_by_default() {
        let req = base_gen();
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            !json.contains("\"think\""),
            "default generate must not emit think: {json}"
        );
    }

    #[test]
    fn generate_request_emits_think_false_when_disabled() {
        let req = base_gen().with_thinking(ThinkingMode::Disabled);
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            json.contains("\"think\":false"),
            "expected think:false in: {json}"
        );
    }

    #[test]
    fn thinking_mode_as_bool_round_trips() {
        assert!(!ThinkingMode::Disabled.as_bool());
        assert!(ThinkingMode::Enabled.as_bool());
    }

    #[test]
    fn think_field_is_top_level_not_inside_options() {
        // Critical: Ollama looks for `think` at the request top level, not
        // inside `options`. If we ever accidentally nest it, thinking-mode
        // models will silently swallow output into `message.thinking`.
        let req = base_chat().with_thinking(ThinkingMode::Disabled);
        let value: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(value.get("think"), Some(&serde_json::Value::Bool(false)));
        assert!(value
            .get("options")
            .is_none_or(|opts| opts.get("think").is_none()));
    }
}
