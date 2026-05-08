//! Originality provider runtime — consent storage + LocalOnly impl.
//!
//! Companion to `booksforge-domain::originality_provider` (which holds
//! the pure types).  This module owns:
//!
//!   - **Consent persistence.**  `load_consent` / `save_consent` /
//!     `clear_consent` read and write a single `MemoryEntry` in
//!     `MemoryScope::Style` under key `originality_provider_consent`.
//!     Going through memory means consent changes ride the existing
//!     audit trail (`last_writer` stamping, snapshotability).
//!
//!   - **LocalOnly provider.**  Wraps `booksforge-validator::originality`
//!     so the application can ask "scan this prose against this
//!     corpus" without knowing whether the active provider is local or
//!     remote.
//!
//! No remote provider ships in MVP.  See BACKLOG §E0d.11 for the
//! consent-gated rollout plan.

use std::sync::Arc;

use booksforge_domain::{
    MemoryEntry, MemoryScope, OriginalityCheckResult, OriginalityConsent, OriginalityProviderId,
};
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use ulid::Ulid;

const CONSENT_KEY: &str = "originality_provider_consent";

/// Load the project's persisted consent, falling back to `LocalOnly`
/// when no row exists.  Never errors — corrupt rows fall through to
/// the safe default.
pub async fn load_consent(storage: &Arc<SqliteStorage>) -> OriginalityConsent {
    let entry = storage
        .memory_get(MemoryScope::Style, CONSENT_KEY)
        .await
        .ok()
        .flatten();
    match entry {
        Some(e) => serde_json::from_value(e.value_json).unwrap_or_default(),
        None    => OriginalityConsent::default(),
    }
}

/// Persist a consent record.  `agent_id` is stamped on the memory row
/// for audit (typically `"user"` for an interactive consent change).
pub async fn save_consent(
    storage:  &Arc<SqliteStorage>,
    consent:  &OriginalityConsent,
    agent_id: &str,
) -> Result<(), booksforge_storage::StorageError> {
    let now = Utc::now();
    let entry = MemoryEntry {
        id:         Ulid::new(),
        scope:      MemoryScope::Style,
        key:        CONSENT_KEY.to_owned(),
        value_json: serde_json::to_value(consent)
            .unwrap_or_else(|_| serde_json::json!({})),
        agent_id:   agent_id.to_owned(),
        created_at: now,
        updated_at: now,
    };
    storage.memory_upsert(&entry).await
}

/// Reset to the default `LocalOnly` consent.  Equivalent to "revoke
/// any opt-in to a remote provider".
pub async fn clear_consent(
    storage:  &Arc<SqliteStorage>,
    agent_id: &str,
) -> Result<(), booksforge_storage::StorageError> {
    save_consent(storage, &OriginalityConsent::default(), agent_id).await
}

/// Return the active provider for a project — exactly the persisted
/// consent's `provider`.  Centralised so application code never picks a
/// remote provider by accident.
pub async fn active_provider(storage: &Arc<SqliteStorage>) -> OriginalityProviderId {
    load_consent(storage).await.provider
}

/// Local-only originality scan over `output` against `source` and/or
/// `prior_corpus`.  Provider-agnostic envelope around the existing
/// `booksforge-validator::originality` n-gram detector — pure logic,
/// no I/O.
///
/// `min_words` defaults to `booksforge_validator::originality::DEFAULT_MIN_WORDS`
/// (12) when the caller passes `None`.
pub fn scan_local(
    output:       &str,
    source:       Option<&str>,
    prior_corpus: Option<&str>,
    min_words:    Option<usize>,
) -> OriginalityCheckResult {
    let mw = min_words.unwrap_or(booksforge_validator::originality::DEFAULT_MIN_WORDS);
    let mut hits = Vec::new();
    if let Some(src) = source {
        hits.extend(booksforge_validator::detect_verbatim_overlap(output, src, mw));
    }
    if let Some(prior) = prior_corpus {
        hits.extend(booksforge_validator::detect_self_plagiarism(output, prior, mw));
    }
    let longest = hits.iter().map(|h| h.words).max().unwrap_or(0);
    let samples = hits.iter().take(10).map(|h| h.quote.clone()).collect();
    OriginalityCheckResult {
        provider:          OriginalityProviderId::LocalOnly,
        hit_count:         hits.len() as u32,
        longest_run_words: longest,
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_storage::{open_pool, run_migrations};

    async fn fresh_storage() -> (Arc<SqliteStorage>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = open_pool(&dir.path().join("op.db")).await.unwrap();
        run_migrations(&pool).await.unwrap();
        (Arc::new(SqliteStorage::new(pool)), dir)
    }

    #[tokio::test]
    async fn default_consent_is_local_only_when_no_row() {
        let (storage, _d) = fresh_storage().await;
        let c = load_consent(&storage).await;
        assert_eq!(c.provider, OriginalityProviderId::LocalOnly);
    }

    #[tokio::test]
    async fn active_provider_reflects_saved_consent() {
        let (storage, _d) = fresh_storage().await;
        // Default: local only.
        assert_eq!(active_provider(&storage).await, OriginalityProviderId::LocalOnly);
        // Save a hypothetical remote consent — active_provider must reflect it.
        save_consent(&storage, &OriginalityConsent {
            provider:    OriginalityProviderId::Copyleaks,
            accepted_at: "2026-05-07T12:00:00Z".into(),
            note:        "test".into(),
        }, "user").await.unwrap();
        assert_eq!(active_provider(&storage).await, OriginalityProviderId::Copyleaks);
        // Clearing returns us to local.
        clear_consent(&storage, "user").await.unwrap();
        assert_eq!(active_provider(&storage).await, OriginalityProviderId::LocalOnly);
    }

    #[test]
    fn scan_local_flags_a_long_verbatim_run() {
        let source = "She walked into the dimly lit corridor and the floorboards groaned beneath her weight.";
        let output = "She walked into the dimly lit corridor and the floorboards groaned beneath her weight, again.";
        let r = scan_local(output, Some(source), None, None);
        assert_eq!(r.provider, OriginalityProviderId::LocalOnly);
        assert!(r.hit_count >= 1);
        assert!(r.longest_run_words >= 12);
    }

    #[test]
    fn scan_local_returns_no_hits_for_clean_prose() {
        let source = "Some unrelated text.";
        let output = "Completely different prose with no overlap.";
        let r = scan_local(output, Some(source), None, None);
        assert_eq!(r.hit_count, 0);
        assert_eq!(r.longest_run_words, 0);
    }
}
