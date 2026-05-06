//! Tauri application entry point and command registration.

#![forbid(unsafe_code)]

pub mod commands;
pub mod state;

use tracing_subscriber::{fmt, EnvFilter};

pub fn run() {
    // Initialise logging from RUST_LOG env var (defaults to INFO).
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::system::app_version,
            commands::system::ollama_status,
            commands::ollama::ollama_probe,
            commands::ollama::ollama_launch,
            commands::ollama::ollama_list_models,
            commands::ollama::ollama_pull,
            commands::ollama::ollama_smoke_test,
            commands::project::project_create,
            commands::project::project_open,
            commands::project::project_close,
            commands::project::project_recent,
            commands::editor::node_list,
            commands::editor::node_create,
            commands::editor::node_update,
            commands::editor::node_delete,
            commands::editor::scene_save,
            commands::editor::scene_load,
            commands::editor::recovery_check,
            commands::editor::recovery_clear,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri app failed to run");
}
