//! Export pipeline (Layer 3 — pure logic).
//!
//! M0 ships the **Markdown** profile only.  The full canonical-HTML pipeline
//! that feeds EPUB-3 / DOCX / PDF lands in M5.  Markdown is the smallest
//! useful export format: it has no sidecar dependency and round-trips cleanly
//! through any external editor (Pandoc, Word import, GitHub preview).
//!
//! All functions in this crate are **pure**.  Layer-4 callers compose:
//!   1. `list_nodes` from storage in document order (LexoRank-sorted).
//!   2. `load_scene` for each leaf node that has prose content.
//!   3. `manuscript_to_markdown(...)` to render the whole book.
//!   4. Write the resulting bytes to `exports/<id>.md` and persist an
//!      `exports` row.
//!
//! Future profiles (DOCX, EPUB-3, PDF) will reuse the same `ManuscriptInput`
//! shape so this crate stays the single source of export truth.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeMap;

use booksforge_domain::{Node, NodeKind};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

// Re-export the canonical ExportProfile so legacy callers (the stub epub
// crate) keep working — domain owns the type.
pub use booksforge_domain::ExportProfile;

/// In-memory result of an export run.  Distinct from `ExportRecord` (the
/// persisted ledger row) — this is what the renderer hands back, the
/// caller persists a record after writing the bytes to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOutcome {
    pub profile: ExportProfile,
    pub output_path: String,
    /// blake3 hex of the rendered bytes (drives reproducibility checks).
    pub hash: String,
}

// ── Public types ──────────────────────────────────────────────────────────────

/// One leaf scene's plain-text body, paired with its node id.  The caller
/// is responsible for converting the stored `pm_doc` JSON into plain text
/// (or providing an empty body for a never-saved scene).
#[derive(Debug, Clone)]
pub struct SceneBody {
    pub node_id: Ulid,
    pub text: String,
}

/// Everything `manuscript_to_markdown` needs.  Decoupled from storage so the
/// crate stays pure logic.
#[derive(Debug, Clone)]
pub struct ManuscriptInput {
    /// All non-deleted nodes in any order.  The renderer sorts internally.
    pub nodes: Vec<Node>,
    /// Scene-content text bodies, keyed by `node_id`.  Missing entries are
    /// rendered as a placeholder line so the structure stays visible.
    pub scene_texts: BTreeMap<Ulid, String>,
    /// Title shown as the H1.  Usually the project's `manifest.title`.
    pub title: String,
    /// Author byline shown beneath the title.  Empty string suppresses it.
    pub author: String,
}

/// Counters returned alongside the rendered text so callers can show the
/// user / write to the `exports` ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStats {
    pub bytes: u64,
    pub scene_count: u32,
    pub chapter_count: u32,
    pub part_count: u32,
    pub word_count: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("export failed: {message}")]
    Failed { message: String },
    #[error("sidecar '{binary}' not found or not executable")]
    SidecarMissing { binary: String },
}

// ── pm_doc → plain text ───────────────────────────────────────────────────────

/// Best-effort conversion of a ProseMirror JSON document to Markdown-flavoured
/// plaintext.  Recognises:
///   - paragraph blocks
///   - heading blocks (`level` 1–6)
///   - bullet_list / ordered_list / list_item blocks
///   - blockquote
///   - code_block
///   - hard_break / inline marks (bold, italic, code)
///
/// Anything we don't recognise is rendered as its inline-text concatenation
/// — the user keeps their words even if a fancy node type slips through.
pub fn pm_doc_to_markdown(pm_doc: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(blocks) = pm_doc.get("content").and_then(|v| v.as_array()) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n");
            }
            render_block(block, &mut out, 0);
        }
    }
    out
}

// "paragraph" is documented explicitly even though its body matches
// the `_` wildcard — keeping it visible at the top of the match is
// clearer than relying on the catch-all to render paragraphs.
#[allow(clippy::match_same_arms)]
fn render_block(node: &serde_json::Value, out: &mut String, list_indent: usize) {
    let kind = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match kind {
        "paragraph" => render_inlines(node, out),
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .clamp(1, 6) as usize;
            for _ in 0..level {
                out.push('#');
            }
            out.push(' ');
            render_inlines(node, out);
        }
        "blockquote" => {
            let mut inner = String::new();
            if let Some(children) = node.get("content").and_then(|v| v.as_array()) {
                for (i, c) in children.iter().enumerate() {
                    if i > 0 {
                        inner.push_str("\n\n");
                    }
                    render_block(c, &mut inner, list_indent);
                }
            }
            for line in inner.lines() {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
            // Trim trailing newline; outer joiner adds the paragraph break.
            while out.ends_with('\n') {
                out.pop();
            }
        }
        "code_block" => {
            out.push_str("```\n");
            render_inlines(node, out);
            out.push_str("\n```");
        }
        "bullet_list" | "ordered_list" => {
            let ordered = kind == "ordered_list";
            if let Some(items) = node.get("content").and_then(|v| v.as_array()) {
                for (i, item) in items.iter().enumerate() {
                    for _ in 0..list_indent {
                        out.push_str("  ");
                    }
                    if ordered {
                        out.push_str(&format!("{}. ", i + 1));
                    } else {
                        out.push_str("- ");
                    }
                    if let Some(item_blocks) = item.get("content").and_then(|v| v.as_array()) {
                        for (j, sub) in item_blocks.iter().enumerate() {
                            if j > 0 {
                                out.push('\n');
                                for _ in 0..(list_indent + 1) {
                                    out.push_str("  ");
                                }
                            }
                            render_block(sub, out, list_indent + 1);
                        }
                    }
                    if i + 1 < items.len() {
                        out.push('\n');
                    }
                }
            }
        }
        "horizontal_rule" => out.push_str("---"),
        _ => render_inlines(node, out),
    }
}

fn render_inlines(node: &serde_json::Value, out: &mut String) {
    let Some(content) = node.get("content").and_then(|v| v.as_array()) else {
        return;
    };
    for inline in content {
        let t = inline.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match t {
            "text" => {
                let text = inline.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let marks = inline.get("marks").and_then(|v| v.as_array());
                let (bold, italic, code, link) = marks
                    .map(|ms| {
                        let mut bold = false;
                        let mut italic = false;
                        let mut code = false;
                        let mut link: Option<String> = None;
                        for m in ms {
                            match m.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                                "bold" | "strong" => bold = true,
                                "italic" | "em" => italic = true,
                                "code" => code = true,
                                "link" => {
                                    if let Some(href) = m
                                        .get("attrs")
                                        .and_then(|a| a.get("href"))
                                        .and_then(|v| v.as_str())
                                    {
                                        link = Some(href.to_owned());
                                    }
                                }
                                _ => {}
                            }
                        }
                        (bold, italic, code, link)
                    })
                    .unwrap_or((false, false, false, None));

                let mut s = text.to_owned();
                if code {
                    s = format!("`{s}`");
                }
                if bold {
                    s = format!("**{s}**");
                }
                if italic {
                    s = format!("*{s}*");
                }
                if let Some(href) = link {
                    s = format!("[{s}]({href})");
                }
                out.push_str(&s);
            }
            "hard_break" => out.push_str("  \n"),
            _ => {
                // Unknown inline — recurse to preserve text content.
                render_inlines(inline, out);
            }
        }
    }
}

/// Wordcount on a rendered string (whitespace-separated tokens).
pub fn count_words(s: &str) -> u32 {
    s.split_whitespace().count() as u32
}

// ── Tree traversal in document order ──────────────────────────────────────────

fn build_children_map(nodes: &[Node]) -> BTreeMap<Option<Ulid>, Vec<&Node>> {
    let mut by_parent: BTreeMap<Option<Ulid>, Vec<&Node>> = BTreeMap::new();
    for n in nodes {
        by_parent.entry(n.parent_id).or_default().push(n);
    }
    // Sibling order = LexoRank position.
    for v in by_parent.values_mut() {
        v.sort_by(|a, b| a.position.cmp(&b.position));
    }
    by_parent
}

// ── Manuscript renderer ───────────────────────────────────────────────────────

/// Render a whole manuscript as Markdown.
///
/// Output shape:
///   `# {title}`
///   `*by {author}*` (omitted if author is blank)
///   For each Part:    `## {part.title}`
///   For each Chapter: `### Chapter N — {chapter.title}`
///   For each Scene:   the prose body (already-formatted Markdown).
///
/// FrontMatter / BackMatter nodes are emitted verbatim in document order.
/// Empty scenes get a `_(empty scene)_` placeholder so the structure is
/// still legible.
pub fn manuscript_to_markdown(input: &ManuscriptInput) -> (String, ExportStats) {
    let mut out = String::new();
    out.push_str(&format!("# {}\n", input.title.trim()));
    if !input.author.trim().is_empty() {
        out.push_str(&format!("\n*by {}*\n", input.author.trim()));
    }

    let by_parent = build_children_map(&input.nodes);
    let project_node = input.nodes.iter().find(|n| n.kind == NodeKind::Project);

    let mut stats = ExportStats {
        bytes: 0,
        scene_count: 0,
        chapter_count: 0,
        part_count: 0,
        word_count: 0,
    };

    // Render top-level children.  The root is the Project node when present;
    // otherwise we fall back to anything with `parent_id == None`.
    let top_parent = project_node.map(|p| Some(p.id)).unwrap_or(None);
    let top_children: Vec<&Node> = by_parent.get(&top_parent).cloned().unwrap_or_default();

    let mut chapter_counter: u32 = 0;

    for child in &top_children {
        render_node_recursive(
            child,
            &by_parent,
            input,
            &mut out,
            &mut stats,
            &mut chapter_counter,
            2,
        );
    }

    stats.bytes = out.len() as u64;
    (out, stats)
}

#[allow(clippy::too_many_arguments)]
fn render_node_recursive(
    node: &Node,
    by_parent: &BTreeMap<Option<Ulid>, Vec<&Node>>,
    input: &ManuscriptInput,
    out: &mut String,
    stats: &mut ExportStats,
    chapter_counter: &mut u32,
    heading_level: usize,
) {
    let title = if node.title.trim().is_empty() {
        "(untitled)"
    } else {
        node.title.trim()
    };

    match node.kind {
        NodeKind::Part => {
            stats.part_count += 1;
            out.push_str("\n\n");
            for _ in 0..heading_level {
                out.push('#');
            }
            out.push_str(&format!(" {title}"));
        }
        NodeKind::Chapter => {
            stats.chapter_count += 1;
            *chapter_counter += 1;
            out.push_str("\n\n");
            for _ in 0..heading_level {
                out.push('#');
            }
            out.push_str(&format!(" Chapter {} — {title}", *chapter_counter));
        }
        NodeKind::Scene => {
            stats.scene_count += 1;
            let body = input.scene_texts.get(&node.id);
            out.push_str("\n\n");
            match body {
                Some(text) if !text.trim().is_empty() => {
                    let trimmed = text.trim();
                    stats.word_count += count_words(trimmed);
                    out.push_str(trimmed);
                }
                _ => out.push_str("_(empty scene)_"),
            }
            return; // scenes have no children
        }
        NodeKind::FrontMatter | NodeKind::BackMatter => {
            let body = input.scene_texts.get(&node.id);
            out.push_str("\n\n");
            for _ in 0..heading_level {
                out.push('#');
            }
            out.push_str(&format!(" {title}"));
            if let Some(text) = body {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    out.push_str("\n\n");
                    stats.word_count += count_words(trimmed);
                    out.push_str(trimmed);
                }
            }
            return;
        }
        NodeKind::Project => {
            // Already emitted as the H1 — recurse only into children.
        }
    }

    if let Some(children) = by_parent.get(&Some(node.id)) {
        let next_level = (heading_level + 1).min(6);
        for child in children {
            render_node_recursive(
                child,
                by_parent,
                input,
                out,
                stats,
                chapter_counter,
                next_level,
            );
        }
    }
}

// ── pm_doc → HTML (EPUB body) ────────────────────────────────────────────────

/// Best-effort conversion of a ProseMirror JSON document to HTML
/// fragment.  Recognises the same node types as `pm_doc_to_markdown`.
/// Output is **fragment-level** (no `<html>` / `<body>` envelope) — the
/// EPUB packager wraps each chapter in an XHTML envelope.
pub fn pm_doc_to_html(pm_doc: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(blocks) = pm_doc.get("content").and_then(|v| v.as_array()) {
        for block in blocks {
            render_block_html(block, &mut out);
            out.push('\n');
        }
    }
    out
}

fn render_block_html(node: &serde_json::Value, out: &mut String) {
    let kind = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match kind {
        "paragraph" => {
            out.push_str("<p>");
            render_inlines_html(node, out);
            out.push_str("</p>");
        }
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .clamp(1, 6) as u32;
            out.push_str(&format!("<h{level}>"));
            render_inlines_html(node, out);
            out.push_str(&format!("</h{level}>"));
        }
        "blockquote" => {
            out.push_str("<blockquote>");
            if let Some(children) = node.get("content").and_then(|v| v.as_array()) {
                for c in children {
                    render_block_html(c, out);
                }
            }
            out.push_str("</blockquote>");
        }
        "code_block" => {
            out.push_str("<pre><code>");
            // For code blocks we want the raw text, not formatted inlines.
            if let Some(content) = node.get("content").and_then(|v| v.as_array()) {
                for inline in content {
                    if let Some(t) = inline.get("text").and_then(|v| v.as_str()) {
                        out.push_str(&html_escape(t));
                    }
                }
            }
            out.push_str("</code></pre>");
        }
        "bullet_list" | "ordered_list" => {
            let tag = if kind == "ordered_list" { "ol" } else { "ul" };
            out.push_str(&format!("<{tag}>"));
            if let Some(items) = node.get("content").and_then(|v| v.as_array()) {
                for item in items {
                    out.push_str("<li>");
                    if let Some(item_blocks) = item.get("content").and_then(|v| v.as_array()) {
                        for sub in item_blocks {
                            render_block_html(sub, out);
                        }
                    }
                    out.push_str("</li>");
                }
            }
            out.push_str(&format!("</{tag}>"));
        }
        "horizontal_rule" => out.push_str("<hr/>"),
        _ => {
            // Unknown block — wrap inline text in a paragraph so we keep
            // the content rather than dropping it.
            out.push_str("<p>");
            render_inlines_html(node, out);
            out.push_str("</p>");
        }
    }
}

fn render_inlines_html(node: &serde_json::Value, out: &mut String) {
    let Some(content) = node.get("content").and_then(|v| v.as_array()) else {
        return;
    };
    for inline in content {
        let t = inline.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match t {
            "text" => {
                let text = inline.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let marks = inline.get("marks").and_then(|v| v.as_array());
                let (bold, italic, code, link) = marks
                    .map(|ms| {
                        let mut bold = false;
                        let mut italic = false;
                        let mut code = false;
                        let mut link: Option<String> = None;
                        for m in ms {
                            match m.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                                "bold" | "strong" => bold = true,
                                "italic" | "em" => italic = true,
                                "code" => code = true,
                                "link" => {
                                    if let Some(href) = m
                                        .get("attrs")
                                        .and_then(|a| a.get("href"))
                                        .and_then(|v| v.as_str())
                                    {
                                        link = Some(href.to_owned());
                                    }
                                }
                                _ => {}
                            }
                        }
                        (bold, italic, code, link)
                    })
                    .unwrap_or((false, false, false, None));

                let mut s = html_escape(text);
                if code {
                    s = format!("<code>{s}</code>");
                }
                if bold {
                    s = format!("<strong>{s}</strong>");
                }
                if italic {
                    s = format!("<em>{s}</em>");
                }
                if let Some(href) = link {
                    s = format!("<a href=\"{}\">{s}</a>", html_escape(&href));
                }
                out.push_str(&s);
            }
            "hard_break" => out.push_str("<br/>"),
            _ => render_inlines_html(inline, out),
        }
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

// ── Manuscript → HTML chapter array (for EPUB) ───────────────────────────────

/// One chapter's HTML body, suitable for the EPUB packager.  The `node_id`
/// is the chapter node's ULID; `title` is taken from the node title.
#[derive(Debug, Clone)]
pub struct HtmlChapter {
    pub node_id: Ulid,
    pub title: String,
    pub html_body: String,
}

/// Group an entire manuscript into per-chapter HTML for EPUB packaging.
/// Each top-level Chapter node becomes one `HtmlChapter` whose body
/// concatenates every Scene under it (in LexoRank order) plus a leading
/// `<h1>` derived from the chapter title.  Scenes inside Parts are
/// attributed to the chapter that contains them; Parts themselves are
/// rendered as a section heading at the start of their first chapter.
///
/// Stand-alone Scene leaves (no Chapter parent) become single-scene
/// chapters with their own title.  FrontMatter / BackMatter become
/// chapters wrapped in their respective `<section epub:type="…">`.
pub fn manuscript_to_html_chapters(input: &ManuscriptInput) -> Vec<HtmlChapter> {
    // Build by-parent map.
    let by_parent = build_children_map(&input.nodes);
    let project = input.nodes.iter().find(|n| n.kind == NodeKind::Project);
    let top_parent = project.map(|p| Some(p.id)).unwrap_or(None);
    let top_children: Vec<&Node> = by_parent.get(&top_parent).cloned().unwrap_or_default();

    let mut out = Vec::new();
    let mut chapter_index = 0u32;
    let mut pending_part_heading: Option<String> = None;

    for node in &top_children {
        emit_top_level(
            node,
            &by_parent,
            input,
            &mut out,
            &mut chapter_index,
            &mut pending_part_heading,
        );
    }

    out
}

fn emit_top_level(
    node: &Node,
    by_parent: &BTreeMap<Option<Ulid>, Vec<&Node>>,
    input: &ManuscriptInput,
    out: &mut Vec<HtmlChapter>,
    chapter_index: &mut u32,
    pending_part_heading: &mut Option<String>,
) {
    match node.kind {
        NodeKind::Part => {
            // Defer the part heading until the next chapter — Parts on
            // their own don't get an EPUB chapter.
            *pending_part_heading = Some(node.title.clone());
            if let Some(children) = by_parent.get(&Some(node.id)) {
                for c in children {
                    emit_top_level(
                        c,
                        by_parent,
                        input,
                        out,
                        chapter_index,
                        pending_part_heading,
                    );
                }
            }
        }
        NodeKind::Chapter => {
            *chapter_index += 1;
            let mut body = String::new();
            if let Some(part) = pending_part_heading.take() {
                body.push_str(&format!(
                    "<section class=\"part\"><h1>{}</h1></section>\n",
                    html_escape(part.trim()),
                ));
            }
            body.push_str(&format!(
                "<h1>Chapter {} — {}</h1>\n",
                *chapter_index,
                html_escape(node.title.trim()),
            ));
            // Scenes under the chapter, in document order.
            if let Some(scenes) = by_parent.get(&Some(node.id)) {
                for s in scenes {
                    if matches!(s.kind, NodeKind::Scene) {
                        if let Some(text) = input.scene_texts.get(&s.id) {
                            // The caller passes pre-rendered HTML when
                            // available; otherwise treat the string as
                            // plain text and wrap in a paragraph.
                            if text.contains("<p>") || text.contains("<h") {
                                body.push_str(text);
                            } else {
                                body.push_str("<p>");
                                body.push_str(&html_escape(text.trim()));
                                body.push_str("</p>");
                            }
                            body.push('\n');
                        }
                    }
                }
            }
            out.push(HtmlChapter {
                node_id: node.id,
                title: node.title.clone(),
                html_body: body,
            });
        }
        NodeKind::Scene => {
            // Stand-alone scene with no Chapter parent — render as a
            // single-scene chapter so the spine is well-formed.
            *chapter_index += 1;
            let body = match input.scene_texts.get(&node.id) {
                Some(t) if t.contains("<p>") || t.contains("<h") => t.clone(),
                Some(t) => format!("<p>{}</p>\n", html_escape(t.trim())),
                None => "<p><em>(empty scene)</em></p>\n".to_owned(),
            };
            out.push(HtmlChapter {
                node_id: node.id,
                title: if node.title.trim().is_empty() {
                    format!("Scene {chapter_index}")
                } else {
                    node.title.clone()
                },
                html_body: body,
            });
        }
        NodeKind::FrontMatter | NodeKind::BackMatter => {
            let kind_str = if matches!(node.kind, NodeKind::FrontMatter) {
                "frontmatter"
            } else {
                "backmatter"
            };
            let body = match input.scene_texts.get(&node.id) {
                Some(t) if t.contains("<p>") || t.contains("<h") => t.clone(),
                Some(t) => format!("<p>{}</p>\n", html_escape(t.trim())),
                None => "".to_owned(),
            };
            let body = format!(
                "<section epub:type=\"{kind_str}\"><h1>{}</h1>\n{}</section>",
                html_escape(node.title.trim()),
                body,
            );
            out.push(HtmlChapter {
                node_id: node.id,
                title: node.title.clone(),
                html_body: body,
            });
        }
        NodeKind::Project => {
            if let Some(children) = by_parent.get(&Some(node.id)) {
                for c in children {
                    emit_top_level(
                        c,
                        by_parent,
                        input,
                        out,
                        chapter_index,
                        pending_part_heading,
                    );
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::NodeStatus;
    use chrono::Utc;

    fn paragraph(text: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{ "type": "text", "text": text }]
            }]
        })
    }

    #[test]
    fn pm_doc_paragraph_is_plain_text() {
        let md = pm_doc_to_markdown(&paragraph("Hello world."));
        assert_eq!(md, "Hello world.");
    }

    #[test]
    fn pm_doc_heading_uses_hashes() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 },
                  "content": [{ "type": "text", "text": "Section" }] },
                { "type": "paragraph",
                  "content": [{ "type": "text", "text": "Body." }] }
            ]
        });
        let md = pm_doc_to_markdown(&doc);
        assert_eq!(md, "## Section\n\nBody.");
    }

    #[test]
    fn pm_doc_marks_bold_italic_code() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "alpha" },
                    { "type": "text", "text": " bold ", "marks": [{ "type": "bold" }] },
                    { "type": "text", "text": "italic", "marks": [{ "type": "italic" }] },
                    { "type": "text", "text": " ." },
                ]
            }]
        });
        let md = pm_doc_to_markdown(&doc);
        assert_eq!(md, "alpha** bold ***italic* .");
    }

    fn node(id: Ulid, parent: Option<Ulid>, kind: NodeKind, title: &str, position: &str) -> Node {
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

    #[test]
    fn manuscript_renders_three_part_book() {
        let project = Ulid::new();
        let part_a = Ulid::new();
        let chap_a1 = Ulid::new();
        let scene_a1a = Ulid::new();
        let scene_a1b = Ulid::new();
        let nodes = vec![
            node(project, None, NodeKind::Project, "Test Book", "0|hzzzzz:"),
            node(part_a, Some(project), NodeKind::Part, "Part 1", "0|i00000:"),
            node(
                chap_a1,
                Some(part_a),
                NodeKind::Chapter,
                "Beginnings",
                "0|i00000:",
            ),
            node(
                scene_a1a,
                Some(chap_a1),
                NodeKind::Scene,
                "scene a",
                "0|i00000:",
            ),
            node(
                scene_a1b,
                Some(chap_a1),
                NodeKind::Scene,
                "scene b",
                "0|j00000:",
            ),
        ];
        let mut texts = BTreeMap::new();
        texts.insert(scene_a1a, "Once upon a midnight dreary.".to_owned());
        texts.insert(scene_a1b, "While I pondered, weak and weary.".to_owned());

        let (md, stats) = manuscript_to_markdown(&ManuscriptInput {
            nodes,
            scene_texts: texts,
            title: "Test Book".to_owned(),
            author: "Jane Doe".to_owned(),
        });

        assert!(md.starts_with("# Test Book"), "must start with H1 title");
        assert!(md.contains("*by Jane Doe*"));
        assert!(md.contains("## Part 1"));
        assert!(md.contains("### Chapter 1 — Beginnings"));
        assert!(md.contains("Once upon a midnight dreary."));
        assert!(md.contains("While I pondered, weak and weary."));
        assert_eq!(stats.part_count, 1);
        assert_eq!(stats.chapter_count, 1);
        assert_eq!(stats.scene_count, 2);
        assert!(stats.word_count >= 8);
    }

    #[test]
    fn empty_scene_gets_placeholder() {
        let project = Ulid::new();
        let part = Ulid::new();
        let chap = Ulid::new();
        let scene = Ulid::new();
        let nodes = vec![
            node(project, None, NodeKind::Project, "T", "0|hzzzzz:"),
            node(part, Some(project), NodeKind::Part, "P", "0|i00000:"),
            node(chap, Some(part), NodeKind::Chapter, "C", "0|i00000:"),
            node(scene, Some(chap), NodeKind::Scene, "S", "0|i00000:"),
        ];
        let (md, _) = manuscript_to_markdown(&ManuscriptInput {
            nodes,
            scene_texts: BTreeMap::new(),
            title: "T".to_owned(),
            author: "".to_owned(),
        });
        assert!(md.contains("_(empty scene)_"));
    }

    #[test]
    fn pm_doc_to_html_paragraph() {
        let html = pm_doc_to_html(&paragraph("Hello & welcome <world>."));
        assert!(html.contains("<p>Hello &amp; welcome &lt;world&gt;.</p>"));
    }

    #[test]
    fn pm_doc_to_html_heading_and_marks() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 },
                  "content": [{ "type": "text", "text": "Title" }] },
                { "type": "paragraph",
                  "content": [
                    { "type": "text", "text": "bold", "marks": [{ "type": "bold" }] },
                    { "type": "text", "text": " " },
                    { "type": "text", "text": "italic", "marks": [{ "type": "italic" }] },
                  ] }
            ]
        });
        let html = pm_doc_to_html(&doc);
        assert!(html.contains("<h2>Title</h2>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn manuscript_to_html_chapters_groups_scenes_under_chapter() {
        let project = Ulid::new();
        let part = Ulid::new();
        let chap = Ulid::new();
        let s1 = Ulid::new();
        let s2 = Ulid::new();
        let nodes = vec![
            node(project, None, NodeKind::Project, "Book", "0|hzzzzz:"),
            node(part, Some(project), NodeKind::Part, "Part 1", "0|i00000:"),
            node(
                chap,
                Some(part),
                NodeKind::Chapter,
                "Beginnings",
                "0|i00000:",
            ),
            node(s1, Some(chap), NodeKind::Scene, "scene a", "0|i00000:"),
            node(s2, Some(chap), NodeKind::Scene, "scene b", "0|j00000:"),
        ];
        let mut texts = BTreeMap::new();
        texts.insert(s1, "<p>First scene.</p>".to_owned());
        texts.insert(s2, "<p>Second scene.</p>".to_owned());

        let chapters = manuscript_to_html_chapters(&ManuscriptInput {
            nodes,
            scene_texts: texts,
            title: "Book".into(),
            author: "".into(),
        });
        assert_eq!(chapters.len(), 1, "one chapter node → one HtmlChapter");
        let body = &chapters[0].html_body;
        assert!(body.contains("Chapter 1 — Beginnings"));
        assert!(
            body.contains("Part 1"),
            "part heading rolls into the next chapter"
        );
        assert!(body.contains("First scene."));
        assert!(body.contains("Second scene."));
    }

    #[test]
    fn siblings_render_in_lexorank_order() {
        let project = Ulid::new();
        let p1 = Ulid::new();
        let p2 = Ulid::new();
        // Insert P2 first to verify rendering follows position, not vec order.
        let nodes = vec![
            node(project, None, NodeKind::Project, "Book", "0|hzzzzz:"),
            node(p2, Some(project), NodeKind::Part, "Second", "0|j00000:"),
            node(p1, Some(project), NodeKind::Part, "First", "0|i00000:"),
        ];
        let (md, _) = manuscript_to_markdown(&ManuscriptInput {
            nodes,
            scene_texts: BTreeMap::new(),
            title: "Book".to_owned(),
            author: "".to_owned(),
        });
        let first_idx = md.find("## First").unwrap();
        let second_idx = md.find("## Second").unwrap();
        assert!(first_idx < second_idx, "First must come before Second");
    }
}
