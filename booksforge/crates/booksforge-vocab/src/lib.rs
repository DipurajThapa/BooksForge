//! Vocabulary dictionary subsystem (Layer 3 — pure logic).
//!
//! Dictionaries are layered: book → genre → sub-genre → domain → audience →
//! character-voice → chapter-type. Lookups merge all applicable layers;
//! the more-specific layer wins on conflicts.
//!
//! Full implementation in M2/M3. See VOCABULARY_DICTIONARIES.md.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// What the dictionary entry prescribes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Use this term.
    Prefer,
    /// Never use this term in this context.
    Avoid,
    /// Replace this term with `replacement`.
    Replace,
}

/// A single vocabulary dictionary entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabEntry {
    pub term:        String,
    pub kind:        EntryKind,
    pub replacement: Option<String>,
    pub rationale:   Option<String>,
    /// Context tags: genre, sub-genre, domain, audience, voice, chapter-type.
    pub context_tags: Vec<String>,
}

/// Layered lookup: returns the entries that apply to the given context tags,
/// with more-specific layers overriding less-specific ones.
pub fn lookup<'a>(
    entries: &'a [VocabEntry],
    context_tags: &[&str],
) -> Vec<&'a VocabEntry> {
    entries
        .iter()
        .filter(|e| {
            e.context_tags.is_empty()
                || e.context_tags.iter().any(|t| context_tags.contains(&t.as_str()))
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum VocabError {
    #[error("dictionary not found for layer: {layer}")]
    LayerNotFound { layer: String },
}
