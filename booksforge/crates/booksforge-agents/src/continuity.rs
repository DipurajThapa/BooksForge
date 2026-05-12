//! Continuity Agent (AGENTS.md §4.5) — LLM adjudicator half.
//!
//! The deterministic linter (in `booksforge-validator::continuity`) runs
//! first.  This agent receives only the *ambiguous* findings from that
//! linter, refines them, and proposes fixes (rename / annotate / none).
//! Internal batching: ≤10 findings per call, ≤80 per run.

use booksforge_domain::{
    ContinuityEvidence, ContinuityFix, ContinuityFixKind, ContinuityFixScope, ContinuityKind,
    ContinuityReport, ContinuityReportEntry, Severity,
};
use booksforge_prompt::PromptTemplateId;
use serde::Deserialize;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "false-alias-flag",
        description: "Flagged a known alias as a name drift.",
        recoverable: true,
    },
    FailureMode {
        id: "missing-evidence",
        description: "Finding has no evidence span.",
        recoverable: true,
    },
    FailureMode {
        id: "rename-target-empty",
        description: "Rename fix lacks both `from` and `to`.",
        recoverable: true,
    },
    FailureMode {
        id: "kind-out-of-enum",
        description: "Finding kind is not in the fixed enum.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "continuity",
        name:             "Continuity",
        purpose:          "Adjudicate the ambiguous half of the deterministic continuity linter's findings — name drift, POV violations, tense flips, timeline contradictions — and propose concrete rename or annotate fixes scoped to scene/chapter/project. Never reads ahead of established memory.",
        input_schema_id:  "ContinuityAdjudicationInput",
        output_schema_id: "ContinuityReport",
        prompt_template:  PromptTemplateId::new("continuity", "v1"),
        model_preference: ModelPreference {
            family:   ModelFamily::Multilingual,
            min_size: ModelSizeHint::Medium,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // Per-finding adjudication; we batch ≤10 findings per call.
            max_context_tokens: 3_000,
            max_output_tokens:  2_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
            CrossCuttingValidator::EntitySanity,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        default_thinking: DefaultThinking::Enabled,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelEvidence {
    pub node_id: String,
    pub range_from: u32,
    pub range_to: u32,
    pub excerpt: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelFix {
    pub kind: String, // "rename" | "annotate" | "none"
    pub from: Option<String>,
    pub to: Option<String>,
    pub scope: String, // "scene" | "chapter" | "project"
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    pub kind: String, // "name_drift" | "pov_drift" | "tense_drift" | "timeline" | "other"
    pub severity: String,
    pub evidence: Vec<ModelEvidence>,
    pub diagnosis: String,
    pub proposed_fix: ModelFix,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelOutput {
    pub findings: Vec<ModelEntry>,
}

/// One-shot parser for the runner: raw text → typed `ContinuityReport`
/// with semantic validation.
pub fn parse_and_validate(raw: &str) -> Result<ContinuityReport, String> {
    let model: ModelOutput =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    let domain = into_domain(model);
    let errs = domain.validate();
    if !errs.is_empty() {
        return Err(format!("semantic validation failed: {}", errs.join("; ")));
    }
    Ok(domain)
}

pub fn into_domain(out: ModelOutput) -> ContinuityReport {
    let findings = out
        .findings
        .into_iter()
        .map(|e| ContinuityReportEntry {
            kind: parse_kind(&e.kind),
            severity: parse_severity(&e.severity),
            evidence: e
                .evidence
                .into_iter()
                .map(|ev| ContinuityEvidence {
                    node_id: ev.node_id,
                    range_from: ev.range_from,
                    range_to: ev.range_to,
                    excerpt: ev.excerpt,
                })
                .collect(),
            diagnosis: e.diagnosis,
            proposed_fix: ContinuityFix {
                kind: parse_fix_kind(&e.proposed_fix.kind),
                from: e.proposed_fix.from,
                to: e.proposed_fix.to,
                scope: parse_fix_scope(&e.proposed_fix.scope),
            },
        })
        .collect();
    ContinuityReport { findings }
}

fn parse_kind(s: &str) -> ContinuityKind {
    match s.trim().to_ascii_lowercase().as_str() {
        "name_drift" | "name-drift" => ContinuityKind::NameDrift,
        "pov_drift" | "pov-drift" => ContinuityKind::PovDrift,
        "tense_drift" | "tense-drift" => ContinuityKind::TenseDrift,
        "timeline" => ContinuityKind::Timeline,
        _ => ContinuityKind::Other,
    }
}
fn parse_severity(s: &str) -> Severity {
    match s.trim().to_ascii_lowercase().as_str() {
        "error" => Severity::Error,
        "warning" | "warn" => Severity::Warning,
        _ => Severity::Info,
    }
}
fn parse_fix_kind(s: &str) -> ContinuityFixKind {
    match s.trim().to_ascii_lowercase().as_str() {
        "rename" => ContinuityFixKind::Rename,
        "annotate" => ContinuityFixKind::Annotate,
        _ => ContinuityFixKind::None,
    }
}
fn parse_fix_scope(s: &str) -> ContinuityFixScope {
    match s.trim().to_ascii_lowercase().as_str() {
        "scene" => ContinuityFixScope::Scene,
        "chapter" => ContinuityFixScope::Chapter,
        _ => ContinuityFixScope::Project,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kind_round_trips_known_values() {
        assert!(matches!(
            parse_kind("name_drift"),
            ContinuityKind::NameDrift
        ));
        assert!(matches!(parse_kind("POV-DRIFT"), ContinuityKind::PovDrift));
        assert!(matches!(parse_kind("garbage"), ContinuityKind::Other));
    }

    #[test]
    fn parse_fix_kind_normalises_case() {
        assert!(matches!(
            parse_fix_kind("Rename"),
            ContinuityFixKind::Rename
        ));
        assert!(matches!(
            parse_fix_kind("ANNOTATE"),
            ContinuityFixKind::Annotate
        ));
        assert!(matches!(parse_fix_kind(""), ContinuityFixKind::None));
    }
}
