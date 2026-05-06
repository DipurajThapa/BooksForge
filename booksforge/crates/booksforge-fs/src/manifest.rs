//! manifest.toml read/write for bundle bundles.
//!
//! The TOML format is split into `[project]` and `[meta]` sections so the file
//! remains human-readable and git-diffable.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use booksforge_domain::project::{BookMode, ProjectMeta};

use crate::FsError;

/// Root structure that serialises to / deserialises from `manifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub project: ManifestProject,
    pub meta:    ProjectMeta,
}

/// `[project]` section of `manifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProject {
    pub id:               String, // ULID as string
    pub schema_version:   u32,
    pub mode:             BookMode,
    pub template_id:      String,
    pub template_version: String,
    pub ai_enabled:       bool,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
}

impl BundleManifest {
    /// Serialise to a TOML string suitable for writing to `manifest.toml`.
    pub fn to_toml(&self) -> Result<String, FsError> {
        toml::to_string_pretty(self)
            .map_err(|e| FsError::Serialization(format!("manifest serialize error: {e}")))
    }

    /// Parse from the text content of `manifest.toml`.
    pub fn from_toml(text: &str) -> Result<Self, FsError> {
        toml::from_str(text)
            .map_err(|e| FsError::Serialization(format!("manifest parse error: {e}")))
    }

    /// Create a new manifest for a freshly created project.
    pub fn new(
        title: String,
        author: String,
        genre: Option<String>,
        mode: BookMode,
    ) -> Self {
        let now = Utc::now();
        let id = Ulid::new().to_string();
        Self {
            project: ManifestProject {
                id,
                schema_version: 1,
                mode,
                template_id: "default".to_owned(),
                template_version: "1.0.0".to_owned(),
                ai_enabled: false,
                created_at: now,
                updated_at: now,
            },
            meta: ProjectMeta {
                title,
                subtitle: genre,
                authors: vec![author],
                language: "en-US".to_owned(),
                target_words: None,
            },
        }
    }

    /// Read `manifest.toml` from a bundle directory.
    pub async fn read_from_bundle(bundle: &crate::bundle::BundlePath) -> Result<Self, FsError> {
        let path = bundle.manifest();
        let bytes = tokio::fs::read(&path).await.map_err(|e| FsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| FsError::Serialization("manifest.toml is not valid UTF-8".to_owned()))?;
        Self::from_toml(text)
    }
}
