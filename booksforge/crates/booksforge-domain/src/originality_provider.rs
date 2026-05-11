//! Originality / plagiarism provider abstraction (BACKLOG §E0d.11).
//!
//! BooksForge ships a local-only originality detector by default
//! (`booksforge-validator::originality`).  External plagiarism APIs
//! (Copyleaks, Plagscan, Turnitin Originality, etc.) cannot be invoked
//! without explicit one-time-per-project consent — this module defines
//! the contract that any provider must honour.
//!
//! These are pure types — no I/O.  Implementations live in
//! `booksforge-orchestrator::originality_provider`.

use serde::{Deserialize, Serialize};

/// Identifies a configured provider.  Stored as a string in the
/// `Style`-scope memory entry `originality_provider_consent` so future
/// providers can register themselves without a schema migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalityProviderId {
    /// On-device n-gram detector — never makes a network call.
    /// Default and only provider that ships in MVP.
    LocalOnly,
    /// Reserved — Copyleaks API.  Requires explicit consent + API key.
    Copyleaks,
    /// Reserved — Plagscan API.  Requires explicit consent + API key.
    Plagscan,
    /// Reserved — Turnitin Originality API.
    Turnitin,
}

impl OriginalityProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnly => "local_only",
            Self::Copyleaks => "copyleaks",
            Self::Plagscan => "plagscan",
            Self::Turnitin => "turnitin",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "local_only" => Some(Self::LocalOnly),
            "copyleaks" => Some(Self::Copyleaks),
            "plagscan" => Some(Self::Plagscan),
            "turnitin" => Some(Self::Turnitin),
            _ => None,
        }
    }

    /// True for providers that send manuscript content off-device.
    /// `LocalOnly` is the only one for which this is false.
    pub fn sends_content_offdevice(self) -> bool {
        !matches!(self, Self::LocalOnly)
    }
}

/// Persisted consent record.  Lives as a `MemoryEntry` in
/// `MemoryScope::Style` under key `originality_provider_consent`, so
/// every consent change rides the audit trail like any other memory
/// write (including `last_writer` stamping).
///
/// The presence of a row with `provider != LocalOnly` is the *only*
/// authorisation under which an off-device provider may run.  A
/// privacy-invariant test asserts this contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginalityConsent {
    pub provider: OriginalityProviderId,
    /// ISO-8601 timestamp when the user accepted the consent dialog.
    pub accepted_at: String,
    /// Free-form note the user can add (e.g. "review window — Q4 only").
    /// Empty for the default LocalOnly record.
    pub note: String,
}

impl Default for OriginalityConsent {
    fn default() -> Self {
        Self {
            provider: OriginalityProviderId::LocalOnly,
            accepted_at: String::new(),
            note: String::new(),
        }
    }
}

/// Result of a single originality check, as surfaced to the UI.
/// Provider-agnostic so the same panel can render hits from the local
/// detector or any future remote service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginalityCheckResult {
    pub provider: OriginalityProviderId,
    /// Total number of overlap hits.
    pub hit_count: u32,
    /// Longest verbatim run, in words.
    pub longest_run_words: u32,
    /// Provider-specific quote / metadata strings.  For LocalOnly these
    /// are the same `OverlapHit.quote`s the in-process detector returns.
    pub samples: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_only_is_the_only_on_device_provider() {
        assert!(!OriginalityProviderId::LocalOnly.sends_content_offdevice());
        assert!(OriginalityProviderId::Copyleaks.sends_content_offdevice());
        assert!(OriginalityProviderId::Plagscan.sends_content_offdevice());
        assert!(OriginalityProviderId::Turnitin.sends_content_offdevice());
    }

    #[test]
    fn provider_id_round_trips_through_string() {
        for p in [
            OriginalityProviderId::LocalOnly,
            OriginalityProviderId::Copyleaks,
            OriginalityProviderId::Plagscan,
            OriginalityProviderId::Turnitin,
        ] {
            assert_eq!(OriginalityProviderId::from_str(p.as_str()), Some(p));
        }
    }
}
