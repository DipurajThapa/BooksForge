//! IPC types for the Stage 6 cover & boilerplate flow.
//!
//! These wrap `booksforge_domain::CoverSet` and
//! `booksforge_domain::BoilerplatePage` with thin DTOs so the
//! TypeScript frontend gets exact type bindings via ts-rs.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input to `cover_import`. The frontend passes an absolute filesystem
/// path the user picked from a file dialog plus which slot to import
/// it into ("front" | "back" | "spine"). The command copies the file
/// into `<bundle>/assets/cover-<slot>.<ext>` and persists the
/// resulting `CoverAsset` into the project's `book:cover_set` memory
/// entry.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverImportInput {
    /// Absolute path to the source image on the user's filesystem.
    pub source_path: String,
    /// Which cover slot: "front" | "back" | "spine".
    pub slot: String,
}

/// Result of `cover_import` — echoes back the full `CoverSet` after
/// the import so the UI can re-render in one round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverSetDto {
    /// Empty slot is `null` JSON, present slot is the asset.
    pub front: Option<CoverAssetDto>,
    pub back: Option<CoverAssetDto>,
    pub spine: Option<CoverAssetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverAssetDto {
    pub bundle_path: String,
    pub original_filename: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    /// RFC-3339 UTC timestamp.
    pub imported_at: String,
}

impl From<&booksforge_domain::CoverAsset> for CoverAssetDto {
    fn from(a: &booksforge_domain::CoverAsset) -> Self {
        Self {
            bundle_path: a.bundle_path.clone(),
            original_filename: a.original_filename.clone(),
            size_bytes: a.size_bytes,
            mime_type: a.mime_type.clone(),
            width_px: a.width_px,
            height_px: a.height_px,
            imported_at: a.imported_at.to_rfc3339(),
        }
    }
}

impl From<&booksforge_domain::CoverSet> for CoverSetDto {
    fn from(c: &booksforge_domain::CoverSet) -> Self {
        Self {
            front: c.front.as_ref().map(CoverAssetDto::from),
            back: c.back.as_ref().map(CoverAssetDto::from),
            spine: c.spine.as_ref().map(CoverAssetDto::from),
        }
    }
}

/// Input to `cover_remove`. Clears a single slot.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverRemoveInput {
    pub slot: String,
}

/// One boilerplate page as passed across IPC. Mirrors
/// `booksforge_domain::BoilerplatePage` field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BoilerplatePageDto {
    pub id: String,
    /// `"title_page" | "copyright" | "dedication" | "epigraph"
    /// | "foreword" | "preface" | "acknowledgments" | "about_author"
    /// | "also_by" | "back_cover_blurb" | "other"`.
    pub kind: String,
    pub title: String,
    pub body_md: String,
    pub order: u32,
    pub include_in_export: bool,
}

impl From<&booksforge_domain::BoilerplatePage> for BoilerplatePageDto {
    fn from(p: &booksforge_domain::BoilerplatePage) -> Self {
        // serde_json knows the snake_case rename for the enum; route
        // through it so we don't drift if a new variant lands.
        let kind = serde_json::to_value(p.kind)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_owned()))
            .unwrap_or_else(|| "other".to_owned());
        Self {
            id: p.id.clone(),
            kind,
            title: p.title.clone(),
            body_md: p.body_md.clone(),
            order: p.order,
            include_in_export: p.include_in_export,
        }
    }
}

impl BoilerplatePageDto {
    pub fn to_domain(&self) -> Result<booksforge_domain::BoilerplatePage, String> {
        let kind: booksforge_domain::BoilerplateKind =
            serde_json::from_value(serde_json::Value::String(self.kind.clone()))
                .map_err(|e| format!("unknown boilerplate kind {:?}: {e}", self.kind))?;
        Ok(booksforge_domain::BoilerplatePage {
            id: self.id.clone(),
            kind,
            title: self.title.clone(),
            body_md: self.body_md.clone(),
            order: self.order,
            include_in_export: self.include_in_export,
        })
    }
}

/// Input to `boilerplate_save`. Whole-list upsert — the saved list
/// replaces whatever is in `book:boilerplate_pages` memory.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BoilerplateSaveInput {
    pub pages: Vec<BoilerplatePageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BoilerplateSaveResult {
    pub saved_count: u32,
}
