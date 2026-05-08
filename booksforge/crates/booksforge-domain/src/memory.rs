//! Book / chapter / entity / style memory subsystem (Layer 3 — pure logic).
//!
//! Memory is *continuous*: every chapter save, accepted edit, and chapter
//! finalise updates it.  All writes are typed and scoped — an agent declares
//! which scopes it may write to; out-of-scope writes are rejected by the
//! orchestrator per the agent's spec in `AGENTS.md`.
//!
//! The on-disk schema uses one flat `memory_entries` table keyed by
//! `(scope, key)`.  This module owns the value-object types; storage CRUD
//! lives in `booksforge-storage`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Scope of a memory entry.  Maps to the `memory_entries.scope` CHECK
/// constraint in the migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    /// Whole-book facts (genre, setting, voice, themes).
    Book,
    /// Chapter-level summaries and rolling beats.
    Chapter,
    /// Entity-level data (character traits, locations, items).
    Entity,
    /// Style memory (em-dash, oxford comma, etc.).
    Style,
}

impl MemoryScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Book    => "book",
            Self::Chapter => "chapter",
            Self::Entity  => "entity",
            Self::Style   => "style",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "book"    => Some(Self::Book),
            "chapter" => Some(Self::Chapter),
            "entity"  => Some(Self::Entity),
            "style"   => Some(Self::Style),
            _         => None,
        }
    }
}

/// One row in the `memory_entries` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id:         Ulid,
    pub scope:      MemoryScope,
    /// Human-readable lookup key — e.g. `"protagonist"`, `"chapter:01:summary"`.
    pub key:        String,
    pub value_json: serde_json::Value,
    /// The agent that wrote this entry (for audit and out-of-scope checks).
    pub agent_id:   String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Errors raised by memory operations.  Storage-side errors stay typed;
/// authorisation failures are domain-pure so the orchestrator can reject
/// without hitting the database.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("write rejected: agent '{agent_id}' is not allowed to write to scope {scope:?}")]
    OutOfScopeWrite { agent_id: String, scope: MemoryScope },
    #[error("memory key not found: {scope:?} / {key}")]
    NotFound { scope: MemoryScope, key: String },
}

// ── Agent-scope authorisation (pure-logic) ────────────────────────────────────

/// What scopes a given agent may write to.  Pulled into the orchestrator
/// so cross-agent contamination is caught at the boundary.
///
/// Per `AGENTS.md §3` — every agent declares its memory writes; this is the
/// machine-readable mirror.  Agents not listed get **zero** write access.
pub fn allowed_write_scopes(agent_id: &str) -> &'static [MemoryScope] {
    match agent_id {
        // Memory Curator owns everything except style (which is the
        // copyeditor's territory).
        "memory-curator"     => &[
            MemoryScope::Book,
            MemoryScope::Chapter,
            MemoryScope::Entity,
        ],
        // Vocabulary Dictionary writes style + entity (entity for character
        // voice / canonical names).
        "vocab-dictionary"   => &[MemoryScope::Style, MemoryScope::Entity],
        // Continuity reads everything but writes only entity facts it
        // confirms (e.g. "Aidan's eye colour = hazel").
        "continuity"         => &[MemoryScope::Entity],
        // Copyeditor owns the style-book.
        "copyeditor"         => &[MemoryScope::Style],
        // Outline architect seeds book-level memory at intake time.
        "outline-architect"  => &[MemoryScope::Book],
        // Everyone else: read-only by default.
        _ => &[],
    }
}

/// Check that `agent_id` may write to `scope`.  Returns `Ok` or a typed
/// error suitable for surfacing through `OrchestratorError`.
pub fn authorise_write(
    agent_id: &str,
    scope:    MemoryScope,
) -> Result<(), MemoryError> {
    if allowed_write_scopes(agent_id).contains(&scope) {
        Ok(())
    } else {
        Err(MemoryError::OutOfScopeWrite {
            agent_id: agent_id.to_owned(),
            scope,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_roundtrip() {
        for s in [MemoryScope::Book, MemoryScope::Chapter, MemoryScope::Entity, MemoryScope::Style] {
            assert_eq!(MemoryScope::from_str(s.as_str()), Some(s));
        }
    }

    #[test]
    fn memory_curator_can_write_book_chapter_entity() {
        assert!(authorise_write("memory-curator", MemoryScope::Book).is_ok());
        assert!(authorise_write("memory-curator", MemoryScope::Chapter).is_ok());
        assert!(authorise_write("memory-curator", MemoryScope::Entity).is_ok());
    }

    #[test]
    fn memory_curator_cannot_write_style() {
        let err = authorise_write("memory-curator", MemoryScope::Style).unwrap_err();
        assert!(matches!(err, MemoryError::OutOfScopeWrite { .. }));
    }

    #[test]
    fn copyeditor_only_writes_style() {
        assert!(authorise_write("copyeditor", MemoryScope::Style).is_ok());
        assert!(authorise_write("copyeditor", MemoryScope::Book).is_err());
        assert!(authorise_write("copyeditor", MemoryScope::Entity).is_err());
    }

    #[test]
    fn unknown_agent_has_zero_write_scopes() {
        assert!(allowed_write_scopes("totally-made-up").is_empty());
        assert!(authorise_write("totally-made-up", MemoryScope::Book).is_err());
    }
}
