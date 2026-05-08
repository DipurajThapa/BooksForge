//! Anti-AI-tell prompt guard — assembled at orchestrator-binding time and
//! injected into every prose-emitting agent's prompt vars.
//!
//! Three layers compose into a single `PromptGuard` block that ships to
//! the model:
//!
//! 1. **Active-vocab guard** — the project's currently-active layered
//!    vocabulary `avoid` list, formatted as a numbered watch-list.
//!    Rendered from the project's `vocab_entries` rows resolved through
//!    `resolve_vocab` (most-specific layer wins).
//!
//! 2. **Voice fingerprint guard** — the project's `VoiceFingerprint`
//!    rendered as concrete sentence-cadence / em-dash / discourse-marker
//!    targets so the model writes in the project's established voice
//!    rather than a model-default register.
//!
//! 3. **Empathy & humanity guard** — fixed prose covering emotional
//!    clarity, sensory specificity, and concrete-over-abstract
//!    language.  The same block ships to every prose-emitting agent so
//!    Copyeditor / Humanization / Chapter-Drafter / Final-Review-Editor
//!    apply consistent humanity criteria.
//!
//! The guard is *additive* — each agent's primary prompt template is
//! responsible for its task; the guard is appended as an "Always
//! observe these constraints" section.

use booksforge_domain::{VoiceFingerprint, EntryKind};

/// One vocab `avoid` rule rendered for the prompt.
#[derive(Debug, Clone)]
pub struct AvoidRule<'a> {
    pub term:        &'a str,
    pub kind:        EntryKind,
    pub replacement: Option<&'a str>,
    pub rationale:   &'a str,
}

/// Render the full guard block.  Caller passes the project's resolved
/// `avoid`/`replace` rules plus the `VoiceFingerprint`.
pub fn render(
    avoid_rules: &[AvoidRule<'_>],
    fingerprint: &VoiceFingerprint,
) -> String {
    let mut out = String::with_capacity(2_500);
    out.push_str("\n\n=== ALWAYS-OBSERVE CONSTRAINTS ===\n");
    out.push_str(&render_originality_ethics());
    out.push('\n');
    out.push_str(&render_humanity());
    out.push('\n');
    out.push_str(&render_voice(fingerprint));
    out.push('\n');
    out.push_str(&render_avoid_block(avoid_rules));
    out.push_str("\n=== END CONSTRAINTS ===\n");
    out
}

// ── Originality / anti-plagiarism block (static) ─────────────────────────────

/// Ethics + anti-plagiarism rules.  Rendered first so plagiarism is the
/// most prominent constraint the model sees.  Pairs with the local
/// `Originality` cross-cutting validator that runs on every output —
/// belt-and-braces: prompt-level guidance + post-hoc detection.
fn render_originality_ethics() -> String {
    [
        "Originality & attribution (non-negotiable)",
        "  - Do NOT copy long verbatim spans from the source you were given.  Generate original phrasing.  A 12+ word stretch lifted from the input is plagiarism and will be flagged and rejected.",
        "  - Do NOT recycle long stretches from prior chapters.  If a callback is intentional, reference the prior beat with new wording.",
        "  - Do NOT reproduce text from other authors, news articles, song lyrics, poems, or copyrighted works.  If a quote is essential, mark it explicitly with attribution: who said it, where it appeared.",
        "  - When the user supplies a real-world reference (a book title, a person's name, a place), summarise or paraphrase facts in your own words; do not reproduce passages from those sources.",
        "  - Citations: if you must include a verbatim line that you did not write, wrap it in straight ASCII quotes (\"…\") AND name the source in adjacent prose.  The originality detector treats quoted-and-attributed spans as legitimate citations.",
        "  - When uncertain whether something is original, write it differently rather than risk overlap.",
    ].join("\n") + "\n"
}

// ── Humanity & empathy block (static) ────────────────────────────────────────

/// Static empathy / humanity guidance.  Updated only with a prompt
/// template version bump.  Not parameterised on the project — these are
/// the project-wide "what makes prose feel human" rules.
fn render_humanity() -> String {
    [
        "Humanity & empathy",
        "  - Specific over abstract: name the thing.  \"a cracked teacup\", not \"a vessel\".",
        "  - Sensory grounding: when a scene turns emotional, anchor it in one concrete sense (a sound, a smell, a texture).  Avoid abstract emotion words alone.",
        "  - Show interiority through *behaviour*: hesitations, half-finished sentences, the small thing a character does with their hands.",
        "  - Subtext over explanation: if a character is angry, do not write \"she was angry\".  Write the line the angry version of her would say.",
        "  - Stakes must be legible to a stranger.  After every paragraph, the reader should know what the protagonist wants and what is in their way.",
        "  - Empathy is *witnessing*, not *narrating*.  Resist the omniscient explainer voice.",
    ].join("\n") + "\n"
}

// ── Voice fingerprint block (per-project) ────────────────────────────────────

fn render_voice(fp: &VoiceFingerprint) -> String {
    if !fp.is_established() {
        return [
            "Voice (project voice not yet established)",
            "  - Use varied sentence length: alternate short (5–10 words) and longer (18–28 words) sentences.  Avoid uniform mid-length cadence.",
            "  - Em-dashes are seasoning, not glue.  Use sparingly.",
            "  - Avoid stock discourse markers (indeed, moreover, furthermore, thus).",
        ].join("\n") + "\n";
    }

    let cadence_hint = if fp.sentence_words_stddev / fp.sentence_words_mean.max(1.0) < 0.4 {
        "  - The project's existing prose has VARIED sentence length.  Match it: alternate short and long.\n".to_owned()
    } else {
        format!(
            "  - Match the project's cadence: mean {:.0} words/sentence, stddev {:.0}.\n",
            fp.sentence_words_mean, fp.sentence_words_stddev,
        )
    };
    let em_hint = if fp.em_dash_per_1000 < 2.5 {
        "  - This project uses em-dashes sparingly (< 2.5 per 1k words).  Do not over-use them.\n"
    } else {
        ""
    };
    let triad_hint = if fp.ai_tell_triad_per_1000 < 0.1 {
        "  - This project's voice does NOT use the words \"delve\", \"tapestry\", or \"intricate\".  Do not introduce them.\n"
    } else {
        ""
    };
    let ttr_hint = if fp.type_token_ratio > 0.45 {
        "  - This project's voice is vocabulary-rich.  Prefer specific over generic words.\n"
    } else {
        ""
    };

    let mut buf = String::from("Voice (match the project's existing prose)\n");
    buf.push_str(&cadence_hint);
    buf.push_str(em_hint);
    buf.push_str(triad_hint);
    buf.push_str(ttr_hint);
    buf
}

// ── Avoid-rule block (per-project, layered vocab) ────────────────────────────

fn render_avoid_block(rules: &[AvoidRule<'_>]) -> String {
    if rules.is_empty() {
        return "Avoid-rules\n  (none active for this project)\n".to_owned();
    }
    let mut buf = String::from("Avoid-rules — these terms are flagged for this project\n");
    for (i, r) in rules.iter().take(40).enumerate() {
        let kind_str = match r.kind {
            EntryKind::Avoid   => "AVOID",
            EntryKind::Replace => "REPLACE",
            EntryKind::Prefer  => "PREFER",
        };
        let line = match (r.kind, r.replacement) {
            (EntryKind::Replace, Some(rep)) => format!("  {:>2}. [{kind_str}] \"{}\" → \"{rep}\" — {}", i + 1, r.term, r.rationale),
            _                               => format!("  {:>2}. [{kind_str}] \"{}\" — {}",            i + 1, r.term, r.rationale),
        };
        buf.push_str(&line);
        buf.push('\n');
    }
    if rules.len() > 40 {
        buf.push_str(&format!("  …and {} more (truncated to fit context).\n", rules.len() - 40));
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fp() -> VoiceFingerprint {
        VoiceFingerprint {
            corpus_tokens: 5_000,
            sentence_words_mean: 14.0, sentence_words_stddev: 9.0,
            em_dash_per_1000: 1.0, ly_adverb_per_1000: 8.0,
            ai_tell_triad_per_1000: 0.0, discourse_marker_per_1000: 0.5,
            type_token_ratio: 0.50,
        }
    }

    #[test]
    fn render_includes_all_three_blocks() {
        let r = render(&[], &sample_fp());
        assert!(r.contains("Humanity & empathy"));
        assert!(r.contains("Voice"));
        assert!(r.contains("Avoid-rules"));
    }

    #[test]
    fn voice_guard_for_unestablished_uses_generic_advice() {
        let r = render_voice(&VoiceFingerprint::default());
        assert!(r.contains("not yet established"));
    }

    #[test]
    fn avoid_block_renders_replacements() {
        let rules = vec![AvoidRule {
            term: "delve",
            kind: EntryKind::Replace,
            replacement: Some("explore"),
            rationale: "AI-tell",
        }];
        let block = render_avoid_block(&rules);
        assert!(block.contains("\"delve\" → \"explore\""));
    }

    #[test]
    fn avoid_block_truncates_long_lists() {
        let rules: Vec<AvoidRule> = (0..100).map(|_| AvoidRule {
            term: "x", kind: EntryKind::Avoid, replacement: None, rationale: "y",
        }).collect();
        let block = render_avoid_block(&rules);
        assert!(block.contains("more (truncated"));
    }

    #[test]
    fn empty_avoid_list_renders_none_message() {
        let block = render_avoid_block(&[]);
        assert!(block.contains("none active"));
    }
}
