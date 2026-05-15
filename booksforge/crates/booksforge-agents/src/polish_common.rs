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
    // First try JSON. If the model emitted a well-shaped PolishProposal
    // (uncommon — the templates instruct "Return ONLY the revised
    // chapter — no commentary" which is plain prose), use it. Otherwise
    // fall through to bare-prose recovery.
    if let Ok((repaired, audit)) = crate::json_repair::parse_and_repair(raw) {
        if audit.dropped_list_elements > 0 {
            tracing::warn!(
                agent = "polish",
                stage = expected.as_str(),
                dropped = audit.dropped_list_elements,
                "json_repair salvaged malformed list elements before deserialise",
            );
        }
        if let Ok(parsed) = serde_json::from_value::<PolishProposal>(repaired) {
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
            return Ok(parsed);
        }
    }

    // Bare-prose recovery: the model returned plain markdown (the
    // contract the polish templates actually ask for). Synthesise a
    // minimal `PolishProposal` whose `revised_pm_doc` carries the prose
    // as one paragraph node per blank-separated block. Matches the
    // scene-drafter-fic recovery pattern for the same reason — the
    // template's natural output isn't a JSON struct.
    let prose = strip_code_fences(raw).trim().to_owned();
    if prose.is_empty() {
        return Err("polish stage returned empty output".to_owned());
    }
    let pm_doc = prose_to_pm_doc(&prose);
    let word_count = prose.split_whitespace().count() as u32;
    let proposal = PolishProposal {
        stage_id: expected,
        revised_pm_doc: pm_doc,
        revised_word_count: word_count,
        edit_notes: format!("(synthesised from bare-prose output; {word_count} words)"),
    };
    tracing::warn!(
        agent = "polish",
        stage = expected.as_str(),
        words = word_count,
        "synthesised PolishProposal from bare-prose output",
    );
    let errs = proposal.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(proposal)
}

/// Strip an optional triple-backtick fence the model added around its
/// prose output.
fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim_start();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip the optional language tag on the first line.
        let after_first_newline = rest.find('\n').map(|i| &rest[i + 1..]).unwrap_or(rest);
        if let Some(idx) = after_first_newline.rfind("```") {
            return after_first_newline[..idx].trim_end().to_owned();
        }
        return after_first_newline.to_owned();
    }
    raw.to_owned()
}

/// Build a minimal ProseMirror doc from blank-line-separated paragraphs.
fn prose_to_pm_doc(prose: &str) -> serde_json::Value {
    use serde_json::json;
    let paragraphs: Vec<&str> = prose
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let content: Vec<serde_json::Value> = paragraphs
        .iter()
        .map(|p| {
            let text: String = p.split_whitespace().collect::<Vec<_>>().join(" ");
            json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": text }]
            })
        })
        .collect();
    json!({ "type": "doc", "content": content })
}
