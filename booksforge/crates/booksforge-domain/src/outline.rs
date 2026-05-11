use serde::{Deserialize, Serialize};

/// A single scene entry within an outline chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePlan {
    /// One-sentence synopsis of the scene.
    pub synopsis: String,
    /// Point-of-view character, if applicable.
    pub pov: Option<String>,
    /// Story beat or structural marker (e.g., "inciting incident").
    pub beat: Option<String>,
    /// Suggested word count target for this scene.
    pub target_word_count: Option<u32>,
}

/// A single chapter within an outline part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterPlan {
    pub title: String,
    /// One-sentence statement of the chapter's narrative purpose.
    /// Optional in deserialisation (`#[serde(default)]` → `""`) because
    /// local LLMs occasionally omit it on long outlines and we'd rather
    /// keep the rest of the outline than reject the whole proposal.
    /// The prompt still asks for it; this only affects salvage.
    #[serde(default)]
    pub purpose: String,
    /// Same `#[serde(default)]` rationale as `purpose`: 9B-class models
    /// sometimes truncate the per-chapter `scenes` array on long
    /// outlines (12+ chapters). Defaulting to an empty vec lets the
    /// outline parse; the chapter just has zero scenes, which the
    /// writer can fill in manually or via a follow-up agent run.
    /// Without this, ONE missing array kills the whole proposal and
    /// burns 30-60s of model time per retry.
    #[serde(default)]
    pub scenes: Vec<ScenePlan>,
}

/// A story part (act / section) grouping chapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartPlan {
    pub title: String,
    /// Optional in deserialisation for the same reason as `ChapterPlan.purpose`.
    #[serde(default)]
    pub purpose: String,
    /// Same `#[serde(default)]` rationale as `ChapterPlan.scenes`:
    /// salvage parsing of an outline whose final part dropped its
    /// `chapters` array under JSON-mode budget pressure.
    #[serde(default)]
    pub chapters: Vec<ChapterPlan>,
}

/// Full outline proposal returned by the Outline Architect Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineProposal {
    pub parts: Vec<PartPlan>,
    /// Rationale for structural choices (≤300 words).
    /// Optional on deserialise because local LLMs sometimes truncate
    /// the trailing `rationale` / `notes_to_user` fields when the
    /// preceding `parts` array exhausts the JSON-mode budget — we'd
    /// rather salvage the structural outline than reject the whole
    /// proposal over missing metadata.
    #[serde(default)]
    pub rationale: String,
    /// Advice / caveats the agent wants to surface to the user.
    #[serde(default)]
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
    ///
    /// `brief_word_count` is no longer enforced here — local LLMs are
    /// unreliable at sticking to a per-scene word budget, but they're
    /// usually right about the structural shape of the outline. The
    /// runner now calls `rescale_to_brief_target()` after a successful
    /// parse to bring the per-scene targets into line, which is how a
    /// human editor would treat outline word-count proposals anyway:
    /// suggestive, not binding.
    pub fn validate(&self, target_chapter_count: u32, _brief_word_count: u32) -> Vec<String> {
        let mut errors = Vec::new();

        let actual = self.chapter_count() as u32;
        let low = (target_chapter_count as f64 * 0.8).floor() as u32;
        let high = (target_chapter_count as f64 * 1.2).ceil() as u32;
        if actual < low || actual > high {
            errors.push(format!(
                "chapter count {actual} is outside ±20% of target {target_chapter_count}"
            ));
        }

        // Every scene must have a non-empty synopsis.
        for (pi, part) in self.parts.iter().enumerate() {
            for (ci, ch) in part.chapters.iter().enumerate() {
                for (si, sc) in ch.scenes.iter().enumerate() {
                    if sc.synopsis.trim().is_empty() {
                        errors.push(format!(
                            "part[{pi}].chapters[{ci}].scenes[{si}] has empty synopsis"
                        ));
                    }
                }
            }
        }

        errors
    }

    /// Rescale every scene's `target_word_count` so the total matches
    /// `brief_word_count` (within rounding). No-op when `brief_word_count`
    /// is zero or no scene carries an explicit target.
    ///
    /// Why: 9B-range local models routinely produce outlines whose
    /// per-scene targets sum to 1.5×–3× the brief budget. The structural
    /// outline is still useful — only the arithmetic needs fixing. This
    /// converts the model's RELATIVE weights (Scene A is twice Scene B)
    /// into ABSOLUTE counts that fit the brief.
    pub fn rescale_to_brief_target(&mut self, brief_word_count: u32) {
        if brief_word_count == 0 {
            return;
        }
        let current = self.total_target_words();
        if current == 0 {
            // Nothing to scale from — distribute the brief evenly across scenes.
            let scene_count = self
                .parts
                .iter()
                .flat_map(|p| p.chapters.iter())
                .map(|c| c.scenes.len() as u32)
                .sum::<u32>();
            if scene_count == 0 {
                return;
            }
            let per_scene = brief_word_count / scene_count;
            for part in &mut self.parts {
                for ch in &mut part.chapters {
                    for sc in &mut ch.scenes {
                        sc.target_word_count = Some(per_scene);
                    }
                }
            }
            return;
        }
        let factor = brief_word_count as f64 / current as f64;
        // Skip the rescale if it's already within a tight window — avoids
        // perturbing outlines the model got right.
        if (0.95..=1.05).contains(&factor) {
            return;
        }
        for part in &mut self.parts {
            for ch in &mut part.chapters {
                for sc in &mut ch.scenes {
                    if let Some(w) = sc.target_word_count {
                        let scaled = (w as f64 * factor).round() as u32;
                        sc.target_word_count = Some(scaled.max(1));
                    }
                }
            }
        }
    }
}
