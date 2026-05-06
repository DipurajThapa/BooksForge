//! IPC types shared between the Rust backend and the TypeScript frontend.
//!
//! Every type that travels over a Tauri command derives [`serde::Serialize`],
//! [`serde::Deserialize`], and [`ts_rs::TS`]. The TypeScript bindings are
//! generated into `packages/shared-types/src/bindings/` by running:
//!
//! ```sh
//! cargo test -p booksforge-ipc
//! ```
//!
//! Commit the generated files. CI fails if the bindings drift from the Rust
//! source (see `.github/workflows/ci.yml` job `ipc-drift`).

#![forbid(unsafe_code)]

pub mod editor;
pub mod error;
pub mod project;
pub mod system;

pub use editor::{
    NodeCreateInput, NodeInfo, NodeUpdateInput, RecoveryStatus, SceneLoadResult, SceneSaveInput,
};
pub use error::BooksForgeError;
pub use project::{CreateProjectInput, OpenProjectInput, OpenProjectResult, RecentProjectEntry};
pub use system::AppVersion;

// ── ts-rs export test ────────────────────────────────────────────────────────
// Running `cargo test -p booksforge-ipc` regenerates all TypeScript bindings.
#[cfg(test)]
mod ts_bindings {
    use ts_rs::TS as _;

    const BINDINGS_DIR: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/shared-types/src/bindings");

    #[test]
    fn export_system_bindings() {
        crate::AppVersion::export_all_to(BINDINGS_DIR)
            .expect("failed to export AppVersion bindings");
    }

    #[test]
    fn export_error_bindings() {
        crate::BooksForgeError::export_all_to(BINDINGS_DIR)
            .expect("failed to export BooksForgeError bindings");
    }

    #[test]
    fn export_editor_bindings() {
        use crate::editor::*;
        NodeInfo::export_all_to(BINDINGS_DIR).expect("NodeInfo");
        NodeCreateInput::export_all_to(BINDINGS_DIR).expect("NodeCreateInput");
        NodeUpdateInput::export_all_to(BINDINGS_DIR).expect("NodeUpdateInput");
        SceneSaveInput::export_all_to(BINDINGS_DIR).expect("SceneSaveInput");
        SceneLoadResult::export_all_to(BINDINGS_DIR).expect("SceneLoadResult");
        RecoveryStatus::export_all_to(BINDINGS_DIR).expect("RecoveryStatus");
    }

    #[test]
    fn export_project_bindings() {
        use crate::project::*;
        CreateProjectInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export CreateProjectInput");
        OpenProjectInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export OpenProjectInput");
        OpenProjectResult::export_all_to(BINDINGS_DIR)
            .expect("failed to export OpenProjectResult");
        RecentProjectEntry::export_all_to(BINDINGS_DIR)
            .expect("failed to export RecentProjectEntry");
    }
}
