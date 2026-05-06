//! Workflow trigger types and run handle.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// What triggers a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "trigger", rename_all = "snake_case")]
pub enum WorkflowTrigger {
    /// User submitted a new book brief; runs intake + outline-architect.
    BookIntake {
        brief_text: String,
    },

    /// User requests a chapter to be drafted.
    DraftChapter {
        node_id: String,
    },

    /// User accepted an outline and requests full draft generation.
    AcceptOutline {
        project_id: String,
    },

    /// User requests a dev-edit pass on a chapter.
    DevEdit {
        node_id: String,
    },

    /// User requests a continuity check across all chapters.
    ContinuityCheck {
        project_id: String,
    },

    /// User requests a copyedit pass on a chapter.
    Copyedit {
        node_id: String,
    },

    /// User requests the humanization pass on a chapter.
    Humanize {
        node_id: String,
    },
}

/// An opaque handle to a running (or completed) workflow.
/// The orchestrator returns this immediately so the UI can poll for events.
#[derive(Debug, Clone)]
pub struct RunHandle {
    pub run_id: Ulid,
}

impl RunHandle {
    pub fn new() -> Self {
        Self { run_id: Ulid::new() }
    }

    pub fn id_string(&self) -> String {
        self.run_id.to_string()
    }
}

impl Default for RunHandle {
    fn default() -> Self {
        Self::new()
    }
}
