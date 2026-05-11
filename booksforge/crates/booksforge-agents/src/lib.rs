//! Agent registry and spec types (Layer 3 — pure logic).
//!
//! Agents are prompt-in / schema-out units. They do not perform I/O,
//! call tools, or invoke other agents. The Orchestrator (Layer 4) is the sole
//! controller that sequences agents and applies their proposals.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod audience_mapper;
pub mod chapter_drafter;
pub mod chapter_drafter_nf;
pub mod character_bible;
pub mod character_bible_card;
pub mod character_critic;
pub mod concept_scorer;
pub mod continuity;
pub mod copyeditor;
pub mod dev_editor;
pub mod dialogue_polish;
pub mod final_review_editor;
pub mod humanization;
pub mod intake;
pub mod json_repair;
pub mod memory_curator;
pub mod metaphor_polish;
pub mod outline_architect;
pub mod peer_review;
pub mod polish_common;
pub mod proposal_validator;
pub mod registry;
pub mod scene_critic;
pub mod scene_drafter_fic;
pub mod scene_planner;
pub mod scene_tension_polish;
pub mod spec;
pub mod structure_critic;
pub mod vocab_dictionary;
pub mod voice_polish;
pub mod world_bible;

pub use registry::{find_agent, mvp_agents};
pub use spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun, STD_VALIDATORS,
};
