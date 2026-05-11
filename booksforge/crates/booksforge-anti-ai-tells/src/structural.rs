//! Structural anti-AI-tells — patterns that span multiple sentences or
//! paragraphs, where a single regex cannot reach.
//!
//! The Run #11 quality review (book-output/integrated-runs/20260509-221828)
//! identified four collapse modes that the lexicon-only detector in
//! `lib.rs` could not catch because they only become visible at the
//! sentence-distribution scale:
//!
//!   1. **Anaphora chain** — 4+ consecutive sentences sharing the same
//!      3-token opening clause ("The hand held the letter. The hand
//!      held the date. The hand held the name…"). Run #11 had chains
//!      of 8 and 11.
//!   2. **Substitution game** — within a paragraph, the same `[X] was
//!      [Y]` template applied recursively to a slot-fill of varied
//!      nouns ("The name was a stone. The name was a scar. The name
//!      was a wound…").
//!   3. **Low sentence-length variance** — a paragraph of 4+ sentences
//!      whose interquartile range is < 3 words. Run #11 hit IQR = 1
//!      (median 5, p25 4, p75 5) — mechanical uniformity.
//!   4. **No concrete sensory noun** — a paragraph composed entirely of
//!      abstract nouns ("name", "silence", "grief", "inheritance") with
//!      zero touch / smell / sound / weight anchors. The single
//!      strongest indicator of "this prose is shaped by a model that
//!      doesn't know what hands feel like."
//!
//! Each detector returns a [`crate::TellHit`] with a `structural:*`
//! category so downstream consumers can distinguish per-token tells
//! (the lexicon dictionary in `lib.rs`) from structural tells
//! (this file). The `structural_tells` aggregator runs all four.

use crate::TellHit;
use booksforge_voice::{split_sentences, word_tokens};

/// One sensory concrete noun (singular base form). Curated to span
/// seven domains so the detector doesn't bias toward urban-domestic
/// prose — fiction set in a forest, on a coast, in a kitchen, or in
/// a workshop should each find its own anchors here.
///
/// Stored as singular base forms only; the lookup path lemmatizes
/// the input via [`singularize`] so `"hands"` and `"stones"` both
/// resolve to entries here. FEATURE_HARDENING_PLAN.md §3.2 + §3.3.
const CONCRETE_NOUNS: &[&str] = &[
    // Body
    "skin",
    "hand",
    "knuckle",
    "wrist",
    "throat",
    "shoulder",
    "rib",
    "spine",
    "knee",
    "ankle",
    "tongue",
    "lip",
    "tooth",
    "jaw",
    "hair",
    "brow",
    "palm",
    "finger",
    "thumb",
    "nail",
    "eye",
    "ear",
    "nose",
    "cheek",
    "chin",
    "neck",
    "breath",
    "sweat",
    "blood",
    "scar",
    "bruise",
    "bone",
    "pulse",
    "heel",
    "sole",
    "hip",
    "elbow",
    "temple",
    "lash",
    "freckle",
    // Senses (smell, taste, sight)
    "smell",
    "scent",
    "smoke",
    "salt",
    "pepper",
    "metal",
    "iron",
    "rust",
    "leather",
    "soap",
    "vinegar",
    "lemon",
    "wood",
    "earth",
    "rain",
    "mud",
    "mist",
    "frost",
    "hail",
    "sleet",
    "fog",
    "dew",
    "dust",
    "ash",
    "mold",
    "tar",
    "wax",
    "musk",
    // Sound + touch
    "sound",
    "click",
    "creak",
    "rasp",
    "scrape",
    "thud",
    "tap",
    "hum",
    "whisper",
    "rustle",
    "knock",
    "clatter",
    "bang",
    "snap",
    "weight",
    "warmth",
    "heat",
    "cold",
    "chill",
    "draft",
    "breeze",
    "wind",
    "steam",
    "hiss",
    "groan",
    // Materials with tactile presence
    "brass",
    "copper",
    "tin",
    "cloth",
    "linen",
    "wool",
    "denim",
    "paper",
    "twine",
    "rope",
    "string",
    "wire",
    "plastic",
    "glass",
    "ceramic",
    "porcelain",
    "marble",
    "granite",
    "slate",
    "oak",
    "pine",
    "birch",
    "cedar",
    "bark",
    "sap",
    "resin",
    "felt",
    "silk",
    "burlap",
    "canvas",
    // Kitchen objects
    "knife",
    "spoon",
    "fork",
    "plate",
    "bowl",
    "cup",
    "mug",
    "kettle",
    "pan",
    "pot",
    "oven",
    "stove",
    "sink",
    "fridge",
    "drawer",
    "shelf",
    "jar",
    "lid",
    "cork",
    "bottle",
    // Room objects
    "wall",
    "window",
    "door",
    "floor",
    "ceiling",
    "stair",
    "ledge",
    "threshold",
    "mantel",
    "hearth",
    "fireplace",
    "blanket",
    "sheet",
    "pillow",
    "mattress",
    "table",
    "chair",
    "sofa",
    "lamp",
    "switch",
    "bulb",
    "mirror",
    "rug",
    "curtain",
    "key",
    "lock",
    "hinge",
    // Work / garage / craft
    "file",
    "ledger",
    "stapler",
    "keyboard",
    "screen",
    "paperclip",
    "oil",
    "grease",
    "wrench",
    "bolt",
    "nut",
    "screw",
    "hammer",
    "drill",
    "saw",
    "sandpaper",
    "paint",
    "brush",
    "pencil",
    "pen",
    "notebook",
    // Outdoor / nature
    "stone",
    "gravel",
    "pebble",
    "twig",
    "fern",
    "leaf",
    "branch",
    "root",
    "soil",
    "sand",
    "beach",
    "river",
    "creek",
    "pond",
    "puddle",
    "hill",
    "ridge",
    "cliff",
    "valley",
    "garden",
    "weed",
    "vine",
    "thorn",
    "mulch",
    "snow",
    "ice",
    "swamp",
    "stream",
    "lake",
    "ocean",
    "shore",
    // Food
    "bread",
    "water",
    "tea",
    "coffee",
    "wine",
    "milk",
    "butter",
    "cheese",
    "egg",
    "toast",
    "soup",
    "stew",
    "rice",
    "bean",
    "broth",
    "garlic",
    "onion",
    "ginger",
    "honey",
    "sugar",
    "flour",
    "oatmeal",
    "jam",
    "syrup",
    // Urban / built environment
    "curb",
    "gutter",
    "asphalt",
    "brick",
    "neon",
    "awning",
    "fence",
    "gate",
    "latch",
    "mailbox",
    "pavement",
    "crosswalk",
    "alley",
    "stoop",
    "porch",
];

/// Result of all six structural detectors merged + sorted by `start`.
/// Returns an empty vec if the text has fewer than 4 sentences total
/// (the detectors operate on multi-sentence patterns and don't apply
/// to flash-length excerpts).
pub fn structural_tells(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    hits.extend(detect_anaphora_chain(text));
    hits.extend(detect_substitution_game(text));
    hits.extend(detect_pronoun_substitution_game(text));
    hits.extend(detect_low_variance_paragraphs(text));
    hits.extend(detect_low_burstiness_paragraphs(text));
    hits.extend(detect_missing_concrete_noun(text));
    hits.sort_by_key(|h| h.start);
    hits
}

// ── 1. Anaphora chain ─────────────────────────────────────────────────────

/// Detect runs of 4+ consecutive sentences that share the same first
/// 3 word-tokens. Returns one [`TellHit`] per chain spanning the full
/// chain's character range.
pub fn detect_anaphora_chain(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    let sentences = split_sentences(text);
    if sentences.len() < 4 {
        return hits;
    }

    // Walk consecutive groups by 3-token opening. Resolve each
    // sentence's character range in the source text by left-to-right
    // scanning so we don't have to thread positions through the
    // splitter.
    let positions = locate_in_source(text, &sentences);
    let openings: Vec<Option<String>> = sentences.iter().map(|s| opening3(s)).collect();

    let mut i = 0;
    while i < openings.len() {
        let cur = &openings[i];
        if cur.is_none() {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < openings.len() && openings[j] == *cur {
            j += 1;
        }
        let chain_len = j - i;
        if chain_len >= 4 {
            let start = positions.get(i).map_or(0, |(s, _)| *s);
            let end = positions.get(j - 1).map_or(text.len(), |(_, e)| *e);
            let matched = text.get(start..end).unwrap_or("").to_owned();
            hits.push(TellHit {
                start,
                end,
                matched,
                severity: 3,
                category: "structural:anaphora_chain".to_owned(),
                why: format!(
                    "{chain_len} consecutive sentences share the opening {:?} — break the pattern with one mid-length, sensory sentence",
                    cur.as_deref().unwrap_or(""),
                ),
                suggested_replacement: None,
            });
        }
        i = j.max(i + 1);
    }
    hits
}

// ── 2. Substitution game ───────────────────────────────────────────────────

/// Detect runs where 5+ sentences in the same paragraph fit the
/// `[Det] [Noun] (was|is|held|stood|carried) [Object]` template, with
/// the noun varying across the run. This is the Run #11
/// "the name was a stone / a scar / a wound" pattern.
pub fn detect_substitution_game(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    let mut paragraph_start = 0usize;
    for para in text.split("\n\n") {
        let trimmed = para.trim_start();
        // recover absolute start offset of the trimmed paragraph
        let leading = para.len() - trimmed.len();
        let para_abs_start = paragraph_start + leading;

        if !trimmed.is_empty() {
            let sentences = split_sentences(trimmed);
            // Count sentences that fit `[Det] [Noun] (was|is|held|stood|carried) ...`
            let mut hits_in_para = 0usize;
            let mut nouns_seen: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for s in &sentences {
                if let Some(noun) = match_substitution_template(s) {
                    hits_in_para += 1;
                    nouns_seen.insert(noun);
                }
            }
            // Trigger when the template recurs ≥ 5x with at least 3
            // distinct nouns — i.e. the model is genuinely playing the
            // slot-fill game, not legitimately repeating one image.
            if hits_in_para >= 5 && nouns_seen.len() >= 3 {
                let end = para_abs_start + trimmed.len();
                let matched = text.get(para_abs_start..end).unwrap_or("").to_owned();
                hits.push(TellHit {
                    start: para_abs_start,
                    end,
                    matched: truncate_for_display(&matched, 200),
                    severity: 3,
                    category: "structural:substitution_game".to_owned(),
                    why: format!(
                        "{hits_in_para} sentences in this paragraph share the [Det Noun was/held/stood Y] template across {} distinct nouns — break the recursion",
                        nouns_seen.len(),
                    ),
                    suggested_replacement: None,
                });
            }
        }
        // Advance: paragraph + the "\n\n" separator.
        paragraph_start += para.len() + 2;
    }
    hits
}

// ── 3. Low sentence-length variance per paragraph ─────────────────────────

/// Flag any paragraph of 4+ sentences whose sentence-length IQR is
/// less than 3 words. Run #11 hit IQR = 1 in every paragraph.
pub fn detect_low_variance_paragraphs(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    let mut cursor = 0usize;
    for para in text.split("\n\n") {
        let trimmed = para.trim_start();
        let leading = para.len() - trimmed.len();
        let para_abs_start = cursor + leading;

        if !trimmed.is_empty() {
            let sentences = split_sentences(trimmed);
            if sentences.len() >= 4 {
                let mut lens: Vec<usize> = sentences.iter().map(|s| word_tokens(s).len()).collect();
                lens.sort_unstable();
                let p25 = quartile(&lens, 0.25);
                let p75 = quartile(&lens, 0.75);
                let iqr = p75 - p25;
                if iqr < 3.0 {
                    let end = para_abs_start + trimmed.len();
                    hits.push(TellHit {
                        start: para_abs_start,
                        end,
                        matched: truncate_for_display(
                            text.get(para_abs_start..end).unwrap_or(""),
                            200,
                        ),
                        severity: 2,
                        category: "structural:low_variance".to_owned(),
                        why: format!(
                            "paragraph of {} sentences has sentence-length IQR {:.1} (target ≥ 3) — intersperse one or two sentences of 18+ words",
                            sentences.len(),
                            iqr,
                        ),
                        suggested_replacement: None,
                    });
                }
            }
        }
        cursor += para.len() + 2;
    }
    hits
}

// ── 4. No concrete sensory noun in paragraph ──────────────────────────────

/// Flag any paragraph of 3+ sentences that contains zero concrete
/// sensory nouns (see [`CONCRETE_NOUNS`]). The strongest single
/// indicator of model-shaped vs body-shaped prose.
///
/// FEATURE_HARDENING_PLAN.md §3.2 — lemmatizes plural inputs via
/// [`singularize`] so `"hands"`, `"stones"`, `"creaks"` all resolve
/// to their base form before lookup. The list itself stores
/// singulars only.
pub fn detect_missing_concrete_noun(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    let lookup: std::collections::HashSet<&str> = CONCRETE_NOUNS.iter().copied().collect();

    let mut cursor = 0usize;
    for para in text.split("\n\n") {
        let trimmed = para.trim_start();
        let leading = para.len() - trimmed.len();
        let para_abs_start = cursor + leading;

        if !trimmed.is_empty() {
            let sentences = split_sentences(trimmed);
            if sentences.len() >= 3 {
                let tokens = word_tokens(trimmed);
                let any = tokens.iter().any(|t| {
                    lookup.contains(t.as_str()) || lookup.contains(singularize(t).as_str())
                });
                if !any {
                    let end = para_abs_start + trimmed.len();
                    hits.push(TellHit {
                        start: para_abs_start,
                        end,
                        matched: truncate_for_display(
                            text.get(para_abs_start..end).unwrap_or(""),
                            200,
                        ),
                        severity: 2,
                        category: "structural:no_concrete_noun".to_owned(),
                        why:
                            "paragraph contains zero concrete sensory nouns (skin/hand/smell/sound/weight/etc.) — anchor in physical experience"
                                .to_owned(),
                        suggested_replacement: None,
                    });
                }
            }
        }
        cursor += para.len() + 2;
    }
    hits
}

// ── Burstiness detector (FEATURE_HARDENING_PLAN.md §3.1) ──────────────────

/// Flag any paragraph of 5+ sentences whose burstiness score is below
/// 1.0. *Burstiness* — variance(sentence_lengths) / mean(sentence_lengths)
/// — is the AI-detection industry's converged-upon paragraph-level
/// signal: human prose has a Fano factor well above 1, AI prose tends
/// to cluster near or below 1. Complements the IQR-based
/// [`detect_low_variance_paragraphs`] (a coarser cutoff using the
/// inter-quartile range); this one operates on the full distribution
/// shape and catches paragraphs that are uniformly LONG as well as
/// uniformly SHORT.
pub fn detect_low_burstiness_paragraphs(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    let mut cursor = 0usize;
    for para in text.split("\n\n") {
        let trimmed = para.trim_start();
        let leading = para.len() - trimmed.len();
        let para_abs_start = cursor + leading;

        if !trimmed.is_empty() {
            let sentences = split_sentences(trimmed);
            if sentences.len() >= 5 {
                let lens: Vec<f32> = sentences
                    .iter()
                    .map(|s| word_tokens(s).len() as f32)
                    .collect();
                let n = lens.len() as f32;
                let mean = lens.iter().sum::<f32>() / n;
                let variance = lens.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / n;
                let burstiness = variance / mean.max(1.0);
                if burstiness < 1.0 {
                    let end = para_abs_start + trimmed.len();
                    hits.push(TellHit {
                        start: para_abs_start,
                        end,
                        matched: truncate_for_display(
                            text.get(para_abs_start..end).unwrap_or(""),
                            200,
                        ),
                        severity: 2,
                        category: "structural:low_burstiness".to_owned(),
                        why: format!(
                            "paragraph of {} sentences has burstiness {:.2} (target ≥ 1.0) — Fano factor at this level is the canonical AI-detection signal",
                            sentences.len(),
                            burstiness,
                        ),
                        suggested_replacement: None,
                    });
                }
            }
        }
        cursor += para.len() + 2;
    }
    hits
}

// ── Pronoun substitution game (FEATURE_HARDENING_PLAN.md §3.4) ────────────

/// Detect runs where 5+ sentences in the same paragraph fit
/// `[Pronoun] [Verb] [State]` with the same `[Pronoun] [Verb]` repeated
/// across varying states. Mirrors [`detect_substitution_game`] but for
/// pronoun-driven slot-fill rhythms like:
///
/// > She was hungry. She was tired. She was alone. She was cold.
/// > She was finished.
///
/// Same family of failure as the determiner case but trips a different
/// template — the v1 detector missed all of these.
pub fn detect_pronoun_substitution_game(text: &str) -> Vec<TellHit> {
    let mut hits: Vec<TellHit> = Vec::new();
    let mut paragraph_start = 0usize;
    for para in text.split("\n\n") {
        let trimmed = para.trim_start();
        let leading = para.len() - trimmed.len();
        let para_abs_start = paragraph_start + leading;

        if !trimmed.is_empty() {
            let sentences = split_sentences(trimmed);
            // Group by `(pronoun, verb)` pair; trigger when a pair appears
            // 5+ times with 3+ distinct trailing states.
            let mut by_pair: std::collections::HashMap<
                (String, String),
                std::collections::HashSet<String>,
            > = std::collections::HashMap::new();
            for s in &sentences {
                if let Some((pronoun, verb, state)) = match_pronoun_template(s) {
                    by_pair.entry((pronoun, verb)).or_default().insert(state);
                }
            }
            for ((pronoun, verb), states) in &by_pair {
                if states.len() >= 5 {
                    let end = para_abs_start + trimmed.len();
                    hits.push(TellHit {
                        start: para_abs_start,
                        end,
                        matched: truncate_for_display(
                            text.get(para_abs_start..end).unwrap_or(""),
                            200,
                        ),
                        severity: 3,
                        category: "structural:substitution_game_pronoun".to_owned(),
                        why: format!(
                            "{} sentences in this paragraph share the [{} {}] pronoun-template across {} distinct states — break the recursion",
                            states.len(), pronoun, verb, states.len(),
                        ),
                        suggested_replacement: None,
                    });
                    break; // one hit per paragraph is enough
                }
            }
        }
        paragraph_start += para.len() + 2;
    }
    hits
}

fn match_pronoun_template(sentence: &str) -> Option<(String, String, String)> {
    let toks = word_tokens(sentence);
    if toks.len() < 3 {
        return None;
    }
    const PRONOUNS: &[&str] = &["she", "he", "they", "it", "i", "you", "we"];
    const VERBS: &[&str] = &[
        "was", "is", "felt", "seemed", "looked", "became", "grew", "stood", "sat", "lay",
    ];
    if !PRONOUNS.contains(&toks[0].as_str()) {
        return None;
    }
    if !VERBS.contains(&toks[1].as_str()) {
        return None;
    }
    Some((toks[0].clone(), toks[1].clone(), toks[2].clone()))
}

/// Reduce a (lowercased) word to its singular base form via the
/// minimal English plural rules — strip `-ies` → `-y`, strip `-es`
/// only when the stem ends in `s/sh/ch/x/z`, otherwise strip `-s`.
/// Returns the input unchanged when no rule applies.
///
/// Deliberately simple: we want the lookup to FIND `"stones"` →
/// `"stone"` more often than we want it to be linguistically
/// correct. False positives (over-stemming) only matter for the
/// inverse direction (finding a non-noun in the list), which our
/// hand-curated `CONCRETE_NOUNS` rules out.
fn singularize(word: &str) -> String {
    if word.len() > 3 && word.ends_with("ies") {
        let mut base = word[..word.len() - 3].to_owned();
        base.push('y');
        return base;
    }
    if word.len() > 2 && word.ends_with("es") {
        let stem = &word[..word.len() - 2];
        if stem.ends_with('s')
            || stem.ends_with("sh")
            || stem.ends_with("ch")
            || stem.ends_with('x')
            || stem.ends_with('z')
        {
            return stem.to_owned();
        }
        return word[..word.len() - 1].to_owned();
    }
    if word.len() > 1 && word.ends_with('s') && !word.ends_with("ss") {
        return word[..word.len() - 1].to_owned();
    }
    word.to_owned()
}

// ── helpers (private) ─────────────────────────────────────────────────────

fn opening3(sentence: &str) -> Option<String> {
    let toks = word_tokens(sentence);
    if toks.len() < 3 {
        return None;
    }
    Some(toks.iter().take(3).cloned().collect::<Vec<_>>().join(" "))
}

/// Resolve each sentence to its (start, end) byte range in the source
/// text by left-to-right scanning. Sentences are returned in source
/// order by `split_sentences` so the cursor only moves forward.
fn locate_in_source(text: &str, sentences: &[String]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(sentences.len());
    let mut cursor = 0usize;
    for s in sentences {
        if let Some(rel) = text.get(cursor..).and_then(|tail| tail.find(s.as_str())) {
            let start = cursor + rel;
            let end = start + s.len();
            out.push((start, end));
            cursor = end;
        } else {
            // Splitter added/dropped whitespace — use the cursor as a
            // best-effort fallback so the chain hit still has a range.
            out.push((cursor, cursor + s.len()));
            cursor += s.len();
        }
    }
    out
}

fn match_substitution_template(sentence: &str) -> Option<String> {
    // We only look at the first 4 word tokens; the template is shaped
    // [Det] [Noun] (was|is|held|stood|carried) [Object].
    let toks = word_tokens(sentence);
    if toks.len() < 4 {
        return None;
    }
    const DETS: &[&str] = &["the", "a", "an", "her", "his", "their", "that", "this"];
    const VERBS: &[&str] = &["was", "is", "held", "stood", "carried", "felt"];
    if !DETS.contains(&toks[0].as_str()) {
        return None;
    }
    if !VERBS.contains(&toks[2].as_str()) {
        return None;
    }
    Some(toks[1].clone())
}

fn truncate_for_display(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

fn quartile(sorted: &[usize], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = (sorted.len() as f32 - 1.0) * q;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = pos - lo as f32;
    sorted[lo] as f32 * (1.0 - frac) + sorted[hi] as f32 * frac
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // The exact paragraphs flagged in the Run #11 quality-review.md.
    const RUN11_HAND_HELD: &str = "The hand held the letter. The hand held the date. \
                                    The hand held the name. The hand held the truth. \
                                    The hand held the lie. The hand held the silence. \
                                    The hand held the grief. The hand held the inheritance.";

    const RUN11_NAME_WAS: &str = "The name was a stone. The name was a scar. \
                                   The name was a wound. The name was a question. \
                                   The question was a scream. The scream was silent. \
                                   The scream was in the throat. The scream was in the chest.";

    const CLEAN_PROSE: &str = "She did not turn the light on. The fridge clicked once and went \
                                quiet. From the porch came the slow scrape of a chair drawn \
                                back across old wood, a sound she had not heard in three years \
                                and could still place. Arthur. The name arrived before she had \
                                decided to think it.";

    #[test]
    fn anaphora_chain_catches_run11_hand_held() {
        let hits = detect_anaphora_chain(RUN11_HAND_HELD);
        assert_eq!(hits.len(), 1, "single chain should produce one hit");
        let h = &hits[0];
        assert_eq!(h.category, "structural:anaphora_chain");
        assert!(h.why.contains("the hand held"));
        assert!(h.matched.starts_with("The hand held the letter"));
    }

    #[test]
    fn anaphora_chain_clean_on_varied_openings() {
        assert!(detect_anaphora_chain(CLEAN_PROSE).is_empty());
    }

    #[test]
    fn anaphora_chain_skips_short_runs() {
        // Three repeats — under the 4-sentence floor. Allowed device.
        let three = "The hand held the date. The hand held the name. The hand held the lie.";
        assert!(detect_anaphora_chain(three).is_empty());
    }

    #[test]
    fn substitution_game_catches_run11_name_was() {
        let hits = detect_substitution_game(RUN11_NAME_WAS);
        assert_eq!(hits.len(), 1, "single paragraph one hit");
        assert_eq!(hits[0].category, "structural:substitution_game");
        assert!(hits[0].why.contains("template"));
    }

    #[test]
    fn substitution_game_clean_on_varied_prose() {
        assert!(detect_substitution_game(CLEAN_PROSE).is_empty());
    }

    #[test]
    fn substitution_game_does_not_trigger_on_legitimate_repetition() {
        // Repeating ONE noun is allowed; only varying-noun slot-fill triggers.
        let single_noun = "The name was a stone. The name was a stone. \
                           The name was a stone. The name was a stone. \
                           The name was a stone.";
        assert!(detect_substitution_game(single_noun).is_empty());
    }

    #[test]
    fn low_variance_catches_uniform_short_paragraph() {
        let uniform = "She walked. He waited. They did not speak. It started to rain.";
        let hits = detect_low_variance_paragraphs(uniform);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].category, "structural:low_variance");
    }

    #[test]
    fn low_variance_clean_on_varied_paragraph() {
        assert!(detect_low_variance_paragraphs(CLEAN_PROSE).is_empty());
    }

    #[test]
    fn low_variance_exempts_short_paragraphs() {
        // 3 short sentences — under the 4-sentence floor.
        let three_short = "She left. He stayed. The house was quiet.";
        assert!(detect_low_variance_paragraphs(three_short).is_empty());
    }

    #[test]
    fn concrete_noun_catches_abstract_only_paragraph() {
        // All abstract nouns. No body, no objects, no senses.
        let abstract_only = "The grief was a question. The question had no answer. \
                             The silence was a kind of mercy. The mercy was its own betrayal.";
        let hits = detect_missing_concrete_noun(abstract_only);
        assert_eq!(hits.len(), 1, "abstract-only paragraph must be flagged");
        assert_eq!(hits[0].category, "structural:no_concrete_noun");
    }

    #[test]
    fn concrete_noun_clean_when_paragraph_has_sensory_anchor() {
        // Has "scrape", "wood", "chair", "porch" — multiple anchors.
        assert!(detect_missing_concrete_noun(CLEAN_PROSE).is_empty());
    }

    #[test]
    fn concrete_noun_exempts_very_short_paragraphs() {
        let two = "The grief had no name. The silence was its own answer.";
        assert!(detect_missing_concrete_noun(two).is_empty());
    }

    #[test]
    fn structural_tells_aggregator_returns_sorted_hits() {
        // Build a text with two flagged paragraphs separated by clean prose.
        let text = format!("{RUN11_HAND_HELD}\n\n{CLEAN_PROSE}\n\n{RUN11_NAME_WAS}");
        let hits = structural_tells(&text);
        assert!(hits.len() >= 2, "should flag both Run #11 paragraphs");
        // hits sorted by start
        let starts: Vec<usize> = hits.iter().map(|h| h.start).collect();
        let mut sorted = starts.clone();
        sorted.sort_unstable();
        assert_eq!(starts, sorted);
    }

    #[test]
    fn truncate_preserves_short_strings() {
        assert_eq!(truncate_for_display("hello", 100), "hello");
        let long = "a".repeat(500);
        let t = truncate_for_display(&long, 50);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() == 51);
    }

    // ── FEATURE_HARDENING_PLAN.md §3.1 — burstiness ──────────────────────

    #[test]
    fn burstiness_clean_on_varied_paragraph() {
        // 5+ sentences with varied lengths — should pass.
        let varied = "The room was quiet, the way old rooms get when the heat \
                      has clicked off and the wind has finally died down. \
                      She listened. From the porch came a single creak, then \
                      another, and then the long deliberate scrape of a chair \
                      drawn back across old wood. She did not move. Arthur. \
                      The name arrived before she had decided to think it.";
        let hits = detect_low_burstiness_paragraphs(varied);
        assert!(
            hits.is_empty(),
            "varied prose should not trip burstiness, got {hits:?}"
        );
    }

    #[test]
    fn burstiness_catches_uniformly_short_paragraph() {
        // 5 sentences, each exactly 4 tokens — variance ≈ 0, mean ≈ 4,
        // burstiness ≈ 0 — far below the 1.0 threshold.
        let uniform = "She walked down. He waited there. They did not speak. \
                       The rain was cold. The night stayed quiet.";
        let hits = detect_low_burstiness_paragraphs(uniform);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].category, "structural:low_burstiness");
    }

    #[test]
    fn burstiness_catches_uniformly_long_paragraph() {
        // The asymmetry vs. low_variance: this paragraph has all-LONG
        // sentences, but each is the same length so variance is ~0.
        let uniform_long = "The narrative unfolded in deliberate measured steps that built upon each preceding clause. \
                            The protagonist walked toward the doorway in a manner consistent with quiet contemplation and resolve. \
                            The room awaited her with the heavy stillness reserved for moments of necessary confrontation. \
                            The window framed a view of the street beyond which traffic continued indifferent to the small drama. \
                            The clock above the mantel offered the only sound besides her own measured breath in the long silence.";
        let hits = detect_low_burstiness_paragraphs(uniform_long);
        assert_eq!(hits.len(), 1, "uniformly LONG should also trip burstiness");
    }

    #[test]
    fn burstiness_exempts_short_paragraphs() {
        let four_sents = "She left. He stayed. The house was quiet. The rain fell.";
        assert!(detect_low_burstiness_paragraphs(four_sents).is_empty());
    }

    // ── FEATURE_HARDENING_PLAN.md §3.2 — lemmatize concrete-noun lookup ──

    #[test]
    fn singularize_handles_common_plurals() {
        assert_eq!(singularize("hands"), "hand");
        assert_eq!(singularize("stones"), "stone");
        assert_eq!(singularize("knives"), "knive"); // approximate; "knife" is irregular but already in list
        assert_eq!(singularize("cherries"), "cherry");
        assert_eq!(singularize("dishes"), "dish"); // s/sh stem case
        assert_eq!(singularize("walls"), "wall");
        // No-ops:
        assert_eq!(singularize("hand"), "hand");
        assert_eq!(singularize("name"), "name"); // doesn't end in s — no change
        assert_eq!(singularize("glass"), "glass"); // ends in -ss, stays
    }

    #[test]
    fn concrete_noun_finds_plural_anchors() {
        // 3-sentence paragraph using only PLURAL forms of concrete nouns.
        // Pre-§3.2 this would have falsely tripped; with lemmatization
        // it correctly finds the anchors.
        let prose = "Her hands shook. The stones in the wall stayed cold. \
                     The branches outside scratched against the window.";
        let hits = detect_missing_concrete_noun(prose);
        assert!(
            hits.is_empty(),
            "plurals should resolve via lemmatize, got {hits:?}"
        );
    }

    // ── FEATURE_HARDENING_PLAN.md §3.4 — pronoun substitution game ───────

    #[test]
    fn pronoun_substitution_catches_she_was_run() {
        // 5 sentences with same `[she was]` template across distinct states.
        let prose = "She was hungry. She was tired. She was alone. \
                     She was cold. She was finished.";
        let hits = detect_pronoun_substitution_game(prose);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].category, "structural:substitution_game_pronoun");
    }

    #[test]
    fn pronoun_substitution_clean_on_varied_pronouns() {
        // Same pattern but pronouns AND verbs vary — not a slot-fill game.
        let prose = "She was hungry. He was tired. They felt alone. \
                     I grew cold. You stood waiting.";
        assert!(detect_pronoun_substitution_game(prose).is_empty());
    }

    #[test]
    fn pronoun_substitution_does_not_trigger_on_4_repetitions() {
        // 4 reps is allowed — the threshold is 5+.
        let prose = "She was hungry. She was tired. She was alone. She was cold.";
        assert!(detect_pronoun_substitution_game(prose).is_empty());
    }

    #[test]
    fn structural_aggregator_now_runs_six_detectors() {
        // Build a paragraph that trips burstiness + pronoun-game.
        let prose = "She was hungry. She was tired. She was alone. \
                     She was cold. She was finished.";
        let hits = structural_tells(prose);
        let cats: std::collections::HashSet<&str> =
            hits.iter().map(|h| h.category.as_str()).collect();
        assert!(cats.contains("structural:substitution_game_pronoun"));
        assert!(cats.contains("structural:low_burstiness"));
    }
}
