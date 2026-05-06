//! `StorageRepository` trait — the Layer 3 ↔ Layer 4 boundary for SQLite.
//!
//! All methods are async and return typed errors.  The trait is object-safe
//! (via `async_trait`) so it can be stored as `Arc<dyn StorageRepository>`.

use async_trait::async_trait;
use booksforge_domain::{Entity, Node, SceneContent, SnapshotRecord, StyleBook};
use ulid::Ulid;

use crate::StorageError;

#[async_trait]
pub trait StorageRepository: Send + Sync {
    // ── Nodes ─────────────────────────────────────────────────────────────

    /// Return all non-deleted nodes ordered by position (LexoRank asc).
    async fn list_nodes(&self) -> Result<Vec<Node>, StorageError>;

    /// Insert a new node row.
    async fn insert_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Update mutable fields of an existing node.
    async fn update_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Soft-delete a node (sets `deleted_at = now`).
    async fn delete_node(&self, id: Ulid) -> Result<(), StorageError>;

    // ── Scene content ──────────────────────────────────────────────────────

    /// Load scene content for a node.  Returns `None` if never saved.
    async fn load_scene(&self, node_id: Ulid) -> Result<Option<SceneContent>, StorageError>;

    /// Upsert scene content (INSERT OR REPLACE).
    async fn save_scene(&self, content: &SceneContent) -> Result<(), StorageError>;

    // ── Style book ─────────────────────────────────────────────────────────

    /// Read the singleton style-book row.  Returns `StyleBook::default()` if
    /// the row does not exist yet.
    async fn load_style_book(&self) -> Result<StyleBook, StorageError>;

    /// Upsert the singleton style-book row.
    async fn save_style_book(&self, style: &StyleBook) -> Result<(), StorageError>;

    // ── Entities ───────────────────────────────────────────────────────────

    /// List all non-deleted entities with aliases populated.
    async fn list_entities(&self) -> Result<Vec<Entity>, StorageError>;

    /// Insert a new entity and its aliases atomically.
    async fn insert_entity(&self, entity: &Entity) -> Result<(), StorageError>;

    /// Soft-delete an entity.
    async fn delete_entity(&self, id: Ulid) -> Result<(), StorageError>;

    // ── Snapshots ──────────────────────────────────────────────────────────

    /// Insert a snapshot manifest row.
    async fn insert_snapshot(&self, snap: &SnapshotRecord) -> Result<(), StorageError>;

    /// List snapshot records newest-first, optionally filtered by scope_id.
    async fn list_snapshots(
        &self,
        scope_id: Option<Ulid>,
    ) -> Result<Vec<SnapshotRecord>, StorageError>;
}
