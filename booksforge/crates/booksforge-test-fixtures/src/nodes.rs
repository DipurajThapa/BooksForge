use booksforge_domain::{Node, NodeKind, NodeStatus};
use chrono::Utc;
use ulid::Ulid;

/// A deterministic chapter node for use in tests.
pub fn chapter_node() -> Node {
    Node {
        id: Ulid::from_string("01HZFAKECHAPTERNODE00000001").unwrap_or_else(|_| Ulid::new()),
        parent_id: None,
        kind: NodeKind::Chapter,
        title: "Chapter One".to_owned(),
        position: "0|hzzzzz:".to_owned(),
        status: NodeStatus::Drafting,
        pov: Some("Alice".to_owned()),
        beat: Some("Inciting incident".to_owned()),
        target_words: Some(2_500),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    }
}

/// A scene node nested under `chapter_node`.
pub fn scene_node() -> Node {
    Node {
        id: Ulid::from_string("01HZFAKESCENENODEEE00000001").unwrap_or_else(|_| Ulid::new()),
        parent_id: Some(
            Ulid::from_string("01HZFAKECHAPTERNODE00000001").unwrap_or_else(|_| Ulid::new()),
        ),
        kind: NodeKind::Scene,
        title: "Scene One".to_owned(),
        position: "0|i00000:".to_owned(),
        status: NodeStatus::Planned,
        pov: None,
        beat: None,
        target_words: Some(800),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    }
}
