use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The application version returned by the `app_version` Tauri command.
///
/// Corresponds to the semver fields in `Cargo.toml [workspace.package]`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/shared-types/src/bindings/")]
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// Pre-release tag (e.g. `"alpha.1"`).  Always serialised — TS sees
    /// `string | null`.  Earlier versions used `skip_serializing_if`
    /// which ts-rs 10 can't parse; emitting `null` is just as clear and
    /// keeps the bindings deterministic.
    pub pre:   Option<String>,
}

impl AppVersion {
    pub const CURRENT: Self = Self {
        major: 0,
        minor: 0,
        patch: 1,
        pre:   None,
    };
}
