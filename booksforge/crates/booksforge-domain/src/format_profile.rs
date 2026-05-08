//! Format profile — genre + sub-genre book typography.
//!
//! ## Two-level design (BACKLOG §H8.2)
//!
//! BACKLOG §H8.1 shipped seven generic-fiction / non-fiction profiles
//! covering the trade-paperback / literary / YA / memoir / academic /
//! practical-non-fiction starter set.  This module extends that with a
//! **Genre × Sub-genre** taxonomy:
//!
//!   - **Genre** — top-level pick: Romance, Comedy, Non-fiction,
//!     Thriller, Horror (plus a `Generic` umbrella for the original
//!     seven profiles).
//!   - **Sub-genre** — second-level pick that varies *typography*
//!     within a genre: e.g. Romance has Contemporary / Historical /
//!     Paranormal / Suspense.
//!
//! Each sub-genre is its own `FormatProfile` variant.  The original
//! seven profiles remain under the `Generic` genre so existing
//! callers / fixtures don't break.
//!
//! ## Per-sub-genre knobs
//!
//! Every sub-genre carries its own:
//!
//!   - **Trim size + page geometry**
//!   - **Body / heading font** — drawn from a curated **Google Font
//!     bundle** chosen for book typography (EB Garamond, Crimson Pro,
//!     Lora, Source Serif 4, Vollkorn, Playfair Display, Cormorant
//!     Garamond, Inter, Source Sans 3, JetBrains Mono).
//!   - **Body size + line-height**
//!   - **Drop-cap policy**
//!   - **Scene-break style** — Unicode glyph fallback + a hand-curated
//!     **inline SVG ornament** (option *a* from BACKLOG §H8.2).
//!   - **Paragraph indent**
//!   - **Pandoc `documentclass` + `classoption`**
//!   - **Front-matter components**
//!
//! The implementation uses a single `ProfileSpec` lookup table so
//! adding a new sub-genre is one new const + one match arm.
//!
//! ## SVG ornaments
//!
//! Hand-curated, deliberately minimal: a sub-100-byte inline SVG per
//! sub-genre that renders well at small sizes both on screen (EPUB)
//! and in print (PDF when xelatex is configured to render SVG).  The
//! SVGs use `currentColor` for stroke so they pick up the surrounding
//! text colour and work in both light- and dark-mode readers.

use serde::{Deserialize, Serialize};

// ── Genre ───────────────────────────────────────────────────────────────────

/// Top-level book genre.  The UI presents this as the first picker in
/// the two-level Genre → Sub-genre cascade.  `Generic` covers the
/// original seven profiles from H8.1 (`fiction_trade_*`, `academic`,
/// `non_fiction_*`) so they're still pickable without forcing every
/// user into the new taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Genre {
    /// Original H8.1 generic profiles — trade fiction, literary, YA,
    /// practical / memoir non-fiction, academic.  Kept so existing
    /// fixtures and callers continue to work unchanged.
    Generic,
    Romance,
    Comedy,
    NonFiction,
    Thriller,
    Horror,
}

impl Genre {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic    => "generic",
            Self::Romance    => "romance",
            Self::Comedy     => "comedy",
            Self::NonFiction => "non_fiction",
            Self::Thriller   => "thriller",
            Self::Horror     => "horror",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "generic"     => Some(Self::Generic),
            "romance"     => Some(Self::Romance),
            "comedy"      => Some(Self::Comedy),
            "non_fiction" => Some(Self::NonFiction),
            "thriller"    => Some(Self::Thriller),
            "horror"      => Some(Self::Horror),
            _              => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Generic    => "Generic",
            Self::Romance    => "Romance",
            Self::Comedy     => "Comedy",
            Self::NonFiction => "Non-fiction",
            Self::Thriller   => "Thriller",
            Self::Horror     => "Horror",
        }
    }

    /// Every supported genre, in UI display order.
    pub fn all() -> &'static [Genre] {
        &[
            Self::Romance,
            Self::Comedy,
            Self::NonFiction,
            Self::Thriller,
            Self::Horror,
            Self::Generic,
        ]
    }

    /// Sub-genre profiles that belong to this genre, in UI order.
    pub fn sub_genres(self) -> &'static [FormatProfile] {
        match self {
            Self::Generic => &[
                FormatProfile::FictionTradeMass,
                FormatProfile::FictionTradeStandard,
                FormatProfile::FictionLiterary,
                FormatProfile::FictionYoungAdult,
                FormatProfile::NonFictionPractical,
                FormatProfile::NonFictionMemoir,
                FormatProfile::Academic,
            ],
            Self::Romance => &[
                FormatProfile::RomanceContemporary,
                FormatProfile::RomanceHistorical,
                FormatProfile::RomanceParanormal,
                FormatProfile::RomanceSuspense,
            ],
            Self::Comedy => &[
                FormatProfile::ComedyRomCom,
                FormatProfile::ComedySatire,
                FormatProfile::ComedyLiteraryHumor,
                FormatProfile::ComedyCozy,
            ],
            Self::NonFiction => &[
                FormatProfile::NonFictionNarrative,
                FormatProfile::NonFictionCookbook,
                FormatProfile::NonFictionWorkbook,
                FormatProfile::NonFictionSelfHelp,
            ],
            Self::Thriller => &[
                FormatProfile::ThrillerPsychological,
                FormatProfile::ThrillerCrime,
                FormatProfile::ThrillerEspionage,
                FormatProfile::ThrillerAction,
            ],
            Self::Horror => &[
                FormatProfile::HorrorGothic,
                FormatProfile::HorrorCosmic,
                FormatProfile::HorrorSlasher,
                FormatProfile::HorrorSupernatural,
            ],
        }
    }
}

// ── FormatProfile ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatProfile {
    // ── Generic (H8.1, kept for back-compat) ──
    FictionTradeMass,
    FictionTradeStandard,
    FictionLiterary,
    FictionYoungAdult,
    NonFictionPractical,
    NonFictionMemoir,
    Academic,

    // ── Romance (H8.2) ──
    RomanceContemporary,
    RomanceHistorical,
    RomanceParanormal,
    RomanceSuspense,

    // ── Comedy (H8.2) ──
    ComedyRomCom,
    ComedySatire,
    ComedyLiteraryHumor,
    ComedyCozy,

    // ── Non-fiction (H8.2) ──
    NonFictionNarrative,
    NonFictionCookbook,
    NonFictionWorkbook,
    NonFictionSelfHelp,

    // ── Thriller (H8.2) ──
    ThrillerPsychological,
    ThrillerCrime,
    ThrillerEspionage,
    ThrillerAction,

    // ── Horror (H8.2) ──
    HorrorGothic,
    HorrorCosmic,
    HorrorSlasher,
    HorrorSupernatural,
}

/// Inline-table view of every per-profile knob.  Adding a sub-genre is
/// one new `ProfileSpec` const + one new match arm in [`spec`].
struct ProfileSpec {
    as_str:                &'static str,
    label:                 &'static str,
    blurb:                 &'static str,
    genre:                 Genre,
    trim:                  (&'static str, &'static str),
    chapter_starts_recto:  bool,
    drop_cap:              bool,
    /// Body font *family* — short Google Font name (e.g. "EB Garamond").
    /// The CSS factory in `booksforge-export-epub` builds the full
    /// font stack from this.
    google_body:           &'static str,
    google_heading:        &'static str,
    body_em:               &'static str,
    body_size_pt:          &'static str,
    line_height:           &'static str,
    scene_break_glyph:     &'static str,
    /// Inline SVG ornament — drawn at scene breaks and (optionally)
    /// chapter-flourish positions.  Uses `currentColor` so it inherits
    /// the surrounding text colour for light/dark theming.
    ornament_svg:          &'static str,
    paragraph_indent_em:   &'static str,
    pandoc_documentclass:  &'static str,
    pdf_toc:               bool,
    front_matter:          &'static [FrontMatterPage],
}

// Curated Google Font bundle for book typography.  Each constant is
// the family name as it appears in Google Fonts; the CSS factory wraps
// it into the appropriate `@font-face` and falls back to system fonts.
mod fonts {
    pub(super) const EB_GARAMOND:        &str = "EB Garamond";
    pub(super) const CRIMSON_PRO:        &str = "Crimson Pro";
    pub(super) const LORA:               &str = "Lora";
    pub(super) const SOURCE_SERIF_4:     &str = "Source Serif 4";
    pub(super) const VOLLKORN:           &str = "Vollkorn";
    pub(super) const PLAYFAIR_DISPLAY:   &str = "Playfair Display";
    pub(super) const CORMORANT_GARAMOND: &str = "Cormorant Garamond";
    pub(super) const INTER:              &str = "Inter";
    pub(super) const SOURCE_SANS_3:      &str = "Source Sans 3";
}

/// Hand-curated minimal SVG ornaments.  Each is an inline SVG fragment
/// (no XML decl, no doctype) that the CSS factory wraps in a data URI
/// for `hr::before { background-image: url(...) }`.  Sized 80×16
/// viewBox so they look good at scene-break sizes.
mod ornaments {
    /// Three-asterism dot row — clean trade-paperback default.
    pub(super) const ASTERISM_DOTS: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><circle cx='28' cy='8' r='1.4' fill='currentColor'/><circle cx='40' cy='8' r='1.4' fill='currentColor'/><circle cx='52' cy='8' r='1.4' fill='currentColor'/></svg>"#;

    /// Romance — flourish heart with side curls.
    pub(super) const ROMANCE_FLOURISH: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M10 8 Q 20 2, 32 8 M48 8 Q 60 2, 70 8' fill='none' stroke='currentColor' stroke-width='1'/><path d='M40 11 q -3 -3 0 -5 q 3 -2 0 5z' fill='currentColor'/></svg>"#;

    /// Romance Regency — Georgian-era horizontal cartouche.
    pub(super) const ROMANCE_REGENCY: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M5 8 Q 12 2, 24 8 T 40 8 T 56 8 T 76 8' fill='none' stroke='currentColor' stroke-width='1'/><circle cx='40' cy='8' r='2.2' fill='none' stroke='currentColor' stroke-width='1'/></svg>"#;

    /// Romance Paranormal — moon + stars.
    pub(super) const ROMANCE_PARANORMAL: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M40 4 a 4 4 0 1 0 0 8 a 3 3 0 1 1 0 -8z' fill='currentColor'/><circle cx='28' cy='8' r='0.8' fill='currentColor'/><circle cx='52' cy='8' r='0.8' fill='currentColor'/></svg>"#;

    /// Comedy — gentle wavy line.
    pub(super) const COMEDY_WAVE: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M10 8 Q 18 2, 26 8 T 42 8 T 58 8 T 74 8' fill='none' stroke='currentColor' stroke-width='1.2'/></svg>"#;

    /// Cookbook — three small squares (plates).
    pub(super) const COOKBOOK_PLATES: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><rect x='28' y='5' width='6' height='6' fill='none' stroke='currentColor'/><rect x='37' y='5' width='6' height='6' fill='none' stroke='currentColor'/><rect x='46' y='5' width='6' height='6' fill='none' stroke='currentColor'/></svg>"#;

    /// Workbook — three checkboxes.
    pub(super) const WORKBOOK_CHECKS: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><rect x='28' y='4' width='8' height='8' fill='none' stroke='currentColor'/><rect x='38' y='4' width='8' height='8' fill='none' stroke='currentColor'/><rect x='48' y='4' width='8' height='8' fill='none' stroke='currentColor'/></svg>"#;

    /// Self-help — three forward arrows.
    pub(super) const SELFHELP_ARROWS: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M28 8 L34 8 M32 5 L34 8 L32 11 M40 8 L46 8 M44 5 L46 8 L44 11 M52 8 L58 8 M56 5 L58 8 L56 11' fill='none' stroke='currentColor' stroke-width='1.2'/></svg>"#;

    /// Thriller — solitary vertical bar (urgent break).
    pub(super) const THRILLER_BAR: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><rect x='39' y='2' width='2' height='12' fill='currentColor'/></svg>"#;

    /// Thriller Crime — three slashes.
    pub(super) const THRILLER_CRIME: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M30 12 L34 4 M40 12 L44 4 M50 12 L54 4' stroke='currentColor' stroke-width='1.4'/></svg>"#;

    /// Espionage — small diamond cluster.
    pub(super) const THRILLER_ESPIONAGE: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M32 8 l 4 -4 l 4 4 l -4 4 z M44 8 l 4 -4 l 4 4 l -4 4 z' fill='currentColor'/></svg>"#;

    /// Horror Gothic — gothic cross.
    pub(super) const HORROR_GOTHIC: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><rect x='39' y='2' width='2' height='12' fill='currentColor'/><rect x='34' y='6' width='12' height='2' fill='currentColor'/></svg>"#;

    /// Horror Cosmic — descending triangles.
    pub(super) const HORROR_COSMIC: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M28 4 l 4 6 l 4 -6 z M40 4 l 4 6 l 4 -6 z M52 4 l 4 6 l 4 -6 z' fill='currentColor'/></svg>"#;

    /// Horror Slasher — single jagged stroke.
    pub(super) const HORROR_SLASHER: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M28 12 L34 6 L36 9 L42 4 L44 8 L52 5' fill='none' stroke='currentColor' stroke-width='1.4'/></svg>"#;

    /// Horror Supernatural — crescent moon flanked by dots.
    pub(super) const HORROR_SUPERNATURAL: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M40 3 a 5 5 0 1 0 0 10 a 4 4 0 1 1 0 -10z' fill='none' stroke='currentColor' stroke-width='1'/><circle cx='28' cy='8' r='0.8' fill='currentColor'/><circle cx='52' cy='8' r='0.8' fill='currentColor'/></svg>"#;

    /// Literary — single fleuron.
    pub(super) const LITERARY_FLEURON: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><path d='M40 3 q -4 4 0 6 q 4 -2 0 -6 M40 13 q -4 -4 0 -6 q 4 2 0 6' fill='currentColor'/></svg>"#;

    /// Generic three-dot rule — used for academic / practical.
    pub(super) const GENERIC_BULLETS: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 80 16'><circle cx='32' cy='8' r='1.6' fill='currentColor'/><circle cx='40' cy='8' r='1.6' fill='currentColor'/><circle cx='48' cy='8' r='1.6' fill='currentColor'/></svg>"#;

    /// Empty — academic profile suppresses ornaments.
    pub(super) const NONE: &str = "";
}

/// Standard four-page front-matter (title / copyright / dedication /
/// epigraph) — most fiction sub-genres use this.
const FM_FICTION_FULL: &[FrontMatterPage] = &[
    FrontMatterPage::TitlePage, FrontMatterPage::Copyright,
    FrontMatterPage::Dedication, FrontMatterPage::Epigraph,
];

/// Title + copyright only — mass-market lean.
const FM_FICTION_LEAN: &[FrontMatterPage] = &[
    FrontMatterPage::TitlePage, FrontMatterPage::Copyright,
];

/// Title + copyright + ToC — non-fiction with chapters needs ToC.
const FM_NON_FICTION_TOC: &[FrontMatterPage] = &[
    FrontMatterPage::TitlePage, FrontMatterPage::Copyright, FrontMatterPage::TableOfContents,
];

/// Title + copyright + dedication — YA convention.
const FM_FICTION_DED: &[FrontMatterPage] = &[
    FrontMatterPage::TitlePage, FrontMatterPage::Copyright, FrontMatterPage::Dedication,
];

const fn spec(p: FormatProfile) -> &'static ProfileSpec {
    match p {
        FormatProfile::FictionTradeMass     => &SPEC_FICTION_TRADE_MASS,
        FormatProfile::FictionTradeStandard => &SPEC_FICTION_TRADE_STANDARD,
        FormatProfile::FictionLiterary      => &SPEC_FICTION_LITERARY,
        FormatProfile::FictionYoungAdult    => &SPEC_FICTION_YA,
        FormatProfile::NonFictionPractical  => &SPEC_NF_PRACTICAL,
        FormatProfile::NonFictionMemoir     => &SPEC_NF_MEMOIR,
        FormatProfile::Academic             => &SPEC_ACADEMIC,

        FormatProfile::RomanceContemporary  => &SPEC_ROMANCE_CONTEMPORARY,
        FormatProfile::RomanceHistorical    => &SPEC_ROMANCE_HISTORICAL,
        FormatProfile::RomanceParanormal    => &SPEC_ROMANCE_PARANORMAL,
        FormatProfile::RomanceSuspense      => &SPEC_ROMANCE_SUSPENSE,

        FormatProfile::ComedyRomCom         => &SPEC_COMEDY_ROMCOM,
        FormatProfile::ComedySatire         => &SPEC_COMEDY_SATIRE,
        FormatProfile::ComedyLiteraryHumor  => &SPEC_COMEDY_LITERARY,
        FormatProfile::ComedyCozy           => &SPEC_COMEDY_COZY,

        FormatProfile::NonFictionNarrative  => &SPEC_NF_NARRATIVE,
        FormatProfile::NonFictionCookbook   => &SPEC_NF_COOKBOOK,
        FormatProfile::NonFictionWorkbook   => &SPEC_NF_WORKBOOK,
        FormatProfile::NonFictionSelfHelp   => &SPEC_NF_SELFHELP,

        FormatProfile::ThrillerPsychological => &SPEC_THRILLER_PSYCH,
        FormatProfile::ThrillerCrime        => &SPEC_THRILLER_CRIME,
        FormatProfile::ThrillerEspionage    => &SPEC_THRILLER_ESPIONAGE,
        FormatProfile::ThrillerAction       => &SPEC_THRILLER_ACTION,

        FormatProfile::HorrorGothic         => &SPEC_HORROR_GOTHIC,
        FormatProfile::HorrorCosmic         => &SPEC_HORROR_COSMIC,
        FormatProfile::HorrorSlasher        => &SPEC_HORROR_SLASHER,
        FormatProfile::HorrorSupernatural   => &SPEC_HORROR_SUPERNATURAL,
    }
}

// ── Generic (H8.1) ──────────────────────────────────────────────────────────

const SPEC_FICTION_TRADE_MASS: ProfileSpec = ProfileSpec {
    as_str: "fiction_trade_mass",
    label:  "Fiction — Mass-Market (5×8)",
    blurb:  "Mass-market thrillers, romance, mystery.  Tight type, no drop caps.",
    genre:  Genre::Generic,
    trim:   ("5in", "8in"),
    chapter_starts_recto:  true,
    drop_cap:              false,
    google_body:           fonts::CRIMSON_PRO,
    google_heading:        fonts::INTER,
    body_em:               "1em",
    body_size_pt:          "10.5pt",
    line_height:           "1.4",
    scene_break_glyph:     "* * *",
    ornament_svg:          ornaments::ASTERISM_DOTS,
    paragraph_indent_em:   "1.2em",
    pandoc_documentclass:  "memoir",
    pdf_toc:               false,
    front_matter:          FM_FICTION_LEAN,
};

const SPEC_FICTION_TRADE_STANDARD: ProfileSpec = ProfileSpec {
    as_str: "fiction_trade_standard",
    label:  "Fiction — Trade Paperback (6×9)",
    blurb:  "Most adult fiction.  Trade paperback typography with drop caps.",
    genre:  Genre::Generic,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::EB_GARAMOND,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "* * *",
    ornament_svg:         ornaments::ASTERISM_DOTS,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_LEAN,
};

const SPEC_FICTION_LITERARY: ProfileSpec = ProfileSpec {
    as_str: "fiction_literary",
    label:  "Fiction — Literary (6×9)",
    blurb:  "Literary fiction, story collections.  Generous spacing, ornament scene breaks.",
    genre:  Genre::Generic,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::EB_GARAMOND,
    google_heading:       fonts::EB_GARAMOND,
    body_em:              "1.02em",
    body_size_pt:         "11.5pt",
    line_height:          "1.55",
    scene_break_glyph:    "❦",
    ornament_svg:         ornaments::LITERARY_FLEURON,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_FICTION_YA: ProfileSpec = ProfileSpec {
    as_str: "fiction_young_adult",
    label:  "Young Adult (5.5×8.5)",
    blurb:  "YA + upper middle-grade.  Larger body type, sans-serif headings.",
    genre:  Genre::Generic,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::LORA,
    google_heading:       fonts::SOURCE_SANS_3,
    body_em:              "1.05em",
    body_size_pt:         "12pt",
    line_height:          "1.6",
    scene_break_glyph:    "·   ·   ·",
    ornament_svg:         ornaments::GENERIC_BULLETS,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "book",
    pdf_toc:              false,
    front_matter:         FM_FICTION_DED,
};

const SPEC_NF_PRACTICAL: ProfileSpec = ProfileSpec {
    as_str: "non_fiction_practical",
    label:  "Non-Fiction — Practical (6×9)",
    blurb:  "How-to, business, self-help.  Sans-serif heads, callouts, smaller body.",
    genre:  Genre::Generic,
    trim:   ("6in", "9in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::SOURCE_SERIF_4,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "10.5pt",
    line_height:          "1.45",
    scene_break_glyph:    "• • •",
    ornament_svg:         ornaments::GENERIC_BULLETS,
    paragraph_indent_em:  "0",
    pandoc_documentclass: "book",
    pdf_toc:              true,
    front_matter:         FM_NON_FICTION_TOC,
};

const SPEC_NF_MEMOIR: ProfileSpec = ProfileSpec {
    as_str: "non_fiction_memoir",
    label:  "Non-Fiction — Memoir (6×9)",
    blurb:  "Memoir, biography, narrative non-fiction.  Footnote support, photo-plate.",
    genre:  Genre::Generic,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::EB_GARAMOND,
    google_heading:       fonts::EB_GARAMOND,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "❦",
    ornament_svg:         ornaments::LITERARY_FLEURON,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_ACADEMIC: ProfileSpec = ProfileSpec {
    as_str: "academic",
    label:  "Academic (6×9)",
    blurb:  "Scholarly + textbooks.  Numbered chapters, footnotes, citations.",
    genre:  Genre::Generic,
    trim:   ("6in", "9in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::SOURCE_SERIF_4,
    google_heading:       fonts::INTER,
    body_em:              "0.95em",
    body_size_pt:         "10.5pt",
    line_height:          "1.4",
    scene_break_glyph:    "",
    ornament_svg:         ornaments::NONE,
    paragraph_indent_em:  "1.5em",
    pandoc_documentclass: "book",
    pdf_toc:              true,
    front_matter:         FM_NON_FICTION_TOC,
};

// ── Romance (H8.2) ──────────────────────────────────────────────────────────

const SPEC_ROMANCE_CONTEMPORARY: ProfileSpec = ProfileSpec {
    as_str: "romance_contemporary",
    label:  "Romance — Contemporary",
    blurb:  "Modern romance with warm Lora body and Playfair Display chapter heads.",
    genre:  Genre::Romance,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::LORA,
    google_heading:       fonts::PLAYFAIR_DISPLAY,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "♥",
    ornament_svg:         ornaments::ROMANCE_FLOURISH,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_ROMANCE_HISTORICAL: ProfileSpec = ProfileSpec {
    as_str: "romance_historical",
    label:  "Romance — Historical / Regency",
    blurb:  "Period typography — Cormorant Garamond throughout, ornate scene-break flourish.",
    genre:  Genre::Romance,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::CORMORANT_GARAMOND,
    google_heading:       fonts::CORMORANT_GARAMOND,
    body_em:              "1.02em",
    body_size_pt:         "11.5pt",
    line_height:          "1.55",
    scene_break_glyph:    "✦",
    ornament_svg:         ornaments::ROMANCE_REGENCY,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_ROMANCE_PARANORMAL: ProfileSpec = ProfileSpec {
    as_str: "romance_paranormal",
    label:  "Romance — Paranormal",
    blurb:  "Crimson Pro body with Cormorant heads — moonlit, slightly otherworldly.",
    genre:  Genre::Romance,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::CRIMSON_PRO,
    google_heading:       fonts::CORMORANT_GARAMOND,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "☾",
    ornament_svg:         ornaments::ROMANCE_PARANORMAL,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_ROMANCE_SUSPENSE: ProfileSpec = ProfileSpec {
    as_str: "romance_suspense",
    label:  "Romance — Suspense",
    blurb:  "Romance pacing with thriller-adjacent typography (Lora + Inter).",
    genre:  Genre::Romance,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::LORA,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "* * *",
    ornament_svg:         ornaments::ASTERISM_DOTS,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

// ── Comedy (H8.2) ───────────────────────────────────────────────────────────

const SPEC_COMEDY_ROMCOM: ProfileSpec = ProfileSpec {
    as_str: "comedy_romcom",
    label:  "Comedy — Romantic Comedy",
    blurb:  "Romance typography with playful Playfair heads and a wave ornament.",
    genre:  Genre::Comedy,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::LORA,
    google_heading:       fonts::PLAYFAIR_DISPLAY,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "～",
    ornament_svg:         ornaments::COMEDY_WAVE,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_COMEDY_SATIRE: ProfileSpec = ProfileSpec {
    as_str: "comedy_satire",
    label:  "Comedy — Satire",
    blurb:  "Source Serif 4 + Inter — sharp, dry, literary-leaning.",
    genre:  Genre::Comedy,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             false,
    google_body:          fonts::SOURCE_SERIF_4,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "§ § §",
    ornament_svg:         ornaments::ASTERISM_DOTS,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_COMEDY_LITERARY: ProfileSpec = ProfileSpec {
    as_str: "comedy_literary_humor",
    label:  "Comedy — Literary Humor",
    blurb:  "Garamond throughout — comedy with literary credentials.",
    genre:  Genre::Comedy,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::EB_GARAMOND,
    google_heading:       fonts::EB_GARAMOND,
    body_em:              "1.02em",
    body_size_pt:         "11.5pt",
    line_height:          "1.55",
    scene_break_glyph:    "❦",
    ornament_svg:         ornaments::LITERARY_FLEURON,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_COMEDY_COZY: ProfileSpec = ProfileSpec {
    as_str: "comedy_cozy",
    label:  "Comedy — Cozy",
    blurb:  "Smaller trim, friendly Lora body, Playfair heads with a soft wave break.",
    genre:  Genre::Comedy,
    trim:   ("5.25in", "8in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::LORA,
    google_heading:       fonts::PLAYFAIR_DISPLAY,
    body_em:              "1.02em",
    body_size_pt:         "11pt",
    line_height:          "1.55",
    scene_break_glyph:    "～",
    ornament_svg:         ornaments::COMEDY_WAVE,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

// ── Non-fiction (H8.2 sub-genres) ───────────────────────────────────────────

const SPEC_NF_NARRATIVE: ProfileSpec = ProfileSpec {
    as_str: "non_fiction_narrative",
    label:  "Non-fiction — Narrative",
    blurb:  "Long-form journalism / narrative non-fiction.  Reads like literary fiction.",
    genre:  Genre::NonFiction,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::EB_GARAMOND,
    google_heading:       fonts::EB_GARAMOND,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "❦",
    ornament_svg:         ornaments::LITERARY_FLEURON,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              true,
    front_matter:         FM_NON_FICTION_TOC,
};

const SPEC_NF_COOKBOOK: ProfileSpec = ProfileSpec {
    as_str: "non_fiction_cookbook",
    label:  "Non-fiction — Cookbook",
    blurb:  "Larger trim, sans-serif throughout, callouts for ingredients and steps.",
    genre:  Genre::NonFiction,
    trim:   ("7in", "9in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::SOURCE_SANS_3,
    google_heading:       fonts::SOURCE_SANS_3,
    body_em:              "1em",
    body_size_pt:         "10.5pt",
    line_height:          "1.4",
    scene_break_glyph:    "▪ ▪ ▪",
    ornament_svg:         ornaments::COOKBOOK_PLATES,
    paragraph_indent_em:  "0",
    pandoc_documentclass: "book",
    pdf_toc:              true,
    front_matter:         FM_NON_FICTION_TOC,
};

const SPEC_NF_WORKBOOK: ProfileSpec = ProfileSpec {
    as_str: "non_fiction_workbook",
    label:  "Non-fiction — Workbook",
    blurb:  "Letter-size workbook trim with checkbox ornaments and block paragraphs.",
    genre:  Genre::NonFiction,
    trim:   ("8.5in", "11in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::SOURCE_SANS_3,
    google_heading:       fonts::SOURCE_SANS_3,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "□ □ □",
    ornament_svg:         ornaments::WORKBOOK_CHECKS,
    paragraph_indent_em:  "0",
    pandoc_documentclass: "book",
    pdf_toc:              true,
    front_matter:         FM_NON_FICTION_TOC,
};

const SPEC_NF_SELFHELP: ProfileSpec = ProfileSpec {
    as_str: "non_fiction_self_help",
    label:  "Non-fiction — Self-help",
    blurb:  "Crimson Pro body + Inter heads — action-oriented, motivational.",
    genre:  Genre::NonFiction,
    trim:   ("6in", "9in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::CRIMSON_PRO,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.45",
    scene_break_glyph:    "→ → →",
    ornament_svg:         ornaments::SELFHELP_ARROWS,
    paragraph_indent_em:  "0",
    pandoc_documentclass: "book",
    pdf_toc:              true,
    front_matter:         FM_NON_FICTION_TOC,
};

// ── Thriller (H8.2) ─────────────────────────────────────────────────────────

const SPEC_THRILLER_PSYCH: ProfileSpec = ProfileSpec {
    as_str: "thriller_psychological",
    label:  "Thriller — Psychological",
    blurb:  "Mass-market lean — Crimson Pro body, Inter heads, vertical-bar scene break.",
    genre:  Genre::Thriller,
    trim:   ("5.5in", "8.25in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::CRIMSON_PRO,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "10.5pt",
    line_height:          "1.4",
    scene_break_glyph:    "‖",
    ornament_svg:         ornaments::THRILLER_BAR,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_LEAN,
};

const SPEC_THRILLER_CRIME: ProfileSpec = ProfileSpec {
    as_str: "thriller_crime",
    label:  "Thriller — Crime / Hard-boiled",
    blurb:  "Vollkorn body for weight, Inter heads, slash-stroke scene break.",
    genre:  Genre::Thriller,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::VOLLKORN,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "/ / /",
    ornament_svg:         ornaments::THRILLER_CRIME,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_LEAN,
};

const SPEC_THRILLER_ESPIONAGE: ProfileSpec = ProfileSpec {
    as_str: "thriller_espionage",
    label:  "Thriller — Spy / Espionage",
    blurb:  "Source Serif 4 + Inter — clean, modern, diamond-cluster scene break.",
    genre:  Genre::Thriller,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::SOURCE_SERIF_4,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "◆ ◆",
    ornament_svg:         ornaments::THRILLER_ESPIONAGE,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_LEAN,
};

const SPEC_THRILLER_ACTION: ProfileSpec = ProfileSpec {
    as_str: "thriller_action",
    label:  "Thriller — Action",
    blurb:  "Mass-market trim, Vollkorn body, sharp Inter heads, vertical-bar break.",
    genre:  Genre::Thriller,
    trim:   ("5.5in", "8.25in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::VOLLKORN,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "10.5pt",
    line_height:          "1.4",
    scene_break_glyph:    "‖",
    ornament_svg:         ornaments::THRILLER_BAR,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_LEAN,
};

// ── Horror (H8.2) ───────────────────────────────────────────────────────────

const SPEC_HORROR_GOTHIC: ProfileSpec = ProfileSpec {
    as_str: "horror_gothic",
    label:  "Horror — Gothic",
    blurb:  "Cormorant Garamond throughout — period-gothic mood, cross ornament.",
    genre:  Genre::Horror,
    trim:   ("5.5in", "8.5in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::CORMORANT_GARAMOND,
    google_heading:       fonts::CORMORANT_GARAMOND,
    body_em:              "1.02em",
    body_size_pt:         "11.5pt",
    line_height:          "1.55",
    scene_break_glyph:    "✠",
    ornament_svg:         ornaments::HORROR_GOTHIC,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_HORROR_COSMIC: ProfileSpec = ProfileSpec {
    as_str: "horror_cosmic",
    label:  "Horror — Cosmic",
    blurb:  "Vollkorn weight + Cormorant heads, descending-triangle ornament.",
    genre:  Genre::Horror,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::VOLLKORN,
    google_heading:       fonts::CORMORANT_GARAMOND,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "▼ ▼ ▼",
    ornament_svg:         ornaments::HORROR_COSMIC,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

const SPEC_HORROR_SLASHER: ProfileSpec = ProfileSpec {
    as_str: "horror_slasher",
    label:  "Horror — Slasher",
    blurb:  "Mass-market trim, Crimson Pro + Inter, jagged-stroke break.",
    genre:  Genre::Horror,
    trim:   ("5.5in", "8.25in"),
    chapter_starts_recto: false,
    drop_cap:             false,
    google_body:          fonts::CRIMSON_PRO,
    google_heading:       fonts::INTER,
    body_em:              "1em",
    body_size_pt:         "10.5pt",
    line_height:          "1.4",
    scene_break_glyph:    "⚡",
    ornament_svg:         ornaments::HORROR_SLASHER,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_LEAN,
};

const SPEC_HORROR_SUPERNATURAL: ProfileSpec = ProfileSpec {
    as_str: "horror_supernatural",
    label:  "Horror — Supernatural",
    blurb:  "Vollkorn + Cormorant heads, crescent-moon ornament for ghosted-page mood.",
    genre:  Genre::Horror,
    trim:   ("6in", "9in"),
    chapter_starts_recto: true,
    drop_cap:             true,
    google_body:          fonts::VOLLKORN,
    google_heading:       fonts::CORMORANT_GARAMOND,
    body_em:              "1em",
    body_size_pt:         "11pt",
    line_height:          "1.5",
    scene_break_glyph:    "☾",
    ornament_svg:         ornaments::HORROR_SUPERNATURAL,
    paragraph_indent_em:  "1.2em",
    pandoc_documentclass: "memoir",
    pdf_toc:              false,
    front_matter:         FM_FICTION_FULL,
};

// ── Public API on FormatProfile ─────────────────────────────────────────────

impl FormatProfile {
    pub fn as_str(self) -> &'static str { spec(self).as_str }
    pub fn label(self) -> &'static str  { spec(self).label  }
    pub fn blurb(self) -> &'static str  { spec(self).blurb  }
    pub fn genre(self) -> Genre         { spec(self).genre  }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        // Linear scan over the small enum is fine — done once per
        // export, not on a hot path.
        ALL_PROFILES.iter().find(|&&p| spec(p).as_str == s).copied()
    }

    pub fn trim_inches(self) -> (&'static str, &'static str) { spec(self).trim }
    pub fn chapter_starts_recto(self) -> bool { spec(self).chapter_starts_recto }
    pub fn drop_cap(self) -> bool             { spec(self).drop_cap }

    /// Body font *family* — the short Google Font name.
    pub fn google_body_family(self) -> &'static str    { spec(self).google_body }
    pub fn google_heading_family(self) -> &'static str { spec(self).google_heading }

    /// Full CSS font stack for the body — Google Font followed by
    /// reasonable system fallbacks so the EPUB still looks book-like
    /// when the Google Font isn't available on the reader's device.
    pub fn body_font_family(self) -> String {
        let g = self.google_body_family();
        // Romance / fantasy / literary tend to be serif-leaning; the
        // sans-serif workbook / cookbook break that pattern.
        let is_sans = matches!(g, fonts::INTER | fonts::SOURCE_SANS_3);
        if is_sans {
            format!("\"{g}\", \"Source Sans Pro\", \"Helvetica Neue\", Helvetica, Arial, sans-serif")
        } else {
            format!("\"{g}\", \"Adobe Garamond Pro\", Garamond, Georgia, \"Times New Roman\", serif")
        }
    }

    pub fn heading_font_family(self) -> String {
        let g = self.google_heading_family();
        let is_sans = matches!(g, fonts::INTER | fonts::SOURCE_SANS_3);
        if is_sans {
            format!("\"{g}\", \"Helvetica Neue\", Helvetica, Arial, sans-serif")
        } else {
            format!("\"{g}\", Garamond, Georgia, \"Times New Roman\", serif")
        }
    }

    pub fn body_em(self) -> &'static str       { spec(self).body_em }
    pub fn body_size_pt(self) -> &'static str  { spec(self).body_size_pt }
    pub fn line_height(self) -> &'static str   { spec(self).line_height }
    pub fn scene_break_glyph(self) -> &'static str { spec(self).scene_break_glyph }

    /// Inline SVG ornament for the scene break.  Empty for `Academic`.
    /// The CSS factory wraps this in a data URI.
    pub fn ornament_svg(self) -> &'static str  { spec(self).ornament_svg }

    pub fn paragraph_indent_em(self) -> &'static str { spec(self).paragraph_indent_em }
    pub fn pandoc_documentclass(self) -> &'static str { spec(self).pandoc_documentclass }

    pub fn pandoc_classoption(self) -> &'static str {
        if self.chapter_starts_recto() { "twoside,openright" } else { "twoside" }
    }

    pub fn pdf_toc(self) -> bool { spec(self).pdf_toc }

    pub fn front_matter_pages(self) -> &'static [FrontMatterPage] {
        spec(self).front_matter
    }
}

/// Every supported profile, used by `from_str` and exhaustive tests.
const ALL_PROFILES: &[FormatProfile] = &[
    FormatProfile::FictionTradeMass, FormatProfile::FictionTradeStandard,
    FormatProfile::FictionLiterary,  FormatProfile::FictionYoungAdult,
    FormatProfile::NonFictionPractical, FormatProfile::NonFictionMemoir,
    FormatProfile::Academic,
    FormatProfile::RomanceContemporary, FormatProfile::RomanceHistorical,
    FormatProfile::RomanceParanormal,   FormatProfile::RomanceSuspense,
    FormatProfile::ComedyRomCom,        FormatProfile::ComedySatire,
    FormatProfile::ComedyLiteraryHumor, FormatProfile::ComedyCozy,
    FormatProfile::NonFictionNarrative, FormatProfile::NonFictionCookbook,
    FormatProfile::NonFictionWorkbook,  FormatProfile::NonFictionSelfHelp,
    FormatProfile::ThrillerPsychological, FormatProfile::ThrillerCrime,
    FormatProfile::ThrillerEspionage,   FormatProfile::ThrillerAction,
    FormatProfile::HorrorGothic,        FormatProfile::HorrorCosmic,
    FormatProfile::HorrorSlasher,       FormatProfile::HorrorSupernatural,
];

impl Default for FormatProfile {
    fn default() -> Self { Self::FictionTradeStandard }
}

/// One front-matter page the packager can auto-emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontMatterPage {
    TitlePage,
    Copyright,
    Dedication,
    Epigraph,
    TableOfContents,
}

impl FrontMatterPage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TitlePage         => "title_page",
            Self::Copyright         => "copyright",
            Self::Dedication        => "dedication",
            Self::Epigraph          => "epigraph",
            Self::TableOfContents   => "table_of_contents",
        }
    }
}

/// Every Google Font family BooksForge uses anywhere in the book
/// typography bundle.  Exposed so the EPUB CSS factory and the Pandoc
/// args path can emit a single canonical `@font-face` / `mainfont`
/// section per profile.
pub const GOOGLE_FONT_BUNDLE: &[&str] = &[
    fonts::EB_GARAMOND,
    fonts::CRIMSON_PRO,
    fonts::LORA,
    fonts::SOURCE_SERIF_4,
    fonts::VOLLKORN,
    fonts::PLAYFAIR_DISPLAY,
    fonts::CORMORANT_GARAMOND,
    fonts::INTER,
    fonts::SOURCE_SANS_3,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_str_for_every_profile() {
        for &p in ALL_PROFILES {
            assert_eq!(FormatProfile::from_str(p.as_str()), Some(p));
            assert!(!p.label().is_empty());
            assert!(!p.blurb().is_empty());
            let (w, h) = p.trim_inches();
            assert!(w.ends_with("in") && h.ends_with("in"));
        }
    }

    #[test]
    fn every_profile_belongs_to_its_genres_subgenre_list() {
        for &p in ALL_PROFILES {
            let g = p.genre();
            assert!(g.sub_genres().contains(&p),
                    "FormatProfile {p:?} reports genre {g:?} but isn't in its sub-genre list");
        }
    }

    #[test]
    fn every_subgenre_lists_only_profiles_of_that_genre() {
        for &g in Genre::all() {
            for &p in g.sub_genres() {
                assert_eq!(p.genre(), g,
                          "Genre::{g:?}::sub_genres() lists {p:?} but its genre() returns {:?}", p.genre());
            }
        }
    }

    #[test]
    fn every_profile_has_a_google_body_font() {
        for &p in ALL_PROFILES {
            assert!(GOOGLE_FONT_BUNDLE.contains(&p.google_body_family()),
                    "{p:?} body font {} not in GOOGLE_FONT_BUNDLE", p.google_body_family());
            assert!(GOOGLE_FONT_BUNDLE.contains(&p.google_heading_family()),
                    "{p:?} heading font {} not in GOOGLE_FONT_BUNDLE", p.google_heading_family());
        }
    }

    #[test]
    fn every_non_academic_profile_has_an_ornament() {
        for &p in ALL_PROFILES {
            if matches!(p, FormatProfile::Academic) { continue; }
            assert!(!p.ornament_svg().is_empty(), "{p:?} should have an ornament SVG");
            assert!(p.ornament_svg().contains("<svg"), "{p:?} ornament must be SVG");
        }
    }

    #[test]
    fn academic_omits_ornament_and_glyph() {
        assert!(FormatProfile::Academic.ornament_svg().is_empty());
        assert!(FormatProfile::Academic.scene_break_glyph().is_empty());
    }

    #[test]
    fn front_matter_includes_title_page_for_every_profile() {
        for &p in ALL_PROFILES {
            assert!(p.front_matter_pages().contains(&FrontMatterPage::TitlePage));
        }
    }

    #[test]
    fn default_is_fiction_trade_standard() {
        assert_eq!(FormatProfile::default(), FormatProfile::FictionTradeStandard);
    }

    #[test]
    fn five_genres_each_have_at_least_three_subgenres() {
        for &g in &[Genre::Romance, Genre::Comedy, Genre::NonFiction, Genre::Thriller, Genre::Horror] {
            assert!(g.sub_genres().len() >= 3,
                    "{g:?} should have at least 3 sub-genres for the H8.2 starter set");
        }
    }

    #[test]
    fn font_family_strings_quote_the_google_name() {
        let p = FormatProfile::RomanceHistorical;
        let body = p.body_font_family();
        assert!(body.starts_with("\"Cormorant Garamond\""), "got: {body}");
    }
}
