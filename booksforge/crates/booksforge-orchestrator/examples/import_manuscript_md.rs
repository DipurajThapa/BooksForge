//! Import a multi-chapter Markdown manuscript into an existing
//! `*.booksforge/` bundle, so the desktop app can open and read it.
//!
//! Background: `multi_chapter_run.rs` writes its prose output to
//! `book-output/multi-chapter-runs/<timestamp>/manuscript.md` because
//! its bundle is a `tempfile::tempdir()` that gets deleted on exit.
//! That left a workflow gap — the user opens the desktop app, sees
//! every recent project as empty, and has no way to bring the
//! generated prose back into a project they can actually read.
//!
//! This binary closes that gap. It:
//!   1. Opens an existing bundle (must NOT be open in the desktop app
//!      at the same time — close it there first).
//!   2. Parses the Markdown by `# Chapter N` and `## Scene Title`.
//!   3. Inserts a `Chapter` Node under the project root for each
//!      chapter, then a `Scene` Node under each chapter.
//!   4. Converts each scene's body paragraphs to ProseMirror JSON and
//!      writes a `scene_content` row with blake3 hash + word_count.
//!
//! Usage:
//!   cargo run -p booksforge-orchestrator --example import_manuscript_md \
//!       -- <bundle_path> <manuscript_md_path>
//!
//! Safety: idempotent only on the first run. Re-running will append a
//! second copy of every chapter/scene rather than dedupe — close the
//! desktop app, run once, then reopen.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines
)]

use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use booksforge_domain::{Node, NodeKind, NodeStatus, SceneContent};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use serde_json::json;
use ulid::Ulid;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: {} <bundle_path> <manuscript_md_path>",
            args.first()
                .map(String::as_str)
                .unwrap_or("import_manuscript_md")
        );
        eprintln!("\nexample:");
        eprintln!(
            "  cargo run -p booksforge-orchestrator --example import_manuscript_md -- \\\n\
             '/Users/you/Work/MyBooks/My Book.booksforge' \\\n\
             '/Users/you/Work/AIProjects/BooksForge/book-output/multi-chapter-runs/20260510-174553/manuscript.md'"
        );
        return ExitCode::from(2);
    }
    let bundle_path = Path::new(&args[1]);
    let md_path = Path::new(&args[2]);

    if !bundle_path.is_dir() {
        eprintln!("bundle_path is not a directory: {}", bundle_path.display());
        return ExitCode::from(2);
    }
    let db_path = bundle_path.join("project.db");
    if !db_path.is_file() {
        eprintln!("no project.db at {}", db_path.display());
        return ExitCode::from(2);
    }

    let md = match std::fs::read_to_string(md_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not read {}: {e}", md_path.display());
            return ExitCode::from(2);
        }
    };

    let parsed = parse_markdown(&md);
    if parsed.chapters.is_empty() {
        eprintln!(
            "no chapters detected in {}. expected `# Chapter N` headings.",
            md_path.display()
        );
        return ExitCode::from(2);
    }

    println!(
        "parsed {} chapter(s) and {} scene(s) from {}",
        parsed.chapters.len(),
        parsed
            .chapters
            .iter()
            .map(|c| c.scenes.len())
            .sum::<usize>(),
        md_path.display(),
    );

    // Open the bundle's SQLite. We deliberately do NOT acquire the
    // bundle lock — the user is expected to have closed the project
    // in the desktop app first. The `manifest.toml` is left untouched.
    let pool = match open_pool(&db_path).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("open_pool failed: {e}");
            eprintln!("hint: close the project in the desktop app first.");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = run_migrations(&pool).await {
        eprintln!("run_migrations failed: {e}");
        return ExitCode::from(1);
    }
    let storage: Arc<dyn StorageRepository> = Arc::new(SqliteStorage::new(pool));

    // Find the project root.
    let nodes = match storage.list_nodes().await {
        Ok(ns) => ns,
        Err(e) => {
            eprintln!("list_nodes failed: {e}");
            return ExitCode::from(1);
        }
    };
    let root = nodes.into_iter().find(|n| n.kind == NodeKind::Project);
    let root_id = match root {
        Some(r) => r.id,
        None => {
            eprintln!("no project root node in bundle — cannot import.");
            return ExitCode::from(1);
        }
    };

    let now = Utc::now();
    let mut total_scenes = 0usize;
    let mut total_words = 0usize;

    for (chap_idx, chap) in parsed.chapters.iter().enumerate() {
        let chap_id = Ulid::new();
        let chap_pos = position_for_index(chap_idx);
        let chap_node = Node {
            id: chap_id,
            parent_id: Some(root_id),
            kind: NodeKind::Chapter,
            title: chap.title.clone(),
            position: chap_pos,
            status: NodeStatus::Drafting,
            pov: None,
            beat: None,
            target_words: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        if let Err(e) = storage.insert_node(&chap_node).await {
            eprintln!("insert chapter '{}': {e}", chap.title);
            return ExitCode::from(1);
        }

        for (sc_idx, sc) in chap.scenes.iter().enumerate() {
            let sc_id = Ulid::new();
            let sc_pos = position_for_index(sc_idx);
            let sc_node = Node {
                id: sc_id,
                parent_id: Some(chap_id),
                kind: NodeKind::Scene,
                title: sc.title.clone(),
                position: sc_pos,
                status: NodeStatus::Drafting,
                pov: None,
                beat: None,
                target_words: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            };
            if let Err(e) = storage.insert_node(&sc_node).await {
                eprintln!("insert scene '{}': {e}", sc.title);
                return ExitCode::from(1);
            }

            let pm_doc = paragraphs_to_pm_doc(&sc.paragraphs);
            let pm_str = serde_json::to_string(&pm_doc).expect("pm_doc serialise");
            let hash = blake3::hash(pm_str.as_bytes()).to_hex().to_string();
            let word_count: u32 = sc
                .paragraphs
                .iter()
                .map(|p| p.split_whitespace().count() as u32)
                .sum();
            let char_count: u32 = sc.paragraphs.iter().map(|p| p.chars().count() as u32).sum();

            let content = SceneContent {
                node_id: sc_id,
                pm_doc,
                word_count,
                char_count,
                hash,
                updated_at: now,
            };
            if let Err(e) = storage.save_scene(&content).await {
                eprintln!("save_scene '{}': {e}", sc.title);
                return ExitCode::from(1);
            }
            total_scenes += 1;
            total_words += word_count as usize;
            println!(
                "  imported  {:>2}.{:>2}  {:<32}  {:>5} words",
                chap_idx + 1,
                sc_idx + 1,
                truncate(&sc.title, 32),
                word_count
            );
        }
    }

    println!(
        "\n✓ imported {} chapter(s), {} scene(s), {} words into\n  {}",
        parsed.chapters.len(),
        total_scenes,
        total_words,
        bundle_path.display()
    );
    println!("\nReopen the bundle in the desktop app — the binder should now show\nthe imported chapters and scenes with their prose.");
    ExitCode::SUCCESS
}

// ── Markdown parser ─────────────────────────────────────────────────────────

#[derive(Debug)]
struct ParsedManuscript {
    chapters: Vec<ParsedChapter>,
}

#[derive(Debug)]
struct ParsedChapter {
    title: String,
    scenes: Vec<ParsedScene>,
}

#[derive(Debug)]
struct ParsedScene {
    title: String,
    paragraphs: Vec<String>,
}

fn parse_markdown(md: &str) -> ParsedManuscript {
    let mut chapters: Vec<ParsedChapter> = Vec::new();
    let mut current_chapter: Option<ParsedChapter> = None;
    let mut current_scene: Option<ParsedScene> = None;
    let mut paragraph_buf: Vec<String> = Vec::new();

    fn flush_paragraph(buf: &mut Vec<String>, scene: &mut Option<ParsedScene>) {
        if buf.is_empty() {
            return;
        }
        let text = buf.join(" ").trim().to_owned();
        buf.clear();
        if text.is_empty() {
            return;
        }
        if let Some(sc) = scene.as_mut() {
            sc.paragraphs.push(text);
        }
        // Lines outside any scene (e.g. the manuscript title block) are
        // intentionally dropped — they are metadata, not prose.
    }

    fn flush_scene(scene: &mut Option<ParsedScene>, chapter: &mut Option<ParsedChapter>) {
        if let Some(sc) = scene.take() {
            if let Some(ch) = chapter.as_mut() {
                ch.scenes.push(sc);
            }
        }
    }

    for line in md.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("# ") {
            // Chapter heading. Push pending scene → chapter, then start fresh.
            flush_paragraph(&mut paragraph_buf, &mut current_scene);
            flush_scene(&mut current_scene, &mut current_chapter);
            // The very first `# ` in the file is often the manuscript
            // title (e.g. "# Incomplete Curse — 2-chapter draft"). Skip
            // it if the heading isn't recognisable as a chapter heading.
            let is_chapter_heading = rest.starts_with("Chapter")
                || rest.starts_with("CHAPTER")
                || rest.starts_with("Ch ")
                || rest.starts_with("Ch. ");
            if !is_chapter_heading && current_chapter.is_none() {
                // Manuscript-title line — skip silently.
                continue;
            }
            // Otherwise: close the current chapter and open a new one.
            if let Some(ch) = current_chapter.take() {
                chapters.push(ch);
            }
            current_chapter = Some(ParsedChapter {
                title: rest.to_owned(),
                scenes: Vec::new(),
            });
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("## ") {
            flush_paragraph(&mut paragraph_buf, &mut current_scene);
            flush_scene(&mut current_scene, &mut current_chapter);
            // A `## ` outside any chapter is either a top-level section
            // (drop it) or the user's manuscript has no chapter
            // headings — wrap it in a synthetic Chapter 1.
            if current_chapter.is_none() {
                current_chapter = Some(ParsedChapter {
                    title: "Chapter 1".to_owned(),
                    scenes: Vec::new(),
                });
            }
            current_scene = Some(ParsedScene {
                title: rest.to_owned(),
                paragraphs: Vec::new(),
            });
            continue;
        }

        // Italic-only metadata line ("_Generated by …_") — skip when at
        // the top of the file (no scene open yet).
        if current_scene.is_none()
            && trimmed.starts_with('_')
            && trimmed.ends_with('_')
            && trimmed.len() > 2
        {
            continue;
        }

        if trimmed.is_empty() {
            // Blank line ends the current paragraph.
            flush_paragraph(&mut paragraph_buf, &mut current_scene);
            continue;
        }

        // Body line — accumulate into the current paragraph buffer.
        paragraph_buf.push(trimmed.to_owned());
    }

    // Drain trailing buffers.
    flush_paragraph(&mut paragraph_buf, &mut current_scene);
    flush_scene(&mut current_scene, &mut current_chapter);
    if let Some(ch) = current_chapter.take() {
        chapters.push(ch);
    }

    // Drop any chapter that ended up with no scenes (e.g. the
    // manuscript-title line that slipped past).
    chapters.retain(|ch| !ch.scenes.is_empty());

    ParsedManuscript { chapters }
}

// ── ProseMirror builder ─────────────────────────────────────────────────────

fn paragraphs_to_pm_doc(paragraphs: &[String]) -> serde_json::Value {
    let content: Vec<serde_json::Value> = paragraphs
        .iter()
        .map(|p| {
            json!({
                "type": "paragraph",
                "content": [{
                    "type": "text",
                    "text": p,
                }],
            })
        })
        .collect();
    json!({
        "type": "doc",
        "content": if content.is_empty() {
            // ProseMirror requires at least one block — empty paragraph fallback.
            vec![json!({ "type": "paragraph" })]
        } else {
            content
        },
    })
}

// ── Position-key helper ─────────────────────────────────────────────────────

/// Build a position key that sorts in insertion order. The schema
/// accepts any string; we use a fixed-width zero-padded decimal suffix
/// so lexicographic sort matches numeric sort. Avoids pulling in
/// LexoRank for a one-shot importer.
fn position_for_index(idx: usize) -> String {
    format!("0|{idx:08}:")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chapters_and_scenes() {
        let md = "\
# Some Title — 2-chapter draft

_Generated by BooksForge_

# Chapter 1

## Scene A

Para one.

Para two.

## Scene B

Only paragraph.

# Chapter 2

## Scene C

Final.
";
        let p = parse_markdown(md);
        assert_eq!(p.chapters.len(), 2);
        assert_eq!(p.chapters[0].title, "Chapter 1");
        assert_eq!(p.chapters[0].scenes.len(), 2);
        assert_eq!(p.chapters[0].scenes[0].title, "Scene A");
        assert_eq!(p.chapters[0].scenes[0].paragraphs.len(), 2);
        assert_eq!(p.chapters[1].scenes[0].title, "Scene C");
    }

    #[test]
    fn paragraph_lines_join_with_space() {
        let md = "\
# Chapter 1

## Scene A

This sentence
spans two lines.

Another paragraph.
";
        let p = parse_markdown(md);
        assert_eq!(
            p.chapters[0].scenes[0].paragraphs[0],
            "This sentence spans two lines."
        );
    }

    #[test]
    fn position_for_index_orders_correctly() {
        let p0 = position_for_index(0);
        let p1 = position_for_index(1);
        let p25 = position_for_index(25);
        let p26 = position_for_index(26);
        assert!(p0 < p1);
        assert!(p1 < p25);
        assert!(p25 < p26);
    }

    #[test]
    fn pm_doc_has_paragraph_blocks() {
        let doc = paragraphs_to_pm_doc(&["one".to_owned(), "two".to_owned()]);
        let content = doc["content"].as_array().expect("content array");
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "paragraph");
    }
}
