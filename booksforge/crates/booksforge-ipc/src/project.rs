//! IPC types for project lifecycle commands.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Inputs ───────────────────────────────────────────────────────────────────

/// Input for `project_create`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectInput {
    /// Absolute path where the `*.booksforge/` bundle should be created.
    pub bundle_path: String,
    pub title: String,
    pub author: String,
    /// Optional `genre` string (free-form tag for UI grouping).
    pub genre: Option<String>,
    /// Project classification — drives the workflow router (per-genre
    /// prompts, polish-stack ordering, rubric weights). Set by the
    /// NewProjectWizard's first step (Phase 5 of PRODUCT_ROADMAP_E2E.md).
    /// Optional for backwards compatibility with the prior wizard;
    /// `None` triggers the post-create onboarding overlay.
    #[serde(default)]
    pub book_kind: Option<String>,
}

/// Input for `project_open`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenProjectInput {
    /// Absolute path to an existing `*.booksforge/` bundle.
    pub bundle_path: String,
}

// ── Outputs ──────────────────────────────────────────────────────────────────

/// Returned by `project_create` and `project_open` on success.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenProjectResult {
    pub project_id: String,
    pub title: String,
    pub author: String,
    pub bundle_path: String,
    /// Current project classification (Phase 4 of PRODUCT_ROADMAP_E2E.md).
    /// `None` for projects created before BookKind landed; the desktop
    /// app surfaces an onboarding overlay so the user can backfill it.
    #[serde(default)]
    pub book_kind: Option<String>,
}

/// Input for `project_kind_set` (Phase 4 / Phase 5B). Updates the
/// open project's `book_kind` (manifest.toml + open-project state).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectKindSetInput {
    /// One of `"literary-fiction" | "genre-fiction" | "non-fiction" |
    /// "memoir" | "childrens-book"`. Forgiving parse via
    /// `BookKind::from_str` accepts snake-case + bare aliases.
    pub book_kind: String,
}

/// Result of `project_kind_set`.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectKindSetResult {
    pub project_id: String,
    pub book_kind: String,
}

/// Input to `project_brief_save`. Persists the writer's manually-edited
/// `ProjectBrief` to book-scope memory (key `project_brief`) so the
/// orchestrator's `creative_profile` block picks it up on every agent
/// run. Round 5 of PRODUCT_ROADMAP_E2E.md.
///
/// All fields are optional except the brief structure itself. Empty
/// arrays / null Options for the uniqueness fields are valid — they
/// signal "no signal here, render conservatively."
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectBriefSaveInput {
    /// The full edited brief, serialised as JSON. Validated against the
    /// `ProjectBrief` shape on the backend before write.
    #[ts(type = "unknown")]
    pub brief_json: serde_json::Value,
}

/// Result of `project_brief_save` and `project_brief_load`. The
/// `value_json` field is the brief itself; `loaded` is false when no
/// brief has ever been saved for this project (i.e. the project is
/// pre-intake).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectBriefDto {
    pub loaded: bool,
    #[ts(type = "unknown")]
    pub brief_json: serde_json::Value,
    /// Audit-ledger origin of the saved brief. Surfaced in the
    /// BriefEditorPanel so the writer knows where the data came from
    /// — `"wizard"` (collected by the New Project wizard before
    /// outline generation), `"intake"` (extracted by the intake
    /// agent from a free-form idea), `"user-edit"` (manually saved
    /// from the Brief panel), or empty for never-saved.
    #[serde(default)]
    pub source: String,
    /// ISO-8601 timestamp of the last save. Empty for never-saved.
    #[serde(default)]
    pub updated_at: String,
}

/// One row in the recent-projects list (returned by `project_recent`).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecentProjectEntry {
    pub id: String,
    pub path: String,
    pub name: String,
    /// ISO-8601 timestamp string.
    pub last_opened: String,
    /// `true` when the path no longer exists on disk.
    pub missing: bool,
}

/// Input for `project_recent_remove`.  Removes a single entry from
/// `~/.booksforge/settings.toml`'s recent-projects list.  Does NOT
/// delete the bundle on disk — only the entry in the picker.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecentRemoveInput {
    /// Absolute bundle path of the entry to remove (matches the
    /// `path` field returned by `project_recent`).
    pub path: String,
}
