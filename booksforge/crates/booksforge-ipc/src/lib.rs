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
pub mod cover_boilerplate;
pub mod diagnostics;
pub mod editor;
pub mod error;
pub mod export;
pub mod memory_vocab;
pub mod ollama;
pub mod project;
pub mod publishing;
pub mod quality;
pub mod quick_action;
pub mod snapshot;
pub mod system;
pub mod validator;

pub use agent_events::{
    AgentCancelInput, AgentRunCompletedEvent, AgentRunProgressEvent, AgentRunStartedEvent,
};
pub use agent_runs::{
    AgentRunResultDto, PeerReviewConcernDto, PeerReviewResultDto, ProposalValidationDto,
    RunChapterDrafterInput, RunContinuityInput, RunCopyeditInput, RunDevEditorInput,
    RunHumanizationInput, RunIntakeInput, RunMemoryCuratorInput, RunProposalValidatorInput,
    RunVocabDictionaryInput, ValidationCheckDto, VerificationReportDto,
};
pub use agents::{
    ApplyChapterDrafterInput, ApplyChapterDrafterResultDto, ApplyCharacterBibleInput,
    ApplyCharacterBibleResultDto, ApplyContinuityInput, ApplyContinuityResultDto,
    ApplyCopyeditInput, ApplyCopyeditResult, ApplyHumanizationInput, ApplyOutlineInput,
    ApplyOutlineResult, ApplyPolishInput, ApplyPolishResultDto, ApplySceneDrafterFicInput,
    ApplySceneDrafterFicResultDto, ApplyWorldBibleInput, ApplyWorldBibleResultDto,
    ContinuityScenePassDto, EntityBibleApplyInput, EntityBibleApplyResult, OriginalityScanInput,
    OriginalityScanResult, OutlineRunResult, OverlapHitDto, RunCharacterBibleInput,
    RunDevelopmentalReviewInput, RunDevelopmentalReviewResult, RunIntakeAndOutlineInput,
    RunIntakeAndOutlineResult, RunOutlineInput, RunPolishStageInput, RunSceneCriticInput,
    RunSceneDrafterFicInput, RunWorldBibleInput, VocabApplyInput, VocabApplyResult,
};
pub use cover_boilerplate::{
    BoilerplatePageDto, BoilerplateSaveInput, BoilerplateSaveResult, CoverAssetDto,
    CoverImportInput, CoverRemoveInput, CoverSetDto,
};
pub use diagnostics::{SaveDiagnosticBundleInput, SaveDiagnosticBundleResult};
pub use editor::{
    NodeCreateInput, NodeInfo, NodeUpdateInput, RecoveryStatus, SceneLoadResult, SceneSaveInput,
};
pub use error::BooksForgeError;
pub use export::{
    ExportDependencyReport, ExportDependencyStatus, ExportHistoryEntry, ExportMarkdownInput,
    ExportMarkdownResult, ExportRunInput, ExportRunResult,
};
pub use memory_vocab::{
    MemoryDeleteInput, MemoryEntryDto, MemoryListInput, MemoryUpsertInput, VocabEntryDto,
    VocabListInput, VocabUpsertInput,
};
pub use ollama::{ModelListEntry, OllamaProbeResult, PullProgressPayload, SmokeTestResult};
pub use project::{
    CreateProjectInput, OpenProjectInput, OpenProjectResult, ProjectBriefDto,
    ProjectBriefSaveInput, ProjectKindSetInput, ProjectKindSetResult, RecentProjectEntry,
    RecentRemoveInput, RevealInFinderInput,
};
pub use publishing::{
    PlatformReadiness, PrepareForPublishingInput, PrepareForPublishingResult, PublishingMetadata,
    ReadinessItem,
};
pub use quality::{
    GenrePackInput, StylometricDistanceInput, StylometricDistanceResult, TellsScanInput,
    TellsScanResult, VoiceAnchorGetResult, VoiceAnchorSetInput, VoiceAnchorSetResult,
    VoiceFingerprintInput, VoiceFingerprintResult,
};
pub use quick_action::{
    AiApplyInput, AiApplyResult, AiCancelInput, AiSuggestDoneEvent, AiSuggestInput,
    AiSuggestStartedResult, AiSuggestTokenEvent,
};
pub use snapshot::{
    NodeDiffDto, SnapshotCreateInput, SnapshotDiffInput, SnapshotDto, SnapshotListInput,
    SnapshotRestoreInput, SnapshotRestoreResult,
};
pub use system::AppVersion;
pub use validator::{
    ApplyFixInput, ApplyFixResult, ExportGateDto, ValidatorIssueDto, ValidatorReportDto,
};

// ── ts-rs export test ────────────────────────────────────────────────────────
// Running `cargo test -p booksforge-ipc` regenerates all TypeScript bindings.
#[cfg(test)]
mod ts_bindings {
    use ts_rs::TS as _;

    const BINDINGS_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/shared-types/src/bindings"
    );

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
        OpenProjectInput::export_all_to(BINDINGS_DIR).expect("failed to export OpenProjectInput");
        OpenProjectResult::export_all_to(BINDINGS_DIR).expect("failed to export OpenProjectResult");
        RecentProjectEntry::export_all_to(BINDINGS_DIR)
            .expect("failed to export RecentProjectEntry");
        // Recent-projects management — Remove action in the picker.
        RecentRemoveInput::export_all_to(BINDINGS_DIR).expect("failed to export RecentRemoveInput");
        // F10 — "Reveal in Finder/Explorer" affordance on the recents list.
        RevealInFinderInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export RevealInFinderInput");
        // Phase 4 — book-kind editing post-creation.
        ProjectKindSetInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export ProjectKindSetInput");
        ProjectKindSetResult::export_all_to(BINDINGS_DIR)
            .expect("failed to export ProjectKindSetResult");
        // Round 5 — manually-edited ProjectBrief save/load round-trip.
        crate::ProjectBriefSaveInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export ProjectBriefSaveInput");
        crate::ProjectBriefDto::export_all_to(BINDINGS_DIR)
            .expect("failed to export ProjectBriefDto");
        // Phase 4 — domain BookKind enum (re-exported by ipc + emitted here
        // so the index.ts re-export driven by codegen-drift picks it up).
        booksforge_domain::BookKind::export_all_to(BINDINGS_DIR)
            .expect("failed to export BookKind");
        // Phase 7 — Prepare-for-Publishing single-action types.
        crate::PrepareForPublishingInput::export_all_to(BINDINGS_DIR)
            .expect("failed to export PrepareForPublishingInput");
        crate::PrepareForPublishingResult::export_all_to(BINDINGS_DIR)
            .expect("failed to export PrepareForPublishingResult");
        crate::PublishingMetadata::export_all_to(BINDINGS_DIR)
            .expect("failed to export PublishingMetadata");
        crate::PlatformReadiness::export_all_to(BINDINGS_DIR)
            .expect("failed to export PlatformReadiness");
        crate::ReadinessItem::export_all_to(BINDINGS_DIR).expect("failed to export ReadinessItem");
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
        ApplyChapterDrafterInput::export_all_to(BINDINGS_DIR).expect("ApplyChapterDrafterInput");
        ApplyChapterDrafterResultDto::export_all_to(BINDINGS_DIR)
            .expect("ApplyChapterDrafterResultDto");
        // Fiction agents (BACKLOG §A13 / Phase 1).
        RunCharacterBibleInput::export_all_to(BINDINGS_DIR).expect("RunCharacterBibleInput");
        ApplyCharacterBibleInput::export_all_to(BINDINGS_DIR).expect("ApplyCharacterBibleInput");
        ApplyCharacterBibleResultDto::export_all_to(BINDINGS_DIR)
            .expect("ApplyCharacterBibleResultDto");
        RunWorldBibleInput::export_all_to(BINDINGS_DIR).expect("RunWorldBibleInput");
        ApplyWorldBibleInput::export_all_to(BINDINGS_DIR).expect("ApplyWorldBibleInput");
        ApplyWorldBibleResultDto::export_all_to(BINDINGS_DIR).expect("ApplyWorldBibleResultDto");
        RunSceneDrafterFicInput::export_all_to(BINDINGS_DIR).expect("RunSceneDrafterFicInput");
        ApplySceneDrafterFicInput::export_all_to(BINDINGS_DIR).expect("ApplySceneDrafterFicInput");
        ApplySceneDrafterFicResultDto::export_all_to(BINDINGS_DIR)
            .expect("ApplySceneDrafterFicResultDto");
        // Specialist polish stack (BACKLOG §A15 / Phase 2).
        RunPolishStageInput::export_all_to(BINDINGS_DIR).expect("RunPolishStageInput");
        ApplyPolishInput::export_all_to(BINDINGS_DIR).expect("ApplyPolishInput");
        ApplyPolishResultDto::export_all_to(BINDINGS_DIR).expect("ApplyPolishResultDto");
        RunSceneCriticInput::export_all_to(BINDINGS_DIR).expect("RunSceneCriticInput");
        // Phase C quality gates.
        RunConceptScorerInput::export_all_to(BINDINGS_DIR).expect("RunConceptScorerInput");
        RunAudienceMapperInput::export_all_to(BINDINGS_DIR).expect("RunAudienceMapperInput");
        RunCharacterCriticInput::export_all_to(BINDINGS_DIR).expect("RunCharacterCriticInput");
        RunStructureCriticInput::export_all_to(BINDINGS_DIR).expect("RunStructureCriticInput");
        // Quality stack types from sister crates (BACKLOG §A16 / Phase 3).
        booksforge_voice::VoiceProfile::export_all_to(BINDINGS_DIR).expect("VoiceProfile");
        booksforge_voice::StylometricDistance::export_all_to(BINDINGS_DIR)
            .expect("StylometricDistance");
        booksforge_voice::StylometricComponent::export_all_to(BINDINGS_DIR)
            .expect("StylometricComponent");
        booksforge_anti_ai_tells::TellsReport::export_all_to(BINDINGS_DIR).expect("TellsReport");
        booksforge_anti_ai_tells::TellHit::export_all_to(BINDINGS_DIR).expect("TellHit");
        // BookKind is exported above from `booksforge_domain` (Phase 4
        // moved it). The `booksforge_genre_packs::BookKind` re-export
        // produces the same TS file, so this line is no longer needed.
        booksforge_genre_packs::GenrePack::export_all_to(BINDINGS_DIR).expect("GenrePack");
        // Quality-stack IPC wrapper types (live in this crate's quality.rs).
        crate::VoiceFingerprintInput::export_all_to(BINDINGS_DIR).expect("VoiceFingerprintInput");
        crate::VoiceFingerprintResult::export_all_to(BINDINGS_DIR).expect("VoiceFingerprintResult");
        crate::VoiceAnchorSetInput::export_all_to(BINDINGS_DIR).expect("VoiceAnchorSetInput");
        crate::VoiceAnchorSetResult::export_all_to(BINDINGS_DIR).expect("VoiceAnchorSetResult");
        crate::VoiceAnchorGetResult::export_all_to(BINDINGS_DIR).expect("VoiceAnchorGetResult");
        crate::StylometricDistanceInput::export_all_to(BINDINGS_DIR)
            .expect("StylometricDistanceInput");
        crate::StylometricDistanceResult::export_all_to(BINDINGS_DIR)
            .expect("StylometricDistanceResult");
        crate::TellsScanInput::export_all_to(BINDINGS_DIR).expect("TellsScanInput");
        crate::TellsScanResult::export_all_to(BINDINGS_DIR).expect("TellsScanResult");
        crate::GenrePackInput::export_all_to(BINDINGS_DIR).expect("GenrePackInput");
        ApplyHumanizationInput::export_all_to(BINDINGS_DIR).expect("ApplyHumanizationInput");
        ApplyContinuityInput::export_all_to(BINDINGS_DIR).expect("ApplyContinuityInput");
        ApplyContinuityResultDto::export_all_to(BINDINGS_DIR).expect("ApplyContinuityResultDto");
        VocabApplyInput::export_all_to(BINDINGS_DIR).expect("VocabApplyInput");
        VocabApplyResult::export_all_to(BINDINGS_DIR).expect("VocabApplyResult");
        RunIntakeAndOutlineInput::export_all_to(BINDINGS_DIR).expect("RunIntakeAndOutlineInput");
        RunIntakeAndOutlineResult::export_all_to(BINDINGS_DIR).expect("RunIntakeAndOutlineResult");
        RunDevelopmentalReviewInput::export_all_to(BINDINGS_DIR)
            .expect("RunDevelopmentalReviewInput");
        RunDevelopmentalReviewResult::export_all_to(BINDINGS_DIR)
            .expect("RunDevelopmentalReviewResult");
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
    fn export_cover_boilerplate_bindings() {
        use crate::cover_boilerplate::*;
        CoverImportInput::export_all_to(BINDINGS_DIR).expect("CoverImportInput");
        CoverRemoveInput::export_all_to(BINDINGS_DIR).expect("CoverRemoveInput");
        CoverAssetDto::export_all_to(BINDINGS_DIR).expect("CoverAssetDto");
        CoverSetDto::export_all_to(BINDINGS_DIR).expect("CoverSetDto");
        BoilerplatePageDto::export_all_to(BINDINGS_DIR).expect("BoilerplatePageDto");
        BoilerplateSaveInput::export_all_to(BINDINGS_DIR).expect("BoilerplateSaveInput");
        BoilerplateSaveResult::export_all_to(BINDINGS_DIR).expect("BoilerplateSaveResult");
    }

    #[test]
    fn export_diagnostics_bindings() {
        use crate::diagnostics::*;
        SaveDiagnosticBundleInput::export_all_to(BINDINGS_DIR).expect("SaveDiagnosticBundleInput");
        SaveDiagnosticBundleResult::export_all_to(BINDINGS_DIR)
            .expect("SaveDiagnosticBundleResult");
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
