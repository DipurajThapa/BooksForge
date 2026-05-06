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

pub mod error;
pub mod system;

pub use error::BooksForgeError;
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
}
