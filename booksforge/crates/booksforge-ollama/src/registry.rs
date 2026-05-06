//! Compile-time curated model registry loaded from `models.toml`.

use serde::Deserialize;

use crate::OllamaError;

/// Metadata for one curated Ollama model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    /// Ollama pull tag (e.g. `"qwen2.5:7b-instruct-q4_K_M"`).
    pub id:              String,
    pub display_name:    String,
    pub family:          String,
    /// Approximate download size in bytes.
    pub size_bytes:      u64,
    /// Minimum RAM (GB) for comfortable inference.
    pub ram_min_gb:      u32,
    pub context_window:  u32,
    pub chat_format:     String,
    pub recommended_for: Vec<String>,
    pub strengths:       Vec<String>,
    pub notes:           String,
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
        toml::from_str::<Registry>(MODELS_TOML)
            .expect("models.toml is embedded and must parse correctly")
            .model
    })
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
    fn recommend_returns_error_when_nothing_available() {
        let err = recommend("fiction", &[]).unwrap_err();
        assert!(matches!(err, OllamaError::NoSuitableModel { .. }));
    }

    #[test]
    fn recommend_returns_genre_preferred_model() {
        let available = vec!["llama3.1:8b-instruct-q4_K_M".into(), "qwen2.5:7b-instruct-q4_K_M".into()];
        let m = recommend("fiction", &available).unwrap();
        // Both are recommended for fiction; the first curated match wins.
        assert!(m.recommended_for.contains(&"fiction".to_owned()));
    }
}
