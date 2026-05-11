//! Project classification (Layer 3, pure logic).
//!
//! `BookKind` is a finer-grained classification than `BookMode`. Where
//! `BookMode` answers "is this fiction or non-fiction?", `BookKind`
//! answers "*what kind* of fiction / non-fiction is this?" — which
//! drives the orchestrator's workflow router (per-genre prompts,
//! critic axes, polish-stack ordering, rubric weights).
//!
//! This enum lives in `booksforge-domain` so the project schema, the
//! genre packs (`booksforge-genre-packs`), and the orchestrator can all
//! refer to the same type without circular dependencies.
//!
//! Migration policy: existing projects (schema version 1, no
//! `book_kind` field in manifest) get `None`. The desktop app surfaces
//! a one-time onboarding overlay when it sees `None` and asks the user
//! to pick. New projects pick a `BookKind` in the wizard's first step.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum BookKind {
    /// Voice-driven prose; sentence-craft over plot velocity. Polish
    /// stack runs `voice → metaphor → dialogue → scene_tension`.
    LiteraryFiction,
    /// Cozy fantasy / thriller / romance / mystery / YA. Pacing first.
    /// Polish stack runs `scene_tension → dialogue → metaphor → voice`.
    GenreFiction,
    /// Strategy / popular science / long-form essay / business. Argument
    /// + evidence weighed highest. NEVER fabricates stats / dates / quotes.
    NonFiction,
    /// Memoir — prose-craft + interiority weighed like literary, but
    /// with non-fiction's no-fabrication rule.
    Memoir,
    /// Children's picture / chapter book — different word counts, layout,
    /// reading-level constraints. Out of MVP scope; the kind is recognised
    /// so the wizard can show "coming soon" and route the user back.
    ChildrensBook,
}

impl BookKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LiteraryFiction => "literary-fiction",
            Self::GenreFiction => "genre-fiction",
            Self::NonFiction => "non-fiction",
            Self::Memoir => "memoir",
            Self::ChildrensBook => "childrens-book",
        }
    }

    /// Human-readable display name (UI tooltips / settings labels).
    pub fn display_name(self) -> &'static str {
        match self {
            Self::LiteraryFiction => "Literary Fiction",
            Self::GenreFiction => "Genre Fiction",
            Self::NonFiction => "Non-Fiction",
            Self::Memoir => "Memoir",
            Self::ChildrensBook => "Children's Book",
        }
    }

    /// Forgiving parse — accepts kebab, snake, and the bare name.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "literary-fiction" | "literary_fiction" | "literary" => Some(Self::LiteraryFiction),
            "genre-fiction" | "genre_fiction" | "genre" => Some(Self::GenreFiction),
            "non-fiction" | "non_fiction" | "nonfiction" => Some(Self::NonFiction),
            "memoir" => Some(Self::Memoir),
            "childrens-book" | "childrens_book" | "childrens" | "children" => {
                Some(Self::ChildrensBook)
            }
            _ => None,
        }
    }

    /// Whether the kind is supported by the full pipeline today.
    /// `ChildrensBook` returns `false` (out of MVP scope).
    pub fn is_supported_in_mvp(self) -> bool {
        !matches!(self, Self::ChildrensBook)
    }

    /// Map a `BookMode` to the most plausible `BookKind` — used to seed
    /// the onboarding overlay's default for projects migrated from
    /// schema-version-1 manifests.
    pub fn from_mode_default(mode: crate::BookMode) -> Self {
        use crate::BookMode;
        match mode {
            BookMode::Fiction => Self::LiteraryFiction, // user can change to GenreFiction
            // Both NonFiction and Academic map to NonFiction. Memoir is
            // a separate BookKind (handled below).
            BookMode::NonFiction | BookMode::Academic => Self::NonFiction,
            BookMode::Memoir => Self::Memoir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BookMode;

    #[test]
    fn roundtrip_all_variants() {
        for k in [
            BookKind::LiteraryFiction,
            BookKind::GenreFiction,
            BookKind::NonFiction,
            BookKind::Memoir,
            BookKind::ChildrensBook,
        ] {
            assert_eq!(BookKind::from_str(k.as_str()), Some(k));
        }
    }

    #[test]
    fn forgiving_parse_aliases() {
        assert_eq!(
            BookKind::from_str("literary"),
            Some(BookKind::LiteraryFiction)
        );
        assert_eq!(BookKind::from_str("genre"), Some(BookKind::GenreFiction));
        assert_eq!(BookKind::from_str("nonfiction"), Some(BookKind::NonFiction));
        assert_eq!(BookKind::from_str("Memoir"), Some(BookKind::Memoir)); // case-insensitive
        assert_eq!(
            BookKind::from_str("childrens_book"),
            Some(BookKind::ChildrensBook)
        );
        assert_eq!(BookKind::from_str("totally-made-up"), None);
    }

    #[test]
    fn mvp_support_excludes_childrens() {
        assert!(BookKind::LiteraryFiction.is_supported_in_mvp());
        assert!(BookKind::GenreFiction.is_supported_in_mvp());
        assert!(BookKind::NonFiction.is_supported_in_mvp());
        assert!(BookKind::Memoir.is_supported_in_mvp());
        assert!(!BookKind::ChildrensBook.is_supported_in_mvp());
    }

    #[test]
    fn from_mode_maps_correctly() {
        assert_eq!(
            BookKind::from_mode_default(BookMode::Fiction),
            BookKind::LiteraryFiction
        );
        assert_eq!(
            BookKind::from_mode_default(BookMode::NonFiction),
            BookKind::NonFiction
        );
        assert_eq!(
            BookKind::from_mode_default(BookMode::Memoir),
            BookKind::Memoir
        );
        assert_eq!(
            BookKind::from_mode_default(BookMode::Academic),
            BookKind::NonFiction
        );
    }

    #[test]
    fn display_names_are_human_friendly() {
        assert_eq!(BookKind::LiteraryFiction.display_name(), "Literary Fiction");
        assert_eq!(BookKind::ChildrensBook.display_name(), "Children's Book");
    }
}
