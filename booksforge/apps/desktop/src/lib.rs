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
        .invoke_handler(tauri::generate_handler![
            commands::system::app_version,
            commands::system::ollama_status,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri app failed to run");
}
