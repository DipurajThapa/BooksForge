//! Hard caps for orchestrator runs — values from ARCHITECTURE.md §5.3.

/// Configuration governing a single orchestrator workflow run.
///
/// All cap fields have canonical defaults matching the spec.  They may be
/// tightened (lower) per run but never relaxed above the hard maximums.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum number of individual agent LLM calls in one run.
    /// Hard ceiling: 8.
    pub max_agent_calls: u32,

    /// Maximum wall-clock duration of a run in seconds.
    /// Hard ceiling: 600 (10 minutes).
    pub max_duration_secs: u64,

    /// Maximum tokens consumed (prompt + completion) across all agent calls.
    /// Hard ceiling: 200 000.
    pub max_tokens: u32,

    /// Maximum retries for a single failing agent step before aborting the run.
    /// Hard ceiling: 3.
    pub max_retries_per_step: u32,

    /// Whether the Tier-2 (LLM-backed) ProposalValidator runs automatically
    /// after every primary agent that succeeds Tier-1.  Disabled by default
    /// because it adds a second LLM call per run; enable in projects that
    /// have set `validators.high_confidence_mode = true`.
    pub tier2_enabled: bool,

    /// **Pipeline-wide num_ctx pin** (RCA_RUN15_THRASH.md Fix 1).
    ///
    /// When `Some(n)`, every Ollama call in this run uses `n` as
    /// `num_ctx` — overriding each agent spec's `context_budget.total()`.
    /// This eliminates the model-context-thrashing failure mode we
    /// observed in Run #15: changing `num_ctx` between agent calls
    /// forces qwen3.6 36B to reload + reinit KV cache for the new
    /// context size, costing 5-15 min wall-clock per reload on Apple
    /// Silicon. Pinning at the maximum any agent in the run will need
    /// trades modest VRAM (extra KV cache during low-context calls)
    /// for predictable wall-clock (zero reloads after first warm-up).
    ///
    /// `None` (the default) preserves legacy behaviour: each agent
    /// declares its own num_ctx and pays the reload cost when transitions
    /// happen. Single-call workflows (intake-only) don't need pinning.
    pub pipeline_num_ctx_override: Option<u32>,
}

impl OrchestratorConfig {
    /// Hard-coded spec maximums — nothing may exceed these values.
    pub const MAX_AGENT_CALLS_HARD: u32 = 8;
    /// Wall-clock cap per agent call. Originally 600s; raised to
    /// 1800s (30 min) for `character_bible` on qwen3.5:27b which
    /// generated 4-6 cards per call. Raised again to 3600s (60 min)
    /// after the multi-chapter test surfaced a Run-#15-class failure:
    /// when Ollama swaps `num_ctx` between calls (24k for world bible
    /// ↔ 64k for drafter), the qwen3.6 36B MoE re-initialises its KV
    /// cache for the new context size, which on Apple Silicon costs
    /// 5-15 min on top of generation. 30 min was tight; the failed
    /// drafter sat at 28 min CPU-active when the cap fired. 60 min
    /// gives headroom for one cold-load + a full thinking-mode
    /// generation, and still catches genuinely runaway calls (no
    /// production agent should legitimately take > 1 hour).
    pub const MAX_DURATION_HARD: u64 = 3600;
    pub const MAX_TOKENS_HARD: u32 = 200_000;
    pub const MAX_RETRIES_HARD: u32 = 3;

    /// Returns the canonical default config that matches the spec exactly.
    pub fn default_spec() -> Self {
        Self {
            max_agent_calls: Self::MAX_AGENT_CALLS_HARD,
            max_duration_secs: Self::MAX_DURATION_HARD,
            max_tokens: Self::MAX_TOKENS_HARD,
            max_retries_per_step: Self::MAX_RETRIES_HARD,
            tier2_enabled: false,
            pipeline_num_ctx_override: None,
        }
    }

    /// Builder helper for the pipeline-wide num_ctx pin.
    pub fn with_pipeline_num_ctx(mut self, num_ctx: u32) -> Self {
        self.pipeline_num_ctx_override = Some(num_ctx);
        self
    }

    /// Validate that no cap exceeds the hard ceiling.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_agent_calls > Self::MAX_AGENT_CALLS_HARD {
            return Err("max_agent_calls exceeds hard ceiling of 8");
        }
        if self.max_duration_secs > Self::MAX_DURATION_HARD {
            return Err("max_duration_secs exceeds hard ceiling of 3600");
        }
        if self.max_tokens > Self::MAX_TOKENS_HARD {
            return Err("max_tokens exceeds hard ceiling of 200 000");
        }
        if self.max_retries_per_step > Self::MAX_RETRIES_HARD {
            return Err("max_retries_per_step exceeds hard ceiling of 3");
        }
        Ok(())
    }
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self::default_spec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_passes_validation() {
        assert!(OrchestratorConfig::default().validate().is_ok());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn exceeding_any_cap_fails_validation() {
        // Each scenario flips one cap to its over-limit value;
        // reassign-after-default reads more clearly here than the
        // struct-update form that clippy prefers.
        let mut cfg = OrchestratorConfig::default();
        cfg.max_agent_calls = 9;
        assert!(cfg.validate().is_err());

        let mut cfg = OrchestratorConfig::default();
        cfg.max_tokens = 200_001;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn caps_match_spec() {
        assert_eq!(OrchestratorConfig::MAX_AGENT_CALLS_HARD, 8);
        assert_eq!(OrchestratorConfig::MAX_DURATION_HARD, 3600);
        assert_eq!(OrchestratorConfig::MAX_TOKENS_HARD, 200_000);
        assert_eq!(OrchestratorConfig::MAX_RETRIES_HARD, 3);
    }
}
