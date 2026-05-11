//! Quick-action presets (MZ-08) — Sharpen / Continue / Rephrase.
//!
//! Quick actions are inline editor helpers that emit raw prose, not validated
//! JSON.  They are recorded in `ai_calls` (not `agent_runs / agent_tasks /
//! agent_outputs`) and use a `pre_ai` snapshot trigger when the suggestion
//! is accepted.
//!
//! # Streaming contract
//!
//! `run_quick_action` accepts a caller-supplied [`TokenSink`] so the Tauri
//! command layer can re-emit tokens to the frontend as they arrive.  The
//! function additionally accumulates the full text into `ai_calls.output_text`
//! for audit and apply-on-accept, even if the user cancels mid-stream.
//!
//! # Audit ledger guarantee
//!
//! Every call writes exactly one `ai_calls` row, regardless of outcome:
//!   - `status = ok`        — model returned, full text persisted.
//!   - `status = cancelled` — `CancelToken::cancel()` triggered; partial
//!                            buffer persisted.
//!   - `status = error`     — Ollama error; partial buffer (if any) persisted.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use booksforge_domain::{AiCall, AiCallStatus, QuickActionPreset, SnapshotScope};
use booksforge_ollama::{
    types::{CancelToken, GenerateOptions, GenerateRequest},
    TokenSink,
};
use booksforge_prompt::{render, PromptTemplateId, TemplateVars};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

// ── Public types ──────────────────────────────────────────────────────────────

/// Per-call options that the UI may override.
#[derive(Debug, Clone)]
pub struct QuickActionOptions {
    /// Optional model override; falls back to the project's default model
    /// when `None`.
    pub model: Option<String>,
    /// Sampling temperature.  MVP default is 0.4 — a small amount of variety
    /// without going off-piste.
    pub temperature: Option<f32>,
    /// Output token cap.  MVP default 1024.
    pub max_output: Option<u32>,
    /// Optional template vars merged into the render context (e.g. genre,
    /// audience, preceding_context for Continue).  Keys must not collide
    /// with `scope_text`.
    pub extra_vars: serde_json::Map<String, serde_json::Value>,
}

impl Default for QuickActionOptions {
    fn default() -> Self {
        Self {
            model: None,
            temperature: Some(0.4),
            max_output: Some(1024),
            extra_vars: serde_json::Map::new(),
        }
    }
}

/// Outcome of a quick-action call.  `ai_call_id` always points at a real
/// `ai_calls` row so the UI can subsequently call `apply_quick_action`.
#[derive(Debug, Clone)]
pub struct QuickActionOutcome {
    pub ai_call_id: Ulid,
    pub status: AiCallStatus,
    pub output_text: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

// ── Orchestrator methods ──────────────────────────────────────────────────────

impl Orchestrator {
    /// Run a quick-action preset: render its template, stream tokens through
    /// the supplied sink, and persist exactly one `ai_calls` row.
    ///
    /// Returns the outcome — including the full accumulated output (even on
    /// cancellation) — so the UI's diff panel can show the partial result
    /// for inspection or discard.
    pub async fn run_quick_action(
        &self,
        node_id: Ulid,
        preset: QuickActionPreset,
        scope_text: String,
        model: String,
        options: QuickActionOptions,
        cancel: CancelToken,
        ui_sink: TokenSink,
    ) -> Result<QuickActionOutcome, OrchestratorError> {
        let storage: Arc<SqliteStorage> = self.storage_arc();

        // High-end presets override the caller-supplied model.  This keeps
        // the world-class polish flow always pointed at qwen3.6, even if
        // the project default is a lower-RAM model.
        let model = preset.pinned_model().map(|p| p.to_owned()).unwrap_or(model);

        // 1. Render the prompt template.
        let (tpl_id, tpl_ver) = preset.template();
        let template_id = PromptTemplateId::new(tpl_id, tpl_ver);
        let mut vars = TemplateVars::new();
        vars.insert("scope_text".to_owned(), serde_json::json!(scope_text));
        for (k, v) in options.extra_vars.iter() {
            vars.insert(k.clone(), v.clone());
        }
        let rendered = render(&template_id, &vars).map_err(|e| {
            OrchestratorError::Storage(format!("prompt render failed for {tpl_id}: {e}"))
        })?;
        let prompt_template_id = format!("{tpl_id}.{tpl_ver}");
        let prompt_template_hash = rendered.template_hash.to_hex().to_string();

        // 2. Build a tee-sink that forwards tokens to the UI and also
        //    accumulates them into a buffer for audit + apply.
        let buf = Arc::new(Mutex::new(String::new()));
        let buf_for_sink = Arc::clone(&buf);
        let mut ui_sink = ui_sink; // FnMut — must be `mut`
        let tee_sink: TokenSink = Box::new(move |tok: &str| {
            if let Ok(mut b) = buf_for_sink.lock() {
                b.push_str(tok);
            }
            ui_sink(tok);
        });

        // 3. Compose the prompt — concat system + user.  `generate` takes a
        //    single prompt string; we put system instructions first to mirror
        //    the chat-format Ollama would build for an instruct model.
        let prompt_full = format!("{}\n\n{}", rendered.system.trim(), rendered.user.trim());
        let req = GenerateRequest {
            model: model.clone(),
            prompt: prompt_full,
            system: None,
            stream: true,
            think: None,
            // quick_action emits free-form text, not JSON — leave `format`
            // unset so Ollama doesn't constrain the output to JSON tokens.
            format: None,
            options: Some(GenerateOptions {
                temperature: options.temperature,
                top_p: None,
                num_ctx: None,
                num_predict: options.max_output,
            }),
        };

        // 4. Run the call.  Capture timing for the audit row.
        let started = Instant::now();
        let ai_call_id = Ulid::new();

        let result = self
            .ollama_arc()
            .generate(req, tee_sink, cancel.clone())
            .await;
        let duration_ms = started.elapsed().as_millis() as u64;

        let buffered = buf.lock().map(|s| s.clone()).unwrap_or_default();

        // 5. Classify outcome and persist exactly one ledger row.
        let (status, error_message, ctx_tok, out_tok) = match &result {
            Ok(outcome) => (
                AiCallStatus::Ok,
                None,
                Some(outcome.prompt_eval_count),
                Some(outcome.eval_count),
            ),
            Err(booksforge_ollama::OllamaError::Cancelled) => (
                AiCallStatus::Cancelled,
                Some("cancelled by user".to_owned()),
                None,
                None,
            ),
            Err(e) => (AiCallStatus::Error, Some(e.to_string()), None, None),
        };

        let row = AiCall {
            id: ai_call_id,
            node_id,
            preset,
            model,
            prompt_template_id,
            prompt_template_hash,
            scope_text_len: scope_text.chars().count() as u32,
            output_text: (!buffered.is_empty()).then(|| buffered.clone()),
            context_tokens: ctx_tok,
            output_tokens: out_tok,
            duration_ms: Some(duration_ms),
            status,
            error_message: error_message.clone(),
            created_at: Utc::now(),
            pre_edit_snapshot_id: None,
            applied_at: None,
        };
        storage
            .ai_call_insert(&row)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        Ok(QuickActionOutcome {
            ai_call_id,
            status,
            output_text: buffered,
            duration_ms,
            error: error_message,
        })
    }

    /// Accept a previously-suggested quick-action: take the mandatory
    /// `pre_ai` snapshot, replace the scene's prose body with the accepted
    /// text, and stamp the `ai_calls` row with the snapshot id + applied_at.
    ///
    /// `op` controls whether the accepted text replaces the scope or is
    /// appended to it (Continue uses `append`).
    pub async fn apply_quick_action(
        &self,
        ai_call_id: Ulid,
        accepted_text: String,
        op: ApplyOp,
    ) -> Result<ApplyQuickActionResult, OrchestratorError> {
        let svc: Arc<SnapshotService> = self.snapshot().ok_or_else(|| {
            OrchestratorError::Storage("snapshot service not attached".to_owned())
        })?;
        let storage: Arc<SqliteStorage> = self.storage_arc();

        // 1. Resolve the call + target node.
        let call = storage
            .ai_call_get(ai_call_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| OrchestratorError::Storage(format!("ai_call {ai_call_id} not found")))?;
        if call.applied_at.is_some() {
            return Err(OrchestratorError::AlreadyApplied {
                task_id: ai_call_id,
            });
        }

        // 2. Take the mandatory pre_ai snapshot.
        let snap = svc
            .pre_ai_snapshot(
                SnapshotScope::Scene,
                Some(call.node_id),
                Some(format!(
                    "pre-{} for ai_call {ai_call_id}",
                    call.preset.as_str()
                )),
            )
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 3. Build the new scene_content.pm_doc.
        let existing = storage
            .load_scene(call.node_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        let new_text = match op {
            ApplyOp::Replace => accepted_text.clone(),
            ApplyOp::Append => {
                let prev = existing
                    .as_ref()
                    .and_then(|s| extract_plain_text(&s.pm_doc))
                    .unwrap_or_default();
                if prev.is_empty() {
                    accepted_text.clone()
                } else {
                    format!("{prev}\n\n{accepted_text}")
                }
            }
        };
        let pm_doc = pm_doc_from_plain_text(&new_text);
        let bytes = serde_json::to_vec(&pm_doc).unwrap_or_default();
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let scene = booksforge_domain::SceneContent {
            node_id: call.node_id,
            pm_doc,
            word_count: new_text.split_whitespace().count() as u32,
            char_count: new_text.chars().count() as u32,
            hash,
            updated_at: Utc::now(),
        };
        storage
            .save_scene(&scene)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 4. Stamp the ledger row.
        let applied_at = Utc::now();
        storage
            .ai_call_update_apply(ai_call_id, snap.id, applied_at)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        Ok(ApplyQuickActionResult {
            ai_call_id,
            pre_snapshot_id: snap.id,
            applied_at,
        })
    }

    /// Internal accessor — quick_action.rs needs to call the Ollama client.
    pub(crate) fn ollama_arc(&self) -> Arc<dyn booksforge_ollama::client::OllamaClient> {
        self.ollama_clone()
    }
}

/// How an accepted quick-action suggestion is applied to the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyOp {
    /// Sharpen / Rephrase: replace the scoped passage entirely.
    Replace,
    /// Continue: append the new paragraphs after the existing scene text.
    Append,
}

/// Outcome of `apply_quick_action`.
#[derive(Debug, Clone)]
pub struct ApplyQuickActionResult {
    pub ai_call_id: Ulid,
    pub pre_snapshot_id: Ulid,
    pub applied_at: chrono::DateTime<Utc>,
}

// ── pm_doc ↔ plain-text helpers ───────────────────────────────────────────────

/// Read a single-paragraph-per-block ProseMirror doc as plain text.  Best-
/// effort: collects all `text` leaves separated by paragraph breaks.
fn extract_plain_text(pm_doc: &serde_json::Value) -> Option<String> {
    let blocks = pm_doc.get("content")?.as_array()?;
    let mut paragraphs: Vec<String> = Vec::with_capacity(blocks.len());
    for block in blocks {
        let mut buf = String::new();
        if let Some(inlines) = block.get("content").and_then(|v| v.as_array()) {
            for inline in inlines {
                if let Some(t) = inline.get("text").and_then(|v| v.as_str()) {
                    buf.push_str(t);
                }
            }
        }
        if !buf.is_empty() {
            paragraphs.push(buf);
        }
    }
    Some(paragraphs.join("\n\n"))
}

/// Wrap plain text into a ProseMirror doc with one paragraph per blank-line-
/// separated block.  Empty input produces an empty paragraph (TipTap requires
/// at least one block).
fn pm_doc_from_plain_text(text: &str) -> serde_json::Value {
    let blocks: Vec<serde_json::Value> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            serde_json::json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": p }]
            })
        })
        .collect();

    let content = if blocks.is_empty() {
        vec![serde_json::json!({ "type": "paragraph" })]
    } else {
        blocks
    };

    serde_json::json!({ "type": "doc", "content": content })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pm_doc_roundtrip() {
        let doc = pm_doc_from_plain_text("Hello.\n\nWorld.");
        let plain = extract_plain_text(&doc).unwrap();
        assert_eq!(plain, "Hello.\n\nWorld.");
    }

    #[test]
    fn pm_doc_empty_input_yields_empty_paragraph() {
        let doc = pm_doc_from_plain_text("");
        let blocks = doc.get("content").and_then(|v| v.as_array()).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn quick_action_options_default_is_modest_temperature() {
        let opts = QuickActionOptions::default();
        assert_eq!(opts.temperature, Some(0.4));
        assert_eq!(opts.max_output, Some(1024));
    }
}
