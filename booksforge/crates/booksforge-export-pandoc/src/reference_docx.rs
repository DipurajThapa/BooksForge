//! Programmatic Pandoc reference-doc generator (BACKLOG §H8.2 follow-up).
//!
//! Pandoc's `--reference-doc=<path>` flag lets us pre-style every DOCX
//! export by handing it a tiny "skeleton" `.docx` containing nothing
//! but the styles we want applied.  Pandoc then rewrites our manuscript
//! Markdown into that styled shell.
//!
//! Until this turn, BooksForge required writers to drop a
//! `reference.docx` in their bundle for genre-specific styling.  This
//! module **builds the reference doc programmatically from a
//! [`FormatProfile`]** — body / heading font, body size, line height,
//! paragraph indent, and text-block headings (1-6) all flow straight
//! from the profile's typography knobs.  The result is a
//! deterministic, byte-identical-for-byte-identical-input `.docx` zip
//! we hand to Pandoc as the reference doc.
//!
//! ## Determinism
//!
//! Same input → same bytes.  We pin the ZIP entry timestamp to
//! 1980-01-01 (the lowest value the format permits) and write entries
//! in a fixed order so two builds with the same `FormatProfile`
//! produce byte-identical `.docx` output.  This matters for
//! reproducibility tests that hash the eventual export artefact.
//!
//! ## What's inside
//!
//! The generated `.docx` is a minimal but valid OOXML package:
//!
//!   - `[Content_Types].xml` — MIME map for the parts.
//!   - `_rels/.rels` — top-level rel pointer to `word/document.xml`.
//!   - `word/_rels/document.xml.rels` — empty rels for the document.
//!   - `word/document.xml` — empty body.  Pandoc uses this only as a
//!     starting frame; the manuscript content replaces the body.
//!   - `word/styles.xml` — the **load-bearing file** here.  Holds:
//!       - `docDefaults` — body font, size, line-height pulled from
//!         `FormatProfile`.
//!       - `Normal` (paragraph default).
//!       - `Heading1`–`Heading6` — heading font from `FormatProfile`,
//!         sizes scaled from the body size.
//!
//! Pandoc's reference-doc machinery only reads `styles.xml`, so this
//! is the bare minimum that gets us styled output.
//!
//! ## Open follow-ups
//!
//! - Per-genre cover-page / front-matter style overrides (would need
//!   `glossary.xml`, `theme1.xml`).
//! - Drop cap on first paragraph of a chapter (Word doesn't have a
//!   first-class drop cap; would need a frame-anchored shape).
//! - Ornament SVG insertion at scene breaks (the EPUB pipeline
//!   already does this; for DOCX we'd embed the SVG as an inline
//!   drawing).
//!
//! All three are tracked under "DOCX styling parity" in BACKLOG §H8.2.

use std::io::{Cursor, Write as _};

use booksforge_domain::FormatProfile;
use zip::{write::SimpleFileOptions, CompressionMethod, DateTime, ZipWriter};

/// Build a minimal styled `reference.docx` for the given `FormatProfile`
/// and return its bytes.  Deterministic — same profile → same bytes.
///
/// The `expect()` calls below write to an in-memory `Cursor<Vec<u8>>`,
/// which is infallible (the Vec grows as needed) and never blocks on
/// I/O — so they cannot fail in practice.  `unwrap_or_else` would
/// silently mask a real bug, so we explicitly allow `expect_used`.
#[allow(clippy::expect_used)]
pub fn build_reference_docx(profile: FormatProfile) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(4 * 1024);
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        // Pinned epoch so ZIP timestamps don't leak across runs / hosts.
        let epoch = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
            .unwrap_or_else(|_| DateTime::default());
        let opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(epoch)
            .unix_permissions(0o644);

        // Order matters for determinism — pick a fixed sequence and stick
        // to it across all builds.
        write_entry(&mut zip, opts, "[Content_Types].xml", CONTENT_TYPES);
        write_entry(&mut zip, opts, "_rels/.rels", TOP_RELS);
        write_entry(&mut zip, opts, "word/_rels/document.xml.rels", DOC_RELS);
        write_entry(&mut zip, opts, "word/document.xml", EMPTY_DOCUMENT);
        let styles = render_styles_xml(profile);
        write_entry(&mut zip, opts, "word/styles.xml", &styles);

        zip.finish().expect("finish zip into in-memory buffer");
    }
    buf
}

/// Writes one ZIP entry to the in-memory `Cursor<Vec<u8>>` — the
/// inner buffer grows as needed and never blocks on I/O, so the
/// `expect()` calls below are infallible.  See
/// `build_reference_docx` for the broader rationale.
#[allow(clippy::expect_used)]
fn write_entry(
    zip: &mut ZipWriter<Cursor<&mut Vec<u8>>>,
    opts: SimpleFileOptions,
    name: &str,
    body: &str,
) {
    zip.start_file(name, opts).expect("start_file");
    zip.write_all(body.as_bytes()).expect("write entry body");
}

// ── Static parts (identical across every reference doc we ship) ────────────

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#;

const TOP_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;

const EMPTY_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p/>
  </w:body>
</w:document>"#;

// ── styles.xml (the load-bearing per-profile part) ─────────────────────────

/// Render `word/styles.xml` from a `FormatProfile`.  Body font + size +
/// line-height come straight from the profile; heading 1–6 are scaled
/// from the body size on a `1.8 / 1.5 / 1.25 / 1.1 / 1.0 / 0.9`
/// progression chosen to mimic the EPUB CSS rendering rhythm.
fn render_styles_xml(profile: FormatProfile) -> String {
    let body_font = profile.google_body_family();
    let heading_font = profile.google_heading_family();
    // OOXML sizes are in *half-points*: 11pt → 22; 14pt → 28.
    let body_pt = parse_pt(profile.body_size_pt()).unwrap_or(11.0);
    let body_hp = (body_pt * 2.0).round() as u32;
    // Line-height in OOXML is 240ths of a line at 1.0; 1.5 → 360,
    // 1.4 → 336, 1.6 → 384.  Use `lineRule="auto"` so Word
    // treats it as a multiple of the font size rather than absolute.
    let leading = (parse_line_height(profile.line_height()).unwrap_or(1.5) * 240.0).round() as u32;
    // Paragraph indent in twentieths of a point ("twips"):
    // 1em ≈ body_pt × 20 (Word's default em = 12pt = 240 twips for 12pt body).
    let indent_twips =
        (parse_em(profile.paragraph_indent_em()).unwrap_or(1.2) * body_pt * 20.0).round() as u32;
    // Six-step heading scale (h1 large → h6 small).
    const HEADING_SCALE: [f64; 6] = [1.8, 1.5, 1.25, 1.1, 1.0, 0.9];

    let mut headings = String::new();
    for (i, scale) in HEADING_SCALE.iter().enumerate() {
        let level = i + 1;
        let hp = (body_pt * scale * 2.0).round() as u32;
        headings.push_str(&format!(
            r#"<w:style w:type="paragraph" w:styleId="Heading{level}">
    <w:name w:val="heading {level}"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:rPr>
      <w:rFonts w:ascii="{heading_font}" w:hAnsi="{heading_font}"/>
      <w:b/>
      <w:sz w:val="{hp}"/>
      <w:szCs w:val="{hp}"/>
    </w:rPr>
  </w:style>
"#,
        ));
    }

    // Drop-cap paragraph style (BACKLOG §H8.2 follow-up).
    //
    // Word's native drop cap mechanism is `<w:framePr w:dropCap="drop" .../>`
    // on the paragraph, which floats the first character into a 3-line
    // frame.  Profiles that don't enable drop caps in `format_profile`
    // still emit the style — Pandoc applies it only when the source
    // markup tags the paragraph with `{.drop}`, so unused styles are
    // harmless overhead.
    let drop_block = if profile.drop_cap() {
        format!(
            r#"<w:style w:type="paragraph" w:styleId="Drop">
    <w:name w:val="Drop Cap"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:pPr>
      <w:framePr w:wrap="around" w:vAnchor="text" w:hAnchor="text" w:dropCap="drop" w:lines="3"/>
      <w:ind w:firstLine="0"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="{heading_font}" w:hAnsi="{heading_font}"/>
    </w:rPr>
  </w:style>
"#,
        )
    } else {
        String::new()
    };

    // Scene-break paragraph style (BACKLOG §H8.2 follow-up).
    //
    // Centred paragraph with the profile's Unicode glyph as the
    // visible content (e.g. "❦" for FictionLiterary, "* * *" for
    // FictionTradeStandard).  The full SVG ornament drawing emitted
    // in EPUB requires a `<w:drawing>` with an inline SVG part
    // reference — defer that to a later turn; the Unicode glyph
    // covers the common case and is what most published trade
    // paperbacks actually print.
    let scene_glyph = profile.scene_break_glyph();
    let scene_block = if scene_glyph.is_empty() {
        String::new()
    } else {
        format!(
            r#"<w:style w:type="paragraph" w:styleId="SceneBreak">
    <w:name w:val="Scene Break"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:pPr>
      <w:jc w:val="center"/>
      <w:spacing w:before="240" w:after="240" w:line="{leading}" w:lineRule="auto"/>
      <w:ind w:firstLine="0"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="{body_font}" w:hAnsi="{body_font}"/>
    </w:rPr>
  </w:style>
"#,
        )
    };
    let _ = scene_glyph; // glyph itself is rendered by manuscript_to_markdown,
                         // not by the styles.xml — kept for parity and future
                         // SVG-drawing follow-up.

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="{body_font}" w:hAnsi="{body_font}"/>
        <w:sz w:val="{body_hp}"/>
        <w:szCs w:val="{body_hp}"/>
      </w:rPr>
    </w:rPrDefault>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:line="{leading}" w:lineRule="auto" w:after="0"/>
        <w:ind w:firstLine="{indent_twips}"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
  {headings}{drop_block}{scene_block}</w:styles>"#,
    )
}

fn parse_pt(s: &str) -> Option<f64> {
    s.strip_suffix("pt").and_then(|n| n.trim().parse().ok())
}

fn parse_em(s: &str) -> Option<f64> {
    if s == "0" {
        return Some(0.0);
    }
    s.strip_suffix("em").and_then(|n| n.trim().parse().ok())
}

fn parse_line_height(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_produces_a_valid_zip_archive() {
        let bytes = build_reference_docx(FormatProfile::FictionTradeStandard);
        // Smoke check: opens as a zip + has the load-bearing entries.
        let cursor = Cursor::new(&bytes[..]);
        let mut zip = zip::ZipArchive::new(cursor).expect("re-open zip");
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/_rels/document.xml.rels",
            "word/document.xml",
            "word/styles.xml",
        ] {
            assert!(
                zip.by_name(name).is_ok(),
                "expected entry {name} in reference.docx"
            );
        }
    }

    #[test]
    fn styles_xml_quotes_the_format_profile_fonts() {
        let xml = render_styles_xml(FormatProfile::RomanceHistorical);
        // RomanceHistorical uses Cormorant Garamond for both body and heading.
        assert!(
            xml.contains(r#"w:ascii="Cormorant Garamond""#),
            "Cormorant Garamond missing from styles.xml: {xml}"
        );
    }

    #[test]
    fn body_size_round_trips_from_format_profile_pt_to_half_points() {
        let xml = render_styles_xml(FormatProfile::FictionTradeMass); // 10.5pt body
                                                                      // 10.5pt → 21 half-points.
        assert!(
            xml.contains(r#"<w:sz w:val="21"/>"#),
            "expected w:sz=21 (10.5pt × 2): {xml}"
        );
    }

    #[test]
    fn build_is_byte_deterministic_across_runs() {
        let a = build_reference_docx(FormatProfile::FictionLiterary);
        let b = build_reference_docx(FormatProfile::FictionLiterary);
        assert_eq!(a, b, "reference.docx must be byte-deterministic");
    }

    #[test]
    fn build_differs_for_different_profiles() {
        let a = build_reference_docx(FormatProfile::FictionLiterary);
        let b = build_reference_docx(FormatProfile::ThrillerCrime);
        assert_ne!(
            a, b,
            "different profiles must produce different reference.docx"
        );
    }

    #[test]
    fn drop_cap_style_emitted_when_profile_enables_drop_cap() {
        // FictionTradeStandard has drop_cap=true.
        let xml = render_styles_xml(FormatProfile::FictionTradeStandard);
        assert!(
            xml.contains(r#"w:styleId="Drop""#),
            "expected Drop paragraph style for drop-cap-enabled profile"
        );
        assert!(
            xml.contains(r#"w:dropCap="drop""#),
            "expected w:framePr with dropCap=drop"
        );
    }

    #[test]
    fn drop_cap_style_omitted_when_profile_disables_drop_cap() {
        // FictionYoungAdult has drop_cap=false.
        let xml = render_styles_xml(FormatProfile::FictionYoungAdult);
        assert!(
            !xml.contains(r#"w:styleId="Drop""#),
            "Drop style should not appear when drop_cap()=false"
        );
    }

    #[test]
    fn scene_break_style_emitted_for_profiles_with_a_glyph() {
        let xml = render_styles_xml(FormatProfile::FictionLiterary);
        assert!(
            xml.contains(r#"w:styleId="SceneBreak""#),
            "expected SceneBreak style for non-Academic profiles"
        );
        assert!(
            xml.contains(r#"<w:jc w:val="center"/>"#),
            "scene-break should be centre-aligned"
        );
    }

    #[test]
    fn scene_break_style_omitted_for_academic() {
        // Academic profile has empty scene_break_glyph.
        let xml = render_styles_xml(FormatProfile::Academic);
        assert!(
            !xml.contains(r#"w:styleId="SceneBreak""#),
            "Academic profile suppresses scene breaks"
        );
    }

    #[test]
    fn line_height_serialises_as_auto_multiple() {
        let xml = render_styles_xml(FormatProfile::FictionTradeStandard); // 1.5
                                                                          // 1.5 × 240 = 360.
        assert!(
            xml.contains(r#"w:line="360" w:lineRule="auto""#),
            "expected line=360 auto: {xml}"
        );
    }
}
