//! Typed wrappers around Tauri IPC `invoke` calls.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentRunResultDto,
  AiApplyInput,
  AiApplyResult,
  AiCancelInput,
  AiSuggestDoneEvent,
  AiSuggestInput,
  AiSuggestStartedResult,
  AiSuggestTokenEvent,
  ApplyOutlineInput,
  ApplyOutlineResult,
  ApplyCopyeditInput,
  ApplyCopyeditResult,
  ApplyHumanizationInput,
  ApplyContinuityInput,
  ApplyContinuityResultDto,
  OriginalityScanInput,
  OriginalityScanResult,
  VocabApplyInput,
  VocabApplyResult,
  ApplyFixInput,
  ApplyFixResult,
  RunChapterDrafterInput,
  RunContinuityInput,
  RunCopyeditInput,
  RunDevEditorInput,
  RunHumanizationInput,
  RunIntakeInput,
  RunIntakeAndOutlineInput,
  RunIntakeAndOutlineResult,
  AgentCancelInput,
  AgentRunStartedEvent,
  AgentRunCompletedEvent,
  AgentRunProgressEvent,
  RunDevelopmentalReviewInput,
  RunDevelopmentalReviewResult,
  EntityBibleApplyInput,
  EntityBibleApplyResult,
  RunMemoryCuratorInput,
  RunProposalValidatorInput,
  RunVocabDictionaryInput,
  ExportGateDto,
  ExportMarkdownInput,
  ExportMarkdownResult,
  ExportRunInput,
  ExportRunResult,
  ExportHistoryEntry,
  ExportDependencyReport,
  SaveDiagnosticBundleInput,
  SaveDiagnosticBundleResult,
  AppVersion,
  MemoryDeleteInput,
  MemoryEntryDto,
  MemoryListInput,
  MemoryUpsertInput,
  ValidatorReportDto,
  VocabEntryDto,
  VocabListInput,
  VocabUpsertInput,
  CreateProjectInput,
  ModelListEntry,
  NodeCreateInput,
  NodeDiffDto,
  NodeInfo,
  NodeUpdateInput,
  OllamaProbeResult,
  OpenProjectInput,
  OpenProjectResult,
  OutlineRunResult,
  RecentProjectEntry,
  RecoveryStatus,
  RunOutlineInput,
  SceneLoadResult,
  SceneSaveInput,
  SmokeTestResult,
  SnapshotCreateInput,
  SnapshotDiffInput,
  SnapshotDto,
  SnapshotListInput,
  SnapshotRestoreInput,
  SnapshotRestoreResult,
} from "@booksforge/shared-types";

export const ipc = {
  // ── Project lifecycle ─────────────────────────────────────────────────────
  projectCreate(input: CreateProjectInput): Promise<OpenProjectResult> {
    return invoke("project_create", { input });
  },
  projectOpen(input: OpenProjectInput): Promise<OpenProjectResult> {
    return invoke("project_open", { input });
  },
  projectClose(): Promise<void> {
    return invoke("project_close");
  },
  projectRecent(): Promise<RecentProjectEntry[]> {
    return invoke("project_recent");
  },

  // ── Document tree (nodes) ─────────────────────────────────────────────────
  nodeList(): Promise<NodeInfo[]> {
    return invoke("node_list");
  },
  nodeCreate(input: NodeCreateInput): Promise<NodeInfo> {
    return invoke("node_create", { input });
  },
  nodeUpdate(input: NodeUpdateInput): Promise<NodeInfo> {
    return invoke("node_update", { input });
  },
  nodeDelete(id: string): Promise<void> {
    return invoke("node_delete", { id });
  },

  // ── Scene content ─────────────────────────────────────────────────────────
  sceneSave(input: SceneSaveInput): Promise<void> {
    return invoke("scene_save", { input });
  },
  sceneLoad(nodeId: string): Promise<SceneLoadResult | null> {
    return invoke("scene_load", { nodeId });
  },

  // ── Crash recovery ────────────────────────────────────────────────────────
  recoveryCheck(): Promise<RecoveryStatus> {
    return invoke("recovery_check");
  },
  recoveryClear(): Promise<void> {
    return invoke("recovery_clear");
  },

  // ── Ollama / AI setup ─────────────────────────────────────────────────────
  ollamaProbe(): Promise<OllamaProbeResult> {
    return invoke("ollama_probe");
  },
  ollamaLaunch(): Promise<void> {
    return invoke("ollama_launch");
  },
  ollamaListModels(): Promise<ModelListEntry[]> {
    return invoke("ollama_list_models");
  },
  ollamaPull(model: string): Promise<void> {
    return invoke("ollama_pull", { model });
  },
  ollamaSmokeTest(model: string): Promise<SmokeTestResult> {
    return invoke("ollama_smoke_test", { model });
  },

  // ── Agent workflows ───────────────────────────────────────────────────────
  agentRunOutline(input: RunOutlineInput): Promise<OutlineRunResult> {
    return invoke("agent_run_outline", { input });
  },
  agentApplyOutline(input: ApplyOutlineInput): Promise<ApplyOutlineResult> {
    return invoke("agent_apply_outline", { input });
  },
  agentApplyCopyedit(input: ApplyCopyeditInput): Promise<ApplyCopyeditResult> {
    return invoke("agent_apply_copyedit", { input });
  },
  agentApplyHumanization(input: ApplyHumanizationInput): Promise<ApplyCopyeditResult> {
    return invoke("agent_apply_humanization", { input });
  },
  agentApplyContinuity(input: ApplyContinuityInput): Promise<ApplyContinuityResultDto> {
    return invoke("agent_apply_continuity", { input });
  },
  vocabApplyProposals(input: VocabApplyInput): Promise<VocabApplyResult> {
    return invoke("vocab_apply_proposals", { input });
  },
  originalityConsentLoad(): Promise<unknown> {
    return invoke("originality_consent_load");
  },
  originalityConsentSet(consentJson: string): Promise<void> {
    return invoke("originality_consent_set", { consentJson });
  },
  originalityConsentClear(): Promise<void> {
    return invoke("originality_consent_clear");
  },
  agentRunCopyedit(input: RunCopyeditInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_copyedit", { input });
  },
  agentRunContinuity(input: RunContinuityInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_continuity", { input });
  },
  agentRunIntake(input: RunIntakeInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_intake", { input });
  },
  agentRunIntakeAndOutline(input: RunIntakeAndOutlineInput): Promise<RunIntakeAndOutlineResult> {
    return invoke("agent_run_intake_and_outline", { input });
  },
  agentCancel(input: AgentCancelInput): Promise<void> {
    return invoke("agent_cancel", { input });
  },
  agentRunDevelopmentalReview(input: RunDevelopmentalReviewInput): Promise<RunDevelopmentalReviewResult> {
    return invoke("agent_run_developmental_review", { input });
  },
  entityBibleApplyProposals(input: EntityBibleApplyInput): Promise<EntityBibleApplyResult> {
    return invoke("entity_bible_apply_proposals", { input });
  },
  /**
   * Subscribe to live agent-run events (BACKLOG §E4).  Fires
   * `agent-run-started` when a dispatch begins and `agent-run-completed`
   * when it resolves.  Returns an unlisten function the caller MUST
   * invoke on cleanup.
   */
  onAgentRunStarted(cb: (e: AgentRunStartedEvent) => void): Promise<UnlistenFn> {
    return listen<AgentRunStartedEvent>("agent-run-started", evt => cb(evt.payload));
  },
  onAgentRunCompleted(cb: (e: AgentRunCompletedEvent) => void): Promise<UnlistenFn> {
    return listen<AgentRunCompletedEvent>("agent-run-completed", evt => cb(evt.payload));
  },
  onAgentRunProgress(cb: (e: AgentRunProgressEvent) => void): Promise<UnlistenFn> {
    return listen<AgentRunProgressEvent>("agent-run-progress", evt => cb(evt.payload));
  },
  agentRunMemoryCurator(input: RunMemoryCuratorInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_memory_curator", { input });
  },
  agentRunVocabDictionary(input: RunVocabDictionaryInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_vocab_dictionary", { input });
  },
  agentRunChapterDrafter(input: RunChapterDrafterInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_chapter_drafter", { input });
  },
  agentRunDevEditor(input: RunDevEditorInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_dev_editor", { input });
  },
  agentRunHumanization(input: RunHumanizationInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_humanization", { input });
  },
  agentRunProposalValidator(input: RunProposalValidatorInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_proposal_validator", { input });
  },
  voiceFingerprintRefresh(): Promise<unknown> {
    return invoke("voice_fingerprint_refresh");
  },
  voiceFingerprintLoad(): Promise<unknown> {
    return invoke("voice_fingerprint_load");
  },
  originalityScanChapter(input: OriginalityScanInput): Promise<OriginalityScanResult> {
    return invoke("originality_scan_chapter", { input });
  },

  // ── Quick-action presets (MZ-08) ──────────────────────────────────────────
  aiSuggest(input: AiSuggestInput): Promise<AiSuggestStartedResult> {
    return invoke("ai_suggest", { input });
  },
  aiCancel(input: AiCancelInput): Promise<void> {
    return invoke("ai_cancel", { input });
  },
  aiApply(input: AiApplyInput): Promise<AiApplyResult> {
    return invoke("ai_apply", { input });
  },
  /**
   * Subscribe to streaming token events for a specific job. Returns an
   * unlisten function the caller MUST invoke on cleanup.
   */
  onAiSuggestToken(jobId: string, cb: (e: AiSuggestTokenEvent) => void): Promise<UnlistenFn> {
    return listen<AiSuggestTokenEvent>(`ai-suggest:${jobId}:token`, (evt) => cb(evt.payload));
  },
  /**
   * Subscribe to the terminal "done" event for a specific job. Fires once
   * (status: ok | cancelled | error).
   */
  onAiSuggestDone(jobId: string, cb: (e: AiSuggestDoneEvent) => void): Promise<UnlistenFn> {
    return listen<AiSuggestDoneEvent>(`ai-suggest:${jobId}:done`, (evt) => cb(evt.payload));
  },

  // ── Snapshots (MZ-06) ─────────────────────────────────────────────────────
  snapshotCreate(input: SnapshotCreateInput): Promise<SnapshotDto> {
    return invoke("snapshot_create", { input });
  },
  snapshotList(input: SnapshotListInput = { scope_id: null }): Promise<SnapshotDto[]> {
    return invoke("snapshot_list", { input });
  },
  snapshotDiff(input: SnapshotDiffInput): Promise<NodeDiffDto[]> {
    return invoke("snapshot_diff", { input });
  },
  snapshotRestore(input: SnapshotRestoreInput): Promise<SnapshotRestoreResult> {
    return invoke("snapshot_restore", { input });
  },

  // ── Export pipeline (Markdown direct + unified export_run for all profiles) ──
  exportMarkdown(input: ExportMarkdownInput): Promise<ExportMarkdownResult> {
    return invoke("export_markdown", { input });
  },
  exportRun(input: ExportRunInput): Promise<ExportRunResult> {
    return invoke("export_run", { input });
  },
  exportHistory(): Promise<ExportHistoryEntry[]> {
    return invoke("export_history");
  },
  exportCheckDependencies(): Promise<ExportDependencyReport> {
    return invoke("export_check_dependencies");
  },
  saveDiagnosticBundle(input: SaveDiagnosticBundleInput): Promise<SaveDiagnosticBundleResult> {
    return invoke("save_diagnostic_bundle", { input });
  },
  appVersion(): Promise<AppVersion> {
    return invoke("app_version");
  },

  // ── Validators + pre-export gate (Phase 4) ────────────────────────────────
  validatorsRun(): Promise<ValidatorReportDto> {
    return invoke("validators_run");
  },
  validatorsGate(): Promise<ExportGateDto> {
    return invoke("validators_gate");
  },
  validatorsApplyFix(input: ApplyFixInput): Promise<ApplyFixResult> {
    return invoke("validators_apply_fix", { input });
  },

  // ── Memory + vocabulary (Phase 3 + Turn B IPC surface) ───────────────────
  memoryList(input: MemoryListInput): Promise<MemoryEntryDto[]> {
    return invoke("memory_list", { input });
  },
  vocabList(input: VocabListInput): Promise<VocabEntryDto[]> {
    return invoke("vocab_list", { input });
  },

  // ── Manual memory + vocabulary CRUD (audit #30) ──────────────────────────
  memoryUpsert(input: MemoryUpsertInput): Promise<MemoryEntryDto> {
    return invoke("memory_upsert", { input });
  },
  memoryDelete(input: MemoryDeleteInput): Promise<boolean> {
    return invoke("memory_delete", { input });
  },
  vocabUpsert(input: VocabUpsertInput): Promise<VocabEntryDto> {
    return invoke("vocab_upsert", { input });
  },
};
