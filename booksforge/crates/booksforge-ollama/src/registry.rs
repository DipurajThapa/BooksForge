//! Compile-time curated model registry loaded from `models.toml`.

use serde::Deserialize;

use crate::OllamaError;

/// Metadata for one curated Ollama model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    /// Ollama pull tag (e.g. `"qwen2.5:7b-instruct-q4_K_M"`).
    pub id:               String,
    pub display_name:     String,
    pub family:           String,
    /// Approximate download size in bytes.
    pub size_bytes:       u64,
    /// Minimum RAM (GB) for comfortable inference.
    pub ram_min_gb:       u32,
    pub context_window:   u32,
    pub chat_format:      String,
    pub recommended_for:  Vec<String>,
    pub strengths:        Vec<String>,
    pub notes:            String,
    /// Modes for which this model is the recommended default (`[]` = not a default).
    #[serde(default)]
    pub default_for_modes: Vec<String>,
    /// Whether this model appears in the UI picker (`false` = internal/smoke only).
    #[serde(default = "default_true")]
    pub official:         bool,
}

fn default_true() -> bool { true }

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
        toml::from_str::<Registry>(MODELS_TOML)
            .expect("models.toml is embedded and must parse correctly")
            .model
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

/// Find the default model for a given book mode, constrained by available RAM.
/// Returns `None` if nothing fits.
pub fn default_model_for_mode(
    mode: &str,
    available_ram_gb: u32,
) -> Option<&'static ModelEntry> {
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
        assert!(!models.is_empty(), "models.toml must have at least one [[model]]");
    }

    #[test]
    fn all_models_have_required_fields() {
        for m in all_models() {
            assert!(!m.id.is_empty(),           "model id must not be empty");
            assert!(!m.display_name.is_empty(), "display_name must not be empty");
            assert!(m.context_window > 0,       "context_window must be positive");
            assert!(m.ram_min_gb > 0,           "ram_min_gb must be positive");
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
}
