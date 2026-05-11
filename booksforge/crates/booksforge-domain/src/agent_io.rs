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
    pub kind: ContinuityKind,
    pub severity: Severity,
    pub evidence: Vec<ContinuityEvidence>,
    /// One-line description.  The deterministic linter writes a fixed string;
    /// the LLM adjudicator is allowed to expand it.
    pub diagnosis: String,
    /// `true` if this finding warrants LLM adjudication; `false` for findings
    /// the deterministic linter is fully confident about.
    pub ambiguous: bool,
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
    pub node_id: String, // ULID; stringified for portability
    pub range_from: u32,
    pub range_to: u32,
    pub excerpt: String, // ≤200 chars
}

/// LLM adjudicator output.  Each entry is a refined `ContinuityFinding` with
/// an optional `proposed_fix`.  Apply path: rename or annotate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityReport {
    pub findings: Vec<ContinuityReportEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityReportEntry {
    pub kind: ContinuityKind,
    pub severity: Severity,
    pub evidence: Vec<ContinuityEvidence>,
    pub diagnosis: String,
    pub proposed_fix: ContinuityFix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityFixKind {
    Rename,
    Annotate,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityFix {
    pub kind: ContinuityFixKind,
    pub from: Option<String>,
    pub to: Option<String>,
    pub scope: ContinuityFixScope,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityFixScope {
    Scene,
    Chapter,
    Project,
}

impl ContinuityReport {
    /// Semantic validators per AGENTS.md §4.5.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (i, f) in self.findings.iter().enumerate() {
            for (j, ev) in f.evidence.iter().enumerate() {
                if Ulid::from_string(&ev.node_id).is_err() {
                    errors.push(format!(
                        "findings[{i}].evidence[{j}].node_id is not a valid ULID"
                    ));
                }
                if ev.range_from >= ev.range_to {
                    errors.push(format!(
                        "findings[{i}].evidence[{j}] has range_from >= range_to"
                    ));
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
                errors.push(format!(
                    "findings[{i}] rename fix requires both from and to"
                ));
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
    pub range_to: u32,
    pub before: String,
    pub after: String,
    pub category: CopyeditCategory,
    pub rationale: String, // ≤30 words
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
    pub edits: Vec<CopyeditEdit>,
    pub summary: String, // ≤80 words
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
                errors.push(format!(
                    "edits[{i}] range_to {} exceeds source length {}",
                    e.range_to, len
                ));
                continue;
            }
            // The actual source slice must match `before`.
            let actual: String = chars[e.range_from as usize..e.range_to as usize]
                .iter()
                .collect();
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
            let after_words = e.after.split_whitespace().count();
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
pub enum MemoryRefreshScope {
    Book,
    Chapter,
    Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRefreshInput {
    pub scope: MemoryRefreshScope,
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
    pub scope: String,
    pub key: String,
    pub value: serde_json::Value,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStub {
    pub kind: String, // "character" | "location" | "item" | …
    pub name: String,
    pub aliases: Vec<String>,
    pub fields: serde_json::Value,
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
    pub additions: Vec<VocabAddition>,
    pub modifications: Vec<VocabModification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabAddition {
    pub term: String,
    pub kind: String,  // "prefer" | "avoid" | "replace"
    pub layer: String, // matching layer enum
    pub replacement: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabModification {
    pub term: String,
    pub layer: String,
    pub field: String, // "kind" | "replacement" | …
    pub new_value: serde_json::Value,
    pub rationale: String,
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
    pub pm_doc: serde_json::Value,
    /// Estimated word count (the orchestrator recomputes from pm_doc).
    pub word_count: u32,
    /// Free-text notes about choices the agent made (≤120 words).
    pub notes: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Adaptive Polish Planner (Item 4)
// ──────────────────────────────────────────────────────────────────────────────

/// One entry in the adaptive polish plan emitted by the
/// `scene-planner` agent.
///
/// Each entry names a specific polish stage to invoke, the
/// observed reason it's in the plan (drawn from
/// `VoiceScore.failed_dimensions` and the tells report), and a
/// **targeted instruction** the polish stage should operate on
/// rather than its default behaviour.
///
/// Example: instead of `polish:voice` running the generic
/// voice-fingerprint matcher, the planner can route it as
///   `{ stage_id: "polish:voice",
///      reason: "median sentence length 4 < target 7",
///      instruction: "convert 25% of short sentences into 9-17 word
///                    compounds; insert at least one 18+ word sentence
///                    per paragraph",
///      severity: 3 }`
///
/// This is the load-bearing signal the planner gives the polish
/// stack — without it polish stages run on instinct; with it they
/// run on observed gaps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolishPlanEntry {
    /// Stage identifier — must match one of the known polish stages
    /// (`polish:voice`, `polish:metaphor`, `polish:dialogue`,
    /// `polish:scene_tension`). Future planner versions may emit
    /// new stage ids that the orchestrator dispatches to specialist
    /// agents (e.g. `polish:rhythm`, `polish:figurative_inject`).
    pub stage_id: String,
    /// One-line rationale citing the observed signal — used by the
    /// audit ledger and the UI per-stage badge.
    pub reason: String,
    /// Targeted instruction for the polish stage to execute. Empty
    /// string means "run the stage's default behaviour."
    pub instruction: String,
    /// Severity of the gap this entry addresses. `1` cosmetic,
    /// `2` routine, `3` critical (verdict-shifting).
    pub severity: u8,
}

/// The full polish plan emitted by the `scene-planner` agent.
///
/// Item 4 of the FEATURE_HARDENING_PLAN: replaces the static
/// genre-pack polish-stage order with a dynamic DAG selected based
/// on what the drafter actually produced. Stages NOT in the plan
/// are skipped entirely (the genre-pack `should_run` detector still
/// applies as a final guard).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolishPlan {
    /// Polish stages to invoke, in the order they should run.
    /// Empty plan = no polish needed; orchestrator skips the whole
    /// polish stack and proceeds to tells_scan.
    pub entries: Vec<PolishPlanEntry>,
    /// Free-text summary of why this plan was chosen. ≤100 words.
    pub rationale: String,
}

impl PolishPlan {
    /// Sanity-check the plan. Returns a list of human-readable
    /// validation errors (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.rationale.split_whitespace().count() > 100 {
            errors.push("rationale > 100 words".to_owned());
        }
        for (i, e) in self.entries.iter().enumerate() {
            if e.stage_id.trim().is_empty() {
                errors.push(format!("entries[{i}]: stage_id is empty"));
            }
            if !e.stage_id.starts_with("polish:") {
                errors.push(format!(
                    "entries[{i}]: stage_id '{}' must start with 'polish:'",
                    e.stage_id,
                ));
            }
            if e.reason.trim().is_empty() {
                errors.push(format!("entries[{i}]: reason is empty"));
            }
            if e.severity < 1 || e.severity > 3 {
                errors.push(format!(
                    "entries[{i}]: severity {} not in 1..=3",
                    e.severity,
                ));
            }
        }
        errors
    }

    /// Helper: return the plan entries that match the given stage
    /// (zero or one in practice; the planner is expected to emit
    /// each stage at most once but this isn't enforced).
    pub fn instructions_for(&self, stage_id: &str) -> Vec<&PolishPlanEntry> {
        self.entries
            .iter()
            .filter(|e| e.stage_id == stage_id)
            .collect()
    }
}

/// One micro-beat of a longer scene — used by
/// [`Orchestrator::run_scene_drafter_fic_chunked`] to produce
/// long scenes by drafting smaller, complete-feeling sub-units in
/// sequence and concatenating them.
///
/// **The Run #14 fix.** The drafter judges scenes complete at ~500
/// words and stops; padding to a 1500-word target dilutes the
/// craft. The chunked approach mirrors the chunked-bible pattern:
/// break a long scene into 3–5 beats of 400–600 words each, draft
/// each as its own complete micro-scene, then concatenate the
/// pm_doc paragraphs into one combined `SceneDraftProposal`. Each
/// beat advances ONE specific moment of the larger arc (the hook,
/// the escalation, the reveal, the close) and gets its own
/// scene_goal / scene_conflict / scene_reveal so the drafter has
/// a tight target it can actually finish.
///
/// `beat_id` is a stable string the polish stack and audit ledger
/// can reference (`"hook"`, `"escalation_a"`, `"reveal"`, `"close"`,
/// or any project-defined label).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneBeat {
    pub beat_id: String,
    pub goal: String,
    pub conflict: String,
    pub reveal: String,
    pub target_words: u32,
}

impl SceneBeat {
    /// Sanity-check a beat. Returns a list of human-readable
    /// validation errors (empty = valid). The orchestrator's chunked
    /// drafter rejects any beat list with errors before spending
    /// any LLM budget.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.beat_id.trim().is_empty() {
            errors.push("beat_id is empty".to_owned());
        }
        if self.goal.trim().is_empty() {
            errors.push(format!("beat[{}]: goal is empty", self.beat_id));
        }
        if self.conflict.trim().is_empty() {
            errors.push(format!("beat[{}]: conflict is empty", self.beat_id));
        }
        if self.reveal.trim().is_empty() {
            errors.push(format!("beat[{}]: reveal is empty", self.beat_id));
        }
        // Drafter judges scenes complete at ~500 words; 200-1200
        // is the band where chunked beats produce real prose
        // without overflowing thinking-mode budget on either end.
        if self.target_words < 200 {
            errors.push(format!(
                "beat[{}]: target_words {} < 200; too short for a complete micro-scene",
                self.beat_id, self.target_words,
            ));
        }
        if self.target_words > 1200 {
            errors.push(format!(
                "beat[{}]: target_words {} > 1200; split into smaller beats",
                self.beat_id, self.target_words,
            ));
        }
        errors
    }
}

impl SceneDraftProposal {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.pm_doc.get("type").and_then(|v| v.as_str()) != Some("doc") {
            errors.push("pm_doc must have type='doc'".to_owned());
        }
        if self
            .pm_doc
            .get("content")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true)
        {
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
    pub notes: Vec<DevelopmentalNote>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalNote {
    pub axis: DevelopmentalAxis,
    pub severity: Severity,
    pub message: String,
    pub evidence: Vec<ContinuityEvidence>,
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
    pub range_to: u32,
    pub before: String,
    pub after: String,
    /// The vocab `avoid`-rule the original prose triggered.
    pub triggered_rule: String,
    pub rationale: String,
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
            let actual: String = chars[e.range_from as usize..e.range_to as usize]
                .iter()
                .collect();
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
pub enum ValidationVerdict {
    Pass,
    Warn,
    Block,
}

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
pub enum ValidationOutcome {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub axis: ValidationAxis,
    pub outcome: ValidationOutcome,
    pub evidence: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalValidation {
    pub verdict: ValidationVerdict,
    pub checks: Vec<ValidationCheck>,
    pub summary: String, // ≤140 words
    /// Whether the LLM Tier-2 validator ran.
    pub tier_2_ran: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// Specialist Polish Stack (BACKLOG §A15 / Phase 2 — voice-preserving by design)
// ──────────────────────────────────────────────────────────────────────────────

/// Identifier of a specialist polish stage. Lets the orchestrator and the
/// audit ledger record which specific pass produced an edit, so a future
/// "revert this stage's changes" surface can be built without parsing
/// payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolishStageId {
    Dialogue,
    Metaphor,
    Voice,
    SceneTension,
}

impl PolishStageId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dialogue => "dialogue",
            Self::Metaphor => "metaphor",
            Self::Voice => "voice",
            Self::SceneTension => "scene_tension",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dialogue" => Some(Self::Dialogue),
            "metaphor" => Some(Self::Metaphor),
            "voice" => Some(Self::Voice),
            "scene_tension" => Some(Self::SceneTension),
            _ => None,
        }
    }
}

/// Output of a single polish pass. The proposal is a complete revised
/// chapter (the orchestrator diffs against the original to extract
/// per-edit ledger rows on apply). `stage_id` makes the ledger queryable
/// per-stage; `edit_notes` carries free-text rationale (≤120 words) for
/// the user to see in the diff view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishProposal {
    pub stage_id: PolishStageId,
    /// ProseMirror-compatible JSON document for the revised chapter.
    pub revised_pm_doc: serde_json::Value,
    /// Estimated word count (orchestrator recomputes from `revised_pm_doc`).
    pub revised_word_count: u32,
    /// Free-text notes about which edits the stage made (≤120 words).
    pub edit_notes: String,
}

impl PolishProposal {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.revised_pm_doc.get("type").and_then(|v| v.as_str()) != Some("doc") {
            errors.push("revised_pm_doc must have type='doc'".to_owned());
        }
        if self
            .revised_pm_doc
            .get("content")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true)
        {
            errors.push("revised_pm_doc.content is empty".to_owned());
        }
        if self.edit_notes.split_whitespace().count() > 120 {
            errors.push("edit_notes > 120 words".to_owned());
        }
        errors
    }
}

/// Output of a per-scene critique-revise loop. Lists targeted edit
/// instructions the reviser will apply — quote-based, so the reviser can
/// match them mechanically without re-reading the whole scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCritiqueProposal {
    /// Per-axis 1-10 scores. Axes come from the genre pack's `critic_axes`
    /// (literary / genre / non-fiction get different sets); the orchestrator
    /// passes them in as a `vars` block so the agent template renders the
    /// right rubric.
    pub scores: std::collections::BTreeMap<String, u8>,
    /// Name of the worst-scoring axis. The reviser focuses there.
    pub weakest_axis: String,
    /// Targeted edits the reviser should apply.
    pub specific_edits: Vec<TargetedEdit>,
    /// One-line summary of what the scene needs (≤30 words).
    pub overall_one_liner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetedEdit {
    /// Verbatim quote of the weak passage. The reviser locates this in
    /// the source text (with a fuzzy fallback for the inevitable LLM
    /// re-quotation drift) and replaces it with `fix`.
    pub problem_quote: String,
    /// Concrete revised line — what the reviser writes in place of the
    /// `problem_quote`. Same approximate length so paragraph rhythm
    /// holds.
    pub fix: String,
    /// Which critic axis this edit addresses (one of the genre's
    /// `critic_axes`).
    pub axis: String,
}

impl SceneCritiqueProposal {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.scores.is_empty() {
            errors.push("scores map is empty".to_owned());
        }
        for (axis, score) in &self.scores {
            if *score == 0 || *score > 10 {
                errors.push(format!("axis '{axis}' score {score} outside 1-10"));
            }
        }
        if self.weakest_axis.trim().is_empty() {
            errors.push("weakest_axis is empty".to_owned());
        }
        if !self.scores.is_empty() && !self.scores.contains_key(&self.weakest_axis) {
            errors.push(format!(
                "weakest_axis '{}' is not in the scores map",
                self.weakest_axis
            ));
        }
        if self.overall_one_liner.split_whitespace().count() > 30 {
            errors.push("overall_one_liner > 30 words".to_owned());
        }
        for (i, edit) in self.specific_edits.iter().enumerate() {
            if edit.problem_quote.trim().is_empty() {
                errors.push(format!("edit[{i}] has empty problem_quote"));
            }
            if edit.fix.trim().is_empty() {
                errors.push(format!("edit[{i}] has empty fix"));
            }
        }
        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Character Bible Agent (BACKLOG §A13 — fiction-shaped)
// ──────────────────────────────────────────────────────────────────────────────

/// One character card. Mirrors the on-disk template at
/// `crates/booksforge-prompt/templates/character-bible/v1.toml`.
///
/// Hard rules enforced by `validate()`:
///   - `name`, `external_objective`, `internal_need` are non-empty.
///   - `voice_traits` has 3-6 entries, each non-empty.
///   - `chapter_arc.len() == chapter_count` declared by the prompt input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCard {
    pub name: String,
    /// `protagonist | antagonist | mentor | foil | ally | supporting`
    pub role: String,
    pub external_objective: String,
    pub internal_need: String,
    pub fear_or_wound: String,
    pub secret_or_contradiction: String,
    /// 3-6 measurable traits — vocabulary tier, sentence rhythm, evasion
    /// patterns, lexicon. Vague traits like "kind" are not voice traits and
    /// are rejected by the agent template.
    pub voice_traits: Vec<String>,
    pub relationships: Vec<CharacterRelationship>,
    /// One entry per chapter — what changes for this character.
    pub chapter_arc: Vec<String>,
    pub emotional_turning_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRelationship {
    /// Other character's name. Must match a `name` elsewhere in the bible.
    pub to: String,
    /// Free-text nature of the relationship. Local LLMs occasionally
    /// emit `{"to": "Ada"}` without a nature; rather than reject the
    /// whole CharacterCard for one missing one-line description, we
    /// default to an empty string and let the writer fill it in via
    /// the bible editor. The `to` field is the load-bearing one for
    /// downstream consumers (orchestrator threads relationships
    /// through scene_drafter_fic via the bible's `relationships`
    /// list); `nature` is annotation that reads cleanly empty.
    #[serde(default)]
    pub nature: String,
}

/// Output of the character-bible agent. Validated against the brief's
/// declared chapter count by `validate()`.
///
/// `voice_target` (FEATURE_HARDENING_PLAN.md §1.6) optionally carries
/// a numeric voice contract — the prescriptive sentence-length-bucket /
/// MATTR / anaphora-cap distribution the scene drafter must satisfy.
/// When present, the orchestrator renders its
/// [`booksforge_voice::VoiceTarget::directive_block`] into the
/// scene-drafter prompt as a hard contract; when absent, the drafter
/// falls back to the per-character prose `voice_traits`.
///
/// `#[serde(default)]` keeps the field backward-compatible — bibles
/// produced before §1.6 deserialize cleanly with `voice_target = None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBibleProposal {
    pub characters: Vec<CharacterCard>,
    #[serde(default)]
    pub voice_target: Option<booksforge_voice::VoiceTarget>,
}

impl CharacterBibleProposal {
    /// Convenience constructor — characters only, no voice target.
    /// Existing call sites that pre-date §1.6 keep working without
    /// touching the field directly.
    pub fn new(characters: Vec<CharacterCard>) -> Self {
        Self {
            characters,
            voice_target: None,
        }
    }

    /// Render the bible's voice contract for the scene drafter prompt.
    /// Returns the directive block when `voice_target` is set; otherwise
    /// an empty string (the drafter then relies on `voice_traits`).
    pub fn voice_target_directive(&self) -> String {
        match &self.voice_target {
            Some(t) => t.directive_block(),
            None => String::new(),
        }
    }
}

impl CharacterBibleProposal {
    /// Run all semantic checks. Returns a list of human-readable errors
    /// (empty = valid). The orchestrator uses this list both for retry
    /// triggers and for surface-to-user diagnostics.
    pub fn validate(&self, expected_chapter_count: usize) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();

        if self.characters.is_empty() {
            errors.push("characters list is empty".to_owned());
        }
        if self.characters.len() < 2 {
            errors.push(format!(
                "need at least 2 characters (protagonist + antagonist), got {}",
                self.characters.len()
            ));
        }
        if self.characters.len() > 12 {
            errors.push(format!(
                "more than 12 characters ({}); MVP bible should stay focused",
                self.characters.len()
            ));
        }

        let mut names: Vec<&str> = Vec::new();
        let mut has_protagonist = false;

        for (i, c) in self.characters.iter().enumerate() {
            if c.name.trim().is_empty() {
                errors.push(format!("character[{i}] has empty name"));
            } else {
                names.push(c.name.as_str());
            }
            if c.role.eq_ignore_ascii_case("protagonist") {
                has_protagonist = true;
            }
            if c.external_objective.trim().is_empty() {
                errors.push(format!(
                    "character[{i}] '{}': external_objective is empty",
                    c.name
                ));
            }
            if c.internal_need.trim().is_empty() {
                errors.push(format!(
                    "character[{i}] '{}': internal_need is empty",
                    c.name
                ));
            }
            if c.voice_traits.is_empty() || c.voice_traits.len() > 6 {
                errors.push(format!(
                    "character[{i}] '{}': voice_traits must be 1-6 entries, got {}",
                    c.name,
                    c.voice_traits.len()
                ));
            }
            if c.voice_traits.iter().any(|t| t.trim().is_empty()) {
                errors.push(format!(
                    "character[{i}] '{}': voice_traits has an empty entry",
                    c.name
                ));
            }
            if expected_chapter_count > 0 && c.chapter_arc.len() != expected_chapter_count {
                errors.push(format!(
                    "character[{i}] '{}': chapter_arc has {} entries, expected {}",
                    c.name,
                    c.chapter_arc.len(),
                    expected_chapter_count
                ));
            }
        }

        if !has_protagonist {
            errors.push("no character has role 'protagonist'".to_owned());
        }

        // Duplicate-name check.
        let mut sorted = names.clone();
        sorted.sort();
        for w in sorted.windows(2) {
            if w[0] == w[1] {
                errors.push(format!("duplicate character name: '{}'", w[0]));
                break;
            }
        }

        // Relationship targets must reference an existing name.
        for c in &self.characters {
            for rel in &c.relationships {
                if !names.contains(&rel.to.as_str()) && c.name != rel.to {
                    errors.push(format!(
                        "character '{}' has relationship to unknown name '{}'",
                        c.name, rel.to
                    ));
                }
            }
        }

        errors
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// World Bible Agent (BACKLOG §A13 — fiction-shaped)
// ──────────────────────────────────────────────────────────────────────────────

/// A named location with story purpose, sensory signature, and constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldLocation {
    pub name: String,
    pub purpose_in_story: String,
    /// Specific sensory details (e.g. "pine resin and wet stone"), NOT
    /// generic mood-setters ("outdoor smells").
    pub sensory_signature: String,
    /// Rules that govern action there. Empty if none.
    pub key_constraints: String,
}

/// Per-sense palette so the drafter can pull concrete sensory detail
/// without inventing wildly different worlds across chapters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SensoryPalette {
    pub sight: String,
    pub sound: String,
    pub smell: String,
    pub touch: String,
    pub taste: String,
}

/// Output of the world-bible agent. Validated by `validate()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBibleProposal {
    pub main_locations: Vec<WorldLocation>,
    pub social_rules: Vec<String>,
    pub history: String,
    pub sensory_palette: SensoryPalette,
    pub conflict_sources: Vec<String>,
    pub symbolic_motifs: Vec<String>,
    /// Things the writer MUST NOT contradict in any chapter.
    pub continuity_constraints: Vec<String>,
}

impl WorldBibleProposal {
    pub fn validate(&self) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();

        if self.main_locations.is_empty() {
            errors.push("main_locations is empty".to_owned());
        }
        if self.main_locations.len() > 12 {
            errors.push(format!(
                "more than 12 main_locations ({}); MVP bible should stay focused",
                self.main_locations.len()
            ));
        }
        for (i, l) in self.main_locations.iter().enumerate() {
            if l.name.trim().is_empty() {
                errors.push(format!("location[{i}] has empty name"));
            }
            if l.purpose_in_story.trim().is_empty() {
                errors.push(format!(
                    "location[{i}] '{}': purpose_in_story is empty",
                    l.name
                ));
            }
            if l.sensory_signature.trim().is_empty() {
                errors.push(format!(
                    "location[{i}] '{}': sensory_signature is empty",
                    l.name
                ));
            }
        }

        if self.social_rules.is_empty() {
            errors.push("social_rules is empty".to_owned());
        }
        if self.history.split_whitespace().count() < 30 {
            errors.push("history is too short (<30 words) for a usable backstory".to_owned());
        }
        if self.continuity_constraints.is_empty() {
            errors.push(
                "continuity_constraints is empty; the bible needs at least one falsifiable rule"
                    .to_owned(),
            );
        }

        // Sensory palette sanity — at least 3 of 5 senses must be populated.
        let palette_filled = [
            !self.sensory_palette.sight.trim().is_empty(),
            !self.sensory_palette.sound.trim().is_empty(),
            !self.sensory_palette.smell.trim().is_empty(),
            !self.sensory_palette.touch.trim().is_empty(),
            !self.sensory_palette.taste.trim().is_empty(),
        ]
        .iter()
        .filter(|x| **x)
        .count();
        if palette_filled < 3 {
            errors.push(format!(
                "sensory_palette has only {palette_filled} of 5 senses populated; need at least 3",
            ));
        }

        errors
    }
}

impl ProposalValidation {
    /// Convenience: aggregate per-axis worst outcome → verdict.
    pub fn verdict_from_checks(checks: &[ValidationCheck]) -> ValidationVerdict {
        let any_fail = checks
            .iter()
            .any(|c| matches!(c.outcome, ValidationOutcome::Fail));
        let any_warn = checks
            .iter()
            .any(|c| matches!(c.outcome, ValidationOutcome::Warn));
        if any_fail {
            ValidationVerdict::Block
        } else if any_warn {
            ValidationVerdict::Warn
        } else {
            ValidationVerdict::Pass
        }
    }

    pub fn pass(summary: impl Into<String>) -> Self {
        Self {
            verdict: ValidationVerdict::Pass,
            checks: Vec::new(),
            summary: summary.into(),
            tier_2_ran: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(from: u32, to: u32, ex: &str) -> ContinuityEvidence {
        ContinuityEvidence {
            node_id: "01HXAAAAAAAAAAAAAAAAAAAAAA".into(),
            range_from: from,
            range_to: to,
            excerpt: ex.into(),
        }
    }

    #[test]
    fn copyedit_validate_rejects_fabricated_before() {
        let source = "hello  world";
        let p = CopyeditProposals {
            edits: vec![CopyeditEdit {
                range_from: 0,
                range_to: 5,
                before: "WRONG".into(),
                after: "fixed".into(),
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
                CopyeditEdit {
                    range_from: 0,
                    range_to: 5,
                    before: "abcde".into(),
                    after: "ABCDE".into(),
                    category: CopyeditCategory::Casing,
                    rationale: "x".into(),
                },
                CopyeditEdit {
                    range_from: 3,
                    range_to: 8,
                    before: "defgh".into(),
                    after: "DEFGH".into(),
                    category: CopyeditCategory::Casing,
                    rationale: "x".into(),
                },
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
                range_from: 4,
                range_to: 9,
                before: "quick".into(),
                // 1 → 4 words = 300 % change.
                after: "very very very fast".into(),
                category: CopyeditCategory::Other,
                rationale: "x".into(),
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
                kind: ContinuityKind::NameDrift,
                severity: Severity::Warning,
                evidence: vec![ev(0, 4, "Eli")],
                diagnosis: "alias drift".into(),
                proposed_fix: ContinuityFix {
                    kind: ContinuityFixKind::Rename,
                    from: None,
                    to: None,
                    scope: ContinuityFixScope::Project,
                },
            }],
        };
        let errs = r.validate();
        assert!(errs
            .iter()
            .any(|e| e.contains("rename fix requires both from and to")));
    }

    #[test]
    fn validation_verdict_aggregates_correctly() {
        let pass_check = ValidationCheck {
            axis: ValidationAxis::Schema,
            outcome: ValidationOutcome::Pass,
            evidence: "ok".into(),
            remediation: None,
        };
        let warn_check = ValidationCheck {
            axis: ValidationAxis::Length,
            outcome: ValidationOutcome::Warn,
            evidence: "long".into(),
            remediation: None,
        };
        let fail_check = ValidationCheck {
            axis: ValidationAxis::Schema,
            outcome: ValidationOutcome::Fail,
            evidence: "bad".into(),
            remediation: None,
        };
        assert_eq!(
            ProposalValidation::verdict_from_checks(&[pass_check.clone()]),
            ValidationVerdict::Pass
        );
        assert_eq!(
            ProposalValidation::verdict_from_checks(&[warn_check.clone()]),
            ValidationVerdict::Warn
        );
        assert_eq!(
            ProposalValidation::verdict_from_checks(&[fail_check, warn_check, pass_check]),
            ValidationVerdict::Block
        );
    }

    // ── CharacterBibleProposal voice_target wiring (FEATURE_HARDENING_PLAN.md §1.6)

    fn make_card(name: &str, role: &str) -> CharacterCard {
        CharacterCard {
            name: name.into(),
            role: role.into(),
            external_objective: "x".into(),
            internal_need: "x".into(),
            fear_or_wound: "x".into(),
            secret_or_contradiction: "x".into(),
            voice_traits: vec!["short staccato".into(), "specific nouns".into()],
            relationships: Vec::new(),
            chapter_arc: vec!["a".into(), "b".into()],
            emotional_turning_points: Vec::new(),
        }
    }

    #[test]
    fn bible_voice_target_directive_empty_when_none() {
        let b = CharacterBibleProposal::new(vec![make_card("Ada", "protagonist")]);
        assert_eq!(b.voice_target_directive(), "");
        assert!(b.voice_target.is_none());
    }

    #[test]
    fn bible_voice_target_directive_renders_when_set() {
        let mut b = CharacterBibleProposal::new(vec![make_card("Ada", "protagonist")]);
        b.voice_target = Some(booksforge_voice::VoiceTarget::literary_default());
        let dir = b.voice_target_directive();
        assert!(dir.contains("Voice contract"));
        assert!(dir.contains("MUST satisfy"));
        // The literary_default cap on consecutive same-3-tok openings is 3.
        assert!(dir.contains("No more than 3"));
    }

    #[test]
    fn bible_deserializes_old_json_without_voice_target_field() {
        // Bibles persisted before §1.6 had no voice_target key.
        // serde(default) must let them deserialize cleanly with None.
        let raw = r#"{
          "characters": [
            {
              "name": "Ada",
              "role": "protagonist",
              "external_objective": "x",
              "internal_need": "x",
              "fear_or_wound": "x",
              "secret_or_contradiction": "x",
              "voice_traits": ["short", "punchy"],
              "relationships": [],
              "chapter_arc": ["a", "b"],
              "emotional_turning_points": []
            }
          ]
        }"#;
        let b: CharacterBibleProposal = serde_json::from_str(raw).unwrap();
        assert!(
            b.voice_target.is_none(),
            "old-format bible must default voice_target to None"
        );
        assert_eq!(b.characters.len(), 1);
    }

    #[test]
    fn bible_voice_target_round_trips_through_json() {
        let mut original = CharacterBibleProposal::new(vec![make_card("Ada", "protagonist")]);
        original.voice_target = Some(booksforge_voice::VoiceTarget::commercial_default());
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("voice_target"));
        let back: CharacterBibleProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.voice_target, original.voice_target);
    }

    // ── SceneBeat (Run #14 Fix-1: scene-card decomposition) ─────────────

    fn good_beat() -> SceneBeat {
        SceneBeat {
            beat_id: "hook".into(),
            goal: "Elara opens the locked drawer".into(),
            conflict: "She has avoided this drawer for three weeks".into(),
            reveal: "The leather folio is heavier than expected".into(),
            target_words: 500,
        }
    }

    #[test]
    fn scene_beat_validates_well_formed_beat() {
        assert!(good_beat().validate().is_empty());
    }

    #[test]
    fn scene_beat_rejects_empty_beat_id() {
        let mut b = good_beat();
        b.beat_id = "".into();
        let errs = b.validate();
        assert!(errs.iter().any(|e| e.contains("beat_id is empty")));
    }

    #[test]
    fn scene_beat_rejects_empty_goal_conflict_reveal() {
        let mut b = good_beat();
        b.goal = "".into();
        b.conflict = "".into();
        b.reveal = "".into();
        let errs = b.validate();
        assert!(errs.iter().any(|e| e.contains("goal is empty")));
        assert!(errs.iter().any(|e| e.contains("conflict is empty")));
        assert!(errs.iter().any(|e| e.contains("reveal is empty")));
    }

    #[test]
    fn scene_beat_rejects_too_short_target() {
        let mut b = good_beat();
        b.target_words = 100;
        let errs = b.validate();
        assert!(errs.iter().any(|e| e.contains("< 200")));
    }

    #[test]
    fn scene_beat_rejects_too_long_target() {
        // 1200 is the upper band — beats above that should be split.
        let mut b = good_beat();
        b.target_words = 2000;
        let errs = b.validate();
        assert!(errs.iter().any(|e| e.contains("> 1200")));
    }

    #[test]
    fn scene_beat_round_trips_through_json() {
        let original = good_beat();
        let json = serde_json::to_string(&original).unwrap();
        let back: SceneBeat = serde_json::from_str(&json).unwrap();
        assert_eq!(back.beat_id, original.beat_id);
        assert_eq!(back.goal, original.goal);
        assert_eq!(back.target_words, original.target_words);
    }
}
