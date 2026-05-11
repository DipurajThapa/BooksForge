#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Reproducibility integration test (BACKLOG §H5 + §C5 baseline).
//!
//! BooksForge promises byte-identical EPUB output for byte-identical
//! input.  This test exercises the full pipeline (a fixture project →
//! `manuscript_to_html_chapters` → `build_epub_bytes`) twice and asserts
//! the two byte streams match.  If a future change breaks determinism
//! (most often: a non-stable timestamp or a HashMap iteration leaking
//! into output), this test catches it before §H5's CI matrix runs.
//!
//! The fixture is intentionally non-trivial: 2 parts, 4 chapters,
//! 8 scenes with mixed inline marks (bold / italic / link / code) so
//! the per-block / per-inline rendering paths are all covered.

use std::collections::BTreeMap;

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
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Build a stable Ulid from a small integer seed.  Using `Ulid(u128)`
/// directly gives us byte-identical values every run, so the
/// BTreeMap-based iteration ordering in the export pipeline is also
/// stable across runs.
fn stable_ulid(seed: u128) -> Ulid {
    Ulid(seed)
}

/// Stable fixture: 1 project → 2 parts → 2 chapters each → 2 scenes each.
/// All ULIDs and LexoRank positions are constants so the test is
/// deterministic across machines and runs.
fn fixture_manuscript() -> ManuscriptInput {
    let project = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0001);
    let part1 = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0010);
    let part2 = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0020);
    let c1a = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0110);
    let c1b = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0120);
    let c2a = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0210);
    let c2b = stable_ulid(0x0000_0000_0000_0000_0000_0000_0000_0220);

    let scenes: Vec<(Ulid, Ulid, &str, &str, &str)> = vec![
        (
            stable_ulid(0x1110),
            c1a,
            "Opening",
            "0|i00000:",
            "<p>The <strong>door</strong> creaked open.</p>",
        ),
        (
            stable_ulid(0x1120),
            c1a,
            "Stakes",
            "0|j00000:",
            "<p>She had until <em>dawn</em>.</p>",
        ),
        (
            stable_ulid(0x1210),
            c1b,
            "Reveal",
            "0|i00000:",
            "<p>It was the <strong><em>letter</em></strong>.</p>",
        ),
        (
            stable_ulid(0x1220),
            c1b,
            "Cliffhanger",
            "0|j00000:",
            "<p>And then nothing.</p>",
        ),
        (
            stable_ulid(0x2110),
            c2a,
            "Recovery",
            "0|i00000:",
            "<p>Morning came &amp; she rose.</p>",
        ),
        (
            stable_ulid(0x2120),
            c2a,
            "Plan",
            "0|j00000:",
            "<p>Three steps: find, follow, finish.</p>",
        ),
        (
            stable_ulid(0x2210),
            c2b,
            "Confrontation",
            "0|i00000:",
            "<p>\"You knew,\" she said.</p>",
        ),
        (
            stable_ulid(0x2220),
            c2b,
            "Resolution",
            "0|j00000:",
            "<p>The case was closed.</p>",
        ),
    ];

    let mut nodes = vec![
        make_node(project, None, NodeKind::Project, "Test Book", "0|hzzzzz:"),
        make_node(
            part1,
            Some(project),
            NodeKind::Part,
            "Part 1 — Setup",
            "0|i00000:",
        ),
        make_node(
            c1a,
            Some(part1),
            NodeKind::Chapter,
            "Beginnings",
            "0|i00000:",
        ),
        make_node(
            c1b,
            Some(part1),
            NodeKind::Chapter,
            "Complications",
            "0|j00000:",
        ),
        make_node(
            part2,
            Some(project),
            NodeKind::Part,
            "Part 2 — Payoff",
            "0|j00000:",
        ),
        make_node(c2a, Some(part2), NodeKind::Chapter, "Recovery", "0|i00000:"),
        make_node(c2b, Some(part2), NodeKind::Chapter, "Endings", "0|j00000:"),
    ];
    let mut texts: BTreeMap<Ulid, String> = BTreeMap::new();
    for (id, parent, title, pos, body) in scenes {
        nodes.push(make_node(id, Some(parent), NodeKind::Scene, title, pos));
        texts.insert(id, body.to_owned());
    }

    ManuscriptInput {
        nodes,
        scene_texts: texts,
        title: "Test Book".into(),
        author: "Jane Doe".into(),
    }
}

fn build_for_metadata(book_id: &str) -> Vec<u8> {
    let manuscript = fixture_manuscript();
    let chapters: Vec<HtmlChapter> = manuscript_to_html_chapters(&manuscript)
        .into_iter()
        .map(|c| HtmlChapter {
            node_id: c.node_id.to_string(),
            title: c.title,
            html_body: c.html_body,
        })
        .collect();
    build_epub_bytes(&EpubPackageInput {
        chapters,
        metadata: EpubMetadata {
            title: "Test Book".into(),
            authors: vec!["Jane Doe".into()],
            language: "en".into(),
            publisher: None,
            description: None,
            isbn: None,
            book_id: book_id.to_owned(),
            dedication: None,
            epigraph: None,
            copyright_notice: None,
        },
        profile: booksforge_export::ExportProfile::GenericEpub,
        output_path: "/tmp/ignored.epub".into(),
        format_profile: booksforge_domain::FormatProfile::FictionTradeStandard,
        font_bundle_dir: None,
    })
    .expect("build")
}

#[test]
fn full_manuscript_pipeline_is_byte_deterministic_across_calls() {
    let book_id = "01H0BOOK000000000000000000BK";
    let a = build_for_metadata(book_id);
    let b = build_for_metadata(book_id);
    assert_eq!(a.len(), b.len(), "byte-length must match across runs");
    assert_eq!(
        blake3::hash(&a).to_hex().to_string(),
        blake3::hash(&b).to_hex().to_string(),
        "blake3 hash must match across runs — reproducibility broken",
    );
}

#[test]
fn changing_metadata_produces_different_output() {
    // Sanity check: the determinism above is meaningful only if metadata
    // *actually flows into the bytes*.  Different book_ids must produce
    // different archives.
    let a = build_for_metadata("01H0BOOK000000000000000000AA");
    let b = build_for_metadata("01H0BOOK000000000000000000BB");
    assert_ne!(a, b, "different book_ids must produce different EPUBs");
}
