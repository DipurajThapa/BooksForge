//! IPC types shared between the Rust backend and the TypeScript frontend.
//!
//! Every type that travels over a Tauri command derives [`serde::Serialize`],
//! [`serde::Deserialize`], and [`ts_rs::TS`]. The TypeScript bindings are
//! generated into `packages/shared-types/src/bindings/` by running:
//!
//! ```sh
//! cargo test -p booksforge-ipc
//! ```
//!
//! Commit the generated files. CI fails if the bindings drift from the Rust
//! source (see `.github/workflows/ci.yml` job `ipc-drift`).

#![forbid(unsafe_code)]
// BACKLOG §C4: tests freely use `.unwrap()` / `.expect()` against canned
// fixtures; the workspace-level clippy lints fire only on shipped code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod agent_events;
pub mod agent_runs;
pub mod agents;
pub mod diagnostics;
pub mod editor;
pub mod error;
pub mod export;
pub mod memory_vocab;
pub mod ollama;
pub mod project;
pub mod quick_action;
pub mod snapshot;
pub mod system;
pub mod validator;

pub use agent_runs::{
    AgentRunResultDto, PeerReviewConcernDto, PeerReviewResultDto, ProposalValidationDto,
    RunChapterDrafterInput, RunContinuityInput, RunCopyeditInput, RunDevEditorInput,
    RunHumanizationInput, RunIntakeInput, RunMemoryCuratorInput, RunProposalValidatorInput,
    RunVocabDictionaryInput, ValidationCheckDto, VerificationReportDto,
};
pub use agents::{
    ApplyContinuityInput, ApplyContinuityResultDto, ApplyCopyeditInput, ApplyCopyeditResult,
    ApplyHumanizationInput, ApplyOutlineInput, ApplyOutlineResult, ContinuityScenePassDto,
    EntityBibleApplyInput, EntityBibleApplyResult, OriginalityScanInput, OriginalityScanResult,
    OutlineRunResult, OverlapHitDto, RunDevelopmentalReviewInput, RunDevelopmentalReviewResult,
    RunIntakeAndOutlineInput, RunIntakeAndOutlineResult, RunOutlineInput, VocabApplyInput,
    VocabApplyResult,
};
pub use agent_events::{
    AgentCancelInput, AgentRunCompletedEvent, AgentRunProgressEvent, AgentRunStartedEvent,
};
pub use diagnostics::{SaveDiagnosticBundleInput, SaveDiagnosticBundleResult};
pub use export::{
    ExportDependencyReport, ExportDependencyStatus, ExportHistoryEntry, ExportMarkdownInput,
    ExportMarkdownResult, ExportRunInput, ExportRunResult,
};
pub use quick_action::{
    AiApplyInput, AiApplyResult, AiCancelInput, AiSuggestDoneEvent, AiSuggestInput,
    AiSuggestStartedResult, AiSuggestTokenEvent,
};
pub use snapshot::{
    NodeDiffDto, SnapshotCreateInput, SnapshotDiffInput, SnapshotDto, SnapshotListInput,
    SnapshotRestoreInput, SnapshotRestoreResult,
};
pub use validator::{
    ApplyFixInput, ApplyFixResult, ExportGateDto, ValidatorIssueDto, ValidatorReportDto,
};
pub use memory_vocab::{MemoryEntryDto, MemoryListInput, VocabEntryDto, VocabListInput};
pub use editor::{
    NodeCreateInput, NodeInfo, NodeUpdateInput, RecoveryStatus, SceneLoadResult, SceneSaveInput,
};
pub use error::BooksForgeError;
pub use ollama::{ModelListEntry, OllamaProbeResult, PullProgressPayload, SmokeTestResult};
pub use project::{CreateProjectInput, OpenProjectInput, OpenProjectResult, RecentProjectEntry};
pub use system::AppVersion;

// ── ts-rs export test ────────────────────────────────────────────────────────
// Running `cargo test -p booksforge-ipc` regenerates all TypeScript bindings.
#[cfg(test)]
mod ts_bindings {
    use ts_rs::TS as _;

    const BINDINGS_DIR: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/shared-types/src/bindings");

    #[test]
    fn export_system_bindings() {
        crate::AppVersion::export_all_to(BINDINGS_DIR)
            .expect("failed to export AppVersion bindings");
    }

    #[test]
    fn export_error_bindings() {
        crate::BooksForgeError::export_all_to(BINDINGS_DIR)
            .expect("failed to export BooksForgeError bindings");
    }

    #[test]
    fn export_editor_bindings() {
        use crate::editor::*;
        NodeInfo::export_all_to(BINDINGS_DIR).expect("NodeInfo");
        NodeCreateInput::export_all_to(BINDINGS_DIR).expect("NodeCreateInput");
        NodeUpdateInput::export_all_to(BINDINGS_DIR).expect("NodeUpdateInput");
        SceneSaveInput::export_all_to(BINDINGS_DIR).expect("SceneSaveInput");
        SceneLoadResult::export_all_to(BINDINGS_DIR).expect("SceneLoadResult");
        RecoveryStatus::export_all_to(BINDINGS_DIR).expect("RecoveryStatus");
    }

    #[test]
    fn export_project_bindings() {
        use crate::project::*;
        CreateProjectInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export CreateProjectInput");
        OpenProjectInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export OpenProjectInput");
        OpenProjectResult::export_all_to(BINDINGS_DIR)
            .expect("failed to export OpenProjectResult");
        RecentProjectEntry::export_all_to(BINDINGS_DIR)
            .expect("failed to export RecentProjectEntry");
    }

    #[test]
    fn export_ollama_bindings() {
        use crate::ollama::*;
        OllamaProbeResult::export_all_to(BINDINGS_DIR).expect("OllamaProbeResult");
        ModelListEntry::export_all_to(BINDINGS_DIR).expect("ModelListEntry");
        PullProgressPayload::export_all_to(BINDINGS_DIR).expect("PullProgressPayload");
        SmokeTestResult::export_all_to(BINDINGS_DIR).expect("SmokeTestResult");
    }

    #[test]
    fn export_agent_bindings() {
        use crate::agents::*;
        RunOutlineInput::export_all_to(BINDINGS_DIR).expect("RunOutlineInput");
        OutlineRunResult::export_all_to(BINDINGS_DIR).expect("OutlineRunResult");
        ApplyOutlineInput::export_all_to(BINDINGS_DIR).expect("ApplyOutlineInput");
        ApplyOutlineResult::export_all_to(BINDINGS_DIR).expect("ApplyOutlineResult");
        ApplyCopyeditInput::export_all_to(BINDINGS_DIR).expect("ApplyCopyeditInput");
        ApplyCopyeditResult::export_all_to(BINDINGS_DIR).expect("ApplyCopyeditResult");
        ApplyHumanizationInput::export_all_to(BINDINGS_DIR).expect("ApplyHumanizationInput");
        ApplyContinuityInput::export_all_to(BINDINGS_DIR).expect("ApplyContinuityInput");
        ApplyContinuityResultDto::export_all_to(BINDINGS_DIR).expect("ApplyContinuityResultDto");
        VocabApplyInput::export_all_to(BINDINGS_DIR).expect("VocabApplyInput");
        VocabApplyResult::export_all_to(BINDINGS_DIR).expect("VocabApplyResult");
        RunIntakeAndOutlineInput::export_all_to(BINDINGS_DIR).expect("RunIntakeAndOutlineInput");
        RunIntakeAndOutlineResult::export_all_to(BINDINGS_DIR).expect("RunIntakeAndOutlineResult");
        RunDevelopmentalReviewInput::export_all_to(BINDINGS_DIR).expect("RunDevelopmentalReviewInput");
        RunDevelopmentalReviewResult::export_all_to(BINDINGS_DIR).expect("RunDevelopmentalReviewResult");
        ContinuityScenePassDto::export_all_to(BINDINGS_DIR).expect("ContinuityScenePassDto");
        EntityBibleApplyInput::export_all_to(BINDINGS_DIR).expect("EntityBibleApplyInput");
        EntityBibleApplyResult::export_all_to(BINDINGS_DIR).expect("EntityBibleApplyResult");
        OriginalityScanInput::export_all_to(BINDINGS_DIR).expect("OriginalityScanInput");
        OriginalityScanResult::export_all_to(BINDINGS_DIR).expect("OriginalityScanResult");
        OverlapHitDto::export_all_to(BINDINGS_DIR).expect("OverlapHitDto");
    }

    #[test]
    fn export_memory_vocab_bindings() {
        use crate::memory_vocab::*;
        MemoryEntryDto::export_all_to(BINDINGS_DIR).expect("MemoryEntryDto");
        MemoryListInput::export_all_to(BINDINGS_DIR).expect("MemoryListInput");
        VocabEntryDto::export_all_to(BINDINGS_DIR).expect("VocabEntryDto");
        VocabListInput::export_all_to(BINDINGS_DIR).expect("VocabListInput");
    }

    #[test]
    fn export_validator_bindings() {
        use crate::validator::*;
        ValidatorIssueDto::export_all_to(BINDINGS_DIR).expect("ValidatorIssueDto");
        ValidatorReportDto::export_all_to(BINDINGS_DIR).expect("ValidatorReportDto");
        ExportGateDto::export_all_to(BINDINGS_DIR).expect("ExportGateDto");
        ApplyFixInput::export_all_to(BINDINGS_DIR).expect("ApplyFixInput");
        ApplyFixResult::export_all_to(BINDINGS_DIR).expect("ApplyFixResult");
    }

    #[test]
    fn export_agent_run_bindings() {
        use crate::agent_runs::*;
        ValidationCheckDto::export_all_to(BINDINGS_DIR).expect("ValidationCheckDto");
        ProposalValidationDto::export_all_to(BINDINGS_DIR).expect("ProposalValidationDto");
        PeerReviewConcernDto::export_all_to(BINDINGS_DIR).expect("PeerReviewConcernDto");
        PeerReviewResultDto::export_all_to(BINDINGS_DIR).expect("PeerReviewResultDto");
        VerificationReportDto::export_all_to(BINDINGS_DIR).expect("VerificationReportDto");
        AgentRunResultDto::export_all_to(BINDINGS_DIR).expect("AgentRunResultDto");
        RunCopyeditInput::export_all_to(BINDINGS_DIR).expect("RunCopyeditInput");
        RunContinuityInput::export_all_to(BINDINGS_DIR).expect("RunContinuityInput");
        RunIntakeInput::export_all_to(BINDINGS_DIR).expect("RunIntakeInput");
        RunMemoryCuratorInput::export_all_to(BINDINGS_DIR).expect("RunMemoryCuratorInput");
        RunVocabDictionaryInput::export_all_to(BINDINGS_DIR).expect("RunVocabDictionaryInput");
        RunChapterDrafterInput::export_all_to(BINDINGS_DIR).expect("RunChapterDrafterInput");
        RunDevEditorInput::export_all_to(BINDINGS_DIR).expect("RunDevEditorInput");
        RunHumanizationInput::export_all_to(BINDINGS_DIR).expect("RunHumanizationInput");
        RunProposalValidatorInput::export_all_to(BINDINGS_DIR).expect("RunProposalValidatorInput");
    }

    #[test]
    fn export_export_bindings() {
        use crate::export::*;
        ExportMarkdownInput::export_all_to(BINDINGS_DIR).expect("ExportMarkdownInput");
        ExportMarkdownResult::export_all_to(BINDINGS_DIR).expect("ExportMarkdownResult");
        ExportRunInput::export_all_to(BINDINGS_DIR).expect("ExportRunInput");
        ExportRunResult::export_all_to(BINDINGS_DIR).expect("ExportRunResult");
        ExportHistoryEntry::export_all_to(BINDINGS_DIR).expect("ExportHistoryEntry");
        ExportDependencyStatus::export_all_to(BINDINGS_DIR).expect("ExportDependencyStatus");
        ExportDependencyReport::export_all_to(BINDINGS_DIR).expect("ExportDependencyReport");
    }

    #[test]
    fn export_diagnostics_bindings() {
        use crate::diagnostics::*;
        SaveDiagnosticBundleInput::export_all_to(BINDINGS_DIR).expect("SaveDiagnosticBundleInput");
        SaveDiagnosticBundleResult::export_all_to(BINDINGS_DIR).expect("SaveDiagnosticBundleResult");
    }

    #[test]
    fn export_agent_event_bindings() {
        use crate::agent_events::*;
        AgentRunStartedEvent::export_all_to(BINDINGS_DIR).expect("AgentRunStartedEvent");
        AgentRunCompletedEvent::export_all_to(BINDINGS_DIR).expect("AgentRunCompletedEvent");
        AgentRunProgressEvent::export_all_to(BINDINGS_DIR).expect("AgentRunProgressEvent");
        AgentCancelInput::export_all_to(BINDINGS_DIR).expect("AgentCancelInput");
    }

    #[test]
    fn export_quick_action_bindings() {
        use crate::quick_action::*;
        AiSuggestInput::export_all_to(BINDINGS_DIR).expect("AiSuggestInput");
        AiSuggestStartedResult::export_all_to(BINDINGS_DIR).expect("AiSuggestStartedResult");
        AiSuggestTokenEvent::export_all_to(BINDINGS_DIR).expect("AiSuggestTokenEvent");
        AiSuggestDoneEvent::export_all_to(BINDINGS_DIR).expect("AiSuggestDoneEvent");
        AiCancelInput::export_all_to(BINDINGS_DIR).expect("AiCancelInput");
        AiApplyInput::export_all_to(BINDINGS_DIR).expect("AiApplyInput");
        AiApplyResult::export_all_to(BINDINGS_DIR).expect("AiApplyResult");
    }

    #[test]
    fn export_snapshot_bindings() {
        use crate::snapshot::*;
        SnapshotCreateInput::export_all_to(BINDINGS_DIR).expect("SnapshotCreateInput");
        SnapshotListInput::export_all_to(BINDINGS_DIR).expect("SnapshotListInput");
        SnapshotDiffInput::export_all_to(BINDINGS_DIR).expect("SnapshotDiffInput");
        SnapshotRestoreInput::export_all_to(BINDINGS_DIR).expect("SnapshotRestoreInput");
        SnapshotDto::export_all_to(BINDINGS_DIR).expect("SnapshotDto");
        NodeDiffDto::export_all_to(BINDINGS_DIR).expect("NodeDiffDto");
        SnapshotRestoreResult::export_all_to(BINDINGS_DIR).expect("SnapshotRestoreResult");
    }
}
