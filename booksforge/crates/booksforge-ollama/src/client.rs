//! `OllamaClient` trait and `HttpOllamaClient` production implementation.

use async_trait::async_trait;
use reqwest::Client;

use crate::{
    types::{ChatOutcome, ChatRequest, GenerateOutcome, GenerateRequest, LocalModel, OllamaVersion},
    CancelToken, OllamaError,
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

    /// `POST /api/pull` — downloads a model; blocks until complete.
    async fn pull(&self, model: &str) -> Result<(), OllamaError>;

    /// `POST /api/generate` — single-turn text completion (non-streaming).
    async fn generate(
        &self,
        request: GenerateRequest,
        cancel: &CancelToken,
    ) -> Result<GenerateOutcome, OllamaError>;

    /// `POST /api/chat` — multi-turn chat completion (non-streaming).
    async fn chat(
        &self,
        request: ChatRequest,
        cancel: &CancelToken,
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

    async fn pull(&self, model: &str) -> Result<(), OllamaError> {
        #[derive(serde::Serialize)]
        struct PullBody<'a> {
            name:   &'a str,
            stream: bool,
        }

        let resp = self
            .http
            .post(format!("{}/api/pull", self.base_url))
            .json(&PullBody { name: model, stream: false })
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

        Ok(())
    }

    async fn generate(
        &self,
        mut request: GenerateRequest,
        cancel: &CancelToken,
    ) -> Result<GenerateOutcome, OllamaError> {
        request.stream = false;

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

        if cancel.is_cancelled() {
            return Err(OllamaError::Cancelled);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        Ok(resp.json().await?)
    }

    async fn chat(
        &self,
        mut request: ChatRequest,
        cancel: &CancelToken,
    ) -> Result<ChatOutcome, OllamaError> {
        request.stream = false;

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

        if cancel.is_cancelled() {
            return Err(OllamaError::Cancelled);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        Ok(resp.json().await?)
    }
}
