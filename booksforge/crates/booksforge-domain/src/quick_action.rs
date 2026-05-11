//! Quick-action presets (MZ-08).
//!
//! Quick actions are inline AI helpers triggered from the editor (Cmd/Ctrl+K):
//! Sharpen, Continue, Rephrase.  They are **not agents** — they emit raw
//! prose, not validated JSON, and are recorded in their own `ai_calls`
//! ledger rather than `agent_runs / agent_tasks / agent_outputs`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Quick-action presets surfaced from the editor's Cmd/Ctrl+K bar.
///
/// `FinalPolish` is the high-end, model-pinned variant — it always runs
/// against `qwen3.6:latest` regardless of the project default model, since
/// its purpose is "world-class" editorial polish that's worth the runtime
/// cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuickActionPreset {
    /// Tighten the supplied prose without changing facts or paragraph count.
    Sharpen,
    /// Continue the supplied prose by 1–3 paragraphs.
    Continue_,
    /// Rephrase the supplied prose; same meaning, different wording.
    Rephrase,
    /// Senior-editor pass — model-pinned to `qwen3.6:latest`.
    FinalPolish,
    /// Tighten the passage to ≈ half its length, preserving facts.
    Shorten,
    /// Flesh out the passage with grounded sensory + interior detail.
    Expand,
}

impl QuickActionPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sharpen => "sharpen",
            Self::Continue_ => "continue",
            Self::Rephrase => "rephrase",
            Self::FinalPolish => "final_polish",
            Self::Shorten => "shorten",
            Self::Expand => "expand",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sharpen" => Some(Self::Sharpen),
            "continue" => Some(Self::Continue_),
            "rephrase" => Some(Self::Rephrase),
            "final_polish" => Some(Self::FinalPolish),
            "shorten" => Some(Self::Shorten),
            "expand" => Some(Self::Expand),
            _ => None,
        }
    }

    /// `(template_id, version)` for the prompt-engine catalogue.
    pub fn template(self) -> (&'static str, &'static str) {
        match self {
            Self::Sharpen => ("sharpen-prose", "v1"),
            Self::Continue_ => ("continue-paragraph", "v1"),
            Self::Rephrase => ("rephrase", "v1"),
            Self::FinalPolish => ("final-polish", "v1"),
            Self::Shorten => ("shorten", "v1"),
            Self::Expand => ("expand", "v1"),
        }
    }

    /// Returns the model tag this preset is pinned to, if any.
    /// `Sharpen / Continue / Rephrase` use the project default; only
    /// `FinalPolish` overrides.
    pub fn pinned_model(self) -> Option<&'static str> {
        match self {
            Self::FinalPolish => Some("qwen3.6:latest"),
            _ => None,
        }
    }
}

/// Status of an `ai_calls` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiCallStatus {
    Ok,
    Cancelled,
    Error,
}

impl AiCallStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ok" => Some(Self::Ok),
            "cancelled" => Some(Self::Cancelled),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// One row in `ai_calls` — the per-call audit ledger for quick-action
/// presets.  Apply-tracking columns are inlined (no `agent_applied_edits`
/// row) since quick actions have no `agent_tasks` parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCall {
    pub id: Ulid,
    /// The scene node the call ran against (the user's selection scope).
    pub node_id: Ulid,
    pub preset: QuickActionPreset,
    pub model: String,
    pub prompt_template_id: String,
    pub prompt_template_hash: String,
    pub scope_text_len: u32,
    pub output_text: Option<String>,
    pub context_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub duration_ms: Option<u64>,
    pub status: AiCallStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,

    // ── Apply tracking (set when the user clicks Accept) ─────────────────
    /// Snapshot taken before the suggestion was applied.
    pub pre_edit_snapshot_id: Option<Ulid>,
    /// `applied_at` timestamp, or `None` if the suggestion was never accepted.
    pub applied_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_roundtrip() {
        for p in [
            QuickActionPreset::Sharpen,
            QuickActionPreset::Continue_,
            QuickActionPreset::Rephrase,
            QuickActionPreset::FinalPolish,
            QuickActionPreset::Shorten,
            QuickActionPreset::Expand,
        ] {
            assert_eq!(QuickActionPreset::from_str(p.as_str()), Some(p));
        }
    }

    #[test]
    fn preset_template_mapping_is_pinned() {
        assert_eq!(
            QuickActionPreset::Sharpen.template(),
            ("sharpen-prose", "v1")
        );
        assert_eq!(
            QuickActionPreset::Continue_.template(),
            ("continue-paragraph", "v1")
        );
        assert_eq!(QuickActionPreset::Rephrase.template(), ("rephrase", "v1"));
        assert_eq!(
            QuickActionPreset::FinalPolish.template(),
            ("final-polish", "v1")
        );
        assert_eq!(QuickActionPreset::Shorten.template(), ("shorten", "v1"));
        assert_eq!(QuickActionPreset::Expand.template(), ("expand", "v1"));
    }

    #[test]
    fn final_polish_is_model_pinned() {
        assert_eq!(
            QuickActionPreset::FinalPolish.pinned_model(),
            Some("qwen3.6:latest")
        );
        assert_eq!(QuickActionPreset::Sharpen.pinned_model(), None);
        assert_eq!(QuickActionPreset::Continue_.pinned_model(), None);
        assert_eq!(QuickActionPreset::Rephrase.pinned_model(), None);
    }

    #[test]
    fn status_roundtrip() {
        for s in [
            AiCallStatus::Ok,
            AiCallStatus::Cancelled,
            AiCallStatus::Error,
        ] {
            assert_eq!(AiCallStatus::from_str(s.as_str()), Some(s));
        }
    }
}
