//! IPC types for the Prepare-for-Publishing single-action command
//! (Phase 7 of `PRODUCT_ROADMAP_E2E.md`, closes UX recommendation R4).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Default, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PublishingMetadata {
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    /// 7+ keywords for KDP / Google Play / Apple Books search.
    pub keywords: Option<Vec<String>>,
    /// BISAC subject codes (e.g. "FIC009000 FICTION / Fantasy / General").
    pub bisac_codes: Option<Vec<String>>,
    pub age_range: Option<String>,
    pub language: Option<String>,
    pub isbn: Option<String>,
    pub price_usd: Option<String>,
    pub publication_date: Option<String>,
    pub publisher: Option<String>,
    pub rights_statement: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrepareForPublishingInput {
    /// Optional list of platforms to bundle. Empty = all three.
    /// Each entry is `"kdp" | "google_play" | "apple_books"`.
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub metadata_overrides: PublishingMetadata,
}

/// One readiness checklist item. UI renders a colour-coded badge.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReadinessItem {
    pub id: String,
    pub label: String,
    /// `"PASS" | "WARN" | "FAIL" | "HUMAN_REQUIRED"`.
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlatformReadiness {
    pub platform: String,
    pub output_dir: String,
    pub items: Vec<ReadinessItem>,
    pub uploadable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrepareForPublishingResult {
    pub project_id: String,
    pub platforms: Vec<PlatformReadiness>,
    pub elapsed_s: f32,
}
