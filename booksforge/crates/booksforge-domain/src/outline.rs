use serde::{Deserialize, Serialize};

/// A single scene entry within an outline chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePlan {
    /// One-sentence synopsis of the scene.
    pub synopsis:          String,
    /// Point-of-view character, if applicable.
    pub pov:               Option<String>,
    /// Story beat or structural marker (e.g., "inciting incident").
    pub beat:              Option<String>,
    /// Suggested word count target for this scene.
    pub target_word_count: Option<u32>,
}

/// A single chapter within an outline part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterPlan {
    pub title:   String,
    /// One-sentence statement of the chapter's narrative purpose.
    pub purpose: String,
    pub scenes:  Vec<ScenePlan>,
}

/// A story part (act / section) grouping chapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartPlan {
    pub title:    String,
    pub purpose:  String,
    pub chapters: Vec<ChapterPlan>,
}

/// Full outline proposal returned by the Outline Architect Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineProposal {
    pub parts:         Vec<PartPlan>,
    /// Rationale for structural choices (≤300 words).
    pub rationale:     String,
    /// Advice / caveats the agent wants to surface to the user.
    pub notes_to_user: Vec<String>,
}

impl OutlineProposal {
    /// Total chapter count across all parts.
    pub fn chapter_count(&self) -> usize {
        self.parts.iter().map(|p| p.chapters.len()).sum()
    }

    /// Total target word count across all scenes.
    pub fn total_target_words(&self) -> u32 {
        self.parts
            .iter()
            .flat_map(|p| p.chapters.iter())
            .flat_map(|c| c.scenes.iter())
            .filter_map(|s| s.target_word_count)
            .sum()
    }

    /// Validate structural invariants (separate from JSON-schema validation).
    pub fn validate(&self, target_chapter_count: u32, brief_word_count: u32) -> Vec<String> {
        let mut errors = Vec::new();

        let actual = self.chapter_count() as u32;
        let low  = (target_chapter_count as f64 * 0.8).floor() as u32;
        let high = (target_chapter_count as f64 * 1.2).ceil()  as u32;
        if actual < low || actual > high {
            errors.push(format!(
                "chapter count {actual} is outside ±20% of target {target_chapter_count}"
            ));
        }

        if brief_word_count > 0 {
            let total = self.total_target_words();
            let wc_low  = (brief_word_count as f64 * 0.8).floor() as u32;
            let wc_high = (brief_word_count as f64 * 1.2).ceil()  as u32;
            if total > 0 && (total < wc_low || total > wc_high) {
                errors.push(format!(
                    "total target word count {total} is outside ±20% of brief target {brief_word_count}"
                ));
            }
        }

        // Every scene must have a non-empty synopsis.
        for (pi, part) in self.parts.iter().enumerate() {
            for (ci, ch) in part.chapters.iter().enumerate() {
                for (si, sc) in ch.scenes.iter().enumerate() {
                    if sc.synopsis.trim().is_empty() {
                        errors.push(format!("part[{pi}].chapters[{ci}].scenes[{si}] has empty synopsis"));
                    }
                }
            }
        }

        errors
    }
}
