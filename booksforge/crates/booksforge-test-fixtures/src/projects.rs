use booksforge_domain::{BookMode, Project, ProjectMeta};
use chrono::Utc;
use ulid::Ulid;

pub fn fiction_project() -> Project {
    let now = Utc::now();
    Project {
        id:               Ulid::from_string("01HZFAKEPROJECTID000000001")
                              .unwrap_or_else(|_| Ulid::new()),
        schema_version:   Project::CURRENT_SCHEMA_VERSION,
        mode:             BookMode::Fiction,
        template_id:      "fiction-novel".to_owned(),
        template_version: "v1".to_owned(),
        meta: ProjectMeta {
            title:        "The Amber Key".to_owned(),
            subtitle:     None,
            authors:      vec!["Jane Doe".to_owned()],
            language:     "en-US".to_owned(),
            target_words: Some(80_000),
        },
        ai_enabled:       false,
        created_at:       now,
        updated_at:       now,
    }
}

// ── Schema-drift watch (BACKLOG §L2) ────────────────────────────────────
//
// `fiction_project()` is consumed by every test that needs a realistic
// project shell.  If the `Project` / `ProjectMeta` struct gains or
// renames fields without the fixture being updated, downstream tests
// would silently lose coverage of the new field.  The tests below
// enforce that the fixture:
//
//   1. Round-trips through serde JSON without losing any field.
//   2. Sets `schema_version == Project::CURRENT_SCHEMA_VERSION`.
//   3. Has all required fields populated (no defaults sneaking in).
//
// If any of these break, you've changed the canonical project shape;
// update `fiction_project()` to match before merging.

#[cfg(test)]
mod schema_drift {
    use super::*;

    #[test]
    fn fixture_round_trips_through_json() {
        let p = fiction_project();
        let json = serde_json::to_string(&p).expect("serialize");
        let back: Project = serde_json::from_str(&json).expect("round-trip parse");
        assert_eq!(p.id,               back.id);
        assert_eq!(p.schema_version,   back.schema_version);
        assert_eq!(p.template_id,      back.template_id);
        assert_eq!(p.template_version, back.template_version);
        assert_eq!(p.meta.title,       back.meta.title);
        assert_eq!(p.meta.authors,     back.meta.authors);
        assert_eq!(p.meta.language,    back.meta.language);
        assert_eq!(p.meta.target_words, back.meta.target_words);
        assert_eq!(p.ai_enabled,       back.ai_enabled);
    }

    #[test]
    fn fixture_uses_current_schema_version() {
        assert_eq!(
            fiction_project().schema_version,
            Project::CURRENT_SCHEMA_VERSION,
            "fixture is on stale schema_version — bump it after migrations land"
        );
    }

    /// Compile-time guard: exhaustive destructuring fails to compile if
    /// `Project` or `ProjectMeta` gains a new field.  If you land here
    /// after adding a field, update `fiction_project()` above to
    /// populate it — that's the single update site BACKLOG §L2 asks for.
    #[test]
    fn fixture_destructures_exhaustively() {
        let p = fiction_project();
        let Project {
            id: _, schema_version: _, mode: _,
            template_id: _, template_version: _,
            meta, ai_enabled: _,
            created_at: _, updated_at: _,
        } = p;
        let ProjectMeta {
            title: _, subtitle: _, authors: _, language: _, target_words: _,
        } = meta;
    }

    #[test]
    fn fixture_has_realistic_required_fields() {
        let p = fiction_project();
        assert!(!p.meta.title.is_empty(),       "title must be non-empty");
        assert!(!p.meta.authors.is_empty(),     "at least one author");
        assert!(!p.meta.language.is_empty(),    "language tag required");
        assert!(!p.template_id.is_empty(),      "template_id required");
        assert!(p.meta.target_words.is_some(),  "target_words exercised by export tests");
        // ai_enabled defaults to false per the AI-consent contract;
        // tests that need it on flip it explicitly.
        assert!(!p.ai_enabled);
    }
}
