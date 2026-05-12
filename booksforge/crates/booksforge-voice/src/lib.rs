//! Voice fingerprinting + stylometric distance (Layer 3, pure logic).
//!
//! BACKLOG §A16 / Phase 3A — Rust port of `artifacts/ghostwriter/
//! voice_fingerprint.py` so the orchestrator can extract numeric voice
//! constraints from comp samples (or accepted prose) and inject them
//! into the drafter / polish prompts. The same numbers drive the
//! stylometric-distance score reported alongside the rubric.
//!
//! Why measurable, not vibes: LLMs respect numeric constraints (median
//! sentence length 11 with 22% short sentences) but ignore vague
//! prompts ("write literary"). This crate computes the constraint
//! block; the prompt template renders it; the drafter writes against it.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod target;
pub use target::{BucketScore, SentenceLengthBucket, VoiceScore, VoiceTarget};

/// Common stop words, used for the rare-word ratio. Kept small + frozen
/// so the fingerprint is stable across runs (not influenced by changing
/// dictionaries).
const COMMON_WORDS: &[&str] = &[
    "the", "be", "to", "of", "and", "a", "in", "that", "have", "i", "it", "for", "not", "on",
    "with", "he", "as", "you", "do", "at", "this", "but", "his", "by", "from", "they", "we", "say",
    "her", "she", "or", "an", "will", "my", "one", "all", "would", "there", "their", "what", "so",
    "up", "out", "if", "about", "who", "get", "which", "go", "me", "when", "make", "can", "like",
    "time", "no", "just", "him", "know", "take", "people", "into", "year", "your", "good", "some",
    "could", "them", "see", "other", "than", "then", "now", "look", "only", "come", "its", "over",
    "think", "also", "back", "after", "use", "two", "how", "our", "work", "first", "well", "way",
    "even", "new", "want", "because", "any", "these", "give", "day", "most", "us", "is", "are",
    "was", "were", "been", "had", "has", "being", "more", "very", "much", "such", "own", "same",
    "too", "is",
];

/// Numeric voice profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceProfile {
    pub word_count: u32,
    pub sentence_count: u32,
    pub paragraph_count: u32,
    pub median_sentence_length: f32,
    pub p25_sentence_length: f32,
    pub p75_sentence_length: f32,
    /// Share (0-1) of sentences shorter than 8 words.
    pub pct_short_sentences: f32,
    /// Share (0-1) of sentences longer than 25 words.
    pub pct_long_sentences: f32,
    /// Share (0-1) of sentences containing dialogue marks.
    pub dialogue_ratio: f32,
    pub avg_paragraph_length_sentences: f32,
    /// Type-token ratio (vocab richness, 0-1).
    pub type_token_ratio: f32,
    /// Share of words NOT in the COMMON_WORDS list (0-1).
    pub rare_word_ratio: f32,
    pub em_dash_per_1000: f32,
    pub semicolon_per_1000: f32,
    pub parenthetical_per_1000: f32,
    pub avg_word_length: f32,
    /// Share of words with one syllable (rough cadence proxy).
    pub pct_monosyllabic_words: f32,

    // ── Length-stable lexical-diversity measures ──────────────────────────
    //
    // FEATURE_HARDENING_PLAN.md §1.1 — raw TTR has length bias. Short
    // texts trivially score higher because cumulative type count climbs
    // faster than cumulative token count when there are few tokens. The
    // published lexical-diversity literature (Covington & McFall 2010;
    // Koizumi & In'nami 2012; Zenker & Kyle 2025) is unanimous that
    // MATTR is "the only length-insensitive index that can compare texts
    // of different sizes" and MTLD is the second most stable.
    //
    // Both fields default to 0.0 so old persisted profiles don't break
    // deserialise — see `#[serde(default)]`.
    /// Moving-Average Type-Token Ratio with a 50-token sliding window.
    /// Reports 0.0 if the prose has fewer than 50 tokens (window doesn't
    /// fit). Use this — not [`Self::type_token_ratio`] — for any
    /// cross-run comparison where word counts differ.
    #[serde(default)]
    pub mattr_50: f32,

    /// Measure of Textual Lexical Diversity. Bidirectional walk through
    /// tokens; each time the running TTR drops below the 0.72 threshold
    /// constitutes a "factor". `total_tokens / factors` (averaged
    /// forward + backward). Reports 0.0 for prose under ~50 tokens.
    #[serde(default)]
    pub mtld: f32,
}

impl VoiceProfile {
    /// Render as a constraint block the drafter / polish prompts can read.
    pub fn constraints_block(&self, label: &str) -> String {
        let pct = |x: f32| (x * 100.0).round() as i32;
        format!(
            "Voice constraints from {label}:\n\
             - Median sentence length: {} words (IQR {}-{})\n\
             - Short-sentence share (<8 words): {}%\n\
             - Long-sentence share (>25 words): {}%\n\
             - Dialogue line share: {}%\n\
             - Average paragraph length: {:.1} sentences\n\
             - Vocabulary richness (type-token ratio): {:.2}\n\
             - Rare-word share (non-stopword): {}%\n\
             - Em-dashes per 1000 words: {:.1} (do NOT exceed)\n\
             - Semicolons per 1000 words: {:.1}\n\
             - Average word length: {:.2} characters\n\
             - Monosyllabic-word share: {}% (higher = punchier cadence)",
            self.median_sentence_length.round() as i32,
            self.p25_sentence_length.round() as i32,
            self.p75_sentence_length.round() as i32,
            pct(self.pct_short_sentences),
            pct(self.pct_long_sentences),
            pct(self.dialogue_ratio),
            self.avg_paragraph_length_sentences,
            self.type_token_ratio,
            pct(self.rare_word_ratio),
            self.em_dash_per_1000,
            self.semicolon_per_1000,
            self.avg_word_length,
            pct(self.pct_monosyllabic_words),
        )
    }
}

/// One component of the stylometric distance score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StylometricComponent {
    pub dim: String,
    pub delta_norm: f32,
    pub weight: f32,
}

/// Stylometric distance between two profiles. Score 0-10 (10 = identical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StylometricDistance {
    pub distance_score_out_of_10: f32,
    pub components: Vec<StylometricComponent>,
}

/// Build a fingerprint from prose text.
pub fn fingerprint(text: &str) -> VoiceProfile {
    let sentences = split_sentences(text);
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let words = word_tokens(text);

    if words.is_empty() || sentences.is_empty() {
        return VoiceProfile {
            word_count: words.len() as u32,
            sentence_count: sentences.len() as u32,
            paragraph_count: paragraphs.len() as u32,
            median_sentence_length: 0.0,
            p25_sentence_length: 0.0,
            p75_sentence_length: 0.0,
            pct_short_sentences: 0.0,
            pct_long_sentences: 0.0,
            dialogue_ratio: 0.0,
            avg_paragraph_length_sentences: 0.0,
            type_token_ratio: 0.0,
            rare_word_ratio: 0.0,
            em_dash_per_1000: 0.0,
            semicolon_per_1000: 0.0,
            parenthetical_per_1000: 0.0,
            avg_word_length: 0.0,
            pct_monosyllabic_words: 0.0,
            mattr_50: 0.0,
            mtld: 0.0,
        };
    }

    let sent_lens: Vec<usize> = sentences.iter().map(|s| word_tokens(s).len()).collect();
    let median = quartile(&sent_lens, 0.50);
    let p25 = quartile(&sent_lens, 0.25);
    let p75 = quartile(&sent_lens, 0.75);
    let n_short = sent_lens.iter().filter(|l| **l < 8).count();
    let n_long = sent_lens.iter().filter(|l| **l > 25).count();
    let n_dialog = sentences.iter().filter(|s| is_dialogue(s)).count();

    let common_set: std::collections::HashSet<&str> = COMMON_WORDS.iter().copied().collect();
    let n_rare = words
        .iter()
        .filter(|w| !common_set.contains(w.as_str()))
        .count();
    let unique: std::collections::HashSet<&str> = words.iter().map(String::as_str).collect();
    let n_mono = words.iter().filter(|w| count_syllables(w) == 1).count();
    let total_word_chars: usize = words.iter().map(String::len).sum();

    let n_em = text.matches('—').count();
    let n_semi = text.matches(';').count();
    let n_paren = text.matches('(').count();

    VoiceProfile {
        word_count: words.len() as u32,
        sentence_count: sentences.len() as u32,
        paragraph_count: paragraphs.len() as u32,
        median_sentence_length: median,
        p25_sentence_length: p25,
        p75_sentence_length: p75,
        pct_short_sentences: n_short as f32 / sent_lens.len() as f32,
        pct_long_sentences: n_long as f32 / sent_lens.len() as f32,
        dialogue_ratio: n_dialog as f32 / sentences.len() as f32,
        avg_paragraph_length_sentences: sentences.len() as f32 / paragraphs.len().max(1) as f32,
        type_token_ratio: unique.len() as f32 / words.len() as f32,
        rare_word_ratio: n_rare as f32 / words.len() as f32,
        em_dash_per_1000: 1000.0 * n_em as f32 / words.len() as f32,
        semicolon_per_1000: 1000.0 * n_semi as f32 / words.len() as f32,
        parenthetical_per_1000: 1000.0 * n_paren as f32 / words.len() as f32,
        avg_word_length: total_word_chars as f32 / words.len() as f32,
        pct_monosyllabic_words: n_mono as f32 / words.len() as f32,
        mattr_50: mattr(&words, 50),
        mtld: mtld(&words, 0.72),
    }
}

// ── Length-stable lexical-diversity helpers ───────────────────────────────
//
// Both algorithms operate on the lower-cased word-token list produced by
// `word_tokens` so they share the same vocabulary normalization the rest
// of the fingerprint uses. References:
//   - Covington & McFall (2010) — "Cutting the Gordian knot: The
//     Moving-Average Type-Token Ratio (MATTR)"
//   - McCarthy & Jarvis (2010) — "MTLD, vocd-D, and HD-D: A validation
//     study of sophisticated approaches to lexical diversity assessment"
//   - Zenker & Kyle (2025) — "Estimating lexical diversity using MATTR:
//     Pros and cons" (the modern stability re-evaluation)

/// Moving-Average Type-Token Ratio over a sliding window.
///
/// For each window position `i` in `[0, len - window]` compute
/// `len(unique_tokens(words[i..i+window])) / window`, then average
/// across all positions. Length-stable because every window has
/// exactly `window` tokens regardless of the underlying prose length.
///
/// Returns 0.0 if the prose has fewer than `window` tokens — the
/// window doesn't fit, so MATTR is not defined.
fn mattr(words: &[String], window: usize) -> f32 {
    if words.len() < window || window == 0 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    let mut n_windows = 0usize;
    for i in 0..=(words.len() - window) {
        let slice = &words[i..i + window];
        let unique: std::collections::HashSet<&str> = slice.iter().map(String::as_str).collect();
        sum += unique.len() as f32 / window as f32;
        n_windows += 1;
    }
    sum / n_windows.max(1) as f32
}

/// Measure of Textual Lexical Diversity (MTLD).
///
/// Walk the token stream forward, accumulating distinct types. Each
/// time the running TTR crosses below `threshold` (typically 0.72),
/// log a "factor" and reset. Final factor is fractional —
/// `(1.0 - last_ttr) / (1.0 - threshold)`. Repeat backwards. Return
/// the average of forward and backward `total_tokens / factors`.
///
/// Returns 0.0 if there are not enough tokens to form a single full
/// factor — empirical floor is ~50 tokens; below that the metric is
/// dominated by the fractional partial factor and reads as noise.
fn mtld(words: &[String], threshold: f32) -> f32 {
    if words.len() < 50 {
        return 0.0;
    }
    let forward = mtld_one_direction(words, threshold);
    let mut reversed: Vec<String> = words.to_vec();
    reversed.reverse();
    let backward = mtld_one_direction(&reversed, threshold);
    (forward + backward) / 2.0
}

fn mtld_one_direction(words: &[String], threshold: f32) -> f32 {
    let mut factors = 0.0_f32;
    let mut tokens_in_factor = 0usize;
    let mut types: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut last_ttr = 1.0_f32;
    for w in words {
        tokens_in_factor += 1;
        types.insert(w.clone());
        last_ttr = types.len() as f32 / tokens_in_factor as f32;
        if last_ttr <= threshold {
            factors += 1.0;
            tokens_in_factor = 0;
            types.clear();
            last_ttr = 1.0;
        }
    }
    // Add fractional final factor for the dangling partial.
    if tokens_in_factor > 0 {
        let denom = (1.0 - threshold).max(1e-6);
        factors += (1.0 - last_ttr) / denom;
    }
    // McCarthy & Jarvis (2010) convention: when no factor completes
    // because diversity never crosses the threshold, the metric is
    // *bounded above* by the total token count — the prose is "more
    // diverse than we can measure with this factor size." Returning
    // `words.len()` here is the sensible upper read; returning 0
    // (the literal division result) would falsely flag highly
    // diverse prose as low-diversity.
    if factors < 1.0 {
        return words.len() as f32;
    }
    words.len() as f32 / factors
}

/// Compute the stylometric distance (0-10, 10 = identical) between two
/// profiles. Weights match the Python reference implementation so
/// numbers from both sides are directly comparable.
pub fn stylometric_distance(a: &VoiceProfile, b: &VoiceProfile) -> StylometricDistance {
    fn diff(an: f32, bn: f32, scale: f32) -> f32 {
        (an - bn).abs() / scale.max(1e-6)
    }
    // (dim, delta_norm, weight) — weights sum to 1.0
    let raw: [(&str, f32, f32); 10] = [
        (
            "median_sentence_length",
            diff(a.median_sentence_length, b.median_sentence_length, 10.0),
            0.20,
        ),
        (
            "pct_short_sentences",
            diff(a.pct_short_sentences, b.pct_short_sentences, 0.5),
            0.12,
        ),
        (
            "pct_long_sentences",
            diff(a.pct_long_sentences, b.pct_long_sentences, 0.3),
            0.10,
        ),
        (
            "dialogue_ratio",
            diff(a.dialogue_ratio, b.dialogue_ratio, 0.5),
            0.12,
        ),
        (
            "avg_paragraph_length_sentences",
            diff(
                a.avg_paragraph_length_sentences,
                b.avg_paragraph_length_sentences,
                5.0,
            ),
            0.08,
        ),
        (
            "type_token_ratio",
            diff(a.type_token_ratio, b.type_token_ratio, 0.4),
            0.10,
        ),
        (
            "rare_word_ratio",
            diff(a.rare_word_ratio, b.rare_word_ratio, 0.4),
            0.08,
        ),
        (
            "em_dash_per_1000",
            diff(a.em_dash_per_1000, b.em_dash_per_1000, 8.0),
            0.07,
        ),
        (
            "avg_word_length",
            diff(a.avg_word_length, b.avg_word_length, 2.0),
            0.07,
        ),
        (
            "pct_monosyllabic_words",
            diff(a.pct_monosyllabic_words, b.pct_monosyllabic_words, 0.4),
            0.06,
        ),
    ];
    let weighted: f32 = raw.iter().map(|(_, d, w)| d * w).sum();
    let score = (10.0 - weighted * 10.0).max(0.0);
    StylometricDistance {
        distance_score_out_of_10: (score * 100.0).round() / 100.0,
        components: raw
            .iter()
            .map(|(d, dn, w)| StylometricComponent {
                dim: (*d).to_owned(),
                delta_norm: (dn * 1000.0).round() / 1000.0,
                weight: *w,
            })
            .collect(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Period-ending tokens that do NOT terminate a sentence. The published
/// rule-based-SBD baseline is ~95% accuracy; the dominant 5% of
/// boundary errors come from this list of abbreviations being treated
/// as full stops. FEATURE_HARDENING_PLAN.md §1.2.
///
/// Lower-cased for case-insensitive lookup. Includes both name titles
/// (Mr/Mrs/Dr/Prof), Latin abbreviations (e.g/i.e/etc/cf), corporate
/// suffixes (Inc/Ltd/Co/Corp), and time-of-day markers (a.m/p.m).
const SENTENCE_NON_BREAKING_ABBREVIATIONS: &[&str] = &[
    // Personal titles
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "rev", "fr", "st", "mt",
    // Military / civic ranks
    "capt", "sgt", "lt", "col", "gen", "maj", "adm", "cmdr", // Civic / legislative
    "hon", "sen", "rep", "gov", "pres", // Corporate
    "inc", "ltd", "co", "corp", "llc", "plc", // Latin / scholarly
    "etc", "vs", "cf", "viz", "et", "al", "ibid", // Days / months (terse forms)
    "mon", "tue", "wed", "thu", "fri", "sat", "sun", "jan", "feb", "mar", "apr", "jun", "jul",
    "aug", "sep", "sept", "oct", "nov", "dec", // Time-of-day
    "a.m", "p.m", "am", "pm", // Misc
    "vs.", "ph.d", "m.d", "b.a", "m.a", "u.s", "u.k",
];

/// True iff the period at byte index `period_idx` of `chars` follows
/// one of the [`SENTENCE_NON_BREAKING_ABBREVIATIONS`]. Walks left from
/// `period_idx` collecting alphabetic + period chars and lookups
/// against the gazetteer.
fn period_follows_abbreviation(chars: &[char], period_idx: usize) -> bool {
    // Walk left collecting an alphanumeric+dot run.
    let mut start = period_idx;
    while start > 0 {
        let c = chars[start - 1];
        if c.is_ascii_alphabetic() || c == '.' {
            start -= 1;
        } else {
            break;
        }
    }
    // The run is `chars[start..=period_idx]` — drop the trailing period
    // so we can compare against gazetteer entries which omit the
    // terminating dot.
    if start >= period_idx {
        return false;
    }
    let token: String = chars[start..period_idx]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    SENTENCE_NON_BREAKING_ABBREVIATIONS
        .iter()
        .any(|abbr| *abbr == token)
}

/// Split prose into sentence-shaped strings. Public so sibling
/// L3 crates (`booksforge-anti-ai-tells`, planner, polish stages)
/// can score the same sentence boundaries as the fingerprint pass.
///
/// Quote-aware: does NOT break inside `"..."` / `\u{201C}...\u{201D}`
/// runs — `She said "No." and left.` is one sentence, not two.
/// Abbreviation-aware: does NOT break after entries in
/// [`SENTENCE_NON_BREAKING_ABBREVIATIONS`] (Mr., Mrs., Dr., etc.).
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    for i in 0..chars.len() {
        let c = chars[i];
        cur.push(c);
        // Track quote-nesting depth. ASCII quotes toggle; smart quotes
        // open/close explicitly.
        match c {
            '"' => in_double_quote = !in_double_quote,
            '\u{201C}' => in_double_quote = true,
            '\u{201D}' => in_double_quote = false,
            '\u{2018}' => in_single_quote = true,
            '\u{2019}' => in_single_quote = false,
            _ => {}
        }
        if !matches!(c, '.' | '!' | '?') {
            continue;
        }
        // Suppress break inside any quote run (dialogue "No." and
        // left. is one sentence).
        if in_double_quote || in_single_quote {
            continue;
        }
        // Suppress break when the period closes a known abbreviation.
        if c == '.' && period_follows_abbreviation(&chars, i) {
            continue;
        }
        // Look ahead for whitespace + capital / quote (the existing rule).
        let mut j = i + 1;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j < chars.len() {
            let next = chars[j];
            if next.is_ascii_uppercase()
                || next == '"'
                || next == '\u{201C}' /* “ */
                || next == '\''
            {
                let trimmed = cur.trim().to_owned();
                if !trimmed.is_empty() {
                    out.push(trimmed);
                }
                cur.clear();
            }
        }
    }
    let trimmed = cur.trim().to_owned();
    if !trimmed.is_empty() {
        out.push(trimmed);
    }
    out
}

/// Lowercase alphabetic word tokens. Public for the same reason
/// `split_sentences` is — sibling L3 crates need stable tokenization.
pub fn word_tokens(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_ascii_alphabetic() || c == '\'' || c == '-' {
            cur.push(c.to_ascii_lowercase());
        } else if !cur.is_empty() {
            // Drop pure-punctuation residue at the edges.
            let stripped: String = cur
                .trim_matches(|ch: char| ch == '\'' || ch == '-')
                .to_owned();
            if !stripped.is_empty() {
                out.push(stripped);
            }
            cur.clear();
        }
    }
    if !cur.is_empty() {
        let stripped: String = cur
            .trim_matches(|ch: char| ch == '\'' || ch == '-')
            .to_owned();
        if !stripped.is_empty() {
            out.push(stripped);
        }
    }
    out
}

fn is_dialogue(s: &str) -> bool {
    s.contains('"') || s.contains('\u{201C}') || s.contains('\u{201D}')
}

fn count_syllables(word: &str) -> usize {
    // Estimate: count contiguous vowel groups. Floor at 1.
    let mut count = 0usize;
    let mut in_vowel = false;
    for c in word.chars() {
        let is_v = matches!(
            c,
            'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'A' | 'E' | 'I' | 'O' | 'U' | 'Y'
        );
        if is_v {
            if !in_vowel {
                count += 1;
            }
            in_vowel = true;
        } else {
            in_vowel = false;
        }
    }
    count.max(1)
}

fn quartile(xs: &[usize], q: f32) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut s: Vec<usize> = xs.to_vec();
    s.sort_unstable();
    let pos = (s.len() as f32 - 1.0) * q;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(s.len() - 1);
    let frac = pos - lo as f32;
    s[lo] as f32 * (1.0 - frac) + s[hi] as f32 * frac
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_short_text() {
        let sample = "She walked into the kitchen. The light was off. \
                      \"You up?\" she said. Nothing.";
        let p = fingerprint(sample);
        assert!(p.word_count > 0);
        assert!(p.sentence_count >= 4);
        assert!(p.dialogue_ratio > 0.0); // one dialogue sentence
    }

    #[test]
    fn fingerprint_handles_empty_text() {
        let p = fingerprint("");
        assert_eq!(p.word_count, 0);
        assert_eq!(p.median_sentence_length, 0.0);
    }

    #[test]
    fn syllable_counter_floors_at_one() {
        assert_eq!(count_syllables("cat"), 1);
        assert_eq!(count_syllables("rhythm"), 1); // no vowels — floored
        assert!(count_syllables("multifaceted") >= 4);
    }

    #[test]
    fn distance_to_self_is_max() {
        let p = fingerprint("This is a sample. Two short sentences here.");
        let d = stylometric_distance(&p, &p);
        assert_eq!(d.distance_score_out_of_10, 10.0);
        assert_eq!(d.components.len(), 10);
    }

    #[test]
    fn distance_decreases_for_different_voices() {
        // Punchy, short, monosyllabic
        let punchy = "He ran. Hard. Past the gate. Past the church. \
                      The rain hit hard. He kept on. He did not stop.";
        // Long, formal, multisyllabic
        let formal = "He propelled himself forward, navigating the gravelled \
                      passageway, traversing the ecclesiastical perimeter \
                      while precipitation accumulated upon his outerwear, \
                      yet continuing in his determined trajectory.";
        let p_punchy = fingerprint(punchy);
        let p_formal = fingerprint(formal);
        let d = stylometric_distance(&p_punchy, &p_formal);
        // The distance score is NOT 10 — different voices.
        assert!(d.distance_score_out_of_10 < 9.0);
    }

    #[test]
    fn constraints_block_renders_all_dimensions() {
        let p = fingerprint("Short. Then a much longer sentence with more text inside it.");
        let block = p.constraints_block("comp");
        assert!(block.contains("Median sentence length"));
        assert!(block.contains("Em-dashes per 1000 words"));
        assert!(block.contains("Monosyllabic-word share"));
        assert!(block.contains("from comp:"));
    }

    #[test]
    fn dialogue_share_detected() {
        let dialog_heavy = "\"You up?\" she said. \"I am,\" he said.";
        let prose_only = "She walked. He waited. The room was quiet.";
        let pd = fingerprint(dialog_heavy);
        let pp = fingerprint(prose_only);
        assert!(pd.dialogue_ratio > pp.dialogue_ratio);
    }

    // ── MATTR + MTLD (FEATURE_HARDENING_PLAN.md §1.1) ────────────────────

    /// Generate the i-th distinct purely-alphabetic word (a, b, …, z,
    /// aa, ab, …) — the tokenizer strips digits, so we can't use
    /// `tok_5`-style names without all of them collapsing to `tok`.
    fn nth_word(i: usize) -> String {
        let mut k = i + 1;
        let mut s = String::new();
        while k > 0 {
            let r = (k - 1) % 26;
            s.push((b'a' + r as u8) as char);
            k = (k - 1) / 26;
        }
        s.chars().rev().collect()
    }

    /// `n` sentences of 5 unique alphabetic tokens each. Total token
    /// count is `5 * n`, all distinct.
    fn fully_unique_sentences(n: usize) -> String {
        let mut out = String::new();
        let mut idx = 0usize;
        for _ in 0..n {
            let mut words: Vec<String> = (0..5)
                .map(|_| {
                    let w = nth_word(idx);
                    idx += 1;
                    w
                })
                .collect();
            // Capitalize first letter so split_sentences keeps the boundary.
            if let Some(first) = words.first_mut() {
                if let Some(c) = first.chars().next() {
                    *first = format!("{}{}", c.to_ascii_uppercase(), &first[1..]);
                }
            }
            out.push_str(&words.join(" "));
            out.push_str(". ");
        }
        out.trim().to_owned()
    }

    /// Build N sentences that all repeat the same 4 words.
    fn fully_repetitive_sentences(n: usize) -> String {
        (0..n)
            .map(|_| "The cat sat down.")
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn mattr_below_window_size_is_zero() {
        // Window 50 doesn't fit in a 30-token text.
        let words: Vec<String> = (0..30).map(|i| format!("w{i}")).collect();
        assert_eq!(mattr(&words, 50), 0.0);
    }

    #[test]
    fn mattr_is_length_stable_for_constant_diversity() {
        // The literature's headline claim: MATTR (window 50) gives
        // approximately the same value for short and long texts of the
        // same vocabulary structure. Raw TTR drops as length grows
        // (it's bounded above by 1 and types saturate); MATTR doesn't.
        let short = fingerprint(&fully_unique_sentences(80));
        let long = fingerprint(&fully_unique_sentences(400));
        // Raw TTR drops noticeably with length (more repeated function words).
        // MATTR stays within ~0.05 of itself.
        let delta = (long.mattr_50 - short.mattr_50).abs();
        assert!(
            delta < 0.10,
            "mattr should be length-stable; |Δ| = {delta} between 80 and 400 tokens (short {:.3}, long {:.3})",
            short.mattr_50, long.mattr_50,
        );
        assert!(
            short.mattr_50 > 0.0,
            "short text mattr was {:.3}",
            short.mattr_50
        );
    }

    #[test]
    fn mattr_low_for_repetitive_prose_high_for_diverse() {
        let repetitive = fingerprint(&fully_repetitive_sentences(40));
        let diverse = fingerprint(&fully_unique_sentences(80));
        // Repetitive: same 4 words in every window → MATTR should be very low.
        // Diverse: each window has all-unique tokens → MATTR should be very high.
        assert!(
            diverse.mattr_50 > repetitive.mattr_50 + 0.30,
            "diverse should beat repetitive by ≥ 0.30 (got diverse {:.3}, repetitive {:.3})",
            diverse.mattr_50,
            repetitive.mattr_50,
        );
    }

    // ── Sentence boundary detection (FEATURE_HARDENING_PLAN.md §1.2) ─────

    #[test]
    fn sbd_does_not_break_after_mr_dr_mrs() {
        // Run #11-style problem: every "Dr. Smith" used to split after Dr.
        let text = "Dr. Smith arrived. Mr. Jones followed. Mrs. Hill waved.";
        let s = split_sentences(text);
        assert_eq!(s.len(), 3, "expected 3 sentences, got {}: {:?}", s.len(), s);
    }

    #[test]
    fn sbd_does_not_break_after_etc_and_eg_when_lower_followed() {
        // Common Latin abbreviations followed by lower-cased continuation
        // get treated as non-breaking. (When followed by a Capital they
        // legitimately end a sentence; that case is not affected here.)
        let text = "We had bread, cheese, etc. and then the wine arrived.";
        let s = split_sentences(text);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn sbd_does_not_break_inside_quoted_dialogue() {
        // The classic dialogue case: a period inside the quotes is not
        // a sentence break; the surrounding sentence continues.
        let text = "She said \"No.\" and left the room.";
        let s = split_sentences(text);
        assert_eq!(s.len(), 1, "got: {s:?}");
    }

    #[test]
    fn sbd_smart_quotes_also_suppress_breaks() {
        let text = "She said \u{201C}No.\u{201D} and left the room.";
        let s = split_sentences(text);
        assert_eq!(s.len(), 1, "got: {s:?}");
    }

    #[test]
    fn sbd_normal_sentences_still_split() {
        let text = "She walked in. He waited. Nothing happened. The room was quiet.";
        let s = split_sentences(text);
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn sbd_dialogue_followed_by_normal_sentence_splits_correctly() {
        // The harder case: after the quoted dialogue closes there IS a
        // legitimate sentence boundary.
        let text = "\"You up?\" she said. He did not answer.";
        let s = split_sentences(text);
        assert_eq!(s.len(), 2, "got: {s:?}");
    }

    #[test]
    fn mtld_low_for_repetitive_prose_high_for_diverse() {
        let repetitive = fingerprint(&fully_repetitive_sentences(60));
        let diverse = fingerprint(&fully_unique_sentences(120));
        // MTLD = total_tokens / factors. Diverse prose racks up factors
        // very slowly (TTR stays high), so MTLD is large. Repetitive
        // prose racks up factors quickly, so MTLD is small.
        assert!(
            diverse.mtld > repetitive.mtld,
            "diverse MTLD {:.3} should beat repetitive MTLD {:.3}",
            diverse.mtld,
            repetitive.mtld,
        );
        // Sanity floor: diverse prose should be far above the threshold default.
        assert!(diverse.mtld > 50.0);
    }
}
