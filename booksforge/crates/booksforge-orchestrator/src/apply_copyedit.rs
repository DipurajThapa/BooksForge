//! Per-edit copyedit acceptance flow (BACKLOG §E0d.5).
//!
//! `Orchestrator::apply_copyedit_edit` accepts one entry from a stored
//! `CopyeditProposals` (persisted by a copyeditor run as an
//! `agent_outputs.content_inline` row keyed by `task_id`) and applies it to
//! the live scene's `pm_doc`.
//!
//! Flow per AGENTS.md §4.6 + the broader apply-path pattern in `apply.rs`:
//!   1. Idempotency: refuse if `(task_id, edit_index)` already has a row in
//!      `agent_applied_edits`.
//!   2. Take the mandatory `pre_agent_edit` snapshot (scope = Scene).
//!   3. Verify the edit is still applicable to the current scene text:
//!      `before` must match at the original char range, or — if the scene
//!      drifted — appear exactly once elsewhere in the flattened text.
//!   4. Replace the matched span with `after` in the flat text and rebuild
//!      `pm_doc` as a sequence of paragraph blocks (one per non-empty line).
//!   5. Save the scene and insert one `agent_applied_edits` row with
//!      `edit_kind = TextReplace`.
//!
//! Known limitation (tracked in BACKLOG): the rebuild loses inline marks
//! (bold / italic / links).  Acceptable for MVP since the Copyeditor's
//! remit is mechanical fixes (punctuation, spacing, casing, quotes,
//! dashes, spelling) which almost always live in plain stretches.  A
//! mark-preserving applier is a follow-up.

use std::sync::Arc;

use booksforge_domain::{
    flat_text_to_pm_doc, pm_doc_to_text, AppliedEditKind, CopyeditEdit, CopyeditProposals,
    HumanizationEdit, HumanizationProposals, SceneContent, SnapshotScope,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

/// Outcome of a single accepted copyedit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCopyeditResult {
    pub task_id:         String,
    pub edit_index:      u32,
    pub scene_id:        String,
    pub pre_snapshot_id: String,
    pub applied_edit_id: String,
    /// True when `before` no longer matched at the original range and we
    /// fell back to a single-occurrence search.  Surfaced to the caller so
    /// the UI can warn that the position drifted.
    pub used_fallback_search: bool,
}

impl Orchestrator {
    /// Accept the `edit_index`-th edit from the `CopyeditProposals` stored
    /// against `task_id`, applying it to scene `scene_id`.
    pub async fn apply_copyedit_edit(
        &self,
        task_id:    Ulid,
        scene_id:   Ulid,
        edit_index: u32,
    ) -> Result<ApplyCopyeditResult, OrchestratorError> {
        let snapshot: Arc<SnapshotService> = self
            .snapshot()
            .ok_or_else(|| OrchestratorError::Storage(
                "snapshot service not attached".to_owned()
            ))?;
        let storage: Arc<SqliteStorage> = self.storage_arc();

        // 1. Load the persisted proposal.
        let output = storage
            .agent_output_load(task_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| OrchestratorError::Storage(format!(
                "no agent_outputs row for task {task_id}"
            )))?;
        let raw = output
            .content_inline
            .ok_or_else(|| OrchestratorError::Storage(format!(
                "agent_outputs[{task_id}] has no inline content"
            )))?;
        let proposal: CopyeditProposals = serde_json::from_str(&raw)
            .map_err(|e| OrchestratorError::Storage(format!(
                "could not deserialise stored CopyeditProposals: {e}"
            )))?;

        let edit = proposal
            .edits
            .get(edit_index as usize)
            .cloned()
            .ok_or_else(|| OrchestratorError::OutlineApply(format!(
                "edit_index {edit_index} out of range (have {} edits)",
                proposal.edits.len()
            )))?;

        // 2. Per-edit idempotency.  Decode existing payloads and refuse a
        //    second accept of the same index.
        let prior = storage
            .list_applied_edits_for_task(task_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        for row in &prior {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&row.edit_payload_json) {
                if v.get("edit_index").and_then(|x| x.as_u64()) == Some(edit_index as u64) {
                    return Err(OrchestratorError::AlreadyApplied { task_id });
                }
            }
        }

        // 3. Load the scene.
        let scene = storage
            .load_scene(scene_id)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| OrchestratorError::Storage(format!(
                "scene {scene_id} not found"
            )))?;

        let flat = pm_doc_to_text(&scene.pm_doc);
        let (new_flat, used_fallback) = apply_one_to_text(&flat, &edit)
            .map_err(OrchestratorError::OutlineApply)?;

        // 4. Pre-edit snapshot (mandatory before mutation).
        let pre = snapshot
            .pre_agent_edit_snapshot(
                SnapshotScope::Scene,
                Some(scene_id),
                Some(format!(
                    "pre-copyedit-apply task {task_id} edit {edit_index}"
                )),
            )
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 5. Save the new scene content.
        let new_pm  = flat_text_to_pm_doc(&new_flat);
        let new_str = serde_json::to_string(&new_pm).unwrap_or_default();
        let new_hash = blake3::hash(new_str.as_bytes()).to_hex().to_string();
        let words: u32 = new_flat.split_whitespace().count() as u32;
        let chars: u32 = new_flat.chars().count() as u32;
        let new_scene = SceneContent {
            node_id:    scene_id,
            pm_doc:     new_pm,
            word_count: words,
            char_count: chars,
            hash:       new_hash,
            updated_at: Utc::now(),
        };
        storage
            .save_scene(&new_scene)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        // 6. Ledger row.
        let payload = serde_json::json!({
            "edit_index":   edit_index,
            "before":       edit.before,
            "after":        edit.after,
            "range_from":   edit.range_from,
            "range_to":     edit.range_to,
            "category":     format!("{:?}", edit.category).to_lowercase(),
            "rationale":    edit.rationale,
            "used_fallback_search": used_fallback,
        }).to_string();
        let applied = SnapshotService::build_applied_edit(
            task_id,
            scene_id,
            pre.id,
            AppliedEditKind::TextReplace,
            payload,
        );
        storage
            .agent_applied_edit_insert(&applied)
            .await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        tracing::info!(
            %task_id, %scene_id, edit_index,
            pre_snapshot = %pre.id,
            "copyedit applied"
        );

        Ok(ApplyCopyeditResult {
            task_id:         task_id.to_string(),
            edit_index,
            scene_id:        scene_id.to_string(),
            pre_snapshot_id: pre.id.to_string(),
            applied_edit_id: applied.id.to_string(),
            used_fallback_search: used_fallback,
        })
    }
}

impl Orchestrator {
    /// Accept the `edit_index`-th edit from the `HumanizationProposals`
    /// stored against `task_id`, applying it to `scene_id`.  Same flow
    /// as `apply_copyedit_edit` (BACKLOG §E0d.6).
    pub async fn apply_humanization_edit(
        &self,
        task_id:    Ulid,
        scene_id:   Ulid,
        edit_index: u32,
    ) -> Result<ApplyCopyeditResult, OrchestratorError> {
        let snapshot: Arc<SnapshotService> = self
            .snapshot()
            .ok_or_else(|| OrchestratorError::Storage(
                "snapshot service not attached".to_owned()
            ))?;
        let storage: Arc<SqliteStorage> = self.storage_arc();

        let output = storage.agent_output_load(task_id).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| OrchestratorError::Storage(format!(
                "no agent_outputs row for task {task_id}"
            )))?;
        let raw = output.content_inline.ok_or_else(|| OrchestratorError::Storage(
            format!("agent_outputs[{task_id}] has no inline content"),
        ))?;
        let proposal: HumanizationProposals = serde_json::from_str(&raw)
            .map_err(|e| OrchestratorError::Storage(format!(
                "could not deserialise stored HumanizationProposals: {e}"
            )))?;
        let edit = proposal.edits.get(edit_index as usize).cloned()
            .ok_or_else(|| OrchestratorError::OutlineApply(format!(
                "edit_index {edit_index} out of range (have {} edits)",
                proposal.edits.len()
            )))?;

        let prior = storage.list_applied_edits_for_task(task_id).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        for row in &prior {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&row.edit_payload_json) {
                if v.get("edit_index").and_then(|x| x.as_u64()) == Some(edit_index as u64) {
                    return Err(OrchestratorError::AlreadyApplied { task_id });
                }
            }
        }

        let scene = storage.load_scene(scene_id).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| OrchestratorError::Storage(format!("scene {scene_id} not found")))?;

        let flat = pm_doc_to_text(&scene.pm_doc);
        let (new_flat, used_fallback) = apply_one_humanization_to_text(&flat, &edit)
            .map_err(OrchestratorError::OutlineApply)?;

        let pre = snapshot.pre_agent_edit_snapshot(
            SnapshotScope::Scene, Some(scene_id),
            Some(format!("pre-humanization-apply task {task_id} edit {edit_index}")),
        ).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        let new_pm  = flat_text_to_pm_doc(&new_flat);
        let new_str = serde_json::to_string(&new_pm).unwrap_or_default();
        let new_hash = blake3::hash(new_str.as_bytes()).to_hex().to_string();
        let words: u32 = new_flat.split_whitespace().count() as u32;
        let chars: u32 = new_flat.chars().count() as u32;
        storage.save_scene(&SceneContent {
            node_id: scene_id, pm_doc: new_pm, word_count: words,
            char_count: chars, hash: new_hash, updated_at: Utc::now(),
        }).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        let payload = serde_json::json!({
            "edit_index":    edit_index,
            "before":        edit.before,
            "after":         edit.after,
            "range_from":    edit.range_from,
            "range_to":      edit.range_to,
            "triggered_rule": edit.triggered_rule,
            "rationale":     edit.rationale,
            "used_fallback_search": used_fallback,
            "agent":         "humanization",
        }).to_string();
        let applied = SnapshotService::build_applied_edit(
            task_id, scene_id, pre.id, AppliedEditKind::TextReplace, payload,
        );
        storage.agent_applied_edit_insert(&applied).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        tracing::info!(
            %task_id, %scene_id, edit_index, pre_snapshot = %pre.id,
            "humanization applied"
        );

        Ok(ApplyCopyeditResult {
            task_id:         task_id.to_string(),
            edit_index,
            scene_id:        scene_id.to_string(),
            pre_snapshot_id: pre.id.to_string(),
            applied_edit_id: applied.id.to_string(),
            used_fallback_search: used_fallback,
        })
    }
}

/// Apply a single `CopyeditEdit` to flat text — thin wrapper over
/// `apply_replacement`.
pub fn apply_one_to_text(flat: &str, edit: &CopyeditEdit) -> Result<(String, bool), String> {
    apply_replacement(flat, edit.range_from, edit.range_to, &edit.before, &edit.after)
}

/// Apply a single `HumanizationEdit` to flat text.
pub fn apply_one_humanization_to_text(
    flat: &str,
    edit: &HumanizationEdit,
) -> Result<(String, bool), String> {
    apply_replacement(flat, edit.range_from, edit.range_to, &edit.before, &edit.after)
}

/// Try the agent's original char range first; if `before` no longer matches
/// there, fall back to a single-occurrence substring search.  Returns the
/// new flat text plus `used_fallback` so callers can warn when the position
/// drifted.  Refuses ambiguous (multiple-match) and missing fallbacks.
///
/// Pure function — no I/O.
pub fn apply_replacement(
    flat:       &str,
    range_from: u32,
    range_to:   u32,
    before:     &str,
    after:      &str,
) -> Result<(String, bool), String> {
    let chars: Vec<char> = flat.chars().collect();
    let len = chars.len();
    let from = range_from as usize;
    let to   = range_to   as usize;

    if to <= len && from < to {
        let actual: String = chars[from..to].iter().collect();
        if actual == before {
            let mut out = String::with_capacity(flat.len());
            out.extend(chars[..from].iter());
            out.push_str(after);
            out.extend(chars[to..].iter());
            return Ok((out, false));
        }
    }

    if before.is_empty() {
        return Err("edit.before is empty — cannot locate insertion point".to_owned());
    }
    let occurrences: Vec<usize> = flat
        .match_indices(before)
        .map(|(i, _)| i)
        .collect();
    match occurrences.as_slice() {
        []  => Err(
            "edit no longer applicable: `before` text not found in current scene".to_owned()
        ),
        [byte_idx] => {
            let start = *byte_idx;
            let end   = start + before.len();
            let mut out = String::with_capacity(flat.len());
            out.push_str(&flat[..start]);
            out.push_str(after);
            out.push_str(&flat[end..]);
            Ok((out, true))
        }
        _ => Err(format!(
            "edit ambiguous: `before` matches {} positions in current scene",
            occurrences.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::CopyeditCategory;

    fn edit(from: u32, to: u32, before: &str, after: &str) -> CopyeditEdit {
        CopyeditEdit {
            range_from: from, range_to: to,
            before: before.into(), after: after.into(),
            category: CopyeditCategory::Punctuation,
            rationale: "fix".into(),
        }
    }

    #[test]
    fn original_range_match_no_fallback() {
        let src = "Hello — world.";
        // chars: H(0)e(1)l(2)l(3)o(4) (5)—(6) (7)w(8)o(9)r(10)l(11)d(12).(13)
        let e = edit(6, 7, "—", "--");
        let (out, fb) = apply_one_to_text(src, &e).unwrap();
        assert_eq!(out, "Hello -- world.");
        assert!(!fb);
    }

    #[test]
    fn fallback_finds_unique_occurrence_when_range_drifted() {
        // Original positions are wrong, but `before` appears once.
        let src = "X X Hello world.";
        let e = edit(0, 5, "Hello", "Howdy");
        let (out, fb) = apply_one_to_text(src, &e).unwrap();
        assert_eq!(out, "X X Howdy world.");
        assert!(fb);
    }

    #[test]
    fn fallback_refuses_ambiguous_multiple_matches() {
        // Original range points at non-matching text; fallback search finds
        // multiple occurrences of `before` and refuses to guess.
        let src = "XXXX and and and";
        let e = edit(0, 3, "and", "und");  // chars 0..3 = "XXX", not "and"
        let err = apply_one_to_text(src, &e).unwrap_err();
        assert!(err.contains("ambiguous"), "got {err}");
    }

    #[test]
    fn fallback_refuses_when_before_absent() {
        let src = "completely different text";
        let e = edit(0, 5, "Hello", "Howdy");
        let err = apply_one_to_text(src, &e).unwrap_err();
        assert!(err.contains("not found"), "got {err}");
    }

    // pm_doc_to_text + flat_text_to_pm_doc are tested in
    // booksforge_domain::pm_doc — deduplicated when those helpers moved
    // to the domain crate.
}
