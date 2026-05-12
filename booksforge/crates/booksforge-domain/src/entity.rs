use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// The kind of entity in the series bible.
///
/// Maps to the `kind` column CHECK constraint in the `entities` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Character,
    Location,
    Item,
    Organisation,
    Theme,
    Custom,
}

/// A named entity tracked by the Continuity and Memory Curator agents.
///
/// `aliases` is the aggregated view of the `entity_aliases` join table; it is
/// populated by the storage layer on load.  The `fields_json` blob stores
/// kind-specific attributes (age, role, eye colour for characters, coordinates
/// for locations, etc.) as a free-form JSON object — schema is per-kind in the
/// entity template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Ulid,
    pub kind: EntityKind,
    /// The canonical display name (primary key for matching).
    pub name: String,
    /// Alternate spellings / nicknames, loaded from `entity_aliases`.
    pub aliases: Vec<String>,
    /// Kind-specific structured attributes as JSON.
    pub fields_json: serde_json::Value,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Entity {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Returns `true` if `query` matches the canonical name or any alias
    /// (case-insensitive, trimmed).
    pub fn matches_name(&self, query: &str) -> bool {
        let lower = query.trim().to_lowercase();
        self.name.to_lowercase() == lower || self.aliases.iter().any(|a| a.to_lowercase() == lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(name: &str, aliases: &[&str]) -> Entity {
        Entity {
            id: Ulid::new(),
            kind: EntityKind::Character,
            name: name.to_owned(),
            aliases: aliases.iter().map(|s| (*s).to_string()).collect(),
            fields_json: serde_json::json!({}),
            notes: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn matches_canonical_case_insensitive() {
        let e = make_entity("Aidan Morrow", &["Aiden"]);
        assert!(e.matches_name("aidan morrow"));
        assert!(e.matches_name("AIDAN MORROW"));
    }

    #[test]
    fn matches_alias() {
        let e = make_entity("Aidan Morrow", &["Aiden", "Aid"]);
        assert!(e.matches_name("aiden"));
        assert!(e.matches_name("AID"));
    }

    #[test]
    fn does_not_match_unrelated_name() {
        let e = make_entity("Aidan Morrow", &["Aiden"]);
        assert!(!e.matches_name("Marcus"));
    }

    #[test]
    fn trims_whitespace_before_matching() {
        let e = make_entity("Alice", &[]);
        assert!(e.matches_name("  Alice  "));
    }
}
