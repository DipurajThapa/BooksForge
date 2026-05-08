//! Generate fixture HTML pairs (preview + EPUB chapter) for the
//! Playwright visual-regression suite (BACKLOG §H6).
//!
//! Walks every `FormatProfile` in the visual-regression matrix and
//! writes two files per profile under
//! `tests/visual-regression/fixtures/<profile>/`:
//!
//!   - `preview.html`    — the editor's HTML preview output wrapped
//!                          in a minimal `<style>` block that mirrors
//!                          the editor's `<ProsePreview>` component.
//!   - `epub-chapter.xhtml` — the same chapter extracted from a
//!                          freshly built EPUB plus its `book.css`,
//!                          inlined for self-contained rendering.
//!
//! Run: `cargo run -p booksforge-export-epub --example visual_fixtures`.
//! The Playwright suite then loads each pair under headless Chromium
//! and pixel-diffs them against committed goldens.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use booksforge_domain::{FormatProfile, Node, NodeKind, NodeStatus};
use booksforge_export::{manuscript_to_html_chapters, ExportProfile, ManuscriptInput};
use booksforge_export_epub::{build_epub_bytes, EpubMetadata, EpubPackageInput, HtmlChapter};
use chrono::{DateTime, TimeZone, Utc};
use ulid::Ulid;

const PROFILES: &[FormatProfile] = &[
    FormatProfile::FictionTradeStandard,
    FormatProfile::FictionLiterary,
    FormatProfile::RomanceHistorical,
    FormatProfile::ThrillerCrime,
];

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().expect("valid timestamp")
}

fn make_node(id: Ulid, parent: Option<Ulid>, kind: NodeKind, title: &str, position: &str) -> Node {
    let now = fixed_now();
    Node {
        id, parent_id: parent, kind,
        title: title.to_owned(),
        position: position.to_owned(),
        status: NodeStatus::Drafting,
        pov: None, beat: None, target_words: None,
        created_at: now, updated_at: now, deleted_at: None,
    }
}

fn stable_ulid(seed: u128) -> Ulid { Ulid(seed) }

fn fixture_manuscript() -> ManuscriptInput {
    let project = stable_ulid(0x01);
    let part1   = stable_ulid(0x10);
    let c1      = stable_ulid(0x11);
    let scenes: &[(Ulid, Ulid, &str, &str, &str)] = &[
        (stable_ulid(0xa1), c1, "Open",  "0|i00000:",
         "<p>The <strong>door</strong> creaked open.</p>\
          <p>She paused — and listened.</p>\
          <hr/>\
          <p>It was the <em>letter</em> she'd been waiting for.</p>\
          <p>For sixteen long months — every birthday, every dawn — she had\
          waited.  Now it lay on the threshold, the wax seal pressed deep\
          into the page like a thumbprint.</p>"),
    ];
    let mut nodes = vec![
        make_node(project, None,           NodeKind::Project, "Test Book", "0|hzzzzz:"),
        make_node(part1,   Some(project),  NodeKind::Part,    "Part 1",    "0|i00000:"),
        make_node(c1,      Some(part1),    NodeKind::Chapter, "One",       "0|i00000:"),
    ];
    let mut texts: BTreeMap<Ulid, String> = BTreeMap::new();
    for (id, parent, title, pos, body) in scenes {
        nodes.push(make_node(*id, Some(*parent), NodeKind::Scene, title, pos));
        texts.insert(*id, (*body).to_owned());
    }
    ManuscriptInput {
        nodes, scene_texts: texts,
        title: "Test Book".to_owned(),
        author: "Jane Doe".to_owned(),
    }
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.ancestors().nth(2).expect("workspace root").to_owned()
}

fn fixtures_dir() -> PathBuf {
    workspace_root().join("tests").join("visual-regression").join("fixtures")
}

fn main() {
    let manuscript = fixture_manuscript();
    let root = fixtures_dir();
    std::fs::create_dir_all(&root).expect("create fixtures dir");

    for profile in PROFILES {
        let profile_dir = root.join(profile.as_str());
        std::fs::create_dir_all(&profile_dir).expect("create profile dir");

        // 1. Editor preview HTML.
        let preview = manuscript_to_html_chapters(&manuscript);
        let chapter = preview.first().expect("at least one chapter");
        let preview_html = wrap_preview(&chapter.html_body, profile);
        std::fs::write(profile_dir.join("preview.html"), preview_html).expect("write preview");

        // 2. EPUB chapter XHTML — build the EPUB, extract chapter-001.xhtml +
        //    book.css, inline the CSS into the chapter so it's self-contained.
        let chapters: Vec<HtmlChapter> = preview.iter()
            .map(|c| HtmlChapter {
                node_id:   c.node_id.to_string(),
                title:     c.title.clone(),
                html_body: c.html_body.clone(),
            })
            .collect();
        let bytes = build_epub_bytes(&EpubPackageInput {
            chapters,
            metadata: EpubMetadata {
                title:       "Test Book".to_owned(),
                authors:     vec!["Jane Doe".to_owned()],
                language:    "en".to_owned(),
                publisher:   None, description: None, isbn: None,
                book_id:     "01HZBOOK000000000000000FIXT".to_owned(),
                dedication: None, epigraph: None, copyright_notice: None,
            },
            profile: ExportProfile::GenericEpub,
            output_path: "/tmp/ignored.epub".to_owned(),
            format_profile: *profile,
            font_bundle_dir: None,
        }).expect("build EPUB");

        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes[..])).expect("open epub zip");
        let chapter_xhtml = read_zip_entry(&mut zip, "OEBPS/text/chapter-001.xhtml");
        let book_css      = read_zip_entry(&mut zip, "OEBPS/styles/book.css");

        let inlined = inline_chapter_with_css(&chapter_xhtml, &book_css);
        std::fs::write(profile_dir.join("epub-chapter.xhtml"), inlined)
            .expect("write epub chapter");

        println!("[fixtures] {} → {}", profile.as_str(), profile_dir.display());
    }
    println!("Done.  Run `pnpm --filter @booksforge/visual-regression test` next.");
}

fn read_zip_entry(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> String {
    let mut entry = zip.by_name(name).expect("entry exists");
    let mut s = String::new();
    entry.read_to_string(&mut s).expect("read entry");
    s
}

/// Inline the EPUB stylesheet into the chapter XHTML so the file is
/// self-contained when loaded via `file://` in Playwright (no need to
/// honour the relative `<link href="../styles/book.css">` reference).
fn inline_chapter_with_css(chapter_xhtml: &str, book_css: &str) -> String {
    chapter_xhtml.replacen(
        r#"<link rel="stylesheet" type="text/css" href="../styles/book.css"/>"#,
        &format!("<style>{book_css}</style>"),
        1,
    )
}

/// Wrap the editor's HTML chapter body in a minimal page that mirrors
/// `<ProsePreview>` — the same body styles the user sees in the
/// editor's preview pane.  Kept intentionally close to the EPUB CSS
/// so the diff isolates rendering drift, not stylistic choice.
fn wrap_preview(html_body: &str, profile: &FormatProfile) -> String {
    // Minimal body styles drawn from FormatProfile so the preview
    // matches what the EPUB would render.  The visual-regression
    // suite then compares the two pages, which differ only by the
    // EPUB's chapter wrapper / fonts-from-disk path.
    let body_family    = profile.body_font_family();
    let heading_family = profile.heading_font_family();
    let body_em        = profile.body_em();
    let line_height    = profile.line_height();
    let para_indent    = profile.paragraph_indent_em();
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <title>preview — {profile_str}</title>
  <style>
    html, body {{ margin: 0; padding: 0; }}
    body {{
      font-family: {body_family};
      font-size:   {body_em};
      line-height: {line_height};
      margin:      2em;
      max-width:   38em;
    }}
    h1, h2, h3, h4 {{ font-family: {heading_family}; }}
    p {{ text-indent: {para_indent}; margin: 0 0 0.4em; }}
    p:first-of-type, h1 + p, h2 + p, h3 + p {{ text-indent: 0; }}
    hr {{ border: none; text-align: center; margin: 1.5em auto; }}
    hr::before {{ content: "{glyph}"; }}
  </style>
</head>
<body>
  {html_body}
</body>
</html>"#,
        profile_str = profile.as_str(),
        glyph = profile.scene_break_glyph(),
    )
}
