//! The 16 shipped MVP validators.
//!
//! Each validator is a pure function `fn(&ValidatorContext) -> Vec<Issue>`.
//! Severity choices follow the spec:
//!   - **Error**   blocks export until resolved.
//!   - **Warning** is shown to the user pre-export with a confirm prompt.
//!   - **Info**    is silent in the export gate but visible in the panel.

use booksforge_domain::{
    validator::{Severity, ValidatorIssue},
    EmDash, EntryKind, NodeKind, QuoteStyle,
};

use crate::{is_prose_node, issue, scene_word_count, walk_text_nodes_mut, Validator, ValidatorContext};

// ── Registry of every shipped validator ──────────────────────────────────────

pub static ALL_VALIDATORS: &[Validator] = &[
    Validator { id: "double-spaces",          description: "Two or more spaces between words.",                                  run: double_spaces,         fix: Some(fix_double_spaces) },
    Validator { id: "trailing-whitespace",    description: "Lines ending in whitespace.",                                          run: trailing_whitespace,   fix: Some(fix_trailing_whitespace) },
    Validator { id: "multiple-blank-lines",   description: "Three or more consecutive blank lines.",                               run: multiple_blank_lines,  fix: Some(fix_multiple_blank_lines) },
    Validator { id: "em-dash-style",          description: "Dash usage inconsistent with the project style book.",                 run: em_dash_style,         fix: Some(fix_em_dash_style) },
    Validator { id: "quote-style",            description: "Smart vs straight quotes mixed in the same paragraph.",                run: quote_style,           fix: None },
    Validator { id: "unmatched-quotes",       description: "Odd number of straight or smart quotes in a paragraph.",               run: unmatched_quotes,      fix: None },
    Validator { id: "ellipsis-form",          description: "Ellipsis form does not match style-book preference.",                  run: ellipsis_form,         fix: None },
    Validator { id: "heading-hierarchy",      description: "Heading levels skip a step (e.g. H1 → H3).",                           run: heading_hierarchy,     fix: None },
    Validator { id: "missing-alt-text",       description: "Image without alt text — accessibility / KDP requirement.",            run: missing_alt_text,      fix: None },
    Validator { id: "broken-links",           description: "Markdown link with empty URL.",                                         run: broken_links,          fix: None },
    Validator { id: "orphan-chapter",         description: "Chapter has zero scenes.",                                              run: orphan_chapter,        fix: None },
    Validator { id: "very-short-scene",       description: "Scene under 50 words — likely a stub.",                                 run: very_short_scene,      fix: None },
    Validator { id: "very-long-scene",        description: "Scene over 5000 words — review for splitting.",                         run: very_long_scene,       fix: None },
    Validator { id: "untitled-node",          description: "Project / part / chapter / scene with empty title.",                    run: untitled_node,         fix: None },
    Validator { id: "ai-tells-detected",      description: "Phrase flagged by the active vocabulary as an LLM tell.",               run: ai_tells_detected,     fix: None },
    Validator { id: "vocab-replace-pending",  description: "Term has a `replace` rule that hasn't been applied to this scene.",    run: vocab_replace_pending, fix: Some(fix_vocab_replace) },
    Validator { id: "kdp-metadata",           description: "KDP-eBook metadata gate: title / author / language / ISBN sanity.",    run: kdp_metadata,          fix: None },
];

// ── 1. double-spaces ──────────────────────────────────────────────────────────

fn double_spaces(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        let mut chars = scene.text.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c == ' ' {
                if let Some(&(_, next)) = chars.peek() {
                    if next == ' ' {
                        out.push(issue(
                            "double-spaces", "DBL01", Severity::Warning,
                            "Two or more consecutive spaces — likely a typo or post-paste artefact.",
                            Some(scene.node_id),
                            Some(i as u32), Some((i + 2) as u32),
                            true,
                        ));
                        // skip until run of spaces ends
                        while let Some(&(_, c2)) = chars.peek() {
                            if c2 == ' ' { chars.next(); } else { break; }
                        }
                    }
                }
            }
        }
    }
    out
}

// ── 2. trailing-whitespace ────────────────────────────────────────────────────

fn trailing_whitespace(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        for (i, line) in scene.text.lines().enumerate() {
            if !line.is_empty() && (line.ends_with(' ') || line.ends_with('\t')) {
                out.push(issue(
                    "trailing-whitespace", "TWS01", Severity::Info,
                    format!("Line {} ends with whitespace.", i + 1),
                    Some(scene.node_id), None, None, true,
                ));
            }
        }
    }
    out
}

// ── 3. multiple-blank-lines ───────────────────────────────────────────────────

fn multiple_blank_lines(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        let mut blanks = 0;
        for line in scene.text.lines() {
            if line.trim().is_empty() {
                blanks += 1;
                if blanks == 3 {
                    out.push(issue(
                        "multiple-blank-lines", "MBL01", Severity::Warning,
                        "Three or more consecutive blank lines — likely a paste artefact.",
                        Some(scene.node_id), None, None, true,
                    ));
                }
            } else {
                blanks = 0;
            }
        }
    }
    out
}

// ── 4. em-dash-style ──────────────────────────────────────────────────────────

fn em_dash_style(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    let preferred = ctx.style.em_dash;
    for scene in ctx.scenes {
        let has_em  = scene.text.contains('—');
        let has_en  = scene.text.contains('–');
        let has_dbl = scene.text.contains("--");

        let unwanted: Vec<&str> = match preferred {
            EmDash::Em     => {
                let mut v = vec![];
                if has_en  { v.push("en-dash (–)"); }
                if has_dbl { v.push("double hyphen (--)"); }
                v
            }
            EmDash::En     => {
                let mut v = vec![];
                if has_em  { v.push("em-dash (—)"); }
                if has_dbl { v.push("double hyphen (--)"); }
                v
            }
            EmDash::Hyphen => {
                let mut v = vec![];
                if has_em { v.push("em-dash (—)"); }
                if has_en { v.push("en-dash (–)"); }
                v
            }
        };
        if !unwanted.is_empty() {
            out.push(issue(
                "em-dash-style", "EMD01", Severity::Warning,
                format!(
                    "Project prefers {:?} but this scene contains: {}.",
                    preferred, unwanted.join(", ")
                ),
                Some(scene.node_id), None, None, true,
            ));
        }
    }
    out
}

// ── 5. quote-style ────────────────────────────────────────────────────────────

fn quote_style(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    let pref = ctx.style.quote_style;
    for scene in ctx.scenes {
        let has_smart   = scene.text.contains('“') || scene.text.contains('”');
        let has_straight = scene.text.contains('"');
        let mixed = has_smart && has_straight;

        if mixed {
            out.push(issue(
                "quote-style", "QSTY01", Severity::Warning,
                "Smart and straight quotes mixed in the same scene.",
                Some(scene.node_id), None, None, true,
            ));
        } else if matches!(pref, QuoteStyle::Smart) && has_straight {
            out.push(issue(
                "quote-style", "QSTY02", Severity::Warning,
                "Project prefers smart quotes but this scene uses straight quotes.",
                Some(scene.node_id), None, None, true,
            ));
        } else if matches!(pref, QuoteStyle::Straight) && has_smart {
            out.push(issue(
                "quote-style", "QSTY03", Severity::Warning,
                "Project prefers straight quotes but this scene uses smart quotes.",
                Some(scene.node_id), None, None, true,
            ));
        }
    }
    out
}

// ── 6. unmatched-quotes ───────────────────────────────────────────────────────

fn unmatched_quotes(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        // Count straight quotes — paired apostrophes are tricky, so we flag
        // odd counts at scene level.  The smart-quote case is detected by
        // mismatched “/” counts.
        let straight = scene.text.matches('"').count();
        if straight % 2 == 1 {
            out.push(issue(
                "unmatched-quotes", "UMQ01", Severity::Warning,
                "Odd number of straight double quotes — likely an unclosed dialogue line.",
                Some(scene.node_id), None, None, false,
            ));
        }
        let opens  = scene.text.matches('“').count();
        let closes = scene.text.matches('”').count();
        if opens != closes {
            out.push(issue(
                "unmatched-quotes", "UMQ02", Severity::Warning,
                format!(
                    "Smart-quote imbalance: {opens} opening (“) vs {closes} closing (”)."
                ),
                Some(scene.node_id), None, None, false,
            ));
        }
    }
    out
}

// ── 7. ellipsis-form ──────────────────────────────────────────────────────────

fn ellipsis_form(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    use booksforge_domain::EllipsisForm;
    let mut out = Vec::new();
    for scene in ctx.scenes {
        let has_glyph = scene.text.contains('…');
        let has_dots  = scene.text.contains("...");
        match ctx.style.ellipsis_form {
            EllipsisForm::SingleGlyph if has_dots => {
                out.push(issue(
                    "ellipsis-form", "ELL01", Severity::Warning,
                    "Project prefers a single ellipsis glyph (…) but this scene uses three dots.",
                    Some(scene.node_id), None, None, true,
                ));
            }
            EllipsisForm::ThreeDots if has_glyph => {
                out.push(issue(
                    "ellipsis-form", "ELL02", Severity::Warning,
                    "Project prefers three dots (...) but this scene uses the ellipsis glyph.",
                    Some(scene.node_id), None, None, true,
                ));
            }
            _ => {}
        }
    }
    out
}

// ── 8. heading-hierarchy ──────────────────────────────────────────────────────

// Each (parent, child) arm is its own documented case in the
// AGENTS.md hierarchy — collapsing them via `|` would hide the
// distinct semantics of "Project → Part" vs "Project → Chapter".
#[allow(clippy::match_same_arms)]
fn heading_hierarchy(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    // Walk the structural nodes in document order: a Project should only
    // contain Parts or Chapters; Parts contain Chapters; Chapters contain
    // Scenes / FrontMatter / BackMatter.  Anything else is a hierarchy
    // violation.
    for node in ctx.nodes {
        let parent_kind = node.parent_id
            .and_then(|pid| ctx.nodes.iter().find(|n| n.id == pid))
            .map(|p| p.kind);
        let valid = match (parent_kind, node.kind) {
            (None,                                 NodeKind::Project) => true,
            (Some(NodeKind::Project),              NodeKind::Part)    => true,
            (Some(NodeKind::Project),              NodeKind::Chapter) => true,
            (Some(NodeKind::Project),              NodeKind::FrontMatter) => true,
            (Some(NodeKind::Project),              NodeKind::BackMatter)  => true,
            (Some(NodeKind::Part),                 NodeKind::Chapter) => true,
            (Some(NodeKind::Chapter),              NodeKind::Scene)   => true,
            (Some(NodeKind::Chapter),              NodeKind::FrontMatter) => true,
            (Some(NodeKind::Chapter),              NodeKind::BackMatter)  => true,
            // Standalone scene under no parent — odd but tolerated.
            (None,                                 NodeKind::Scene)   => false,
            // Anything else is a real hierarchy violation.
            _ => false,
        };
        if !valid && node.parent_id.is_some() {
            out.push(issue(
                "heading-hierarchy", "HRC01", Severity::Error,
                format!(
                    "{:?} '{}' has parent of kind {:?} — invalid nesting.",
                    node.kind, node.title, parent_kind
                ),
                Some(node.id), None, None, false,
            ));
        }
        if !valid && node.parent_id.is_none() && node.kind == NodeKind::Scene {
            out.push(issue(
                "heading-hierarchy", "HRC02", Severity::Warning,
                format!("Scene '{}' has no parent chapter.", node.title),
                Some(node.id), None, None, false,
            ));
        }
    }
    out
}

// ── 9. missing-alt-text ───────────────────────────────────────────────────────

fn missing_alt_text(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    // Plain text representation includes Markdown image syntax.  Detect
    // `![](url)` (empty alt) — KDP and screen readers both require alt.
    for scene in ctx.scenes {
        let mut search = scene.text.as_str();
        while let Some(idx) = search.find("![]") {
            out.push(issue(
                "missing-alt-text", "ALT01", Severity::Error,
                "Image without alt text — accessibility and KDP requirement.",
                Some(scene.node_id),
                Some(idx as u32), Some((idx + 3) as u32),
                false,
            ));
            search = &search[idx + 3..];
        }
    }
    out
}

// ── 10. broken-links ──────────────────────────────────────────────────────────

fn broken_links(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        // `[text]()` — empty URL is always an error.
        let mut search = scene.text.as_str();
        let mut absolute_offset = 0usize;
        while let Some(rel) = search.find("]()") {
            let abs = absolute_offset + rel;
            // Walk back to find `[` so we have a sensible offset to report.
            let bracket_start = scene.text[..abs].rfind('[').unwrap_or(abs);
            out.push(issue(
                "broken-links", "LNK01", Severity::Error,
                "Markdown link with empty URL.",
                Some(scene.node_id),
                Some(bracket_start as u32), Some((abs + 3) as u32),
                false,
            ));
            absolute_offset = abs + 3;
            search = &scene.text[absolute_offset..];
        }
    }
    out
}

// ── 11. orphan-chapter ────────────────────────────────────────────────────────

fn orphan_chapter(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for chapter in ctx.nodes.iter().filter(|n| n.kind == NodeKind::Chapter) {
        let has_scene = ctx.nodes.iter().any(|n|
            n.parent_id == Some(chapter.id) && is_prose_node(n.kind)
        );
        if !has_scene {
            out.push(issue(
                "orphan-chapter", "ORP01", Severity::Warning,
                format!("Chapter '{}' has no scenes.", chapter.title),
                Some(chapter.id), None, None, false,
            ));
        }
    }
    out
}

// ── 12. very-short-scene ──────────────────────────────────────────────────────

fn very_short_scene(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        let words = scene_word_count(&scene.text);
        if words > 0 && words < 50 {
            out.push(issue(
                "very-short-scene", "SHRT01", Severity::Info,
                format!("Scene has {words} words — under the 50-word threshold; likely a stub."),
                Some(scene.node_id), None, None, false,
            ));
        }
    }
    out
}

// ── 13. very-long-scene ───────────────────────────────────────────────────────

fn very_long_scene(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for scene in ctx.scenes {
        let words = scene_word_count(&scene.text);
        if words > 5_000 {
            out.push(issue(
                "very-long-scene", "LONG01", Severity::Info,
                format!("Scene has {words} words — review whether to split."),
                Some(scene.node_id), None, None, false,
            ));
        }
    }
    out
}

// ── 14. untitled-node ─────────────────────────────────────────────────────────

fn untitled_node(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    for node in ctx.nodes {
        if node.title.trim().is_empty() {
            out.push(issue(
                "untitled-node", "TIT01", Severity::Warning,
                format!("{:?} has no title.", node.kind),
                Some(node.id), None, None, false,
            ));
        }
    }
    out
}

// ── 15. ai-tells-detected ─────────────────────────────────────────────────────

fn ai_tells_detected(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    let resolved = booksforge_domain::resolve_vocab(ctx.vocab, ctx.active_vocab_layers);

    // Only the `Avoid` rules apply to plain detection — `Replace` rules
    // are handled by `vocab-replace-pending`.
    let avoid_terms: Vec<(&str, &str, Option<&str>)> = resolved.iter()
        .filter(|e| e.kind == EntryKind::Avoid)
        .map(|e| (e.term.as_str(), e.layer.as_str(), e.rationale.as_deref()))
        .collect();

    for scene in ctx.scenes {
        let lower = scene.text.to_lowercase();
        for (term, layer, rationale) in &avoid_terms {
            if let Some(idx) = lower.find(term) {
                let kind_label = if *layer == "ai_tells" { "AI tell" }
                                 else { "vocab rule" };
                let mut msg = format!(
                    "{kind_label} ({layer}): \"{term}\" — replace or rephrase.",
                );
                if let Some(r) = rationale { msg.push(' '); msg.push_str(r); }
                out.push(issue(
                    "ai-tells-detected", "AIT01",
                    if *layer == "ai_tells" { Severity::Info } else { Severity::Warning },
                    msg,
                    Some(scene.node_id),
                    Some(idx as u32),
                    Some((idx + term.len()) as u32),
                    false,
                ));
            }
        }
    }
    out
}

// ── 16. vocab-replace-pending ─────────────────────────────────────────────────

fn vocab_replace_pending(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    let resolved = booksforge_domain::resolve_vocab(ctx.vocab, ctx.active_vocab_layers);
    let replace_rules: Vec<(&str, &str, &str)> = resolved.iter()
        .filter(|e| e.kind == EntryKind::Replace && e.replacement.is_some())
        .map(|e| (e.term.as_str(), e.replacement.as_deref().unwrap_or_default(), e.layer.as_str()))
        .collect();

    for scene in ctx.scenes {
        let lower = scene.text.to_lowercase();
        for (term, replacement, layer) in &replace_rules {
            if let Some(idx) = lower.find(term) {
                out.push(issue(
                    "vocab-replace-pending", "VRP01", Severity::Warning,
                    format!(
                        "Vocabulary rule ({layer}): replace \"{term}\" → \"{replacement}\".",
                    ),
                    Some(scene.node_id),
                    Some(idx as u32),
                    Some((idx + term.len()) as u32),
                    true,
                ));
            }
        }
    }
    out
}

// ── 17. kdp-metadata ──────────────────────────────────────────────────────────

fn kdp_metadata(ctx: &ValidatorContext) -> Vec<ValidatorIssue> {
    let mut out = Vec::new();
    let Some(meta) = ctx.project else { return out };

    if meta.title.trim().is_empty() {
        out.push(issue(
            "kdp-metadata", "KDP01", Severity::Error,
            "KDP: project title is empty.",
            None, None, None, false,
        ));
    }
    if meta.author.trim().is_empty() {
        out.push(issue(
            "kdp-metadata", "KDP02", Severity::Error,
            "KDP: author byline is empty.",
            None, None, None, false,
        ));
    }
    let lang = meta.language.trim();
    if lang.is_empty() {
        out.push(issue(
            "kdp-metadata", "KDP03", Severity::Warning,
            "KDP: project doesn't have a language tag (BCP-47, e.g. en-US).",
            None, None, None, false,
        ));
    } else if !is_plausible_bcp47(lang) {
        out.push(issue(
            "kdp-metadata", "KDP04", Severity::Error,
            format!("KDP: '{lang}' is not a plausible BCP-47 language tag (expected like 'en', 'en-US', 'fr-FR')."),
            None, None, None, false,
        ));
    }
    if let Some(raw) = &meta.isbn {
        let cleaned: String = raw.chars().filter(|c| c.is_ascii_digit() || *c == 'X').collect();
        if !(cleaned.len() == 10 || cleaned.len() == 13) {
            out.push(issue(
                "kdp-metadata", "KDP05", Severity::Error,
                format!("KDP: ISBN '{raw}' must be 10 or 13 digits (got {} useful chars).", cleaned.len()),
                None, None, None, false,
            ));
        }
    }
    out
}

// ── Auto-fix functions (G5) ───────────────────────────────────────────────────
//
// Each takes the scene's pm_doc by mutable reference, walks every text node
// via `walk_text_nodes_mut`, and returns the count of nodes whose text was
// rewritten.  Calling these never invents new content — they are pure
// mechanical normalisations.

fn fix_double_spaces(pm_doc: &mut serde_json::Value, _ctx: &ValidatorContext) -> u32 {
    walk_text_nodes_mut(pm_doc, |t| {
        let mut out = String::with_capacity(t.len());
        let mut last_space = false;
        for c in t.chars() {
            if c == ' ' {
                if !last_space { out.push(' '); }
                last_space = true;
            } else {
                out.push(c);
                last_space = false;
            }
        }
        if out != t { Some(out) } else { None }
    })
}

fn fix_trailing_whitespace(pm_doc: &mut serde_json::Value, _ctx: &ValidatorContext) -> u32 {
    walk_text_nodes_mut(pm_doc, |t| {
        // Strip trailing spaces/tabs from each line (preserve newlines).
        let stripped: String = t
            .split('\n')
            .map(|line| line.trim_end_matches([' ', '\t']))
            .collect::<Vec<_>>()
            .join("\n");
        if stripped != t { Some(stripped) } else { None }
    })
}

fn fix_multiple_blank_lines(pm_doc: &mut serde_json::Value, _ctx: &ValidatorContext) -> u32 {
    walk_text_nodes_mut(pm_doc, |t| {
        let mut out = String::with_capacity(t.len());
        let mut blank_streak = 0;
        for line in t.split('\n') {
            if line.trim().is_empty() {
                blank_streak += 1;
                if blank_streak <= 1 {
                    if !out.is_empty() { out.push('\n'); }
                    out.push_str(line);
                }
            } else {
                if !out.is_empty() { out.push('\n'); }
                out.push_str(line);
                blank_streak = 0;
            }
        }
        if out != t { Some(out) } else { None }
    })
}

fn fix_em_dash_style(pm_doc: &mut serde_json::Value, ctx: &ValidatorContext) -> u32 {
    use booksforge_domain::EmDash;
    let preferred = ctx.style.em_dash;
    walk_text_nodes_mut(pm_doc, |t| {
        let mut out = t.to_owned();
        match preferred {
            EmDash::Em     => {
                out = out.replace("--", "—");
                out = out.replace('–', "—");
            }
            EmDash::En     => {
                out = out.replace("--", "–");
                out = out.replace('—', "–");
            }
            EmDash::Hyphen => {
                out = out.replace('—', "-");
                out = out.replace('–', "-");
            }
        }
        if out != t { Some(out) } else { None }
    })
}

fn fix_vocab_replace(pm_doc: &mut serde_json::Value, ctx: &ValidatorContext) -> u32 {
    let resolved = booksforge_domain::resolve_vocab(ctx.vocab, ctx.active_vocab_layers);
    let pairs: Vec<(String, String)> = resolved.iter()
        .filter(|e| e.kind == EntryKind::Replace && e.replacement.is_some())
        .map(|e| (e.term.clone(), e.replacement.clone().unwrap_or_default()))
        .collect();
    if pairs.is_empty() { return 0; }

    walk_text_nodes_mut(pm_doc, |t| {
        let mut out = t.to_owned();
        let mut changed = false;
        for (term, replacement) in &pairs {
            // Case-insensitive whole-word-ish replace: scan lowercase form,
            // splice replacements back into the original casing-preserving
            // output (the replacement string is used verbatim — vocab
            // replacements are short canonical alternatives).
            let lower = out.to_lowercase();
            // Outer caller iterates this whole helper, so we apply at
            // most one replacement per call and let the next iteration
            // re-derive the lowercase shadow against the modified
            // string.  This keeps the byte-offset arithmetic trivially
            // correct after the splice.
            if let Some(idx) = lower.find(term.as_str()) {
                let end = idx + term.len();
                out.replace_range(idx..end, replacement);
                changed = true;
            }
            // Fallback: if the simple slice approach above broke out early,
            // run a straightforward case-insensitive global replace.
            if !changed {
                let re_lower = out.to_lowercase();
                if re_lower.contains(term.as_str()) {
                    out = naive_ci_replace(&out, term, replacement);
                    changed = true;
                }
            }
        }
        if changed { Some(out) } else { None }
    })
}

/// Case-insensitive global replace.  Walks the source once, copying chars
/// directly when no match starts at the cursor and emitting `replacement`
/// when the lowercase form of the next `term.len()` bytes equals `term`.
fn naive_ci_replace(src: &str, term: &str, replacement: &str) -> String {
    let term_lower = term.to_lowercase();
    let term_len = term_lower.len();
    if term_len == 0 { return src.to_owned(); }
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + term_len <= bytes.len() {
            // Only attempt match on a UTF-8 char boundary.
            if src.is_char_boundary(i) && src.is_char_boundary(i + term_len) {
                let candidate = &src[i..i + term_len];
                if candidate.to_lowercase() == term_lower {
                    out.push_str(replacement);
                    i += term_len;
                    continue;
                }
            }
        }
        // Copy a single char.  `i < bytes.len()` (loop guard) and
        // `src` is a `&str` so the next char is always present —
        // `unwrap_or_default` would silently swallow the impossible
        // case; `break` matches the loop's existing termination
        // contract.
        let ch = match src[i..].chars().next() {
            Some(c) => c,
            None    => break,
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Cheap BCP-47 sniff: matches `aa`, `aa-AA`, `aa-Aaaa`, `aa-AA-x-anything`.
/// Not a full parser — rejects obvious junk while accepting the tags writers
/// actually use.
fn is_plausible_bcp47(s: &str) -> bool {
    let mut parts = s.split('-');
    let Some(primary) = parts.next() else { return false };
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    for part in parts {
        let len = part.len();
        if !(1..=8).contains(&len) { return false; }
        let ok = part.chars().all(|c| c.is_ascii_alphanumeric());
        if !ok { return false; }
    }
    true
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SceneText;
    use booksforge_domain::{Node, NodeStatus, StyleBook};
    use chrono::Utc;
    use ulid::Ulid;

    fn scene(id: Ulid, text: &str) -> SceneText {
        SceneText { node_id: id, text: text.to_owned() }
    }

    fn node(id: Ulid, parent: Option<Ulid>, kind: NodeKind, title: &str) -> Node {
        let now = Utc::now();
        Node {
            id, parent_id: parent, kind, title: title.to_owned(),
            position: Node::DEFAULT_POSITION.to_owned(), status: NodeStatus::Drafting,
            pov: None, beat: None, target_words: None,
            created_at: now, updated_at: now, deleted_at: None,
        }
    }

    fn ctx<'a>(
        nodes: &'a [Node], scenes: &'a [SceneText], style: &'a StyleBook,
        vocab: &'a [booksforge_domain::VocabEntry], layers: &'a [&'a str],
    ) -> ValidatorContext<'a> {
        ValidatorContext {
            nodes, scenes, style, vocab,
            active_vocab_layers: layers,
            project: None,
        }
    }

    #[test]
    fn double_spaces_flags_two_spaces() {
        let id = Ulid::new();
        let s = vec![scene(id, "Two  spaces here.")];
        let nodes = vec![];
        let style = StyleBook::default();
        let issues = double_spaces(&ctx(&nodes, &s, &style, &[], &[]));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
    }

    #[test]
    fn double_spaces_skips_clean_text() {
        let id = Ulid::new();
        let s = vec![scene(id, "Single spaces only.")];
        let nodes = vec![];
        let style = StyleBook::default();
        assert!(double_spaces(&ctx(&nodes, &s, &style, &[], &[])).is_empty());
    }

    #[test]
    fn unmatched_quotes_detects_odd_count() {
        let id = Ulid::new();
        let s = vec![scene(id, "She said \"hello and walked away.")];
        let nodes = vec![];
        let style = StyleBook::default();
        let issues = unmatched_quotes(&ctx(&nodes, &s, &style, &[], &[]));
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn em_dash_style_flags_double_hyphen_when_em_preferred() {
        let id = Ulid::new();
        let s = vec![scene(id, "Wait--what?")];
        let nodes = vec![];
        let style = StyleBook::default();
        assert_eq!(style.em_dash, EmDash::Em);
        let issues = em_dash_style(&ctx(&nodes, &s, &style, &[], &[]));
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn missing_alt_text_finds_empty_alt_image() {
        let id = Ulid::new();
        let s = vec![scene(id, "See diagram: ![](image.png)")];
        let nodes = vec![];
        let style = StyleBook::default();
        let issues = missing_alt_text(&ctx(&nodes, &s, &style, &[], &[]));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn heading_hierarchy_flags_scene_under_part() {
        let project = Ulid::new();
        let part = Ulid::new();
        let bad_scene = Ulid::new();
        let nodes = vec![
            node(project, None, NodeKind::Project, "Book"),
            node(part, Some(project), NodeKind::Part, "Part 1"),
            node(bad_scene, Some(part), NodeKind::Scene, "Direct scene"),
        ];
        let style = StyleBook::default();
        let issues = heading_hierarchy(&ctx(&nodes, &[], &style, &[], &[]));
        assert!(issues.iter().any(|i| i.severity == Severity::Error));
    }

    #[test]
    fn very_short_scene_warns_under_threshold() {
        let id = Ulid::new();
        let s = vec![scene(id, "Just three words.")];
        let nodes = vec![];
        let style = StyleBook::default();
        let issues = very_short_scene(&ctx(&nodes, &s, &style, &[], &[]));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Info);
    }

    #[test]
    fn ai_tells_detected_uses_active_layer_only() {
        use booksforge_domain::{EntryKind, EntrySource, VocabEntry};
        let id = Ulid::new();
        let s = vec![scene(id, "We will delve into the data.")];
        let nodes = vec![];
        let style = StyleBook::default();
        let vocab = vec![
            VocabEntry::new("ai_tells", "Delve", EntryKind::Avoid, EntrySource::Starter),
        ];
        let issues = ai_tells_detected(&ctx(&nodes, &s, &style, &vocab, &["ai_tells"]));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Info);

        // With the layer disabled, no issue.
        let issues2 = ai_tells_detected(&ctx(&nodes, &s, &style, &vocab, &[]));
        assert!(issues2.is_empty());
    }

    #[test]
    fn kdp_metadata_blocks_on_empty_title_or_author() {
        use crate::ProjectMetaSummary;
        let nodes  = vec![];
        let scenes = vec![];
        let style  = StyleBook::default();
        let meta = ProjectMetaSummary {
            title: "".into(),
            author: "".into(),
            language: "en-US".into(),
            isbn: None,
        };
        let ctx = ValidatorContext {
            nodes: &nodes, scenes: &scenes, style: &style,
            vocab: &[], active_vocab_layers: &[],
            project: Some(&meta),
        };
        let issues = kdp_metadata(&ctx);
        assert_eq!(issues.iter().filter(|i| i.severity == Severity::Error).count(), 2);
        assert!(issues.iter().any(|i| i.code == "KDP01"));
        assert!(issues.iter().any(|i| i.code == "KDP02"));
    }

    #[test]
    fn kdp_metadata_passes_with_valid_meta() {
        use crate::ProjectMetaSummary;
        let nodes  = vec![];
        let scenes = vec![];
        let style  = StyleBook::default();
        let meta = ProjectMetaSummary {
            title: "Test Book".into(),
            author: "Jane Doe".into(),
            language: "en-US".into(),
            isbn: Some("9781234567890".into()),
        };
        let ctx = ValidatorContext {
            nodes: &nodes, scenes: &scenes, style: &style,
            vocab: &[], active_vocab_layers: &[],
            project: Some(&meta),
        };
        assert!(kdp_metadata(&ctx).is_empty());
    }

    #[test]
    fn kdp_metadata_warns_when_no_meta_supplied() {
        let nodes  = vec![];
        let scenes = vec![];
        let style  = StyleBook::default();
        let ctx = ValidatorContext {
            nodes: &nodes, scenes: &scenes, style: &style,
            vocab: &[], active_vocab_layers: &[],
            project: None,
        };
        assert!(kdp_metadata(&ctx).is_empty(), "no meta = no checks (graceful)");
    }

    #[test]
    fn bcp47_sniff_accepts_common_tags() {
        for ok in ["en", "en-US", "fr-FR", "zh-Hant", "es-419", "pt-BR"] {
            assert!(is_plausible_bcp47(ok), "should accept {ok}");
        }
        for bad in ["", "EN", "en_US", "english", "12-AA"] {
            assert!(!is_plausible_bcp47(bad), "should reject {bad}");
        }
    }

    fn paragraph_doc(text: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{ "type": "text", "text": text }],
            }],
        })
    }

    fn first_paragraph_text(doc: &serde_json::Value) -> &str {
        doc["content"][0]["content"][0]["text"].as_str().unwrap_or("")
    }

    #[test]
    fn fix_double_spaces_collapses_runs() {
        let mut doc = paragraph_doc("Two  spaces   here.");
        let count = fix_double_spaces(&mut doc, &ctx(&[], &[], &StyleBook::default(), &[], &[]));
        assert_eq!(count, 1);
        assert_eq!(first_paragraph_text(&doc), "Two spaces here.");
    }

    #[test]
    fn fix_trailing_whitespace_strips_line_endings() {
        let mut doc = paragraph_doc("alpha   \nbeta\t\ngamma");
        let count = fix_trailing_whitespace(&mut doc, &ctx(&[], &[], &StyleBook::default(), &[], &[]));
        assert_eq!(count, 1);
        assert_eq!(first_paragraph_text(&doc), "alpha\nbeta\ngamma");
    }

    #[test]
    fn fix_multiple_blank_lines_collapses_to_one() {
        let mut doc = paragraph_doc("alpha\n\n\n\nbeta");
        let count = fix_multiple_blank_lines(&mut doc, &ctx(&[], &[], &StyleBook::default(), &[], &[]));
        assert_eq!(count, 1);
        assert_eq!(first_paragraph_text(&doc), "alpha\n\nbeta");
    }

    #[test]
    fn fix_em_dash_style_normalises_to_em_when_preferred() {
        let mut doc = paragraph_doc("Wait--what? Then--really--again.");
        let style = StyleBook::default();
        assert_eq!(style.em_dash, EmDash::Em);
        let count = fix_em_dash_style(&mut doc, &ctx(&[], &[], &style, &[], &[]));
        assert_eq!(count, 1);
        assert_eq!(first_paragraph_text(&doc), "Wait—what? Then—really—again.");
    }

    #[test]
    fn fix_vocab_replace_substitutes_terms_case_insensitive() {
        use booksforge_domain::{EntryKind, EntrySource, VocabEntry};
        let mut doc = paragraph_doc("We Delve into the data and showcase the results.");
        let mut entry1 = VocabEntry::new("ai_tells", "delve",    EntryKind::Replace, EntrySource::Starter);
        entry1.replacement = Some("explore".into());
        let mut entry2 = VocabEntry::new("ai_tells", "showcase", EntryKind::Replace, EntrySource::Starter);
        entry2.replacement = Some("show".into());
        let vocab = vec![entry1, entry2];
        let count = fix_vocab_replace(
            &mut doc,
            &ctx(&[], &[], &StyleBook::default(), &vocab, &["ai_tells"]),
        );
        assert_eq!(count, 1);
        let result = first_paragraph_text(&doc);
        assert!(result.contains("explore"));
        assert!(result.contains("show "));
        assert!(!result.contains("Delve"));
    }

    #[test]
    fn vocab_replace_pending_flags_with_auto_fix_hint() {
        use booksforge_domain::{EntryKind, EntrySource, VocabEntry};
        let id = Ulid::new();
        let s = vec![scene(id, "We delve into the topic.")];
        let nodes = vec![];
        let style = StyleBook::default();
        let mut entry = VocabEntry::new(
            "ai_tells", "delve", EntryKind::Replace, EntrySource::Starter,
        );
        entry.replacement = Some("explore".into());
        let vocab = vec![entry];
        let issues = vocab_replace_pending(&ctx(&nodes, &s, &style, &vocab, &["ai_tells"]));
        assert_eq!(issues.len(), 1);
        assert!(issues[0].auto_fixable);
    }
}
