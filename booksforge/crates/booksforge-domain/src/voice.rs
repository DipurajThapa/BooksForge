//! Voice fingerprint — a structural description of the project's prose
//! voice that prose-emitting agents (chapter-drafter, copyeditor,
//! humanization, final-review-editor) consume so their output sounds
//! like the same author wrote it.
//!
//! Computed lazily from accepted scenes and refreshed by Memory Curator
//! on chapter finalise.  Stored as JSON in `style_memory.voice_json`.
//!
//! The fingerprint is **structural, not stylistic content** — it
//! captures *how* the author writes (sentence cadence, vocabulary
//! richness, em-dash density), not *what* they write about (which is
//! the StyleBook + memory's job).
//!
//! Anti-AI-tell intent: the fingerprint includes signals known to
//! distinguish human prose from LLM prose:
//!   - Sentence-length variability (LLMs produce more uniform lengths)
//!   - Em-dash density (LLMs over-use them)
//!   - Adverb density (LLMs over-use `-ly` adverbs, especially "carefully")
//!   - Three-tell triad: "delve / tapestry / intricate" usage rate
//!   - Discourse marker rate ("indeed", "moreover", "furthermore" — LLMs love these)

use serde::{Deserialize, Serialize};

/// Captured statistics about the project's prose voice.
///
/// All fields are non-negative and finite.  `compute()` produces these
/// from a corpus of accepted prose; agents read them via prompt
/// template injection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceFingerprint {
    /// Mean and standard deviation of sentence length (in words).
    /// Human prose typically has SD ≥ 0.6 × mean.  LLM prose has lower
    /// SD because models prefer "balanced" sentences.
    pub sentence_words_mean: f32,
    pub sentence_words_stddev: f32,

    /// Em-dashes per 1,000 words.  LLM-generated prose tends to use
    /// ~3–5×; calibrated humans range 0.5–4.
    pub em_dash_per_1000: f32,

    /// Adverbs ending in `-ly` per 1,000 words.  LLMs over-use.
    pub ly_adverb_per_1000: f32,

    /// Combined rate of three classic AI-tells per 1,000 words
    /// (`delve`, `tapestry`, `intricate`).  Should be near zero in
    /// human prose; spikes in LLM prose.
    pub ai_tell_triad_per_1000: f32,

    /// Discourse markers (indeed/moreover/furthermore/thus) per 1,000 words.
    pub discourse_marker_per_1000: f32,

    /// Ratio of distinct lemmas to total tokens — vocabulary richness.
    /// Cliché-heavy prose (LLM-typical) clusters around 0.35–0.42 for
    /// 1,000-word samples; voice-rich human prose tends 0.45–0.55.
    pub type_token_ratio: f32,

    /// Token count the fingerprint was computed from.  Below ~2,000
    /// tokens the signal is too noisy to use; consumers should treat
    /// `corpus_tokens < 2_000` as "fingerprint not yet established".
    pub corpus_tokens: u32,
}

impl Default for VoiceFingerprint {
    /// "Unknown voice" defaults — agents that read this should treat
    /// `corpus_tokens == 0` as "no fingerprint, use generic prose
    /// guidelines."
    fn default() -> Self {
        Self {
            sentence_words_mean: 16.0,
            sentence_words_stddev: 8.0,
            em_dash_per_1000: 1.5,
            ly_adverb_per_1000: 12.0,
            ai_tell_triad_per_1000: 0.0,
            discourse_marker_per_1000: 1.0,
            type_token_ratio: 0.45,
            corpus_tokens: 0,
        }
    }
}

impl VoiceFingerprint {
    pub fn is_established(&self) -> bool {
        self.corpus_tokens >= 2_000
    }

    /// Compute a fingerprint from a plain-text corpus.
    pub fn compute(text: &str) -> Self {
        let words: Vec<&str> = text.split_whitespace().collect();
        let total_words = words.len() as u32;
        if total_words == 0 {
            return Self::default();
        }

        // Sentence segmentation: naive split on `.`, `!`, `?` followed by
        // whitespace+capital.  Sufficient for fingerprint statistics.
        let mut sentences: Vec<usize> = Vec::new();
        let mut current = 0usize;
        let mut prev_end = false;
        for w in &words {
            current += 1;
            let last = w.chars().last().unwrap_or(' ');
            if matches!(last, '.' | '!' | '?') {
                prev_end = true;
            } else if prev_end {
                if current > 1 {
                    sentences.push(current - 1);
                    current = 1;
                }
                prev_end = false;
            }
        }
        if current > 0 {
            sentences.push(current);
        }

        let (sent_mean, sent_sd) = mean_stddev(&sentences);

        // Per-1000-word signals.
        let em_dash_count = text.matches('—').count() as f32;
        let ly_adverbs = words
            .iter()
            .filter(|w| {
                let lower = w.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
                lower.len() > 3 && lower.ends_with("ly")
            })
            .count() as f32;
        let lower = text.to_ascii_lowercase();
        let triad_count = ["delve", "tapestry", "intricate"]
            .iter()
            .map(|t| count_word(&lower, t))
            .sum::<usize>() as f32;
        let disc_count = ["indeed", "moreover", "furthermore", "thus", "hence"]
            .iter()
            .map(|t| count_word(&lower, t))
            .sum::<usize>() as f32;

        let scale = 1_000.0 / total_words as f32;

        // Type-token ratio — distinct lower-case alphabetic tokens / total.
        use std::collections::HashSet;
        let lemmas: HashSet<String> = words
            .iter()
            .map(|w| w.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let ttr = lemmas.len() as f32 / total_words.max(1) as f32;

        Self {
            sentence_words_mean: sent_mean,
            sentence_words_stddev: sent_sd,
            em_dash_per_1000: em_dash_count * scale,
            ly_adverb_per_1000: ly_adverbs * scale,
            ai_tell_triad_per_1000: triad_count * scale,
            discourse_marker_per_1000: disc_count * scale,
            type_token_ratio: ttr,
            corpus_tokens: total_words,
        }
    }
}

fn mean_stddev(xs: &[usize]) -> (f32, f32) {
    if xs.is_empty() {
        return (0.0, 0.0);
    }
    let n = xs.len() as f32;
    let mean = xs.iter().sum::<usize>() as f32 / n;
    let var = xs
        .iter()
        .map(|x| {
            let d = *x as f32 - mean;
            d * d
        })
        .sum::<f32>()
        / n;
    (mean, var.sqrt())
}

fn count_word(haystack_lower: &str, needle: &str) -> usize {
    let mut n = 0;
    for token in haystack_lower.split(|c: char| !c.is_alphabetic()) {
        if token == needle {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unestablished() {
        assert!(!VoiceFingerprint::default().is_established());
    }

    #[test]
    fn compute_counts_em_dashes() {
        let text = "She paused — and then ran. He watched — and waited.";
        let fp = VoiceFingerprint::compute(text);
        assert!(
            fp.em_dash_per_1000 > 100.0,
            "two em-dashes in tiny corpus → high rate"
        );
    }

    #[test]
    fn compute_flags_ai_tell_triad() {
        let text = "She would delve into the intricate tapestry of her mind.";
        let fp = VoiceFingerprint::compute(text);
        assert!(fp.ai_tell_triad_per_1000 > 100.0);
    }

    #[test]
    fn compute_returns_default_on_empty_input() {
        let fp = VoiceFingerprint::compute("");
        assert_eq!(fp, VoiceFingerprint::default());
    }

    #[test]
    fn type_token_ratio_lower_for_repetitive_text() {
        let repetitive = "the the the the the the the the the the".to_string()
            + " the the the the the the the the the the";
        let varied = "the quick brown fox jumps over the lazy dog beside ".to_string()
            + "a sleeping cat under bright stars while wind whispers through";
        let fp_rep = VoiceFingerprint::compute(&repetitive);
        let fp_var = VoiceFingerprint::compute(&varied);
        assert!(fp_var.type_token_ratio > fp_rep.type_token_ratio);
    }
}
