//! Tauri application entry point and command registration.

#![forbid(unsafe_code)]
// BACKLOG §C4: tests freely use `.unwrap()` / `.expect()` against canned
// fixtures; the workspace-level clippy lints fire only on shipped code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod commands;
pub mod logging;
pub mod scheduler;
pub mod state;

// `tauri::Builder::run` calls `process::exit` internally on the
// platform-native event loop teardown — this is the documented entry
// point and is not avoidable.  The workspace clippy policy denies
// `exit` and warns on `expect`; the few uses here are at app boot
// (logging init, tokio rt for orphan-temp cleanup) where there is no
// recoverable path — failing fast is the right behaviour and keeps
// the boot sequence simple.
#[allow(clippy::exit, clippy::expect_used)]
pub fn run() {
    // Initialise logging — stdout + rotating file appender, with a
    // PII redaction barrier on log file writes.  See `logging.rs`.
    let _file_log_guard = logging::init_tracing();

    // Clean up any temp dirs left by crashed bundle-creation attempts.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(booksforge_fs::cleanup_orphan_temp_dirs());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .setup(|app| {
            // D7 — hourly auto-snapshot loop.  Touched on every scene_save;
            // fires only when there's been activity since the last tick.
            scheduler::spawn_auto_snapshot_task(app.handle().clone());
            Ok(())
        })
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
            commands::agents::agent_run_outline,
            commands::agents::agent_apply_outline,
            commands::agents::agent_apply_copyedit,
            commands::agents::agent_apply_humanization,
            commands::agents::agent_apply_continuity,
            commands::agents::agent_run_copyedit,
            commands::agents::agent_run_continuity,
            commands::agents::agent_run_intake,
            commands::agents::agent_run_intake_and_outline,
            commands::agents::agent_run_developmental_review,
            commands::agents::entity_bible_apply_proposals,
            commands::agents::agent_cancel,
            commands::agents::agent_run_memory_curator,
            commands::agents::agent_run_vocab_dictionary,
            commands::agents::agent_run_chapter_drafter,
            commands::agents::agent_run_dev_editor,
            commands::agents::agent_run_humanization,
            commands::agents::agent_run_proposal_validator,
            commands::agents::voice_fingerprint_refresh,
            commands::agents::voice_fingerprint_load,
            commands::agents::originality_scan_chapter,
            commands::agents::vocab_apply_proposals,
            commands::agents::originality_consent_load,
            commands::agents::originality_consent_set,
            commands::agents::originality_consent_clear,
            commands::snapshot::snapshot_create,
            commands::snapshot::snapshot_list,
            commands::snapshot::snapshot_diff,
            commands::snapshot::snapshot_restore,
            commands::ai::ai_suggest,
            commands::ai::ai_cancel,
            commands::ai::ai_apply,
            commands::export::export_markdown,
            commands::export::export_run,
            commands::export::export_history,
            commands::export::export_check_dependencies,
            commands::diagnostics::save_diagnostic_bundle,
            commands::validators::validators_run,
            commands::validators::validators_gate,
            commands::validators::validators_apply_fix,
            commands::memory_vocab::memory_list,
            commands::memory_vocab::vocab_list,
            commands::memory_vocab::memory_upsert,
            commands::memory_vocab::memory_delete,
            commands::memory_vocab::vocab_upsert,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri app failed to run");
}
