use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Where in the document tree a node lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Project,
    Part,
    Chapter,
    Scene,
    FrontMatter,
    BackMatter,
}

impl NodeKind {
    /// Returns `true` if this kind can hold child nodes.
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Project | Self::Part | Self::Chapter)
    }

    /// Returns `true` if this kind holds actual prose content.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Scene | Self::FrontMatter | Self::BackMatter)
    }
}

/// Editorial status of a node, driven by the writer's workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    #[default]
    Planned,
    Drafting,
    Revised,
    Final,
}

/// A node in the document tree (Project → Part → Chapter → Scene).
///
/// `position` is a LexoRank string (e.g. `"0|hzzzzz:"`).  Siblings are
/// ordered lexicographically.  Rebalancing is handled in the storage layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Ulid,
    pub parent_id: Option<Ulid>,
    pub kind: NodeKind,
    pub title: String,
    /// LexoRank string — lexicographic ordering.  Default: `"0|hzzzzz:"`.
    pub position: String,
    pub status: NodeStatus,
    pub pov: Option<String>,
    pub beat: Option<String>,
    pub target_words: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Node {
    pub const DEFAULT_POSITION: &'static str = "0|hzzzzz:";

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// The prose content of a Scene or FrontMatter/BackMatter node.
///
/// `pm_doc` is the ProseMirror document serialised as JSON. The storage layer
/// writes it verbatim; the editor layer owns interpretation.
///
/// `hash` is the blake3 hex digest of the JSON representation, updated on
/// every save.  It is used for snapshot deduplication and change detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContent {
    pub node_id: Ulid,
    pub pm_doc: serde_json::Value,
    pub word_count: u32,
    pub char_count: u32,
    /// blake3 hex hash of `pm_doc` (updated on every save).
    pub hash: String,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_kind_container_vs_leaf() {
        assert!(NodeKind::Project.is_container());
        assert!(NodeKind::Chapter.is_container());
        assert!(!NodeKind::Scene.is_container());
        assert!(NodeKind::Scene.is_leaf());
        assert!(!NodeKind::Part.is_leaf());
    }
}
