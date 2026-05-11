//! Character Bible **Card** Agent — per-character variant
//!
//! Companion to `character_bible` (the monolithic agent). Generates
//! ONE `CharacterCard` per invocation. Used by the orchestrator's
//! chunked-bibles helper which calls this agent N times (one per
//! desired character role) and stitches the responses into a
//! `CharacterBibleProposal`.
//!
//! The chunked approach is the Round 7 RCA fix for bottleneck #3
//! where the monolithic agent's single-shot 4-6 nested-objects
//! response exceeded small-model competence: qwen3.5:9b spent 11.6
//! min on the monolithic prompt and returned an empty array because
//! the runner cycled through max retries on validation failures.
//! Per-character output (~250-400 tokens) fits in a 9B model's
//! single-shot competence window. Failure isolation means a bad
//! character N triggers retry of just N, not the whole bible.

use booksforge_domain::CharacterCard;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "duplicate-name",
        description: "Generated character name matches a name in `prior_characters`.",
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
        description: "A relationship's `to` references a name not in `prior_characters` (or this card's own name).",
        recoverable: true,
    },
    FailureMode {
        id: "wrapped-output",
        description: "Model wrapped the response in {\"characters\": [...]} instead of emitting a bare CharacterCard.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "character-bible-card",
        name:             "Character Bible (single card)",
        purpose:          "Generate ONE CharacterCard. The orchestrator calls this agent N times to assemble a CharacterBibleProposal — fixes the Round 7 RCA where the monolithic character_bible exceeded small-model competence.",
        input_schema_id:  "CharacterCardInput",
        output_schema_id: "CharacterCard",
        prompt_template:  PromptTemplateId::new("character-bible-card", "v1"),
        model_preference: ModelPreference {
            // Per-card emission is small enough that medium models
            // suffice (in fact, that's the whole point of chunking).
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // One card; ~200-400 tokens output, prompt ~1500 with
            // brief + prior_characters list.
            max_context_tokens: 4_000,
            max_output_tokens:  1_200,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        // Per-card generation needs less reasoning headroom than the
        // full bible — keep thinking-mode off to save wall-clock.
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Every field name the model is allowed to emit anywhere in a
/// `CharacterCard` payload (top-level + nested `relationships`).
/// Used by the schema-aware self-healing pass in [`parse_and_validate`]
/// to rename obvious typos (the Run #11 `external_object` → `external_objective`
/// case being the canonical example).
///
/// `characters` is included so the wrapped-response model quirk
/// (`{"characters": [...]}`) doesn't get rewritten by the healer
/// before the unwrap step removes it.
const ALLOWED_FIELD_NAMES: &[&str] = &[
    // wrapper key (tolerated model quirk)
    "characters",
    // CharacterCard top-level
    "name",
    "role",
    "external_objective",
    "internal_need",
    "fear_or_wound",
    "secret_or_contradiction",
    "voice_traits",
    "relationships",
    "chapter_arc",
    "emotional_turning_points",
    // CharacterRelationship
    "to",
    "nature",
];

/// Parse the model's raw output into a `CharacterCard`. The schema
/// permits one model quirk: when the model wraps the response as
/// `{"characters": [card]}` we unwrap and return the first element.
/// Otherwise we expect a bare `CharacterCard` object.
///
/// FEATURE_HARDENING_PLAN.md §2.5 first half — runs the input through
/// `json_repair::parse_and_repair_with_schema_keys` BEFORE serde
/// deserialise. This heals field-name typos (the Run #11 card #2
/// failure where the model emitted `"external_object"` for the schema's
/// `"external_objective"`, edit distance 4, normalized 0.22 — admitted
/// by the default `RepairPolicy`). Renames are logged via `tracing::warn`
/// so the audit ledger surfaces what was healed.
pub fn parse_and_validate(
    raw: &str,
    expected_chapter_count: usize,
    prior_names: &[String],
) -> Result<CharacterCard, String> {
    // Accept either bare `{...}` (preferred) or wrapped
    // `{"characters": [{...}]}` (model quirk; tolerated). Strip
    // common ```json code-fence wrappers some local models emit.
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix("```json") {
        s = rest.trim();
    } else if let Some(rest) = s.strip_prefix("```") {
        s = rest.trim();
    }
    if let Some(rest) = s.strip_suffix("```") {
        s = rest.trim();
    }
    let (parsed, audit) =
        crate::json_repair::parse_and_repair_with_schema_keys(s, ALLOWED_FIELD_NAMES)
            .map_err(|e| format!("JSON parse error: {e}"))?;
    if !audit.field_renames.is_empty() {
        tracing::warn!(
            agent   = "character-bible-card",
            renames = ?audit.field_renames,
            "schema-aware repair healed {} field-name typo(s) before deserialise",
            audit.field_renames.len(),
        );
    }
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "character-bible-card",
            dropped = audit.dropped_list_elements,
            "schema-aware repair dropped {} malformed list element(s)",
            audit.dropped_list_elements,
        );
    }
    let card_value = match parsed {
        serde_json::Value::Object(ref m) if m.contains_key("characters") => m
            .get("characters")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .cloned()
            .ok_or_else(|| "wrapped response had empty `characters` array".to_owned())?,
        v => v,
    };
    let card: CharacterCard = serde_json::from_value(card_value)
        .map_err(|e| format!("CharacterCard parse error: {e}"))?;
    let errs = validate_card(&card, expected_chapter_count, prior_names);
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(card)
}

/// Per-card semantic validator. Mirrors the per-character checks
/// inside `CharacterBibleProposal::validate` but operates on a single
/// card with the prior-card names supplied externally. The full-bible
/// cross-validation (no-protagonist-missing, no-duplicate-objectives)
/// runs once after all cards are stitched.
pub fn validate_card(
    card: &CharacterCard,
    expected_chapter_count: usize,
    prior_names: &[String],
) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    if card.name.trim().is_empty() {
        errors.push("name is empty".to_owned());
    }
    if card.external_objective.trim().is_empty() {
        errors.push(format!("'{}': external_objective is empty", card.name));
    }
    if card.internal_need.trim().is_empty() {
        errors.push(format!("'{}': internal_need is empty", card.name));
    }
    if card.voice_traits.is_empty() || card.voice_traits.len() > 6 {
        errors.push(format!(
            "'{}': voice_traits must be 1-6 entries, got {}",
            card.name,
            card.voice_traits.len()
        ));
    }
    if card.voice_traits.iter().any(|t| t.trim().is_empty()) {
        errors.push(format!("'{}': voice_traits has an empty entry", card.name));
    }
    if expected_chapter_count > 0 && card.chapter_arc.len() != expected_chapter_count {
        errors.push(format!(
            "'{}': chapter_arc has {} entries, expected {}",
            card.name,
            card.chapter_arc.len(),
            expected_chapter_count
        ));
    }
    if prior_names.iter().any(|n| n == &card.name) {
        errors.push(format!(
            "'{}': name collides with an already-generated character",
            card.name
        ));
    }
    // Relationships may reference any prior name OR this card's own
    // name (self-references like "married to herself" are nonsense
    // but not outright errors at this layer).
    for rel in &card.relationships {
        if rel.to == card.name {
            continue;
        }
        if !prior_names.iter().any(|n| n == &rel.to) {
            errors.push(format!(
                "'{}': relationship to unknown name '{}' (must be a prior character)",
                card.name, rel.to,
            ));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_card_json() -> &'static str {
        r#"{
          "name": "Ada",
          "role": "protagonist",
          "external_objective": "find the sender of the wound clock",
          "internal_need": "let go of the man she thought she married",
          "fear_or_wound": "her late husband had a hidden life",
          "secret_or_contradiction": "her hands remember more than she does",
          "voice_traits": ["short declaratives", "no hedge openers", "ends scenes mid-sentence"],
          "relationships": [],
          "chapter_arc": ["doubts the clock is real", "drives to ask"],
          "emotional_turning_points": ["finds the photograph", "decides not to know"]
        }"#
    }

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "character-bible-card");
        assert_eq!(s.output_schema_id, "CharacterCard");
        assert!(matches!(s.user_gate, UserGate::Required));
        assert_eq!(s.failure_modes.len(), 5);
    }

    #[test]
    fn parse_accepts_bare_card() {
        let r = parse_and_validate(good_card_json(), 2, &[]);
        assert!(r.is_ok(), "expected ok, got {r:?}");
    }

    #[test]
    fn parse_unwraps_wrapped_response() {
        let wrapped = format!(r#"{{"characters": [{}]}}"#, good_card_json());
        let r = parse_and_validate(&wrapped, 2, &[]);
        assert!(r.is_ok(), "expected ok, got {r:?}");
    }

    #[test]
    fn parse_rejects_duplicate_name() {
        let r = parse_and_validate(good_card_json(), 2, &["Ada".to_owned()]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("collides"));
    }

    #[test]
    fn parse_rejects_chapter_arc_mismatch() {
        let r = parse_and_validate(good_card_json(), 5, &[]);
        assert!(r.is_err());
        assert!(r
            .unwrap_err()
            .contains("chapter_arc has 2 entries, expected 5"));
    }

    #[test]
    fn parse_rejects_relationship_to_unknown() {
        let json = r#"{
          "name": "Cal",
          "role": "supporting",
          "external_objective": "ferry Ada home",
          "internal_need": "redemption",
          "fear_or_wound": "his last passenger",
          "secret_or_contradiction": "knows the back roads",
          "voice_traits": ["clipped", "all questions"],
          "relationships": [{"to": "Bryn", "nature": "friend"}],
          "chapter_arc": ["arrives", "leaves"],
          "emotional_turning_points": ["the river"]
        }"#;
        // Bryn isn't in prior_names → invalid.
        let r = parse_and_validate(json, 2, &["Ada".to_owned()]);
        assert!(r.is_err());
        assert!(r
            .unwrap_err()
            .contains("relationship to unknown name 'Bryn'"));
    }

    #[test]
    fn parse_accepts_relationship_to_prior() {
        let json = r#"{
          "name": "Cal",
          "role": "supporting",
          "external_objective": "ferry Ada home",
          "internal_need": "redemption",
          "fear_or_wound": "his last passenger",
          "secret_or_contradiction": "knows the back roads",
          "voice_traits": ["clipped", "all questions"],
          "relationships": [{"to": "Ada", "nature": "friend"}],
          "chapter_arc": ["arrives", "leaves"],
          "emotional_turning_points": ["the river"]
        }"#;
        let r = parse_and_validate(json, 2, &["Ada".to_owned()]);
        assert!(r.is_ok(), "expected ok, got {r:?}");
    }

    // ── Schema-aware self-healing (FEATURE_HARDENING_PLAN.md §2.5 first half)

    #[test]
    fn parse_self_heals_run11_external_object_typo() {
        // The exact Run #11 card #2 failure: model emitted
        // `external_object` for the schema's `external_objective`
        // (edit distance 4, normalized 4/18 = 0.22). Pre-§2.5 this
        // card was rejected outright; the chunked-bible run had to
        // rely on its lenient retry policy. Now the default
        // RepairPolicy heals it automatically.
        let json = r#"{
          "name": "Ada",
          "role": "protagonist",
          "external_object": "find the sender of the wound clock",
          "internal_need": "let go of the man she thought she married",
          "fear_or_wound": "her late husband had a hidden life",
          "secret_or_contradiction": "her hands remember more than she does",
          "voice_traits": ["short declaratives", "no hedge openers", "ends scenes mid-sentence"],
          "relationships": [],
          "chapter_arc": ["doubts the clock is real", "drives to ask"],
          "emotional_turning_points": ["finds the photograph", "decides not to know"]
        }"#;
        let r = parse_and_validate(json, 2, &[]);
        assert!(
            r.is_ok(),
            "Run #11 typo case must self-heal at default policy, got: {r:?}"
        );
        let card = r.unwrap();
        assert_eq!(
            card.external_objective,
            "find the sender of the wound clock"
        );
    }

    #[test]
    fn parse_self_heals_voice_traits_typo() {
        let json = r#"{
          "name": "Ada",
          "role": "protagonist",
          "external_objective": "find the sender",
          "internal_need": "let go",
          "fear_or_wound": "the hidden life",
          "secret_or_contradiction": "her hands remember",
          "voce_traits": ["short", "punchy", "specific"],
          "relationships": [],
          "chapter_arc": ["doubts", "drives"],
          "emotional_turning_points": ["the photograph", "the silence"]
        }"#;
        let r = parse_and_validate(json, 2, &[]);
        assert!(
            r.is_ok(),
            "voce_traits → voice_traits should heal, got: {r:?}"
        );
        let card = r.unwrap();
        assert_eq!(card.voice_traits.len(), 3);
    }

    #[test]
    fn parse_does_not_clobber_when_both_keys_present() {
        // The model emitted BOTH the typo'd key AND the canonical key.
        // The healer must not silently drop the canonical value by
        // overwriting it with the typo's value.
        let json = r#"{
          "name": "Ada",
          "role": "protagonist",
          "external_objective": "the real one",
          "external_object": "the typo'd one",
          "internal_need": "x",
          "fear_or_wound": "x",
          "secret_or_contradiction": "x",
          "voice_traits": ["short"],
          "relationships": [],
          "chapter_arc": ["a", "b"],
          "emotional_turning_points": ["x"]
        }"#;
        let r = parse_and_validate(json, 2, &[]);
        assert!(r.is_ok(), "expected ok, got {r:?}");
        let card = r.unwrap();
        assert_eq!(
            card.external_objective, "the real one",
            "canonical-keyed value must survive when both keys are present",
        );
    }
}
