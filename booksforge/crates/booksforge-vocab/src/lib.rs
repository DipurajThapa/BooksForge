//! Vocabulary subsystem facade + shipped starter dictionaries.
//!
//! Domain types live in `booksforge-domain::vocab`.  This crate adds:
//!
//!   • The shipped baseline dictionaries (compiled-in TOML) — `ai_tells`,
//!     `genre:fantasy`, `genre:romance`, `mode:non_fiction`.  These are
//!     loaded on project creation by the seed helper in `booksforge-storage`.
//!   • The `starter_entries(...)` constructor that turns a TOML file into
//!     a `Vec<VocabEntry>` ready for upsert.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use booksforge_domain::{EntryKind, EntrySource, VocabEntry};
use serde::Deserialize;

pub use booksforge_domain::vocab::{
    layer_specificity, replacement_for, resolve, EntryKind as VocabEntryKind,
    EntrySource as VocabEntrySource, VocabEntry as VocabRow, VocabError,
};

// ── Shipped TOML dictionaries (compile-time embed) ────────────────────────────

const AI_TELLS_V1: &str = include_str!("../dictionaries/ai-tells.toml");
const FANTASY_V1: &str = include_str!("../dictionaries/genre-fantasy.toml");
const ROMANCE_V1: &str = include_str!("../dictionaries/genre-romance.toml");
const NONFICTION_V1: &str = include_str!("../dictionaries/mode-non-fiction.toml");

/// All shipped starter layers, in load order.  The seed loader writes them
/// once at project creation; users can later override any row.
pub fn starter_layers() -> Vec<&'static str> {
    vec![
        "ai_tells",
        "genre:fantasy",
        "genre:romance",
        "mode:non_fiction",
    ]
}

/// Source bytes for a starter layer slug, or `None` if unknown.
pub fn starter_source(layer: &str) -> Option<&'static str> {
    match layer {
        "ai_tells" => Some(AI_TELLS_V1),
        "genre:fantasy" => Some(FANTASY_V1),
        "genre:romance" => Some(ROMANCE_V1),
        "mode:non_fiction" => Some(NONFICTION_V1),
        _ => None,
    }
}

/// Parse a shipped TOML dictionary into a list of `VocabEntry`s tagged
/// with `EntrySource::Starter`.  Lower-cased term + display term are both
/// preserved.
pub fn starter_entries(layer: &str) -> Result<Vec<VocabEntry>, VocabLoadError> {
    let raw =
        starter_source(layer).ok_or_else(|| VocabLoadError::UnknownLayer(layer.to_owned()))?;
    let parsed: TomlDictionary =
        toml::from_str(raw).map_err(|e| VocabLoadError::Parse(format!("layer '{layer}': {e}")))?;

    let mut out = Vec::with_capacity(parsed.entries.len());
    for raw_entry in parsed.entries {
        let kind = EntryKind::from_str(&raw_entry.kind)
            .ok_or_else(|| VocabLoadError::Parse(format!("invalid kind '{}'", raw_entry.kind)))?;
        let mut entry = VocabEntry::new(layer, &raw_entry.term, kind, EntrySource::Starter);
        if let Some(r) = raw_entry.replacement {
            entry = entry.with_replacement(r);
        }
        if let Some(r) = raw_entry.rationale {
            entry = entry.with_rationale(r);
        }
        out.push(entry);
    }
    Ok(out)
}

/// All shipped starter entries across every layer.  Convenience for the
/// seed loader.
pub fn all_starter_entries() -> Result<Vec<VocabEntry>, VocabLoadError> {
    let mut all = Vec::new();
    for layer in starter_layers() {
        all.extend(starter_entries(layer)?);
    }
    Ok(all)
}

// ── TOML schema for shipped dictionaries ─────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TomlDictionary {
    #[serde(default)]
    entries: Vec<TomlEntry>,
}

#[derive(Debug, Deserialize)]
struct TomlEntry {
    term: String,
    kind: String,
    #[serde(default)]
    replacement: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum VocabLoadError {
    #[error("unknown vocab layer slug: {0}")]
    UnknownLayer(String),
    #[error("vocab dictionary parse error: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_starter_layer_parses() {
        for layer in starter_layers() {
            let entries = starter_entries(layer).expect(layer);
            assert!(!entries.is_empty(), "{layer} must have at least one entry");
        }
    }

    #[test]
    fn ai_tells_layer_has_at_least_a_dozen_entries() {
        let entries = starter_entries("ai_tells").unwrap();
        assert!(
            entries.len() >= 12,
            "expected ≥12 ai-tells entries, got {}",
            entries.len()
        );
    }

    #[test]
    fn all_starter_entries_aggregates_layers() {
        let all = all_starter_entries().unwrap();
        let layers: std::collections::HashSet<&str> =
            all.iter().map(|e| e.layer.as_str()).collect();
        assert_eq!(layers.len(), starter_layers().len());
    }

    #[test]
    fn ai_tells_terms_are_lowercase() {
        for entry in starter_entries("ai_tells").unwrap() {
            assert_eq!(entry.term, entry.term.to_lowercase());
        }
    }
}
