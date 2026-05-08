//! Domain layer — pure logic, no I/O, no clocks, no randomness.
//!
//! All types here are value objects or pure-function modules.  Any timestamp
//! or ID that needs to be "now" or "new" is passed in by the caller so that
//! tests can use deterministic values.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod agent_io;
pub mod agent_record;
pub mod brief;
pub mod council;
pub mod crash_report;
pub mod entity;
pub mod error;
pub mod lexorank;
pub mod node;
pub mod outline;
pub mod export_record;
pub mod format_profile;
pub mod memory;
pub mod originality_provider;
pub mod outline_apply;
pub mod pm_doc;
pub mod project;
pub mod quick_action;
pub mod settings;
pub mod snapshot;
pub mod style;
pub mod validator;
pub mod vocab;
pub mod voice;

pub use agent_io::{
    ContinuityEvidence, ContinuityFinding, ContinuityFix, ContinuityFixKind, ContinuityFixScope,
    ContinuityKind, ContinuityReport, ContinuityReportEntry, CopyeditCategory, CopyeditEdit,
    CopyeditProposals, DevelopmentalAxis, DevelopmentalNote, DevelopmentalNotes, EntityStub,
    HumanizationEdit, HumanizationProposals, MemoryRefreshInput, MemoryRefreshProposals,
    MemoryRefreshScope, MemoryUpsert, ProposalValidation, SceneDraftProposal, ValidationAxis,
    ValidationCheck, ValidationOutcome, ValidationVerdict, VocabAddition, VocabModification,
    VocabUpdateProposals,
};
pub use agent_record::{AgentOutput, AgentRun, AgentTask, AgentTaskStatus};
pub use council::{
    peer_reviewers_for, PeerConcernSeverity, PeerReviewConcern, PeerReviewFocus,
    PeerReviewPairing, PeerReviewRequest, PeerReviewResult, VerificationReport,
};
pub use voice::VoiceFingerprint;
pub use brief::{ProjectBrief, RawIdea};
pub use entity::{Entity, EntityKind};
pub use error::DomainError;
pub use export_record::{ExportProfile, ExportRecord};
pub use format_profile::{FormatProfile, FrontMatterPage, Genre, GOOGLE_FONT_BUNDLE};
pub use memory::{
    allowed_write_scopes, authorise_write, MemoryEntry, MemoryError, MemoryScope,
};
pub use node::{Node, NodeKind, NodeStatus, SceneContent};
pub use outline::{ChapterPlan, OutlineProposal, PartPlan, ScenePlan};
pub use originality_provider::{
    OriginalityCheckResult, OriginalityConsent, OriginalityProviderId,
};
pub use outline_apply::{outline_to_tree, NodeTreeDelta, OutlineApplyError};
pub use pm_doc::{flat_text_to_pm_doc, pm_doc_to_text};
pub use project::{BookMode, Project, ProjectMeta};
pub use quick_action::{AiCall, AiCallStatus, QuickActionPreset};
pub use settings::{OllamaSettings, RecentProject, RecentProjectsList, UiSettings, UserSettings};
pub use snapshot::{
    diff_trees, AgentAppliedEdit, AppliedEditKind, NodeDiff, NodeDiffKind, SnapshotRecord,
    SnapshotScope, SnapshotTree, SnapshotTrigger, TreeEntry,
};
pub use style::{EllipsisForm, EmDash, QuoteStyle, StyleBook};
pub use validator::{
    pre_export_gate, GateOutcome, Severity, ValidationReport, ValidatorIssue, ValidatorRun,
    ValidatorRunStatus,
};
pub use vocab::{
    layer_specificity, replacement_for, resolve as resolve_vocab, EntryKind, EntrySource,
    VocabEntry, VocabError,
};
