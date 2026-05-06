//! Domain layer — pure logic, no I/O, no clocks, no randomness.
//!
//! All types here are value objects or pure-function modules.  Any timestamp
//! or ID that needs to be "now" or "new" is passed in by the caller so that
//! tests can use deterministic values.

#![forbid(unsafe_code)]

pub mod entity;
pub mod error;
pub mod node;
pub mod project;
pub mod settings;
pub mod snapshot;
pub mod style;

pub use entity::{Entity, EntityKind};
pub use error::DomainError;
pub use node::{Node, NodeKind, NodeStatus, SceneContent};
pub use project::{BookMode, Project, ProjectMeta};
pub use settings::{OllamaSettings, RecentProject, RecentProjectsList, UiSettings, UserSettings};
pub use snapshot::{SnapshotRecord, SnapshotScope, SnapshotTrigger};
pub use style::{EllipsisForm, EmDash, QuoteStyle, StyleBook};
