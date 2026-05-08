//! Canonical input/output types for every MVP agent.
//!
//! Each type maps 1:1 to the JSON shape declared in `AGENTS.md §4`.
//! Validation runs in two stages:
//!   1. **Structural** — `serde_json::from_str` into the type itself.
//!   2. **Semantic** — `validate()` on the type, returning a list of
//!      human-readable errors (empty = valid).
//!
//! Existing types live in their original modules:
//!   - `OutlineProposal` (outline.rs) — outline-architect output
//!   - `ProjectBrief`    (brief.rs)   — intake output
//!
//! This module owns the seven new ones plus `ProposalValidation`.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::validator::Severity;

// ──────────────────────────────────────────────────────────────────────────────
// Continuity Agent (§4.5) — deterministic linter findings + LLM adjudicator report
// ──────────────────────────────────────────────────────────────────────────────

/// One deterministic finding from the Rust continuity linter.  The LLM
/// adjudicator receives these as input and turns ambiguous ones into a
/// `ContinuityReport.findings` entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinuityFinding {
    pub kind:        ContinuityKind,
    pub severity:    Severity,
    pub evidence:    Vec<ContinuityEvidence>,
    /// One-line description.  The deterministic linter writes a fixed string;
    /// the LLM adjudicator is allowed to expand it.
    pub diagnosis:   String,
    /// `true` if this finding warrants LLM adjudication; `false` for findings
    /// the deterministic linter is fully confident about.
    pub ambiguous:   bool,
}

/// The closed enum of continuity finding kinds, per AGENTS.md §4.5.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityKind {
    NameDrift,
    PovDrift,
    TenseDrift,
    Timeline,
    Other,
}

/// A single span pointing at the offending text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinuityEvidence {
    pub node_id:    String,    // ULID; stringified for portability
    pub range_from: u32,
    pub range_to:   u32,
    pub excerpt:    String,    // ≤200 chars
}

/// LLM adjudicator output.  Each entry is a refined `ContinuityFinding` with
/// an optional `proposed_fix`.  Apply path: rename or annotate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityReport {
    pub findings: Vec<ContinuityReportEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityReportEntry {
    pub kind:         ContinuityKind,
    pub severity:     Severity,
    pub evidence:     Vec<ContinuityEvidence>,
    pub diagnosis:    String,
    pub proposed_fix: ContinuityFix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityFixKind { Rename, Annotate, None }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityFix {
    pub kind:  ContinuityFixKind,
    pub from:  Option<String>,
    pub to:    Option<String>,
    pub scope: ContinuityFixScope,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityFixScope { Scene, Chapter, Project }

impl ContinuityReport {
    /// Semantic validators per AGENTS.md §4.5.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (i, f) in self.findings.iter().enumerate() {
            for (j, ev) in f.evidence.iter().enumerate() {
                if Ulid::from_string(&ev.node_id).is_err() {
                    errors.push(format!("findings[{i}].evidence[{j}].node_id is not a valid ULID"));
                }
                if ev.range_from >= ev.range_to {
                    errors.push(format!("findings[{i}].evidence[{j}] has range_from >= range_to"));
                }
                if ev.excerpt.chars().count() > 200 {
                    errors.push(format!("findings[{i}].evidence[{j}].excerpt > 200 chars"));
                }
            }
            if f.diagnosis.is_empty() {
                errors.push(format!("findings[{i}] missing diagnosis"));
            }
            // Rename fixes must have non-empty `from` and `to`.
            if matches!(f.proposed_fix.kind, ContinuityFixKind::Rename)
                && (f.proposed_fix.from.as_deref().unwrap_or("").is_empty()
                    || f.proposed_fix.to.as_deref().unwrap_or("").is_empty())
                {
                    errors.push(format!("findings[{i}] rename fix requires both from and to"));
                }
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Copyeditor Agent (§4.6) — concrete edit pairs
// ──────────────────────────────────────────────────────────────────────────────

/// One concrete copyedit edit pair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CopyeditEdit {
    pub range_from: u32,
    pub range_to:   u32,
    pub before:     String,
    pub after:      String,
    pub category:   CopyeditCategory,
    pub rationale:  String,    // ≤30 words
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CopyeditCategory {
    Punctuation,
    Spacing,
    Casing,
    Quotes,
    Dashes,
    Spelling,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyeditProposals {
    pub edits:   Vec<CopyeditEdit>,
    pub summary: String,    // ≤80 words
}

impl CopyeditProposals {
    /// Semantic validators per AGENTS.md §4.6.
    /// `source_text` is the actual scene text-content the agent operated on;
    /// passed in so `before`-matches-source-at-range can be enforced here.
    pub fn validate(&self, source_text: &str) -> Vec<String> {
        let mut errors = Vec::new();
        let chars: Vec<char> = source_text.chars().collect();
        let len = chars.len() as u32;

        for (i, e) in self.edits.iter().enumerate() {
            if e.range_from >= e.range_to {
                errors.push(format!("edits[{i}] has range_from >= range_to"));
            }
            if e.range_to > len {
                errors.push(format!("edits[{i}] range_to {} exceeds source length {}", e.range_to, len));
                continue;
            }
            // The actual source slice must match `before`.
            let actual: String = chars[e.range_from as usize..e.range_to as usize].iter().collect();
            if actual != e.before {
                errors.push(format!(
                    "edits[{i}] before-text does not match source at range — fabricated position"
                ));
            }
            if e.before == e.after {
                errors.push(format!("edits[{i}] before == after (no-op edit)"));
            }
            // Word-count change cap: ±10 % of `before`'s word count.
            let before_words = e.before.split_whitespace().count();
            let after_words  = e.after.split_whitespace().count();
            if before_words > 0 {
                let delta = (after_words as i64 - before_words as i64).unsigned_abs() as f64;
                if delta / before_words as f64 > 0.10 {
                    errors.push(format!(
                        "edits[{i}] word-count change > 10% (Copyeditor must not reword)"
                    ));
                }
            }
            if e.rationale.split_whitespace().count() > 30 {
                errors.push(format!("edits[{i}].rationale > 30 words"));
            }
        }
        if self.summary.split_whitespace().count() > 80 {
            errors.push("summary > 80 words".to_owned());
        }
        // No two edits may overlap.
        for i in 0..self.edits.len() {
            for j in (i + 1)..self.edits.len() {
                let a = &self.edits[i];
                let b = &self.edits[j];
                if a.range_from < b.range_to && b.range_from < a.range_to {
                    errors.push(format!("edits[{i}] and edits[{j}] overlap"));
                }
            }
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Memory Curator Agent (§4.7)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRefreshScope { Book, Chapter, Entity }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRefreshInput {
    pub scope:   MemoryRefreshScope,
    pub node_id: Option<String>,
}

/// Memory Curator output — proposed memory writes the orchestrator will
/// authorize against `allowed_write_scopes("memory-curator")` before applying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRefreshProposals {
    pub upserts: Vec<MemoryUpsert>,
    /// Optional: entity stubs to insert if the agent saw new proper nouns.
    pub new_entities: Vec<EntityStub>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUpsert {
    /// `book` | `chapter` | `entity` | `style`
    pub scope:    String,
    pub key:      String,
    pub value:    serde_json::Value,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStub {
    pub kind:    String,        // "character" | "location" | "item" | …
    pub name:    String,
    pub aliases: Vec<String>,
    pub fields:  serde_json::Value,
}

impl MemoryRefreshProposals {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (i, u) in self.upserts.iter().enumerate() {
            if u.key.is_empty() {
                errors.push(format!("upserts[{i}] missing key"));
            }
            if !matches!(u.scope.as_str(), "book" | "chapter" | "entity" | "style") {
                errors.push(format!("upserts[{i}] scope '{}' not in enum", u.scope));
            }
        }
        for (i, e) in self.new_entities.iter().enumerate() {
            if e.name.trim().is_empty() {
                errors.push(format!("new_entities[{i}] missing name"));
            }
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Vocabulary Dictionary Agent (§4.8)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabUpdateProposals {
    pub additions:    Vec<VocabAddition>,
    pub modifications: Vec<VocabModification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabAddition {
    pub term:        String,
    pub kind:        String,    // "prefer" | "avoid" | "replace"
    pub layer:       String,    // matching layer enum
    pub replacement: Option<String>,
    pub rationale:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabModification {
    pub term:           String,
    pub layer:          String,
    pub field:          String,    // "kind" | "replacement" | …
    pub new_value:      serde_json::Value,
    pub rationale:      String,
}

impl VocabUpdateProposals {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (i, a) in self.additions.iter().enumerate() {
            if a.term.trim().is_empty() {
                errors.push(format!("additions[{i}] empty term"));
            }
            if !matches!(a.kind.as_str(), "prefer" | "avoid" | "replace") {
                errors.push(format!("additions[{i}].kind '{}' not in enum", a.kind));
            }
            if a.kind == "replace" && a.replacement.as_deref().unwrap_or("").is_empty() {
                errors.push(format!("additions[{i}] kind=replace requires replacement"));
            }
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Chapter Drafting Agent (§4.3)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDraftProposal {
    /// ProseMirror-compatible JSON document for the new scene draft.
    pub pm_doc:     serde_json::Value,
    /// Estimated word count (the orchestrator recomputes from pm_doc).
    pub word_count: u32,
    /// Free-text notes about choices the agent made (≤120 words).
    pub notes:      String,
}

impl SceneDraftProposal {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.pm_doc.get("type").and_then(|v| v.as_str()) != Some("doc") {
            errors.push("pm_doc must have type='doc'".to_owned());
        }
        if self.pm_doc.get("content").and_then(|v| v.as_array()).map(|a| a.is_empty()).unwrap_or(true) {
            errors.push("pm_doc.content is empty".to_owned());
        }
        if self.notes.split_whitespace().count() > 120 {
            errors.push("notes > 120 words".to_owned());
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Developmental Editor Agent (§4.4)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalNotes {
    pub chapter_id: String,
    pub notes:      Vec<DevelopmentalNote>,
    pub summary:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalNote {
    pub axis:       DevelopmentalAxis,
    pub severity:   Severity,
    pub message:    String,
    pub evidence:   Vec<ContinuityEvidence>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentalAxis {
    Pacing,
    Stakes,
    Character,
    PovTension,
    Theme,
    StructuralBalance,
    Other,
}

impl DevelopmentalNotes {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if Ulid::from_string(&self.chapter_id).is_err() {
            errors.push("chapter_id not a valid ULID".to_owned());
        }
        if self.notes.len() > 25 {
            errors.push(format!("notes count {} exceeds cap 25", self.notes.len()));
        }
        if self.summary.split_whitespace().count() > 200 {
            errors.push("summary > 200 words".to_owned());
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Humanization Agent (§4.9)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanizationProposals {
    pub edits: Vec<HumanizationEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanizationEdit {
    pub range_from: u32,
    pub range_to:   u32,
    pub before:     String,
    pub after:      String,
    /// The vocab `avoid`-rule the original prose triggered.
    pub triggered_rule: String,
    pub rationale:  String,
}

impl HumanizationProposals {
    pub fn validate(&self, source_text: &str) -> Vec<String> {
        let mut errors = Vec::new();
        let chars: Vec<char> = source_text.chars().collect();
        let len = chars.len() as u32;
        for (i, e) in self.edits.iter().enumerate() {
            if e.range_from >= e.range_to {
                errors.push(format!("edits[{i}] inverted range"));
            }
            if e.range_to > len {
                errors.push(format!("edits[{i}] range exceeds source length"));
                continue;
            }
            let actual: String = chars[e.range_from as usize..e.range_to as usize].iter().collect();
            if actual != e.before {
                errors.push(format!("edits[{i}] before-text mismatch"));
            }
            if e.triggered_rule.trim().is_empty() {
                errors.push(format!("edits[{i}] missing triggered_rule"));
            }
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Proposal Validator (Tier 1 + Tier 2) — meta-agent output
// ──────────────────────────────────────────────────────────────────────────────

/// Verdict from the proposal validator.  `block` halts the proposal flow
/// before the user sees it; `warn` annotates the result; `pass` is silent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationVerdict { Pass, Warn, Block }

/// One axis of validation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationAxis {
    // Tier 1 (deterministic)
    Schema,
    Contract,
    Range,
    Redaction,
    Length,
    EntitySanity,
    MemoryScope,
    Idempotent,
    /// Plagiarism / verbatim-overlap detector.  Flags long verbatim spans
    /// from the source the agent was given (catches "agent copy-pasted
    /// instead of generating") and from the project's prior accepted
    /// scenes (catches self-plagiarism across chapters).  Never sends
    /// content off-device — local n-gram match only.
    Originality,
    // Tier 2 (LLM)
    Faithfulness,
    Style,
    Coherence,
    SelfConsistency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutcome { Pass, Warn, Fail }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub axis:        ValidationAxis,
    pub outcome:     ValidationOutcome,
    pub evidence:    String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalValidation {
    pub verdict:        ValidationVerdict,
    pub checks:         Vec<ValidationCheck>,
    pub summary:        String,    // ≤140 words
    /// Whether the LLM Tier-2 validator ran.
    pub tier_2_ran:     bool,
}

impl ProposalValidation {
    /// Convenience: aggregate per-axis worst outcome → verdict.
    pub fn verdict_from_checks(checks: &[ValidationCheck]) -> ValidationVerdict {
        let any_fail = checks.iter().any(|c| matches!(c.outcome, ValidationOutcome::Fail));
        let any_warn = checks.iter().any(|c| matches!(c.outcome, ValidationOutcome::Warn));
        if any_fail      { ValidationVerdict::Block }
        else if any_warn { ValidationVerdict::Warn }
        else             { ValidationVerdict::Pass }
    }

    pub fn pass(summary: impl Into<String>) -> Self {
        Self { verdict: ValidationVerdict::Pass, checks: Vec::new(), summary: summary.into(), tier_2_ran: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(from: u32, to: u32, ex: &str) -> ContinuityEvidence {
        ContinuityEvidence {
            node_id: "01HXAAAAAAAAAAAAAAAAAAAAAA".into(),
            range_from: from, range_to: to, excerpt: ex.into(),
        }
    }

    #[test]
    fn copyedit_validate_rejects_fabricated_before() {
        let source = "hello  world";
        let p = CopyeditProposals {
            edits: vec![CopyeditEdit {
                range_from: 0, range_to: 5,
                before: "WRONG".into(), after: "fixed".into(),
                category: CopyeditCategory::Spelling,
                rationale: "x".into(),
            }],
            summary: "x".into(),
        };
        let errs = p.validate(source);
        assert!(errs.iter().any(|e| e.contains("fabricated position")));
    }

    #[test]
    fn copyedit_validate_rejects_overlap() {
        let source = "abcdefghijk";
        let p = CopyeditProposals {
            edits: vec![
                CopyeditEdit { range_from: 0, range_to: 5,
                    before: "abcde".into(), after: "ABCDE".into(),
                    category: CopyeditCategory::Casing, rationale: "x".into() },
                CopyeditEdit { range_from: 3, range_to: 8,
                    before: "defgh".into(), after: "DEFGH".into(),
                    category: CopyeditCategory::Casing, rationale: "x".into() },
            ],
            summary: "x".into(),
        };
        let errs = p.validate(source);
        assert!(errs.iter().any(|e| e.contains("overlap")));
    }

    #[test]
    fn copyedit_validate_caps_word_count_change() {
        let source = "the quick brown fox";
        let p = CopyeditProposals {
            edits: vec![CopyeditEdit {
                range_from: 4, range_to: 9,
                before: "quick".into(),
                // 1 → 4 words = 300 % change.
                after: "very very very fast".into(),
                category: CopyeditCategory::Other, rationale: "x".into(),
            }],
            summary: "x".into(),
        };
        let errs = p.validate(source);
        assert!(errs.iter().any(|e| e.contains("word-count change > 10%")));
    }

    #[test]
    fn continuity_validate_requires_rename_targets() {
        let r = ContinuityReport {
            findings: vec![ContinuityReportEntry {
                kind: ContinuityKind::NameDrift, severity: Severity::Warning,
                evidence: vec![ev(0, 4, "Eli")],
                diagnosis: "alias drift".into(),
                proposed_fix: ContinuityFix {
                    kind: ContinuityFixKind::Rename, from: None, to: None,
                    scope: ContinuityFixScope::Project,
                },
            }],
        };
        let errs = r.validate();
        assert!(errs.iter().any(|e| e.contains("rename fix requires both from and to")));
    }

    #[test]
    fn validation_verdict_aggregates_correctly() {
        let pass_check = ValidationCheck {
            axis: ValidationAxis::Schema, outcome: ValidationOutcome::Pass,
            evidence: "ok".into(), remediation: None,
        };
        let warn_check = ValidationCheck {
            axis: ValidationAxis::Length, outcome: ValidationOutcome::Warn,
            evidence: "long".into(), remediation: None,
        };
        let fail_check = ValidationCheck {
            axis: ValidationAxis::Schema, outcome: ValidationOutcome::Fail,
            evidence: "bad".into(), remediation: None,
        };
        assert_eq!(ProposalValidation::verdict_from_checks(&[pass_check.clone()]), ValidationVerdict::Pass);
        assert_eq!(ProposalValidation::verdict_from_checks(&[warn_check.clone()]), ValidationVerdict::Warn);
        assert_eq!(ProposalValidation::verdict_from_checks(&[fail_check, warn_check, pass_check]), ValidationVerdict::Block);
    }
}
