//! Tauri command for the manuscript validator subsystem (Phase 4).

use std::sync::Arc;

use booksforge_domain::{Severity, ValidationReport};
use booksforge_export::pm_doc_to_markdown;
use booksforge_ipc::{
    ApplyFixInput, ApplyFixResult, BooksForgeError, ExportGateDto, ValidatorIssueDto,
    ValidatorReportDto,
};
use booksforge_storage::StorageRepository;
use booksforge_validator::{
    apply_fix, compute_scope_hash, run_all_validators, ProjectMetaSummary, SceneText,
    ValidatorContext,
};
use chrono::Utc;
use tauri::State;
use ulid::Ulid;

use crate::state::AppState;

/// Run every shipped validator against the open project, persist a
/// `validator_runs` row, and return the report shaped for the UI.
#[tauri::command]
pub async fn validators_run(
    state: State<'_, AppState>,
) -> Result<ValidatorReportDto, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let storage: Arc<_> = project.storage.clone();

    // Gather everything the validators need.
    let nodes = storage.list_nodes().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let scene_rows = storage.list_all_scene_content().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let scenes: Vec<SceneText> = scene_rows
        .iter()
        .map(|s| SceneText {
            node_id: s.node_id,
            text:    pm_doc_to_markdown(&s.pm_doc),
        })
        .collect();

    let style = storage.load_style_book().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // Active layers.  MVP heuristic: project + ai_tells + the recorded mode
    // and genre from the manifest.  For now we always include `project`
    // and `ai_tells`; later this will read from the project manifest.
    let layers: Vec<&str> = vec!["project", "ai_tells"];
    let vocab = storage.vocab_list_by_layers(&layers).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // Project metadata used by the KDP-metadata validator (G3).  Pulled
    // from the open-project state today; once the manifest exposes a
    // language tag it'll flow through here automatically.
    let meta = ProjectMetaSummary {
        title:    project.title.clone(),
        author:   project.author.clone(),
        language: String::new(), // tracked in manifest.toml; future field
        isbn:     None,
    };

    let ctx = ValidatorContext {
        nodes:               &nodes,
        scenes:              &scenes,
        style:               &style,
        vocab:               &vocab,
        active_vocab_layers: &layers,
        project:             Some(&meta),
    };

    // Cache short-circuit: if the previous run hashed the same content,
    // hand back the persisted issues without re-running the 16-validator
    // battery.  Saves a couple-hundred-ms on every re-open / pre-export
    // gate when nothing has changed.
    let current_hash = compute_scope_hash(&ctx);
    if let Ok(Some(latest)) = storage.latest_validator_run().await {
        if latest.scope_hash == current_hash {
            if let Ok(issues) = storage
                .list_validator_issues_for_run(latest.id)
                .await
            {
                let cached = ValidationReport {
                    run:    latest,
                    issues,
                };
                return Ok(report_to_dto(&cached));
            }
        }
    }

    let report: ValidationReport = run_all_validators(&ctx);

    // Persist the run + issues for history / caching.  Best-effort: a
    // failure here doesn't block the UI from seeing the result.
    let _ = storage.validator_run_persist(&report.run, &report.issues).await;

    Ok(report_to_dto(&report))
}

/// Apply the export gate (errors block, warnings prompt, info silent) and
/// return the result for the UI.
#[tauri::command]
pub async fn validators_gate(
    state: State<'_, AppState>,
) -> Result<ExportGateDto, BooksForgeError> {
    let report_dto = validators_run(state).await?;
    let errors:   Vec<ValidatorIssueDto> = report_dto.issues.iter()
        .filter(|i| i.severity == "error").cloned().collect();
    let warnings: Vec<ValidatorIssueDto> = report_dto.issues.iter()
        .filter(|i| i.severity == "warning").cloned().collect();
    let outcome = if !errors.is_empty()  { "block" }
                  else if !warnings.is_empty() { "warn" }
                  else { "pass" };
    Ok(ExportGateDto {
        outcome:  outcome.to_owned(),
        errors,
        warnings,
    })
}

// ── Mapping helpers ───────────────────────────────────────────────────────────

fn report_to_dto(report: &ValidationReport) -> ValidatorReportDto {
    let issues: Vec<ValidatorIssueDto> = report.issues.iter().map(|i| {
        ValidatorIssueDto {
            validator_id: i.validator_id.clone(),
            code:         i.code.clone(),
            severity:     i.severity.as_str().to_owned(),
            message:      i.message.clone(),
            node_id:      i.node_id.map(|u| u.to_string()),
            offset_from:  i.offset_from,
            offset_to:    i.offset_to,
            auto_fixable: i.auto_fixable,
        }
    }).collect();

    ValidatorReportDto {
        run_id:        report.run.id.to_string(),
        status:        report.run.status.as_str().to_owned(),
        duration_ms:   report.run.duration_ms,
        error_count:   report.count(Severity::Error)   as u32,
        warning_count: report.count(Severity::Warning) as u32,
        info_count:    report.count(Severity::Info)    as u32,
        issues,
    }
}

/// Apply a deterministic auto-fix to one scene's `pm_doc`.  Idempotent:
/// re-running on already-clean content is a successful no-op.  Each fix
/// is pure-logic — no AI, no network — and `auto_fixable` is the
/// validator's own claim.
#[tauri::command]
pub async fn validators_apply_fix(
    input: ApplyFixInput,
    state: State<'_, AppState>,
) -> Result<ApplyFixResult, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;
    let storage = project.storage.clone();

    let node_id = Ulid::from_string(&input.node_id)
        .map_err(|_| BooksForgeError::validation("invalid node_id ULID".to_owned()))?;

    // Load the scene (no-op fast path if it's never been saved).
    let mut scene = match storage.load_scene(node_id).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
    {
        Some(s) => s,
        None => return Ok(ApplyFixResult {
            validator_id:  input.validator_id,
            node_id:       input.node_id,
            fixes_applied: 0,
        }),
    };

    // Build a minimal context.  The fixes that need vocab pull it; the
    // others ignore it.  Same `active_vocab_layers` policy as
    // `validators_run`.
    let style = storage.load_style_book().await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let layers: Vec<&str> = vec!["project", "ai_tells"];
    let vocab = storage.vocab_list_by_layers(&layers).await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let ctx = ValidatorContext {
        nodes:               &[],
        scenes:              &[],
        style:               &style,
        vocab:               &vocab,
        active_vocab_layers: &layers,
        project:             None,
    };

    // Run the fix in place on a clone of pm_doc, then persist if changed.
    let mut new_pm = scene.pm_doc.clone();
    let count = apply_fix(&input.validator_id, &mut new_pm, &ctx)
        .ok_or_else(|| BooksForgeError::validation(
            format!("validator '{}' has no auto-fix", input.validator_id),
        ))?;

    if count > 0 {
        // Recompute hash + word/char counts from the new content; the
        // editor will re-render once `node_list` refreshes.
        let bytes = serde_json::to_vec(&new_pm).unwrap_or_default();
        scene.pm_doc     = new_pm;
        scene.hash       = blake3::hash(&bytes).to_hex().to_string();
        scene.updated_at = Utc::now();
        storage.save_scene(&scene).await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        // The persisted SceneContent struct still carries the old word /
        // char counts; that's fine — the next autosave or node_list
        // refresh will reconcile.  We deliberately don't recompute them
        // here because pm_doc → text → wordcount is the editor's
        // responsibility.
        state.touch(); // wake the auto-snapshot scheduler
    }

    Ok(ApplyFixResult {
        validator_id:  input.validator_id,
        node_id:       input.node_id,
        fixes_applied: count,
    })
}
