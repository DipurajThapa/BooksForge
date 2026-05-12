//! Reader Expectation Map — Stage 2's generated output.
//!
//! Produced by the `audience-mapper` agent from a saved `ProjectBrief`.
//! Structured so the scene drafter, polish stack, and content-quality
//! pass can each read the specific axis they care about:
//!   - drafter reads `emotional_promises` + `recommended_themes` + `pacing_expectation`
//!   - polish reads `tropes_to_avoid`
//!   - dev-edit reads `genre_anti_patterns`
//!
//! Persisted to `book:audience_map` memory (separate key from
//! `book:project_brief` so re-running this agent doesn't overwrite the
//! Setup brief — both can be regenerated independently).
//!
//! Same `#[serde(default)]` schema-tolerance pattern as the rest of
//! the agent outputs (see 2026-05-11 relaxation in `outline.rs`).

use serde::{Deserialize, Deserializer, Serialize};

/// Reader-pacing expectation. Drives the scene drafter's escalation
/// pattern + the polish stack's chapter-hook length budget.
///
/// Note: this enum is deserialised via a tolerant helper (see
/// `deserialize_pacing_tolerant` below) so an unknown / typo'd
/// pacing string from a misbehaving 9B model falls back to
/// `SlowBuild` rather than failing the whole `ReaderExpectationMap`
/// parse. This mirrors the `#[serde(default)]` schema-tolerance
/// pattern used everywhere else in Phase C.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PacingExpectation {
    /// Quiet build with deferred payoff. Literary fiction, slow-burn
    /// thrillers. Polish targets longer rhythm + subtext.
    SlowBuild,
    /// High-velocity per-chapter resolution + open question into the
    /// next. Genre fiction, thriller, YA romance.
    PageTurner,
    /// Each chapter / poem stands alone; the through-line is thematic
    /// rather than plot-causal. Short fiction collections, essays.
    Episodic,
    /// Voice-led with extended figurative passages. Poetry-adjacent
    /// literary; trades plot velocity for image density.
    Lyrical,
}

impl Default for PacingExpectation {
    fn default() -> Self {
        Self::SlowBuild
    }
}

/// Tolerant deserializer for the `pacing_expectation` field. Falls
/// back to `PacingExpectation::default()` (SlowBuild) when the field
/// is missing, null, not a string, or a string we don't recognise.
/// Never errors out — a bad pacing value must not kill the whole
/// `ReaderExpectationMap` parse and lose the other six lists with it.
///
/// We route through `serde_json::Value` rather than `Option<String>`
/// because serde's stream-deserialise cannot recover its position
/// after a type mismatch (a numeric `42` would surface a parser
/// error *outside* this helper). `Value` is the universal accept-
/// anything bridge that always consumes exactly one JSON value.
fn deserialize_pacing_tolerant<'de, D>(deserializer: D) -> Result<PacingExpectation, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    Ok(match v.as_str() {
        Some("slow_build") => PacingExpectation::SlowBuild,
        Some("page_turner") => PacingExpectation::PageTurner,
        Some("episodic") => PacingExpectation::Episodic,
        Some("lyrical") => PacingExpectation::Lyrical,
        _ => PacingExpectation::default(),
    })
}

/// Output of the `audience-mapper` agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReaderExpectationMap {
    /// What readers in this genre / sub-genre expect to find. The
    /// scene drafter reads these to keep the book "in the room"
    /// with the genre.
    #[serde(default)]
    pub genre_expectations: Vec<String>,
    /// Things readers in this genre actively dislike. The polish
    /// stack down-weights drafts that drift into these patterns.
    #[serde(default)]
    pub genre_anti_patterns: Vec<String>,
    /// Emotional promises the book should fulfil — the feelings
    /// the writer is signing up to deliver. Drives scene-critic's
    /// emotional-beat scoring.
    #[serde(default)]
    pub emotional_promises: Vec<String>,
    /// Themes the agent recommends the writer braid through. Read
    /// per-scene by the drafter via `creative_profile`.
    #[serde(default)]
    pub recommended_themes: Vec<String>,
    /// Genre-shaped tropes worth using — distinct from themes, these
    /// are structural patterns (locked-room, slow-burn romance,
    /// found-family rescue). Optional.
    #[serde(default)]
    pub recommended_tropes: Vec<String>,
    /// Tropes the brief / writer marked as forbidden + tropes the
    /// agent flags as overused for the genre.
    #[serde(default)]
    pub tropes_to_avoid: Vec<String>,
    /// One enum value. Drives drafter pacing + polish stack ordering.
    /// Tolerant of unknown strings — see `deserialize_pacing_tolerant`.
    #[serde(default, deserialize_with = "deserialize_pacing_tolerant")]
    pub pacing_expectation: PacingExpectation,
    /// One-paragraph editorial note — the human-readable summary the
    /// writer reads alongside the structured lists. Optional.
    #[serde(default)]
    pub overall_note: String,
}

impl ReaderExpectationMap {
    /// Gate for Stage 2: the map is "complete enough" when each of
    /// the five lists has at least one entry. Empty lists tell us the
    /// agent failed to extract a signal from the brief — usually
    /// because the brief itself was too thin.
    pub fn is_complete(&self) -> bool {
        !self.genre_expectations.is_empty()
            && !self.emotional_promises.is_empty()
            && !self.recommended_themes.is_empty()
            && !self.tropes_to_avoid.is_empty()
    }

    /// Total content density (sum of list lengths). Used as a soft
    /// completeness score the UI can show alongside the gate.
    pub fn total_entries(&self) -> usize {
        self.genre_expectations.len()
            + self.genre_anti_patterns.len()
            + self.emotional_promises.len()
            + self.recommended_themes.len()
            + self.recommended_tropes.len()
            + self.tropes_to_avoid.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pacing_is_slow_build() {
        assert_eq!(PacingExpectation::default(), PacingExpectation::SlowBuild);
    }

    #[test]
    fn empty_map_is_not_complete() {
        let m = ReaderExpectationMap::default();
        assert!(!m.is_complete());
        assert_eq!(m.total_entries(), 0);
    }

    #[test]
    fn map_with_all_required_lists_is_complete() {
        let m = ReaderExpectationMap {
            genre_expectations: vec!["specific sensory detail".into()],
            genre_anti_patterns: vec![],
            emotional_promises: vec!["a long quiet ache that resolves into clarity".into()],
            recommended_themes: vec!["inheritance as silence".into()],
            recommended_tropes: vec![],
            tropes_to_avoid: vec!["chosen-one prophecy".into()],
            pacing_expectation: PacingExpectation::SlowBuild,
            overall_note: String::new(),
        };
        assert!(m.is_complete());
        assert_eq!(m.total_entries(), 4);
    }

    #[test]
    fn map_missing_emotional_promises_is_not_complete() {
        let m = ReaderExpectationMap {
            genre_expectations: vec!["x".into()],
            recommended_themes: vec!["y".into()],
            tropes_to_avoid: vec!["z".into()],
            ..Default::default()
        };
        assert!(
            !m.is_complete(),
            "missing emotional_promises must fail gate"
        );
    }

    #[test]
    fn pacing_serialises_as_snake_case() {
        let m = ReaderExpectationMap {
            pacing_expectation: PacingExpectation::PageTurner,
            ..Default::default()
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"page_turner\""), "got: {json}");
    }

    #[test]
    fn deserialise_tolerates_missing_optional_fields() {
        // A minimal model response with only the required arrays.
        let json = r#"{
            "genre_expectations":  ["specific sensory detail"],
            "emotional_promises":  ["clarity after long ache"],
            "recommended_themes":  ["inheritance as silence"],
            "tropes_to_avoid":     ["chosen-one prophecy"]
        }"#;
        let m: ReaderExpectationMap = serde_json::from_str(json).expect("parses");
        assert!(m.is_complete());
        assert_eq!(m.pacing_expectation, PacingExpectation::SlowBuild);
        assert_eq!(m.recommended_tropes.len(), 0);
        assert_eq!(m.overall_note, "");
    }

    #[test]
    fn unknown_pacing_string_falls_back_to_default() {
        // Schema tolerance — an unknown pacing string must not kill
        // the whole parse. The map should land with default pacing
        // and the rest of its lists preserved.
        let json = r#"{
            "genre_expectations":  ["specific sensory detail"],
            "emotional_promises":  ["clarity after long ache"],
            "recommended_themes":  ["inheritance as silence"],
            "tropes_to_avoid":     ["chosen-one prophecy"],
            "pacing_expectation":  "warp_speed"
        }"#;
        let m: ReaderExpectationMap = serde_json::from_str(json).expect("parses tolerantly");
        assert_eq!(m.pacing_expectation, PacingExpectation::default());
        assert!(m.is_complete(), "other lists must survive");
    }

    #[test]
    fn null_pacing_falls_back_to_default() {
        let json = r#"{ "pacing_expectation": null }"#;
        let m: ReaderExpectationMap = serde_json::from_str(json).expect("parses");
        assert_eq!(m.pacing_expectation, PacingExpectation::default());
    }

    #[test]
    fn non_string_pacing_falls_back_to_default() {
        // Model returns a number or object where a string was expected.
        let json = r#"{ "pacing_expectation": 42 }"#;
        let m: ReaderExpectationMap = serde_json::from_str(json).expect("parses");
        assert_eq!(m.pacing_expectation, PacingExpectation::default());

        let json = r#"{ "pacing_expectation": { "kind": "page_turner" } }"#;
        let m: ReaderExpectationMap = serde_json::from_str(json).expect("parses");
        assert_eq!(m.pacing_expectation, PacingExpectation::default());
    }

    #[test]
    fn every_known_pacing_variant_round_trips() {
        for (s, expected) in [
            ("slow_build", PacingExpectation::SlowBuild),
            ("page_turner", PacingExpectation::PageTurner),
            ("episodic", PacingExpectation::Episodic),
            ("lyrical", PacingExpectation::Lyrical),
        ] {
            let json = format!(r#"{{ "pacing_expectation": "{s}" }}"#);
            let m: ReaderExpectationMap = serde_json::from_str(&json).expect("parses");
            assert_eq!(m.pacing_expectation, expected, "for input {s}");
        }
    }
}
