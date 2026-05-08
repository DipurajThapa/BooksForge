//! Continuity rename / annotate apply path (BACKLOG §E0d.7).
//!
//! `Orchestrator::apply_continuity_fix` accepts one entry from a stored
//! `ContinuityReport` and applies the proposed fix:
//!
//!   - **Rename** (`from → to`, scope = Scene / Chapter / Project) — takes a
//!     pre-edit snapshot at the appropriate scope, walks every scene in
//!     the scope, replaces `from` with `to` in the flat text via
//!     whole-word match (so "Anna" doesn't rewrite "Annapurna"), saves
//!     each affected scene.  Inserts one `agent_applied_edits` row of
//!     kind `RenameEntity` per affected scene.
//!
//!   - **Annotate** — upserts a `MemoryEntry` in the Entity scope under
//!     key `continuity:<finding_index>`, value = the finding's
//!     diagnosis.  Inserts one `agent_applied_edits` row of kind
//!     `NoteAdd`.
//!
//!   - **None** — refuses; the user shouldn't accept a "no fix" finding
//!     as if it were applied.
//!
//! Like the copyedit path, idempotency is per-(task_id, finding_index):
//! a second accept of the same finding is refused.

use std::sync::Arc;

use booksforge_domain::{
    flat_text_to_pm_doc, pm_doc_to_text, AppliedEditKind, ContinuityFixKind,
    ContinuityFixScope, ContinuityReport, MemoryEntry, MemoryScope, NodeKind, SceneContent,
    SnapshotScope,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{SqliteStorage, StorageRepository as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Orchestrator, OrchestratorError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyContinuityResult {
    pub task_id:           String,
    pub finding_index:     u32,
    pub kind:              String,                 // "rename" | "annotate"
    pub pre_snapshot_id:   String,
    pub applied_edit_ids:  Vec<String>,
    pub scenes_touched:    u32,
    pub from_term:         Option<String>,
    pub to_term:           Option<String>,
}

impl Orchestrator {
    /// Accept the `finding_index`-th entry from the `ContinuityReport`
    /// stored against `task_id` and apply its `proposed_fix`.
    /// `project_id` is needed to scope a `SnapshotScope::Project` rename.
    pub async fn apply_continuity_fix(
        &self,
        project_id:    Ulid,
        task_id:       Ulid,
        finding_index: u32,
    ) -> Result<ApplyContinuityResult, OrchestratorError> {
        let snapshot: Arc<SnapshotService> = self.snapshot()
            .ok_or_else(|| OrchestratorError::Storage(
                "snapshot service not attached".to_owned()
            ))?;
        let storage: Arc<SqliteStorage> = self.storage_arc();

        // Load the persisted report.
        let output = storage.agent_output_load(task_id).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
            .ok_or_else(|| OrchestratorError::Storage(format!(
                "no agent_outputs row for task {task_id}"
            )))?;
        let raw = output.content_inline.ok_or_else(|| OrchestratorError::Storage(
            format!("agent_outputs[{task_id}] has no inline content"),
        ))?;
        let report: ContinuityReport = serde_json::from_str(&raw)
            .map_err(|e| OrchestratorError::Storage(format!(
                "could not deserialise stored ContinuityReport: {e}"
            )))?;

        let finding = report.findings.get(finding_index as usize).cloned()
            .ok_or_else(|| OrchestratorError::OutlineApply(format!(
                "finding_index {finding_index} out of range (have {} findings)",
                report.findings.len()
            )))?;

        // Idempotency.
        let prior = storage.list_applied_edits_for_task(task_id).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        for row in &prior {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&row.edit_payload_json) {
                if v.get("finding_index").and_then(|x| x.as_u64()) == Some(finding_index as u64) {
                    return Err(OrchestratorError::AlreadyApplied { task_id });
                }
            }
        }

        match finding.proposed_fix.kind {
            ContinuityFixKind::None => Err(OrchestratorError::OutlineApply(
                "finding has no proposed fix to apply".to_owned()
            )),
            ContinuityFixKind::Rename => {
                let from = finding.proposed_fix.from.clone().unwrap_or_default();
                let to   = finding.proposed_fix.to.clone().unwrap_or_default();
                if from.is_empty() || to.is_empty() {
                    return Err(OrchestratorError::OutlineApply(
                        "rename fix missing from/to".to_owned()
                    ));
                }
                apply_rename(
                    &storage, &snapshot, project_id, task_id, finding_index,
                    &from, &to, finding.proposed_fix.scope,
                    finding.evidence.iter().filter_map(|e| Ulid::from_string(&e.node_id).ok()).collect(),
                ).await
            }
            ContinuityFixKind::Annotate => {
                apply_annotate(
                    &storage, &snapshot, project_id, task_id, finding_index,
                    &finding.diagnosis,
                ).await
            }
        }
    }
}

async fn apply_rename(
    storage:       &Arc<SqliteStorage>,
    snapshot:      &Arc<SnapshotService>,
    project_id:    Ulid,
    task_id:       Ulid,
    finding_index: u32,
    from:          &str,
    to:            &str,
    scope:         ContinuityFixScope,
    evidence_node_ids: Vec<Ulid>,
) -> Result<ApplyContinuityResult, OrchestratorError> {
    // Scope decides snapshot scope + which scenes are candidates.
    let (snap_scope, scope_id, candidate_scenes) = match scope {
        ContinuityFixScope::Project => (SnapshotScope::Project, Some(project_id), None),
        ContinuityFixScope::Chapter => {
            // The evidence's first node is presumed to be a scene under the
            // affected chapter.  Walk up the tree to find the chapter id.
            let nodes = storage.list_nodes().await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            let chapter_id = evidence_node_ids.first()
                .and_then(|nid| {
                    let scene = nodes.iter().find(|n| n.id == *nid)?;
                    scene.parent_id
                });
            let scenes: Vec<Ulid> = if let Some(cid) = chapter_id {
                nodes.iter()
                    .filter(|n| n.parent_id == Some(cid) && matches!(n.kind, NodeKind::Scene))
                    .map(|n| n.id)
                    .collect()
            } else {
                evidence_node_ids.clone()
            };
            (SnapshotScope::Project, Some(project_id), Some(scenes))
        }
        ContinuityFixScope::Scene => (
            SnapshotScope::Scene,
            evidence_node_ids.first().copied(),
            Some(evidence_node_ids.clone()),
        ),
    };

    // Take the pre-edit snapshot.
    let pre = snapshot.pre_agent_edit_snapshot(
        snap_scope, scope_id,
        Some(format!("pre-continuity-rename task {task_id} finding {finding_index} ({from} -> {to})")),
    ).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

    // Resolve the candidate scenes.
    let scenes_to_walk: Vec<Ulid> = match candidate_scenes {
        Some(s) => s,
        None => {
            let nodes = storage.list_nodes().await
                .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
            nodes.iter()
                .filter(|n| matches!(n.kind, NodeKind::Scene))
                .map(|n| n.id)
                .collect()
        }
    };

    let mut applied_ids = Vec::new();
    let mut touched = 0u32;
    for scene_id in scenes_to_walk {
        let scene = match storage.load_scene(scene_id).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?
        { Some(s) => s, None => continue };

        let flat = pm_doc_to_text(&scene.pm_doc);
        let new_flat = whole_word_replace(&flat, from, to);
        if new_flat == flat { continue; } // no occurrences in this scene

        let new_pm  = flat_text_to_pm_doc(&new_flat);
        let new_str = serde_json::to_string(&new_pm).unwrap_or_default();
        let new_hash = blake3::hash(new_str.as_bytes()).to_hex().to_string();
        let words: u32 = new_flat.split_whitespace().count() as u32;
        let chars: u32 = new_flat.chars().count() as u32;
        storage.save_scene(&SceneContent {
            node_id: scene_id, pm_doc: new_pm,
            word_count: words, char_count: chars, hash: new_hash,
            updated_at: Utc::now(),
        }).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

        let payload = serde_json::json!({
            "finding_index": finding_index,
            "from":          from,
            "to":            to,
            "scope":         format!("{scope:?}").to_lowercase(),
            "scene_id":      scene_id.to_string(),
        }).to_string();
        let edit = SnapshotService::build_applied_edit(
            task_id, scene_id, pre.id, AppliedEditKind::RenameEntity, payload,
        );
        storage.agent_applied_edit_insert(&edit).await
            .map_err(|e| OrchestratorError::Storage(e.to_string()))?;
        applied_ids.push(edit.id.to_string());
        touched += 1;
    }

    tracing::info!(
        %task_id, finding_index, from, to,
        scope = ?scope, scenes_touched = touched, pre_snapshot = %pre.id,
        "continuity rename applied"
    );

    Ok(ApplyContinuityResult {
        task_id:          task_id.to_string(),
        finding_index,
        kind:             "rename".to_owned(),
        pre_snapshot_id:  pre.id.to_string(),
        applied_edit_ids: applied_ids,
        scenes_touched:   touched,
        from_term:        Some(from.to_owned()),
        to_term:          Some(to.to_owned()),
    })
}

async fn apply_annotate(
    storage:       &Arc<SqliteStorage>,
    snapshot:      &Arc<SnapshotService>,
    project_id:    Ulid,
    task_id:       Ulid,
    finding_index: u32,
    diagnosis:     &str,
) -> Result<ApplyContinuityResult, OrchestratorError> {
    // Snapshot the project state before mutating memory.  Memory writes
    // are smaller-blast-radius than scene rewrites, but we still want the
    // pre-state on hand if the user wants to revert.
    let pre = snapshot.pre_agent_edit_snapshot(
        SnapshotScope::Project, Some(project_id),
        Some(format!("pre-continuity-annotate task {task_id} finding {finding_index}")),
    ).await.map_err(|e| OrchestratorError::Storage(e.to_string()))?;

    let key = format!("continuity:{finding_index}");
    let now = Utc::now();
    let entry = MemoryEntry {
        id:         Ulid::new(),
        scope:      MemoryScope::Entity,
        key:        key.clone(),
        value_json: serde_json::json!({
            "diagnosis":     diagnosis,
            "task_id":       task_id.to_string(),
            "finding_index": finding_index,
        }),
        agent_id:   "continuity".to_owned(),
        created_at: now,
        updated_at: now,
    };
    storage.memory_upsert(&entry).await
        .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

    // Use the project root as the ledger node_id for project-wide annotations.
    let node_id = project_id;
    let payload = serde_json::json!({
        "finding_index": finding_index,
        "memory_key":    key,
        "diagnosis":     diagnosis,
    }).to_string();
    let edit = SnapshotService::build_applied_edit(
        task_id, node_id, pre.id, AppliedEditKind::NoteAdd, payload,
    );
    storage.agent_applied_edit_insert(&edit).await
        .map_err(|e| OrchestratorError::Storage(e.to_string()))?;

    tracing::info!(
        %task_id, finding_index, pre_snapshot = %pre.id,
        "continuity annotation applied"
    );

    Ok(ApplyContinuityResult {
        task_id:          task_id.to_string(),
        finding_index,
        kind:             "annotate".to_owned(),
        pre_snapshot_id:  pre.id.to_string(),
        applied_edit_ids: vec![edit.id.to_string()],
        scenes_touched:   0,
        from_term:        None,
        to_term:          None,
    })
}

/// Whole-word, case-sensitive replacement.  Avoids "Anna → Anya" rewriting
/// "Annapurna" — we only replace when the match is bounded by non-alphanumeric
/// characters (or string boundaries).  Pure function — public for tests.
pub fn whole_word_replace(haystack: &str, from: &str, to: &str) -> String {
    if from.is_empty() || haystack.is_empty() { return haystack.to_owned(); }
    let bytes = haystack.as_bytes();
    let from_bytes = from.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if i + from_bytes.len() <= bytes.len()
            && &bytes[i..i + from_bytes.len()] == from_bytes
        {
            let left_ok  = i == 0 || !is_word_char(bytes[i - 1] as char);
            let right_idx = i + from_bytes.len();
            let right_ok = right_idx >= bytes.len() || !is_word_char(bytes[right_idx] as char);
            if left_ok && right_ok {
                out.push_str(to);
                i = right_idx;
                continue;
            }
        }
        // Push the next char (UTF-8 safe).
        let ch_start = i;
        while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 { i += 1; }
        i = (ch_start + 1).max(i + 1).min(bytes.len());
        out.push_str(&haystack[ch_start..i]);
    }
    out
}

fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_word_replaces_only_full_matches() {
        assert_eq!(whole_word_replace("Anna and Annapurna", "Anna", "Anya"),
                   "Anya and Annapurna");
    }

    #[test]
    fn whole_word_handles_punctuation_boundaries() {
        assert_eq!(whole_word_replace("Anna, Anna's pen.", "Anna", "Anya"),
                   "Anya, Anya's pen.");
    }

    #[test]
    fn whole_word_no_op_when_term_absent() {
        let s = "no occurrences here";
        assert_eq!(whole_word_replace(s, "Anna", "Anya"), s);
    }

    #[test]
    fn whole_word_handles_repeated_occurrences() {
        assert_eq!(whole_word_replace("Anna Anna Anna", "Anna", "X"),
                   "X X X");
    }

    #[test]
    fn whole_word_empty_inputs() {
        assert_eq!(whole_word_replace("", "x", "y"), "");
        assert_eq!(whole_word_replace("abc", "", "y"), "abc");
    }
}
