//! Quality-stack Tauri commands (BACKLOG §A16 / Phase 3).
//!
//! Surfaces three pure-logic crates to the UI:
//!   - `booksforge-voice`         → voice fingerprinting + stylometric distance
//!   - `booksforge-anti-ai-tells`  → AI-prose density measurement + spans
//!   - `booksforge-genre-packs`   → per-genre prompts + rubric weights
//!
//! All inputs/results live in `booksforge-ipc::quality` so ts-rs bindings
//! land in `packages/shared-types/src/bindings/` alongside the rest.

use booksforge_anti_ai_tells::{
    find_tells, revision_prompt as tells_revision_prompt, tells_per_1000_words,
};
use booksforge_genre_packs::{pack_for, BookKind, GenrePack};
use booksforge_ipc::{
    BooksForgeError, GenrePackInput, StylometricDistanceInput, StylometricDistanceResult,
    TellsScanInput, TellsScanResult, VoiceAnchorGetResult, VoiceAnchorSetInput,
    VoiceAnchorSetResult, VoiceFingerprintInput, VoiceFingerprintResult,
};
use booksforge_voice::{fingerprint, stylometric_distance};
use tauri::State;
use ulid::Ulid;

use crate::state::AppState;

// ── Voice fingerprint ────────────────────────────────────────────────────────

/// Compute a numeric voice profile from raw prose. No project state
/// required; useful for ad-hoc analysis from the UI's Voice Anchor card.
#[tauri::command]
pub async fn voice_fingerprint(
    input: VoiceFingerprintInput,
) -> Result<VoiceFingerprintResult, BooksForgeError> {
    let profile = fingerprint(&input.text);
    let constraints_block = profile.constraints_block("comp samples");
    Ok(VoiceFingerprintResult {
        profile,
        constraints_block,
    })
}

/// Set the project's voice anchor — the comp-sample fingerprint that
/// every drafter and polish run consumes as the voice constraint.
/// Persisted to book-scope memory under `voice:anchor`.
#[tauri::command]
pub async fn voice_anchor_set(
    input: VoiceAnchorSetInput,
    state: State<'_, AppState>,
) -> Result<VoiceAnchorSetResult, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;
    let profile = fingerprint(&input.comp_samples);
    let constraints_block = profile.constraints_block("project voice anchor");
    let value_json = serde_json::json!({
        "profile": profile,
        "constraints_block": constraints_block,
        "comp_sample_word_count": profile.word_count,
    });
    let now = chrono::Utc::now();
    let entry = booksforge_domain::MemoryEntry {
        id: Ulid::new(),
        scope: booksforge_domain::MemoryScope::Book,
        key: "voice:anchor".to_owned(),
        value_json,
        agent_id: "voice-anchor".to_owned(),
        created_at: now,
        updated_at: now,
    };
    use booksforge_storage::StorageRepository as _;
    project
        .storage
        .memory_upsert(&entry)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(VoiceAnchorSetResult {
        profile,
        constraints_block,
    })
}

/// Read the project's stored voice anchor (or empty if not set).
#[tauri::command]
pub async fn voice_anchor_get(
    state: State<'_, AppState>,
) -> Result<VoiceAnchorGetResult, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;
    use booksforge_storage::StorageRepository as _;
    let entry = project
        .storage
        .memory_get(booksforge_domain::MemoryScope::Book, "voice:anchor")
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    if let Some(m) = entry {
        let profile = serde_json::from_value(m.value_json["profile"].clone()).ok();
        let constraints_block = m.value_json["constraints_block"]
            .as_str()
            .map(|s| s.to_owned());
        Ok(VoiceAnchorGetResult {
            profile,
            constraints_block,
        })
    } else {
        Ok(VoiceAnchorGetResult {
            profile: None,
            constraints_block: None,
        })
    }
}

/// Compute stylometric distance between two prose samples (0-10, 10 = identical).
#[tauri::command]
pub async fn stylometric_distance_compute(
    input: StylometricDistanceInput,
) -> Result<StylometricDistanceResult, BooksForgeError> {
    let a = fingerprint(&input.anchor_text);
    let b = fingerprint(&input.measured_text);
    Ok(StylometricDistanceResult {
        distance: stylometric_distance(&a, &b),
    })
}

// ── Anti-AI-tells ────────────────────────────────────────────────────────────

/// Scan prose for AI-prose fingerprints. Returns the per-span hits +
/// a density report + a ready-to-use revision-prompt fragment.
#[tauri::command]
pub async fn tells_scan(input: TellsScanInput) -> Result<TellsScanResult, BooksForgeError> {
    let report = tells_per_1000_words(&input.text);
    let hits = find_tells(&input.text);
    let revision_prompt = tells_revision_prompt(&input.text, 30);
    Ok(TellsScanResult {
        report,
        hits,
        revision_prompt,
    })
}

// ── Genre packs ──────────────────────────────────────────────────────────────

/// Return the full pack (system prompt, lens, critic axes, polish stack
/// order, rubric weights, hard rules) for a given book kind.
#[tauri::command]
pub async fn genre_pack_get(input: GenrePackInput) -> Result<GenrePack, BooksForgeError> {
    let kind = BookKind::from_str(&input.kind)
        .ok_or_else(|| BooksForgeError::validation(format!("unknown book kind: {}", input.kind)))?;
    Ok(pack_for(kind))
}
