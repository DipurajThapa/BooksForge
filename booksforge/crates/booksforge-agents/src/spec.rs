/// The specification for a single agent. All fields are `'static` so the
/// registry can be a `const` slice with zero heap allocation.
///
/// Full fields (schemas, prompt template ids, memory scopes, validators) are
/// added in M2/M5 when the orchestrator and prompt engine are built.
#[derive(Debug, Clone, Copy)]
pub struct AgentSpec {
    /// Stable, kebab-case identifier. Never reuse a retired id.
    pub id: &'static str,
    /// Human-readable name shown in the UI.
    pub name: &'static str,
    /// One-sentence purpose shown in tooltips.
    pub purpose: &'static str,
    /// Whether the agent runs automatically or requires user action.
    pub when_to_run: WhenToRun,
    /// Whether the user must confirm before output is applied.
    pub user_gate: UserGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhenToRun {
    /// Triggered automatically by a workflow step.
    Automatic,
    /// Only runs when the user explicitly requests it.
    OnDemand,
    /// Runs on a schedule (e.g., after every chapter finalise).
    Scheduled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserGate {
    /// Output is never applied without explicit user accept.
    Required,
    /// Output can be applied automatically (used for non-mutating agents).
    NotRequired,
}
