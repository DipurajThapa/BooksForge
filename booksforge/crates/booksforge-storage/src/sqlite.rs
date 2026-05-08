//! `SqliteStorage` — production implementation of `StorageRepository`.
//!
//! All SQL goes through `sqlx::query!` macros for compile-time checking.
//! IDs are stored as ULID strings; timestamps as ISO-8601 UTC text.

use async_trait::async_trait;
use booksforge_domain::{
    AgentAppliedEdit, AgentOutput, AgentRun, AgentTask, AgentTaskStatus, AiCall, AiCallStatus,
    AppliedEditKind, EllipsisForm, EmDash, Entity, EntityKind, EntryKind, EntrySource,
    ExportProfile, ExportRecord, MemoryEntry, MemoryScope, Node, NodeKind, NodeStatus,
    QuickActionPreset, QuoteStyle, SceneContent, Severity, SnapshotRecord, SnapshotScope,
    SnapshotTrigger, StyleBook, ValidatorIssue, ValidatorRun, ValidatorRunStatus, VocabEntry,
};
use chrono::{DateTime, Utc};
use ulid::Ulid;

use crate::{pool::DbPool, repository::StorageRepository, StorageError};

/// Production SQLite-backed implementation of `StorageRepository`.
#[derive(Clone)]
pub struct SqliteStorage {
    pool: DbPool,
}

impl SqliteStorage {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn ulid_to_str(id: Ulid) -> String {
    id.to_string()
}

fn str_to_ulid(s: &str) -> Result<Ulid, StorageError> {
    Ulid::from_string(s).map_err(|e| StorageError::ConstraintViolation {
        detail: format!("invalid ULID '{s}': {e}"),
    })
}

fn ts_to_str(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn str_to_ts(s: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| StorageError::ConstraintViolation {
            detail: format!("invalid timestamp '{s}': {e}"),
        })
}

// ── StorageRepository impl ───────────────────────────────────────────────────

#[async_trait]
impl StorageRepository for SqliteStorage {
    // ── Nodes ──────────────────────────────────────────────────────────────

    async fn list_nodes(&self) -> Result<Vec<Node>, StorageError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, parent_id, kind, title, position, status,
                   pov, beat, target_words, created_at, updated_at, deleted_at
            FROM nodes
            WHERE deleted_at IS NULL
            ORDER BY position ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(Node {
                    id:           str_to_ulid(&r.id)?,
                    parent_id:    r.parent_id.as_deref().map(str_to_ulid).transpose()?,
                    kind:         parse_node_kind(&r.kind)?,
                    title:        r.title,
                    position:     r.position,
                    status:       parse_node_status(&r.status)?,
                    pov:          r.pov,
                    beat:         r.beat,
                    target_words: r.target_words.map(|v| v as u32),
                    created_at:   str_to_ts(&r.created_at)?,
                    updated_at:   str_to_ts(&r.updated_at)?,
                    deleted_at:   r.deleted_at.as_deref().map(str_to_ts).transpose()?,
                })
            })
            .collect()
    }

    async fn insert_node(&self, node: &Node) -> Result<(), StorageError> {
        let id        = ulid_to_str(node.id);
        let parent_id = node.parent_id.map(ulid_to_str);
        let kind      = node_kind_str(node.kind);
        let status    = node_status_str(node.status);
        let created   = ts_to_str(node.created_at);
        let updated   = ts_to_str(node.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO nodes
                (id, parent_id, kind, title, position, status,
                 pov, beat, target_words, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id, parent_id, kind, node.title, node.position, status,
            node.pov, node.beat, node.target_words, created, updated,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_nodes_batch(&self, nodes: &[Node]) -> Result<(), StorageError> {
        if nodes.is_empty() { return Ok(()); }
        let mut tx = self.pool.begin().await?;
        for node in nodes {
            let id        = ulid_to_str(node.id);
            let parent_id = node.parent_id.map(ulid_to_str);
            let kind      = node_kind_str(node.kind);
            let status    = node_status_str(node.status);
            let created   = ts_to_str(node.created_at);
            let updated   = ts_to_str(node.updated_at);
            sqlx::query!(
                r#"
                INSERT INTO nodes
                    (id, parent_id, kind, title, position, status,
                     pov, beat, target_words, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                id, parent_id, kind, node.title, node.position, status,
                node.pov, node.beat, node.target_words, created, updated,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn upsert_node(&self, node: &Node) -> Result<(), StorageError> {
        let id        = ulid_to_str(node.id);
        let parent_id = node.parent_id.map(ulid_to_str);
        let kind      = node_kind_str(node.kind);
        let status    = node_status_str(node.status);
        let created   = ts_to_str(node.created_at);
        let updated   = ts_to_str(node.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO nodes
                (id, parent_id, kind, title, position, status,
                 pov, beat, target_words, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                parent_id    = excluded.parent_id,
                kind         = excluded.kind,
                title        = excluded.title,
                position     = excluded.position,
                status       = excluded.status,
                pov          = excluded.pov,
                beat         = excluded.beat,
                target_words = excluded.target_words,
                updated_at   = excluded.updated_at,
                deleted_at   = NULL
            "#,
            id, parent_id, kind, node.title, node.position, status,
            node.pov, node.beat, node.target_words, created, updated,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_node(&self, node: &Node) -> Result<(), StorageError> {
        let status  = node_status_str(node.status);
        let updated = ts_to_str(node.updated_at);
        let id      = ulid_to_str(node.id);
        sqlx::query!(
            r#"
            UPDATE nodes
            SET title = ?, position = ?, status = ?,
                pov = ?, beat = ?, target_words = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            node.title, node.position, status,
            node.pov, node.beat, node.target_words,
            updated, id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_node(&self, id: Ulid) -> Result<(), StorageError> {
        let now = ts_to_str(Utc::now());
        let id_str = ulid_to_str(id);
        sqlx::query!(
            "UPDATE nodes SET deleted_at = ? WHERE id = ?",
            now,
            id_str,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Scene content ──────────────────────────────────────────────────────

    async fn load_scene(&self, node_id: Ulid) -> Result<Option<SceneContent>, StorageError> {
        let id_str = ulid_to_str(node_id);
        let row = sqlx::query!(
            r#"
            SELECT node_id, pm_doc, word_count, char_count, hash, updated_at
            FROM scene_content
            WHERE node_id = ?
            "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some(SceneContent {
                node_id:    str_to_ulid(&r.node_id)?,
                pm_doc:     serde_json::from_str(&r.pm_doc)?,
                word_count: r.word_count as u32,
                char_count: r.char_count as u32,
                hash:       r.hash,
                updated_at: str_to_ts(&r.updated_at)?,
            })),
        }
    }

    async fn save_scene(&self, content: &SceneContent) -> Result<(), StorageError> {
        let pm_str  = serde_json::to_string(&content.pm_doc)?;
        let id      = ulid_to_str(content.node_id);
        let word    = content.word_count as i64;
        let chars   = content.char_count as i64;
        let updated = ts_to_str(content.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO scene_content (node_id, pm_doc, word_count, char_count, hash, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(node_id) DO UPDATE SET
                pm_doc     = excluded.pm_doc,
                word_count = excluded.word_count,
                char_count = excluded.char_count,
                hash       = excluded.hash,
                updated_at = excluded.updated_at
            "#,
            id, pm_str, word, chars, content.hash, updated,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Style book ─────────────────────────────────────────────────────────

    async fn load_style_book(&self) -> Result<StyleBook, StorageError> {
        let row = sqlx::query!(
            r#"
            SELECT em_dash, oxford_comma, quote_style, spaces_after_period,
                   ellipsis_form, spelling_locale, capitalize_after_colon,
                   bold_emphasis_allowed
            FROM style_book WHERE id = 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(StyleBook::default()),
            Some(r) => Ok(StyleBook {
                em_dash:                parse_em_dash(&r.em_dash)?,
                oxford_comma:           r.oxford_comma != 0,
                quote_style:            parse_quote_style(&r.quote_style)?,
                spaces_after_period:    r.spaces_after_period as u8,
                ellipsis_form:          parse_ellipsis(&r.ellipsis_form)?,
                spelling_locale:        r.spelling_locale,
                capitalize_after_colon: r.capitalize_after_colon != 0,
                bold_emphasis_allowed:  r.bold_emphasis_allowed != 0,
            }),
        }
    }

    async fn save_style_book(&self, style: &StyleBook) -> Result<(), StorageError> {
        let now      = ts_to_str(Utc::now());
        let em_dash  = em_dash_str(style.em_dash);
        let oxford   = style.oxford_comma as i64;
        let quote    = quote_style_str(style.quote_style);
        let spaces   = style.spaces_after_period as i64;
        let ellipsis = ellipsis_str(style.ellipsis_form);
        let locale   = &style.spelling_locale;
        let cap_colon = style.capitalize_after_colon as i64;
        let bold      = style.bold_emphasis_allowed as i64;
        sqlx::query!(
            r#"
            INSERT INTO style_book
                (id, em_dash, oxford_comma, quote_style, spaces_after_period,
                 ellipsis_form, spelling_locale, capitalize_after_colon,
                 bold_emphasis_allowed, updated_at)
            VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                em_dash                = excluded.em_dash,
                oxford_comma           = excluded.oxford_comma,
                quote_style            = excluded.quote_style,
                spaces_after_period    = excluded.spaces_after_period,
                ellipsis_form          = excluded.ellipsis_form,
                spelling_locale        = excluded.spelling_locale,
                capitalize_after_colon = excluded.capitalize_after_colon,
                bold_emphasis_allowed  = excluded.bold_emphasis_allowed,
                updated_at             = excluded.updated_at
            "#,
            em_dash, oxford, quote, spaces, ellipsis, locale, cap_colon, bold, now,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Entities ───────────────────────────────────────────────────────────

    async fn list_entities(&self) -> Result<Vec<Entity>, StorageError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, kind, name, fields_json, notes,
                   created_at, updated_at, deleted_at
            FROM entities
            WHERE deleted_at IS NULL
            ORDER BY name ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entities = Vec::with_capacity(rows.len());
        for r in rows {
            let entity_id = r.id.clone();
            let aliases = sqlx::query!(
                "SELECT alias FROM entity_aliases WHERE entity_id = ? ORDER BY alias ASC",
                entity_id
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|a| a.alias)
            .collect();

            entities.push(Entity {
                id:          str_to_ulid(&r.id)?,
                kind:        parse_entity_kind(&r.kind)?,
                name:        r.name,
                aliases,
                fields_json: serde_json::from_str(&r.fields_json)?,
                notes:       r.notes,
                created_at:  str_to_ts(&r.created_at)?,
                updated_at:  str_to_ts(&r.updated_at)?,
                deleted_at:  r.deleted_at.as_deref().map(str_to_ts).transpose()?,
            });
        }
        Ok(entities)
    }

    async fn insert_entity(&self, entity: &Entity) -> Result<(), StorageError> {
        let fields_str = serde_json::to_string(&entity.fields_json)?;
        let id_str    = ulid_to_str(entity.id);
        let kind_str  = entity_kind_str(entity.kind);
        let created   = ts_to_str(entity.created_at);
        let updated   = ts_to_str(entity.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO entities
                (id, kind, name, fields_json, notes, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            id_str, kind_str, entity.name, fields_str, entity.notes, created, updated,
        )
        .execute(&self.pool)
        .await?;

        for alias in &entity.aliases {
            sqlx::query!(
                "INSERT OR IGNORE INTO entity_aliases (entity_id, alias) VALUES (?, ?)",
                id_str,
                alias,
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn delete_entity(&self, id: Ulid) -> Result<(), StorageError> {
        let now = ts_to_str(Utc::now());
        let id_str = ulid_to_str(id);
        sqlx::query!(
            "UPDATE entities SET deleted_at = ? WHERE id = ?",
            now,
            id_str,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Snapshots ──────────────────────────────────────────────────────────

    async fn insert_snapshot(&self, snap: &SnapshotRecord) -> Result<(), StorageError> {
        let id          = ulid_to_str(snap.id);
        let scope       = snapshot_scope_str(snap.scope);
        let scope_id    = snap.scope_id.map(ulid_to_str);
        let trigger     = snapshot_trigger_str(snap.trigger);
        let created_at  = ts_to_str(snap.created_at);
        let size_bytes  = snap.size_bytes as i64;
        sqlx::query!(
            r#"
            INSERT INTO snapshots
                (id, scope, scope_id, label, trigger, tree_hash, created_at, size_bytes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id, scope, scope_id, snap.label, trigger, snap.tree_hash, created_at, size_bytes,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_snapshots(
        &self,
        scope_id: Option<Ulid>,
    ) -> Result<Vec<SnapshotRecord>, StorageError> {
        let scope_str = scope_id.map(ulid_to_str);
        let rows = sqlx::query!(
            r#"
            SELECT id, scope, scope_id, label, trigger, tree_hash, created_at, size_bytes
            FROM snapshots
            WHERE (? IS NULL OR scope_id = ?)
            ORDER BY created_at DESC
            "#,
            scope_str,
            scope_str,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(SnapshotRecord {
                    id:         str_to_ulid(&r.id)?,
                    scope:      parse_snapshot_scope(&r.scope)?,
                    scope_id:   r.scope_id.as_deref().map(str_to_ulid).transpose()?,
                    label:      r.label,
                    trigger:    parse_snapshot_trigger(&r.trigger)?,
                    tree_hash:  r.tree_hash,
                    created_at: str_to_ts(&r.created_at)?,
                    size_bytes: r.size_bytes as u64,
                })
            })
            .collect()
    }

    async fn get_snapshot(&self, id: Ulid) -> Result<Option<SnapshotRecord>, StorageError> {
        let id_str = ulid_to_str(id);
        let row = sqlx::query!(
            r#"
            SELECT id, scope, scope_id, label, trigger, tree_hash, created_at, size_bytes
            FROM snapshots
            WHERE id = ?
            "#,
            id_str,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some(SnapshotRecord {
                id:         str_to_ulid(&r.id)?,
                scope:      parse_snapshot_scope(&r.scope)?,
                scope_id:   r.scope_id.as_deref().map(str_to_ulid).transpose()?,
                label:      r.label,
                trigger:    parse_snapshot_trigger(&r.trigger)?,
                tree_hash:  r.tree_hash,
                created_at: str_to_ts(&r.created_at)?,
                size_bytes: r.size_bytes as u64,
            })),
        }
    }

    async fn agent_applied_edit_insert(
        &self,
        edit: &AgentAppliedEdit,
    ) -> Result<(), StorageError> {
        // Invariant: pre-edit snapshot must exist and predate `applied_at`.
        let snap_id  = ulid_to_str(edit.pre_edit_snapshot_id);
        let snap_row = sqlx::query!(
            "SELECT created_at FROM snapshots WHERE id = ?",
            snap_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        let snap_created_at = match snap_row {
            None => {
                return Err(StorageError::ConstraintViolation {
                    detail: format!(
                        "pre_edit_snapshot_id {} does not exist",
                        edit.pre_edit_snapshot_id
                    ),
                });
            }
            Some(r) => str_to_ts(&r.created_at)?,
        };

        if snap_created_at >= edit.applied_at {
            return Err(StorageError::ConstraintViolation {
                detail: format!(
                    "snapshot.created_at {snap_created_at} must precede applied_at {}",
                    edit.applied_at
                ),
            });
        }

        let id         = ulid_to_str(edit.id);
        let task_id    = ulid_to_str(edit.task_id);
        let node_id    = ulid_to_str(edit.node_id);
        let applied_at = ts_to_str(edit.applied_at);
        let edit_kind  = applied_edit_kind_str(edit.edit_kind);
        let reverted   = edit.reverted_at.map(ts_to_str);
        sqlx::query!(
            r#"
            INSERT INTO agent_applied_edits
                (id, task_id, node_id, pre_edit_snapshot_id,
                 applied_at, edit_kind, edit_payload_json, reverted_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id, task_id, node_id, snap_id,
            applied_at, edit_kind, edit.edit_payload_json, reverted,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn count_applied_edits_for_task(
        &self,
        task_id: Ulid,
    ) -> Result<u32, StorageError> {
        let id = ulid_to_str(task_id);
        let row = sqlx::query!(
            "SELECT COUNT(*) AS c FROM agent_applied_edits WHERE task_id = ?",
            id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.c as u32)
    }

    async fn list_applied_edits_for_task(
        &self,
        task_id: Ulid,
    ) -> Result<Vec<AgentAppliedEdit>, StorageError> {
        let task_id_str = ulid_to_str(task_id);
        let rows = sqlx::query!(
            r#"
            SELECT id, task_id, node_id, pre_edit_snapshot_id,
                   applied_at, edit_kind, edit_payload_json, reverted_at
            FROM agent_applied_edits
            WHERE task_id = ?
            ORDER BY applied_at ASC
            "#,
            task_id_str,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(AgentAppliedEdit {
                    id:                   str_to_ulid(&r.id)?,
                    task_id:              str_to_ulid(&r.task_id)?,
                    node_id:              str_to_ulid(&r.node_id)?,
                    pre_edit_snapshot_id: str_to_ulid(&r.pre_edit_snapshot_id)?,
                    applied_at:           str_to_ts(&r.applied_at)?,
                    edit_kind:            parse_applied_edit_kind(&r.edit_kind)?,
                    edit_payload_json:    r.edit_payload_json,
                    reverted_at:          r.reverted_at.as_deref().map(str_to_ts).transpose()?,
                })
            })
            .collect()
    }

    async fn recent_applied_edits_for_project(
        &self,
        project_id: Ulid,
        kind:       AppliedEditKind,
        limit:      u32,
    ) -> Result<Vec<AgentAppliedEdit>, StorageError> {
        let project_id_str = ulid_to_str(project_id);
        let kind_str = applied_edit_kind_str(kind);
        let lim      = limit as i64;
        let rows = sqlx::query!(
            r#"
            SELECT e.id, e.task_id, e.node_id, e.pre_edit_snapshot_id,
                   e.applied_at, e.edit_kind, e.edit_payload_json, e.reverted_at
            FROM agent_applied_edits e
            JOIN agent_tasks t ON t.id = e.task_id
            JOIN agent_runs  r ON r.id = t.run_id
            WHERE r.project_id = ? AND e.edit_kind = ?
            ORDER BY e.applied_at DESC
            LIMIT ?
            "#,
            project_id_str, kind_str, lim,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(AgentAppliedEdit {
                    id:                   str_to_ulid(&r.id)?,
                    task_id:              str_to_ulid(&r.task_id)?,
                    node_id:              str_to_ulid(&r.node_id)?,
                    pre_edit_snapshot_id: str_to_ulid(&r.pre_edit_snapshot_id)?,
                    applied_at:           str_to_ts(&r.applied_at)?,
                    edit_kind:            parse_applied_edit_kind(&r.edit_kind)?,
                    edit_payload_json:    r.edit_payload_json,
                    reverted_at:          r.reverted_at.as_deref().map(str_to_ts).transpose()?,
                })
            })
            .collect()
    }

    async fn list_applied_edits_for_run(
        &self,
        run_id: Ulid,
    ) -> Result<Vec<AgentAppliedEdit>, StorageError> {
        let run_id_str = ulid_to_str(run_id);
        let rows = sqlx::query!(
            r#"
            SELECT e.id, e.task_id, e.node_id, e.pre_edit_snapshot_id,
                   e.applied_at, e.edit_kind, e.edit_payload_json, e.reverted_at
            FROM agent_applied_edits e
            JOIN agent_tasks t ON t.id = e.task_id
            WHERE t.run_id = ?
            ORDER BY e.applied_at ASC
            "#,
            run_id_str,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(AgentAppliedEdit {
                    id:                   str_to_ulid(&r.id)?,
                    task_id:              str_to_ulid(&r.task_id)?,
                    node_id:              str_to_ulid(&r.node_id)?,
                    pre_edit_snapshot_id: str_to_ulid(&r.pre_edit_snapshot_id)?,
                    applied_at:           str_to_ts(&r.applied_at)?,
                    edit_kind:            parse_applied_edit_kind(&r.edit_kind)?,
                    edit_payload_json:    r.edit_payload_json,
                    reverted_at:          r.reverted_at.as_deref().map(str_to_ts).transpose()?,
                })
            })
            .collect()
    }

    // ── Agent audit ledger ────────────────────────────────────────────────

    async fn agent_run_insert(&self, run: &AgentRun) -> Result<(), StorageError> {
        let id           = ulid_to_str(run.id);
        let project_id   = ulid_to_str(run.project_id);
        let status       = run.status.as_str();
        let started_at   = ts_to_str(run.started_at);
        let completed_at = run.completed_at.map(ts_to_str);
        let total_tokens = run.total_tokens.map(|t| t as i64);
        let user_init    = run.user_initiated as i64;
        sqlx::query!(
            r#"
            INSERT INTO agent_runs
                (id, workflow_id, project_id, status, started_at,
                 completed_at, total_tokens, error_message, ollama_version, user_initiated)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id, run.workflow_id, project_id, status, started_at,
            completed_at, total_tokens, run.error_message, run.ollama_version, user_init,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn agent_run_update(
        &self,
        id:            Ulid,
        status:        AgentTaskStatus,
        total_tokens:  Option<u32>,
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let id_str       = ulid_to_str(id);
        let status_str   = status.as_str();
        let completed_at = ts_to_str(Utc::now());
        let tokens       = total_tokens.map(|t| t as i64);
        sqlx::query!(
            r#"
            UPDATE agent_runs
            SET status = ?, completed_at = ?, total_tokens = ?, error_message = ?
            WHERE id = ?
            "#,
            status_str, completed_at, tokens, error_message, id_str,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn agent_task_insert(&self, task: &AgentTask) -> Result<(), StorageError> {
        let id       = ulid_to_str(task.id);
        let run_id   = ulid_to_str(task.run_id);
        let step     = task.step_index as i64;
        let status   = task.status.as_str();
        let retries  = task.retries as i64;
        let ctx_tok  = task.context_tokens.map(|t| t as i64);
        let out_tok  = task.output_tokens.map(|t| t as i64);
        let dur_ms   = task.duration_ms.map(|d| d as i64);
        let created  = ts_to_str(task.created_at);
        let updated  = ts_to_str(task.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO agent_tasks
                (id, run_id, step_index, agent_id, prompt_template_id,
                 prompt_template_hash, model, model_digest, input_hash,
                 output_hash, context_tokens, output_tokens, duration_ms,
                 retries, status, error_category, error_message, created_at, updated_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
            id, run_id, step, task.agent_id, task.prompt_template_id,
            task.prompt_template_hash, task.model, task.model_digest, task.input_hash,
            task.output_hash, ctx_tok, out_tok, dur_ms,
            retries, status, task.error_category, task.error_message, created, updated,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn agent_task_update(
        &self,
        id:             Ulid,
        status:         AgentTaskStatus,
        output_hash:    Option<&str>,
        context_tokens: Option<u32>,
        output_tokens:  Option<u32>,
        duration_ms:    Option<u64>,
        retries:        u32,
        error_category: Option<&str>,
        error_message:  Option<&str>,
    ) -> Result<(), StorageError> {
        let id_str  = ulid_to_str(id);
        let st      = status.as_str();
        let updated = ts_to_str(Utc::now());
        let ctx     = context_tokens.map(|t| t as i64);
        let out     = output_tokens.map(|t| t as i64);
        let dur     = duration_ms.map(|d| d as i64);
        let ret     = retries as i64;
        sqlx::query!(
            r#"
            UPDATE agent_tasks
            SET status = ?, output_hash = ?, context_tokens = ?, output_tokens = ?,
                duration_ms = ?, retries = ?, error_category = ?, error_message = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            st, output_hash, ctx, out, dur, ret, error_category, error_message, updated, id_str,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn agent_output_insert(&self, output: &AgentOutput) -> Result<(), StorageError> {
        let task_id      = ulid_to_str(output.task_id);
        let schema_ver   = output.schema_version as i64;
        let validated_at = ts_to_str(output.validated_at);
        sqlx::query!(
            r#"
            INSERT INTO agent_outputs
                (task_id, schema_id, schema_version, content_inline,
                 content_path, hash, validated_at)
            VALUES (?,?,?,?,?,?,?)
            "#,
            task_id, output.schema_id, schema_ver,
            output.content_inline, output.content_path,
            output.hash, validated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn agent_output_load(&self, task_id: Ulid) -> Result<Option<AgentOutput>, StorageError> {
        let task_id_str = ulid_to_str(task_id);
        let row = sqlx::query!(
            r#"
            SELECT task_id, schema_id, schema_version, content_inline,
                   content_path, hash, validated_at
            FROM agent_outputs
            WHERE task_id = ?
            "#,
            task_id_str,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some(AgentOutput {
                task_id:        str_to_ulid(&r.task_id)?,
                schema_id:      r.schema_id,
                schema_version: r.schema_version as u32,
                content_inline: r.content_inline,
                content_path:   r.content_path,
                hash:           r.hash,
                validated_at:   str_to_ts(&r.validated_at)?,
            })),
        }
    }

    // ── Quick-action ledger (MZ-08) ───────────────────────────────────────

    async fn ai_call_insert(&self, call: &AiCall) -> Result<(), StorageError> {
        let id           = ulid_to_str(call.id);
        let node_id      = ulid_to_str(call.node_id);
        let preset       = quick_action_preset_str(call.preset);
        let scope_len    = call.scope_text_len as i64;
        let ctx_tok      = call.context_tokens.map(|t| t as i64);
        let out_tok      = call.output_tokens.map(|t| t as i64);
        let dur_ms       = call.duration_ms.map(|d| d as i64);
        let status       = ai_call_status_str(call.status);
        let created_at   = ts_to_str(call.created_at);
        let pre_snap_id  = call.pre_edit_snapshot_id.map(ulid_to_str);
        let applied_at   = call.applied_at.map(ts_to_str);
        sqlx::query!(
            r#"
            INSERT INTO ai_calls
                (id, node_id, preset, model, prompt_template_id, prompt_template_hash,
                 scope_text_len, output_text, context_tokens, output_tokens, duration_ms,
                 status, error_message, created_at, pre_edit_snapshot_id, applied_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id, node_id, preset, call.model,
            call.prompt_template_id, call.prompt_template_hash,
            scope_len, call.output_text, ctx_tok, out_tok, dur_ms,
            status, call.error_message, created_at, pre_snap_id, applied_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn ai_call_update_apply(
        &self,
        id:                   Ulid,
        pre_edit_snapshot_id: Ulid,
        applied_at:           DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let id_str    = ulid_to_str(id);
        let snap_str  = ulid_to_str(pre_edit_snapshot_id);
        let when      = ts_to_str(applied_at);
        sqlx::query!(
            r#"
            UPDATE ai_calls
            SET pre_edit_snapshot_id = ?, applied_at = ?
            WHERE id = ?
            "#,
            snap_str, when, id_str,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn ai_call_get(&self, id: Ulid) -> Result<Option<AiCall>, StorageError> {
        let id_str = ulid_to_str(id);
        let row = sqlx::query!(
            r#"
            SELECT id, node_id, preset, model, prompt_template_id, prompt_template_hash,
                   scope_text_len, output_text, context_tokens, output_tokens, duration_ms,
                   status, error_message, created_at, pre_edit_snapshot_id, applied_at
            FROM ai_calls
            WHERE id = ?
            "#,
            id_str,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some(AiCall {
                id:                   str_to_ulid(&r.id)?,
                node_id:              str_to_ulid(&r.node_id)?,
                preset:               parse_quick_action_preset(&r.preset)?,
                model:                r.model,
                prompt_template_id:   r.prompt_template_id,
                prompt_template_hash: r.prompt_template_hash,
                scope_text_len:       r.scope_text_len as u32,
                output_text:          r.output_text,
                context_tokens:       r.context_tokens.map(|n| n as u32),
                output_tokens:        r.output_tokens.map(|n| n as u32),
                duration_ms:          r.duration_ms.map(|n| n as u64),
                status:               parse_ai_call_status(&r.status)?,
                error_message:        r.error_message,
                created_at:           str_to_ts(&r.created_at)?,
                pre_edit_snapshot_id: r.pre_edit_snapshot_id.as_deref().map(str_to_ulid).transpose()?,
                applied_at:           r.applied_at.as_deref().map(str_to_ts).transpose()?,
            })),
        }
    }

    // ── Scene content bulk + export ledger ────────────────────────────────

    async fn list_all_scene_content(&self) -> Result<Vec<SceneContent>, StorageError> {
        let rows = sqlx::query!(
            r#"
            SELECT node_id, pm_doc, word_count, char_count, hash, updated_at
            FROM scene_content
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| Ok(SceneContent {
                node_id:    str_to_ulid(&r.node_id)?,
                pm_doc:     serde_json::from_str(&r.pm_doc)?,
                word_count: r.word_count as u32,
                char_count: r.char_count as u32,
                hash:       r.hash,
                updated_at: str_to_ts(&r.updated_at)?,
            }))
            .collect()
    }

    async fn list_nodes_with_scene_content_consistent(
        &self,
    ) -> Result<(Vec<Node>, Vec<SceneContent>), StorageError> {
        // BEGIN IMMEDIATE acquires a reserved lock immediately; subsequent
        // writers wait until we COMMIT.  That guarantees both reads observe
        // the same state.  We use a regular `begin()` here because sqlx's
        // SQLite driver opens transactions in IMMEDIATE mode by default
        // when the connection runs in WAL with `journal_mode=WAL` and the
        // first statement is a SELECT — but to make the intent explicit
        // and bind to documented semantics we issue `BEGIN IMMEDIATE` by
        // hand.
        let mut conn = self.pool.acquire().await?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

        // Wrap the actual reads so an early return can still ROLLBACK.
        let result: Result<(Vec<Node>, Vec<SceneContent>), StorageError> = async {
            let node_rows = sqlx::query!(
                r#"
                SELECT id, parent_id, kind, title, position, status,
                       pov, beat, target_words, created_at, updated_at, deleted_at
                FROM nodes
                WHERE deleted_at IS NULL
                ORDER BY position ASC
                "#
            )
            .fetch_all(&mut *conn)
            .await?;

            let nodes: Vec<Node> = node_rows.into_iter()
                .map(|r| Ok::<_, StorageError>(Node {
                    id:           str_to_ulid(&r.id)?,
                    parent_id:    r.parent_id.as_deref().map(str_to_ulid).transpose()?,
                    kind:         parse_node_kind(&r.kind)?,
                    title:        r.title,
                    position:     r.position,
                    status:       parse_node_status(&r.status)?,
                    pov:          r.pov,
                    beat:         r.beat,
                    target_words: r.target_words.map(|v| v as u32),
                    created_at:   str_to_ts(&r.created_at)?,
                    updated_at:   str_to_ts(&r.updated_at)?,
                    deleted_at:   r.deleted_at.as_deref().map(str_to_ts).transpose()?,
                }))
                .collect::<Result<_, _>>()?;

            let scene_rows = sqlx::query!(
                r#"
                SELECT node_id, pm_doc, word_count, char_count, hash, updated_at
                FROM scene_content
                "#
            )
            .fetch_all(&mut *conn)
            .await?;

            let scenes: Vec<SceneContent> = scene_rows.into_iter()
                .map(|r| Ok::<_, StorageError>(SceneContent {
                    node_id:    str_to_ulid(&r.node_id)?,
                    pm_doc:     serde_json::from_str(&r.pm_doc)?,
                    word_count: r.word_count as u32,
                    char_count: r.char_count as u32,
                    hash:       r.hash,
                    updated_at: str_to_ts(&r.updated_at)?,
                }))
                .collect::<Result<_, _>>()?;

            Ok((nodes, scenes))
        }.await;

        // Always commit — we only ever read, so commit-or-rollback is
        // semantically equivalent on success; on error we rollback so the
        // immediate-lock is released cleanly.
        match &result {
            Ok(_)  => { let _ = sqlx::query("COMMIT").execute(&mut *conn).await; }
            Err(_) => { let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await; }
        }
        result
    }

    async fn export_insert(&self, record: &ExportRecord) -> Result<(), StorageError> {
        let id         = ulid_to_str(record.id);
        let profile    = export_profile_str(record.profile);
        let created_at = ts_to_str(record.created_at);
        sqlx::query!(
            r#"
            INSERT INTO exports (id, profile, output_path, hash, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
            id, profile, record.output_path, record.hash, created_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_exports(&self) -> Result<Vec<ExportRecord>, StorageError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, profile, output_path, hash, created_at
            FROM exports
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| Ok(ExportRecord {
                id:          str_to_ulid(&r.id)?,
                profile:     parse_export_profile(&r.profile)?,
                output_path: r.output_path,
                hash:        r.hash,
                created_at:  str_to_ts(&r.created_at)?,
            }))
            .collect()
    }

    // ── Memory ledger ─────────────────────────────────────────────────────

    async fn memory_upsert(&self, entry: &MemoryEntry) -> Result<(), StorageError> {
        let id          = ulid_to_str(entry.id);
        let scope       = memory_scope_str(entry.scope);
        let value_str   = serde_json::to_string(&entry.value_json)?;
        let created_at  = ts_to_str(entry.created_at);
        let updated_at  = ts_to_str(entry.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO memory_entries
                (id, scope, key, value_json, agent_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(scope, key) DO UPDATE SET
                value_json = excluded.value_json,
                agent_id   = excluded.agent_id,
                updated_at = excluded.updated_at
            "#,
            id, scope, entry.key, value_str, entry.agent_id, created_at, updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn memory_get(
        &self,
        scope: MemoryScope,
        key:   &str,
    ) -> Result<Option<MemoryEntry>, StorageError> {
        let scope_str = memory_scope_str(scope);
        let row = sqlx::query!(
            r#"
            SELECT id, scope, key, value_json, agent_id, created_at, updated_at
            FROM memory_entries
            WHERE scope = ? AND key = ?
            "#,
            scope_str, key,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some(MemoryEntry {
                id:         str_to_ulid(&r.id)?,
                scope:      parse_memory_scope(&r.scope)?,
                key:        r.key,
                value_json: serde_json::from_str(&r.value_json)?,
                agent_id:   r.agent_id,
                created_at: str_to_ts(&r.created_at)?,
                updated_at: str_to_ts(&r.updated_at)?,
            })),
        }
    }

    async fn memory_list_by_scope(
        &self,
        scope: MemoryScope,
    ) -> Result<Vec<MemoryEntry>, StorageError> {
        let scope_str = memory_scope_str(scope);
        let rows = sqlx::query!(
            r#"
            SELECT id, scope, key, value_json, agent_id, created_at, updated_at
            FROM memory_entries
            WHERE scope = ?
            ORDER BY key ASC
            "#,
            scope_str,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| Ok(MemoryEntry {
                id:         str_to_ulid(&r.id)?,
                scope:      parse_memory_scope(&r.scope)?,
                key:        r.key,
                value_json: serde_json::from_str(&r.value_json)?,
                agent_id:   r.agent_id,
                created_at: str_to_ts(&r.created_at)?,
                updated_at: str_to_ts(&r.updated_at)?,
            }))
            .collect()
    }

    async fn memory_delete(
        &self,
        scope: MemoryScope,
        key:   &str,
    ) -> Result<u32, StorageError> {
        let scope_str = memory_scope_str(scope);
        let result = sqlx::query!(
            "DELETE FROM memory_entries WHERE scope = ? AND key = ?",
            scope_str, key,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as u32)
    }

    // ── Vocabulary ledger ─────────────────────────────────────────────────

    async fn vocab_upsert(&self, entry: &VocabEntry) -> Result<(), StorageError> {
        let id         = ulid_to_str(entry.id);
        let kind       = entry.kind.as_str();
        let source     = entry.source.as_str();
        let created_at = ts_to_str(entry.created_at);
        let updated_at = ts_to_str(entry.updated_at);
        sqlx::query!(
            r#"
            INSERT INTO vocab_entries
                (id, layer, term, display_term, kind, replacement, rationale,
                 source, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(layer, term, kind) DO UPDATE SET
                display_term = excluded.display_term,
                replacement  = excluded.replacement,
                rationale    = excluded.rationale,
                source       = excluded.source,
                updated_at   = excluded.updated_at
            "#,
            id, entry.layer, entry.term, entry.display_term, kind,
            entry.replacement, entry.rationale, source, created_at, updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn vocab_seed_starters(&self, entries: &[VocabEntry]) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        // Wipe shipped rows first; user/agent rows persist.
        sqlx::query!("DELETE FROM vocab_entries WHERE source = 'starter'")
            .execute(&mut *tx)
            .await?;
        for entry in entries {
            let id         = ulid_to_str(entry.id);
            let kind       = entry.kind.as_str();
            let source     = entry.source.as_str();
            let created_at = ts_to_str(entry.created_at);
            let updated_at = ts_to_str(entry.updated_at);
            sqlx::query!(
                r#"
                INSERT INTO vocab_entries
                    (id, layer, term, display_term, kind, replacement, rationale,
                     source, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                id, entry.layer, entry.term, entry.display_term, kind,
                entry.replacement, entry.rationale, source, created_at, updated_at,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn vocab_list_by_layers(
        &self,
        layers: &[&str],
    ) -> Result<Vec<VocabEntry>, StorageError> {
        if layers.is_empty() { return Ok(Vec::new()); }

        // Pull the full set in one shot, then filter in-memory — `IN (?, ?, ?)`
        // bindings are awkward with `sqlx::query!` because the slot count
        // must be statically known.  Vocab rows are ~hundreds, never enough
        // to justify dynamic SQL.
        let rows = sqlx::query!(
            r#"
            SELECT id, layer, term, display_term, kind, replacement, rationale,
                   source, created_at, updated_at
            FROM vocab_entries
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .filter(|r| layers.iter().any(|l| *l == r.layer))
            .map(|r| Ok(VocabEntry {
                id:           str_to_ulid(&r.id)?,
                layer:        r.layer,
                term:         r.term,
                display_term: r.display_term,
                kind:         parse_entry_kind(&r.kind)?,
                replacement:  r.replacement,
                rationale:    r.rationale,
                source:       parse_entry_source(&r.source)?,
                created_at:   str_to_ts(&r.created_at)?,
                updated_at:   str_to_ts(&r.updated_at)?,
            }))
            .collect()
    }

    async fn vocab_count_by_layer(&self, layer: &str) -> Result<u32, StorageError> {
        let row = sqlx::query!(
            "SELECT COUNT(*) AS c FROM vocab_entries WHERE layer = ?",
            layer,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.c as u32)
    }

    // ── Validator ledger (Phase 4) ────────────────────────────────────────

    async fn validator_run_persist(
        &self,
        run:    &ValidatorRun,
        issues: &[ValidatorIssue],
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;

        let run_id     = ulid_to_str(run.id);
        let ran_at     = ts_to_str(run.ran_at);
        let status     = run.status.as_str();
        let duration   = run.duration_ms as i64;
        sqlx::query!(
            r#"
            INSERT INTO validator_runs
                (id, validator_id, ran_at, status, duration_ms, scope_hash)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            run_id, run.validator_id, ran_at, status, duration, run.scope_hash,
        )
        .execute(&mut *tx)
        .await?;

        for issue in issues {
            let issue_id = ulid_to_str(Ulid::new());
            let node_id  = issue.node_id.map(ulid_to_str);
            let severity = issue.severity.as_str();
            let off_from = issue.offset_from.map(|n| n as i64);
            let off_to   = issue.offset_to.map(|n| n as i64);
            let auto_fix = issue.auto_fixable as i64;
            sqlx::query!(
                r#"
                INSERT INTO validator_issues
                    (id, run_id, node_id, severity, code, message,
                     offset_from, offset_to, auto_fixable)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                issue_id, run_id, node_id, severity, issue.code, issue.message,
                off_from, off_to, auto_fix,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn latest_validator_run(&self) -> Result<Option<ValidatorRun>, StorageError> {
        let row = sqlx::query!(
            r#"
            SELECT id, validator_id, ran_at, status, duration_ms, scope_hash
            FROM validator_runs
            ORDER BY ran_at DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some(ValidatorRun {
                id:           str_to_ulid(&r.id)?,
                validator_id: r.validator_id,
                ran_at:       str_to_ts(&r.ran_at)?,
                status:       parse_run_status(&r.status)?,
                duration_ms:  r.duration_ms as u64,
                scope_hash:   r.scope_hash,
            })),
        }
    }

    async fn list_validator_issues_for_run(
        &self,
        run_id: Ulid,
    ) -> Result<Vec<ValidatorIssue>, StorageError> {
        let id_str = ulid_to_str(run_id);
        let rows = sqlx::query!(
            r#"
            SELECT node_id, severity, code, message, offset_from, offset_to,
                   auto_fixable
            FROM validator_issues
            WHERE run_id = ?
            ORDER BY severity DESC, code ASC
            "#,
            id_str,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| Ok(ValidatorIssue {
                // The schema doesn't store a per-issue validator_id; we
                // derive a stable surrogate from the issue's `code` so the
                // UI can group by it consistently.
                validator_id: r.code.clone(),
                code:         r.code,
                severity:     parse_severity(&r.severity)?,
                message:      r.message,
                node_id:      r.node_id.as_deref().map(str_to_ulid).transpose()?,
                offset_from:  r.offset_from.map(|n| n as u32),
                offset_to:    r.offset_to.map(|n| n as u32),
                auto_fixable: r.auto_fixable != 0,
            }))
            .collect()
    }
}

// ── String ↔ enum helpers ─────────────────────────────────────────────────────

fn node_kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Project     => "project",
        NodeKind::Part        => "part",
        NodeKind::Chapter     => "chapter",
        NodeKind::Scene       => "scene",
        NodeKind::FrontMatter => "front_matter",
        NodeKind::BackMatter  => "back_matter",
    }
}

fn parse_node_kind(s: &str) -> Result<NodeKind, StorageError> {
    match s {
        "project"      => Ok(NodeKind::Project),
        "part"         => Ok(NodeKind::Part),
        "chapter"      => Ok(NodeKind::Chapter),
        "scene"        => Ok(NodeKind::Scene),
        "front_matter" => Ok(NodeKind::FrontMatter),
        "back_matter"  => Ok(NodeKind::BackMatter),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown node kind: {other}"),
        }),
    }
}

fn node_status_str(s: NodeStatus) -> &'static str {
    match s {
        NodeStatus::Planned  => "planned",
        NodeStatus::Drafting => "drafting",
        NodeStatus::Revised  => "revised",
        NodeStatus::Final    => "final",
    }
}

fn parse_node_status(s: &str) -> Result<NodeStatus, StorageError> {
    match s {
        "planned"  => Ok(NodeStatus::Planned),
        "drafting" => Ok(NodeStatus::Drafting),
        "revised"  => Ok(NodeStatus::Revised),
        "final"    => Ok(NodeStatus::Final),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown node status: {other}"),
        }),
    }
}

fn entity_kind_str(k: EntityKind) -> &'static str {
    match k {
        EntityKind::Character    => "character",
        EntityKind::Location     => "location",
        EntityKind::Item         => "item",
        EntityKind::Organisation => "organisation",
        EntityKind::Theme        => "theme",
        EntityKind::Custom       => "custom",
    }
}

fn parse_entity_kind(s: &str) -> Result<EntityKind, StorageError> {
    match s {
        "character"    => Ok(EntityKind::Character),
        "location"     => Ok(EntityKind::Location),
        "item"         => Ok(EntityKind::Item),
        "organisation" => Ok(EntityKind::Organisation),
        "theme"        => Ok(EntityKind::Theme),
        "custom"       => Ok(EntityKind::Custom),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown entity kind: {other}"),
        }),
    }
}

fn em_dash_str(e: EmDash) -> &'static str {
    match e { EmDash::Em => "em", EmDash::En => "en", EmDash::Hyphen => "hyphen" }
}

fn parse_em_dash(s: &str) -> Result<EmDash, StorageError> {
    match s {
        "em"     => Ok(EmDash::Em),
        "en"     => Ok(EmDash::En),
        "hyphen" => Ok(EmDash::Hyphen),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown em_dash value: {other}"),
        }),
    }
}

fn quote_style_str(q: QuoteStyle) -> &'static str {
    match q { QuoteStyle::Smart => "smart", QuoteStyle::Straight => "straight" }
}

fn parse_quote_style(s: &str) -> Result<QuoteStyle, StorageError> {
    match s {
        "smart"    => Ok(QuoteStyle::Smart),
        "straight" => Ok(QuoteStyle::Straight),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown quote_style: {other}"),
        }),
    }
}

fn ellipsis_str(e: EllipsisForm) -> &'static str {
    match e {
        EllipsisForm::SingleGlyph => "single_glyph",
        EllipsisForm::ThreeDots   => "three_dots",
    }
}

fn parse_ellipsis(s: &str) -> Result<EllipsisForm, StorageError> {
    match s {
        "single_glyph" => Ok(EllipsisForm::SingleGlyph),
        "three_dots"   => Ok(EllipsisForm::ThreeDots),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown ellipsis_form: {other}"),
        }),
    }
}

fn snapshot_scope_str(s: SnapshotScope) -> &'static str {
    match s {
        SnapshotScope::Project => "project",
        SnapshotScope::Part    => "part",
        SnapshotScope::Chapter => "chapter",
        SnapshotScope::Scene   => "scene",
    }
}

fn parse_snapshot_scope(s: &str) -> Result<SnapshotScope, StorageError> {
    match s {
        "project" => Ok(SnapshotScope::Project),
        "part"    => Ok(SnapshotScope::Part),
        "chapter" => Ok(SnapshotScope::Chapter),
        "scene"   => Ok(SnapshotScope::Scene),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown snapshot scope: {other}"),
        }),
    }
}

fn snapshot_trigger_str(t: SnapshotTrigger) -> &'static str {
    match t {
        SnapshotTrigger::Manual        => "manual",
        SnapshotTrigger::Auto          => "auto",
        SnapshotTrigger::PreAi         => "pre_ai",
        SnapshotTrigger::PreExport     => "pre_export",
        SnapshotTrigger::PreMigration  => "pre_migration",
        SnapshotTrigger::PreAgentEdit  => "pre_agent_edit",
        SnapshotTrigger::PreRestore    => "pre_restore",
        SnapshotTrigger::CrashRecovery => "crash_recovery",
    }
}

fn parse_snapshot_trigger(s: &str) -> Result<SnapshotTrigger, StorageError> {
    match s {
        "manual"         => Ok(SnapshotTrigger::Manual),
        "auto"           => Ok(SnapshotTrigger::Auto),
        "pre_ai"         => Ok(SnapshotTrigger::PreAi),
        "pre_export"     => Ok(SnapshotTrigger::PreExport),
        "pre_migration"  => Ok(SnapshotTrigger::PreMigration),
        "pre_agent_edit" => Ok(SnapshotTrigger::PreAgentEdit),
        "pre_restore"    => Ok(SnapshotTrigger::PreRestore),
        "crash_recovery" => Ok(SnapshotTrigger::CrashRecovery),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown snapshot trigger: {other}"),
        }),
    }
}

fn applied_edit_kind_str(k: AppliedEditKind) -> &'static str { k.as_str() }

fn parse_applied_edit_kind(s: &str) -> Result<AppliedEditKind, StorageError> {
    AppliedEditKind::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown agent_applied_edit kind: {s}"),
    })
}

fn quick_action_preset_str(p: QuickActionPreset) -> &'static str { p.as_str() }

fn parse_quick_action_preset(s: &str) -> Result<QuickActionPreset, StorageError> {
    QuickActionPreset::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown quick-action preset: {s}"),
    })
}

fn ai_call_status_str(s: AiCallStatus) -> &'static str { s.as_str() }

fn parse_ai_call_status(s: &str) -> Result<AiCallStatus, StorageError> {
    AiCallStatus::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown ai_call status: {s}"),
    })
}

fn export_profile_str(p: ExportProfile) -> &'static str { p.as_str() }

fn parse_export_profile(s: &str) -> Result<ExportProfile, StorageError> {
    ExportProfile::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown export profile: {s}"),
    })
}

fn memory_scope_str(s: MemoryScope) -> &'static str { s.as_str() }

fn parse_memory_scope(s: &str) -> Result<MemoryScope, StorageError> {
    MemoryScope::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown memory scope: {s}"),
    })
}

fn parse_entry_kind(s: &str) -> Result<EntryKind, StorageError> {
    EntryKind::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown vocab entry kind: {s}"),
    })
}

fn parse_entry_source(s: &str) -> Result<EntrySource, StorageError> {
    EntrySource::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown vocab entry source: {s}"),
    })
}

fn parse_severity(s: &str) -> Result<Severity, StorageError> {
    Severity::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown validator severity: {s}"),
    })
}

fn parse_run_status(s: &str) -> Result<ValidatorRunStatus, StorageError> {
    ValidatorRunStatus::from_str(s).ok_or_else(|| StorageError::ConstraintViolation {
        detail: format!("unknown validator run status: {s}"),
    })
}
