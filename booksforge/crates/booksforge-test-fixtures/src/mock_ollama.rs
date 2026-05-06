//! `MockOllamaClient` — a test double for `OllamaClient`.
//!
//! Configure canned responses before the test, then inject into the code under
//! test via the `OllamaClient` trait.
//!
//! # Example
//! ```rust
//! use booksforge_test_fixtures::mock_ollama::MockOllamaClient;
//! use booksforge_ollama::{GenerateOutcome, GenerateRequest};
//!
//! let mut mock = MockOllamaClient::default();
//! mock.set_generate_response("Hello from the mock!".into());
//! ```

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use booksforge_ollama::{
    client::OllamaClient,
    types::{
        CancelToken, ChatMessage, ChatOutcome, ChatRequest, GenerateOutcome, GenerateRequest,
        LocalModel, OllamaVersion, ProgressSink, TokenSink,
    },
    OllamaError,
};

/// Configurable canned responses for all `OllamaClient` methods.
#[derive(Debug, Clone, Default)]
pub struct MockConfig {
    /// Response returned by `version()`.  `None` → return a default version.
    pub version:           Option<Result<OllamaVersion, MockError>>,
    /// Models returned by `list_local_models()`.
    pub local_models:      Option<Result<Vec<LocalModel>, MockError>>,
    /// Whether `pull()` should succeed.
    pub pull_ok:           bool,
    /// Text returned as the `response` field of `GenerateOutcome`.
    pub generate_response: Option<String>,
    /// Whether `generate()` should fail.
    pub generate_error:    Option<MockError>,
    /// Text returned as the assistant message content in `ChatOutcome`.
    pub chat_response:     Option<String>,
    /// Whether `chat()` should fail.
    pub chat_error:        Option<MockError>,
}

/// Simplified error type for configuring mock failures.
#[derive(Debug, Clone)]
pub enum MockError {
    NotRunning,
    ModelNotFound(String),
    NoSuitableModel(String),
    ContextTooLong,
    OutOfMemory,
    Cancelled,
    Http { status: u16, body: String },
}

impl From<MockError> for OllamaError {
    fn from(e: MockError) -> Self {
        match e {
            MockError::NotRunning             => OllamaError::NotRunning,
            MockError::ModelNotFound(m)       => OllamaError::ModelNotFound { model: m },
            MockError::NoSuitableModel(r)     => OllamaError::NoSuitableModel { reason: r },
            MockError::ContextTooLong         => OllamaError::ContextTooLong,
            MockError::OutOfMemory            => OllamaError::OutOfMemory,
            MockError::Cancelled              => OllamaError::Cancelled,
            MockError::Http { status, body }  => OllamaError::Http { status, body },
        }
    }
}

/// Mock implementation of `OllamaClient`.
///
/// All mutating setup methods (`set_*`) are synchronous and must be called
/// before the mock is passed to async code.
#[derive(Clone, Default)]
pub struct MockOllamaClient {
    cfg: Arc<Mutex<MockConfig>>,
    /// Records all model IDs passed to `pull()`.
    pub pulled:   Arc<Mutex<Vec<String>>>,
    /// Records all prompts passed to `generate()`.
    pub generated: Arc<Mutex<Vec<String>>>,
    /// Records all message sequences passed to `chat()`.
    pub chatted:  Arc<Mutex<Vec<Vec<ChatMessage>>>>,
}

impl MockOllamaClient {
    pub fn new(cfg: MockConfig) -> Self {
        Self {
            cfg: Arc::new(Mutex::new(cfg)),
            ..Default::default()
        }
    }

    pub fn set_generate_response(&self, text: String) {
        self.cfg.lock().unwrap().generate_response = Some(text);
    }

    pub fn set_generate_error(&self, err: MockError) {
        self.cfg.lock().unwrap().generate_error = Some(err);
    }

    pub fn set_chat_response(&self, text: String) {
        self.cfg.lock().unwrap().chat_response = Some(text);
    }

    pub fn set_chat_error(&self, err: MockError) {
        self.cfg.lock().unwrap().chat_error = Some(err);
    }

    pub fn set_local_models(&self, models: Vec<LocalModel>) {
        self.cfg.lock().unwrap().local_models = Some(Ok(models));
    }

    pub fn set_pull_ok(&self, ok: bool) {
        self.cfg.lock().unwrap().pull_ok = ok;
    }

    pub fn pull_count(&self) -> usize {
        self.pulled.lock().unwrap().len()
    }

    pub fn generate_count(&self) -> usize {
        self.generated.lock().unwrap().len()
    }

    pub fn chat_count(&self) -> usize {
        self.chatted.lock().unwrap().len()
    }
}

#[async_trait]
impl OllamaClient for MockOllamaClient {
    async fn version(&self) -> Result<OllamaVersion, OllamaError> {
        let cfg = self.cfg.lock().unwrap();
        match &cfg.version {
            Some(Ok(v))  => Ok(v.clone()),
            Some(Err(e)) => Err(e.clone().into()),
            None         => Ok(OllamaVersion { version: "0.0.0-mock".into() }),
        }
    }

    async fn list_local_models(&self) -> Result<Vec<LocalModel>, OllamaError> {
        let cfg = self.cfg.lock().unwrap();
        match &cfg.local_models {
            Some(Ok(models)) => Ok(models.clone()),
            Some(Err(e))     => Err(e.clone().into()),
            None             => Ok(vec![]),
        }
    }

    async fn show(&self, model: &str) -> Result<booksforge_ollama::ModelInfo, OllamaError> {
        Ok(booksforge_ollama::ModelInfo {
            name:               model.to_owned(),
            digest:             Some("sha256:mock".into()),
            family:             Some("mock".into()),
            parameter_size:     Some("7B".into()),
            quantization_level: Some("Q4_K_M".into()),
        })
    }

    async fn pull(
        &self,
        model: &str,
        progress: ProgressSink,
    ) -> Result<(), OllamaError> {
        self.pulled.lock().unwrap().push(model.to_owned());
        let ok = self.cfg.lock().unwrap().pull_ok;
        if !ok {
            return Err(OllamaError::ModelNotFound { model: model.to_owned() });
        }
        // Emit a single synthetic progress event so callers know the sink works.
        progress(PullProgress {
            status:    "success".into(),
            completed: None,
            total:     None,
        });
        Ok(())
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        mut sink: TokenSink,
        _cancel: CancelToken,
    ) -> Result<GenerateOutcome, OllamaError> {
        self.generated.lock().unwrap().push(request.prompt.clone());
        let cfg = self.cfg.lock().unwrap();

        if let Some(err) = &cfg.generate_error {
            return Err(err.clone().into());
        }

        let text = cfg
            .generate_response
            .clone()
            .unwrap_or_else(|| "Mock generate response.".into());

        drop(cfg);
        sink(&text);

        Ok(GenerateOutcome {
            model:             request.model,
            response:          text,
            prompt_eval_count: 1,
            eval_count:        1,
            total_duration_ns: 0,
        })
    }

    async fn chat(
        &self,
        request: ChatRequest,
        mut sink: TokenSink,
        _cancel: CancelToken,
    ) -> Result<ChatOutcome, OllamaError> {
        self.chatted.lock().unwrap().push(request.messages.clone());
        let cfg = self.cfg.lock().unwrap();

        if let Some(err) = &cfg.chat_error {
            return Err(err.clone().into());
        }

        let text = cfg
            .chat_response
            .clone()
            .unwrap_or_else(|| "Mock chat response.".into());

        drop(cfg);
        sink(&text);

        Ok(ChatOutcome {
            model:             request.model,
            message:           ChatMessage::assistant(text),
            prompt_eval_count: 1,
            eval_count:        1,
            total_duration_ns: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_generate_returns_canned_response() {
        let mock = MockOllamaClient::default();
        mock.set_generate_response("Test output".into());

        let req = GenerateRequest {
            model:   "test:latest".into(),
            prompt:  "hello".into(),
            system:  None,
            stream:  false,
            options: None,
        };

        let mut tokens = vec![];
        let sink: TokenSink = Box::new(|t: &str| tokens.push(t.to_owned()));
        let outcome = mock.generate(req, sink, CancelToken::new()).await.unwrap();

        assert_eq!(outcome.response, "Test output");
        assert_eq!(mock.generate_count(), 1);
    }

    #[tokio::test]
    async fn mock_generate_returns_configured_error() {
        let mock = MockOllamaClient::default();
        mock.set_generate_error(MockError::NotRunning);

        let req = GenerateRequest {
            model:   "test:latest".into(),
            prompt:  "hello".into(),
            system:  None,
            stream:  false,
            options: None,
        };

        let err = mock
            .generate(req, Box::new(|_| {}), CancelToken::new())
            .await
            .unwrap_err();

        assert!(matches!(err, OllamaError::NotRunning));
    }

    #[tokio::test]
    async fn mock_pull_records_model_and_emits_progress() {
        let mock = MockOllamaClient::default();
        mock.set_pull_ok(true);

        let mut events = vec![];
        let sink: ProgressSink = Box::new(|p| events.push(p.status.clone()));
        mock.pull("qwen2.5:7b-instruct-q4_K_M", sink).await.unwrap();

        assert_eq!(mock.pull_count(), 1);
        assert!(events.contains(&"success".to_owned()));
    }
}
