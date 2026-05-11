//! Read/write `~/.booksforge/settings.toml` atomically.

use std::path::PathBuf;

use booksforge_domain::settings::UserSettings;

use crate::{atomic::atomic_write, FsError};

/// Return the canonical settings-file path: `~/.booksforge/settings.toml`.
///
/// Falls back to the current working directory if the home dir cannot be
/// determined (sandboxed / unusual env).
pub fn settings_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join(".booksforge")
        .join("settings.toml")
}

/// Load `UserSettings` from `~/.booksforge/settings.toml`.
///
/// Returns `Ok(Default::default())` if the file does not exist yet —
/// first-launch scenario.
pub async fn load_settings() -> Result<UserSettings, FsError> {
    let path = settings_path();
    if !path.exists() {
        return Ok(UserSettings::default());
    }
    let bytes = tokio::fs::read(&path).await.map_err(|e| FsError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| FsError::Serialization("settings.toml is not valid UTF-8".to_owned()))?;
    toml::from_str(text)
        .map_err(|e| FsError::Serialization(format!("settings.toml parse error: {e}")))
}

/// Persist `UserSettings` to `~/.booksforge/settings.toml` atomically.
pub async fn save_settings(settings: &UserSettings) -> Result<(), FsError> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| FsError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
    }
    let text = toml::to_string_pretty(settings)
        .map_err(|e| FsError::Serialization(format!("settings serialize error: {e}")))?;
    atomic_write(&path, text.as_bytes()).await
}

// ── Platform home-dir helper ──────────────────────────────────────────────────

fn home_dir() -> Option<PathBuf> {
    // std::env::home_dir is deprecated but still correct on our target platforms.
    // The deprecation is about HOMEDRIVE/HOMEPATH quirks on Windows XP which don't
    // apply to Windows 10+.
    #[allow(deprecated)]
    std::env::home_dir()
}
