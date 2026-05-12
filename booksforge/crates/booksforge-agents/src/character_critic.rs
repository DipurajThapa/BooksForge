//! Character Critic Agent — Stage 3 quality gate (Phase C).
//!
//! Reads the saved `CharacterBibleProposal` + brief, returns a
//! `CharacterCriticProposal` with:
//!   - Per-card 5-axis scores (depth, consistency, uniqueness,
//!     narrative_usefulness, emotional_impact).
//!   - Cross-card structural findings (duplicate names, dangling
//!     relationships, coverage sum > 105%, missing antagonist).
//!   - Per-card edit suggestions.
//!
//! Medium tier — the agent reads ~2-4 KB of structured bible + brief
//! and writes ~1 KB of structured JSON per character. A bible of 4
//! characters fits comfortably; larger bibles may need budget tuning.

use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "no-characters",
        description: "Character bible is empty — nothing to score.",
        recoverable: false,
    },
    FailureMode {
        id: "scores-mismatch-bible",
        description:
            "Returned scores don't match the bible's character names (typo or hallucination).",
        recoverable: true,
    },
    FailureMode {
        id: "praise-bot",
        description: "All cards score 9.0+ regardless of input quality — model is praising.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "character-critic",
        name:             "Character Critic",
        purpose:          "Score every CharacterCard in the saved bible on five axes (depth, consistency, uniqueness, narrative usefulness, emotional impact); flag cross-card structural issues; propose targeted revisions. Used as Stage 3's quality gate.",
        input_schema_id:  "CharacterCriticInput",
        output_schema_id: "CharacterCriticProposal",
        prompt_template:  PromptTemplateId::new("character-critic", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Bible + brief ~3 KB / ~750 tokens; output ~4 cards ×
            // ~800 tokens of JSON = ~3200 tokens. Add headroom for
            // bigger bibles up to ~8 characters.
            max_context_tokens: 6_000,
            max_output_tokens:  4_500,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes:   FAILURE_MODES,
        when_to_run:     WhenToRun::OnDemand,
        user_gate:       UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse a raw model response into a validated
/// `CharacterCriticProposal`. Clamps every per-axis score on every
/// card to [0, 10].
pub fn parse_and_validate(raw: &str) -> Result<booksforge_domain::CharacterCriticProposal, String> {
    let mut proposal: booksforge_domain::CharacterCriticProposal =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    proposal.clamp_all();
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_fields_are_correct() {
        let s = spec();
        assert_eq!(s.id, "character-critic");
        assert_eq!(s.default_thinking, DefaultThinking::Disabled);
    }

    #[test]
    fn parse_and_validate_clamps_out_of_range() {
        let raw = r#"{
            "scores": [{
                "character": "Ada",
                "depth":                { "score": 15.0 },
                "consistency":          { "score": -2.0 },
                "uniqueness":           { "score": 8.0 },
                "narrative_usefulness": { "score": 9.0 },
                "emotional_impact":     { "score": 9.0 }
            }]
        }"#;
        let p = parse_and_validate(raw).expect("parses");
        assert!(p.scores[0].depth.score <= 10.0);
        assert!(p.scores[0].consistency.score >= 0.0);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse_and_validate("not json").is_err());
    }

    #[test]
    fn parse_accepts_full_response() {
        let raw = r#"{
            "scores": [
                {
                    "character": "Ada",
                    "depth":                { "score": 9.0, "reason": "specific wound" },
                    "consistency":          { "score": 9.5, "reason": "fields cohere" },
                    "uniqueness":           { "score": 8.5, "reason": "distinct voice" },
                    "narrative_usefulness": { "score": 9.0, "reason": "drives plot" },
                    "emotional_impact":     { "score": 9.0, "reason": "reader will care" }
                }
            ],
            "cross_card_findings": [],
            "edits": [],
            "overall_summary": "A solid lead with one clear arc."
        }"#;
        let p = parse_and_validate(raw).expect("parses");
        assert!(p.passes_gate());
    }
}
