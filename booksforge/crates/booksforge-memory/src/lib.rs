//! Book/chapter/entity/style memory subsystem (Layer 3 — pure logic).
//!
//! Memory is *continuous*: every chapter save, accepted edit, and chapter
//! finalise updates it. All writes are typed and scoped — an agent declares
//! which tables it may write to; out-of-scope writes are rejected by the
//! Orchestrator.
//!
//! Schema and full implementation in M2/M3. See MEMORY_SYSTEM.md.

#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Scope of a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Book,
    Chapter,
    Entity,
    Style,
}

/// A typed memory entry written by the Memory Curator or Vocabulary Dictionary agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id:         Ulid,
    pub scope:      MemoryScope,
    pub key:        String,
    pub value_json: serde_json::Value,
    pub agent_id:   String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("write rejected: agent '{agent_id}' is not allowed to write to scope {scope:?}")]
    OutOfScopeWrite { agent_id: String, scope: MemoryScope },
    #[error("memory key not found: {key}")]
    NotFound { key: String },
}
