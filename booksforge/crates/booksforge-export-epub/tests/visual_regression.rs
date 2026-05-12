#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Visual regression: editor preview HTML vs unzipped EPUB chapter HTML
//! (BACKLOG §H6, Rust-side scaffold).
//!
//! The full pixel-diff harness lives in a Playwright suite that runs
//! after §I1 (accessibility audit) lands the final styled rendering
//! target.  Until then, this test enforces the *content-level*
//! invariant the pixel diff would otherwise catch: every paragraph the
//! editor preview renders for a chapter must appear, byte-for-byte and
//! in order, inside the EPUB's `OEBPS/text/chapter-NNN.xhtml` body.
//!
//! If a future change to the EPUB packager reformats inline marks,
//! reorders paragraphs, drops an empty paragraph, or escapes characters
//! differently from `pm_doc_to_html`, this test fires before the
//! Playwright pixel diff would.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use booksforge_domain::{Node, NodeKind, NodeStatus};
use booksforge_export::{manuscript_to_html_chapters, ManuscriptInput};
use booksforge_export_epub::{build_epub_bytes, EpubMetadata, EpubPackageInput, HtmlChapter};
use chrono::Utc;
use ulid::Ulid;

fn make_node(id: Ulid, parent: Option<Ulid>, kind: NodeKind, title: &str, position: &str) -> Node {
    let now = Utc::now();
    Node {
        id,
        parent_id: parent,
        kind,
        title: title.to_owned(),
        position: position.to_owned(),
        status: NodeStatus::Drafting,
        pov: None,
        beat: None,
        target_words: None,
        synopsis: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

fn stable_ulid(seed: u128) -> Ulid {
    Ulid(seed)
}

/// Smaller fixture than the reproducibility test — we only need a
/// representative sample of inline marks (bold / italic / link).
fn fixture_manuscript() -> ManuscriptInput {
    let project = stable_ulid(0x01);
    let part1 = stable_ulid(0x10);
    let c1 = stable_ulid(0x11);

    let scenes: Vec<(Ulid, Ulid, &str, &str, &str)> = vec![
        (
            stable_ulid(0xa1),
            c1,
            "Open",
            "0|i00000:",
            "<p>The <strong>door</strong> creaked open.</p><p>She paused — and listened.</p>",
        ),
        (
            stable_ulid(0xa2),
            c1,
            "Close",
            "0|j00000:",
            "<p>It was the <em>letter</em> she'd been waiting for.</p>",
        ),
    ];

    let mut nodes = vec![
        make_node(project, None, NodeKind::Project, "Test", "0|hzzzzz:"),
        make_node(part1, Some(project), NodeKind::Part, "Part", "0|i00000:"),
        make_node(c1, Some(part1), NodeKind::Chapter, "One", "0|i00000:"),
    ];
    let mut texts: BTreeMap<Ulid, String> = BTreeMap::new();
    for (id, parent, title, pos, body) in scenes {
        nodes.push(make_node(id, Some(parent), NodeKind::Scene, title, pos));
        texts.insert(id, body.to_owned());
    }
    ManuscriptInput {
        nodes,
        scene_texts: texts,
        title: "T".into(),
        author: "A".into(),
    }
}

/// Pull every `<p>...</p>` block out of an XHTML body string in source order.
/// Naive on purpose: regex would be heavier than this needs to be, and the
/// EPUB chapter bodies we generate are all flat paragraph lists.
fn paragraphs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<p>") {
        let after = &rest[start + 3..];
        let Some(end) = after.find("</p>") else { break };
        out.push(after[..end].to_owned());
        rest = &after[end + 4..];
    }
    out
}

fn extract_chapter_body(zip_bytes: &[u8], chapter_index: usize) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(zip_bytes)).expect("open zip");
    let name = format!("OEBPS/text/chapter-{:03}.xhtml", chapter_index + 1);
    let mut entry = zip.by_name(&name).expect("chapter exists in zip");
    let mut s = String::new();
    entry.read_to_string(&mut s).expect("read chapter xhtml");
    s
}

#[test]
fn epub_chapter_paragraphs_match_editor_preview() {
    let manuscript = fixture_manuscript();

    // Editor-preview HTML chapters (this is the same call site
    // <ProsePreview> in the UI uses for "what will my book look like").
    let preview_chapters = manuscript_to_html_chapters(&manuscript);
    assert!(
        !preview_chapters.is_empty(),
        "fixture must produce at least one chapter"
    );

    // Build the EPUB.
    let chapters: Vec<HtmlChapter> = preview_chapters
        .iter()
        .map(|c| HtmlChapter {
            node_id: c.node_id.to_string(),
            title: c.title.clone(),
            html_body: c.html_body.clone(),
        })
        .collect();
    let bytes = build_epub_bytes(&EpubPackageInput {
        chapters,
        metadata: EpubMetadata {
            title: "T".into(),
            authors: vec!["A".into()],
            language: "en".into(),
            publisher: None,
            description: None,
            isbn: None,
            book_id: "01HZBOOK00000000000000000A".into(),
            dedication: None,
            epigraph: None,
            copyright_notice: None,
        },
        profile: booksforge_export::ExportProfile::GenericEpub,
        output_path: "/tmp/ignored.epub".into(),
        format_profile: booksforge_domain::FormatProfile::FictionTradeStandard,
        font_bundle_dir: None,
    })
    .expect("build");

    // For each editor-preview chapter, every paragraph must appear in
    // the unzipped EPUB chapter body in the same order.  We compare the
    // paragraph *content* (the inner HTML between <p> and </p>) so the
    // EPUB's surrounding XHTML wrapper / chapter title heading don't
    // cause spurious diffs.
    for (i, prev) in preview_chapters.iter().enumerate() {
        let preview_paras = paragraphs(&prev.html_body);
        let epub_xhtml = extract_chapter_body(&bytes, i);
        let epub_paras = paragraphs(&epub_xhtml);

        for (j, para) in preview_paras.iter().enumerate() {
            assert!(
                epub_paras.iter().any(|ep| ep == para),
                "chapter {i}: preview paragraph {j} not found in EPUB chapter body\n\
                 preview: {para:?}\n\
                 epub paragraphs: {epub_paras:#?}",
            );
        }
    }
}

#[test]
fn paragraph_order_is_preserved_across_paths() {
    let manuscript = fixture_manuscript();
    let preview = manuscript_to_html_chapters(&manuscript);
    let chapters: Vec<HtmlChapter> = preview
        .iter()
        .map(|c| HtmlChapter {
            node_id: c.node_id.to_string(),
            title: c.title.clone(),
            html_body: c.html_body.clone(),
        })
        .collect();
    let bytes = build_epub_bytes(&EpubPackageInput {
        chapters,
        metadata: EpubMetadata {
            title: "T".into(),
            authors: vec!["A".into()],
            language: "en".into(),
            publisher: None,
            description: None,
            isbn: None,
            book_id: "01HZBOOK00000000000000000B".into(),
            dedication: None,
            epigraph: None,
            copyright_notice: None,
        },
        profile: booksforge_export::ExportProfile::GenericEpub,
        output_path: "/tmp/ignored.epub".into(),
        format_profile: booksforge_domain::FormatProfile::FictionTradeStandard,
        font_bundle_dir: None,
    })
    .expect("build");

    for (i, prev) in preview.iter().enumerate() {
        let preview_paras = paragraphs(&prev.html_body);
        let epub_paras = paragraphs(&extract_chapter_body(&bytes, i));
        // For every adjacent pair in the preview, that order must hold in the EPUB.
        for w in preview_paras.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let pos_a = epub_paras.iter().position(|p| p == a);
            let pos_b = epub_paras.iter().position(|p| p == b);
            match (pos_a, pos_b) {
                (Some(ia), Some(ib)) => assert!(
                    ia < ib,
                    "chapter {i}: preview ordered {a:?} before {b:?} but EPUB inverted them",
                ),
                _ => panic!("chapter {i}: preview paragraphs missing in EPUB body"),
            }
        }
    }
}
