//! Tauri commands for the single-scene editor:
//! node CRUD, scene save/load, crash recovery.

use std::sync::Arc;

use booksforge_domain::{Node, NodeKind, NodeStatus, SceneContent};
use booksforge_fs::{markdown_mirror, recovery};
use booksforge_ipc::{
    editor::{
        NodeCreateInput, NodeInfo, NodeUpdateInput, RecoveryStatus, SceneLoadResult, SceneSaveInput,
    },
    BooksForgeError,
};
use booksforge_storage::StorageRepository;
use chrono::Utc;
use tauri::State;
use ulid::Ulid;

use crate::state::AppState;

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn require_project(
    state: &AppState,
) -> Result<Arc<crate::state::OpenProject>, BooksForgeError> {
    state
        .open_project
        .lock()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| BooksForgeError::not_found("no project is currently open"))
}

fn parse_node_kind(s: &str) -> Result<NodeKind, BooksForgeError> {
    match s {
        "project" => Ok(NodeKind::Project),
        "part" => Ok(NodeKind::Part),
        "chapter" => Ok(NodeKind::Chapter),
        "scene" => Ok(NodeKind::Scene),
        "front_matter" => Ok(NodeKind::FrontMatter),
        "back_matter" => Ok(NodeKind::BackMatter),
        other => Err(BooksForgeError::validation(format!(
            "unknown node kind: {other}"
        ))),
    }
}

fn parse_node_status(s: &str) -> Result<NodeStatus, BooksForgeError> {
    match s {
        "planned" => Ok(NodeStatus::Planned),
        "drafting" => Ok(NodeStatus::Drafting),
        "revised" => Ok(NodeStatus::Revised),
        "final" => Ok(NodeStatus::Final),
        other => Err(BooksForgeError::validation(format!(
            "unknown node status: {other}"
        ))),
    }
}

fn node_kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Project => "project",
        NodeKind::Part => "part",
        NodeKind::Chapter => "chapter",
        NodeKind::Scene => "scene",
        NodeKind::FrontMatter => "front_matter",
        NodeKind::BackMatter => "back_matter",
    }
}

fn node_status_str_local(s: NodeStatus) -> &'static str {
    match s {
        NodeStatus::Planned => "planned",
        NodeStatus::Drafting => "drafting",
        NodeStatus::Revised => "revised",
        NodeStatus::Final => "final",
    }
}

fn node_to_info(node: Node) -> NodeInfo {
    NodeInfo {
        id: node.id.to_string(),
        parent_id: node.parent_id.map(|id| id.to_string()),
        kind: node_kind_str(node.kind).to_owned(),
        title: node.title,
        position: node.position,
        status: node_status_str_local(node.status).to_owned(),
        pov: node.pov,
        beat: node.beat,
        target_words: node.target_words,
        word_count: 0,
        created_at: node.created_at.to_rfc3339(),
        updated_at: node.updated_at.to_rfc3339(),
    }
}

// ── node_list ─────────────────────────────────────────────────────────────────

/// Return all non-deleted nodes ordered by LexoRank position, joined with
/// the per-scene word count from `scene_content`.  Container nodes (Part,
/// Chapter, Project) get `word_count = sum(descendant scenes)` so the
/// binder and outline view can show rollups without a second round-trip.
#[tauri::command]
pub async fn node_list(state: State<'_, AppState>) -> Result<Vec<NodeInfo>, BooksForgeError> {
    use std::collections::HashMap;

    let project = require_project(&state).await?;
    let nodes = project
        .storage
        .list_nodes()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let scenes = project
        .storage
        .list_all_scene_content()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // Per-scene word count, keyed by node id.
    let mut leaf_counts: HashMap<Ulid, u32> = HashMap::with_capacity(scenes.len());
    for s in scenes {
        leaf_counts.insert(s.node_id, s.word_count);
    }

    // For container kinds (Project / Part / Chapter), aggregate descendants.
    // Build a parent → children map first so we can DFS without recursion
    // hitting the storage layer again.
    let mut by_parent: HashMap<Option<Ulid>, Vec<Ulid>> = HashMap::new();
    for n in &nodes {
        by_parent.entry(n.parent_id).or_default().push(n.id);
    }

    fn aggregate(
        node_id: Ulid,
        by_parent: &HashMap<Option<Ulid>, Vec<Ulid>>,
        leaf_counts: &HashMap<Ulid, u32>,
    ) -> u32 {
        let mut total = leaf_counts.get(&node_id).copied().unwrap_or(0);
        if let Some(children) = by_parent.get(&Some(node_id)) {
            for child in children {
                total = total.saturating_add(aggregate(*child, by_parent, leaf_counts));
            }
        }
        total
    }

    Ok(nodes
        .into_iter()
        .map(|node| {
            let count = aggregate(node.id, &by_parent, &leaf_counts);
            let mut info = node_to_info(node);
            info.word_count = count;
            info
        })
        .collect())
}

// ── node_create ───────────────────────────────────────────────────────────────

/// Create a new node in the document tree.
#[tauri::command]
pub async fn node_create(
    input: NodeCreateInput,
    state: State<'_, AppState>,
) -> Result<NodeInfo, BooksForgeError> {
    let project = require_project(&state).await?;
    let now = Utc::now();

    let parent_id = input
        .parent_id
        .as_deref()
        .map(Ulid::from_string)
        .transpose()
        .map_err(|e| BooksForgeError::validation(format!("invalid parent_id: {e}")))?;

    let node = Node {
        id: Ulid::new(),
        parent_id,
        kind: parse_node_kind(&input.kind)?,
        title: input.title,
        position: input.position,
        status: parse_node_status(&input.status)?,
        pov: None,
        beat: None,
        target_words: input.target_words,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    project
        .storage
        .insert_node(&node)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(node_to_info(node))
}

// ── node_update ───────────────────────────────────────────────────────────────

/// Update mutable fields of an existing node.
#[tauri::command]
pub async fn node_update(
    input: NodeUpdateInput,
    state: State<'_, AppState>,
) -> Result<NodeInfo, BooksForgeError> {
    let project = require_project(&state).await?;

    let id = Ulid::from_string(&input.id)
        .map_err(|e| BooksForgeError::validation(format!("invalid id: {e}")))?;

    // Load current to merge optional fields.
    let nodes = project
        .storage
        .list_nodes()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    let mut node = nodes
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| BooksForgeError::not_found(format!("node {}", input.id)))?;

    if let Some(title) = input.title {
        node.title = title;
    }
    if let Some(pos) = input.position {
        node.position = pos;
    }
    if let Some(status) = input.status {
        node.status = parse_node_status(&status)?;
    }
    if let Some(pov) = input.pov {
        node.pov = Some(pov);
    }
    if let Some(beat) = input.beat {
        node.beat = Some(beat);
    }
    if let Some(tw) = input.target_words {
        node.target_words = Some(tw);
    }
    node.updated_at = Utc::now();

    project
        .storage
        .update_node(&node)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(node_to_info(node))
}

// ── node_delete ───────────────────────────────────────────────────────────────

/// Soft-delete a node by ID.
#[tauri::command]
pub async fn node_delete(id: String, state: State<'_, AppState>) -> Result<(), BooksForgeError> {
    let project = require_project(&state).await?;
    let ulid = Ulid::from_string(&id)
        .map_err(|e| BooksForgeError::validation(format!("invalid id: {e}")))?;
    project
        .storage
        .delete_node(ulid)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))
}

// ── scene_save ────────────────────────────────────────────────────────────────

/// Save (upsert) scene content.  Writes the recovery log entry BEFORE
/// committing to SQLite, and the Markdown mirror AFTER.
#[tauri::command]
pub async fn scene_save(
    input: SceneSaveInput,
    state: State<'_, AppState>,
) -> Result<(), BooksForgeError> {
    let project = require_project(&state).await?;

    let node_id = Ulid::from_string(&input.node_id)
        .map_err(|e| BooksForgeError::validation(format!("invalid node_id: {e}")))?;

    let now = Utc::now();
    let ts = now.to_rfc3339();

    // 1. Write recovery log BEFORE SQLite commit.
    recovery::write_pending(&project.bundle, &input.node_id, &ts)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // 2. Compute blake3 hash of the serialised pm_doc.
    let pm_str = serde_json::to_string(&input.pm_doc)
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let hash = blake3::hash(pm_str.as_bytes()).to_hex().to_string();

    let content = SceneContent {
        node_id,
        pm_doc: input.pm_doc.clone(),
        word_count: input.word_count,
        char_count: input.char_count,
        hash,
        updated_at: now,
    };

    // 3. Commit to SQLite.
    project
        .storage
        .save_scene(&content)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // 4. Mark committed in recovery log.
    recovery::write_committed(&project.bundle, &ts)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // 5. Write Markdown mirror (best-effort — do not propagate errors).
    let _ = markdown_mirror::write_mirror(&project.bundle, &input.node_id, &input.pm_doc).await;

    // 6. Mark the project as dirty so the hourly auto-snapshot scheduler
    //    fires on the next tick (D7).
    state.touch();

    Ok(())
}

// ── scene_load ────────────────────────────────────────────────────────────────

/// Load scene content by node ID.
#[tauri::command]
pub async fn scene_load(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Option<SceneLoadResult>, BooksForgeError> {
    let project = require_project(&state).await?;

    let ulid = Ulid::from_string(&node_id)
        .map_err(|e| BooksForgeError::validation(format!("invalid node_id: {e}")))?;

    let content = project
        .storage
        .load_scene(ulid)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(content.map(|c| SceneLoadResult {
        node_id: c.node_id.to_string(),
        pm_doc: c.pm_doc,
        word_count: c.word_count,
        char_count: c.char_count,
        updated_at: c.updated_at.to_rfc3339(),
    }))
}

// ── recovery_check ────────────────────────────────────────────────────────────

/// Check whether the open project has any uncommitted scene saves from a
/// prior crash.
#[tauri::command]
pub async fn recovery_check(state: State<'_, AppState>) -> Result<RecoveryStatus, BooksForgeError> {
    let project = require_project(&state).await?;

    match recovery::check(&project.bundle)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?
    {
        None => Ok(RecoveryStatus {
            has_pending: false,
            node_id: None,
            pending_at: None,
        }),
        Some((node_id, ts)) => Ok(RecoveryStatus {
            has_pending: true,
            node_id: Some(node_id),
            pending_at: Some(ts),
        }),
    }
}

// ── recovery_clear ────────────────────────────────────────────────────────────

/// Clear the recovery log after the user dismisses or applies recovery.
#[tauri::command]
pub async fn recovery_clear(state: State<'_, AppState>) -> Result<(), BooksForgeError> {
    let project = require_project(&state).await?;
    recovery::clear(&project.bundle)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))
}
