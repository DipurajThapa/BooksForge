//! Agent registry and spec types (Layer 3 — pure logic).
//!
//! Agents are prompt-in / schema-out units. They do not perform I/O,
//! call tools, or invoke other agents. The Orchestrator (Layer 4) is the sole
//! controller that sequences agents and applies their proposals.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod chapter_drafter;
pub mod continuity;
pub mod copyeditor;
pub mod dev_editor;
pub mod final_review_editor;
pub mod humanization;
pub mod intake;
pub mod memory_curator;
pub mod outline_architect;
pub mod proposal_validator;
pub mod peer_review;
pub mod registry;
pub mod spec;
pub mod vocab_dictionary;

pub use registry::{find_agent, mvp_agents};
pub use spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, FailureMode, ModelFamily, ModelPreference,
    ModelSizeHint, UserGate, WhenToRun, STD_VALIDATORS,
};
