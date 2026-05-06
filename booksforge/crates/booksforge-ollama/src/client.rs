//! `OllamaClient` trait and `HttpOllamaClient` production implementation.

use async_trait::async_trait;
use futures_util::TryStreamExt as _;
use reqwest::Client;

use crate::{
    types::{
        CancelToken, ChatOutcome, ChatRequest, GenerateOutcome, GenerateRequest, LocalModel,
        OllamaVersion, ProgressSink, PullProgress, TokenSink,
    },
    OllamaError,
};

const BASE_URL: &str = "http://127.0.0.1:11434";

/// Stable interface for all Ollama operations.
/// Implemented by `HttpOllamaClient` in production and a mock in tests.
#[async_trait]
pub trait OllamaClient: Send + Sync {
    /// `GET /api/version` — returns the running Ollama server version.
    async fn version(&self) -> Result<OllamaVersion, OllamaError>;

    /// `GET /api/tags` — lists all locally available models.
    async fn list_local_models(&self) -> Result<Vec<LocalModel>, OllamaError>;

    /// `POST /api/pull` — downloads a model; streams NDJSON progress events to `progress`.
    async fn pull(&self, model: &str, progress: ProgressSink) -> Result<(), OllamaError>;

    /// `POST /api/generate` — single-turn streaming completion; tokens delivered via `sink`.
    async fn generate(
        &self,
        request: GenerateRequest,
        sink: TokenSink,
        cancel: CancelToken,
    ) -> Result<GenerateOutcome, OllamaError>;

    /// `POST /api/chat` — multi-turn streaming completion; tokens delivered via `sink`.
    async fn chat(
        &self,
        request: ChatRequest,
        sink: TokenSink,
        cancel: CancelToken,
    ) -> Result<ChatOutcome, OllamaError>;
}

/// Production HTTP implementation of `OllamaClient`.
#[derive(Debug, Clone)]
pub struct HttpOllamaClient {
    http: Client,
    base_url: String,
}

impl HttpOllamaClient {
    /// Create with default base URL (`http://127.0.0.1:11434`).
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("reqwest client build should not fail"),
            base_url: BASE_URL.to_owned(),
        }
    }

    /// Override the base URL — used in integration tests against a mock server.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
        }
    }
}

impl Default for HttpOllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OllamaClient for HttpOllamaClient {
    async fn version(&self) -> Result<OllamaVersion, OllamaError> {
        let resp = self
            .http
            .get(format!("{}/api/version", self.base_url))
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::NotRunning
                } else {
                    OllamaError::Request(e)
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        Ok(resp.json().await?)
    }

    async fn list_local_models(&self) -> Result<Vec<LocalModel>, OllamaError> {
        #[derive(serde::Deserialize)]
        struct TagsResponse {
            models: Vec<LocalModel>,
        }

        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() { OllamaError::NotRunning } else { OllamaError::Request(e) }
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        let tags: TagsResponse = resp.json().await?;
        Ok(tags.models)
    }

    async fn pull(&self, model: &str, progress: ProgressSink) -> Result<(), OllamaError> {
        #[derive(serde::Serialize)]
        struct PullBody<'a> { name: &'a str, stream: bool }

        #[derive(serde::Deserialize)]
        struct PullLine {
            status:    String,
            completed: Option<u64>,
            total:     Option<u64>,
        }

        let resp = self
            .http
            .post(format!("{}/api/pull", self.base_url))
            .json(&PullBody { name: model, stream: true })
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() { OllamaError::NotRunning } else { OllamaError::Request(e) }
            })?;

        if resp.status().as_u16() == 404 {
            return Err(OllamaError::ModelNotFound { model: model.to_owned() });
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        // Consume NDJSON stream line by line.
        use tokio::io::AsyncBufReadExt as _;
        let stream = resp.bytes_stream();
        let reader = tokio_util::io::StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await.map_err(|e| OllamaError::Io(e))? {
            if line.trim().is_empty() { continue; }
            if let Ok(p) = serde_json::from_str::<PullLine>(&line) {
                progress(PullProgress {
                    status:    p.status,
                    completed: p.completed,
                    total:     p.total,
                });
            }
        }

        Ok(())
    }

    async fn generate(
        &self,
        mut request: GenerateRequest,
        sink: TokenSink,
        cancel: CancelToken,
    ) -> Result<GenerateOutcome, OllamaError> {
        request.stream = true;

        if cancel.is_cancelled() {
            return Err(OllamaError::Cancelled);
        }

        let resp = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() { OllamaError::NotRunning } else { OllamaError::Request(e) }
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        #[derive(serde::Deserialize)]
        struct StreamChunk {
            response: String,
            done:     bool,
            // Final chunk fields (only present when done = true).
            model:               Option<String>,
            prompt_eval_count:   Option<u32>,
            eval_count:          Option<u32>,
            total_duration:      Option<u64>,
        }

        use tokio::io::AsyncBufReadExt as _;
        let stream = resp.bytes_stream();
        let reader = tokio_util::io::StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let mut lines = reader.lines();
        let mut full_response = String::new();
        let mut final_chunk: Option<StreamChunk> = None;

        while let Some(line) = lines.next_line().await.map_err(|e| OllamaError::Io(e))? {
            if cancel.is_cancelled() {
                return Err(OllamaError::Cancelled);
            }
            if line.trim().is_empty() { continue; }
            let chunk: StreamChunk = serde_json::from_str(&line)?;
            if !chunk.response.is_empty() {
                sink(&chunk.response);
                full_response.push_str(&chunk.response);
            }
            if chunk.done {
                final_chunk = Some(chunk);
                break;
            }
        }

        let fc = final_chunk.ok_or_else(|| OllamaError::Http {
            status: 0,
            body: "stream ended without done=true".into(),
        })?;

        Ok(GenerateOutcome {
            model:             fc.model.unwrap_or_else(|| request.model.clone()),
            response:          full_response,
            prompt_eval_count: fc.prompt_eval_count.unwrap_or(0),
            eval_count:        fc.eval_count.unwrap_or(0),
            total_duration_ns: fc.total_duration.unwrap_or(0),
        })
    }

    async fn chat(
        &self,
        mut request: ChatRequest,
        sink: TokenSink,
        cancel: CancelToken,
    ) -> Result<ChatOutcome, OllamaError> {
        request.stream = true;

        if cancel.is_cancelled() {
            return Err(OllamaError::Cancelled);
        }

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() { OllamaError::NotRunning } else { OllamaError::Request(e) }
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        #[derive(serde::Deserialize)]
        struct MsgChunk { content: String }

        #[derive(serde::Deserialize)]
        struct StreamChunk {
            message:           Option<MsgChunk>,
            done:              bool,
            model:             Option<String>,
            prompt_eval_count: Option<u32>,
            eval_count:        Option<u32>,
            total_duration:    Option<u64>,
        }

        use tokio::io::AsyncBufReadExt as _;
        let stream = resp.bytes_stream();
        let reader = tokio_util::io::StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let mut lines = reader.lines();
        let mut full_content = String::new();
        let mut final_chunk: Option<StreamChunk> = None;

        while let Some(line) = lines.next_line().await.map_err(|e| OllamaError::Io(e))? {
            if cancel.is_cancelled() {
                return Err(OllamaError::Cancelled);
            }
            if line.trim().is_empty() { continue; }
            let chunk: StreamChunk = serde_json::from_str(&line)?;
            if let Some(ref msg) = chunk.message {
                if !msg.content.is_empty() {
                    sink(&msg.content);
                    full_content.push_str(&msg.content);
                }
            }
            if chunk.done {
                final_chunk = Some(chunk);
                break;
            }
        }

        let fc = final_chunk.ok_or_else(|| OllamaError::Http {
            status: 0,
            body: "stream ended without done=true".into(),
        })?;

        Ok(ChatOutcome {
            model:             fc.model.unwrap_or_else(|| request.model.clone()),
            message:           crate::types::ChatMessage::assistant(full_content),
            prompt_eval_count: fc.prompt_eval_count.unwrap_or(0),
            eval_count:        fc.eval_count.unwrap_or(0),
            total_duration_ns: fc.total_duration.unwrap_or(0),
        })
    }
}
