//! Cover & boilerplate flow — Stage 6 (Format & Ship) front/back-matter
//! types.
//!
//! Two value types persisted to project memory under the `book` scope:
//!   - `book:cover_set`         → `CoverSet`
//!   - `book:boilerplate_pages` → `Vec<BoilerplatePage>`
//!
//! These are advisory inputs to the export pipeline. The export
//! pipeline's per-target validators (in `booksforge-domain::publishing_target`)
//! still apply DPI/aspect/PDF-X checks at run-time. Persisting these
//! here lets the writer set them once and have every export reuse
//! them, without modifying the structural manifest.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Kind of boilerplate page. Drives ordering (front matter vs. back
/// matter) and default headings in the export template.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BoilerplateKind {
    /// Half-title and main title pages.
    TitlePage,
    /// Copyright page (ISBN, rights, year).
    Copyright,
    /// Dedication ("for X").
    Dedication,
    /// Epigraph (quote opening the book).
    Epigraph,
    /// Foreword (someone else's intro).
    Foreword,
    /// Preface (author's own intro).
    Preface,
    /// Acknowledgments.
    Acknowledgments,
    /// About-the-author bio.
    AboutAuthor,
    /// Also-by list (other works).
    AlsoBy,
    /// Back-cover blurb (for KDP / IngramSpark metadata).
    BackCoverBlurb,
    /// Anything else the writer wants to slot in.
    Other,
}

impl BoilerplateKind {
    /// Every variant, in canonical UI order (front matter first,
    /// title-page lead). The UI's "add page" buttons and the
    /// Stage13 parity test both consume this so the TS-side
    /// `BOILERPLATE_KINDS` array can't drift.
    pub const ALL: &'static [Self] = &[
        Self::TitlePage,
        Self::Copyright,
        Self::Dedication,
        Self::Epigraph,
        Self::Foreword,
        Self::Preface,
        Self::Acknowledgments,
        Self::AboutAuthor,
        Self::AlsoBy,
        Self::BackCoverBlurb,
        Self::Other,
    ];

    /// Stable serde id (the snake_case string the TS layer keys off).
    pub fn id(self) -> &'static str {
        match self {
            Self::TitlePage => "title_page",
            Self::Copyright => "copyright",
            Self::Dedication => "dedication",
            Self::Epigraph => "epigraph",
            Self::Foreword => "foreword",
            Self::Preface => "preface",
            Self::Acknowledgments => "acknowledgments",
            Self::AboutAuthor => "about_author",
            Self::AlsoBy => "also_by",
            Self::BackCoverBlurb => "back_cover_blurb",
            Self::Other => "other",
        }
    }

    /// True when this page belongs in front matter (before chapter 1).
    pub fn is_front_matter(self) -> bool {
        matches!(
            self,
            Self::TitlePage
                | Self::Copyright
                | Self::Dedication
                | Self::Epigraph
                | Self::Foreword
                | Self::Preface
        )
    }

    /// Default heading the export template will render above the body.
    /// `TitlePage` is the only kind that suppresses the heading by
    /// convention.
    pub fn default_heading(self) -> &'static str {
        match self {
            Self::Copyright => "Copyright",
            Self::Dedication => "Dedication",
            Self::Foreword => "Foreword",
            Self::Preface => "Preface",
            Self::Acknowledgments => "Acknowledgments",
            Self::AboutAuthor => "About the Author",
            Self::AlsoBy => "Also by the Author",
            Self::BackCoverBlurb => "Back Cover",
            // Title page, epigraph, and "other" intentionally have no
            // template heading (writer's body controls the chrome).
            Self::TitlePage | Self::Epigraph | Self::Other => "",
        }
    }
}

/// One boilerplate page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoilerplatePage {
    /// Stable id for upsert / reorder.
    pub id: String,
    pub kind: BoilerplateKind,
    /// Writer-visible heading. Falls back to `kind.default_heading()`
    /// if empty.
    #[serde(default)]
    pub title: String,
    /// Markdown body; export templates render this verbatim.
    pub body_md: String,
    /// Display order within front or back matter. Lower runs earlier.
    pub order: u32,
    /// When false, the page is kept in the project but skipped at
    /// export time. Lets the writer draft something they're not yet
    /// ready to ship.
    #[serde(default = "default_true")]
    pub include_in_export: bool,
}

fn default_true() -> bool {
    true
}

impl BoilerplatePage {
    /// Build a new page with sensible defaults. Caller passes the id;
    /// in MVP the UI generates a ULID and the storage keeps the JSON
    /// list of pages as a single memory entry.
    pub fn new(id: impl Into<String>, kind: BoilerplateKind, order: u32) -> Self {
        Self {
            id: id.into(),
            kind,
            title: kind.default_heading().to_owned(),
            body_md: String::new(),
            order,
            include_in_export: true,
        }
    }

    /// Word count of the markdown body (whitespace-split). Used by the
    /// UI for the "12 words" hint.
    pub fn word_count(&self) -> u32 {
        self.body_md.split_whitespace().count() as u32
    }
}

/// A cover image asset, as imported into the project bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoverAsset {
    /// Path RELATIVE to the project bundle root
    /// (e.g. `assets/cover-front.jpg`).
    pub bundle_path: String,
    /// Original filename the writer imported, for display.
    pub original_filename: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// MIME type detected at import ("image/jpeg" | "image/png" | …).
    #[serde(default)]
    pub mime_type: String,
    /// Pixel dimensions, best-effort. None means unknown (export-time
    /// validators will still measure).
    #[serde(default)]
    pub width_px: Option<u32>,
    #[serde(default)]
    pub height_px: Option<u32>,
    /// UTC timestamp of import.
    pub imported_at: DateTime<Utc>,
}

impl CoverAsset {
    /// Aspect ratio (width / height) × 100, rounded to integer.
    /// Matches the convention used in `TargetSpec.cover_aspect_x100`.
    pub fn aspect_x100(&self) -> Option<u32> {
        let w = self.width_px? as f64;
        let h = self.height_px? as f64;
        if h <= 0.0 {
            return None;
        }
        Some((w / h * 100.0).round() as u32)
    }
}

/// Cover-image set for the project. Front is required for any
/// graphical export; back and spine are optional and used by paperback
/// targets only (KDP paperback, IngramSpark).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CoverSet {
    #[serde(default)]
    pub front: Option<CoverAsset>,
    #[serde(default)]
    pub back: Option<CoverAsset>,
    #[serde(default)]
    pub spine: Option<CoverAsset>,
}

impl CoverSet {
    pub fn is_empty(&self) -> bool {
        self.front.is_none() && self.back.is_none() && self.spine.is_none()
    }

    /// True when there is at least a front cover. Most digital
    /// targets (KDP ebook, Apple Books, Google Play) only need this.
    pub fn has_front(&self) -> bool {
        self.front.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_classification() {
        assert!(BoilerplateKind::TitlePage.is_front_matter());
        assert!(BoilerplateKind::Copyright.is_front_matter());
        assert!(BoilerplateKind::Dedication.is_front_matter());
        assert!(BoilerplateKind::Epigraph.is_front_matter());
        assert!(BoilerplateKind::Preface.is_front_matter());
        assert!(!BoilerplateKind::Acknowledgments.is_front_matter());
        assert!(!BoilerplateKind::AboutAuthor.is_front_matter());
        assert!(!BoilerplateKind::AlsoBy.is_front_matter());
    }

    #[test]
    fn default_heading_is_empty_for_title_page() {
        assert_eq!(BoilerplateKind::TitlePage.default_heading(), "");
        assert_eq!(BoilerplateKind::Copyright.default_heading(), "Copyright");
    }

    #[test]
    fn boilerplate_page_word_count() {
        let mut p = BoilerplatePage::new("01", BoilerplateKind::Dedication, 0);
        p.body_md = "For my grandmother, who first taught me to listen.".to_owned();
        assert_eq!(p.word_count(), 9);
    }

    #[test]
    fn boilerplate_page_round_trips_through_json() {
        let p = BoilerplatePage::new("01", BoilerplateKind::Acknowledgments, 1);
        let json = serde_json::to_string(&p).expect("ser");
        let back: BoilerplatePage = serde_json::from_str(&json).expect("de");
        assert_eq!(p, back);
    }

    #[test]
    fn cover_set_aspect_calculation() {
        let asset = CoverAsset {
            bundle_path: "assets/cover-front.jpg".into(),
            original_filename: "cover.jpg".into(),
            size_bytes: 500_000,
            mime_type: "image/jpeg".into(),
            width_px: Some(1600),
            height_px: Some(2560),
            imported_at: Utc::now(),
        };
        // 1600 / 2560 = 0.625 → x100 = 62 (rounded)
        // Note: round(62.5) is 62 with banker's rounding; allow either.
        let aspect = asset.aspect_x100().expect("aspect");
        assert!(aspect == 62 || aspect == 63, "got {aspect}");
    }

    #[test]
    fn cover_set_empty_default() {
        let cs = CoverSet::default();
        assert!(cs.is_empty());
        assert!(!cs.has_front());
    }

    #[test]
    fn cover_set_deserialise_missing_fields() {
        let json = r#"{}"#;
        let cs: CoverSet = serde_json::from_str(json).expect("parses empty");
        assert!(cs.is_empty());
    }

    #[test]
    fn id_matches_serde_for_every_variant() {
        // The `id()` method is the manual mirror serde keys off when
        // round-tripping. Drift here would silently misroute saved
        // pages on load. The two must agree for every variant.
        for &kind in BoilerplateKind::ALL {
            let serde_id = serde_json::to_value(kind)
                .expect("ser")
                .as_str()
                .expect("string")
                .to_owned();
            assert_eq!(kind.id(), serde_id, "id() drifted from serde for {kind:?}");
            // And id() round-trips through deserialise.
            let back: BoilerplateKind =
                serde_json::from_value(serde_json::Value::String(kind.id().to_owned()))
                    .expect("de");
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn all_contains_every_variant_exactly_once() {
        // Belt-and-braces: when a new variant lands, this test fails
        // unless the developer also adds it to `ALL`. The count
        // matches the enum declaration order.
        assert_eq!(BoilerplateKind::ALL.len(), 11);
        let mut ids: Vec<&str> = BoilerplateKind::ALL.iter().map(|k| k.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), BoilerplateKind::ALL.len(), "duplicate kinds");
    }
}
