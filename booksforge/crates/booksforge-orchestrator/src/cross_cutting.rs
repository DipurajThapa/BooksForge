//! Cross-cutting validators applied to every agent output, per AGENTS.md §6.
//!
//! These run **after** the per-agent semantic validators succeed.  They are
//! pure-logic and produce `ValidationCheck` rows for the
//! `ProposalValidation` aggregator.  None of them require an LLM call.
//!
//! Bound at orchestrator-binding time; the agent's `AgentSpec.validators`
//! slice tells the orchestrator which checks to run.

use booksforge_agents::CrossCuttingValidator;
use booksforge_domain::{
    allowed_write_scopes, Entity, ValidationAxis, ValidationCheck, ValidationOutcome,
};

/// Run a single cross-cutting validator and produce a `ValidationCheck`.
///
/// `source_text` and `prior_scene_corpus` power the `Originality` check:
/// pass `None` when the agent has no associated source/corpus (intake,
/// vocab-dictionary), or when the validator is not requested.  The empty
/// case is a no-op pass — never a false positive.
#[allow(clippy::too_many_arguments)]
pub fn run_validator(
    validator: CrossCuttingValidator,
    raw_output: &str,
    parsed: &serde_json::Value,
    entity_bible: &[Entity],
    agent_id: &str,
    proposed_memory_scopes: &[String],
    source_text: Option<&str>,
    prior_scene_corpus: Option<&str>,
) -> ValidationCheck {
    match validator {
        CrossCuttingValidator::Schema => check_schema(parsed),
        CrossCuttingValidator::Redaction => check_redaction(raw_output),
        CrossCuttingValidator::Length => check_length(raw_output, parsed),
        CrossCuttingValidator::EntitySanity => check_entity_sanity(parsed, entity_bible),
        CrossCuttingValidator::MemoryScope => check_memory_scope(agent_id, proposed_memory_scopes),
        CrossCuttingValidator::Originality => {
            check_originality(parsed, source_text, prior_scene_corpus)
        }
    }
}

/// Originality / anti-plagiarism check.  Walks the agent's prose-bearing
/// output fields (same set as `EntitySanity`) and runs the local n-gram
/// detector against the source text + prior-scene corpus.
fn check_originality(
    parsed: &serde_json::Value,
    source_text: Option<&str>,
    prior_scene_corpus: Option<&str>,
) -> ValidationCheck {
    if source_text.is_none() && prior_scene_corpus.is_none() {
        return ok(
            ValidationAxis::Originality,
            "no source corpus supplied — skipped",
        );
    }
    // Same prose-bearing field set the entity check uses, plus `excerpt`
    // and `text` for chapter-drafter outputs.
    let prose_fields = [
        "excerpt",
        "before",
        "after",
        "diagnosis",
        "message",
        "notes",
        "summary",
        "text",
        "scene_text",
        "draft",
    ];
    let mut prose = String::new();
    walk_prose(parsed, &prose_fields, &mut |s: &str| {
        prose.push_str(s);
        prose.push('\n');
    });
    if prose.trim().is_empty() {
        return ok(ValidationAxis::Originality, "no prose fields to check");
    }
    let mut hits: Vec<booksforge_validator::OverlapHit> = Vec::new();
    if let Some(src) = source_text {
        hits.extend(booksforge_validator::detect_verbatim_overlap(
            &prose,
            src,
            booksforge_validator::originality::DEFAULT_MIN_WORDS,
        ));
    }
    if let Some(prior) = prior_scene_corpus {
        hits.extend(booksforge_validator::detect_self_plagiarism(
            &prose,
            prior,
            booksforge_validator::originality::DEFAULT_MIN_WORDS,
        ));
    }
    if hits.is_empty() {
        return ok(
            ValidationAxis::Originality,
            "no verbatim overlap above threshold",
        );
    }
    // Fail when any hit is ≥20 words (clearly copy-paste, not coincidence).
    // Warn for shorter (12–19 words) hits — could be legitimate idioms or a
    // reviewer-caught issue.
    let max_words = hits.iter().map(|h| h.words).max().unwrap_or(0);
    let evidence = format!(
        "{n} verbatim overlap hit(s); longest run = {max_words} words; first quote: {q:?}",
        n = hits.len(),
        q = hits.first().map(|h| h.quote.as_str()).unwrap_or(""),
    );
    if max_words >= 20 {
        fail(
            ValidationAxis::Originality,
            evidence,
            "rewrite the flagged span(s) in original phrasing; quote and attribute if intentional",
        )
    } else {
        warn(
            ValidationAxis::Originality,
            evidence,
            "review the flagged span(s); short overlaps may be incidental but should be checked",
        )
    }
}

/// Schema = "the parsed output is a JSON object" (we deserialize into the
/// agent's typed output upstream — this is the boundary residual).
fn check_schema(parsed: &serde_json::Value) -> ValidationCheck {
    if parsed.is_object() {
        ok(ValidationAxis::Schema, "output parsed as JSON object")
    } else {
        fail(
            ValidationAxis::Schema,
            "output is not a JSON object",
            "request the agent retry with the JSON-mode reminder",
        )
    }
}

/// AGENTS.md §6: "output does not contain anything that looks like a system
/// prompt or chain-of-thought leak."
fn check_redaction(raw: &str) -> ValidationCheck {
    let lower = raw.to_ascii_lowercase();
    let suspicious_phrases = [
        "you are an experienced",
        "you are a senior",
        "system:",
        "user:",
        "<<<user_content>>>",
        "<<<end_user_content>>>",
        "as an ai language model",
        "i cannot",
        "i'm sorry, but i",
        "let me think step by step",
        "first, i'll",
    ];
    for phrase in suspicious_phrases {
        if lower.contains(phrase) {
            return warn(
                ValidationAxis::Redaction,
                format!("contains suspicious phrase: {phrase:?}"),
                "the model leaked the system prompt or chain-of-thought",
            );
        }
    }
    ok(ValidationAxis::Redaction, "no leakage indicators found")
}

/// Output is bounded and non-trivial: more than 4 bytes, less than 64 KiB.
fn check_length(raw: &str, parsed: &serde_json::Value) -> ValidationCheck {
    let bytes = raw.len();
    if bytes < 4 {
        return fail(
            ValidationAxis::Length,
            format!("output is too short ({bytes} bytes)"),
            "agent returned empty or near-empty output",
        );
    }
    if bytes > 65_536 {
        return warn(
            ValidationAxis::Length,
            format!("output is very large ({bytes} bytes)"),
            "consider tightening the prompt or capping output earlier",
        );
    }
    // Soft check: empty top-level object is suspicious.
    if let Some(obj) = parsed.as_object() {
        if obj.is_empty() {
            return warn(
                ValidationAxis::Length,
                "parsed object is empty",
                "no fields populated",
            );
        }
    }
    ok(ValidationAxis::Length, format!("output {bytes} bytes"))
}

/// EntitySanity: every proper-noun-looking token in the output's prose
/// fields must appear in the entity bible (canonical name or alias).
///
/// This is a *soft* check — small models drift on names.  Implemented as a
/// simple capitalised-word scan over fields named `excerpt`, `before`,
/// `after`, `diagnosis`, `message`, `notes`.  Runs only when the agent's
/// spec lists `EntitySanity` in its validators slice.
fn check_entity_sanity(parsed: &serde_json::Value, bible: &[Entity]) -> ValidationCheck {
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in bible {
        known.insert(e.name.to_lowercase());
        for a in &e.aliases {
            known.insert(a.to_lowercase());
        }
    }
    // Common allowlist for English prose so we don't flag e.g. "I", "Dr.", days.
    for w in ALLOWLIST_PROPER {
        known.insert((*w).to_lowercase());
    }
    let prose_fields = [
        "excerpt",
        "before",
        "after",
        "diagnosis",
        "message",
        "notes",
        "summary",
    ];
    let mut unknown = Vec::new();
    walk_prose(parsed, &prose_fields, &mut |s: &str| {
        for token in s.split_whitespace() {
            let trimmed = token.trim_matches(|c: char| !c.is_alphabetic());
            if trimmed.len() < 2 {
                continue;
            }
            let first = trimmed.chars().next().unwrap_or(' ');
            if !first.is_uppercase() {
                continue;
            }
            // Exclude all-caps acronyms.
            if trimmed.chars().all(|c| c.is_uppercase()) {
                continue;
            }
            if !known.contains(&trimmed.to_lowercase()) {
                unknown.push(trimmed.to_owned());
            }
        }
    });
    if unknown.is_empty() {
        ok(ValidationAxis::EntitySanity, "all proper nouns recognised")
    } else if unknown.len() > 6 {
        // Lots of unknowns → likely false-positive scan; downgrade to warn.
        warn(
            ValidationAxis::EntitySanity,
            format!(
                "{} unknown proper nouns (sample: {:?})",
                unknown.len(),
                &unknown[..6]
            ),
            "verify these aren't drifted aliases of entities in the bible",
        )
    } else {
        warn(
            ValidationAxis::EntitySanity,
            format!("unknown proper nouns: {unknown:?}"),
            "extend the entity bible or correct the agent's output",
        )
    }
}

const ALLOWLIST_PROPER: &[&str] = &[
    "I",
    "Mr",
    "Mrs",
    "Ms",
    "Dr",
    "St",
    "Mt",
    "Lord",
    "Lady",
    "Sir",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
    "God",
    "Heaven",
    "Hell",
    "Earth",
];

/// MemoryScope: every proposed memory write must be within the agent's
/// `allowed_write_scopes`.  We accept the orchestrator's pre-extracted list
/// of scope strings (`"book"`, `"chapter"`, `"entity"`, `"style"`).
fn check_memory_scope(agent_id: &str, proposed: &[String]) -> ValidationCheck {
    let allowed = allowed_write_scopes(agent_id);
    let allowed_strs: Vec<&'static str> = allowed.iter().map(|s| s.as_str()).collect();
    let mut violations = Vec::new();
    for s in proposed {
        if !allowed_strs.contains(&s.as_str()) {
            violations.push(s.clone());
        }
    }
    if violations.is_empty() {
        ok(
            ValidationAxis::MemoryScope,
            format!("all writes within scopes: {allowed_strs:?}"),
        )
    } else {
        fail(
            ValidationAxis::MemoryScope,
            format!("agent '{agent_id}' attempted out-of-scope writes: {violations:?}"),
            "drop those entries or escalate to memory-curator",
        )
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn ok(axis: ValidationAxis, evidence: impl Into<String>) -> ValidationCheck {
    ValidationCheck {
        axis,
        outcome: ValidationOutcome::Pass,
        evidence: evidence.into(),
        remediation: None,
    }
}
fn warn(
    axis: ValidationAxis,
    evidence: impl Into<String>,
    remediation: impl Into<String>,
) -> ValidationCheck {
    ValidationCheck {
        axis,
        outcome: ValidationOutcome::Warn,
        evidence: evidence.into(),
        remediation: Some(remediation.into()),
    }
}
fn fail(
    axis: ValidationAxis,
    evidence: impl Into<String>,
    remediation: impl Into<String>,
) -> ValidationCheck {
    ValidationCheck {
        axis,
        outcome: ValidationOutcome::Fail,
        evidence: evidence.into(),
        remediation: Some(remediation.into()),
    }
}

/// Recursively visit string-valued fields whose key is in `field_names`.
fn walk_prose<F: FnMut(&str)>(v: &serde_json::Value, field_names: &[&str], visit: &mut F) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                if field_names.contains(&k.as_str()) {
                    if let Some(s) = val.as_str() {
                        visit(s);
                    }
                }
                walk_prose(val, field_names, visit);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                walk_prose(item, field_names, visit);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_flags_chain_of_thought_leak() {
        let raw = "let me think step by step about this scene...";
        let c = check_redaction(raw);
        assert_eq!(c.outcome, ValidationOutcome::Warn);
    }

    #[test]
    fn redaction_flags_user_content_echo() {
        let raw = "{\"x\":1} <<<USER_CONTENT>>>";
        let c = check_redaction(raw);
        assert_eq!(c.outcome, ValidationOutcome::Warn);
    }

    #[test]
    fn length_rejects_empty_output() {
        let c = check_length("", &serde_json::json!({}));
        assert_eq!(c.outcome, ValidationOutcome::Fail);
    }

    #[test]
    fn schema_passes_for_object_output() {
        let c = check_schema(&serde_json::json!({"k": 1}));
        assert_eq!(c.outcome, ValidationOutcome::Pass);
    }

    #[test]
    fn memory_scope_rejects_out_of_scope_write() {
        // Copyeditor is allowed style scope only.
        let c = check_memory_scope("copyeditor", &["book".to_owned()]);
        assert_eq!(c.outcome, ValidationOutcome::Fail);
    }

    #[test]
    fn memory_scope_passes_when_in_scope() {
        let c = check_memory_scope("copyeditor", &["style".to_owned()]);
        assert_eq!(c.outcome, ValidationOutcome::Pass);
    }
}
