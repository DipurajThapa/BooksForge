//! World Bible Agent (BACKLOG §A13 — fiction-shaped).
//!
//! Companion to `character_bible`. Builds a world / setting bible from the
//! `ProjectBrief`: locations, social rules, history, sensory palette,
//! conflict sources, motifs, continuity constraints. Every part of the
//! bible is consumed by `scene-drafter-fic/v1` so the drafter has the
//! context the non-fiction drafter never needed.

use booksforge_domain::WorldBibleProposal;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "no-locations",
        description: "main_locations is empty — drafter cannot ground scenes.",
        recoverable: true,
    },
    FailureMode {
        id: "vague-sensory-palette",
        description: "Sensory palette uses generic mood-setters instead of specific details.",
        recoverable: true,
    },
    FailureMode {
        id: "no-continuity-constraints",
        description: "continuity_constraints is empty; bible has no falsifiable rules.",
        recoverable: true,
    },
    FailureMode {
        id: "history-too-thin",
        description: "history is < 30 words; backstory cannot shape scenes.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "world-bible",
        name:             "World Bible",
        purpose:          "Build a world / setting bible (locations, social rules, history, sensory palette, conflict sources, motifs, continuity constraints) from a ProjectBrief. Companion to character-bible; both feed scene-drafter-fic.",
        input_schema_id:  "WorldBibleInput",
        output_schema_id: "WorldBibleProposal",
        prompt_template:  PromptTemplateId::new("world-bible", "v1"),
        model_preference: ModelPreference {
            // World bibles benefit from larger models — sensory specificity
            // and internal-consistency rules are exactly the dimension small
            // models flatten.
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Large,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // PERMANENT HEADROOM (Run #13 fix). World bible is the
            // second `default_thinking: Enabled` agent and faces the
            // same Run #11 truncation risk as scene_drafter_fic.
            // qwen3.6:latest natively supports 256k context; 24k
            // total here is conservative and gives the model room
            // to think about location interactions, social rules,
            // and continuity constraints across multi-location
            // worlds without the JSON output getting truncated by
            // thinking-mode budget exhaustion.
            //   - 12k input — fits the brief + prior memory entries
            //     + creative profile.
            //   - 12k output — supports up to ~6k of <think> reasoning
            //     plus ~6k of bible JSON, comfortable for worlds with
            //     5-8 locations and full social-rule sets.
            max_context_tokens: 12_000,
            max_output_tokens:  12_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        // Run #16 — disabled. World bibles are STRUCTURED data
        // (location list + social rules + sensory palette etc.),
        // not creative prose. Thinking-mode added 4-6 min per call
        // for no measurable quality lift in the structured output —
        // the model thinks at length, then emits the same JSON it
        // would have emitted without thinking. Disabled for the
        // same reason scene_drafter_fic was: explicit reasoning
        // isn't earning its budget cost on this prompt class.
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse the model's raw output into a `WorldBibleProposal` and run the
/// proposal's `validate()`. Uses the workspace `json_repair` helper
/// (BACKLOG §A10) so a malformed list element is salvaged.
pub fn parse_and_validate(raw: &str) -> Result<WorldBibleProposal, String> {
    // `main_locations` is the only object-list at the top level; the
    // rest (social_rules, conflict_sources, motifs, continuity_constraints)
    // are list-of-strings.
    let (repaired, audit) =
        crate::json_repair::parse_and_repair_strict_objects(raw, &["main_locations"])?;
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "world-bible",
            dropped = audit.dropped_list_elements,
            "json_repair salvaged malformed list elements before deserialise",
        );
    }
    let proposal: WorldBibleProposal = serde_json::from_value(repaired)
        .map_err(|e| format!("JSON parse error after repair: {e}"))?;
    let errs = proposal.validate();
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
        assert_eq!(s.id, "world-bible");
        assert_eq!(s.input_schema_id, "WorldBibleInput");
        assert_eq!(s.output_schema_id, "WorldBibleProposal");
        assert!(matches!(s.user_gate, UserGate::Required));
    }

    #[test]
    fn parse_rejects_empty_locations() {
        let raw = r#"{
          "main_locations": [],
          "social_rules": ["one"],
          "history": "A short history that is at least thirty words long so the validator does not also flag the history field length as too short.",
          "sensory_palette": {"sight":"x","sound":"y","smell":"z","touch":"","taste":""},
          "conflict_sources": ["x"],
          "symbolic_motifs": ["m"],
          "continuity_constraints": ["c"]
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("main_locations is empty"));
    }

    #[test]
    fn parse_accepts_well_formed_world() {
        let raw = r#"{
          "main_locations": [
            {
              "name": "The Workshop",
              "purpose_in_story": "Ada's late husband's repair bench; where the wound clock arrives.",
              "sensory_signature": "wet wool, oil, the click of the door hinge that never quite closed",
              "key_constraints": "Ada has not touched the tools since his death."
            }
          ],
          "social_rules": [
            "small-town news travels by post office before phone",
            "widows are visited unannounced for the first six weeks"
          ],
          "history": "Ada and her husband ran the workshop for twenty-eight years before his death; the town leans on it for the small repairs the city folk do not bother with, and on her hands now for the same.",
          "sensory_palette": {
            "sight": "low gray light, dust on the bench",
            "sound": "the click of the wrong-side switch",
            "smell": "wet wool and old oil",
            "touch": "cold brass",
            "taste": "tea gone cold"
          },
          "conflict_sources": ["a hidden life she did not know about"],
          "symbolic_motifs": ["the wound clock", "the wrong-side light switch"],
          "continuity_constraints": [
            "Ada's husband died exactly six weeks before chapter 1",
            "The clock was wound when she found it; she did not wind it"
          ]
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "expected ok, got {res:?}");
    }

    #[test]
    fn parse_repairs_string_placeholder_in_list() {
        let raw = r#"{
          "main_locations": [
            {"name":"x","purpose_in_story":"y","sensory_signature":"specific detail","key_constraints":""},
            "loc_2_placeholder"
          ],
          "social_rules": ["rule one", "rule two"],
          "history": "This is a long-enough history that comfortably exceeds the thirty-word minimum so the validator does not flag this field as too thin in addition to the placeholder repair pass that we are exercising in this particular unit test of the world bible parser.",
          "sensory_palette": {"sight":"x","sound":"y","smell":"z","touch":"","taste":""},
          "conflict_sources": ["c"],
          "symbolic_motifs": ["m"],
          "continuity_constraints": ["c"]
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "json_repair should salvage; got {res:?}");
        assert_eq!(res.unwrap().main_locations.len(), 1);
    }
}
