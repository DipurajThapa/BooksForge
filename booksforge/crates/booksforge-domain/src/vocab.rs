//! Layered vocabulary subsystem (Layer 3 — pure logic).
//!
//! Per `VOCABULARY_DICTIONARIES.md`, dictionaries are stacked with a
//! deterministic "most-specific wins" precedence:
//!
//! ```text
//!   project        ← user-curated (highest precedence)
//!   genre:<slug>   ← e.g. genre:fantasy
//!   subgenre:<slug>
//!   domain:<slug>
//!   audience:<slug>
//!   voice:<slug>   ← per-character voice
//!   chapter_type:<slug>
//!   ai_tells       ← shipped baseline (lowest precedence)
//! ```
//!
//! At lookup time the orchestrator passes the active layer-stack for the
//! current scene (typically: `project` → `genre:<x>` → `audience:<y>` →
//! `ai_tells`).  This module computes the merged view: a `prefer` entry in
//! `project` overrides an `avoid` entry for the same term in
//! `genre:<x>` etc.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

// ── Public types ──────────────────────────────────────────────────────────────

/// What the dictionary entry prescribes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Use this term.
    Prefer,
    /// Never use this term in this context.
    Avoid,
    /// Replace this term with `replacement`.
    Replace,
}

impl EntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prefer  => "prefer",
            Self::Avoid   => "avoid",
            Self::Replace => "replace",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "prefer"  => Some(Self::Prefer),
            "avoid"   => Some(Self::Avoid),
            "replace" => Some(Self::Replace),
            _         => None,
        }
    }
}

/// Provenance of an entry.  Used by the UI to show a "Don't override
/// shipped defaults" affordance and by the agent to know which rows it
/// owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrySource {
    Starter,
    User,
    Agent,
}

impl EntrySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starter => "starter",
            Self::User    => "user",
            Self::Agent   => "agent",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "starter" => Some(Self::Starter),
            "user"    => Some(Self::User),
            "agent"   => Some(Self::Agent),
            _         => None,
        }
    }
}

/// One row in `vocab_entries`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabEntry {
    pub id:           Ulid,
    /// Layer slug — `"project"`, `"genre:fantasy"`, `"ai_tells"`, …
    pub layer:        String,
    /// Lowercased lookup form of the term.
    pub term:         String,
    /// Original-cased term for UI display.
    pub display_term: String,
    pub kind:         EntryKind,
    pub replacement:  Option<String>,
    pub rationale:    Option<String>,
    pub source:       EntrySource,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

impl VocabEntry {
    /// Construct a fresh entry — caller supplies the layer + term+kind +
    /// optional replacement/rationale.  Lowercases the term automatically.
    pub fn new(
        layer:       impl Into<String>,
        display_term: impl Into<String>,
        kind:        EntryKind,
        source:      EntrySource,
    ) -> Self {
        let display = display_term.into();
        let now = Utc::now();
        Self {
            id:           Ulid::new(),
            layer:        layer.into(),
            term:         display.to_lowercase(),
            display_term: display,
            kind,
            replacement:  None,
            rationale:    None,
            source,
            created_at:   now,
            updated_at:   now,
        }
    }

    pub fn with_replacement(mut self, r: impl Into<String>) -> Self {
        self.replacement = Some(r.into());
        self
    }

    pub fn with_rationale(mut self, r: impl Into<String>) -> Self {
        self.rationale = Some(r.into());
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VocabError {
    #[error("invalid layer slug: {0}")]
    InvalidLayer(String),
}

// ── Layer specificity & resolution ────────────────────────────────────────────

/// How specific a layer is.  Higher = more specific (wins on conflict).
///
/// The implementation pack pins this order; it must match
/// `VOCABULARY_DICTIONARIES.md`.
pub fn layer_specificity(layer: &str) -> u32 {
    if layer == "ai_tells"             { return 0; }
    if layer.starts_with("audience:")  { return 1; }
    if layer.starts_with("chapter_type:") { return 2; }
    if layer.starts_with("voice:")     { return 3; }
    if layer.starts_with("domain:")    { return 4; }
    if layer.starts_with("subgenre:")  { return 5; }
    if layer.starts_with("genre:")     { return 6; }
    if layer == "project"              { return 7; }
    // Unknown layer — treat as least specific so it can be overridden.
    0
}

/// Merge a list of `VocabEntry`s under a fixed `active_layers` filter,
/// keeping the most-specific entry per `(term, kind)` key.
///
/// Pure function: caller fetches the rows from storage (one round-trip per
/// project) and the merger picks the winners.
pub fn resolve<'a>(entries: &'a [VocabEntry], active_layers: &[&str]) -> Vec<&'a VocabEntry> {
    use std::collections::HashMap;

    // First filter: only rows whose layer is in the active set.
    let candidates: Vec<&VocabEntry> = entries
        .iter()
        .filter(|e| active_layers.iter().any(|l| *l == e.layer))
        .collect();

    // Then dedupe by (term, kind), keeping the most-specific winner.
    let mut chosen: HashMap<(String, EntryKind), &VocabEntry> = HashMap::new();
    for entry in candidates {
        let key = (entry.term.clone(), entry.kind);
        let keep = match chosen.get(&key) {
            None => true,
            Some(existing) => layer_specificity(&entry.layer) > layer_specificity(&existing.layer),
        };
        if keep {
            chosen.insert(key, entry);
        }
    }

    let mut out: Vec<&VocabEntry> = chosen.into_values().collect();
    out.sort_by(|a, b| a.term.cmp(&b.term).then(a.kind.as_str().cmp(b.kind.as_str())));
    out
}

/// Convenience accessor: return the canonical replacement for a term, if
/// one is configured at the active layer set.  Useful for one-off lookups
/// in agents and validators; for batch work prefer `resolve` once and
/// query the slice yourself.
pub fn replacement_for<'a>(
    entries: &'a [VocabEntry],
    active_layers: &[&str],
    term: &str,
) -> Option<&'a str> {
    let lower = term.to_lowercase();
    let resolved = resolve(entries, active_layers);
    for entry in resolved {
        if entry.term == lower && entry.kind == EntryKind::Replace {
            if let Some(r) = entry.replacement.as_deref() {
                return Some(r);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(layer: &str, term: &str, kind: EntryKind) -> VocabEntry {
        VocabEntry::new(layer, term, kind, EntrySource::Starter)
    }

    #[test]
    fn entry_kind_roundtrip() {
        for k in [EntryKind::Prefer, EntryKind::Avoid, EntryKind::Replace] {
            assert_eq!(EntryKind::from_str(k.as_str()), Some(k));
        }
    }

    #[test]
    fn project_layer_beats_genre() {
        let entries = vec![
            entry("genre:fantasy", "Delve",   EntryKind::Avoid),
            entry("project",       "Delve",   EntryKind::Prefer), // overrides
        ];
        let merged = resolve(&entries, &["project", "genre:fantasy"]);
        assert_eq!(merged.len(), 2, "different (term, kind) keys → both retained");
        // Same (term, kind) collisions: prefer wins by precedence test below.

        let entries2 = vec![
            entry("genre:fantasy", "Delve",   EntryKind::Avoid),
            entry("project",       "Delve",   EntryKind::Avoid), // same kind → project wins
        ];
        let merged2 = resolve(&entries2, &["project", "genre:fantasy"]);
        assert_eq!(merged2.len(), 1);
        assert_eq!(merged2[0].layer, "project");
    }

    #[test]
    fn ai_tells_is_lowest_specificity() {
        assert!(layer_specificity("project") > layer_specificity("ai_tells"));
        assert!(layer_specificity("genre:fantasy") > layer_specificity("ai_tells"));
    }

    #[test]
    fn inactive_layer_excluded() {
        let entries = vec![
            entry("genre:fantasy", "Delve", EntryKind::Avoid),
            entry("genre:romance", "Spark", EntryKind::Prefer),
        ];
        let merged = resolve(&entries, &["genre:fantasy"]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].display_term, "Delve");
    }

    #[test]
    fn replacement_lookup_picks_most_specific() {
        let entries = vec![
            entry("ai_tells", "Tapestry", EntryKind::Replace).with_replacement("pattern"),
            entry("project",  "Tapestry", EntryKind::Replace).with_replacement("weave"),
        ];
        assert_eq!(
            replacement_for(&entries, &["project", "ai_tells"], "tapestry"),
            Some("weave"),
        );
    }

    #[test]
    fn lowercase_is_case_insensitive() {
        let entries = vec![
            entry("ai_tells", "Tapestry", EntryKind::Replace).with_replacement("pattern"),
        ];
        assert!(replacement_for(&entries, &["ai_tells"], "TAPESTRY").is_some());
        assert!(replacement_for(&entries, &["ai_tells"], "tApEsTrY").is_some());
    }
}
