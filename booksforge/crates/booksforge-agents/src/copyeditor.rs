//! Copyeditor Agent (AGENTS.md §4.6).
//!
//! Mechanical and stylistic micro-fixes only.  Never rewords prose.  Output
//! is concrete `before / after` edit pairs the user accepts or rejects in
//! a per-edit inline diff.

use booksforge_domain::{CopyeditCategory, CopyeditEdit, CopyeditProposals};
use booksforge_prompt::PromptTemplateId;
use serde::Deserialize;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "range-mismatch",
        description:
            "`before` text doesn't match the source at the given range — fabricated position.",
        recoverable: true,
    },
    FailureMode {
        id: "rewrite-not-fix",
        description: "Edit changes word count by more than 10% — Copyeditor must not reword.",
        recoverable: true,
    },
    FailureMode {
        id: "overlap",
        description: "Two edits overlap on the same span.",
        recoverable: true,
    },
    FailureMode {
        id: "category-out-of-enum",
        description: "Edit category is not in the fixed enum.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "copyeditor",
        name:             "Copyeditor",
        purpose:          "Mechanical fixes only — punctuation, spacing, em-dashes, quotes, casing, spelling. Never rewords. Never restructures sentences. Edits arrive as concrete before/after pairs the writer accepts or rejects per-edit.",
        input_schema_id:  "CopyeditorInput",
        output_schema_id: "CopyeditProposals",
        prompt_template:  PromptTemplateId::new("copyeditor", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::AnyInstruct,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            max_context_tokens: 8_000,
            max_output_tokens:  4_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
            CrossCuttingValidator::Originality,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Wire-format edit emitted by the model.  Translated to the
/// `CopyeditEdit` domain type once the orchestrator validates it
/// against the source text.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelEdit {
    pub range_from: u32,
    pub range_to: u32,
    pub before: String,
    pub after: String,
    pub category: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelOutput {
    pub edits: Vec<ModelEdit>,
    pub summary: String,
}

/// Translate the model wire output into the domain type.  Unknown
/// category strings collapse to `CopyeditCategory::Other` — the semantic
/// validator catches that as a separate concern.
pub fn into_domain(out: ModelOutput) -> CopyeditProposals {
    let edits = out
        .edits
        .into_iter()
        .map(|e| CopyeditEdit {
            range_from: e.range_from,
            range_to: e.range_to,
            before: e.before,
            after: e.after,
            category: parse_category(&e.category),
            rationale: e.rationale,
        })
        .collect();
    CopyeditProposals {
        edits,
        summary: out.summary,
    }
}

/// One-shot parser for the runner: raw text → typed `CopyeditProposals`,
/// with semantic validation against the source text.  Returns `Err` with
/// a human-readable reason on parse or semantic failure (the runner uses
/// this to drive the retry loop).
pub fn parse_and_validate(raw: &str, source_text: &str) -> Result<CopyeditProposals, String> {
    let model: ModelOutput =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    let domain = into_domain(model);
    let errs = domain.validate(source_text);
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(domain)
}

fn parse_category(s: &str) -> CopyeditCategory {
    match s.trim().to_ascii_lowercase().as_str() {
        "punctuation" => CopyeditCategory::Punctuation,
        "spacing" => CopyeditCategory::Spacing,
        "casing" => CopyeditCategory::Casing,
        "quotes" => CopyeditCategory::Quotes,
        "dashes" => CopyeditCategory::Dashes,
        "spelling" => CopyeditCategory::Spelling,
        _ => CopyeditCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_domain_translates_categories() {
        let out = ModelOutput {
            edits: vec![ModelEdit {
                range_from: 0,
                range_to: 1,
                before: "a".into(),
                after: "A".into(),
                category: "casing".into(),
                rationale: "title".into(),
            }],
            summary: "x".into(),
        };
        let dom = into_domain(out);
        assert!(matches!(dom.edits[0].category, CopyeditCategory::Casing));
    }

    #[test]
    fn unknown_category_falls_through_to_other() {
        assert!(matches!(
            parse_category("nonsense"),
            CopyeditCategory::Other
        ));
    }
}
