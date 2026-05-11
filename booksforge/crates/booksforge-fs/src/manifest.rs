//! manifest.toml read/write for bundle bundles.
//!
//! The TOML format is split into `[project]` and `[meta]` sections so the file
//! remains human-readable and git-diffable.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use booksforge_domain::project::{BookMode, ProjectMeta};
use booksforge_domain::BookKind;

use crate::FsError;

/// Root structure that serialises to / deserialises from `manifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub project: ManifestProject,
    pub meta: ProjectMeta,
}

/// `[project]` section of `manifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProject {
    pub id: String, // ULID as string
    pub schema_version: u32,
    pub mode: BookMode,
    /// Finer-grained classification (Phase 4 of PRODUCT_ROADMAP_E2E.md).
    /// Optional for backwards compatibility — projects created before
    /// this field existed deserialise with `book_kind = None`. The
    /// desktop app surfaces an onboarding overlay to backfill in that
    /// case. New projects always set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_kind: Option<BookKind>,
    pub template_id: String,
    pub template_version: String,
    pub ai_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

    /// Create a new manifest for a freshly created project. Per Phase 4
    /// of PRODUCT_ROADMAP_E2E.md, the wizard supplies `book_kind`
    /// upfront; pass `None` only when migrating older projects (the
    /// onboarding overlay then asks the user to pick).
    pub fn new(
        title: String,
        author: String,
        genre: Option<String>,
        mode: BookMode,
        book_kind: Option<BookKind>,
    ) -> Self {
        let now = Utc::now();
        let id = Ulid::new().to_string();
        Self {
            project: ManifestProject {
                id,
                schema_version: 1,
                mode,
                book_kind,
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

    /// Set or change the project's `book_kind` field. Returns the new
    /// manifest (callers persist via `to_toml` + atomic write). Use
    /// from the SettingsPanel and the migration onboarding overlay.
    pub fn set_book_kind(&mut self, book_kind: BookKind) {
        self.project.book_kind = Some(book_kind);
        self.project.updated_at = Utc::now();
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
