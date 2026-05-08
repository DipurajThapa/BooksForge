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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBrief {
    pub title_suggestions:   Vec<String>,
    pub mode:                BookMode,
    pub genre:               String,
    pub audience:            String,
    pub tone:                String,
    /// Desired finished word count (5,000–250,000).
    pub target_word_count:   u32,
    /// One to three sentence premise.
    pub premise:             String,
    /// 1–6 key promises the book makes to the reader.
    pub key_promises:        Vec<String>,
    /// Up to 5 clarifying questions for the user.
    pub questions_for_user:  Vec<String>,
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
