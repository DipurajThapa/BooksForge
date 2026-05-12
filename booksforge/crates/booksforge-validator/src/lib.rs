//! Manuscript validator engine (Layer 3 — pure logic).
//!
//! Each validator is a deterministic function that walks a [`ValidatorContext`]
//! and produces zero or more [`ValidatorIssue`]s.  No I/O, no clocks (the
//! caller stamps `ran_at`), no LLM calls.
//!
//! The MVP ships 16 deterministic validators plus the `pre_export_gate`
//! policy from `booksforge-domain::validator`.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::time::Instant;

use booksforge_domain::{
    validator::{Severity, ValidationReport, ValidatorIssue, ValidatorRun, ValidatorRunStatus},
    Node, NodeKind, StyleBook, VocabEntry,
};
use chrono::Utc;
use ulid::Ulid;

pub mod agent_outputs;
pub mod continuity;
pub mod originality;
pub mod validators;

pub use agent_outputs::{validate_agent_output, AgentOutputContext, AgentOutputReport};
pub use continuity::{
    detect_name_drift, detect_pov_drift, detect_tense_drift, detect_timeline, lint_scene,
};
pub use originality::{detect_self_plagiarism, detect_verbatim_overlap, OverlapHit, OverlapKind};
pub use validators::ALL_VALIDATORS;

// ── Context ──────────────────────────────────────────────────────────────────

/// One scene's plain-text body, paired with its node id.
#[derive(Debug, Clone)]
pub struct SceneText {
    pub node_id: Ulid,
    pub text: String,
}

/// Project metadata snapshot — kept tiny so callers can synthesise from
/// either the manifest or the AppState without pulling everything in.
#[derive(Debug, Clone, Default)]
pub struct ProjectMetaSummary {
    pub title: String,
    pub author: String,
    /// BCP-47 language tag.  Empty when the project hasn't recorded one.
    pub language: String,
    /// Optional ISBN — only validated when present.
    pub isbn: Option<String>,
}

/// Everything a validator needs to run.  Pure logic — caller (the
/// Tauri command) is responsible for fetching nodes / scenes / vocab from
/// storage and rendering pm_doc to plain text.
pub struct ValidatorContext<'a> {
    pub nodes: &'a [Node],
    pub scenes: &'a [SceneText],
    pub style: &'a StyleBook,
    pub vocab: &'a [VocabEntry],
    pub active_vocab_layers: &'a [&'a str],
    /// Project meta summary used by the KDP metadata validator (G3).
    /// `None` is interpreted as "metadata not available" — KDP checks
    /// gracefully no-op so existing projects don't suddenly fail.
    pub project: Option<&'a ProjectMetaSummary>,
}

impl ValidatorContext<'_> {
    /// Find the title of a node by id (best-effort, blank string on miss).
    pub fn node_title(&self, id: Ulid) -> &str {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.title.as_str())
            .unwrap_or("")
    }
}

// ── Public batch runner ──────────────────────────────────────────────────────

/// Run every shipped validator against the supplied context.  Returns a
/// fully-populated [`ValidationReport`] including timing and a content
/// hash for caching purposes.
pub fn run_all_validators(ctx: &ValidatorContext) -> ValidationReport {
    let started = Instant::now();
    let mut issues: Vec<ValidatorIssue> = Vec::new();
    for v in ALL_VALIDATORS {
        issues.extend((v.run)(ctx));
    }
    issues.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.validator_id.cmp(&b.validator_id))
            .then(a.message.cmp(&b.message))
    });

    let duration_ms = started.elapsed().as_millis() as u64;
    let scope_hash = compute_scope_hash(ctx);

    let run = ValidatorRun {
        id: Ulid::new(),
        validator_id: "batch:all".into(),
        ran_at: Utc::now(),
        status: ValidatorRunStatus::from_issues(&issues),
        duration_ms,
        scope_hash,
    };

    ValidationReport { run, issues }
}

/// Stable hash of the scoped manuscript content — used to short-circuit
/// re-runs over unchanged input.  Intentionally flat / cheap: hashes
/// every node title + every scene body in document order.
///
/// Public so the Tauri command can compute it before running the full
/// battery and consult `latest_validator_run` for a cache hit.
pub fn compute_scope_hash(ctx: &ValidatorContext) -> String {
    let mut hasher = blake3::Hasher::new();
    for node in ctx.nodes {
        hasher.update(node.id.to_string().as_bytes());
        hasher.update(b"\x1f");
        hasher.update(node.title.as_bytes());
        hasher.update(b"\x1e");
    }
    for scene in ctx.scenes {
        hasher.update(scene.node_id.to_string().as_bytes());
        hasher.update(b"\x1f");
        hasher.update(scene.text.as_bytes());
        hasher.update(b"\x1e");
    }
    hasher.finalize().to_hex().to_string()
}

// ── Validator descriptor ──────────────────────────────────────────────────────

/// One validator's metadata + run function.
pub struct Validator {
    pub id: &'static str,
    pub description: &'static str,
    pub run: fn(&ValidatorContext) -> Vec<ValidatorIssue>,
    /// Optional one-click deterministic fix.  Operates on the scene's
    /// `pm_doc` JSON in place and returns the number of replacements
    /// applied (0 → caller can no-op, > 0 → caller persists).  Validators
    /// that mark issues as `auto_fixable: true` should always supply one.
    pub fix: Option<fn(&mut serde_json::Value, &ValidatorContext) -> u32>,
}

// ── pm_doc text-node walker (used by every fix) ───────────────────────────────

/// Walk every `{ type: "text", text: "…" }` leaf in a pm_doc and apply
/// `f` to its text.  Returns the total number of nodes whose text was
/// rewritten (i.e. for which `f` returned `Some`).  Pure mutation —
/// preserves all marks, attrs, and structural blocks.
pub fn walk_text_nodes_mut(
    pm_doc: &mut serde_json::Value,
    mut f: impl FnMut(&str) -> Option<String>,
) -> u32 {
    fn walk(
        node: &mut serde_json::Value,
        f: &mut dyn FnMut(&str) -> Option<String>,
        count: &mut u32,
    ) {
        if let Some(obj) = node.as_object_mut() {
            // Text leaves: rewrite their `text` field if `f` returns Some.
            if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    if let Some(new_text) = f(text) {
                        if new_text != text {
                            obj.insert("text".to_owned(), serde_json::Value::String(new_text));
                            *count += 1;
                        }
                    }
                }
            }
            // Recurse into `content` arrays.
            if let Some(content) = obj.get_mut("content") {
                if let Some(arr) = content.as_array_mut() {
                    for child in arr {
                        walk(child, f, count);
                    }
                }
            }
        }
    }
    let mut count = 0;
    walk(pm_doc, &mut f, &mut count);
    count
}

/// Run a registered validator's fix in place.  Returns the number of
/// changed text nodes.  Returns `None` if the validator has no fix.
pub fn apply_fix(
    validator_id: &str,
    pm_doc: &mut serde_json::Value,
    ctx: &ValidatorContext,
) -> Option<u32> {
    ALL_VALIDATORS
        .iter()
        .find(|v| v.id == validator_id)
        .and_then(|v| v.fix)
        .map(|f| f(pm_doc, ctx))
}

// ── Helper used by every validator ────────────────────────────────────────────

pub(crate) fn issue(
    validator_id: &str,
    code: &str,
    severity: Severity,
    message: impl Into<String>,
    node_id: Option<Ulid>,
    offset_from: Option<u32>,
    offset_to: Option<u32>,
    auto_fixable: bool,
) -> ValidatorIssue {
    ValidatorIssue {
        validator_id: validator_id.to_owned(),
        code: code.to_owned(),
        severity,
        message: message.into(),
        node_id,
        offset_from,
        offset_to,
        auto_fixable,
    }
}

// ── Helpers re-exported for tests / external use ─────────────────────────────

/// Length of an in-document scene body.
pub fn scene_word_count(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

/// Whether `kind` denotes a node we expect to contain prose body text.
pub fn is_prose_node(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Scene | NodeKind::FrontMatter | NodeKind::BackMatter
    )
}
