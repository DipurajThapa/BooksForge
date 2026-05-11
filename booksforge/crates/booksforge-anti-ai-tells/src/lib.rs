//! Anti-AI-tells dictionary + density measurement (Layer 3, pure logic).
//!
//! BACKLOG §A16 / Phase 3B — Rust port of `artifacts/ghostwriter/
//! anti_ai_tells.py`. Real LLM prose has measurable fingerprints —
//! overused connectives ("Furthermore," "Moreover,"), hedge openers
//! ("It's important to note that…"), the "delve / tapestry /
//! multifaceted" lexicon, em-dash overuse, and cliché body-as-feeling
//! phrases. This module flags them so the orchestrator can run a
//! span-targeted rewrite pass that preserves the surrounding prose.
//!
//! Use it in two ways:
//!   1. `tells_per_1000_words(text)` → density measurement for the
//!      Honest Score panel and the post-polish gate.
//!   2. `revision_prompt(text)` → a prompt fragment a polish LLM can
//!      use to rewrite ONLY the offending spans.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

pub mod paragraph_quality;
pub mod structural;
pub use paragraph_quality::{rank_paragraphs, score_paragraph, ParagraphQualityScore};
pub use structural::{
    detect_anaphora_chain, detect_low_burstiness_paragraphs, detect_low_variance_paragraphs,
    detect_missing_concrete_noun, detect_pronoun_substitution_game, detect_substitution_game,
    structural_tells,
};

#[derive(Debug, Clone)]
struct TellSpec {
    pattern: &'static str,
    multiline: bool,
    severity: u8, // 1=cosmetic, 2=routine, 3=glaring
    category: &'static str,
    why: &'static str,
    suggested_replacement: Option<&'static str>,
}

/// One detected fingerprint hit.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TellHit {
    pub start: usize,
    pub end: usize,
    pub matched: String,
    pub severity: u8,
    pub category: String,
    pub why: String,
    pub suggested_replacement: Option<String>,
}

/// Density-and-verdict report.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TellsReport {
    pub word_count: u32,
    pub tell_count: u32,
    pub density_per_1000: f32,
    pub weighted_density_per_1000: f32,
    pub by_severity_3: u32,
    pub by_severity_2: u32,
    pub by_severity_1: u32,
    /// `"PUBLISHABLE"` (weighted < 6), `"NEEDS_REVISION"` (6-12), or
    /// `"AI_SMELL_HIGH"` (>12).
    pub verdict: String,
    pub by_category_json: String,
}

const TELLS: &[TellSpec] = &[
    // ── Lexicon overused by LLMs (the "GPT smell") ─────────────────────
    TellSpec {
        pattern: r"\bdelve\b",
        multiline: false,
        severity: 3,
        category: "lexicon",
        why: "the canonical AI verb",
        suggested_replacement: Some("explore"),
    },
    TellSpec {
        pattern: r"\btapestry\b",
        multiline: false,
        severity: 3,
        category: "lexicon",
        why: "AI-favourite metaphor",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bmultifaceted\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "vague hedge",
        suggested_replacement: Some("complex"),
    },
    TellSpec {
        pattern: r"\bnuanced\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "AI-overused",
        suggested_replacement: Some("specific"),
    },
    TellSpec {
        pattern: r"\brealm\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI-preferred",
        suggested_replacement: Some("world"),
    },
    TellSpec {
        pattern: r"\bparamount\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "LLM hedge for important",
        suggested_replacement: Some("essential"),
    },
    TellSpec {
        pattern: r"\bcrucial\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "overused hedge",
        suggested_replacement: Some("central"),
    },
    TellSpec {
        pattern: r"\bplethora\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI-flavoured 'many'",
        suggested_replacement: Some("many"),
    },
    TellSpec {
        pattern: r"\bmyriad\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI-flavoured 'many'",
        suggested_replacement: Some("many"),
    },
    TellSpec {
        pattern: r"\bvibrant\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "marketing adjective",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bbustling\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "marketing adjective",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bdynamic\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "marketing adjective",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bseamlessly\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI tic",
        suggested_replacement: Some("smoothly"),
    },
    TellSpec {
        pattern: r"\bintricate\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "AI tic",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bintricacies\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI tic",
        suggested_replacement: Some("details"),
    },
    TellSpec {
        pattern: r"\bsymphony\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI metaphor",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\borchestrating\b",
        multiline: false,
        severity: 1,
        category: "lexicon",
        why: "overused metaphor",
        suggested_replacement: Some("running"),
    },
    TellSpec {
        pattern: r"\bkaleidoscope\b",
        multiline: false,
        severity: 2,
        category: "lexicon",
        why: "AI variety metaphor",
        suggested_replacement: None,
    },
    // ── Hedge openers ──────────────────────────────────────────────────
    TellSpec {
        pattern: r"^\s*It['\u{2019}]s\s+(?:important|worth|vital|essential|crucial)\s+to\s+(?:note|remember|consider|mention)",
        multiline: true,
        severity: 3,
        category: "hedge_opener",
        why: "universal LLM-paragraph opener",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"^\s*Indeed,\s*",
        multiline: true,
        severity: 2,
        category: "hedge_opener",
        why: "AI sentence opener",
        suggested_replacement: Some(""),
    },
    TellSpec {
        pattern: r"^\s*Furthermore,\s*",
        multiline: true,
        severity: 2,
        category: "hedge_opener",
        why: "AI connective",
        suggested_replacement: Some(""),
    },
    TellSpec {
        pattern: r"^\s*Moreover,\s*",
        multiline: true,
        severity: 2,
        category: "hedge_opener",
        why: "AI connective",
        suggested_replacement: Some(""),
    },
    TellSpec {
        pattern: r"^\s*In essence,\s*",
        multiline: true,
        severity: 2,
        category: "hedge_opener",
        why: "rarely earns place",
        suggested_replacement: Some(""),
    },
    // ── Closers ────────────────────────────────────────────────────────
    TellSpec {
        pattern: r"In\s+conclusion,\s*",
        multiline: false,
        severity: 3,
        category: "closer",
        why: "almost never in published prose",
        suggested_replacement: Some(""),
    },
    TellSpec {
        pattern: r"Ultimately,\s*",
        multiline: false,
        severity: 2,
        category: "closer",
        why: "overused at paragraph-end",
        suggested_replacement: Some(""),
    },
    TellSpec {
        pattern: r"At the end of the day,?\s*",
        multiline: false,
        severity: 3,
        category: "cliche",
        why: "high-cliché score",
        suggested_replacement: Some(""),
    },
    // ── Hedge phrases ──────────────────────────────────────────────────
    TellSpec {
        pattern: r"\b(?:a\s+(?:wide\s+)?(?:range|variety|array)\s+of)\b",
        multiline: false,
        severity: 2,
        category: "hedge_phrase",
        why: "vague catalogue",
        suggested_replacement: Some("many"),
    },
    TellSpec {
        pattern: r"\bvarious\b",
        multiline: false,
        severity: 1,
        category: "hedge_phrase",
        why: "specify or cut",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bin\s+today['\u{2019}]s\s+(?:fast-paced|modern|digital|connected|complex)\s+world\b",
        multiline: false,
        severity: 3,
        category: "cliche",
        why: "explicit AI trope",
        suggested_replacement: Some(""),
    },
    // ── Body-as-feeling clichés (especially in fiction) ────────────────
    TellSpec {
        pattern: r"\bblood\s+ran\s+cold\b",
        multiline: false,
        severity: 3,
        category: "cliche",
        why: "high-cliché score",
        suggested_replacement: Some("froze"),
    },
    TellSpec {
        pattern: r"\bheart\s+raced\b",
        multiline: false,
        severity: 2,
        category: "cliche",
        why: "show the symptom",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bbreath\s+caught\b",
        multiline: false,
        severity: 2,
        category: "cliche",
        why: "overused",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bsent\s+shivers\s+down\b",
        multiline: false,
        severity: 3,
        category: "cliche",
        why: "high-cliché score",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bsigh\s+of\s+relief\b",
        multiline: false,
        severity: 2,
        category: "cliche",
        why: "show the body",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\ba\s+rollercoaster\s+of\s+emotions\b",
        multiline: false,
        severity: 3,
        category: "cliche",
        why: "high-cliché",
        suggested_replacement: None,
    },
    // ── Em-dash overuse ────────────────────────────────────────────────
    TellSpec {
        pattern: r"\u{2014}.{1,40}\u{2014}.{1,40}\u{2014}",
        multiline: false,
        severity: 3,
        category: "punctuation",
        why: "three em-dashes within ~80 chars — LLM tic",
        suggested_replacement: None,
    },
    // ── Show-don't-tell labels ─────────────────────────────────────────
    //
    // Run #12 §5: `she felt` and `he felt` were severity-1 noise — they
    // are generic "show don't tell" style notes, not AI fingerprints.
    // Robinson, McCarthy, Strout, Saunders all use them. They were
    // contributing 4 of 6 weighted points in Run #12's NEEDS_REVISION
    // verdict on prose that was actually publishable. Removed as a
    // false-positive class.
    //
    // `she could feel` stays — that one IS a redundant hedge ("could
    // feel" carries no extra meaning over "felt") and is the kind of
    // thing a copy-editor cuts on first pass.
    TellSpec {
        pattern: r"\bshe\s+could\s+feel\b",
        multiline: false,
        severity: 2,
        category: "tell_dont_show",
        why: "redundant",
        suggested_replacement: None,
    },
    // ── Expletives ─────────────────────────────────────────────────────
    TellSpec {
        pattern: r"\bIt\s+is\s+\w+\s+that\b",
        multiline: false,
        severity: 1,
        category: "expletive",
        why: "tighten by removing",
        suggested_replacement: None,
    },
    TellSpec {
        pattern: r"\bThere\s+(?:is|are)\s+\w+\s+that\b",
        multiline: false,
        severity: 1,
        category: "expletive",
        why: "tighten by removing",
        suggested_replacement: None,
    },
];

fn build_regex(spec: &TellSpec) -> Regex {
    // Patterns are all hand-curated literal strings checked at build
    // time by the unit tests below — a panic here on a built-in pattern
    // is a programmer error, not a runtime condition.
    #[allow(clippy::expect_used)]
    RegexBuilder::new(spec.pattern)
        .case_insensitive(true)
        .multi_line(spec.multiline)
        .build()
        .expect("anti-ai-tells: invalid built-in regex")
}

/// Find every per-token fingerprint hit in the text (lexicon, hedge,
/// cliché, em-dash, etc. — the contents of the `TELLS` dictionary).
/// Returned in source-position order. Empty text returns an empty list.
///
/// **Does not run the structural detectors** (anaphora, substitution-game,
/// IQR, concrete-noun). Use [`find_all_tells`] for the merged set, or
/// call [`structural_tells`] directly if you only want the structural set.
pub fn find_tells(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    for spec in TELLS {
        let re = build_regex(spec);
        for m in re.find_iter(text) {
            hits.push(TellHit {
                start: m.start(),
                end: m.end(),
                matched: m.as_str().to_owned(),
                severity: spec.severity,
                category: spec.category.to_owned(),
                why: spec.why.to_owned(),
                suggested_replacement: spec.suggested_replacement.map(|s| s.to_owned()),
            });
        }
    }
    hits.sort_by_key(|h| h.start);
    hits
}

/// Per-token + structural detectors merged. This is the entry point
/// callers should prefer for a complete tells report — Run #11's
/// monotony was invisible to the per-token dictionary alone.
pub fn find_all_tells(text: &str) -> Vec<TellHit> {
    let mut hits = find_tells(text);
    hits.extend(structural_tells(text));
    hits.sort_by_key(|h| h.start);
    hits
}

/// Density measurement — the headline metric for the Honest Score panel.
///
/// Includes both per-token tells (lexicon, hedge, cliché) AND structural
/// tells (anaphora chain, substitution game, low variance, no concrete
/// noun). The Run #11 monotony score was driven entirely by the
/// structural axis — without merging both detector families the verdict
/// would have read PUBLISHABLE on prose that is in fact mechanical.
pub fn tells_per_1000_words(text: &str) -> TellsReport {
    let hits = find_all_tells(text);
    let word_count = text.split_whitespace().count().max(1) as u32;
    let mut by_sev = [0u32; 4]; // index 1, 2, 3
    let mut by_cat: HashMap<String, u32> = HashMap::new();
    let mut weighted: u32 = 0;
    for h in &hits {
        let s = h.severity.min(3) as usize;
        by_sev[s] += 1;
        weighted += h.severity as u32;
        *by_cat.entry(h.category.clone()).or_insert(0) += 1;
    }
    let density = 1000.0 * hits.len() as f32 / word_count as f32;
    let weighted_density = 1000.0 * weighted as f32 / word_count as f32;
    let verdict = if weighted_density < 6.0 {
        "PUBLISHABLE"
    } else if weighted_density < 12.0 {
        "NEEDS_REVISION"
    } else {
        "AI_SMELL_HIGH"
    };
    TellsReport {
        word_count,
        tell_count: hits.len() as u32,
        density_per_1000: (density * 100.0).round() / 100.0,
        weighted_density_per_1000: (weighted_density * 100.0).round() / 100.0,
        by_severity_3: by_sev[3],
        by_severity_2: by_sev[2],
        by_severity_1: by_sev[1],
        verdict: verdict.to_owned(),
        by_category_json: serde_json::to_string(&by_cat).unwrap_or_else(|_| "{}".to_owned()),
    }
}

/// Build a prompt fragment that a polish LLM can use to rewrite ONLY the
/// flagged spans. Crucially we identify spans, not whole paragraphs, so
/// the polisher doesn't flatten the rest of the prose.
///
/// Includes structural tells. For span targeting on the structural side,
/// the matched range is the full offending paragraph or sentence chain;
/// the polish LLM should rewrite within that range without touching
/// surrounding prose.
pub fn revision_prompt(text: &str, max_targets: usize) -> String {
    let mut hits = find_all_tells(text);
    hits.truncate(max_targets);
    if hits.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(
        "Rewrite ONLY the following flagged spans. Preserve the surrounding \
         voice, paragraph rhythm, and meaning. Do not introduce new content. \
         If a span is genuinely necessary in context, leave it untouched and \
         explain in one line why.\n\n\
         Flagged spans (offset, severity, why, suggested replacement):\n",
    );
    for h in &hits {
        let mut line = format!(
            "- @{}-{} (sev {}, {}): {:?} — {}",
            h.start, h.end, h.severity, h.category, h.matched, h.why
        );
        if let Some(s) = &h.suggested_replacement {
            line.push_str(&format!(" → suggested: {s:?}"));
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_covers_canonical_ai_smells() {
        // The pattern set must catch the universal AI-prose pattern.
        let sample = "It's important to note that the realm of AI is multifaceted. \
                      Furthermore, in today's fast-paced world we must delve into \
                      the tapestry of various paradigms.";
        let hits = find_tells(sample);
        assert!(
            hits.len() >= 6,
            "expected ≥6 tells in canonical sample, got {}",
            hits.len()
        );
        let categories: std::collections::HashSet<_> =
            hits.iter().map(|h| h.category.as_str()).collect();
        assert!(categories.contains("hedge_opener"));
        assert!(categories.contains("lexicon"));
    }

    #[test]
    fn density_measurement_grades_correctly() {
        let pristine = "She did not turn the light on. The fridge was clicking.";
        let report_clean = tells_per_1000_words(pristine);
        assert_eq!(report_clean.verdict, "PUBLISHABLE");
        assert_eq!(report_clean.tell_count, 0);

        let slop = "It's important to note that the realm of artificial intelligence \
                    is multifaceted and intricate. Furthermore, in today's fast-paced \
                    world, we must delve into the tapestry of various paradigms. \
                    Indeed, the symphony of dynamic, vibrant, and bustling startups \
                    orchestrating a kaleidoscope of innovation creates a plethora of \
                    opportunities. In conclusion, ultimately, the heart raced as her \
                    blood ran cold.";
        let report_slop = tells_per_1000_words(slop);
        assert_eq!(report_slop.verdict, "AI_SMELL_HIGH");
        assert!(report_slop.tell_count >= 15);
        assert!(report_slop.weighted_density_per_1000 > 100.0);
    }

    #[test]
    fn em_dash_overuse_caught() {
        let sample = "She turned — slowly — toward the door — and froze.";
        let hits = find_tells(sample);
        assert!(hits.iter().any(|h| h.category == "punctuation"));
    }

    #[test]
    fn revision_prompt_skips_clean_text() {
        let prompt = revision_prompt("She did not turn the light on.", 30);
        assert!(prompt.is_empty());
    }

    #[test]
    fn revision_prompt_renders_targets() {
        let slop = "It's important to note that we must delve into the tapestry.";
        let prompt = revision_prompt(slop, 30);
        assert!(prompt.contains("Rewrite ONLY"));
        assert!(prompt.contains("delve"));
        assert!(prompt.contains("tapestry"));
    }

    #[test]
    fn cliche_body_phrases_are_severity_3() {
        let sample = "Her blood ran cold.";
        let hits = find_tells(sample);
        let cliche = hits
            .iter()
            .find(|h| h.matched.to_lowercase().contains("blood"));
        assert!(cliche.is_some());
        assert_eq!(cliche.unwrap().severity, 3);
    }

    #[test]
    fn show_dont_tell_only_flags_redundant_could_feel() {
        // Run #12 §5: `she felt` / `he felt` are no longer flagged
        // (false-positive class on legitimate fiction prose). Only the
        // truly redundant `she could feel` remains.
        let hits_felt = find_tells("She felt the rain. He felt the cold.");
        assert!(
            !hits_felt.iter().any(|h| h.category == "tell_dont_show"),
            "she felt / he felt are no longer noise-flagged: {hits_felt:?}",
        );

        let hits_could_feel = find_tells("She could feel the rain on her skin.");
        assert!(
            hits_could_feel
                .iter()
                .any(|h| h.category == "tell_dont_show"),
            "the redundant `she could feel` hedge is still flagged",
        );
    }

    #[test]
    fn density_per_1000_calculation() {
        // 1 tell in 5 words = 200 per 1000.
        let sample = "delve into the cold realm";
        let r = tells_per_1000_words(sample);
        assert!(r.density_per_1000 >= 200.0 - 1.0);
    }
}
