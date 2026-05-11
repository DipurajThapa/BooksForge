//! Outline Architect Agent specification.
//!
//! Proposes a chapter/scene outline from a `ProjectBrief`.
//! Per AGENTS.md §4.2.

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "premise-too-thin",
        description: "Brief premise lacks enough detail to outline.",
        recoverable: true,
    },
    FailureMode {
        id: "duplicate-synopses",
        description: "Two scene synopses share >40% tokens.",
        recoverable: true,
    },
    FailureMode {
        id: "wrong-chapter-count",
        description: "Output chapter count off the requested target by >20%.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "outline-architect",
        name:             "Outline Architect",
        purpose:          "Propose a chapter and scene outline that delivers the ProjectBrief's promises. Each chapter has a one-sentence purpose; each scene has a one-sentence synopsis with optional POV and beat. Does not write prose. Does not invent characters or facts absent from the brief.",
        input_schema_id:  "OutlineArchitectInput",
        output_schema_id: "OutlineProposal",
        prompt_template:  PromptTemplateId::new("outline-architect", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::LongContext,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        // Brief + prompt ≤ 6 000 tokens; output ≤ 8 000 tokens (AGENTS.md §4.2).
        context_budget: ContextBudget {
            max_context_tokens: 6_000,
            max_output_tokens:  8_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run: WhenToRun::OnDemand,
        user_gate:   UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Semantic validators applied after JSON-schema validation.
///
/// Returns a list of human-readable error strings; empty = valid.
pub fn validate_semantic(
    proposal: &booksforge_domain::OutlineProposal,
    target_chapter_count: u32,
    brief_word_count: u32,
) -> Vec<String> {
    let mut errors = proposal.validate(target_chapter_count, brief_word_count);

    // Check for near-duplicate synopses (>40 % shared tokens ≈ simple word set overlap).
    let all_synopses: Vec<&str> = proposal
        .parts
        .iter()
        .flat_map(|p| p.chapters.iter())
        .flat_map(|c| c.scenes.iter())
        .map(|s| s.synopsis.as_str())
        .collect();

    for i in 0..all_synopses.len() {
        for j in (i + 1)..all_synopses.len() {
            if synopsis_overlap(all_synopses[i], all_synopses[j]) > 0.40 {
                errors.push(format!(
                    "synopses [{i}] and [{j}] share >40% tokens — outline is too generic"
                ));
            }
        }
    }

    errors
}

/// Simple Jaccard-like overlap on lowercase word tokens.
fn synopsis_overlap(a: &str, b: &str) -> f64 {
    let tokens_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let tokens_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 1.0;
    }
    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_fields_are_correct() {
        let s = spec();
        assert_eq!(s.id, "outline-architect");
        assert_eq!(
            s.prompt_template,
            PromptTemplateId::new("outline-architect", "v1")
        );
        assert_eq!(s.user_gate, UserGate::Required);
        assert_eq!(s.context_budget.max_context_tokens, 6_000);
        assert_eq!(s.context_budget.max_output_tokens, 8_000);
    }

    #[test]
    fn identical_synopses_flagged_as_duplicate() {
        assert!(
            synopsis_overlap("the hero meets the villain", "the hero meets the villain") > 0.40
        );
    }

    #[test]
    fn distinct_synopses_not_flagged() {
        assert!(synopsis_overlap("a moonlit forest chase", "dawn breaks over the citadel") < 0.40);
    }
}
