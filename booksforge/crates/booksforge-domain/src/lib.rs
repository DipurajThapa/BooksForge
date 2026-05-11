//! Domain layer — pure logic, no I/O, no clocks, no randomness.
//!
//! All types here are value objects or pure-function modules.  Any timestamp
//! or ID that needs to be "now" or "new" is passed in by the caller so that
//! tests can use deterministic values.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod agent_io;
pub mod agent_record;
pub mod audience_map;
pub mod book_kind;
pub mod brief;
pub mod character_score;
pub mod concept_score;
pub mod council;
pub mod cover_boilerplate;
pub mod entity;
pub mod error;
pub mod export_record;
pub mod format_profile;
pub mod lexorank;
pub mod memory;
pub mod node;
pub mod originality_provider;
pub mod outline;
pub mod outline_apply;
pub mod pm_doc;
pub mod project;
pub mod publishing_target;
pub mod quality_gate;
pub mod quick_action;
pub mod settings;
pub mod snapshot;
pub mod structure_score;
pub mod style;
pub mod validator;
pub mod vocab;
pub mod voice;

pub use agent_io::{
    CharacterBibleProposal, CharacterCard, CharacterRelationship, ContinuityEvidence,
    ContinuityFinding, ContinuityFix, ContinuityFixKind, ContinuityFixScope, ContinuityKind,
    ContinuityReport, ContinuityReportEntry, CopyeditCategory, CopyeditEdit, CopyeditProposals,
    DevelopmentalAxis, DevelopmentalNote, DevelopmentalNotes, EntityStub, HumanizationEdit,
    HumanizationProposals, MemoryRefreshInput, MemoryRefreshProposals, MemoryRefreshScope,
    MemoryUpsert, PolishPlan, PolishPlanEntry, PolishProposal, PolishStageId, ProposalValidation,
    SceneBeat, SceneCritiqueProposal, SceneDraftProposal, SensoryPalette, TargetedEdit,
    ValidationAxis, ValidationCheck, ValidationOutcome, ValidationVerdict, VocabAddition,
    VocabModification, VocabUpdateProposals, WorldBibleProposal, WorldLocation,
};
pub use agent_record::{AgentOutput, AgentRun, AgentTask, AgentTaskStatus};
pub use audience_map::{PacingExpectation, ReaderExpectationMap};
pub use book_kind::BookKind;
pub use brief::{ProjectBrief, RawIdea};
pub use character_score::{
    CharacterCriticProposal, CharacterEdit, CharacterScore, CrossCardFinding,
};
pub use concept_score::{ConceptEdit, ConceptScoreAxis, ConceptScoreProposal};
pub use council::{
    peer_reviewers_for, PeerConcernSeverity, PeerReviewConcern, PeerReviewFocus, PeerReviewPairing,
    PeerReviewRequest, PeerReviewResult, VerificationReport,
};
pub use cover_boilerplate::{BoilerplateKind, BoilerplatePage, CoverAsset, CoverSet};
pub use entity::{Entity, EntityKind};
pub use error::DomainError;
pub use export_record::{ExportProfile, ExportRecord};
pub use format_profile::{FormatProfile, FrontMatterPage, Genre, GOOGLE_FONT_BUNDLE};
pub use memory::{allowed_write_scopes, authorise_write, MemoryEntry, MemoryError, MemoryScope};
pub use node::{Node, NodeKind, NodeStatus, SceneContent};
pub use originality_provider::{OriginalityCheckResult, OriginalityConsent, OriginalityProviderId};
pub use outline::{ChapterPlan, OutlineProposal, PartPlan, ScenePlan};
pub use outline_apply::{empty_subtree_ids, outline_to_tree, NodeTreeDelta, OutlineApplyError};
pub use pm_doc::{flat_text_to_pm_doc, pm_doc_to_text};
pub use project::{BookMode, Project, ProjectMeta};
pub use publishing_target::{
    kdp_paperback_gutter_inches, ArtifactFormat, IdentifierScheme, PublishingTarget, TargetSpec,
};
pub use quality_gate::{AXIS_FLOOR, COMPOSITE_THRESHOLD};
pub use quick_action::{AiCall, AiCallStatus, QuickActionPreset};
pub use settings::{OllamaSettings, RecentProject, RecentProjectsList, UiSettings, UserSettings};
pub use snapshot::{
    diff_trees, AgentAppliedEdit, AppliedEditKind, NodeDiff, NodeDiffKind, SnapshotRecord,
    SnapshotScope, SnapshotTree, SnapshotTrigger, TreeEntry,
};
pub use structure_score::{StructureCriticProposal, StructureEdit, StructureFinding};
pub use style::{EllipsisForm, EmDash, QuoteStyle, StyleBook};
pub use validator::{
    pre_export_gate, GateOutcome, Severity, ValidationReport, ValidatorIssue, ValidatorRun,
    ValidatorRunStatus,
};
pub use vocab::{
    layer_specificity, replacement_for, resolve as resolve_vocab, EntryKind, EntrySource,
    VocabEntry, VocabError,
};
pub use voice::VoiceFingerprint;
