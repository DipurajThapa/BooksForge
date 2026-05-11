//! Final Review Editor Agent.
//!
//! Runs as the **last** agent in the publish pipeline — after copyediting,
//! continuity, humanisation, and developmental notes are settled.  Its job is
//! to fine-tune prose to a publishable, world-class standard: rhythm, word
//! choice, transitions, and emotional clarity, without changing the author's
//! voice or the established facts.
//!
//! # Why a dedicated agent
//!
//! The other editor agents are mechanical (copyeditor) or structural
//! (developmental-editor, continuity).  None of them tackle the qualitative
//! "is this prose good?" pass.  A separate, high-end agent makes the cost /
//! quality trade-off explicit: the user knowingly opts in to a slower, more
//! capable model for the final polish.
//!
//! # Model pin
//!
//! Pinned to `qwen3.6:latest` (36 B MoE, Q4) — the highest-quality local
//! option in the curated registry.  The pin is advisory: if the model is not
//! available locally the orchestrator will fall back to the largest official
//! model that fits RAM.
//!
//! # Inputs (supplied by the orchestrator)
//!
//! - `scene_text`        : the chapter or scene prose, as plain text.
//! - `style_book_json`   : the project's `StyleBook` (em-dash, quote style,
//!                         Oxford comma, capitalisation rules, custom rules).
//! - `vocab_json`        : layered vocabulary (avoid / prefer / replace).
//! - `memory_excerpt`    : key facts, entity names, POV anchors for the scene.
//! - `genre`, `audience` : copied from the project brief.
//!
//! # Output schema (JSON, validated)
//!
//! ```json
//! {
//!   "revised_text": "string — the polished prose",
//!   "changes": [
//!     { "kind": "rewrite|tighten|reorder|word_swap|cut",
//!       "before": "string", "after": "string", "rationale": "string" }
//!   ],
//!   "summary": "string (≤120 words) — what changed and why",
//!   "confidence": "high|medium|low",
//!   "warnings": ["string", "..."]
//! }
//! ```

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

/// Stable model pin for the Final Review Editor.
pub const PINNED_MODEL: &str = "qwen3.6:latest";

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "voice-drift",
        description: "Polished prose loses the author's voice.",
        recoverable: true,
    },
    FailureMode {
        id: "fact-invention",
        description: "Edit introduces facts not present in source.",
        recoverable: false,
    },
    FailureMode {
        id: "model-unavailable",
        description: "Pinned qwen3.6 model not pulled locally.",
        recoverable: false,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "final-review-editor",
        name:             "Final Review Editor",
        purpose: "Final qualitative polish — rhythm, transitions, word choice, emotional clarity — \
                  to a publishable standard while preserving the author's voice fingerprint and \
                  every established fact. Runs last in the publish pipeline. Heavy: opt-in per session.",
        input_schema_id:  "FinalReviewInput",
        output_schema_id: "FinalReviewOutput",
        prompt_template:  PromptTemplateId::new("final-review-editor", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::LongContext,
            min_size: ModelSizeHint::ExtraLarge,
        },
        pinned_model: Some(PINNED_MODEL),
        context_budget: ContextBudget {
            max_context_tokens: 24_000,
            max_output_tokens:  8_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
            CrossCuttingValidator::EntitySanity,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run: WhenToRun::OnDemand,
        user_gate:   UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_fields_are_correct() {
        let s = spec();
        assert_eq!(s.id, "final-review-editor");
        assert_eq!(s.pinned_model, Some(PINNED_MODEL));
        assert_eq!(s.model_preference.min_size, ModelSizeHint::ExtraLarge);
        assert_eq!(s.user_gate, UserGate::Required);
        assert_eq!(s.context_budget.max_context_tokens, 24_000);
        assert_eq!(s.context_budget.max_output_tokens, 8_000);
    }
}
