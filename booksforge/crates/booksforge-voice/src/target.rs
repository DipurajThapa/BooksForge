//! Prescriptive voice targets — the *contract* a draft must satisfy.
//!
//! Where [`crate::VoiceProfile`] *describes* prose that already exists
//! ("median sentence length 5"), [`VoiceTarget`] *prescribes* prose that
//! must be written ("60% of sentences must be ≤ 8 words; at least 15%
//! must be ≥ 20 words; no more than 3 consecutive sentences may share
//! the same 3-word opening").
//!
//! This is the architectural fix for the Run #11 monotony finding
//! (`book-output/integrated-runs/20260509-221828/quality-review.md`):
//! freeform voice descriptions like *"sentences alternate between short
//! staccato fragments and long winding observations"* produced uniformly
//! short prose because the model read the constraint as a ceiling
//! ("under 8 words") and dropped the alternation. Numeric bands force
//! both ends of the distribution.
//!
//! ### Lifecycle
//!
//! 1. The bible / genre pack defines a [`VoiceTarget`].
//! 2. [`VoiceTarget::directive_block`] is rendered into the drafter
//!    prompt as a hard contract.
//! 3. After the draft lands, [`VoiceTarget::score`] runs against the
//!    measured [`crate::VoiceProfile`] + raw text. Failures are surfaced
//!    in the audit ledger and (in the planner-driven polish stack) fed
//!    to a `rhythm_polish` stage as targeted re-write instructions.
//!
//! No new dependencies — uses the same internal sentence splitter as
//! `fingerprint`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{split_sentences, word_tokens, VoiceProfile};

/// One bucket in a sentence-length distribution. `max_words` is
/// *inclusive*; use `u32::MAX` for an unbounded upper end. `target_share`
/// is the fraction of the prose's sentences that should fall in this
/// bucket (0..1). `tolerance` is the +/- band — `target=0.60,
/// tolerance=0.10` accepts anything in `[0.50, 0.70]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SentenceLengthBucket {
    pub min_words: u32,
    pub max_words: u32,
    pub target_share: f32,
    pub tolerance: f32,
}

impl SentenceLengthBucket {
    pub fn contains(&self, words: u32) -> bool {
        words >= self.min_words && words <= self.max_words
    }

    pub fn passes(&self, actual_share: f32) -> bool {
        let lo = (self.target_share - self.tolerance).max(0.0);
        let hi = (self.target_share + self.tolerance).min(1.0);
        actual_share >= lo && actual_share <= hi
    }
}

/// Prescriptive voice contract. All fields are *targets the draft must
/// satisfy*, not statistics measured from existing prose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceTarget {
    /// Human-readable label rendered into the prompt directive
    /// (e.g. `"literary fiction — Cormac McCarthy / Marilynne Robinson cluster"`).
    pub label: String,

    /// Sentence-length distribution. Buckets need not be exhaustive (the
    /// scorer only measures bucket coverage; uncovered ranges contribute
    /// to neither pass nor fail).
    pub sentence_length_buckets: Vec<SentenceLengthBucket>,

    /// Type-token ratio floor (vocabulary richness, 0..1). Literary
    /// norm is ~0.45+; Run #11's collapsed prose hit 0.303.
    ///
    /// **Length-biased.** For cross-run comparisons use [`Self::mattr_min`]
    /// instead — TTR is here for back-compat only.
    pub type_token_ratio_min: f32,

    /// Moving-Average TTR (window 50) floor — the length-stable
    /// counterpart to [`Self::type_token_ratio_min`]. Literature
    /// suggests 0.66 as the threshold separating literary from
    /// generic prose for this window size. Default `0.0` = unenforced.
    /// FEATURE_HARDENING_PLAN.md §1.1.
    #[serde(default)]
    pub mattr_min: f32,

    /// Maximum em-dashes per 1000 words. The Run #11 critique flagged
    /// em-dash overuse as a separate AI-tell; this cap lets a target
    /// allow none, some, or many depending on the voice.
    pub em_dash_per_1000_max: f32,

    /// Maximum number of *consecutive* sentences that may begin with
    /// the same 3-word opening clause. Anaphora is a real device but
    /// past 4 repetitions becomes machine-like (Run #11 had 8 and 11).
    pub repeated_opening_max: u32,

    /// Companion cap for 2-token anaphora — `She fell. She rose. She
    /// fell. She rose.` is invisible to the 3-token check (the third
    /// token differs each time) but is the same machine-rhythm tell.
    /// Default `0` = unenforced. FEATURE_HARDENING_PLAN.md §1.4.
    #[serde(default)]
    pub repeated_opening_max_2tok: u32,

    /// Companion cap for 4-token anaphora — `The hand on the door. The
    /// hand on the lock. The hand on the chain.` Default `0` =
    /// unenforced. FEATURE_HARDENING_PLAN.md §1.4.
    #[serde(default)]
    pub repeated_opening_max_4tok: u32,

    /// At least this many sentences in every paragraph (≥ 3 sentences;
    /// FEATURE_HARDENING_PLAN.md §1.5 dropped the floor from 4 → 3)
    /// must be ≥ 18 words long. Forces the long-sentence end of the
    /// distribution to actually appear, not just be tolerated.
    pub min_long_sentences_per_paragraph: u32,

    /// **Anti-bimodal-collapse interleaving requirement.** Every
    /// paragraph of 4+ sentences must touch at least this many
    /// distinct length-buckets across its sentences. The Run #12
    /// failure mode was 84% short / 0% medium / 16% long — the
    /// drafter satisfied the aggregate distribution but skipped the
    /// medium band entirely (one paragraph was 30 short sentences in
    /// a row, others were one long run-on each). Setting this to 2
    /// forces every long paragraph to interleave at least two of
    /// {short, medium, long}; setting it to 3 forces all three.
    /// Default `0` = unenforced.
    /// FEATURE_HARDENING_PLAN-RUN12.md §1 + Run #12 quality review §8.
    #[serde(default)]
    pub min_band_coverage_per_paragraph: u8,

    /// If true, at least one dialogue sentence is required. Scene-card
    /// driven — most scenes have dialogue, some interior scenes don't.
    /// **Note:** for literary fiction this counts interior monologue
    /// rendered as dialogue (`"What was she doing?"` she thought).
    /// Run #12 produced zero dialogue and the scene goal would have
    /// been improved by 1-2 lines of internal speech.
    pub require_dialogue: bool,
}

impl VoiceTarget {
    /// Literary-fiction default: reads to roughly the IQR profile of
    /// the comp-corpus literary cluster (Robinson, McCarthy, Strout).
    /// The bands are deliberately wide enough that any natural
    /// rhythm passes; only mechanical uniformity (Run #11) fails.
    pub fn literary_default() -> Self {
        Self {
            label: "literary fiction (literary cluster)".to_owned(),
            sentence_length_buckets: vec![
                SentenceLengthBucket {
                    min_words: 0,
                    max_words: 8,
                    target_share: 0.40,
                    tolerance: 0.10, // Run #12: tightened 0.15 → 0.10
                },
                SentenceLengthBucket {
                    min_words: 9,
                    max_words: 17,
                    target_share: 0.35,
                    tolerance: 0.15,
                },
                SentenceLengthBucket {
                    min_words: 18,
                    max_words: u32::MAX,
                    target_share: 0.20,
                    tolerance: 0.10,
                },
            ],
            // Run #12 calibration:
            //   - TTR floor unenforced (length-biased; trust MATTR).
            //     The Run #12 prose scored TTR 0.357 (would fail) but
            //     MATTR 0.69 (passes), and reads as varied to a human.
            //     Disagreement is itself a signal — TTR penalises
            //     deliberate concrete-noun anchoring which bibles
            //     explicitly call for.
            //   - Short-bucket tolerance tightened 0.15 → 0.10. Run
            //     #12 hit 0.84 in the short bucket against a 0.40
            //     target — the wider band silently absorbed a 70%
            //     over-allocation. 0.10 means anything outside
            //     [0.30, 0.50] fails loudly.
            //   - require_dialogue flipped to true. The §6 Run #12
            //     review noted the scene was improved by 1-2 lines of
            //     interior speech rendered as dialogue.
            //   - min_band_coverage_per_paragraph = 2. Forces the
            //     interleaving the bimodal collapse violated.
            type_token_ratio_min: 0.0,
            mattr_min: 0.66,
            em_dash_per_1000_max: 8.0,
            repeated_opening_max: 3,
            repeated_opening_max_2tok: 4,
            repeated_opening_max_4tok: 2,
            min_long_sentences_per_paragraph: 1,
            // Run #12 → Run #13 retighten: bumped 2 → 3. The Run #12
            // failure was bimodal short+long with zero medium-length
            // sentences in any paragraph; a min of 2 still admitted
            // that (short+long touches 2 bands). 3 forces every 4+
            // sentence paragraph to interleave all three bands —
            // the actual Run #11/§8 fix the quality review proposed.
            min_band_coverage_per_paragraph: 3,
            require_dialogue: true,
        }
    }

    /// Commercial / upmarket default: shorter overall, more dialogue,
    /// less vocabulary richness. Calibrated to the Picoult / Hannah
    /// cluster.
    pub fn commercial_default() -> Self {
        Self {
            label: "commercial / upmarket fiction".to_owned(),
            sentence_length_buckets: vec![
                SentenceLengthBucket {
                    min_words: 0,
                    max_words: 8,
                    target_share: 0.50,
                    tolerance: 0.15,
                },
                SentenceLengthBucket {
                    min_words: 9,
                    max_words: 16,
                    target_share: 0.35,
                    tolerance: 0.15,
                },
                SentenceLengthBucket {
                    min_words: 17,
                    max_words: u32::MAX,
                    target_share: 0.10,
                    tolerance: 0.08,
                },
            ],
            // Run #12 calibration mirrors literary_default: TTR
            // unenforced; require_dialogue stays true (commercial
            // fiction is dialogue-heavy by definition).
            type_token_ratio_min: 0.0,
            mattr_min: 0.62,
            em_dash_per_1000_max: 4.0,
            repeated_opening_max: 4,
            repeated_opening_max_2tok: 5,
            repeated_opening_max_4tok: 3,
            min_long_sentences_per_paragraph: 0,
            min_band_coverage_per_paragraph: 2,
            require_dialogue: true,
        }
    }

    /// Punchy / action default: short-dominant, low TTR tolerance,
    /// minimal punctuation variation. Calibrated to the Lee Child /
    /// thriller cluster.
    pub fn punchy_action_default() -> Self {
        Self {
            label: "punchy thriller / action".to_owned(),
            sentence_length_buckets: vec![
                SentenceLengthBucket {
                    min_words: 0,
                    max_words: 6,
                    target_share: 0.55,
                    tolerance: 0.15,
                },
                SentenceLengthBucket {
                    min_words: 7,
                    max_words: 14,
                    target_share: 0.35,
                    tolerance: 0.15,
                },
                SentenceLengthBucket {
                    min_words: 15,
                    max_words: u32::MAX,
                    target_share: 0.08,
                    tolerance: 0.06,
                },
            ],
            // Run #12 calibration: TTR unenforced; punchy admits
            // tighter band coverage (1) because thrillers legitimately
            // run pages of all-short or all-long passages — the
            // bimodal-collapse failure is a literary-fiction problem,
            // not a thriller one.
            type_token_ratio_min: 0.0,
            mattr_min: 0.58,
            em_dash_per_1000_max: 2.0,
            repeated_opening_max: 5,
            repeated_opening_max_2tok: 6,
            repeated_opening_max_4tok: 4,
            min_long_sentences_per_paragraph: 0,
            min_band_coverage_per_paragraph: 1,
            require_dialogue: false,
        }
    }

    /// Render the target as a prompt directive for the drafter. The
    /// language is deliberately prescriptive ("must", "may not"), not
    /// descriptive — that distinction is the Run #11 fix.
    pub fn directive_block(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Voice contract — {}:\n", self.label));
        out.push_str("Your prose MUST satisfy these distributional bands:\n");
        for b in &self.sentence_length_buckets {
            let upper = if b.max_words == u32::MAX {
                "or more".to_owned()
            } else {
                format!("to {}", b.max_words)
            };
            let pct = (b.target_share * 100.0).round() as i32;
            let tol = (b.tolerance * 100.0).round() as i32;
            out.push_str(&format!(
                "  - {pct}% (+/- {tol}%) of sentences must be {} {} words long.\n",
                b.min_words, upper,
            ));
        }
        if self.type_token_ratio_min > 0.0 {
            out.push_str(&format!(
                "  - Type-token ratio must be at least {:.2} (vocabulary richness floor).\n",
                self.type_token_ratio_min,
            ));
        }
        if self.mattr_min > 0.0 {
            out.push_str(&format!(
                "  - Length-stable lexical diversity (MATTR-50) must be at least {:.2}.\n",
                self.mattr_min,
            ));
        }
        out.push_str(&format!(
            "  - Em-dashes per 1000 words must not exceed {:.1}.\n",
            self.em_dash_per_1000_max,
        ));
        out.push_str(&format!(
            "  - No more than {} consecutive sentences may begin with the same 3-word opening clause.\n",
            self.repeated_opening_max,
        ));
        if self.repeated_opening_max_2tok > 0 {
            out.push_str(&format!(
                "  - No more than {} consecutive sentences may begin with the same 2-word opening clause.\n",
                self.repeated_opening_max_2tok,
            ));
        }
        if self.repeated_opening_max_4tok > 0 {
            out.push_str(&format!(
                "  - No more than {} consecutive sentences may begin with the same 4-word opening clause.\n",
                self.repeated_opening_max_4tok,
            ));
        }
        if self.min_long_sentences_per_paragraph > 0 {
            out.push_str(&format!(
                "  - Every paragraph of 3+ sentences must contain at least {} sentence(s) of 18+ words.\n",
                self.min_long_sentences_per_paragraph,
            ));
        }
        if self.min_band_coverage_per_paragraph > 0 {
            let n_bands = self.sentence_length_buckets.len();
            if self.min_band_coverage_per_paragraph as usize >= n_bands {
                // Strongest version: "every band, every paragraph."
                out.push_str(
                    "  - **MANDATORY INTERLEAVING**: every paragraph of 4+ \
                     sentences MUST contain at least one sentence from EACH \
                     length band: at least one short (≤8 words), at least one \
                     medium (9-17 words), AND at least one long (18+ words). \
                     This is a hard requirement, not a guideline. A paragraph \
                     that hits only short+long with zero medium sentences \
                     FAILS the contract — the medium band is the connective \
                     tissue of normal prose; skipping it produces the \
                     bimodal-collapse failure (84% short / 0% medium / 16% \
                     long) observed in Run #12.\n",
                );
                out.push_str(
                    "  - DO NOT produce paragraphs of all-short fragments \
                     (the Run #11 \"the hand held the X. the hand held the Y.\" \
                     pattern). DO NOT produce paragraphs that are one giant \
                     run-on sentence with internal commas (the Run #12 \
                     paragraphs 2-5 pattern). Mix the three bands sentence by \
                     sentence within every paragraph.\n",
                );
            } else {
                out.push_str(&format!(
                    "  - INTERLEAVING REQUIREMENT: every paragraph of 4+ sentences must touch at least {} of the {} length bands. Do NOT produce a burst of one length followed by a burst of another length — alternate within paragraphs sentence-by-sentence.\n",
                    self.min_band_coverage_per_paragraph, n_bands,
                ));
            }
        }
        if self.require_dialogue {
            out.push_str(
                "  - At least one sentence must contain dialogue. For literary \
                 scenes without spoken speech, render interior monologue as \
                 dialogue: e.g. \"What was she doing?\" she thought.\n",
            );
        }
        out.push_str(
            "\nThese are NOT preferences. They are the contract. Plan the scene's \
             rhythm before drafting so the bands are met. A useful mental model: \
             *for every 10 sentences you write, ~4 must be short, ~3-4 must be \
             medium-length, and ~2 must be long*. If you find yourself writing \
             three short sentences in a row, the next sentence MUST be medium \
             or long. Anaphora is permitted as a deliberate device but only \
             within the consecutive-opening cap.\n",
        );
        out
    }

    /// Score a measured profile + raw text against this target. The
    /// returned [`VoiceScore`] flags every failed dimension; an empty
    /// `failed_dimensions` list means the draft passes.
    pub fn score(&self, profile: &VoiceProfile, text: &str) -> VoiceScore {
        let sentences = split_sentences(text);
        let sent_lens: Vec<u32> = sentences
            .iter()
            .map(|s| word_tokens(s).len() as u32)
            .collect();
        let total = sent_lens.len().max(1) as f32;

        let bucket_scores: Vec<BucketScore> = self
            .sentence_length_buckets
            .iter()
            .map(|b| {
                let n = sent_lens.iter().filter(|l| b.contains(**l)).count();
                let actual = n as f32 / total;
                BucketScore {
                    bucket: b.clone(),
                    actual_share: round2(actual),
                    passes: b.passes(actual),
                }
            })
            .collect();

        let ttr_passes = profile.type_token_ratio >= self.type_token_ratio_min;
        // MATTR is the length-stable check; only enforce when both the
        // target asks for it (mattr_min > 0) AND the profile measured
        // it (mattr_50 > 0 — i.e. the prose was long enough to fit a
        // 50-token window). Short scenes get a free pass.
        let mattr_passes =
            self.mattr_min == 0.0 || profile.mattr_50 == 0.0 || profile.mattr_50 >= self.mattr_min;
        let em_dash_passes = profile.em_dash_per_1000 <= self.em_dash_per_1000_max;
        let max_chain = max_consecutive_same_opening(&sentences, 3);
        let max_chain_2tok = max_consecutive_same_opening(&sentences, 2);
        let max_chain_4tok = max_consecutive_same_opening(&sentences, 4);
        let repeated_opening_passes = max_chain <= self.repeated_opening_max
            && (self.repeated_opening_max_2tok == 0
                || max_chain_2tok <= self.repeated_opening_max_2tok)
            && (self.repeated_opening_max_4tok == 0
                || max_chain_4tok <= self.repeated_opening_max_4tok);
        let long_sentence_passes =
            passes_long_sentence_floor(text, self.min_long_sentences_per_paragraph);
        let (band_coverage_passes, worst_paragraph_coverage) = passes_band_coverage_floor(
            text,
            &self.sentence_length_buckets,
            self.min_band_coverage_per_paragraph,
        );

        let mut failed = Vec::new();
        for (i, b) in bucket_scores.iter().enumerate() {
            if !b.passes {
                failed.push(format!(
                    "sentence_length_bucket[{i}] ({}-{}w) actual {:.2} not in [{:.2},{:.2}]",
                    b.bucket.min_words,
                    if b.bucket.max_words == u32::MAX {
                        999
                    } else {
                        b.bucket.max_words
                    },
                    b.actual_share,
                    (b.bucket.target_share - b.bucket.tolerance).max(0.0),
                    (b.bucket.target_share + b.bucket.tolerance).min(1.0),
                ));
            }
        }
        if !ttr_passes {
            failed.push(format!(
                "type_token_ratio {:.3} < min {:.3}",
                profile.type_token_ratio, self.type_token_ratio_min,
            ));
        }
        if !mattr_passes {
            failed.push(format!(
                "mattr_50 {:.3} < min {:.3}",
                profile.mattr_50, self.mattr_min,
            ));
        }
        if !em_dash_passes {
            failed.push(format!(
                "em_dash_per_1000 {:.2} > max {:.2}",
                profile.em_dash_per_1000, self.em_dash_per_1000_max,
            ));
        }
        if !repeated_opening_passes {
            failed.push(format!(
                "repeated_opening 3-tok chain {} > limit {}, 2-tok chain {} > limit {}, 4-tok chain {} > limit {}",
                max_chain, self.repeated_opening_max,
                max_chain_2tok, self.repeated_opening_max_2tok,
                max_chain_4tok, self.repeated_opening_max_4tok,
            ));
        }
        if !long_sentence_passes {
            failed.push(format!(
                "min_long_sentences_per_paragraph {} not met in every 3+ sentence paragraph",
                self.min_long_sentences_per_paragraph,
            ));
        }
        if !band_coverage_passes {
            failed.push(format!(
                "min_band_coverage_per_paragraph {} not met — at least one 4+ sentence paragraph touches only {} band(s)",
                self.min_band_coverage_per_paragraph,
                worst_paragraph_coverage,
            ));
        }
        if self.require_dialogue && profile.dialogue_ratio == 0.0 {
            failed.push("require_dialogue but dialogue_ratio = 0".to_owned());
        }

        VoiceScore {
            bucket_scores,
            ttr_passes,
            mattr_passes,
            em_dash_passes,
            repeated_opening_passes,
            long_sentence_passes,
            band_coverage_passes,
            worst_paragraph_band_coverage: worst_paragraph_coverage,
            max_consecutive_repeated_openings: max_chain,
            overall_passes: failed.is_empty(),
            failed_dimensions: failed,
        }
    }
}

/// One bucket's measurement against its target band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BucketScore {
    pub bucket: SentenceLengthBucket,
    pub actual_share: f32,
    pub passes: bool,
}

/// Overall draft-vs-target score. `overall_passes` is true iff every
/// dimension is in band; otherwise `failed_dimensions` lists each miss
/// in human-readable form so the polish prompt can target them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceScore {
    pub bucket_scores: Vec<BucketScore>,
    pub ttr_passes: bool,
    /// True iff `mattr_min` was 0.0 (unenforced), the prose was too
    /// short for MATTR to be defined, or the measured MATTR-50 is at
    /// or above the target floor.
    #[serde(default = "default_true")]
    pub mattr_passes: bool,
    pub em_dash_passes: bool,
    pub repeated_opening_passes: bool,
    pub long_sentence_passes: bool,
    /// Run #12 anti-bimodal-collapse check — true iff every 4+ sentence
    /// paragraph touches at least `min_band_coverage_per_paragraph`
    /// distinct length-buckets across its sentences.
    #[serde(default = "default_true")]
    pub band_coverage_passes: bool,
    /// Smallest number of length-buckets touched by any 4+ sentence
    /// paragraph. Useful diagnostic for the planner — directly says
    /// "this paragraph collapsed into N bands."
    #[serde(default)]
    pub worst_paragraph_band_coverage: u8,
    pub max_consecutive_repeated_openings: u32,
    pub overall_passes: bool,
    pub failed_dimensions: Vec<String>,
}

fn default_true() -> bool {
    true
}

// ── helpers (private to this module) ──────────────────────────────────────

fn round2(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

/// First `n` lowercased tokens of a sentence joined with spaces.
/// Returns `None` if the sentence has fewer than `n` tokens (so the
/// chain detector won't false-positive on terse sentences).
fn opening_clause(sentence: &str, n: usize) -> Option<String> {
    let toks = word_tokens(sentence);
    if toks.len() < n {
        return None;
    }
    Some(toks.iter().take(n).cloned().collect::<Vec<_>>().join(" "))
}

/// Largest number of consecutive sentences that share the same `n`-token
/// opening. Returns 1 if no chain longer than 1 exists, 0 if the input is
/// empty.
fn max_consecutive_same_opening(sentences: &[String], n: usize) -> u32 {
    if sentences.is_empty() {
        return 0;
    }
    let mut max_chain: u32 = 1;
    let mut cur_chain: u32 = 1;
    let mut prev: Option<String> = opening_clause(&sentences[0], n);
    for s in sentences.iter().skip(1) {
        let cur = opening_clause(s, n);
        if cur.is_some() && cur == prev {
            cur_chain += 1;
            if cur_chain > max_chain {
                max_chain = cur_chain;
            }
        } else {
            cur_chain = 1;
        }
        prev = cur;
    }
    max_chain
}

/// True iff every paragraph with ≥ 3 sentences contains at least
/// `min` sentences of ≥ 18 words. Paragraphs shorter than 3
/// sentences are exempt — the rule is about long-form prose.
///
/// FEATURE_HARDENING_PLAN.md §1.5 dropped the floor from 4 → 3 so
/// 3-sentence uniformly-short paragraphs (a common Run-#11-style
/// failure mode) get caught. The deliberate-minimalism exception
/// (planned in §1.7) will let writers override this per-paragraph.
fn passes_long_sentence_floor(text: &str, min: u32) -> bool {
    if min == 0 {
        return true;
    }
    for para in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        let sents = split_sentences(para);
        if sents.len() < 3 {
            continue;
        }
        let n_long = sents.iter().filter(|s| word_tokens(s).len() >= 18).count() as u32;
        if n_long < min {
            return false;
        }
    }
    true
}

/// Run #12 fix: every paragraph of 4+ sentences must touch at least
/// `min` distinct sentence-length buckets. Returns
/// `(passes, worst_paragraph_coverage)`. When `min` is 0 the rule is
/// unenforced and `worst_paragraph_coverage` reports the smallest
/// coverage observed (purely diagnostic — does not fail the run).
///
/// The bimodal failure Run #12 surfaced (84% short / 0% medium / 16%
/// long) satisfies the per-bucket distribution at the SCENE level but
/// produces individual paragraphs with all-short sentences — exactly
/// what this check rejects.
fn passes_band_coverage_floor(text: &str, buckets: &[SentenceLengthBucket], min: u8) -> (bool, u8) {
    let mut worst: u8 = u8::MAX;
    let mut any_4_plus_paragraph = false;
    for para in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        let sents = split_sentences(para);
        if sents.len() < 4 {
            continue;
        }
        any_4_plus_paragraph = true;
        let mut touched = std::collections::HashSet::new();
        for s in &sents {
            let len = word_tokens(s).len() as u32;
            for (i, b) in buckets.iter().enumerate() {
                if b.contains(len) {
                    touched.insert(i);
                    break;
                }
            }
        }
        let coverage = touched.len() as u8;
        if coverage < worst {
            worst = coverage;
        }
    }
    if !any_4_plus_paragraph {
        // No paragraph long enough to evaluate — rule trivially passes.
        return (true, 0);
    }
    if min == 0 {
        return (true, worst);
    }
    (worst >= min, worst)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::fingerprint;

    #[test]
    fn bucket_contains_inclusive() {
        let b = SentenceLengthBucket {
            min_words: 0,
            max_words: 8,
            target_share: 0.6,
            tolerance: 0.1,
        };
        assert!(b.contains(0));
        assert!(b.contains(8));
        assert!(!b.contains(9));
    }

    #[test]
    fn bucket_passes_within_tolerance() {
        let b = SentenceLengthBucket {
            min_words: 0,
            max_words: 8,
            target_share: 0.60,
            tolerance: 0.10,
        };
        assert!(b.passes(0.55));
        assert!(b.passes(0.65));
        assert!(b.passes(0.50));
        assert!(b.passes(0.70));
        assert!(!b.passes(0.49));
        assert!(!b.passes(0.71));
    }

    #[test]
    fn directive_block_is_prescriptive_not_descriptive() {
        let t = VoiceTarget::literary_default();
        let block = t.directive_block();
        assert!(block.contains("MUST satisfy"));
        assert!(block.contains("must not exceed"));
        assert!(block.contains("contract"));
        // Must NOT use the descriptive phrasing the Run #11 model collapsed on.
        assert!(!block.contains("alternate between"));
    }

    #[test]
    fn run11_collapsed_prose_fails_literary_target() {
        // Reconstructed from the Run #11 quality-review.md. Median 5,
        // IQR 1, anaphora chain of 8.
        let collapsed = "The hand held the letter. The hand held the date. \
                         The hand held the name. The hand held the truth. \
                         The hand held the lie. The hand held the silence. \
                         The hand held the grief. The hand held the inheritance.";
        let p = fingerprint(collapsed);
        let t = VoiceTarget::literary_default();
        let s = t.score(&p, collapsed);
        assert!(
            !s.overall_passes,
            "Run #11 collapsed prose must fail target"
        );
        assert!(
            !s.repeated_opening_passes,
            "anaphora chain of 8 must fail repeated-opening cap of 3"
        );
        assert!(s.max_consecutive_repeated_openings >= 8);
    }

    #[test]
    fn rhythmic_prose_passes_literary_target() {
        // Mixed-length, single-paragraph excerpt that satisfies the
        // literary defaults: short / mid / long alternation, varied
        // openings, no anaphora, decent vocab richness.
        let prose = "She did not turn the light on. The fridge clicked once and \
                     was quiet. From the porch came the slow scrape of a chair \
                     drawn back across old wood, a sound she had not heard in \
                     three years and could still place. Arthur. The name arrived \
                     before she had decided to think it.";
        let p = fingerprint(prose);
        let t = VoiceTarget::literary_default();
        let s = t.score(&p, prose);
        // Anaphora and TTR should pass on this prose; bucket bands may
        // not all pass on a 5-sentence excerpt — assert the structural
        // checks rather than the small-sample distributional ones.
        assert!(
            s.repeated_opening_passes,
            "no repeated openings in this prose"
        );
        assert!(
            s.ttr_passes,
            "TTR floor 0.42 should pass on rich prose, got {:.3}",
            p.type_token_ratio
        );
    }

    #[test]
    fn opening_clause_skips_short_sentences() {
        let s = "Hi.";
        assert_eq!(opening_clause(s, 3), None);
        let s = "The hand held the letter.";
        assert_eq!(opening_clause(s, 3), Some("the hand held".to_owned()));
    }

    #[test]
    fn max_consecutive_chain_counts_correctly() {
        let sents: Vec<String> = vec![
            "The hand held the letter.".into(),
            "The hand held the date.".into(),
            "The hand held the name.".into(),
            "She turned away.".into(),
            "The hand held the lie.".into(),
        ];
        assert_eq!(max_consecutive_same_opening(&sents, 3), 3);
    }

    #[test]
    fn long_sentence_floor_exempts_short_paragraphs() {
        // Under FEATURE_HARDENING_PLAN.md §1.5 the floor is 3 sentences
        // (was 4); a 2-sentence paragraph is still exempt.
        let text = "Short. Short.";
        assert!(passes_long_sentence_floor(text, 1));
    }

    #[test]
    fn long_sentence_floor_enforces_in_3_sentence_paragraphs() {
        // The §1.5 fix: 3-sentence uniformly-short paragraphs now fail
        // (under the v1 floor of 4 they were exempt — a real Run #11
        // failure mode the v1 detector missed).
        let text = "Short. Also short. Still short.";
        assert!(!passes_long_sentence_floor(text, 1));
    }

    #[test]
    fn long_sentence_floor_enforces_in_long_paragraphs() {
        let text = "Short. Also short. Still short. And short again.";
        assert!(!passes_long_sentence_floor(text, 1));
    }

    // ── Multi-token anaphora chains (FEATURE_HARDENING_PLAN.md §1.4) ──────

    #[test]
    fn two_token_chain_caught_when_3_token_chain_passes() {
        // 5 sentences with identical 2-token opening "she fell" but
        // 5 different 3rd tokens. The 3-tok chain is length 1 (each
        // 3-tok opening is unique) so the v1 check passes; the new
        // 2-tok check at cap=3 catches the chain of 5.
        let prose = "She fell apart. She fell again. She fell silent. \
                     She fell hard. She fell back.";
        let mut t = VoiceTarget::literary_default();
        t.repeated_opening_max_2tok = 3;
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert!(
            !s.repeated_opening_passes,
            "2-tok chain of 5 should fail cap of 3"
        );
    }

    #[test]
    fn four_token_chain_caught_when_3_token_passes_within_cap() {
        // "The hand on the door. The hand on the lock. The hand on the
        // chain." — 3 sentences with identical 4-token opening "the
        // hand on the". The 3-tok chain length is also 3 which fits
        // literary cap 3. The 4-tok cap (literary default = 2) catches it.
        let prose = "The hand on the door. The hand on the lock. The hand on the chain.";
        let t = VoiceTarget::literary_default();
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert!(
            !s.repeated_opening_passes,
            "4-tok chain of 3 should fail cap of 2"
        );
    }

    #[test]
    fn unenforced_companion_caps_admit_anything() {
        // When repeated_opening_max_2tok = 0, the 2-tok check is bypassed.
        let prose =
            "She fell apart. She rose anew. She fell anew. She rose hopeful. She fell silent.";
        let mut t = VoiceTarget::literary_default();
        t.repeated_opening_max_2tok = 0;
        t.repeated_opening_max_4tok = 0;
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert!(s.repeated_opening_passes);
    }

    #[test]
    fn punchy_target_allows_more_repetition_than_literary() {
        // Same anaphora chain — punchy allows up to 5 consecutive,
        // literary allows only 3. Sentences need ≥ 3 word-tokens for
        // `opening_clause` to register them as a chain (terse 1-2 word
        // sentences are exempt from the cap by design).
        // Identical 3-token opening "she ran toward" repeated 4×
        // — exceeds the literary cap of 3, fits the punchy cap of 5.
        let chain = "She ran toward the door. She ran toward the gate. \
                     She ran toward the road. She ran toward the lights.";
        let sents = split_sentences(chain);
        let chain_len = max_consecutive_same_opening(&sents, 3);
        let lit = VoiceTarget::literary_default();
        let pun = VoiceTarget::punchy_action_default();
        assert!(
            chain_len > lit.repeated_opening_max,
            "chain_len={chain_len} must exceed literary cap {}",
            lit.repeated_opening_max
        );
        assert!(
            chain_len <= pun.repeated_opening_max,
            "chain_len={chain_len} must be within punchy cap {}",
            pun.repeated_opening_max
        );
    }

    #[test]
    fn defaults_serialize_round_trip() {
        // VoiceTarget is serde-serializable; bibles persist these.
        for t in [
            VoiceTarget::literary_default(),
            VoiceTarget::commercial_default(),
            VoiceTarget::punchy_action_default(),
        ] {
            let json = serde_json::to_string(&t).unwrap();
            let back: VoiceTarget = serde_json::from_str(&json).unwrap();
            assert_eq!(back, t);
        }
    }

    // ── Run #12 calibration: band-coverage + TTR deprecation + dialogue ──

    #[test]
    fn literary_default_deprecates_ttr() {
        // Run #12 §4 conclusion: TTR is length-biased and disagrees with
        // MATTR. The default leaves TTR unenforced (set to 0.0).
        let t = VoiceTarget::literary_default();
        assert_eq!(t.type_token_ratio_min, 0.0, "TTR floor must be deprecated");
        assert!(t.mattr_min > 0.0, "MATTR floor must be enforced instead");
    }

    #[test]
    fn literary_default_now_requires_dialogue() {
        // Run #12 §6: zero-dialogue scenes should be flagged so the
        // drafter renders interior monologue as dialogue.
        let t = VoiceTarget::literary_default();
        assert!(t.require_dialogue);
    }

    #[test]
    fn literary_default_demands_all_three_bands_per_paragraph() {
        // Run #13 retighten: literary now requires all 3 bands per
        // paragraph (was 2 in Run #12, which still admitted bimodal
        // short+long failure).
        let t = VoiceTarget::literary_default();
        assert_eq!(t.min_band_coverage_per_paragraph, 3);
        assert_eq!(t.sentence_length_buckets.len(), 3);
    }

    #[test]
    fn directive_block_says_interleave() {
        // The directive must explicitly forbid bursts-then-bursts.
        let t = VoiceTarget::literary_default();
        let block = t.directive_block();
        // Run #13: literary now uses the strongest "every band" form.
        assert!(block.contains("MANDATORY INTERLEAVING"));
        assert!(block.contains("at least one short"));
        assert!(block.contains("at least one medium"));
        assert!(block.contains("at least one long"));
        assert!(block.contains("bimodal-collapse"));
        assert!(block.contains("interior monologue"));
    }

    #[test]
    fn band_coverage_catches_run11_all_short_paragraph() {
        // The Run #11 failure mode: a paragraph of 30+ all-short
        // sentences ("the hand held the letter. the hand held the
        // date. ...") touches only ONE band. min_band_coverage=2
        // catches it.
        let prose = "She left. He stayed. The room went quiet. The clocks ticked. \
                     Her hand shook. The light was off. The drawer was small. \
                     The key was cold.";
        let t = VoiceTarget::literary_default();
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert!(
            !s.band_coverage_passes,
            "all-short paragraph must fail band_coverage cap of 2; got worst={}",
            s.worst_paragraph_band_coverage
        );
        assert_eq!(
            s.worst_paragraph_band_coverage, 1,
            "8 short sentences = 1 band touched"
        );
    }

    #[test]
    fn band_coverage_now_fails_short_plus_long_without_medium() {
        // Run #13 retighten: with literary's min_band_coverage=3, the
        // Run #12 paragraph 1 failure mode (short + long, no medium)
        // FAILS the band-coverage rule directly, instead of only
        // surfacing as a scene-level bucket distribution failure.
        // This is the "30-min spec tightening" the Run #12 quality
        // review §8 prescribed.
        let prose = "She walked into the room with a deliberate slowness that betrayed the depth of her dread, planting each foot as if testing thin ice. The wood was dark. The light was off. The clocks ticked. Her hand shook. The drawer was small. The key was cold. She did not move. The silence held.";
        let t = VoiceTarget::literary_default();
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert!(
            !s.band_coverage_passes,
            "short+long without medium must FAIL at min=3; got worst={}",
            s.worst_paragraph_band_coverage,
        );
        assert_eq!(
            s.worst_paragraph_band_coverage, 2,
            "the paragraph touches 2 bands; with min=3 that's a fail",
        );
        // Bucket-level check should ALSO fire.
        let bimodal_bucket_failure = s
            .failed_dimensions
            .iter()
            .any(|d| d.starts_with("sentence_length_bucket[1]"));
        assert!(
            bimodal_bucket_failure,
            "missing-medium-band must ALSO surface as a bucket-level failure",
        );
    }

    #[test]
    fn rhythmically_varied_paragraph_passes_band_coverage() {
        // 4-sentence paragraph that genuinely touches all three
        // bands — a short fragment (3 words), a medium-length
        // sentence (~12 words), another short one (~6 words), and a
        // long sentence (~22 words). At literary's min=3 this is the
        // only configuration that passes.
        let prose = "She paused. The chair scraped against the floor with a slow deliberate sound. Arthur, she thought. The name arrived unbidden out of some quiet drawer of memory she had kept locked for years and now suddenly remembered.";
        let t = VoiceTarget::literary_default();
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert_eq!(
            s.worst_paragraph_band_coverage, 3,
            "all-three-band paragraph should hit coverage 3; got {}",
            s.worst_paragraph_band_coverage,
        );
        assert!(s.band_coverage_passes);
    }

    #[test]
    fn band_coverage_floor_exempts_short_paragraphs() {
        // 3 sentences → exempt from band-coverage rule (rule applies at 4+).
        let prose = "She left. He stayed. The room went quiet.";
        let t = VoiceTarget::literary_default();
        let p = crate::fingerprint(prose);
        let s = t.score(&p, prose);
        assert!(s.band_coverage_passes, "short paragraph must be exempt");
    }

    #[test]
    fn short_bucket_tolerance_now_010() {
        let t = VoiceTarget::literary_default();
        let short = &t.sentence_length_buckets[0];
        assert_eq!(short.tolerance, 0.10, "Run #12 fix: tightened 0.15 → 0.10");
    }
}
