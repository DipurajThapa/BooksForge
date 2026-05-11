use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Status of an agent run or task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskStatus {
    Running,
    Completed,
    /// Output failed schema / semantic validation after all retries.
    Invalid,
    Cancelled,
    Error,
}

impl AgentTaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Invalid => "invalid",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "invalid" => Some(Self::Invalid),
            "cancelled" => Some(Self::Cancelled),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// One row in `agent_runs` — represents a whole workflow invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: Ulid,
    pub workflow_id: String,
    pub project_id: Ulid,
    pub status: AgentTaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_tokens: Option<u32>,
    pub error_message: Option<String>,
    pub ollama_version: Option<String>,
    /// Whether this run was explicitly user-initiated (vs. automatic).
    pub user_initiated: bool,
}

/// One row in `agent_tasks` — represents a single agent call within a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: Ulid,
    pub run_id: Ulid,
    pub step_index: u32,
    pub agent_id: String,
    pub prompt_template_id: String,
    pub prompt_template_hash: String,
    pub model: String,
    pub model_digest: Option<String>,
    /// blake3 hex of the serialised input bundle.
    pub input_hash: String,
    /// blake3 hex of the serialised output (set on completion).
    pub output_hash: Option<String>,
    pub context_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub duration_ms: Option<u64>,
    pub retries: u32,
    pub status: AgentTaskStatus,
    pub error_category: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One row in `agent_outputs` — stores the validated output for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub task_id: Ulid,
    pub schema_id: String,
    pub schema_version: u32,
    /// For small outputs: inline JSON. For large: written to `agent_runs/<run_id>/<task_id>.json`.
    pub content_inline: Option<String>,
    pub content_path: Option<String>,
    /// blake3 hex of the content.
    pub hash: String,
    pub validated_at: DateTime<Utc>,
}
