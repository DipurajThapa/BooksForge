//! IPC types for the Ollama Setup Wizard and model management commands.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Result of `ollama_probe` — full status of the local Ollama environment.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct OllamaProbeResult {
    /// Whether the Ollama HTTP API is reachable.
    pub api_reachable: bool,
    /// Ollama server version, if reachable.
    pub version: Option<String>,
    /// Whether an Ollama binary was found on disk (may not be running).
    pub binary_found: bool,
    /// Detected system RAM in GB (`null` if detection failed).
    pub ram_gb: Option<u32>,
}

/// Result of `ollama_status` — minimal Ollama health check.
///
/// Distinct from `OllamaProbeResult`: the *probe* command also detects
/// the binary on disk and the system RAM, while *status* is the
/// fast/cheap path used by the always-visible status indicator.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct OllamaStatusResponse {
    /// Whether the Ollama HTTP API is currently reachable on the
    /// configured loopback host.
    pub running: bool,
    /// Ollama server version, if reachable.
    pub version: Option<String>,
}

/// One entry in the model list returned by `ollama_list_models`.
/// Merges the curated registry entry with local availability information.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct ModelListEntry {
    /// Ollama pull tag.
    pub id: String,
    pub display_name: String,
    pub family: String,
    /// Approximate download size in bytes.
    pub size_bytes: u64,
    /// Minimum RAM (GB) required.
    pub ram_min_gb: u32,
    pub context_window: u32,
    pub recommended_for: Vec<String>,
    pub strengths: Vec<String>,
    pub notes: String,
    pub default_for_modes: Vec<String>,
    /// Whether this model appears in the picker (`false` = internal/smoke only).
    pub official: bool,
    /// Whether the model is already installed locally.
    pub is_installed: bool,
    /// Local digest hash (sha256:…), present when installed.
    pub digest: Option<String>,
}

/// Progress payload emitted as a Tauri event during `ollama_pull`.
/// Event name: `"ollama:pull-progress"`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct PullProgressPayload {
    pub model: String,
    pub status: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
}

/// Result of `ollama_smoke_test`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct SmokeTestResult {
    pub success: bool,
    /// A short excerpt of the model's response (for the UI to display).
    pub response: Option<String>,
    pub duration_ms: u64,
    /// Error message if the test failed.
    pub error: Option<String>,
}
