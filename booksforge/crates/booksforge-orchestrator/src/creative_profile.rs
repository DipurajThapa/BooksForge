//! Story-uniqueness prompt block — assembled at orchestrator-binding
//! time and injected into every generative agent's prompt vars under
//! `creative_profile`.
//!
//! Sibling of `prompt_guard.rs`. Where `prompt_guard` enforces project-
//! wide humanity / voice / avoid rules, `creative_profile` carries the
//! uniqueness signals that should make THIS story THIS story:
//!
//! 1. **Genre pack** — the system prompt + drafting lens + hard rules
//!    sourced from `booksforge-genre-packs::pack_for(book_kind)`. Without
//!    this, two writers picking the same kind get architecturally
//!    similar books.
//!
//! 2. **Brief uniqueness fields** — comp authors, theme keywords,
//!    forbidden tropes, era setting, cultural context, creative seed.
//!    These are extracted by the intake agent and persisted on the
//!    `ProjectBrief`. They re-enter every generative prompt so the
//!    bibles, outline, drafts, and polish passes all braid the same
//!    threads.
//!
//! The block is *additive* and prefixed `=== CREATIVE PROFILE ===` so
//! prompt templates that splice `{{ creative_profile }}` get a
//! visually-distinct, copy-paste-stable section the model will read.
//!
//! When the project has no `book_kind` set yet (legacy projects pre-
//! Phase 4) and no brief uniqueness fields, this returns an empty
//! string — every existing template that splices `{{ creative_profile }}`
//! degrades cleanly to its prior behaviour.

use booksforge_domain::{BookKind, ProjectBrief};
use booksforge_genre_packs::{pack_for, GenrePack};

/// Per-project uniqueness data the runner uses to render the
/// `creative_profile` block. Built once per run by the caller (Tauri
/// command layer) from the project's `book_kind` plus its persisted
/// `ProjectBrief` (if any).
#[derive(Debug, Clone, Default)]
pub struct CreativeProfile {
    pub book_kind: Option<BookKind>,
    pub comp_titles_or_authors: Vec<String>,
    pub theme_keywords: Vec<String>,
    pub forbidden_tropes: Vec<String>,
    pub era_setting: Option<String>,
    pub cultural_context: Option<String>,
    pub creative_seed: Option<String>,
}

impl CreativeProfile {
    /// Construct from a `ProjectBrief` plus the project's current
    /// `book_kind`. The brief's uniqueness fields flow through directly
    /// — they are the writer's stated intent, not derived data.
    pub fn from_brief(book_kind: Option<BookKind>, brief: &ProjectBrief) -> Self {
        Self {
            book_kind,
            comp_titles_or_authors: brief.comp_titles_or_authors.clone(),
            theme_keywords: brief.theme_keywords.clone(),
            forbidden_tropes: brief.forbidden_tropes.clone(),
            era_setting: brief.era_setting.clone(),
            cultural_context: brief.cultural_context.clone(),
            creative_seed: brief.creative_seed.clone(),
        }
    }

    /// True when nothing meaningful is set — caller can short-circuit
    /// the prompt-var injection and let templates degrade gracefully.
    pub fn is_empty(&self) -> bool {
        self.book_kind.is_none()
            && self.comp_titles_or_authors.is_empty()
            && self.theme_keywords.is_empty()
            && self.forbidden_tropes.is_empty()
            && self.era_setting.is_none()
            && self.cultural_context.is_none()
            && self.creative_seed.is_none()
    }
}

/// Render the full creative-profile block. Returns an empty string when
/// the profile carries no signal — callers can splice it
/// unconditionally without producing dangling section headers.
pub fn render(profile: &CreativeProfile) -> String {
    if profile.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(2_000);
    out.push_str("\n\n=== CREATIVE PROFILE ===\n");
    if let Some(kind) = profile.book_kind {
        out.push_str(&render_genre_pack(&pack_for(kind)));
        out.push('\n');
    }
    out.push_str(&render_brief_signals(profile));
    out.push_str("=== END CREATIVE PROFILE ===\n");
    out
}

// ── Genre-pack section (per book_kind) ───────────────────────────────────────

fn render_genre_pack(pack: &GenrePack) -> String {
    let mut buf = String::with_capacity(800);
    buf.push_str(&format!("Genre system ({}):\n  ", pack.genre_label));
    buf.push_str(&pack.system_prompt.replace('\n', "\n  "));
    buf.push('\n');

    buf.push_str("Drafting lens:\n  ");
    buf.push_str(&pack.draft_lens.replace('\n', "\n  "));
    buf.push('\n');

    if !pack.hard_rules.is_empty() {
        buf.push_str("Genre hard rules (non-negotiable):\n");
        for (i, rule) in pack.hard_rules.iter().enumerate() {
            buf.push_str(&format!("  {:>2}. {}\n", i + 1, rule));
        }
    }
    buf
}

// ── Per-brief uniqueness section ─────────────────────────────────────────────

fn render_brief_signals(p: &CreativeProfile) -> String {
    let mut buf = String::with_capacity(600);
    buf.push_str("Story uniqueness (this story is not a generic specimen of the genre):\n");

    if !p.comp_titles_or_authors.is_empty() {
        buf.push_str(&format!(
            "  - Comp anchors (touchstones for voice + mood, not models to imitate): {}\n",
            p.comp_titles_or_authors.join(", "),
        ));
    } else {
        buf.push_str("  - Comp anchors: (none provided — pick concrete sensory anchors yourself, do not default to generic genre tropes)\n");
    }

    if !p.theme_keywords.is_empty() {
        buf.push_str(&format!(
            "  - Theme keywords (must thread through every scene, not all at once): {}\n",
            p.theme_keywords.join(", "),
        ));
    }

    if !p.forbidden_tropes.is_empty() {
        buf.push_str(&format!(
            "  - Forbidden tropes / patterns (do NOT use, even when convenient): {}\n",
            p.forbidden_tropes.join("; "),
        ));
    }

    if let Some(era) = &p.era_setting {
        buf.push_str(&format!(
            "  - Era / setting anchor (every sensory detail must respect it): {era}\n",
        ));
    }

    if let Some(culture) = &p.cultural_context {
        buf.push_str(&format!(
            "  - Cultural context (shapes voice, stakes, and idiom): {culture}\n",
        ));
    }

    if let Some(seed) = &p.creative_seed {
        buf.push_str(&format!(
            "  - Creative seed (structural angle — let it shape choices the genre wouldn't suggest by default): {seed}\n",
        ));
    }
    buf
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn brief_with(comp: Vec<&str>, themes: Vec<&str>) -> ProjectBrief {
        ProjectBrief {
            title_suggestions: vec!["Untitled".into()],
            mode: booksforge_domain::BookMode::Fiction,
            genre: "literary".into(),
            audience: "adult".into(),
            tone: "spare".into(),
            target_word_count: 50_000,
            premise: "A premise.".into(),
            key_promises: vec!["promise".into()],
            questions_for_user: vec![],
            comp_titles_or_authors: comp.into_iter().map(String::from).collect(),
            theme_keywords: themes.into_iter().map(String::from).collect(),
            forbidden_tropes: vec![],
            era_setting: None,
            cultural_context: None,
            creative_seed: None,
        }
    }

    #[test]
    fn empty_profile_renders_empty_string() {
        let p = CreativeProfile::default();
        assert!(p.is_empty());
        assert_eq!(render(&p), "");
    }

    #[test]
    fn profile_with_book_kind_includes_genre_pack() {
        let brief = brief_with(vec![], vec![]);
        let p = CreativeProfile::from_brief(Some(BookKind::LiteraryFiction), &brief);
        let block = render(&p);
        assert!(block.contains("CREATIVE PROFILE"));
        assert!(block.contains("Genre system"));
        assert!(block.contains("literary_fiction") || block.contains("LITERARY"));
    }

    #[test]
    fn profile_with_brief_signals_only_renders_them() {
        let brief = brief_with(vec!["Le Guin", "Station Eleven"], vec!["loneliness"]);
        let p = CreativeProfile::from_brief(None, &brief);
        let block = render(&p);
        assert!(block.contains("Comp anchors"));
        assert!(block.contains("Le Guin"));
        assert!(block.contains("Theme keywords"));
        assert!(block.contains("loneliness"));
        assert!(!block.contains("Genre system"));
    }

    #[test]
    fn forbidden_tropes_render_with_warning_phrasing() {
        let mut brief = brief_with(vec![], vec![]);
        brief.forbidden_tropes = vec!["chosen-one".into(), "love-triangle".into()];
        let p = CreativeProfile::from_brief(None, &brief);
        let block = render(&p);
        assert!(block.contains("Forbidden tropes"));
        assert!(block.contains("do NOT"));
        assert!(block.contains("chosen-one"));
    }

    #[test]
    fn empty_comps_renders_explicit_anti_default_advice() {
        let brief = brief_with(vec![], vec!["x"]);
        let p = CreativeProfile::from_brief(None, &brief);
        let block = render(&p);
        assert!(block.contains("(none provided"));
    }

    #[test]
    fn era_and_cultural_context_render_when_set() {
        let mut brief = brief_with(vec![], vec![]);
        brief.era_setting = Some("1990s rural Pennsylvania".into());
        brief.cultural_context = Some("Bengali-American immigrant".into());
        let p = CreativeProfile::from_brief(None, &brief);
        let block = render(&p);
        assert!(block.contains("Era / setting"));
        assert!(block.contains("1990s"));
        assert!(block.contains("Cultural context"));
    }

    #[test]
    fn creative_seed_renders_when_set() {
        let mut brief = brief_with(vec![], vec![]);
        brief.creative_seed = Some("tell it backwards from the funeral".into());
        let p = CreativeProfile::from_brief(None, &brief);
        let block = render(&p);
        assert!(block.contains("Creative seed"));
        assert!(block.contains("backwards from the funeral"));
    }
}
