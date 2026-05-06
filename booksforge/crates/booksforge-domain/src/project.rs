use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// The writing mode, which determines templates, agents, and validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookMode {
    Fiction,
    NonFiction,
    Memoir,
    /// Academic mode is reduced in MVP: footnotes + CSL citations only.
    Academic,
}

/// Human-readable metadata stored in `manifest.toml [meta]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub title:        String,
    pub subtitle:     Option<String>,
    pub authors:      Vec<String>,
    /// BCP-47 language tag, e.g. "en-US", "fr-FR".
    pub language:     String,
    pub target_words: Option<u32>,
}

/// The project root — one per `.booksforge/` bundle.
///
/// `schema_version` starts at 1 for all MVP projects (greenfield; no migration
/// from any prior format). Incrementing it is a major decision (see ARCHITECTURE_DECISIONS.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id:               Ulid,
    pub schema_version:   u32,
    pub mode:             BookMode,
    pub template_id:      String,
    pub template_version: String,
    pub meta:             ProjectMeta,
    pub ai_enabled:       bool,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
}

impl Project {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;
}
