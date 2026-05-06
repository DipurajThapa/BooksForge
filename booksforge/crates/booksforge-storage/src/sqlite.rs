//! `SqliteStorage` — production implementation of `StorageRepository`.
//!
//! All SQL goes through `sqlx::query!` macros for compile-time checking.
//! IDs are stored as ULID strings; timestamps as ISO-8601 UTC text.

use async_trait::async_trait;
use booksforge_domain::{
    EllipsisForm, EmDash, Entity, EntityKind, Node, NodeKind, NodeStatus, QuoteStyle,
    SceneContent, SnapshotRecord, SnapshotScope, SnapshotTrigger, StyleBook,
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
        sqlx::query!(
            r#"
            INSERT INTO nodes
                (id, parent_id, kind, title, position, status,
                 pov, beat, target_words, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            ulid_to_str(node.id),
            node.parent_id.map(ulid_to_str),
            node_kind_str(node.kind),
            node.title,
            node.position,
            node_status_str(node.status),
            node.pov,
            node.beat,
            node.target_words,
            ts_to_str(node.created_at),
            ts_to_str(node.updated_at),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_node(&self, node: &Node) -> Result<(), StorageError> {
        sqlx::query!(
            r#"
            UPDATE nodes
            SET title = ?, position = ?, status = ?,
                pov = ?, beat = ?, target_words = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            node.title,
            node.position,
            node_status_str(node.status),
            node.pov,
            node.beat,
            node.target_words,
            ts_to_str(node.updated_at),
            ulid_to_str(node.id),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_node(&self, id: Ulid) -> Result<(), StorageError> {
        let now = ts_to_str(Utc::now());
        sqlx::query!(
            "UPDATE nodes SET deleted_at = ? WHERE id = ?",
            now,
            ulid_to_str(id),
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
        let pm_str = serde_json::to_string(&content.pm_doc)?;
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
            ulid_to_str(content.node_id),
            pm_str,
            content.word_count as i64,
            content.char_count as i64,
            content.hash,
            ts_to_str(content.updated_at),
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
        let now = ts_to_str(Utc::now());
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
            em_dash_str(style.em_dash),
            style.oxford_comma as i64,
            quote_style_str(style.quote_style),
            style.spaces_after_period as i64,
            ellipsis_str(style.ellipsis_form),
            style.spelling_locale,
            style.capitalize_after_colon as i64,
            style.bold_emphasis_allowed as i64,
            now,
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
        let id_str = ulid_to_str(entity.id);
        sqlx::query!(
            r#"
            INSERT INTO entities
                (id, kind, name, fields_json, notes, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            id_str,
            entity_kind_str(entity.kind),
            entity.name,
            fields_str,
            entity.notes,
            ts_to_str(entity.created_at),
            ts_to_str(entity.updated_at),
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
        sqlx::query!(
            "UPDATE entities SET deleted_at = ? WHERE id = ?",
            now,
            ulid_to_str(id),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Snapshots ──────────────────────────────────────────────────────────

    async fn insert_snapshot(&self, snap: &SnapshotRecord) -> Result<(), StorageError> {
        sqlx::query!(
            r#"
            INSERT INTO snapshots
                (id, scope, scope_id, label, trigger, tree_hash, created_at, size_bytes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            ulid_to_str(snap.id),
            snapshot_scope_str(snap.scope),
            snap.scope_id.map(ulid_to_str),
            snap.label,
            snapshot_trigger_str(snap.trigger),
            snap.tree_hash,
            ts_to_str(snap.created_at),
            snap.size_bytes as i64,
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
        SnapshotTrigger::Manual       => "manual",
        SnapshotTrigger::Auto         => "auto",
        SnapshotTrigger::PreAi        => "pre_ai",
        SnapshotTrigger::PreExport    => "pre_export",
        SnapshotTrigger::PreMigration => "pre_migration",
        SnapshotTrigger::PreAgentEdit => "pre_agent_edit",
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
        "crash_recovery" => Ok(SnapshotTrigger::CrashRecovery),
        other => Err(StorageError::ConstraintViolation {
            detail: format!("unknown snapshot trigger: {other}"),
        }),
    }
}
