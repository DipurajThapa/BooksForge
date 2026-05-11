//! Best-effort Markdown mirror writer.
//!
//! After every SQLite scene commit, the caller should call `write_mirror` to
//! keep `manuscript/<node_ulid>.md` in sync.  This is intentionally
//! "best-effort" — failure is logged but not propagated, so a mirror-write
//! error never blocks the user's save.
//!
//! The mirror format is a minimal conversion from ProseMirror JSON, sufficient
//! for human readability and disaster recovery.  It is NOT the canonical
//! source of truth; that is `scene_content.pm_doc`.
//!
//! ## Snapshotting policy (BACKLOG §A4)
//!
//! The Markdown mirror is **not** included in snapshots.  Three reasons:
//!
//!   1. **The pm_doc IS in the snapshot.**  Every scene's `pm_doc` JSON
//!      goes into the snapshot's content-addressed store, so a restore
//!      can always rebuild the mirror from the canonical source.  Storing
//!      the mirror separately would duplicate every scene on disk.
//!
//!   2. **Lossy round-trip.**  The mirror loses some inline marks
//!      (links, custom node attrs) on conversion.  Re-importing a mirror
//!      file as authoritative would silently degrade the manuscript.
//!      Restore re-emits the mirror from the snapshotted pm_doc, which
//!      stays lossless.
//!
//!   3. **Mirror is a *projection*, not state.**  The bundle's
//!      `manuscript/*.md` files are an external-tooling courtesy
//!      (Pandoc, Word import, GitHub preview).  Treating them as state
//!      would make every mirror format change a snapshot migration.
//!
//! When a snapshot is restored, callers SHOULD re-run `write_mirror` for
//! every restored scene so the on-disk Markdown matches the
//! freshly-restored database.  `booksforge-snapshot::SnapshotService::restore`
//! does NOT do this today; that's tracked as a follow-up under §A4.

use serde_json::Value;

use crate::{bundle::BundlePath, FsError};

/// Write `manuscript/<node_ulid>.md` from a ProseMirror JSON document.
///
/// Creates the `manuscript/` directory if it doesn't exist.
/// Returns `Ok(())` on success.  The caller should log and ignore errors.
pub async fn write_mirror(
    bundle: &BundlePath,
    node_ulid: &str,
    pm_doc: &Value,
) -> Result<(), FsError> {
    let markdown = pm_doc_to_markdown(pm_doc);
    let path = bundle.chapter_file(node_ulid);

    // Ensure parent exists (manuscript/ should already exist from bundle create).
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| FsError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
    }

    tokio::fs::write(&path, markdown.as_bytes())
        .await
        .map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })
}

// ── ProseMirror JSON → Markdown ───────────────────────────────────────────────

fn pm_doc_to_markdown(doc: &Value) -> String {
    let mut out = String::new();
    if let Some(content) = doc.get("content").and_then(|v| v.as_array()) {
        for node in content {
            render_block(node, &mut out);
        }
    }
    out
}

fn render_block(node: &Value, out: &mut String) {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let content = node.get("content").and_then(|v| v.as_array());

    match node_type {
        "paragraph" => {
            if let Some(inlines) = content {
                for inline in inlines {
                    render_inline(inline, out);
                }
            }
            out.push_str("\n\n");
        }
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;
            out.push_str(&"#".repeat(level));
            out.push(' ');
            if let Some(inlines) = content {
                for inline in inlines {
                    render_inline(inline, out);
                }
            }
            out.push_str("\n\n");
        }
        "blockquote" => {
            if let Some(blocks) = content {
                let mut inner = String::new();
                for block in blocks {
                    render_block(block, &mut inner);
                }
                for line in inner.lines() {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
                out.push('\n');
            }
        }
        "bulletList" => {
            if let Some(items) = content {
                for item in items {
                    out.push_str("- ");
                    render_list_item(item, out);
                }
                out.push('\n');
            }
        }
        "orderedList" => {
            if let Some(items) = content {
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&format!("{}. ", i + 1));
                    render_list_item(item, out);
                }
                out.push('\n');
            }
        }
        "codeBlock" => {
            let lang = node
                .get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            if let Some(inlines) = content {
                for inline in inlines {
                    if let Some(t) = inline.get("text").and_then(|v| v.as_str()) {
                        out.push_str(t);
                    }
                }
            }
            out.push_str("\n```\n\n");
        }
        "horizontalRule" => {
            out.push_str("---\n\n");
        }
        _ => {
            // Unknown block — render inline children if any.
            if let Some(inlines) = content {
                for inline in inlines {
                    render_inline(inline, out);
                }
            }
            out.push_str("\n\n");
        }
    }
}

fn render_list_item(item: &Value, out: &mut String) {
    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
        for block in content {
            if block.get("type").and_then(|v| v.as_str()) == Some("paragraph") {
                if let Some(inlines) = block.get("content").and_then(|v| v.as_array()) {
                    for inline in inlines {
                        render_inline(inline, out);
                    }
                }
            }
        }
    }
    out.push('\n');
}

fn render_inline(node: &Value, out: &mut String) {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match node_type {
        "text" => {
            let text = node.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let marks = node.get("marks").and_then(|v| v.as_array());

            let mut bold = false;
            let mut italic = false;
            let mut code = false;
            let mut link: Option<&str> = None;

            if let Some(marks) = marks {
                for mark in marks {
                    match mark.get("type").and_then(|v| v.as_str()) {
                        Some("bold" | "strong") => bold = true,
                        Some("italic" | "em") => italic = true,
                        Some("code") => code = true,
                        Some("link") => {
                            link = mark
                                .get("attrs")
                                .and_then(|a| a.get("href"))
                                .and_then(|v| v.as_str());
                        }
                        _ => {}
                    }
                }
            }

            if code {
                out.push('`');
                out.push_str(text);
                out.push('`');
            } else {
                let prefix = if bold && italic {
                    "***"
                } else if bold {
                    "**"
                } else if italic {
                    "*"
                } else {
                    ""
                };
                let suffix = prefix;
                if let Some(href) = link {
                    out.push('[');
                    out.push_str(prefix);
                    out.push_str(text);
                    out.push_str(suffix);
                    out.push_str("](");
                    out.push_str(href);
                    out.push(')');
                } else {
                    out.push_str(prefix);
                    out.push_str(text);
                    out.push_str(suffix);
                }
            }
        }
        "hardBreak" => {
            out.push_str("  \n");
        }
        _ => {}
    }
}
