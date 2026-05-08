use booksforge_domain::{Entity, EntityKind};
use chrono::Utc;
use ulid::Ulid;

pub fn protagonist() -> Entity {
    Entity {
        id:          Ulid::from_string("01HZFAKEENTITYID000000001").unwrap_or_else(|_| Ulid::new()),
        kind:        EntityKind::Character,
        name:        "Alice Mercer".to_owned(),
        aliases:     vec!["Alice".to_owned(), "Mercer".to_owned()],
        fields_json: serde_json::json!({
            "age":       29,
            "occupation":"archivist",
            "eye_color": "hazel"
        }),
        notes:       "Protagonist of The Amber Key.".to_owned(),
        created_at:  Utc::now(),
        updated_at:  Utc::now(),
        deleted_at:  None,
    }
}
