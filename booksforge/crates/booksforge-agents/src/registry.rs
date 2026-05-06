use crate::spec::{AgentSpec, UserGate, WhenToRun};

/// All nine MVP LLM agents. Order matches AGENTS.md §2.
pub const MVP_AGENTS: &[AgentSpec] = &[
    AgentSpec {
        id:           "intake",
        name:         "Project Intake",
        purpose:      "Turn a free-text idea into a structured project brief.",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
    AgentSpec {
        id:           "outline-architect",
        name:         "Outline Architect",
        purpose:      "Propose a chapter/scene outline from a brief.",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
    AgentSpec {
        id:           "memory-curator",
        name:         "Memory Curator",
        purpose:      "Maintain book/chapter/entity memory; refresh summaries on chapter finalise.",
        when_to_run:  WhenToRun::Scheduled,
        user_gate:    UserGate::NotRequired,
    },
    AgentSpec {
        id:           "vocab-dictionary",
        name:         "Vocabulary Dictionary",
        purpose:      "Maintain project-layer vocabulary dictionaries from accepted edits.",
        when_to_run:  WhenToRun::Scheduled,
        user_gate:    UserGate::NotRequired,
    },
    AgentSpec {
        id:           "chapter-drafter",
        name:         "Chapter Drafting",
        purpose:      "Draft a scene from a synopsis (off by default).",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
    AgentSpec {
        id:           "dev-editor",
        name:         "Developmental Editor",
        purpose:      "Produce structural notes per chapter.",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
    AgentSpec {
        id:           "continuity",
        name:         "Continuity",
        purpose:      "Flag name drift, POV violations, and timeline issues.",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
    AgentSpec {
        id:           "copyeditor",
        name:         "Copyeditor",
        purpose:      "Mechanical fixes: punctuation, spacing, em-dashes.",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
    AgentSpec {
        id:           "humanization",
        name:         "Humanization",
        purpose:      "Surface AI-tells; propose human-sounding alternatives using vocab + style memory.",
        when_to_run:  WhenToRun::OnDemand,
        user_gate:    UserGate::Required,
    },
];

/// Look up a spec by its stable `id`.
pub fn find_agent(id: &str) -> Option<&'static AgentSpec> {
    MVP_AGENTS.iter().find(|a| a.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_nine_mvp_agents_present() {
        assert_eq!(MVP_AGENTS.len(), 9);
    }

    #[test]
    fn agent_ids_are_unique() {
        let mut ids: Vec<_> = MVP_AGENTS.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), MVP_AGENTS.len(), "duplicate agent ids");
    }

    #[test]
    fn find_outline_architect() {
        let spec = find_agent("outline-architect").expect("outline-architect must exist");
        assert_eq!(spec.name, "Outline Architect");
    }
}
