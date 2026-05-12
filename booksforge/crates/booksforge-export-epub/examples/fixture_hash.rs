#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unimplemented
)]

//! Cross-host reproducibility probe (BACKLOG §C5).
//!
//! Builds a canned EPUB fixture and prints its blake3 hash to stdout.
//! Run on every CI runner; the `compare-hashes` job in `.github/workflows/ci.yml`
//! diffs the outputs and fails the build if any platform disagrees.
//!
//! Keep this fixture **identical** to the one in
//! `tests/reproducibility.rs` — that's the per-host invariant; this
//! binary is the cross-host invariant.  If you change one, change both.

use std::collections::BTreeMap;

use booksforge_domain::{FormatProfile, Node, NodeKind, NodeStatus};
use booksforge_export::{manuscript_to_html_chapters, ExportProfile, ManuscriptInput};
use booksforge_export_epub::{build_epub_bytes, EpubMetadata, EpubPackageInput, HtmlChapter};
use chrono::{DateTime, TimeZone, Utc};
use ulid::Ulid;

fn fixed_now() -> DateTime<Utc> {
    // The integration test uses `Utc::now()`, but timestamps don't
    // flow into the EPUB bytes (the packager pins entry mtime).  We
    // pick a fixed instant here just to make the fixture explicit.
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .single()
        .expect("valid timestamp")
}

fn make_node(id: Ulid, parent: Option<Ulid>, kind: NodeKind, title: &str, position: &str) -> Node {
    let now = fixed_now();
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

fn main() {
    let project = stable_ulid(0x01);
    let part1 = stable_ulid(0x10);
    let c1 = stable_ulid(0x11);

    let scenes: &[(Ulid, Ulid, &str, &str, &str)] = &[
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
        nodes.push(make_node(*id, Some(*parent), NodeKind::Scene, title, pos));
        texts.insert(*id, (*body).to_owned());
    }
    let manuscript = ManuscriptInput {
        nodes,
        scene_texts: texts,
        title: "Test Book".to_owned(),
        author: "Jane Doe".to_owned(),
    };

    let preview = manuscript_to_html_chapters(&manuscript);
    let chapters: Vec<HtmlChapter> = preview
        .into_iter()
        .map(|c| HtmlChapter {
            node_id: c.node_id.to_string(),
            title: c.title,
            html_body: c.html_body,
        })
        .collect();

    let bytes = build_epub_bytes(&EpubPackageInput {
        chapters,
        metadata: EpubMetadata {
            title: "Test Book".to_owned(),
            authors: vec!["Jane Doe".to_owned()],
            language: "en".to_owned(),
            publisher: None,
            description: None,
            isbn: None,
            book_id: "01HZBOOK000000000000000XHASH".to_owned(),
            dedication: None,
            epigraph: None,
            copyright_notice: None,
        },
        profile: ExportProfile::GenericEpub,
        output_path: "/tmp/ignored.epub".to_owned(),
        format_profile: FormatProfile::FictionTradeStandard,
        font_bundle_dir: None,
    })
    .expect("build fixture");

    // Hash to stdout — the CI workflow captures this as an artefact.
    println!("{}", blake3::hash(&bytes).to_hex());
}
