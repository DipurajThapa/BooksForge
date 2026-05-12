//! Shared scaffolding for the 4 voice-preserving polish stages
//! (BACKLOG §A15 / Phase 2 of `PRODUCT_ROADMAP_E2E.md`).
//!
//! Each polish stage is a separate agent with its own prompt template
//! and `spec()`, but they all consume the same `PolishProposal` shape
//! and share the same parse + validate path.

use booksforge_domain::{PolishProposal, PolishStageId};

/// Parse a model's raw output into a typed `PolishProposal` and run
/// `PolishProposal::validate()`. Uses the workspace's default
/// json_repair (null-only — `revised_pm_doc.content` is always a list of
/// objects so a stray null is the realistic failure mode). Verifies the
/// returned `stage_id` matches the expected stage so a mis-routed
/// response can't silently land in the wrong audit bucket.
pub fn parse_and_validate_polish(
    raw: &str,
    expected: PolishStageId,
) -> Result<PolishProposal, String> {
    let (repaired, audit) = crate::json_repair::parse_and_repair(raw)?;
    if audit.dropped_list_elements > 0 {
        tracing::warn!(
            agent = "polish",
            stage = expected.as_str(),
            dropped = audit.dropped_list_elements,
            "json_repair salvaged malformed list elements before deserialise",
        );
    }
    let parsed: PolishProposal = serde_json::from_value(repaired)
        .map_err(|e| format!("JSON parse error after repair: {e}"))?;
    if parsed.stage_id != expected {
        return Err(format!(
            "stage_id mismatch: expected {:?}, got {:?}",
            expected, parsed.stage_id
        ));
    }
    let errs = parsed.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(parsed)
}
