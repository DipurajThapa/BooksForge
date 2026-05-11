//! Local plagiarism / verbatim-overlap detector — pure logic.
//!
//! Two detection passes (both n-gram match, never sent off-device):
//!
//!   1. **Source overlap** — the agent was given some source text (a scene
//!      to copyedit, a synopsis to draft from, a chapter to review).  Any
//!      span of ≥`min_words` consecutive words in the agent's output that
//!      appears verbatim in the source is flagged.  This catches agents
//!      that copy-paste source instead of generating.  An exception is
//!      made for spans inside ASCII-quote runs ("…") since those are
//!      legitimate citations.
//!
//!   2. **Self-plagiarism** — the project's prior accepted scenes are
//!      concatenated into a corpus.  Any span of ≥`min_words` consecutive
//!      words in the new output that appears verbatim in that corpus is
//!      flagged.  Catches "the agent recycled chapter 3's opening".
//!
//! The detector is **strict** (default `min_words = 12` ≈ a full clause)
//! and **deterministic** — same inputs always produce the same hits in
//! the same order.  Pure function, no I/O.
//!
//! `OverlapHit` carries character offsets in the *output* and a quote of
//! the matched span so the UI can highlight it inline and ship the user
//! straight to the editor.

use serde::{Deserialize, Serialize};

/// Default minimum n-gram length (in whitespace-separated words) before a
/// match counts as plagiarism.  Twelve words is roughly one full clause —
/// short enough to catch real copy-paste, long enough to skip incidental
/// matches like "the door was open and he could see" which any two
/// authors might write independently.
pub const DEFAULT_MIN_WORDS: usize = 12;

/// Where the matched span came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlapKind {
    /// Span verbatim from the source the agent was given.  Indicates the
    /// agent copy-pasted instead of generating.
    Source,
    /// Span verbatim from the project's prior accepted scenes.  Indicates
    /// self-plagiarism / unintentional recycling.
    PriorScene,
}

/// One detected verbatim overlap.  Carries enough information to render
/// an inline highlight + "go to source" pointer in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlapHit {
    pub kind: OverlapKind,
    /// Character offset of the matched span's start in the *output* text.
    pub output_from: u32,
    /// Character offset of the matched span's end in the *output* text.
    pub output_to: u32,
    /// Number of whitespace-separated words in the matched span.
    pub words: u32,
    /// The matched text itself (truncated to 200 chars in the struct so
    /// large hits don't bloat the JSON).
    pub quote: String,
}

/// Detect verbatim spans of ≥`min_words` words in `output` that appear in
/// `source`.  Spans wholly inside an ASCII-quote run are treated as
/// citations and skipped (the writer marked them as quotes; that's not
/// plagiarism, that's attribution).
///
/// Returns hits sorted by `output_from` ascending.
pub fn detect_verbatim_overlap(output: &str, source: &str, min_words: usize) -> Vec<OverlapHit> {
    detect_overlap(output, source, min_words, OverlapKind::Source)
}

/// Detect verbatim spans of ≥`min_words` words in `output` that appear in
/// `prior_corpus` (concatenation of prior accepted scenes).  Same n-gram
/// logic as `detect_verbatim_overlap` but tagged `PriorScene`.
pub fn detect_self_plagiarism(
    output: &str,
    prior_corpus: &str,
    min_words: usize,
) -> Vec<OverlapHit> {
    detect_overlap(output, prior_corpus, min_words, OverlapKind::PriorScene)
}

fn detect_overlap(
    output: &str,
    haystack: &str,
    min_words: usize,
    kind: OverlapKind,
) -> Vec<OverlapHit> {
    let min_words = min_words.max(3);
    if output.is_empty() || haystack.is_empty() {
        return Vec::new();
    }

    // Tokenise output into words with character spans.
    let out_tokens = tokenise_with_spans(output);
    if out_tokens.len() < min_words {
        return Vec::new();
    }

    // Normalise haystack words for set membership (lowercased, no punct).
    let hay_norm: Vec<String> = haystack
        .split_whitespace()
        .map(normalise_word)
        .filter(|s| !s.is_empty())
        .collect();
    if hay_norm.len() < min_words {
        return Vec::new();
    }

    // Build a hashset of haystack n-grams of the minimum length.  Any
    // matching n-gram in the output is then *grown* greedily to cover the
    // longest verbatim run, so we don't emit overlapping hits.
    use std::collections::HashSet;
    let mut hay_ngrams: HashSet<String> = HashSet::with_capacity(hay_norm.len());
    for window in hay_norm.windows(min_words) {
        hay_ngrams.insert(window.join(" "));
    }

    let quoted_ranges = ascii_quoted_ranges(output);
    let mut hits = Vec::new();
    let mut i = 0;
    while i + min_words <= out_tokens.len() {
        let window: String = out_tokens[i..i + min_words]
            .iter()
            .map(|t| t.normalised.clone())
            .collect::<Vec<_>>()
            .join(" ");
        if hay_ngrams.contains(&window) {
            // Grow the match: extend `j` while the longer n-gram still
            // matches (rebuild the n-gram as we grow).  Bounded by output
            // length and a conservative cap (say, 256 words) so we don't
            // pathological-grow.
            let mut j = i + min_words;
            while j < out_tokens.len() && j - i < 256 {
                let extended: String = out_tokens[i..=j]
                    .iter()
                    .map(|t| t.normalised.clone())
                    .collect::<Vec<_>>()
                    .join(" ");
                // Cheap test: does any haystack n-gram contain this extended
                // string as a prefix?  Approximate via substring check on
                // the joined haystack-normalised string — for repeated
                // calls on the same haystack this would benefit from a
                // suffix-array, but the corpus is bounded (one project)
                // and clarity beats micro-optimisation.
                let hay_joined = hay_norm.join(" ");
                if hay_joined.contains(&extended) {
                    j += 1;
                } else {
                    break;
                }
            }
            let from = out_tokens[i].char_from;
            let to = out_tokens[j - 1].char_to;
            // Skip hits wholly inside an ASCII-quote run — those are
            // legitimate citations, not plagiarism.
            let inside_quote = quoted_ranges
                .iter()
                .any(|(qa, qb)| from >= *qa && to <= *qb);
            if !inside_quote {
                let words = (j - i) as u32;
                let quote = clamp_quote(output, from, to);
                hits.push(OverlapHit {
                    kind,
                    output_from: from,
                    output_to: to,
                    words,
                    quote,
                });
            }
            i = j; // skip past the matched run
        } else {
            i += 1;
        }
    }

    hits
}

#[derive(Debug, Clone)]
struct Token {
    normalised: String,
    char_from: u32,
    char_to: u32,
}

/// Split `s` into whitespace-separated tokens, returning the normalised
/// (lowercased, leading/trailing punctuation stripped) form plus the
/// character span in the original string.  Skips empty tokens.
fn tokenise_with_spans(s: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut char_idx: u32 = 0;
    let mut start: Option<u32> = None;
    let mut buf = String::new();
    for ch in s.chars() {
        if ch.is_whitespace() {
            if let Some(start_idx) = start.take() {
                let n = normalise_word(&buf);
                if !n.is_empty() {
                    out.push(Token {
                        normalised: n,
                        char_from: start_idx,
                        char_to: char_idx,
                    });
                }
                buf.clear();
            }
        } else {
            if start.is_none() {
                start = Some(char_idx);
            }
            buf.push(ch);
        }
        char_idx += 1;
    }
    if let Some(start_idx) = start {
        let n = normalise_word(&buf);
        if !n.is_empty() {
            out.push(Token {
                normalised: n,
                char_from: start_idx,
                char_to: char_idx,
            });
        }
    }
    out
}

fn normalise_word(w: &str) -> String {
    w.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Find character ranges wrapped in straight ASCII quotes — *inclusive*
/// of the quote chars themselves so a hit whose tokenised span happens
/// to include the surrounding `"` still counts as inside.  Smart-quote
/// runs are intentionally NOT treated as citation markers (those are
/// dialogue typography, not attribution).
fn ascii_quoted_ranges(s: &str) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut open: Option<u32> = None;
    for (idx, ch) in s.chars().enumerate() {
        let idx = idx as u32;
        if ch == '"' {
            match open.take() {
                Some(start) => out.push((start, idx + 1)), // inclusive of both quotes
                None => open = Some(idx),
            }
        }
    }
    out
}

fn clamp_quote(s: &str, from: u32, to: u32) -> String {
    let chars: Vec<char> = s.chars().collect();
    let from = (from as usize).min(chars.len());
    let to = (to as usize).min(chars.len());
    let raw: String = chars[from..to].iter().collect();
    if raw.chars().count() > 200 {
        let trimmed: String = raw.chars().take(200).collect();
        format!("{trimmed}…")
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_long_verbatim_span_from_source() {
        let source = "She walked into the dimly lit corridor and the floorboards groaned beneath her weight.";
        let output = "She walked into the dimly lit corridor and the floorboards groaned beneath her weight, again.";
        let hits = detect_verbatim_overlap(output, source, 12);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, OverlapKind::Source);
        assert!(hits[0].words >= 12);
    }

    #[test]
    fn ignores_short_overlap_below_threshold() {
        let source = "It was a dark and stormy night.";
        let output = "It was a dark and stormy night when she arrived.";
        let hits = detect_verbatim_overlap(output, source, 12);
        assert!(hits.is_empty(), "got {hits:?}");
    }

    #[test]
    fn skips_overlap_inside_ascii_quote_marks() {
        // A quoted citation should not count as plagiarism.
        let source = "She walked into the dimly lit corridor and the floorboards groaned beneath her weight.";
        let output = "He read aloud: \"She walked into the dimly lit corridor and the floorboards groaned beneath her weight.\"";
        let hits = detect_verbatim_overlap(output, source, 12);
        assert!(
            hits.is_empty(),
            "ASCII-quoted citation should be skipped, got {hits:?}"
        );
    }

    #[test]
    fn detects_self_plagiarism_against_prior_corpus() {
        let prior  = "The lighthouse beam swept across the cove three times before he turned away from the window.";
        let output = "Years later, the lighthouse beam swept across the cove three times before he turned away from the window again.";
        let hits = detect_self_plagiarism(output, prior, 12);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, OverlapKind::PriorScene);
    }

    #[test]
    fn deterministic_order() {
        let source = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo";
        let output = format!(
            "{src} and then {src}",
            src = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima"
        );
        let a = detect_verbatim_overlap(&output, source, 12);
        let b = detect_verbatim_overlap(&output, source, 12);
        assert_eq!(a, b, "detector must be deterministic");
    }

    #[test]
    fn returns_empty_for_empty_inputs() {
        assert!(detect_verbatim_overlap("", "anything", 12).is_empty());
        assert!(detect_verbatim_overlap("anything", "", 12).is_empty());
    }

    #[test]
    fn quote_field_is_truncated_for_very_long_matches() {
        let long_clause: String =
            "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima ".repeat(40);
        let hits = detect_verbatim_overlap(&long_clause, &long_clause, 12);
        assert!(!hits.is_empty());
        assert!(hits[0].quote.chars().count() <= 201);
    }
}
