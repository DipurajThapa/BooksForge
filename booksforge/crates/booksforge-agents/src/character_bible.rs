//! Character Bible Agent (BACKLOG §A13 — fiction-shaped).
//!
//! First-class fiction agent. Replaces the naked-LLM character-card prompt
//! that BF-E2E-LOCAL-LLM-FIRST-BOOK-001 Phase 5 had to use because no
//! first-class fiction agent existed.
//!
//! Input: `ProjectBrief` + the chapter count from the outline (so each
//! `CharacterCard.chapter_arc` has the right number of entries) + optional
//! accepted-prose samples (used to derive measurable voice traits).
//!
//! Output: `CharacterBibleProposal` (4–12 cards, one explicitly typed
//! `protagonist`, no duplicates, every card with measurable voice traits
//! and a per-chapter arc whose length matches the outline).

use booksforge_domain::CharacterBibleProposal;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "no-protagonist",
        description: "No character has role 'protagonist'.",
        recoverable: true,
    },
    FailureMode {
        id: "duplicate-name",
        description: "Two characters share the same name.",
        recoverable: true,
    },
    FailureMode {
        id: "vague-voice-traits",
        description: "Voice traits are subjective adjectives ('kind', 'smart') instead of measurable patterns.",
        recoverable: true,
    },
    FailureMode {
        id: "chapter-arc-mismatch",
        description: "chapter_arc length does not match the declared chapter count.",
        recoverable: true,
    },
    FailureMode {
        id: "broken-relationship",
        description: "A relationship's `to` references a name not in the bible.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "character-bible",
        name:             "Character Bible",
        purpose:          "Build a per-character bible (objective, internal need, wound, secret, voice traits, per-chapter arc) from a ProjectBrief plus optionally accepted prose. Replaces the prior naked-LLM prompt; first-class fiction agent.",
        input_schema_id:  "CharacterBibleInput",
        output_schema_id: "CharacterBibleProposal",
        prompt_template:  PromptTemplateId::new("character-bible", "v1"),
        model_preference: ModelPreference {
            // Bibles benefit from larger models — character interiority is
            // exactly the dimension small models flatten.
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Large,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 8_000,
            max_output_tokens:  6_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        // Bibles deserve thinking-mode on capable models — interiority and
        // motivation reasoning is non-trivial.
        default_thinking: DefaultThinking::Enabled,
    }
}

/// Parse the model's raw output into a `CharacterBibleProposal` and run
/// the proposal's `validate()` against the expected chapter count from
/// the outline. Returns `Err(reason)` so the runner can retry.
///
/// Uses the workspace `json_repair` helper (BACKLOG §A10) so a malformed
/// list element (e.g. a string placeholder slipping into `characters`)
/// is salvaged rather than crashing the whole call.
pub fn parse_and_validate(
    raw: &str,
    expected_chapter_count: usize,
) -> Result<CharacterBibleProposal, String> {
    // Strict-object lists at the schema's known dict-typed keys; nulls
    // dropped from the rest (voice_traits, chapter_arc, etc.).
    let (repaired, audit) =
        crate::json_repair::parse_and_repair_strict_objects(raw, &["characters", "relationships"])?;
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "character-bible",
            dropped = audit.dropped_list_elements,
            "json_repair salvaged malformed list elements before deserialise",
        );
    }
    let proposal: CharacterBibleProposal = serde_json::from_value(repaired)
        .map_err(|e| format!("JSON parse error after repair: {e}"))?;
    let errs = proposal.validate(expected_chapter_count);
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "character-bible");
        assert_eq!(s.input_schema_id, "CharacterBibleInput");
        assert_eq!(s.output_schema_id, "CharacterBibleProposal");
        assert!(matches!(s.user_gate, UserGate::Required));
        assert_eq!(s.failure_modes.len(), 5);
    }

    #[test]
    fn parse_rejects_no_protagonist() {
        let raw = r#"{
          "characters": [
            {
              "name": "Bryn",
              "role": "antagonist",
              "external_objective": "stop Ada",
              "internal_need": "be seen",
              "fear_or_wound": "abandonment",
              "secret_or_contradiction": "x",
              "voice_traits": ["short sentences", "rhetorical questions"],
              "relationships": [],
              "chapter_arc": ["a", "b"],
              "emotional_turning_points": ["p1", "p2"]
            },
            {
              "name": "Cal",
              "role": "supporting",
              "external_objective": "ferry Ada",
              "internal_need": "redemption",
              "fear_or_wound": "x",
              "secret_or_contradiction": "x",
              "voice_traits": ["trails off mid-sentence"],
              "relationships": [],
              "chapter_arc": ["a", "b"],
              "emotional_turning_points": ["x"]
            }
          ]
        }"#;
        let res = parse_and_validate(raw, 2);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("no character has role 'protagonist'"));
    }

    #[test]
    fn parse_accepts_well_formed_bible() {
        let raw = r#"{
          "characters": [
            {
              "name": "Ada",
              "role": "protagonist",
              "external_objective": "find the sender of the wound clock",
              "internal_need": "let go of the man she thought she married",
              "fear_or_wound": "her late husband had a hidden life",
              "secret_or_contradiction": "her hands remember more than she does",
              "voice_traits": ["short declaratives", "no hedge openers", "ends scenes mid-sentence"],
              "relationships": [{"to": "Bryn", "nature": "estranged friend"}],
              "chapter_arc": ["doubts the clock is real", "drives to ask"],
              "emotional_turning_points": ["finds the photograph", "decides not to know"]
            },
            {
              "name": "Bryn",
              "role": "antagonist",
              "external_objective": "keep her own husband's secret",
              "internal_need": "be forgiven",
              "fear_or_wound": "her loyalty has a price",
              "secret_or_contradiction": "she knows who H is",
              "voice_traits": ["over-explains", "uses hedges to evade"],
              "relationships": [{"to": "Ada", "nature": "estranged friend"}],
              "chapter_arc": ["fields the question", "refuses to answer"],
              "emotional_turning_points": ["the postmaster pause"]
            }
          ]
        }"#;
        let res = parse_and_validate(raw, 2);
        assert!(res.is_ok(), "expected ok, got {res:?}");
    }

    #[test]
    fn parse_repairs_string_placeholder_in_list() {
        // The exact failure mode from BF-E2E Phase 5.
        let raw = r#"{
          "characters": [
            {
              "name": "Ada",
              "role": "protagonist",
              "external_objective": "x",
              "internal_need": "y",
              "fear_or_wound": "z",
              "secret_or_contradiction": "s",
              "voice_traits": ["short sentences"],
              "relationships": [],
              "chapter_arc": ["a", "b"],
              "emotional_turning_points": ["t"]
            },
            "characters_2",
            {
              "name": "Bryn",
              "role": "antagonist",
              "external_objective": "x",
              "internal_need": "y",
              "fear_or_wound": "z",
              "secret_or_contradiction": "s",
              "voice_traits": ["clipped"],
              "relationships": [{"to":"Ada","nature":"foil"}],
              "chapter_arc": ["a", "b"],
              "emotional_turning_points": ["t"]
            }
          ]
        }"#;
        let res = parse_and_validate(raw, 2);
        assert!(res.is_ok(), "json_repair should salvage; got {res:?}");
        assert_eq!(res.unwrap().characters.len(), 2);
    }
}
