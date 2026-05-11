//! IPC types for the quality-stack Tauri commands (BACKLOG §A16 / Phase 3).
//!
//! Wraps inputs/results for the voice / anti-ai-tells / genre-packs
//! crates' Tauri-callable surfaces. The result types embed the
//! domain-pure structs from those crates directly (re-export via
//! `lib.rs::pub use`).

use booksforge_voice::{StylometricDistance, VoiceProfile};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Voice fingerprint ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceFingerprintInput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceFingerprintResult {
    pub profile: VoiceProfile,
    pub constraints_block: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceAnchorSetInput {
    pub comp_samples: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceAnchorSetResult {
    pub profile: VoiceProfile,
    pub constraints_block: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoiceAnchorGetResult {
    pub profile: Option<VoiceProfile>,
    pub constraints_block: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StylometricDistanceInput {
    pub anchor_text: String,
    pub measured_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StylometricDistanceResult {
    pub distance: StylometricDistance,
}

// ── Anti-AI-tells ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TellsScanInput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TellsScanResult {
    pub report: booksforge_anti_ai_tells::TellsReport,
    pub hits: Vec<booksforge_anti_ai_tells::TellHit>,
    pub revision_prompt: String,
}

// ── Genre packs ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GenrePackInput {
    pub kind: String,
}
