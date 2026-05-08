//! Agent spec — the 12-field canonical shape per `AGENTS.md §3`.
//!
//! Every MVP agent is a value of this struct.  The fields are inspectable
//! at runtime so the orchestrator's cross-cutting machinery (validators,
//! caps, telemetry) can act on them without per-agent special-casing.

use booksforge_prompt::PromptTemplateId;

/// Model family preference for an agent call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    /// Any capable instruct model (≥7B preferred).
    AnyInstruct,
    /// Long-context-friendly family (e.g., Llama 3.1).
    LongContext,
    /// Multilingual-strong family (e.g., Qwen 2.5).
    Multilingual,
}

/// Minimum parameter size the agent needs to produce useful output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSizeHint {
    /// 3B+ acceptable with a low-confidence warning.
    Small,
    /// 7B+ required for good quality.
    Medium,
    /// 13B+ preferred; 7B acceptable.
    Large,
    /// 30B+ recommended — high-end agents (e.g. Final Review Editor) where
    /// world-class quality outweighs runtime cost.
    ExtraLarge,
}

/// Preferred model configuration for an agent.
#[derive(Debug, Clone, Copy)]
pub struct ModelPreference {
    pub family:   ModelFamily,
    pub min_size: ModelSizeHint,
}

/// Per-slot token budget for an agent invocation.
#[derive(Debug, Clone, Copy)]
pub struct ContextBudget {
    pub max_context_tokens: u32,
    pub max_output_tokens:  u32,
}

impl ContextBudget {
    pub fn total(&self) -> u32 { self.max_context_tokens + self.max_output_tokens }
}

/// Whether the agent runs automatically or requires user action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhenToRun { Automatic, OnDemand, Scheduled }

/// Whether the user must confirm before output is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserGate { Required, NotRequired }

/// A documented failure mode for the agent.  Surfaced in telemetry,
/// help tooltips, and the test matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureMode {
    /// Stable, kebab-case identifier (e.g. `"empty-idea"`, `"range-mismatch"`).
    pub id:          &'static str,
    /// One-line description shown in the UI when this mode is hit.
    pub description: &'static str,
    /// Whether this mode is recoverable via retry, or terminal.
    pub recoverable: bool,
}

/// A reference to a cross-cutting validator that should run on this agent's
/// output.  Resolved by `booksforge-orchestrator::cross_cutting`.
///
/// All MVP agents include `Schema`, `Length`, and `Redaction`.  `EntitySanity`
/// is added for agents that emit proper-noun-bearing prose; `MemoryScope` is
/// added when memory writes are proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossCuttingValidator {
    /// Output must deserialize into the declared output type.
    Schema,
    /// Output must not contain system-prompt leakage / chain-of-thought.
    Redaction,
    /// Output token-length is bounded and non-trivial.
    Length,
    /// Proper nouns in output are in the entity bible / allowlist.
    EntitySanity,
    /// Any memory writes are within `allowed_write_scopes(agent_id)`.
    MemoryScope,
    /// Originality / anti-plagiarism — flags long verbatim spans copied
    /// from the source the agent was given or from the project's prior
    /// accepted scenes.  Pure local n-gram match; nothing leaves the
    /// device.  Run on every prose-emitting agent.
    Originality,
}

/// The 12-field canonical agent specification.  Maps 1:1 to AGENTS.md §3
/// except `pinned_model` (project addition) and that `validators` is a list
/// of cross-cutting validator IDs rather than function pointers (those are
/// resolved at orchestrator-binding time).
#[derive(Debug, Clone)]
pub struct AgentSpec {
    pub id:                &'static str,
    pub name:              &'static str,
    pub purpose:           &'static str,
    /// Documentation-grade name of the input type.  Lives in
    /// `booksforge-domain` and round-trips through `serde` parse-validation.
    pub input_schema_id:   &'static str,
    /// Documentation-grade name of the output type.
    pub output_schema_id:  &'static str,
    pub prompt_template:   PromptTemplateId,
    pub model_preference:  ModelPreference,
    /// Project addition — exact-tag pin for high-end agents.
    pub pinned_model:      Option<&'static str>,
    pub context_budget:    ContextBudget,
    pub validators:        &'static [CrossCuttingValidator],
    pub failure_modes:     &'static [FailureMode],
    pub when_to_run:       WhenToRun,
    pub user_gate:         UserGate,
}

/// Helper: standard "always-on" cross-cutting validators every agent must run.
pub const STD_VALIDATORS: &[CrossCuttingValidator] = &[
    CrossCuttingValidator::Schema,
    CrossCuttingValidator::Redaction,
    CrossCuttingValidator::Length,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_validators_are_three() {
        assert_eq!(STD_VALIDATORS.len(), 3);
    }

    #[test]
    fn context_budget_totals_correctly() {
        let b = ContextBudget { max_context_tokens: 4_000, max_output_tokens: 2_000 };
        assert_eq!(b.total(), 6_000);
    }
}
