//! Compile-time curated model registry loaded from `models.toml`.

use serde::Deserialize;

use crate::OllamaError;

/// Metadata for one curated Ollama model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    /// Ollama pull tag (e.g. `"qwen2.5:7b-instruct-q4_K_M"`).
    pub id: String,
    pub display_name: String,
    pub family: String,
    /// Approximate download size in bytes.
    pub size_bytes: u64,
    /// Minimum RAM (GB) for comfortable inference.
    pub ram_min_gb: u32,
    pub context_window: u32,
    pub chat_format: String,
    pub recommended_for: Vec<String>,
    pub strengths: Vec<String>,
    pub notes: String,
    /// Modes for which this model is the recommended default (`[]` = not a default).
    #[serde(default)]
    pub default_for_modes: Vec<String>,
    /// Whether this model appears in the UI picker (`false` = internal/smoke only).
    #[serde(default = "default_true")]
    pub official: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct Registry {
    model: Vec<ModelEntry>,
}

static MODELS_TOML: &str = include_str!("../models.toml");

/// Returns the full list of curated models, parsed once at call time.
/// The TOML is embedded at compile time, so this never touches the filesystem.
pub fn all_models() -> &'static [ModelEntry] {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<Vec<ModelEntry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        // The TOML is `include_str!`'d at compile time and validated by
        // a unit test below; if parsing ever fails here, the build is
        // already broken.  expect() is the right call.
        #[allow(clippy::expect_used)]
        let r = toml::from_str::<Registry>(MODELS_TOML)
            .expect("models.toml is embedded and must parse correctly");
        r.model
    })
}

/// Returns only models shown in the UI picker (official = true).
pub fn official_models() -> impl Iterator<Item = &'static ModelEntry> {
    all_models().iter().filter(|m| m.official)
}

/// Returns official models that fit within `available_ram_gb`.
/// Always includes all official models if none fit (to avoid an empty list).
pub fn models_for_ram(available_ram_gb: u32) -> Vec<&'static ModelEntry> {
    let fits: Vec<_> = official_models()
        .filter(|m| m.ram_min_gb <= available_ram_gb)
        .collect();

    if fits.is_empty() {
        // Fallback: show everything so the user isn't stranded.
        official_models().collect()
    } else {
        fits
    }
}

/// Find the best model for the given genre that is locally available.
///
/// Preference order: recommended_for matches genre first, then any available
/// model.  Returns `Err(NoSuitableModel)` if `available` is empty.
pub fn recommend(genre: &str, available: &[String]) -> Result<&'static ModelEntry, OllamaError> {
    let models = all_models();

    // First pass: genre match + available.
    let by_genre = models
        .iter()
        .find(|m| m.recommended_for.iter().any(|g| g == genre) && available.contains(&m.id));

    if let Some(m) = by_genre {
        return Ok(m);
    }

    // Second pass: any available curated model.
    let fallback = models.iter().find(|m| available.contains(&m.id));

    fallback.ok_or_else(|| OllamaError::NoSuitableModel {
        reason: format!(
            "none of the curated models are installed locally (genre: {genre}). \
             Pull one with Ollama Setup."
        ),
    })
}

/// Coarse tier classification for picking a model at agent-dispatch
/// time. Each agent declares the tier it needs (via `AgentSpec` or a
/// per-agent map); the resolver below maps the tier to a concrete
/// installed Ollama tag with a preference order.
///
/// This decouples agent code from specific model tags so we can swap
/// the underlying model (qwen3.5 → qwen4.0, etc.) by editing one list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Fast, small (~7-9B) — outline, intake, vocab, memory curation.
    Light,
    /// Mid-weight (~14-27B) — bibles, scene critic, dev-editor.
    Medium,
    /// Highest-quality prose (30B+ MoE) — scene drafter, final polish.
    Heavy,
}

/// Tier preference ladders. Earlier entries are preferred. The resolver
/// walks the list and returns the first installed tag.
///
/// Why these orderings:
/// - Light: qwen3.5:9b is the default workhorse — well-tested with the
///   schema-tolerant outline runner. Fall back to llama3.1-storm:8b
///   (fast English instruct) and qwen2.5:7b (the historical default
///   from the 8 GB RAM MVP target).
/// - Medium: qwen3.5:27b sits between the light tier and the heavy MoE.
///   Falls through to llama3.1:70b-q2_K if installed, else back to
///   Light.
/// - Heavy: qwen3.6:latest (36B MoE) is the prose-quality champion the
///   multi-chapter run uses. Falls back to qwen3.5:27b if it's not
///   installed (32 GB RAM users), then Light as last resort so a
///   8 GB-RAM box still gets a valid tag.
const LIGHT_LADDER: &[&str] = &[
    "qwen3.5:9b",
    "llama3.1-storm:8b",
    "llama3.1:8b-instruct-q4_K_M",
    "qwen2.5:7b-instruct-q4_K_M",
    "mistral:7b-instruct-q4_K_M",
    "gemma2:9b-instruct-q4_K_M",
];

const MEDIUM_LADDER: &[&str] = &[
    "qwen3.5:27b",
    "llama3.1:70b-instruct-q2_K",
    "qwen3.5:9b",
    "llama3.1-storm:8b",
    "llama3.1:8b-instruct-q4_K_M",
    "qwen2.5:7b-instruct-q4_K_M",
];

const HEAVY_LADDER: &[&str] = &[
    "qwen3.6:latest",
    "qwen3.5:27b",
    "llama3.1:70b-instruct-q2_K",
    "qwen3.5:9b",
    "llama3.1-storm:8b",
    "qwen2.5:7b-instruct-q4_K_M",
];

impl ModelTier {
    fn ladder(self) -> &'static [&'static str] {
        match self {
            ModelTier::Light => LIGHT_LADDER,
            ModelTier::Medium => MEDIUM_LADDER,
            ModelTier::Heavy => HEAVY_LADDER,
        }
    }
}

/// Resolve a tier to a concrete Ollama tag, given the list of currently
/// installed model tags. Walks the tier's ladder in order and returns
/// the first match. Falls back to the first installed model when no
/// ladder entry matches — that's better than failing the run.
///
/// Returns an owned `String` so the fallback path (echo back the user's
/// own first installed tag) doesn't run into static-lifetime issues.
///
/// `Err(NoSuitableModel)` only when `available` is empty (i.e. the user
/// has zero models pulled), with a message the UI can surface to push
/// them to the OllamaWizard.
pub fn resolve_tier(tier: ModelTier, available: &[String]) -> Result<String, OllamaError> {
    if available.is_empty() {
        return Err(OllamaError::NoSuitableModel {
            reason:
                "no Ollama models are installed locally — open AI Setup to pull qwen3.5:9b first."
                    .to_owned(),
        });
    }
    for tag in tier.ladder() {
        if available.iter().any(|a| a == *tag) {
            return Ok((*tag).to_owned());
        }
    }
    // Last-resort fallback — first installed model so the run goes
    // ahead. The desktop log line at dispatch time records what got
    // picked, so a wrong answer is at least diagnosable.
    Ok(available
        .first()
        .cloned()
        // SAFETY: we early-returned on empty above; this branch is dead.
        .unwrap_or_else(|| "qwen3.5:9b".to_owned()))
}

/// Recommended tier for a given agent_id. Centralised here so the
/// per-agent dispatch sites in `apps/desktop/src/commands/agents.rs`
/// don't have to hard-code model tags one at a time.
///
/// Mapping rationale follows `outputs/AGENTS.md` and the per-agent
/// `AgentSpec.model_preference`:
///   - structural / metadata agents (intake, outline, vocab,
///     memory-curator, proposal-validator) → Light
///   - bibles + critic + dev-editor → Medium
///   - prose drafters + final polish → Heavy
pub fn recommended_tier_for_agent(agent_id: &str) -> ModelTier {
    match agent_id {
        // Structural / metadata — fast iteration matters more than
        // last-mile prose nuance.
        "intake" | "outline-architect" | "memory-curator" | "vocab-dictionary"
        | "proposal-validator" | "scene-critic" | "scene-planner" | "humanization" | "copyedit" => {
            ModelTier::Light
        }

        // Bibles + dev-editor — reasoning-heavy structured outputs that
        // benefit from the mid tier without paying for the heavy MoE.
        "character-bible"
        | "world-bible"
        | "character-bible-card"
        | "dev-editor"
        | "developmental-review"
        | "continuity"
        | "entity-bible" => ModelTier::Medium,

        // Prose generators + final polish stack — quality dominates;
        // wall-clock budget is acceptable.
        "scene-drafter-fic"
        | "chapter-drafter"
        | "chapter-drafter-nf"
        | "voice-polish"
        | "metaphor-polish"
        | "dialogue-polish"
        | "tension-polish"
        | "final-polish"
        | "final-polish-merge"
        | "final-review-editor" => ModelTier::Heavy,

        // Unknown agent — be conservative and use Light.
        _ => ModelTier::Light,
    }
}

/// Find the default model for a given book mode, constrained by available RAM.
/// Returns `None` if nothing fits.
pub fn default_model_for_mode(mode: &str, available_ram_gb: u32) -> Option<&'static ModelEntry> {
    all_models()
        .iter()
        .find(|m| {
            m.default_for_modes.iter().any(|dm| dm == mode)
                && m.ram_min_gb <= available_ram_gb
                && m.official
        })
        .or_else(|| {
            // Fallback: any official model that fits RAM.
            all_models()
                .iter()
                .find(|m| m.official && m.ram_min_gb <= available_ram_gb)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_toml_parses_and_is_non_empty() {
        let models = all_models();
        assert!(
            !models.is_empty(),
            "models.toml must have at least one [[model]]"
        );
    }

    #[test]
    fn all_models_have_required_fields() {
        for m in all_models() {
            assert!(!m.id.is_empty(), "model id must not be empty");
            assert!(!m.display_name.is_empty(), "display_name must not be empty");
            assert!(m.context_window > 0, "context_window must be positive");
            assert!(m.ram_min_gb > 0, "ram_min_gb must be positive");
        }
    }

    #[test]
    fn tinyllama_is_not_official() {
        let tl = all_models().iter().find(|m| m.family == "tinyllama");
        assert!(tl.is_some(), "tinyllama must be in registry");
        assert!(!tl.unwrap().official, "tinyllama must not be official");
    }

    #[test]
    fn official_models_all_have_display_names() {
        let official: Vec<_> = official_models().collect();
        assert!(!official.is_empty());
        for m in &official {
            assert!(!m.display_name.is_empty());
        }
    }

    #[test]
    fn models_for_ram_always_returns_something() {
        // Even with 1 GB it should fall back to all official models.
        assert!(!models_for_ram(1).is_empty());
        // With 16 GB several models should fit.
        let big = models_for_ram(16);
        assert!(big.len() >= 3);
    }

    #[test]
    fn recommend_returns_error_when_nothing_available() {
        let err = recommend("fiction", &[]).unwrap_err();
        assert!(matches!(err, OllamaError::NoSuitableModel { .. }));
    }

    #[test]
    fn recommend_returns_genre_preferred_model() {
        let available = vec![
            "llama3.1:8b-instruct-q4_K_M".into(),
            "qwen2.5:7b-instruct-q4_K_M".into(),
        ];
        let m = recommend("fiction", &available).unwrap();
        assert!(m.recommended_for.contains(&"fiction".to_owned()));
    }

    #[test]
    fn default_model_for_fiction_exists() {
        // At least one model should be the default for fiction with 16 GB.
        let m = default_model_for_mode("fiction", 16);
        assert!(m.is_some());
    }

    #[test]
    fn resolve_tier_errs_when_nothing_installed() {
        let err = resolve_tier(ModelTier::Light, &[]).unwrap_err();
        assert!(matches!(err, OllamaError::NoSuitableModel { .. }));
    }

    #[test]
    fn resolve_light_prefers_qwen35_9b() {
        let installed = vec![
            "qwen3.5:9b".into(),
            "qwen3.6:latest".into(),
            "qwen3.5:27b".into(),
        ];
        let m = resolve_tier(ModelTier::Light, &installed).unwrap();
        assert_eq!(m, "qwen3.5:9b");
    }

    #[test]
    fn resolve_heavy_prefers_qwen36_when_installed() {
        let installed = vec![
            "qwen3.5:9b".into(),
            "qwen3.6:latest".into(),
            "qwen3.5:27b".into(),
        ];
        let m = resolve_tier(ModelTier::Heavy, &installed).unwrap();
        assert_eq!(m, "qwen3.6:latest");
    }

    #[test]
    fn resolve_heavy_falls_back_to_27b_when_36_missing() {
        let installed = vec!["qwen3.5:9b".into(), "qwen3.5:27b".into()];
        let m = resolve_tier(ModelTier::Heavy, &installed).unwrap();
        assert_eq!(m, "qwen3.5:27b");
    }

    #[test]
    fn resolve_falls_back_to_first_installed_when_no_ladder_match() {
        let installed = vec!["some-exotic-model:latest".into()];
        let m = resolve_tier(ModelTier::Light, &installed).unwrap();
        assert_eq!(m, "some-exotic-model:latest");
    }

    #[test]
    fn outline_architect_maps_to_light() {
        assert_eq!(
            recommended_tier_for_agent("outline-architect"),
            ModelTier::Light
        );
    }

    #[test]
    fn scene_drafter_fic_maps_to_heavy() {
        assert_eq!(
            recommended_tier_for_agent("scene-drafter-fic"),
            ModelTier::Heavy
        );
    }

    #[test]
    fn world_bible_maps_to_medium() {
        assert_eq!(recommended_tier_for_agent("world-bible"), ModelTier::Medium);
    }

    #[test]
    fn unknown_agent_falls_back_to_light() {
        assert_eq!(
            recommended_tier_for_agent("a-completely-new-agent"),
            ModelTier::Light
        );
    }
}
