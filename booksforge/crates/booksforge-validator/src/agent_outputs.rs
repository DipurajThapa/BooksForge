//! Per-agent output validators (BACKLOG §E5).
//!
//! Single-entry semantic validation for agent outputs.  Each agent in
//! `AGENTS.md` declares an `output_schema_id`; this module routes the
//! parsed JSON to the canonical validator on the matching domain type
//! (`CopyeditProposals::validate`, `ContinuityReport::validate`, etc.)
//! and returns a unified report.
//!
//! This deliberately wraps the per-type validators that already exist
//! on `booksforge_domain::agent_io::*` rather than reimplementing them.
//! The orchestrator's `proposal_validator::run_tier1` covers
//! cross-cutting checks (schema / contract / redaction / length /
//! originality); this module covers the *semantic* per-agent rules
//! that only make sense once the typed shape is known.
//!
//! ## Routing
//!
//! Schema ids are the strings declared on `AgentSpec.output_schema_id`.
//! Unknown schema ids report `validated: false` rather than erroring —
//! callers may treat that as "no validator wired yet".
//!
//! ## Example
//!
//! ```
//! use booksforge_validator::agent_outputs::{validate_agent_output, AgentOutputContext};
//! let json = serde_json::json!({"edits": [], "summary": "ok"});
//! let report = validate_agent_output(
//!     "CopyeditProposals",
//!     &json,
//!     AgentOutputContext { source_text: Some("hello world"), ..Default::default() },
//! );
//! assert!(report.errors.is_empty());
//! ```

use booksforge_domain::{
    ContinuityReport, CopyeditProposals, DevelopmentalNotes, HumanizationProposals,
    MemoryRefreshProposals, OutlineProposal, SceneDraftProposal, VocabUpdateProposals,
};

/// Optional context the per-agent validators may need.  Only populated
/// for agents that operate on a source text or have brief constraints.
#[derive(Debug, Default, Clone, Copy)]
pub struct AgentOutputContext<'a> {
    /// The scene text the agent was given (for `before`-matches-source
    /// checks on Copyeditor / Humanization / etc.).
    pub source_text:          Option<&'a str>,
    /// Target chapter count — used by `OutlineProposal::validate`.
    pub target_chapter_count: Option<u32>,
    /// Brief total word target — used by `OutlineProposal::validate`
    /// for the per-scene budget cross-check.
    pub brief_word_count:     Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AgentOutputReport {
    pub schema_id: String,
    /// Whether a validator was actually run.  False when the schema id
    /// is unknown (caller may treat as "no validator wired").
    pub validated: bool,
    /// Whether the typed parse from JSON succeeded.
    pub parse_ok:  bool,
    /// Per-rule semantic errors.  Empty == clean pass.
    pub errors:    Vec<String>,
}

impl AgentOutputReport {
    pub fn is_clean(&self) -> bool {
        self.validated && self.parse_ok && self.errors.is_empty()
    }
}

/// Validate `parsed` against the per-agent semantic rules for the
/// given `schema_id`.  Returns a uniform report regardless of which
/// agent the schema id resolves to.
///
/// Schema ids accepted (one per agent in `AGENTS.md §4`):
///   - `OutlineProposal`           (§4.2 Outline Architect)
///   - `SceneDraftProposal`        (§4.3 Chapter Drafter)
///   - `DevelopmentalNotes`        (§4.4 Developmental Editor)
///   - `ContinuityReport`          (§4.5 Continuity)
///   - `CopyeditProposals`         (§4.6 Copyeditor)
///   - `MemoryRefreshProposals`    (§4.7 Memory Curator)
///   - `VocabUpdateProposals`      (§4.8 Vocabulary Dictionary)
///   - `HumanizationProposals`     (§4.9 Humanization)
pub fn validate_agent_output(
    schema_id: &str,
    parsed:    &serde_json::Value,
    ctx:       AgentOutputContext<'_>,
) -> AgentOutputReport {
    let mut report = AgentOutputReport {
        schema_id: schema_id.to_owned(),
        validated: false,
        parse_ok:  false,
        errors:    Vec::new(),
    };

    macro_rules! parse_and_validate {
        ($ty:ty, $invoke:expr) => {{
            report.validated = true;
            match serde_json::from_value::<$ty>(parsed.clone()) {
                Ok(t) => {
                    report.parse_ok = true;
                    report.errors   = ($invoke)(&t);
                }
                Err(e) => {
                    report.parse_ok = false;
                    report.errors.push(format!("typed parse failed: {e}"));
                }
            }
        }};
    }

    match schema_id {
        "OutlineProposal" => {
            parse_and_validate!(OutlineProposal, |t: &OutlineProposal| {
                let chap = ctx.target_chapter_count.unwrap_or_else(|| {
                    t.parts.iter().map(|p| p.chapters.len() as u32).sum()
                });
                let wc   = ctx.brief_word_count.unwrap_or(0);
                t.validate(chap, wc)
            });
        }
        "SceneDraftProposal" => {
            parse_and_validate!(SceneDraftProposal, |t: &SceneDraftProposal| t.validate());
        }
        "DevelopmentalNotes" => {
            parse_and_validate!(DevelopmentalNotes, |t: &DevelopmentalNotes| t.validate());
        }
        "ContinuityReport" => {
            parse_and_validate!(ContinuityReport, |t: &ContinuityReport| t.validate());
        }
        "CopyeditProposals" => {
            parse_and_validate!(CopyeditProposals, |t: &CopyeditProposals| {
                t.validate(ctx.source_text.unwrap_or(""))
            });
        }
        "MemoryRefreshProposals" => {
            parse_and_validate!(MemoryRefreshProposals, |t: &MemoryRefreshProposals| t.validate());
        }
        "VocabUpdateProposals" => {
            parse_and_validate!(VocabUpdateProposals, |t: &VocabUpdateProposals| t.validate());
        }
        "HumanizationProposals" => {
            parse_and_validate!(HumanizationProposals, |t: &HumanizationProposals| {
                t.validate(ctx.source_text.unwrap_or(""))
            });
        }
        // Unknown schema id — caller may treat as "no validator wired".
        _ => {}
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_schema_id_returns_unvalidated() {
        let r = validate_agent_output("FrobnicateOutput", &json!({}), AgentOutputContext::default());
        assert!(!r.validated);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn copyedit_routes_to_validator_and_uses_source_text() {
        // `before`-matches-source enforcement requires source_text.
        let parsed = json!({
            "edits": [{
                "range_from": 0, "range_to": 5,
                "before": "WRONG",                 // mismatches source "hello"
                "after":  "WRONG",                 // also a no-op edit
                "category": "spelling",
                "rationale": "test"
            }],
            "summary": "x"
        });
        let r = validate_agent_output(
            "CopyeditProposals", &parsed,
            AgentOutputContext { source_text: Some("hello world"), ..Default::default() },
        );
        assert!(r.validated);
        assert!(r.parse_ok);
        assert!(r.errors.iter().any(|e| e.contains("fabricated position")),
                "expected fabricated-position error, got: {:#?}", r.errors);
    }

    #[test]
    fn scene_draft_rejects_empty_pm_doc() {
        let parsed = json!({
            "pm_doc": {"type": "doc", "content": []},
            "word_count": 0,
            "notes": ""
        });
        let r = validate_agent_output("SceneDraftProposal", &parsed, AgentOutputContext::default());
        assert!(!r.is_clean());
        assert!(r.errors.iter().any(|e| e.contains("empty")));
    }

    #[test]
    fn vocab_addition_validates_kind_enum() {
        let parsed = json!({
            "additions": [{
                "term": "very", "kind": "bogus_kind",
                "layer": "ai_tells",
                "replacement": null, "rationale": "x"
            }],
            "modifications": []
        });
        let r = validate_agent_output(
            "VocabUpdateProposals", &parsed, AgentOutputContext::default(),
        );
        assert!(r.errors.iter().any(|e| e.contains("not in enum")));
    }

    #[test]
    fn typed_parse_failure_is_reported() {
        // Missing required field `summary` on CopyeditProposals.
        let parsed = json!({"edits": []});
        let r = validate_agent_output(
            "CopyeditProposals", &parsed, AgentOutputContext::default(),
        );
        assert!(r.validated);
        assert!(!r.parse_ok);
        assert!(r.errors.iter().any(|e| e.contains("typed parse failed")));
    }

    #[test]
    fn clean_copyedit_returns_no_errors() {
        let source = "hello world";
        let parsed = json!({
            "edits": [{
                "range_from": 0, "range_to": 5,
                "before": "hello",
                "after":  "Hello",
                "category": "casing",
                "rationale": "sentence start"
            }],
            "summary": "one casing fix"
        });
        let r = validate_agent_output(
            "CopyeditProposals", &parsed,
            AgentOutputContext { source_text: Some(source), ..Default::default() },
        );
        assert!(r.is_clean(), "expected clean, got {:#?}", r.errors);
    }
}
