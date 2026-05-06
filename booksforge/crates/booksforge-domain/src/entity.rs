use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// The kind of entity in the series bible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Character,
    Location,
    Item,
    Organisation,
    Theme,
}

/// A named entity tracked by the Continuity and Memory Curator agents.
///
/// `aliases` are alternate spellings and nicknames. The Continuity Agent flags
/// uses of aliases that have drifted from `canonical_name`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id:             Ulid,
    pub kind:           EntityKind,
    pub canonical_name: String,
    pub aliases:        Vec<String>,
    pub description:    Option<String>,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
    pub deleted_at:     Option<DateTime<Utc>>,
}

impl Entity {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Returns `true` if `name` matches the canonical name or any alias
    /// (case-insensitive).
    pub fn matches_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.canonical_name.to_lowercase() == lower
            || self.aliases.iter().any(|a| a.to_lowercase() == lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_entity(canonical: &str, aliases: &[&str]) -> Entity {
        Entity {
            id:             Ulid::new(),
            kind:           EntityKind::Character,
            canonical_name: canonical.to_owned(),
            aliases:        aliases.iter().map(|s| s.to_string()).collect(),
            description:    None,
            created_at:     Utc::now(),
            updated_at:     Utc::now(),
            deleted_at:     None,
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
}
