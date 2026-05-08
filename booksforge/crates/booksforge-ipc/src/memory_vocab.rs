//! IPC types for the memory + vocabulary subsystems (Phase 3 follow-up).
//!
//! These commands expose Phase 3's storage CRUD to the UI so writers can
//! inspect the per-project memory ledger and the active vocabulary
//! dictionaries without spelunking the SQLite file.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Memory ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MemoryEntryDto {
    pub id:         String,
    /// "book" | "chapter" | "entity" | "style".
    pub scope:      String,
    pub key:        String,
    /// JSON-stringified value — the structure depends on scope.
    pub value_json: String,
    pub agent_id:   String,
    /// ISO-8601 UTC.
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MemoryListInput {
    /// One of: "book" | "chapter" | "entity" | "style".
    pub scope: String,
}

// ── Vocabulary ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VocabEntryDto {
    pub id:           String,
    pub layer:        String,
    pub term:         String,
    pub display_term: String,
    /// "prefer" | "avoid" | "replace".
    pub kind:         String,
    pub replacement:  Option<String>,
    pub rationale:    Option<String>,
    /// "starter" | "user" | "agent".
    pub source:       String,
    pub created_at:   String,
    pub updated_at:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VocabListInput {
    /// Layer slugs to fetch.  Empty = no rows returned.  Common defaults:
    /// `["project", "ai_tells"]`.
    pub layers: Vec<String>,
}
