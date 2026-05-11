//! Publishing target — orthogonal axis to [`FormatProfile`].
//!
//! Where `FormatProfile` describes *typography* (Romance vs. Thriller,
//! 5×8 vs. 6×9, Garamond vs. Lora), `PublishingTarget` describes
//! **platform-compliance**: which spec the export must satisfy to be
//! accepted by a given storefront or distributor.
//!
//! The two axes compose. A user might pick:
//!
//!   - `FormatProfile::ThrillerPsychological` (typography)
//!   - `PublishingTarget::KdpKindle`          (compliance)
//!
//! …and the export pipeline emits an EPUB-3 with thriller-psychological
//! typography *and* KDP-Kindle metadata defaults, ToC-depth cap, cover
//! dimensions, and accessibility tags.
//!
//! ## Targets supported
//!
//! Each target encodes:
//!
//!   - The artifact format(s) it accepts (PDF/X-1a, EPUB-3, DOCX-Shunn).
//!   - Pre-flight validators (ISBN required? cover dims? ToC depth cap?
//!     image DPI minimum?).
//!   - Metadata schema (`urn:isbn:`, `urn:bf:project:`, etc.).
//!   - Accessibility metadata expectations.
//!   - Trim size constraints (KDP allowlist vs. IngramSpark vs. Apple).
//!
//! Source spec snapshots (last refreshed 2026-05-09 per the BooksForge
//! capability test web research):
//!
//!   - **KDP Paperback** — trim sizes 5×8, 5.25×8, 5.5×8.5, 6×9 (most
//!     common); inside (gutter) margin scales with page count
//!     (0.375" < 150 pp; 0.5" 151-300; 0.625" 301-500; 0.75" 501-700;
//!     0.875" >700); outside ≥ 0.25"; bleed 0.125" all sides; PDF/X
//!     preferred (PDF/X-1a); fonts embedded; image min 300 DPI.
//!   - **KDP Kindle** — EPUB-3 (MOBI rejected since 2025-03); EPUBCheck
//!     critical errors block; cover 1600×2560 / 1.6:1 / RGB / JPEG
//!     preferred; image min 300 DPI; full-page 1200×1800.
//!   - **IngramSpark** — EPUB-2 or EPUB-3 (they convert to other
//!     formats); separate ISBN per format; metadata-cover match
//!     required (title on cover must match metadata title).
//!   - **Apple Books** — EPUB-3 + nav.xhtml + landmarks required;
//!     accessibility metadata strongly preferred; ISBN required for
//!     paid titles.
//!   - **Google Play Books** — EPUB-3 (or PDF for text-light); ISBN
//!     not strictly required (Google assigns GGKEY-prefixed IDs) but
//!     strongly preferred; ToC depth ≤3.
//!   - **Shunn manuscript** — TNR/Courier 12pt, double-spaced,
//!     half-inch indent, page header with surname / title / pp,
//!     #/three-asterisks scene breaks, italics (not underline).
//!
//! ## Why a separate axis
//!
//! Conflating typography and platform spec leads to a combinatorial
//! mess (Romance × KDP, Romance × IngramSpark, Thriller × KDP, …).
//! Keeping them orthogonal means each axis stays small and the export
//! pipeline composes them cheaply at run-time.

use serde::{Deserialize, Serialize};

// ── Enum ────────────────────────────────────────────────────────────────────

/// Platform / storefront the export must comply with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishingTarget {
    /// No platform-specific compliance — a plain DOCX/EPUB/PDF for
    /// internal review or a niche storefront BooksForge doesn't yet
    /// model. The export pipeline runs without per-target validators
    /// and uses defaults from the [`FormatProfile`] alone.
    Generic,

    /// Amazon KDP Paperback. Output: PDF (target PDF/X-1a:2001).
    /// Print interior with KDP-conformant trim, gutter, bleed, and
    /// fully embedded fonts. PDF/X-1a conversion is manual via Acrobat
    /// in MVP; BooksForge emits a clean PDF and walks the user through
    /// the conversion step.
    KdpPaperback,

    /// Amazon KDP Hardcover. Output: PDF (PDF/X-1a). Trim sizes
    /// 6×9, 5.5×8.5, 6.14×9.21; otherwise mirrors KDP Paperback rules.
    KdpHardcover,

    /// Amazon KDP Kindle eBook. Output: EPUB-3 (MOBI rejected since
    /// 2025-03). Validates with EPUBCheck; KDP-specific metadata
    /// defaults; cover ≥ 2560×1600 / 1.6:1.
    KdpKindle,

    /// IngramSpark Print (POD). Output: PDF/X-1a:2001. Separate ISBN
    /// from the ebook edition; metadata-cover-match validation.
    IngramSparkPrint,

    /// IngramSpark eBook distribution. Output: EPUB-3 (also accepts
    /// EPUB-2). ISBN required. Distributes to Apple/Kobo/B&N/etc.
    IngramSparkEbook,

    /// Apple Books store. Output: EPUB-3 with full landmarks +
    /// accessibility metadata. EPUBCheck pass strongly recommended.
    AppleBooks,

    /// Google Play Books. Output: EPUB-3 (or PDF for text-light);
    /// ISBN-13 strongly recommended (Google assigns GGKEY otherwise);
    /// ToC depth ≤3 enforced.
    GoogleBooks,

    /// Kobo Writing Life direct submission. Output: EPUB-3.
    KoboDirect,

    /// Standard manuscript format for traditional-publishing
    /// submission (Shunn). Output: DOCX with TNR/Courier 12pt,
    /// double-spaced, page header, hash/asterisk scene breaks.
    ShunnManuscript,
}

// ── Spec ────────────────────────────────────────────────────────────────────

/// The artifact format a target accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFormat {
    PdfX1a,
    Pdf,
    Epub3,
    Epub2,
    Docx,
    Markdown,
}

/// Identifier scheme used in the EPUB OPF `<dc:identifier>` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierScheme {
    /// `urn:isbn:NNN-NNNNNNNNNN` — ISBN-13 required.
    UrnIsbn,
    /// `urn:isbn:NNN-NNNNNNNNNN` — ISBN-13 strongly recommended; if
    /// absent, fall back to `urn:bf:project:<ULID>`.
    UrnIsbnPreferred,
    /// Project ULID only — no ISBN required.
    UrnBfProject,
}

/// Per-target compliance spec — a flat record the export pipeline can
/// inspect at run-time without per-target conditionals scattered
/// everywhere.
///
/// Serialize-only: the spec is constructed at run-time from
/// `PublishingTarget::spec()` and surfaced through Tauri IPC. It is
/// never deserialized (the target enum value is the wire-shape).
/// `Eq` is omitted because `f32` trim widths don't implement `Eq`;
/// `PartialEq` is sufficient for the unit tests.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TargetSpec {
    pub target: PublishingTarget,
    pub label: &'static str,
    pub blurb: &'static str,
    pub artifact_formats: &'static [ArtifactFormat],
    /// Trim sizes the target accepts as `(label, width_in, height_in)`.
    /// Empty for ebook-only targets.
    pub allowed_trims: &'static [(&'static str, f32, f32)],
    /// Identifier scheme for the EPUB OPF / DOCX metadata block.
    pub identifier_scheme: IdentifierScheme,
    /// Maximum ToC depth (heading levels included).
    pub toc_depth_max: u8,
    /// Minimum image DPI.
    pub image_min_dpi: u32,
    /// Cover-image minimum dimensions in pixels (w, h). `(0, 0)` for
    /// targets that don't ingest a separate cover (Shunn / Generic).
    pub cover_min_px: (u32, u32),
    /// Cover-image required aspect ratio in 100ths (e.g. 160 = 1.60).
    /// `0` if any aspect is allowed.
    pub cover_aspect_x100: u32,
    /// `true` if the target's pre-flight requires fully-embedded
    /// fonts (KDP/IngramSpark print). `false` for ebook targets that
    /// do their own font subsetting.
    pub fonts_embedded_required: bool,
    /// `true` if PDF/X-1a is the required print output. `false` for
    /// targets that accept generic PDF or are ebook-only.
    pub pdfx_required: bool,
    /// `true` if the target requires accessibility metadata
    /// (`schema:accessMode`, `schema:accessibilityFeature`,
    /// `schema:accessibilityHazard`). Best practice everywhere; hard
    /// requirement on Apple Books and EU markets.
    pub accessibility_required: bool,
    /// `true` if EPUBCheck must pass cleanly (no critical errors).
    pub epubcheck_required: bool,
    /// Inline copy shown to the user *before* they export, explaining
    /// what the target's spec wants. Plain English; no jargon.
    pub user_briefing: &'static str,
}

// ── Per-target specs ────────────────────────────────────────────────────────

const KDP_PAPERBACK_TRIMS: &[(&str, f32, f32)] = &[
    ("5×8", 5.00, 8.00),
    ("5.06×7.81", 5.06, 7.81),
    ("5.25×8", 5.25, 8.00),
    ("5.5×8.5", 5.50, 8.50),
    ("6×9", 6.00, 9.00),
    ("6.14×9.21", 6.14, 9.21),
    ("6.69×9.61", 6.69, 9.61),
    ("7×10", 7.00, 10.00),
    ("7.44×9.69", 7.44, 9.69),
    ("7.5×9.25", 7.50, 9.25),
    ("8×10", 8.00, 10.00),
    ("8.25×11", 8.25, 11.00),
    ("8.5×11", 8.50, 11.00),
];

const KDP_HARDCOVER_TRIMS: &[(&str, f32, f32)] = &[
    ("5.5×8.5", 5.50, 8.50),
    ("6×9", 6.00, 9.00),
    ("6.14×9.21", 6.14, 9.21),
    ("7×10", 7.00, 10.00),
];

const INGRAM_PRINT_TRIMS: &[(&str, f32, f32)] = &[
    ("5×8", 5.00, 8.00),
    ("5.5×8.5", 5.50, 8.50),
    ("6×9", 6.00, 9.00),
    ("6.14×9.21", 6.14, 9.21),
    ("7×10", 7.00, 10.00),
    ("8.5×11", 8.50, 11.00),
];

const SHUNN_TRIM: &[(&str, f32, f32)] = &[("US Letter (manuscript)", 8.50, 11.00)];

impl PublishingTarget {
    /// Stable lowercase identifier (used in IPC, filesystem, and
    /// `serde` round-trips).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::KdpPaperback => "kdp_paperback",
            Self::KdpHardcover => "kdp_hardcover",
            Self::KdpKindle => "kdp_kindle",
            Self::IngramSparkPrint => "ingram_spark_print",
            Self::IngramSparkEbook => "ingram_spark_ebook",
            Self::AppleBooks => "apple_books",
            Self::GoogleBooks => "google_books",
            Self::KoboDirect => "kobo_direct",
            Self::ShunnManuscript => "shunn_manuscript",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "generic" => Some(Self::Generic),
            "kdp_paperback" => Some(Self::KdpPaperback),
            "kdp_hardcover" => Some(Self::KdpHardcover),
            "kdp_kindle" => Some(Self::KdpKindle),
            "ingram_spark_print" => Some(Self::IngramSparkPrint),
            "ingram_spark_ebook" => Some(Self::IngramSparkEbook),
            "apple_books" => Some(Self::AppleBooks),
            "google_books" => Some(Self::GoogleBooks),
            "kobo_direct" => Some(Self::KoboDirect),
            "shunn_manuscript" => Some(Self::ShunnManuscript),
            _ => None,
        }
    }

    /// All targets, in UI display order — most popular first.
    pub fn all() -> &'static [Self] {
        &[
            Self::KdpPaperback,
            Self::KdpKindle,
            Self::KdpHardcover,
            Self::IngramSparkPrint,
            Self::IngramSparkEbook,
            Self::AppleBooks,
            Self::GoogleBooks,
            Self::KoboDirect,
            Self::ShunnManuscript,
            Self::Generic,
        ]
    }

    /// Per-target spec record. The export pipeline reads this; the UI
    /// reads this; pre-flight validators read this.
    pub fn spec(self) -> TargetSpec {
        match self {
            Self::Generic => TargetSpec {
                target: self,
                label: "Generic",
                blurb: "Plain DOCX/EPUB/PDF — no platform-specific compliance.",
                artifact_formats: &[
                    ArtifactFormat::Markdown,
                    ArtifactFormat::Epub3,
                    ArtifactFormat::Docx,
                    ArtifactFormat::Pdf,
                ],
                allowed_trims: &[],
                identifier_scheme: IdentifierScheme::UrnBfProject,
                toc_depth_max: 6,
                image_min_dpi: 0,
                cover_min_px: (0, 0),
                cover_aspect_x100: 0,
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: false,
                epubcheck_required: false,
                user_briefing: "No platform spec applied. Use this to share an EPUB or DOCX \
                     for review, or to publish to a storefront BooksForge does \
                     not yet model.",
            },

            Self::KdpPaperback => TargetSpec {
                target: self,
                label: "KDP Paperback",
                blurb: "Amazon KDP print interior. PDF/X-1a target; gutter scales with page count.",
                artifact_formats: &[ArtifactFormat::PdfX1a, ArtifactFormat::Pdf],
                allowed_trims: KDP_PAPERBACK_TRIMS,
                identifier_scheme: IdentifierScheme::UrnIsbnPreferred,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1600, 2560),
                cover_aspect_x100: 0,
                fonts_embedded_required: true,
                pdfx_required: true,
                accessibility_required: false,
                epubcheck_required: false,
                user_briefing:
                    "KDP Paperback expects PDF/X-1a:2001. BooksForge emits a clean PDF; \
                     do the final PDF/X-1a conversion in Acrobat (Print Production → \
                     Convert to PDF/X-1a). Trim 6×9 is the default; gutter scales with \
                     page count (0.375\" < 150 pp; 0.5\" 151-300; 0.625\" 301-500; \
                     0.75\" 501-700; 0.875\" >700). All fonts must be embedded; image DPI ≥ 300.",
            },

            Self::KdpHardcover => TargetSpec {
                target: self,
                label: "KDP Hardcover",
                blurb: "Amazon KDP hardback interior. PDF/X-1a; restricted trim list.",
                artifact_formats: &[ArtifactFormat::PdfX1a, ArtifactFormat::Pdf],
                allowed_trims: KDP_HARDCOVER_TRIMS,
                identifier_scheme: IdentifierScheme::UrnIsbnPreferred,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1600, 2560),
                cover_aspect_x100: 0,
                fonts_embedded_required: true,
                pdfx_required: true,
                accessibility_required: false,
                epubcheck_required: false,
                user_briefing: "KDP Hardcover follows the same interior rules as Paperback but \
                     restricts trim to a smaller list (5.5×8.5, 6×9, 6.14×9.21, 7×10).",
            },

            Self::KdpKindle => TargetSpec {
                target: self,
                label: "KDP Kindle",
                blurb: "Amazon KDP eBook. EPUB-3 reflowable. EPUBCheck must pass.",
                artifact_formats: &[ArtifactFormat::Epub3],
                allowed_trims: &[],
                identifier_scheme: IdentifierScheme::UrnBfProject,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1600, 2560),
                cover_aspect_x100: 160, // 1.6:1
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: false,
                epubcheck_required: true,
                user_briefing: "Amazon stopped accepting MOBI in March 2025 — EPUB-3 only. Cover \
                     ≥ 2560×1600 (1.6:1), RGB, JPEG preferred. ToC depth ≤ 3. EPUBCheck \
                     critical errors block upload. From 2026-01-20 readers can download \
                     DRM-free Kindle books as EPUB/PDF, so spec-compliance matters more \
                     than ever.",
            },

            Self::IngramSparkPrint => TargetSpec {
                target: self,
                label: "IngramSpark Print",
                blurb: "POD distribution. Separate ISBN required; metadata-cover match.",
                artifact_formats: &[ArtifactFormat::PdfX1a],
                allowed_trims: INGRAM_PRINT_TRIMS,
                identifier_scheme: IdentifierScheme::UrnIsbn,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1600, 2560),
                cover_aspect_x100: 0,
                fonts_embedded_required: true,
                pdfx_required: true,
                accessibility_required: false,
                epubcheck_required: false,
                user_briefing: "IngramSpark requires a unique ISBN-13 per format (different from \
                     your ebook ISBN). Cover title and metadata title MUST match exactly. \
                     PDF/X-1a interior; fonts embedded; image DPI ≥ 300.",
            },

            Self::IngramSparkEbook => TargetSpec {
                target: self,
                label: "IngramSpark eBook",
                blurb: "Wide ebook distribution (Apple, Kobo, B&N, libraries). EPUB-3.",
                artifact_formats: &[ArtifactFormat::Epub3, ArtifactFormat::Epub2],
                allowed_trims: &[],
                identifier_scheme: IdentifierScheme::UrnIsbn,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1600, 2400),
                cover_aspect_x100: 0,
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: true,
                epubcheck_required: true,
                user_briefing:
                    "IngramSpark distributes to Apple/Kobo/B&N/libraries. Unique ISBN-13 \
                     required (different from your KDP ASIN). EPUBCheck pass; \
                     accessibility metadata strongly preferred (Apple Books and EU \
                     accessibility act enforce it).",
            },

            Self::AppleBooks => TargetSpec {
                target: self,
                label: "Apple Books",
                blurb: "Direct submission. EPUB-3 with landmarks + accessibility metadata.",
                artifact_formats: &[ArtifactFormat::Epub3],
                allowed_trims: &[],
                identifier_scheme: IdentifierScheme::UrnIsbn,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1400, 2100),
                cover_aspect_x100: 0,
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: true,
                epubcheck_required: true,
                user_briefing: "Apple Books requires EPUB-3 with a valid `nav.xhtml` (toc + \
                     landmarks), accessibility metadata (`schema:accessMode`, \
                     `schema:accessibilityFeature`, `schema:accessibilityHazard`), \
                     and EPUBCheck pass. ISBN required for paid titles.",
            },

            Self::GoogleBooks => TargetSpec {
                target: self,
                label: "Google Play Books",
                blurb: "EPUB-3 (or PDF). ISBN-13 preferred — Google assigns GGKEY otherwise.",
                artifact_formats: &[ArtifactFormat::Epub3, ArtifactFormat::Pdf],
                allowed_trims: &[],
                identifier_scheme: IdentifierScheme::UrnIsbnPreferred,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1400, 2100),
                cover_aspect_x100: 0,
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: true,
                epubcheck_required: true,
                user_briefing: "Google Play Books accepts EPUB-3 (preferred) or PDF. ISBN-13 not \
                     strictly required (Google assigns a GGKEY-prefixed identifier when \
                     missing) but strongly preferred for discoverability. ToC depth ≤ 3.",
            },

            Self::KoboDirect => TargetSpec {
                target: self,
                label: "Kobo Writing Life",
                blurb: "Direct Kobo submission. EPUB-3.",
                artifact_formats: &[ArtifactFormat::Epub3],
                allowed_trims: &[],
                identifier_scheme: IdentifierScheme::UrnIsbnPreferred,
                toc_depth_max: 3,
                image_min_dpi: 300,
                cover_min_px: (1400, 2100),
                cover_aspect_x100: 0,
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: true,
                epubcheck_required: true,
                user_briefing: "Kobo Writing Life accepts EPUB-3. ISBN-13 preferred. EPUBCheck \
                     pass. Accessibility metadata recommended.",
            },

            Self::ShunnManuscript => TargetSpec {
                target: self,
                label: "Shunn Manuscript (DOCX)",
                blurb: "Standard manuscript format for traditional submission.",
                artifact_formats: &[ArtifactFormat::Docx],
                allowed_trims: SHUNN_TRIM,
                identifier_scheme: IdentifierScheme::UrnBfProject,
                toc_depth_max: 0, // Shunn doesn't use a ToC; chapter heads only.
                image_min_dpi: 0,
                cover_min_px: (0, 0),
                cover_aspect_x100: 0,
                fonts_embedded_required: false,
                pdfx_required: false,
                accessibility_required: false,
                epubcheck_required: false,
                user_briefing:
                    "Shunn-style manuscript for agent / traditional-publisher submission. \
                     Times New Roman 12pt, double-spaced, half-inch first-line indent, \
                     page header (surname / TITLE-IN-CAPS / pp), single hash (#) or \
                     three asterisks for scene breaks, italics (not underline). Word \
                     count rounded to nearest 100, listed on the title page.",
            },
        }
    }

    /// Convenience: human-readable label for the UI.
    pub fn label(self) -> &'static str {
        self.spec().label
    }

    /// Convenience: one-line blurb for the UI tile.
    pub fn blurb(self) -> &'static str {
        self.spec().blurb
    }
}

// ── Gutter math ─────────────────────────────────────────────────────────────

/// Compute the KDP-conformant inner (gutter) margin in inches based on
/// the manuscript's interior page count. Bands match the Amazon KDP
/// Paperback Submission Guidelines (snapshot 2026-05-09):
///
///   - <  150 pages  → 0.375"
///   - 151-300       → 0.500"
///   - 301-500       → 0.625"
///   - 501-700       → 0.750"
///   - >  700        → 0.875"
pub fn kdp_paperback_gutter_inches(pages: u32) -> f32 {
    match pages {
        0..=150 => 0.375,
        151..=300 => 0.500,
        301..=500 => 0.625,
        501..=700 => 0.750,
        _ => 0.875,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_targets() {
        for t in PublishingTarget::all() {
            assert_eq!(PublishingTarget::from_str(t.as_str()), Some(*t));
        }
    }

    #[test]
    fn kdp_paperback_gutter_bands() {
        assert!((kdp_paperback_gutter_inches(100) - 0.375).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(150) - 0.375).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(151) - 0.500).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(300) - 0.500).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(301) - 0.625).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(500) - 0.625).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(501) - 0.750).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(700) - 0.750).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(701) - 0.875).abs() < 1e-6);
        assert!((kdp_paperback_gutter_inches(1500) - 0.875).abs() < 1e-6);
    }

    #[test]
    fn kindle_requires_epubcheck_and_no_pdfx() {
        let s = PublishingTarget::KdpKindle.spec();
        assert!(s.epubcheck_required);
        assert!(!s.pdfx_required);
        assert!(!s.fonts_embedded_required);
        assert_eq!(s.cover_aspect_x100, 160);
    }

    #[test]
    fn paperback_requires_pdfx_and_embedded_fonts() {
        let s = PublishingTarget::KdpPaperback.spec();
        assert!(s.pdfx_required);
        assert!(s.fonts_embedded_required);
        assert!(!s.epubcheck_required);
        assert!(s.allowed_trims.iter().any(|(label, _, _)| *label == "6×9"));
    }

    #[test]
    fn ingram_print_requires_isbn() {
        let s = PublishingTarget::IngramSparkPrint.spec();
        assert!(matches!(s.identifier_scheme, IdentifierScheme::UrnIsbn));
        assert!(s.pdfx_required);
    }

    #[test]
    fn google_books_isbn_preferred_not_required() {
        let s = PublishingTarget::GoogleBooks.spec();
        assert!(matches!(
            s.identifier_scheme,
            IdentifierScheme::UrnIsbnPreferred
        ));
    }

    #[test]
    fn apple_books_requires_accessibility_and_epubcheck() {
        let s = PublishingTarget::AppleBooks.spec();
        assert!(s.accessibility_required);
        assert!(s.epubcheck_required);
    }

    #[test]
    fn shunn_manuscript_no_toc_no_cover_no_dpi() {
        let s = PublishingTarget::ShunnManuscript.spec();
        assert_eq!(s.toc_depth_max, 0);
        assert_eq!(s.cover_min_px, (0, 0));
        assert_eq!(s.image_min_dpi, 0);
        assert_eq!(s.artifact_formats, &[ArtifactFormat::Docx]);
    }

    #[test]
    fn all_targets_have_distinct_str_ids() {
        let mut ids: Vec<&'static str> =
            PublishingTarget::all().iter().map(|t| t.as_str()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate str id in PublishingTarget");
    }
}
