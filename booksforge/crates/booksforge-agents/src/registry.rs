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

/// All registered agents including internal / orchestrator-grade ones and
/// the non-fiction sibling of `chapter-drafter` (which is selected by mode
/// at orchestration time rather than appearing in the user-visible catalog).
fn all_agents() -> Vec<AgentSpec> {
    let mut v = mvp_agents();
    v.push(crate::proposal_validator::spec());
    v.push(crate::chapter_drafter_nf::spec());
    // Fiction-shaped first-class bibles + scene drafter (BACKLOG §A13).
    // Selected by mode at orchestration time; not in the user-visible catalog.
    v.push(crate::character_bible::spec());
    // Round 7 — per-character chunked variant. Same output schema in
    // aggregate, but called N times by the orchestrator's chunked
    // helper to fit small-model competence.
    v.push(crate::character_bible_card::spec());
    v.push(crate::world_bible::spec());
    v.push(crate::scene_drafter_fic::spec());
    // Specialist polish stack + scene critic (BACKLOG §A15 / Phase 2 of
    // PRODUCT_ROADMAP_E2E.md). Sequenced by `run_polish_stack`; not in
    // the user-visible catalog (UI exposes the stack as a single action).
    v.push(crate::dialogue_polish::spec());
    v.push(crate::metaphor_polish::spec());
    v.push(crate::voice_polish::spec());
    v.push(crate::scene_tension_polish::spec());
    v.push(crate::scene_critic::spec());
    // Adaptive polish planner (Item 4 of FEATURE_HARDENING_PLAN).
    // Reads VoiceScore.failed_dimensions + tells report and emits a
    // PolishPlan that the orchestrator executes instead of the
    // genre-pack's static polish-stage order.
    v.push(crate::scene_planner::spec());
    // Phase C quality gates — score the writer's inputs and propose
    // targeted revisions. concept_scorer is Stage 1's gate.
    v.push(crate::concept_scorer::spec());
    v.push(crate::audience_mapper::spec());
    v.push(crate::character_critic::spec());
    v.push(crate::structure_critic::spec());
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
            assert!(
                !a.input_schema_id.is_empty(),
                "agent '{}' missing input_schema_id",
                a.id
            );
            assert!(
                !a.output_schema_id.is_empty(),
                "agent '{}' missing output_schema_id",
                a.id
            );
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

    #[test]
    fn chapter_drafter_nf_is_findable_but_not_in_catalog() {
        // The non-fiction drafter is selected by ProjectBrief.mode at
        // orchestration time, not from the user-visible catalog.
        assert!(find_agent("chapter-drafter-nf").is_some());
        assert!(mvp_agents().iter().all(|a| a.id != "chapter-drafter-nf"));
    }

    #[test]
    fn fiction_and_nf_drafters_share_io_schema() {
        let f = find_agent("chapter-drafter").expect("fiction drafter");
        let nf = find_agent("chapter-drafter-nf").expect("non-fiction drafter");
        assert_eq!(f.input_schema_id, nf.input_schema_id);
        assert_eq!(f.output_schema_id, nf.output_schema_id);
    }

    #[test]
    fn every_agent_declares_default_thinking() {
        // Compile-time guarantee that the field exists on every spec.
        for a in all_agents() {
            let _ = a.default_thinking;
        }
    }

    #[test]
    fn reasoning_agents_default_to_thinking_enabled() {
        // Note: `peer-review` is per-agent council scaffolding, not a
        // standalone registered agent, so it is intentionally excluded here
        // even though its spec also declares `DefaultThinking::Enabled`.
        use crate::spec::DefaultThinking;
        for id in ["dev-editor", "continuity", "proposal-validator"] {
            let s = find_agent(id).unwrap_or_else(|| panic!("{id} present"));
            assert_eq!(
                s.default_thinking,
                DefaultThinking::Enabled,
                "{id} should default to thinking-enabled"
            );
        }
    }

    #[test]
    fn prose_agents_default_to_thinking_disabled() {
        use crate::spec::DefaultThinking;
        for id in [
            "intake",
            "outline-architect",
            "chapter-drafter",
            "chapter-drafter-nf",
            "copyeditor",
            "humanization",
            "final-review-editor",
            "memory-curator",
            "vocab-dictionary",
        ] {
            let s = find_agent(id).unwrap_or_else(|| panic!("{id} present"));
            assert_eq!(
                s.default_thinking,
                DefaultThinking::Disabled,
                "{id} should default to thinking-disabled (Qwen 3.x footgun)"
            );
        }
    }
}
