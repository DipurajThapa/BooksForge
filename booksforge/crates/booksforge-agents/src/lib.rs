//! Agent registry and spec types (Layer 3 — pure logic).
//!
//! Agents are prompt-in / schema-out units. They do not perform I/O,
//! call tools, or invoke other agents. The Orchestrator (Layer 4) is the sole
//! controller that sequences agents and applies their proposals.
//!
//! Prompt templates are implemented in M5 (`booksforge-prompt`).

#![forbid(unsafe_code)]

pub mod registry;
pub mod spec;

pub use registry::{find_agent, MVP_AGENTS};
pub use spec::AgentSpec;
