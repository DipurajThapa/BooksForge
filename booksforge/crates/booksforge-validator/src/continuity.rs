//! F1 — Deterministic continuity linter.
//!
//! Pure-Rust pass that catches the high-confidence half of continuity
//! issues without an LLM call.  Output is `Vec<ContinuityFinding>`.
//! Findings flagged `ambiguous: true` are forwarded to the LLM
//! adjudicator (`continuity` agent).  High-confidence findings (`ambiguous:
//! false`) bypass the LLM and surface directly to the user.
//!
//! Four kinds, per AGENTS.md §4.5:
//!   - `name_drift`   — proper-noun candidate not in the entity bible.
//!   - `pov_drift`    — pronoun mix doesn't match `project_pov`.
//!   - `tense_drift`  — adjacent paragraphs flip between past and present.
//!   - `timeline`     — incompatible date/duration phrases co-occur.
//!
//! Each kind has a separate function so callers can run them à la carte
//! and per-kind unit tests stay tight.

use booksforge_domain::{ContinuityEvidence, ContinuityFinding, ContinuityKind, Entity, Severity};

/// Run all four detectors and return their union, sorted by `range_from`.
///
/// `node_id` is stamped onto each `ContinuityEvidence` (every finding here
/// is single-scene; cross-scene timeline checks are a V1.0 add).
pub fn lint_scene(
    node_id: &str,
    scene_text: &str,
    project_pov: Option<&str>,
    entity_bible: &[Entity],
) -> Vec<ContinuityFinding> {
    let mut out = Vec::new();
    out.extend(detect_name_drift(node_id, scene_text, entity_bible));
    if let Some(pov) = project_pov {
        out.extend(detect_pov_drift(node_id, scene_text, pov));
    }
    out.extend(detect_tense_drift(node_id, scene_text));
    out.extend(detect_timeline(node_id, scene_text));
    out.sort_by_key(|f| f.evidence.first().map(|e| e.range_from).unwrap_or(0));
    out
}

// ── name drift ────────────────────────────────────────────────────────────────

/// Capitalised tokens that don't appear (case-insensitively) in the bible.
/// Words shorter than 3 chars and ALL-CAPS acronyms are skipped.
pub fn detect_name_drift(
    node_id: &str,
    scene_text: &str,
    entity_bible: &[Entity],
) -> Vec<ContinuityFinding> {
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in entity_bible {
        known.insert(e.name.to_lowercase());
        for a in &e.aliases {
            known.insert(a.to_lowercase());
        }
    }
    for w in COMMON_PROPER {
        known.insert((*w).to_lowercase());
    }

    let mut findings = Vec::new();
    let chars: Vec<char> = scene_text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Skip non-letters.
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        // Skip if previous char ends a sentence (capitalisation is normal here).
        let sentence_initial = i == 0
            || chars[..i]
                .iter()
                .rev()
                .take_while(|c| c.is_whitespace())
                .count()
                > 0
                && chars[..i]
                    .iter()
                    .rev()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|c| matches!(c, '.' | '!' | '?'));

        let start = i;
        while i < chars.len() && chars[i].is_alphabetic() {
            i += 1;
        }
        let token: String = chars[start..i].iter().collect();
        if token.len() < 3 {
            continue;
        }
        if !token
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            continue;
        }
        if token.chars().all(|c| c.is_uppercase()) {
            continue;
        }
        if sentence_initial {
            continue;
        }

        if !known.contains(&token.to_lowercase()) {
            // Find a near-neighbour in the bible for the diagnosis hint.
            let nearest = nearest_alias(&token, entity_bible);
            let diagnosis = match nearest {
                Some((alias, dist)) if dist <= 2 => {
                    format!("'{token}' is not in the bible — possible drift of '{alias}'")
                }
                _ => format!("'{token}' is not a known character/place"),
            };
            findings.push(ContinuityFinding {
                kind: ContinuityKind::NameDrift,
                severity: Severity::Warning,
                evidence: vec![ContinuityEvidence {
                    node_id: node_id.to_owned(),
                    range_from: start as u32,
                    range_to: i as u32,
                    excerpt: token.clone(),
                }],
                diagnosis,
                ambiguous: nearest.is_none(),
            });
        }
    }
    findings
}

const COMMON_PROPER: &[&str] = &[
    "I",
    "Mr",
    "Mrs",
    "Ms",
    "Dr",
    "St",
    "Mt",
    "Lord",
    "Lady",
    "Sir",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
    "God",
    "Heaven",
    "Hell",
    "Earth",
];

/// Levenshtein distance, capped at 3 (we don't care beyond that).
fn levenshtein_capped(a: &str, b: &str, cap: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > cap {
        return cap + 1;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        let mut row_min = curr[0];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            if curr[j] < row_min {
                row_min = curr[j];
            }
        }
        if row_min > cap {
            return cap + 1;
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn nearest_alias<'a>(token: &str, bible: &'a [Entity]) -> Option<(&'a str, usize)> {
    let lower = token.to_lowercase();
    let mut best: Option<(&str, usize)> = None;
    for e in bible {
        for cand in std::iter::once(&e.name).chain(e.aliases.iter()) {
            let d = levenshtein_capped(&lower, &cand.to_lowercase(), 2);
            if d <= 2 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((cand.as_str(), d));
            }
        }
    }
    best
}

// ── POV drift ────────────────────────────────────────────────────────────────

/// Compare pronoun mix to the declared `project_pov` (e.g. `"first"`,
/// `"third-limited"`, `"third-omniscient"`).  Heuristic — flag scenes whose
/// pronoun ratio is the wrong shape for the declared POV.
pub fn detect_pov_drift(
    node_id: &str,
    scene_text: &str,
    project_pov: &str,
) -> Vec<ContinuityFinding> {
    let lower = scene_text.to_lowercase();
    // Token-bounded pronoun counts so we don't count "iron" as "I" etc.
    let count = |needles: &[&str]| -> usize { needles.iter().map(|n| count_word(&lower, n)).sum() };
    let first_singular = count(&["i", "me", "my", "mine", "myself"]);
    let third_singular = count(&[
        "he", "him", "his", "she", "her", "hers", "they", "them", "their", "theirs",
    ]);
    let total = first_singular + third_singular;
    if total < 5 {
        return Vec::new();
    } // too short to judge

    let first_ratio = first_singular as f64 / total as f64;
    let pov_lower = project_pov.to_lowercase();

    let drift = if pov_lower.starts_with("first") {
        // First-person scene with mostly third-person pronouns.
        first_ratio < 0.30
    } else if pov_lower.starts_with("third") {
        // Third-person scene leaking into first-person narration.
        first_ratio > 0.30
    } else {
        false
    };

    if drift {
        vec![ContinuityFinding {
            kind: ContinuityKind::PovDrift,
            severity: Severity::Warning,
            evidence: vec![ContinuityEvidence {
                node_id: node_id.to_owned(),
                range_from: 0,
                range_to: scene_text.chars().count().min(160) as u32,
                excerpt: scene_text.chars().take(160).collect(),
            }],
            diagnosis: format!(
                "expected POV '{project_pov}' but pronoun ratio is {:.0}% first-person",
                first_ratio * 100.0
            ),
            ambiguous: true, // POV drift is highly context-dependent — let LLM adjudicate
        }]
    } else {
        Vec::new()
    }
}

fn count_word(haystack: &str, needle: &str) -> usize {
    let mut n = 0;
    for token in haystack.split(|c: char| !c.is_alphabetic()) {
        if token == needle {
            n += 1;
        }
    }
    n
}

// ── tense drift ──────────────────────────────────────────────────────────────

/// Flag adjacent paragraphs whose tense flips past↔present.  Heuristic — count
/// `-ed` ending past-tense markers vs `-s/-es` present-tense third-person
/// markers per paragraph and flag flips where both signals exceed a floor.
pub fn detect_tense_drift(node_id: &str, scene_text: &str) -> Vec<ContinuityFinding> {
    let mut findings = Vec::new();
    let mut paragraphs: Vec<(usize, &str)> = Vec::new();
    let mut start = 0;
    for (i, c) in scene_text.char_indices() {
        if c == '\n' && scene_text[i + 1..].starts_with('\n') {
            paragraphs.push((start, &scene_text[start..i]));
            start = i + 2;
        }
    }
    if start < scene_text.len() {
        paragraphs.push((start, &scene_text[start..]));
    }
    let signals: Vec<(f64, f64)> = paragraphs
        .iter()
        .map(|(_, p)| {
            let words: Vec<&str> = p.split_whitespace().collect();
            if words.is_empty() {
                return (0.0, 0.0);
            }
            let mut past = 0;
            let mut pres = 0;
            for w in &words {
                let lower = w.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
                if lower.len() < 4 {
                    continue;
                }
                if lower.ends_with("ed") && !COMMON_ED_NON_PAST.contains(&lower.as_str()) {
                    past += 1;
                } else if (lower.ends_with('s') && !lower.ends_with("ss"))
                    && COMMON_PRESENT_S.contains(&lower.as_str())
                {
                    pres += 1;
                }
            }
            let n = words.len() as f64;
            (past as f64 / n, pres as f64 / n)
        })
        .collect();

    for window in signals.windows(2).enumerate() {
        let (i, w) = window;
        let (a_past, a_pres) = w[0];
        let (b_past, b_pres) = w[1];
        // A is mostly past, B is mostly present (or vice versa) — and both have signal.
        let flip = (a_past > 0.04 && b_pres > 0.02 && a_past > a_pres && b_pres > b_past)
            || (a_pres > 0.02 && b_past > 0.04 && a_pres > a_past && b_past > b_pres);
        if flip {
            let (start_b, _) = paragraphs[i + 1];
            let excerpt: String = paragraphs[i + 1].1.chars().take(120).collect();
            findings.push(ContinuityFinding {
                kind: ContinuityKind::TenseDrift,
                severity: Severity::Info,
                evidence: vec![ContinuityEvidence {
                    node_id: node_id.to_owned(),
                    range_from: start_b as u32,
                    range_to: (start_b + paragraphs[i + 1].1.len()) as u32,
                    excerpt,
                }],
                diagnosis: "tense flip between adjacent paragraphs".to_owned(),
                ambiguous: true,
            });
        }
    }
    findings
}

const COMMON_ED_NON_PAST: &[&str] = &[
    "indeed",
    "embed",
    "exceed",
    "succeed",
    "proceed",
    "feed",
    "need",
    "seed",
    "speed",
    "deed",
    "weed",
    "creed",
    "freed",
    "agreed",
    "decreed",
    "guaranteed",
    "red",
    "bed",
    "fed",
    "led",
];
const COMMON_PRESENT_S: &[&str] = &[
    "says", "thinks", "knows", "feels", "wants", "needs", "looks", "sees", "hears", "walks",
    "runs", "stands", "sits", "comes", "goes", "lives", "moves", "works", "smiles", "laughs",
    "shouts", "speaks", "writes", "reads", "holds", "takes", "makes", "gives", "calls", "asks",
    "answers", "tells", "explains",
];

// ── timeline ─────────────────────────────────────────────────────────────────

/// Flag co-occurring contradictory time markers in a single paragraph.
/// Very narrow — high precision, low recall.  False positives are worse than
/// false negatives here because the LLM adjudicator pays per call.
pub fn detect_timeline(node_id: &str, scene_text: &str) -> Vec<ContinuityFinding> {
    let mut findings = Vec::new();
    for (start, paragraph) in split_paragraphs(scene_text) {
        let lower = paragraph.to_lowercase();
        let pairs: &[(&str, &str)] = &[
            ("yesterday", "tomorrow"),
            ("yesterday", "next week"),
            ("last night", "tomorrow"),
            ("last night", "next week"),
            ("an hour ago", "next year"),
            ("this morning", "last week"),
        ];
        for (a, b) in pairs {
            if lower.contains(a) && lower.contains(b) {
                let excerpt: String = paragraph.chars().take(160).collect();
                findings.push(ContinuityFinding {
                    kind: ContinuityKind::Timeline,
                    severity: Severity::Warning,
                    evidence: vec![ContinuityEvidence {
                        node_id: node_id.to_owned(),
                        range_from: start as u32,
                        range_to: (start + paragraph.len()) as u32,
                        excerpt,
                    }],
                    diagnosis: format!("paragraph mentions both '{a}' and '{b}'"),
                    ambiguous: true,
                });
                break;
            }
        }
    }
    findings
}

fn split_paragraphs(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            out.push((start, &text[start..i]));
            start = i + 2;
            i += 2;
        } else {
            i += 1;
        }
    }
    if start < text.len() {
        out.push((start, &text[start..]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::EntityKind;
    use chrono::Utc;
    use ulid::Ulid;

    fn entity(name: &str, aliases: &[&str]) -> Entity {
        Entity {
            id: Ulid::new(),
            kind: EntityKind::Character,
            name: name.to_owned(),
            aliases: aliases.iter().map(|s| (*s).to_string()).collect(),
            fields_json: serde_json::json!({}),
            notes: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn name_drift_flags_unknown_capitalised() {
        let bible = vec![entity("Alice", &[])];
        let scene = "Alice walked. Then Beatrice arrived without warning.";
        let f = detect_name_drift("01HX", scene, &bible);
        assert!(f.iter().any(|x| x.evidence[0].excerpt == "Beatrice"));
    }

    #[test]
    fn name_drift_does_not_flag_aliases() {
        let bible = vec![entity("Alice", &["Ali"])];
        let scene = "Alice ran. Ali laughed quietly.";
        let f = detect_name_drift("01HX", scene, &bible);
        assert!(f.is_empty(), "expected no findings, got {f:?}");
    }

    #[test]
    fn name_drift_marks_close_matches_unambiguous() {
        let bible = vec![entity("Eliot", &[])];
        let scene = "Alice spoke. Then Eliotg entered.";
        let f = detect_name_drift("01HX", scene, &bible);
        let eliotg = f
            .iter()
            .find(|x| x.evidence[0].excerpt == "Eliotg")
            .expect("Eliotg flagged");
        assert!(!eliotg.ambiguous, "close match should be unambiguous");
    }

    #[test]
    fn pov_drift_flags_first_person_in_third_pov() {
        let scene = "I walked. I knew. I felt the cold. I waited. I sighed.";
        let f = detect_pov_drift("01HX", scene, "third-limited");
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn pov_drift_silent_when_too_short() {
        let f = detect_pov_drift("01HX", "I went.", "third-limited");
        assert!(f.is_empty());
    }

    #[test]
    fn timeline_flags_yesterday_tomorrow() {
        let scene = "Yesterday she promised to come tomorrow.";
        let f = detect_timeline("01HX", scene);
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn timeline_does_not_flag_consistent_paragraph() {
        let scene = "She arrived yesterday and stayed all night.";
        let f = detect_timeline("01HX", scene);
        assert!(f.is_empty());
    }

    #[test]
    fn lint_scene_returns_findings_sorted() {
        let bible = vec![entity("Alice", &[])];
        let scene = "Alice ran. Beatrice arrived. Yesterday she'd leave tomorrow.";
        let f = lint_scene("01HX", scene, Some("third-limited"), &bible);
        let positions: Vec<u32> = f.iter().map(|x| x.evidence[0].range_from).collect();
        let mut sorted = positions.clone();
        sorted.sort();
        assert_eq!(positions, sorted);
    }
}
