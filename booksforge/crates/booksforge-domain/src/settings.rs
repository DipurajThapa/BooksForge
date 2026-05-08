use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One entry in the recent-projects list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    /// Project ULID from `manifest.toml [project] id`.
    pub id:          String,
    /// Absolute path to the `.booksforge/` bundle directory.
    pub path:        String,
    /// Display name (`manifest.toml [meta] title`).
    pub name:        String,
    pub last_opened: DateTime<Utc>,
}

/// Contents of `~/.booksforge/settings.toml`.
///
/// Read and written by `booksforge-fs::settings`.  The file is written
/// atomically (tmp + rename) so a crash never leaves a corrupt settings file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSettings {
    #[serde(default)]
    pub recent_projects: RecentProjectsList,
    #[serde(default)]
    pub ollama: OllamaSettings,
    #[serde(default)]
    pub ui: UiSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentProjectsList {
    /// Most-recently-opened first.  Capped at `MAX_RECENT` entries.
    #[serde(default)]
    pub entries: Vec<RecentProject>,
}

impl RecentProjectsList {
    pub const MAX_RECENT: usize = 10;

    /// Upsert an entry (by path).  Moves it to the front and evicts the oldest
    /// if the list would exceed `MAX_RECENT`.
    pub fn touch(&mut self, entry: RecentProject) {
        self.entries.retain(|e| e.path != entry.path);
        self.entries.insert(0, entry);
        self.entries.truncate(Self::MAX_RECENT);
    }

    /// Remove a project by path (used by the "Remove" action in the picker).
    pub fn remove(&mut self, path: &str) {
        self.entries.retain(|e| e.path != path);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaSettings {
    pub host:          String,
    pub default_model: String,
}

impl OllamaSettings {
    /// BooksForge's recommended default model.  Matches the curated registry
    /// entry whose `default_for_modes` list contains `"fiction"` — kept in
    /// sync with `models.toml`.  Chosen to fit the MVP's 8 GB RAM target.
    pub const DEFAULT_MODEL: &'static str = "qwen2.5:7b-instruct-q4_K_M";
}

impl Default for OllamaSettings {
    fn default() -> Self {
        Self {
            host:          "http://127.0.0.1:11434".to_owned(),
            default_model: Self::DEFAULT_MODEL.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiSettings {
    /// `"light"` | `"dark"` | `"system"`
    #[serde(default = "UiSettings::default_theme")]
    pub theme: String,
}

impl UiSettings {
    fn default_theme() -> String {
        "system".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> RecentProject {
        RecentProject {
            id:          "01TEST".to_owned(),
            path:        path.to_owned(),
            name:        path.to_owned(),
            last_opened: chrono::Utc::now(),
        }
    }

    #[test]
    fn touch_moves_to_front() {
        let mut list = RecentProjectsList::default();
        list.touch(entry("/a"));
        list.touch(entry("/b"));
        list.touch(entry("/a")); // re-open /a
        assert_eq!(list.entries[0].path, "/a");
        assert_eq!(list.entries.len(), 2);
    }

    #[test]
    fn touch_evicts_oldest_at_cap() {
        let mut list = RecentProjectsList::default();
        for i in 0..=RecentProjectsList::MAX_RECENT {
            list.touch(entry(&format!("/{i}")));
        }
        assert_eq!(list.entries.len(), RecentProjectsList::MAX_RECENT);
    }

    #[test]
    fn remove_by_path() {
        let mut list = RecentProjectsList::default();
        list.touch(entry("/a"));
        list.touch(entry("/b"));
        list.remove("/a");
        assert_eq!(list.entries.len(), 1);
        assert_eq!(list.entries[0].path, "/b");
    }
}
