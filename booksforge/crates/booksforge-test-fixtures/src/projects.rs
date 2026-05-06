use booksforge_domain::{BookMode, Project, ProjectMeta};
use ulid::Ulid;

pub fn fiction_project() -> Project {
    Project {
        id:      Ulid::from_string("01HZFAKEPROJECTID000000001").unwrap_or_else(|_| Ulid::new()),
        mode:    BookMode::Fiction,
        meta:    ProjectMeta {
            title:        "The Amber Key".to_owned(),
            subtitle:     None,
            authors:      vec!["Jane Doe".to_owned()],
            language:     "en".to_owned(),
            target_words: Some(80_000),
        },
        schema_version: booksforge_domain::Project::CURRENT_SCHEMA_VERSION,
    }
}
