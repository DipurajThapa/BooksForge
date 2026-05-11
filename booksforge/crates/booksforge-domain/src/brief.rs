use serde::{Deserialize, Serialize};

use crate::project::BookMode;

/// Free-text idea submitted by the user — input to the Intake Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawIdea {
    /// The user's book idea; ≤4,000 characters.
    pub idea_text: String,
    /// Optional mode hint from the user.
    pub preferred_mode: Option<BookMode>,
}

/// Structured project brief produced by the Intake Agent.
///
/// This is the output of `intake` and the input to `outline-architect`.
///
/// The trailing `Option`/`Vec` fields default to empty so older briefs
/// (serialized before the uniqueness fields landed) deserialize without
/// migration. They drive the `creative_profile` block injected into every
/// generative prompt — the cure for "two writers with the same brief get
/// near-identical books."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBrief {
    pub title_suggestions: Vec<String>,
    pub mode: BookMode,
    pub genre: String,
    pub audience: String,
    pub tone: String,
    /// Desired finished word count (5,000–250,000).
    pub target_word_count: u32,
    /// One to three sentence premise.
    pub premise: String,
    /// 1–6 key promises the book makes to the reader.
    pub key_promises: Vec<String>,
    /// Up to 5 clarifying questions for the user.
    pub questions_for_user: Vec<String>,
    // ── Uniqueness signals — flow into every generative prompt via
    //    `creative_profile` (orchestrator-injected). All optional; if
    //    intake can't extract them from the writer's idea text, leave
    //    empty rather than invent.
    /// Comp titles or authors the writer named (e.g. "Ursula K. Le Guin",
    /// "The Secret History"). Anchors voice, mood, and genre conventions.
    #[serde(default)]
    pub comp_titles_or_authors: Vec<String>,
    /// Recurring theme keywords / writer obsessions (e.g.
    /// "loneliness", "inheritance", "second-language identity"). Re-injected
    /// scene-by-scene so every chapter braids the same concerns.
    #[serde(default)]
    pub theme_keywords: Vec<String>,
    /// Tropes / patterns the writer wants to avoid (e.g.
    /// "love-triangle", "chosen-one", "AI tells like 'tapestry'").
    #[serde(default)]
    pub forbidden_tropes: Vec<String>,
    /// Time-and-place anchor (e.g. "1990s rural Pennsylvania",
    /// "near-future Lagos"). When set, every scene's sensory palette
    /// must respect it.
    #[serde(default)]
    pub era_setting: Option<String>,
    /// Cultural context shaping voice and stakes (e.g. "Bengali-American
    /// immigrant", "post-Soviet Estonia"). Distinct from `era_setting` —
    /// place can be the same but cultural lens different.
    #[serde(default)]
    pub cultural_context: Option<String>,
    /// "Creative seed" — a short phrase the drafter uses as a
    /// structural divergence dial. Two runs with the same brief but
    /// different seeds explore different angles. Optional; empty =
    /// model picks. Examples: "tell it backwards from the funeral",
    /// "treat each chapter as a deposition transcript".
    #[serde(default)]
    pub creative_seed: Option<String>,
}

impl ProjectBrief {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(5_000..=250_000).contains(&self.target_word_count) {
            return Err("target_word_count must be between 5 000 and 250 000");
        }
        if self.key_promises.is_empty() || self.key_promises.len() > 6 {
            return Err("key_promises must have 1–6 entries");
        }
        if self.questions_for_user.len() > 5 {
            return Err("questions_for_user must have at most 5 entries");
        }
        Ok(())
    }
}
