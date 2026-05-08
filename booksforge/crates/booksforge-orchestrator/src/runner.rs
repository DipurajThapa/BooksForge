//! Generic typed-output agent runner.
//!
//! Replaces the deleted Turn-D `findings.rs` scaffolding.  This runner is
//! agnostic to which agent is being invoked: callers supply
//!   - the agent's `AgentSpec`,
//!   - the rendered template variables,
//!   - a parser closure that turns the model's raw text into a typed
//!     domain output `T` plus performs the per-type semantic validation,
//!
//! …and the runner handles:
//!   - rendering the prompt + recording template hash,
//!   - inserting `agent_runs` / `agent_tasks` ledger rows,
//!   - the streaming Ollama call with cancel + timeout enforcement,
//!   - retry loop (≤3 attempts, AGENTS.md §6),
//!   - **Tier-1 ProposalValidator** invocation on every successful parse,
//!   - **Council assembly** of peer reviews when supplied by the caller,
//!   - persisting the `agent_outputs` row on success,
//!   - returning an `AgentRunResult<T>` with the typed output and the
//!     full `VerificationReport`.
//!
//! Tier-2 ProposalValidator (LLM) and peer-review dispatch live in the
//! caller (the per-agent orchestrator method) because they require
//! agent-specific context assembly that the generic runner doesn't have.

use std::sync::Arc;
use std::time::Instant;

use booksforge_agents::AgentSpec;
use booksforge_domain::{
    AgentOutput, AgentRun, AgentTask, AgentTaskStatus, Entity, PeerReviewResult,
    ProposalValidation, ValidationVerdict, VerificationReport, VocabEntry, VoiceFingerprint,
};
use booksforge_ollama::{
    types::{CancelToken, ChatMessage, ChatRequest, GenerateOptions},
    TokenSink,
};
use booksforge_prompt::{render, TemplateVars};
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::Serialize;
use ulid::Ulid;

use crate::{council, proposal_validator, OrchestratorError};

/// What a successful run returns.  The typed `output: T` is the agent's
/// domain output; `verification` carries the multi-tier review.
pub struct AgentRunResult<T> {
    pub run_id:       Ulid,
    pub task_id:      Ulid,
    pub status:       AgentTaskStatus,
    pub output:       Option<T>,
    pub verification: VerificationReport,
    pub raw_output:   Option<String>,
    pub error:        Option<String>,
}

/// Cross-cutting project context shared across every agent run.  Loaded
/// once by the caller (typically the Tauri command layer) and threaded
/// through to the runner where it drives the prompt-guard injection,
/// the EntitySanity check, and Tier-2 / peer-review dispatch when those
/// are enabled.
#[derive(Clone)]
pub struct RunContext {
    pub entity_bible:       Vec<Entity>,
    pub active_avoid_rules: Vec<VocabEntry>,
    pub voice_fingerprint:  VoiceFingerprint,
}

impl RunContext {
    /// Construct an empty context (no entities, no avoid-rules, default
    /// fingerprint).  Useful for agents that don't need any of these
    /// (e.g. `intake` runs before a project tree exists).
    pub fn empty() -> Self {
        Self {
            entity_bible:       Vec::new(),
            active_avoid_rules: Vec::new(),
            voice_fingerprint:  VoiceFingerprint::default(),
        }
    }
}

/// Per-call inputs the caller assembles.
pub struct RunInput<'a, T> {
    pub spec:        &'a AgentSpec,
    pub workflow_id: &'a str,
    pub project_id:  Ulid,
    pub vars:        TemplateVars,
    pub model:       &'a str,
    pub cancel:      CancelToken,
    /// Parser: raw model text → typed T (or error message for retry).
    /// Callers should run their own per-type semantic validators here.
    pub parse:       Box<dyn Fn(&str) -> Result<T, String> + Send + Sync + 'a>,
    /// Cross-cutting project context (entity bible, active avoid-rules,
    /// voice fingerprint).
    pub context:     &'a RunContext,
    /// Memory scopes the proposal touches (for the MemoryScope check).
    pub proposed_memory_scopes: &'a [String],
    /// Optional peer reviews assembled by the caller before this runner.
    /// Phase 5 wiring landed in Turn J.
    pub peer_reviews: Vec<PeerReviewResult>,
    /// Optional Tier-2 ProposalValidation produced by the caller's
    /// LLM-validator dispatch.  None when Tier-2 is disabled.
    pub tier_2:       Option<ProposalValidation>,
    /// Source text the agent operated on (scene text, synopsis, chapter
    /// text, etc.).  Powers the `Originality` cross-cutting validator —
    /// any long verbatim span in the output that appears here is flagged
    /// as plagiarism (the agent copy-pasted instead of generating).
    pub source_text:  Option<&'a str>,
    /// Concatenated text of the project's prior accepted scenes.  Powers
    /// the self-plagiarism half of `Originality`.
    pub prior_scene_corpus: Option<&'a str>,
    /// Optional per-token pulse (BACKLOG §E4 follow-up).  Called once
    /// for every token the runner receives from Ollama.  Receives the
    /// raw token text — most callers ignore it and just count via an
    /// `AtomicU64` so they can emit periodic progress events without
    /// fanning out one IPC event per token.  Async-safe (the closure
    /// itself must be sync; spawn an emitter task elsewhere).
    pub on_token: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
}

/// Drive a single agent run end-to-end.
pub async fn run<T: Serialize + Send + 'static>(
    storage:  Arc<SqliteStorage>,
    ollama:   Arc<dyn booksforge_ollama::client::OllamaClient>,
    config:   &crate::OrchestratorConfig,
    input:    RunInput<'_, T>,
) -> Result<AgentRunResult<T>, OrchestratorError> {
    let run_id = Ulid::new();
    let now    = Utc::now();

    storage.agent_run_insert(&AgentRun {
        id:             run_id,
        workflow_id:    input.workflow_id.to_owned(),
        project_id:     input.project_id,
        status:         AgentTaskStatus::Running,
        started_at:     now,
        completed_at:   None,
        total_tokens:   None,
        error_message:  None,
        ollama_version: None,
        user_initiated: true,
    }).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

    let result = run_inner(storage.clone(), ollama, config, input, run_id).await;

    let (final_status, err_msg) = match &result {
        Ok(r) if r.output.is_some() => (AgentTaskStatus::Completed, None),
        Ok(_)                        => (AgentTaskStatus::Invalid,   None),
        Err(OrchestratorError::Cancelled) => (AgentTaskStatus::Cancelled, Some("cancelled by user".to_owned())),
        Err(e)                       => (AgentTaskStatus::Error, Some(e.to_string())),
    };
    let _ = storage.agent_run_update(run_id, final_status, None, err_msg.as_deref()).await;
    result
}

async fn run_inner<T: Serialize + Send + 'static>(
    storage:  Arc<SqliteStorage>,
    ollama:   Arc<dyn booksforge_ollama::client::OllamaClient>,
    config:   &crate::OrchestratorConfig,
    input:    RunInput<'_, T>,
    run_id:   Ulid,
) -> Result<AgentRunResult<T>, OrchestratorError> {
    let RunInput {
        spec, mut vars, model, cancel, parse, context, proposed_memory_scopes,
        peer_reviews, tier_2, source_text, prior_scene_corpus, on_token,
        project_id: _, workflow_id: _,
    } = input;
    let entity_bible = &context.entity_bible;
    let active_avoid_rules = &context.active_avoid_rules;
    let voice_fingerprint = &context.voice_fingerprint;
    let template_id = spec.prompt_template.clone();
    let wall_start  = Instant::now();
    let task_id     = Ulid::new();

    // Inject the prompt-guard block (humanity + voice + avoid-rules) into
    // `vars["prompt_guard"]` so the agent's template can splice it in via
    // `{{ prompt_guard }}`.  This is the architectural anti-AI-tell knob:
    // every prose-emitting agent now sees the project's active vocab and
    // voice fingerprint as hard constraints.
    let avoid_borrowed: Vec<crate::prompt_guard::AvoidRule<'_>> = active_avoid_rules.iter()
        .filter(|e| !matches!(e.kind, booksforge_domain::EntryKind::Prefer))
        .map(|e| crate::prompt_guard::AvoidRule {
            term:        &e.display_term,
            kind:        e.kind,
            replacement: e.replacement.as_deref(),
            rationale:   e.rationale.as_deref().unwrap_or(""),
        })
        .collect();
    let guard_block = crate::prompt_guard::render(&avoid_borrowed, voice_fingerprint);
    vars.insert("prompt_guard".into(), serde_json::json!(guard_block));

    let rendered = render(&template_id, &vars)
        .map_err(|e| OrchestratorError::AgentFailed {
            agent_id: spec.id.to_owned(),
            retries:  0,
            reason:   format!("prompt render failed: {e}"),
        })?;
    let template_hash_hex = rendered.template_hash.to_hex().to_string();
    let input_blob = serde_json::to_string(&vars).unwrap_or_default();
    let input_hash = blake3::hash(input_blob.as_bytes()).to_hex().to_string();

    let task_now = Utc::now();
    storage.agent_task_insert(&AgentTask {
        id:                   task_id,
        run_id,
        step_index:           0,
        agent_id:             spec.id.to_owned(),
        prompt_template_id:   format!("{}.{}", template_id.id, template_id.version),
        prompt_template_hash: template_hash_hex,
        model:                model.to_owned(),
        model_digest:         None,
        input_hash,
        output_hash:          None,
        context_tokens:       None,
        output_tokens:        None,
        duration_ms:          None,
        retries:              0,
        status:               AgentTaskStatus::Running,
        error_category:       None,
        error_message:        None,
        created_at:           task_now,
        updated_at:           task_now,
    }).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

    #[allow(unused_assignments)]
    let mut attempt:    u32           = 0;
    #[allow(unused_assignments)]
    let mut last_raw:   Option<String> = None;
    #[allow(unused_assignments)]
    let mut last_error: Option<String> = None;

    loop {
        if cancel.is_cancelled() {
            storage.agent_task_update(
                task_id, AgentTaskStatus::Cancelled, None, None, None,
                Some(wall_start.elapsed().as_millis() as u64),
                attempt, None, Some("cancelled by user"),
            ).await.ok();
            return Err(OrchestratorError::Cancelled);
        }
        if wall_start.elapsed().as_secs() >= config.max_duration_secs {
            let msg = format!("time limit {}s exceeded after attempt {attempt}", config.max_duration_secs);
            storage.agent_task_update(
                task_id, AgentTaskStatus::Error, None, None, None,
                Some(wall_start.elapsed().as_millis() as u64),
                attempt, Some("timeout"), Some(&msg),
            ).await.ok();
            return Err(OrchestratorError::TimeLimitExceeded { limit_secs: config.max_duration_secs });
        }

        let messages = vec![
            ChatMessage { role: "system".to_owned(), content: rendered.system.clone() },
            ChatMessage { role: "user".to_owned(),   content: rendered.user.clone() },
        ];
        let req = ChatRequest {
            model:   model.to_owned(),
            messages,
            stream:  true,
            options: Some(GenerateOptions {
                temperature: Some(0.3),
                top_p:       None,
                num_ctx:     Some(spec.context_budget.total()),
                num_predict: Some(spec.context_budget.max_output_tokens),
            }),
        };

        let buf  = Arc::new(std::sync::Mutex::new(String::new()));
        let buf2 = buf.clone();
        let on_token_clone = on_token.clone();
        let sink: TokenSink = Box::new(move |t: &str| {
            if let Ok(mut b) = buf2.lock() { b.push_str(t); }
            if let Some(cb) = &on_token_clone { cb(t); }
        });

        let step_start  = Instant::now();
        let chat_result = ollama.chat(req, sink, cancel.clone()).await;
        let step_ms     = step_start.elapsed().as_millis() as u64;

        let raw_text = match chat_result {
            Ok(ref outcome) if !outcome.message.content.is_empty() => outcome.message.content.clone(),
            _ => buf.lock().map(|b| b.clone()).unwrap_or_default(),
        };
        last_raw = Some(raw_text.clone());

        let stripped = strip_code_fences(&raw_text);
        match parse(stripped) {
            Ok(typed_output) => {
                // Run Tier-1 ProposalValidator on the parsed output.
                let parsed_value = serde_json::to_value(&typed_output)
                    .unwrap_or_else(|_| serde_json::json!({}));
                let tier_1 = proposal_validator::run_tier1(
                    spec, &raw_text, &parsed_value, entity_bible, proposed_memory_scopes,
                    source_text, prior_scene_corpus,
                );

                // Assemble the council report (Tier-1 + Tier-2 + peer reviews).
                let report = council::assemble_report(
                    spec.id, &task_id.to_string(),
                    tier_1, tier_2.clone(), peer_reviews.clone(),
                );

                if matches!(report.final_verdict, ValidationVerdict::Block)
                    && council::should_retry_primary(report.final_verdict, attempt)
                {
                    last_error = Some(format!(
                        "council blocked the proposal: {}", report.tier_1.summary
                    ));
                    attempt += 1;
                    if attempt > config.max_retries_per_step { break; }
                    continue;
                }

                let out_json = serde_json::to_string(&typed_output).unwrap_or_default();
                let out_hash = blake3::hash(out_json.as_bytes()).to_hex().to_string();
                let validated = Utc::now();
                storage.agent_task_update(
                    task_id, AgentTaskStatus::Completed,
                    Some(&out_hash), None, None, Some(step_ms),
                    attempt, None, None,
                ).await.ok();
                storage.agent_output_insert(&AgentOutput {
                    task_id,
                    schema_id:      spec.output_schema_id.to_owned(),
                    schema_version: 1,
                    content_inline: Some(out_json),
                    content_path:   None,
                    hash:           out_hash,
                    validated_at:   validated,
                }).await.ok();

                return Ok(AgentRunResult {
                    run_id, task_id,
                    status:       AgentTaskStatus::Completed,
                    output:       Some(typed_output),
                    verification: report,
                    raw_output:   last_raw,
                    error:        None,
                });
            }
            Err(reason) => {
                last_error = Some(reason);
            }
        }

        attempt += 1;
        if attempt > config.max_retries_per_step { break; }
        tracing::warn!(
            agent = spec.id, %run_id, attempt, error = ?last_error,
            "agent failed — retrying"
        );
    }

    storage.agent_task_update(
        task_id, AgentTaskStatus::Invalid, None, None, None,
        Some(wall_start.elapsed().as_millis() as u64),
        attempt.saturating_sub(1), Some("schema_or_semantic"), last_error.as_deref(),
    ).await.ok();

    // On exhausted retries we still produce a verification report so the
    // caller can surface why the run was rejected.
    let empty_tier_1 = ProposalValidation {
        verdict: ValidationVerdict::Block,
        checks:  Vec::new(),
        summary: last_error.clone().unwrap_or_else(|| "max retries exceeded".to_owned()),
        tier_2_ran: false,
    };
    let report = council::assemble_report(
        spec.id, &task_id.to_string(), empty_tier_1, None, Vec::new(),
    );

    Ok(AgentRunResult {
        run_id, task_id,
        status:       AgentTaskStatus::Invalid,
        output:       None,
        verification: report,
        raw_output:   last_raw,
        error:        last_error,
    })
}

pub(crate) fn strip_code_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("```json") {
        if let Some(inner2) = inner.strip_suffix("```") { return inner2.trim(); }
    }
    if let Some(inner) = s.strip_prefix("```") {
        if let Some(inner2) = inner.strip_suffix("```") { return inner2.trim(); }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_fences_handles_json_variant() {
        assert_eq!(strip_code_fences("```json\n{}\n```"), "{}");
        assert_eq!(strip_code_fences("```\n{}\n```"), "{}");
        assert_eq!(strip_code_fences("{}"), "{}");
    }
}
