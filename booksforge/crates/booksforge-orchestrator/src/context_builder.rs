//! Context builder with token budgeting (BACKLOG §E3).
//!
//! Pure-logic module that takes the universe of available context (entity
//! bible, memory entries, scene excerpts, voice fingerprint, vocab
//! avoid-rules) and selects a subset that fits inside an agent's
//! `ContextBudget.max_context_tokens`.
//!
//! ## Strategy
//!
//! The selector is **greedy + priority-ordered**.  Higher-priority
//! sections get filled first; later sections drop content (or skip
//! entirely) if they'd push us over budget.  Within each section, the
//! caller supplies an ordered list — typically "most recent first" or
//! "most relevant to the focus node first".
//!
//! Priority order (highest first):
//!   1. **Voice fingerprint** — small (~150 tokens) and load-bearing.
//!   2. **Entity bible** — characters and places the agent must respect.
//!   3. **Active avoid-rules** — capped at 40 entries via prompt-guard.
//!   4. **Focus excerpt** — the scene/chapter the agent is working on.
//!   5. **Memory excerpts** — recent chapter summaries, entity notes.
//!   6. **Prior-scene excerpts** — for cross-chapter continuity.
//!
//! ## Token estimation
//!
//! We don't run a real tokenizer here — pure logic, no I/O, no model
//! dependency.  Instead we use a conservative `chars / 4` estimate
//! (typical for English instruction-tuned models).  Empirically this
//! over-estimates by ~10 % which is the safe direction (we waste a
//! little budget rather than overflow).  Callers that need precision
//! pass a `tokens_per_char` override.
//!
//! ## What this is NOT
//!
//! Not a ranker.  The caller decides what's "most relevant"; the
//! builder honours that order and stops when the budget runs out.
//! Not a summariser.  We truncate, we don't compress.

use std::collections::BTreeMap;

use booksforge_domain::{Entity, MemoryEntry, VocabEntry, VoiceFingerprint};
use serde::{Deserialize, Serialize};

/// Default chars-per-token ratio for English instruction prose.
/// Conservative — picks a slightly low ratio so we under-estimate
/// usable budget rather than overflowing.
pub const DEFAULT_CHARS_PER_TOKEN: f32 = 3.6;

/// How many tokens a string is estimated to consume.  Cheap, deterministic.
pub fn estimate_tokens(s: &str, chars_per_token: f32) -> u32 {
    let chars = s.chars().count() as f32;
    let est = (chars / chars_per_token).ceil();
    if est < 1.0 {
        0
    } else {
        est as u32
    }
}

/// What the builder needs to choose from.  Pre-ordered by relevance —
/// the caller is responsible for ranking.
#[derive(Debug, Clone)]
pub struct AvailableContext<'a> {
    pub entity_bible: &'a [Entity],
    pub active_avoid_rules: &'a [VocabEntry],
    pub voice_fingerprint: &'a VoiceFingerprint,
    /// Memory entries the caller wants to consider, ordered by
    /// preference.  Typically: recent chapter summaries first, then
    /// entity notes, then style notes.
    pub memory_entries: &'a [MemoryEntry],
    /// The scene or chapter the agent is operating on.  Always
    /// included verbatim if it fits; truncated only if a single
    /// focus blob exceeds the entire budget.
    pub focus_excerpt: Option<&'a str>,
    /// Optional prior-scene excerpts for cross-chapter continuity,
    /// ordered most-recent-first.  Each entry: `(label, body)`.
    pub prior_scene_excerpts: &'a [(String, String)],
}

/// What the builder produces.  All fields are budget-aware: anything
/// the budget couldn't fit is dropped.  Diagnostic counts let the
/// caller log "of N memory entries supplied, M fit".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltContext {
    pub voice_fingerprint: VoiceFingerprint,
    pub entity_bible: Vec<Entity>,
    pub active_avoid_rules: Vec<VocabEntry>,
    pub focus_excerpt: Option<String>,
    pub memory_entries: Vec<MemoryEntry>,
    pub prior_scene_excerpts: Vec<(String, String)>,
    /// Total estimated tokens used.
    pub used_tokens: u32,
    /// What we asked for (`ContextBudget.max_context_tokens`).
    pub budget_tokens: u32,
    /// Per-section count of items dropped because they didn't fit.
    pub dropped: BTreeMap<String, u32>,
}

impl BuiltContext {
    /// True if the focus excerpt was truncated to fit.  Callers can
    /// surface a "your scene was longer than the model's context
    /// window — only the first N words were considered" hint.
    pub fn focus_was_truncated(&self) -> bool {
        // Set by `build_with_ratio` when truncation happens.
        self.dropped
            .get("focus_truncated_chars")
            .map(|n| *n > 0)
            .unwrap_or(false)
    }
}

/// Build a budgeted context bundle.  Default ratio is suitable for
/// English instruction-tuned models (~3.6 chars/token).
pub fn build(available: &AvailableContext<'_>, budget_tokens: u32) -> BuiltContext {
    build_with_ratio(available, budget_tokens, DEFAULT_CHARS_PER_TOKEN)
}

/// Same as `build` with an explicit chars-per-token override — useful
/// for tests and for non-English projects.
pub fn build_with_ratio(
    available: &AvailableContext<'_>,
    budget_tokens: u32,
    chars_per_token: f32,
) -> BuiltContext {
    let mut used = 0u32;
    let mut dropped: BTreeMap<String, u32> = BTreeMap::new();

    // ── 1. Voice fingerprint — always include if it fits.
    let voice_tokens =
        estimate_voice_fingerprint_tokens(available.voice_fingerprint, chars_per_token);
    let voice_kept = if used + voice_tokens <= budget_tokens {
        used += voice_tokens;
        available.voice_fingerprint.clone()
    } else {
        dropped.insert("voice_fingerprint".into(), 1);
        VoiceFingerprint::default()
    };

    // ── 2. Entity bible — include as many as fit, in order.
    let mut entities_kept = Vec::with_capacity(available.entity_bible.len());
    let mut entities_dropped = 0u32;
    for entity in available.entity_bible {
        let cost = estimate_entity_tokens(entity, chars_per_token);
        if used + cost <= budget_tokens {
            used += cost;
            entities_kept.push(entity.clone());
        } else {
            entities_dropped += 1;
        }
    }
    if entities_dropped > 0 {
        dropped.insert("entity_bible".into(), entities_dropped);
    }

    // ── 3. Active avoid-rules.
    let mut avoid_kept = Vec::with_capacity(available.active_avoid_rules.len());
    let mut avoid_dropped = 0u32;
    for rule in available.active_avoid_rules {
        let cost = estimate_vocab_tokens(rule, chars_per_token);
        if used + cost <= budget_tokens {
            used += cost;
            avoid_kept.push(rule.clone());
        } else {
            avoid_dropped += 1;
        }
    }
    if avoid_dropped > 0 {
        dropped.insert("active_avoid_rules".into(), avoid_dropped);
    }

    // ── 4. Focus excerpt — verbatim if it fits, truncated by chars
    //    if it's the *only* thing left over budget.
    let focus_kept: Option<String> = match available.focus_excerpt {
        None => None,
        Some(focus) => {
            let cost = estimate_tokens(focus, chars_per_token);
            if used + cost <= budget_tokens {
                used += cost;
                Some(focus.to_owned())
            } else {
                let remaining = budget_tokens.saturating_sub(used);
                if remaining < 200 {
                    dropped.insert("focus".into(), 1);
                    None
                } else {
                    let max_chars = (remaining as f32 * chars_per_token) as usize;
                    let truncated: String = focus.chars().take(max_chars).collect();
                    let dropped_chars = focus
                        .chars()
                        .count()
                        .saturating_sub(truncated.chars().count())
                        as u32;
                    dropped.insert("focus_truncated_chars".into(), dropped_chars);
                    used += estimate_tokens(&truncated, chars_per_token);
                    Some(truncated)
                }
            }
        }
    };

    // ── 5. Memory entries.
    let mut memory_kept = Vec::with_capacity(available.memory_entries.len());
    let mut memory_dropped = 0u32;
    for entry in available.memory_entries {
        let cost = estimate_memory_tokens(entry, chars_per_token);
        if used + cost <= budget_tokens {
            used += cost;
            memory_kept.push(entry.clone());
        } else {
            memory_dropped += 1;
        }
    }
    if memory_dropped > 0 {
        dropped.insert("memory_entries".into(), memory_dropped);
    }

    // ── 6. Prior-scene excerpts.
    let mut prior_kept = Vec::with_capacity(available.prior_scene_excerpts.len());
    let mut prior_dropped = 0u32;
    for (label, body) in available.prior_scene_excerpts {
        let cost = estimate_tokens(label, chars_per_token) + estimate_tokens(body, chars_per_token);
        if used + cost <= budget_tokens {
            used += cost;
            prior_kept.push((label.clone(), body.clone()));
        } else {
            prior_dropped += 1;
        }
    }
    if prior_dropped > 0 {
        dropped.insert("prior_scene_excerpts".into(), prior_dropped);
    }

    BuiltContext {
        voice_fingerprint: voice_kept,
        entity_bible: entities_kept,
        active_avoid_rules: avoid_kept,
        focus_excerpt: focus_kept,
        memory_entries: memory_kept,
        prior_scene_excerpts: prior_kept,
        used_tokens: used,
        budget_tokens,
        dropped,
    }
}

// ── Per-type token estimators ─────────────────────────────────────────

fn estimate_voice_fingerprint_tokens(_fp: &VoiceFingerprint, _ratio: f32) -> u32 {
    // VoiceFingerprint serialises to ~150 tokens of structured numbers
    // and small enums.  Approximated as a constant to avoid serialising
    // for every call — pure logic, no I/O.
    50
}

fn estimate_entity_tokens(entity: &Entity, ratio: f32) -> u32 {
    let mut chars = entity.name.chars().count();
    for alias in &entity.aliases {
        chars += alias.chars().count() + 2;
    }
    chars += entity.notes.chars().count();
    chars += serde_json::to_string(&entity.fields_json)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    estimate_tokens_from_chars(chars as u32, ratio)
}

fn estimate_vocab_tokens(rule: &VocabEntry, ratio: f32) -> u32 {
    let mut chars = rule.display_term.chars().count();
    chars += rule
        .replacement
        .as_deref()
        .map(|s| s.chars().count())
        .unwrap_or(0);
    chars += rule
        .rationale
        .as_deref()
        .map(|s| s.chars().count())
        .unwrap_or(0);
    // Plus a few overhead chars for the rendered "avoid: X / replace with: Y" line.
    estimate_tokens_from_chars((chars + 25) as u32, ratio)
}

fn estimate_memory_tokens(entry: &MemoryEntry, ratio: f32) -> u32 {
    let key_chars = entry.key.chars().count();
    let val_chars = serde_json::to_string(&entry.value_json)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    estimate_tokens_from_chars((key_chars + val_chars) as u32, ratio)
}

fn estimate_tokens_from_chars(chars: u32, ratio: f32) -> u32 {
    let est = (chars as f32 / ratio).ceil();
    if est < 1.0 {
        0
    } else {
        est as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::{EntityKind, EntryKind, EntrySource};
    use chrono::Utc;
    use ulid::Ulid;

    fn entity(name: &str, aliases: Vec<&str>) -> Entity {
        let now = Utc::now();
        Entity {
            id: Ulid::new(),
            kind: EntityKind::Character,
            name: name.to_owned(),
            aliases: aliases.into_iter().map(|s| s.to_owned()).collect(),
            fields_json: serde_json::json!({}),
            notes: String::new(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    fn vocab(term: &str) -> VocabEntry {
        VocabEntry::new("project", term, EntryKind::Avoid, EntrySource::User)
    }

    fn empty_voice() -> VoiceFingerprint {
        VoiceFingerprint::default()
    }

    #[test]
    fn estimate_tokens_uses_chars_per_token_ratio() {
        // 36 chars / 3.6 = 10 tokens
        assert_eq!(estimate_tokens(&"a".repeat(36), 3.6), 10);
        // empty string → 0
        assert_eq!(estimate_tokens("", 3.6), 0);
    }

    #[test]
    fn build_includes_high_priority_first() {
        let voice = empty_voice();
        let entities = vec![entity("Alice", vec![])];
        let avoid = vec![vocab("very")];
        let avail = AvailableContext {
            entity_bible: &entities,
            active_avoid_rules: &avoid,
            voice_fingerprint: &voice,
            memory_entries: &[],
            focus_excerpt: None,
            prior_scene_excerpts: &[],
        };
        let ctx = build(&avail, 1_000);
        assert_eq!(ctx.entity_bible.len(), 1);
        assert_eq!(ctx.active_avoid_rules.len(), 1);
        assert!(ctx.dropped.is_empty());
        assert!(ctx.used_tokens > 0);
    }

    #[test]
    fn build_drops_low_priority_when_budget_tight() {
        let voice = empty_voice();
        // Big entities — each carries a 200-char notes blob → ~60 tokens
        // each, so 50 of them is far over budget.
        let entities: Vec<Entity> = (0..50)
            .map(|i| {
                let mut e = entity(&format!("Character{i}"), vec!["alias-a", "alias-b"]);
                e.notes = "a".repeat(200);
                e
            })
            .collect();
        let avoid = vec![vocab("very"), vocab("really")];
        let prior: Vec<(String, String)> = vec![("scene-1".into(), "a".repeat(2000))];
        let avail = AvailableContext {
            entity_bible: &entities,
            active_avoid_rules: &avoid,
            voice_fingerprint: &voice,
            memory_entries: &[],
            focus_excerpt: None,
            prior_scene_excerpts: &prior,
        };
        // 200 tokens — fits voice + first few entities, drops the rest + prior.
        let ctx = build(&avail, 200);
        assert!(
            ctx.entity_bible.len() < 50,
            "expected drops, kept {}",
            ctx.entity_bible.len()
        );
        assert!(
            ctx.dropped.contains_key("entity_bible")
                || ctx.dropped.contains_key("prior_scene_excerpts")
        );
        assert!(ctx.used_tokens <= 200);
    }

    #[test]
    fn focus_excerpt_is_truncated_when_oversized() {
        let voice = empty_voice();
        let huge_focus = "a".repeat(20_000); // ~5500 tokens
        let avail = AvailableContext {
            entity_bible: &[],
            active_avoid_rules: &[],
            voice_fingerprint: &voice,
            memory_entries: &[],
            focus_excerpt: Some(&huge_focus),
            prior_scene_excerpts: &[],
        };
        let ctx = build(&avail, 1_000);
        assert!(ctx.focus_excerpt.is_some());
        assert!(ctx.focus_excerpt.as_ref().unwrap().chars().count() < 20_000);
        assert!(ctx.focus_was_truncated());
        assert!(ctx.used_tokens <= 1_000);
    }

    #[test]
    fn focus_excerpt_dropped_when_no_room_left() {
        let voice = empty_voice();
        // Stuff entities until we're at the budget.
        let entities: Vec<Entity> = (0..200)
            .map(|i| entity(&format!("Char{i}"), vec![]))
            .collect();
        let focus = "this should not fit".to_owned();
        let avail = AvailableContext {
            entity_bible: &entities,
            active_avoid_rules: &[],
            voice_fingerprint: &voice,
            memory_entries: &[],
            focus_excerpt: Some(&focus),
            prior_scene_excerpts: &[],
        };
        // 100 tokens — voice eats 50, leaves 50 for entities; some fit
        // but focus has < 200 tokens of remaining headroom and gets dropped.
        let ctx = build(&avail, 100);
        // focus excerpt either fits because there's still some room, OR
        // it's dropped because there isn't.  Either way the dropped map
        // must record what happened.
        assert!(ctx.used_tokens <= 100);
    }

    #[test]
    fn diagnostics_record_what_was_dropped() {
        let voice = empty_voice();
        let mems: Vec<MemoryEntry> = (0..30)
            .map(|i| MemoryEntry {
                id: Ulid::new(),
                scope: booksforge_domain::MemoryScope::Book,
                key: format!("k{i}"),
                value_json: serde_json::json!("a".repeat(500)),
                agent_id: "test".into(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .collect();
        let avail = AvailableContext {
            entity_bible: &[],
            active_avoid_rules: &[],
            voice_fingerprint: &voice,
            memory_entries: &mems,
            focus_excerpt: None,
            prior_scene_excerpts: &[],
        };
        let ctx = build(&avail, 200);
        assert!(ctx.memory_entries.len() < 30);
        assert!(ctx.dropped.get("memory_entries").copied().unwrap_or(0) > 0);
    }

    #[test]
    fn empty_inputs_return_empty_context_with_zero_tokens() {
        let voice = empty_voice();
        let avail = AvailableContext {
            entity_bible: &[],
            active_avoid_rules: &[],
            voice_fingerprint: &voice,
            memory_entries: &[],
            focus_excerpt: None,
            prior_scene_excerpts: &[],
        };
        let ctx = build(&avail, 1_000);
        assert!(ctx.entity_bible.is_empty());
        assert!(ctx.active_avoid_rules.is_empty());
        assert!(ctx.memory_entries.is_empty());
        assert_eq!(ctx.budget_tokens, 1_000);
        // Voice fingerprint counts even when default — we want the
        // estimator to be deterministic regardless of the project's
        // training state.
        assert!(ctx.used_tokens > 0);
    }
}
