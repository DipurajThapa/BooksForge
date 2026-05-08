//! MVP agent registry.
//!
//! Per AGENTS.md §2: nine LLM agents in MVP plus the Final Review Editor
//! (which is the high-end polish agent introduced in this project).  The
//! `proposal-validator` is internal/orchestrator-grade and is registered
//! here so it can be looked up by id, but is excluded from `mvp_agents()`
//! which feeds the user-visible catalog.

use crate::spec::AgentSpec;

/// Build the user-visible MVP agent catalog (10 agents incl. FRE).
pub fn mvp_agents() -> Vec<AgentSpec> {
    vec![
        crate::intake::spec(),
        crate::outline_architect::spec(),
        crate::memory_curator::spec(),
        crate::vocab_dictionary::spec(),
        crate::chapter_drafter::spec(),
        crate::dev_editor::spec(),
        crate::continuity::spec(),
        crate::copyeditor::spec(),
        crate::humanization::spec(),
        crate::final_review_editor::spec(),
    ]
}

/// All registered agents including internal / orchestrator-grade ones.
fn all_agents() -> Vec<AgentSpec> {
    let mut v = mvp_agents();
    v.push(crate::proposal_validator::spec());
    v
}

/// Look up a spec by its stable `id` (searches all registered agents).
pub fn find_agent(id: &str) -> Option<AgentSpec> {
    all_agents().into_iter().find(|a| a.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ModelSizeHint, UserGate};

    #[test]
    fn ten_mvp_agents_present() {
        assert_eq!(mvp_agents().len(), 10);
    }

    #[test]
    fn agent_ids_are_unique_across_all() {
        let agents = all_agents();
        let mut ids: Vec<_> = agents.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), agents.len(), "duplicate agent ids");
    }

    #[test]
    fn every_agent_has_at_least_three_validators() {
        for a in all_agents() {
            assert!(
                a.validators.len() >= 3,
                "agent '{}' has fewer than 3 cross-cutting validators",
                a.id
            );
        }
    }

    #[test]
    fn every_agent_has_failure_modes_documented() {
        for a in all_agents() {
            assert!(
                !a.failure_modes.is_empty(),
                "agent '{}' has no documented failure modes",
                a.id
            );
        }
    }

    #[test]
    fn every_agent_has_input_and_output_schema_ids() {
        for a in all_agents() {
            assert!(!a.input_schema_id.is_empty(),  "agent '{}' missing input_schema_id",  a.id);
            assert!(!a.output_schema_id.is_empty(), "agent '{}' missing output_schema_id", a.id);
        }
    }

    #[test]
    fn final_review_editor_pinned_to_qwen36() {
        let s = find_agent("final-review-editor").expect("FRE present");
        assert_eq!(s.pinned_model, Some("qwen3.6:latest"));
        assert_eq!(s.model_preference.min_size, ModelSizeHint::ExtraLarge);
    }

    #[test]
    fn proposal_validator_is_findable_but_not_in_catalog() {
        assert!(find_agent("proposal-validator").is_some());
        assert!(mvp_agents().iter().all(|a| a.id != "proposal-validator"));
    }

    #[test]
    fn memory_curator_is_scheduled_not_user_gated() {
        let s = find_agent("memory-curator").expect("memory-curator present");
        assert_eq!(s.user_gate, UserGate::NotRequired);
    }
}
