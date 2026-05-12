//! ProseMirror document utilities — pure functions over the JSON shape.
//!
//! These helpers used to live in three places (the Tauri command layer, the
//! orchestrator's voice pipeline, and the copyedit applier).  They moved
//! here so all three consume the same canonical implementation.

/// Best-effort plain-text rendering of a ProseMirror doc.  Walks every text
/// node depth-first, joining block children with `\n` between blocks.
/// Sufficient for prompt input, voice-fingerprint computation, and
/// originality scanning.
///
/// Inline marks (bold, italic, links) are dropped — only the text is kept.
pub fn pm_doc_to_text(doc: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(content) = doc.get("content").and_then(|v| v.as_array()) {
        for block in content {
            walk(block, &mut out);
            out.push('\n');
        }
    }
    out
}

fn walk(node: &serde_json::Value, out: &mut String) {
    if node.get("type").and_then(|v| v.as_str()) == Some("text") {
        if let Some(t) = node.get("text").and_then(|v| v.as_str()) {
            out.push_str(t);
        }
        return;
    }
    if let Some(children) = node.get("content").and_then(|v| v.as_array()) {
        for c in children {
            walk(c, out);
        }
    }
}

/// Build a minimal ProseMirror doc from flat text.  One paragraph per
/// non-empty line; empty lines collapse.  Loses inline marks — caller
/// should warn the user when they accept changes through this path.
pub fn flat_text_to_pm_doc(flat: &str) -> serde_json::Value {
    let blocks: Vec<serde_json::Value> = flat
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": line }],
            })
        })
        .collect();
    serde_json::json!({
        "type":    "doc",
        "content": blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pm_doc_to_text_walks_nested_blocks() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "Hello" }] },
                { "type": "paragraph", "content": [{ "type": "text", "text": "world" }] },
            ]
        });
        assert_eq!(pm_doc_to_text(&doc), "Hello\nworld\n");
    }

    #[test]
    fn flat_round_trips() {
        let pm = flat_text_to_pm_doc("First.\nSecond.\n");
        assert_eq!(pm_doc_to_text(&pm), "First.\nSecond.\n");
    }

    #[test]
    fn empty_doc_renders_empty() {
        let doc = serde_json::json!({ "type": "doc", "content": [] });
        assert_eq!(pm_doc_to_text(&doc), "");
    }

    #[test]
    fn flat_skips_empty_lines() {
        let pm = flat_text_to_pm_doc("a\n\nb\n");
        let blocks = pm.get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(blocks.len(), 2);
    }
}
