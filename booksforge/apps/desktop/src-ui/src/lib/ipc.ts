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
  ApplyChapterDrafterInput,
  ApplyChapterDrafterResultDto,
  RunCharacterBibleInput,
  ApplyCharacterBibleInput,
  ApplyCharacterBibleResultDto,
  RunWorldBibleInput,
  ApplyWorldBibleInput,
  ApplyWorldBibleResultDto,
  RunSceneDrafterFicInput,
  ApplySceneDrafterFicInput,
  ApplySceneDrafterFicResultDto,
  // Phase 2 polish stack
  RunPolishStageInput,
  ApplyPolishInput,
  ApplyPolishResultDto,
  RunSceneCriticInput,
  // Phase 3 quality stack
  VoiceFingerprintInput,
  VoiceFingerprintResult,
  VoiceAnchorSetInput,
  VoiceAnchorSetResult,
  VoiceAnchorGetResult,
  StylometricDistanceInput,
  StylometricDistanceResult,
  TellsScanInput,
  TellsScanResult,
  GenrePackInput,
  GenrePack,
  // Phase 4 / 5B — project classification
  ProjectKindSetInput,
  ProjectKindSetResult,
  // Round 5 — Brief editor round-trip
  ProjectBriefSaveInput,
  ProjectBriefDto,
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
  RunAudienceMapperInput,
  RunCharacterCriticInput,
  RunStructureCriticInput,
  RunConceptScorerInput,
  // Stage 6 — cover & boilerplate flow
  CoverSetDto,
  CoverImportInput,
  CoverRemoveInput,
  BoilerplatePageDto,
  BoilerplateSaveInput,
  BoilerplateSaveResult,
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
  // Prepare-for-Publishing (Phase 7 / UX R4)
  PrepareForPublishingInput,
  PrepareForPublishingResult,
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
  RecentRemoveInput,
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

/**
 * Input to the full-scene pipeline (`agent_run_full_scene_pipeline`).
 * Mirrors `RunFullScenePipelineInput` in
 * `apps/desktop/src/commands/workflows.rs`. Hand-typed here because the
 * source struct lives in the desktop crate (not booksforge-ipc), so its
 * ts-rs bindings emit to `apps/desktop/bindings/` rather than into
 * `@booksforge/shared-types`.
 */
export type RunFullScenePipelineInput = {
  project_id:        string;
  node_id:           string;
  scene_goal:        string;
  scene_conflict:    string;
  scene_reveal:      string;
  target_words:      number;
  chapter_pov:       string;
  /** Heavy model used for drafter + polish stack (e.g. qwen3.5:27b). */
  model:             string;
  /** O2 — optional faster model for the scene-critic pass.
   *  Falls back to `model` when undefined. Recommended: qwen3.5:9b. */
  model_critic?:     string | null;
  /** O1 — when true (default), polish stages whose deterministic
   *  skip-detector reports no work are skipped instead of running the
   *  LLM. Saves ~30–40% wall-clock on typical scenes. */
  skip_empty_polish_stages?: boolean;
  /** When true, stops after the scene-critic pass. */
  stop_after_critic?: boolean;
};

export type PipelineStageResult = {
  /** "draft" | "critic" | "polish:dialogue" | "polish:metaphor"
   *  | "polish:voice" | "polish:scene_tension" | "tells_scan" */
  stage:     string;
  /** "completed" | "skipped" | "failed" */
  status:    string;
  task_id:   string;
  summary:   string;
  elapsed_s: number;
};

export type RunFullScenePipelineResult = {
  project_id:           string;
  node_id:              string;
  book_kind:            string;
  stages:               PipelineStageResult[];
  final_tells_verdict:  string;
  total_elapsed_s:      number;
};

/** Input to the book-level pipeline (`agent_run_book_pipeline`).
 *  Mirrors `RunBookPipelineInput` in `apps/desktop/src/commands/workflows.rs`.
 *  Hand-typed because the binding file lives in `apps/desktop/bindings/`
 *  (auto-emitted by ts-rs from the desktop crate's tests). */
export type RunBookPipelineInput = {
  project_id: string;
  /** Default true — scenes that already have prose are left alone. */
  skip_already_drafted_scenes?: boolean;
  /** Cap how many scenes to draft this run. Omit for "all of them". */
  max_scenes?: number | null;
};

export type BookSceneStageResult = {
  scene_id:    string;
  scene_title: string;
  /** "completed" | "skipped" | "failed" | "cancelled" */
  status:      string;
  word_count:  number;
  elapsed_s:   number;
  note:        string;
};

export type RunBookPipelineResult = {
  project_id:               string;
  character_bible_status:   string;
  world_bible_status:       string;
  scenes:                   BookSceneStageResult[];
  total_elapsed_s:          number;
};

/** One progress event from the book pipeline (fires on `book-pipeline:progress`).
 *  `current` / `total` are the per-scene counters when stage = scene-drafter-fic;
 *  zero for the bible stages. */
export type BookPipelineProgressEvent = {
  /** "character-bible" | "world-bible" | "scene-drafter-fic" */
  stage:     string;
  /** "running" | "completed" | "skipped" | "failed" */
  status:    string;
  summary:   string;
  current:   number;
  total:     number;
  elapsed_s: number;
};

/** Result of `bibles_load` — the writer-supplied bibles persisted to memory.
 *  Mirrors `BiblesLoadResult` in `apps/desktop/src/commands/bibles.rs`. */
export type BiblesLoadResult = {
  /** CharacterCard JSON values. Shape matches `CharacterCard` in
   *  booksforge-domain — `name`, `role`, `external_objective`,
   *  `internal_need`, `fear_or_wound`, `secret_or_contradiction`,
   *  `voice_traits[]`, `relationships[]`, `chapter_arc[]`,
   *  `emotional_turning_points[]`. */
  characters:           unknown[];
  /** WorldBibleProposal JSON or null when no world fields are saved. */
  world:                unknown | null;
  has_character_bible:  boolean;
  has_world_bible:      boolean;
};

export type BiblesSaveInput = {
  characters?: unknown[] | null;
  world?:      unknown | null;
};

export type BiblesSaveResult = {
  characters_written:       number;
  characters_removed:       number;
  world_locations_written:  number;
  world_locations_removed:  number;
  world_fields_written:     string[];
};

/**
 * One row from `publishing_targets_list`. Mirrors the
 * `PublishingTargetRow` Rust struct in `commands/export.rs` (which
 * flattens `booksforge_domain::TargetSpec` for the UI). Defined inline
 * here because the source struct uses `&'static str` slices that can't
 * round-trip through ts-rs's auto-binding.
 */
export type PublishingTargetRow = {
  id: string;
  label: string;
  blurb: string;
  user_briefing: string;
  artifact_formats: string[];
  allowed_trims: Array<{ label: string; width_in: number; height_in: number }>;
  identifier_scheme: "urn_isbn" | "urn_isbn_preferred" | "urn_bf_project";
  toc_depth_max: number;
  image_min_dpi: number;
  cover_min_px: [number, number];
  cover_aspect_x100: number;
  fonts_embedded_required: boolean;
  pdfx_required: boolean;
  accessibility_required: boolean;
  epubcheck_required: boolean;
};

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
  /**
   * Remove a single project from the recent-projects list.  Does NOT
   * delete the bundle on disk — only the entry in the picker.  Returns
   * the post-removal list so the caller can re-render in one round-trip.
   */
  projectRecentRemove(input: RecentRemoveInput): Promise<RecentProjectEntry[]> {
    return invoke("project_recent_remove", { input });
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
  agentApplyChapterDrafter(
    input: ApplyChapterDrafterInput,
  ): Promise<ApplyChapterDrafterResultDto> {
    return invoke("agent_apply_chapter_drafter", { input });
  },
  // Fiction agents (BACKLOG §A13 / Phase 1).
  agentRunCharacterBible(input: RunCharacterBibleInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_character_bible", { input });
  },
  agentApplyCharacterBible(
    input: ApplyCharacterBibleInput,
  ): Promise<ApplyCharacterBibleResultDto> {
    return invoke("agent_apply_character_bible", { input });
  },
  agentRunWorldBible(input: RunWorldBibleInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_world_bible", { input });
  },
  agentApplyWorldBible(input: ApplyWorldBibleInput): Promise<ApplyWorldBibleResultDto> {
    return invoke("agent_apply_world_bible", { input });
  },
  agentRunSceneDrafterFic(input: RunSceneDrafterFicInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_scene_drafter_fic", { input });
  },
  agentApplySceneDrafterFic(
    input: ApplySceneDrafterFicInput,
  ): Promise<ApplySceneDrafterFicResultDto> {
    return invoke("agent_apply_scene_drafter_fic", { input });
  },
  /**
   * One-click scene factory: drafts a scene with the genre-correct
   * drafter, runs the scene critic, applies the 4 polish passes in
   * genre-specific order, and finishes with an AI-tells density scan.
   * Per-stage progress events fire on `pipeline:progress`.
   * (Phase 4E of PRODUCT_ROADMAP_E2E.md.)
   */
  agentRunFullScenePipeline(
    input: RunFullScenePipelineInput,
  ): Promise<RunFullScenePipelineResult> {
    return invoke("agent_run_full_scene_pipeline", { input });
  },
  /**
   * Book-level pipeline — runs character-bible → world-bible → for-each
   * scene drafter (skipping already-drafted scenes by default).  Auto-
   * resolves the right model tier per agent (Medium for bibles, Heavy
   * for drafter).  Emits `book-pipeline:progress` events while running.
   */
  agentRunBookPipeline(
    input: RunBookPipelineInput,
  ): Promise<RunBookPipelineResult> {
    return invoke("agent_run_book_pipeline", { input });
  },
  /**
   * Subscribe to book-pipeline progress events.  Returns an unlisten
   * function the caller MUST invoke on cleanup.
   */
  onBookPipelineProgress(
    cb: (e: BookPipelineProgressEvent) => void,
  ): Promise<UnlistenFn> {
    return listen<BookPipelineProgressEvent>("book-pipeline:progress", (evt) => cb(evt.payload));
  },
  /**
   * Load any writer-supplied bibles already persisted to project memory.
   * The book pipeline's auto-skip logic uses `has_*` to decide whether
   * to run the LLM bible stages or short-circuit to drafting scenes.
   */
  biblesLoad(): Promise<BiblesLoadResult> {
    return invoke("bibles_load");
  },
  /**
   * Persist writer-supplied bibles to memory. Either array can be
   * `null` to leave that half untouched (so the UI can save just
   * characters without overwriting the world bible).
   */
  biblesSave(input: BiblesSaveInput): Promise<BiblesSaveResult> {
    return invoke("bibles_save", { input });
  },
  // Specialist polish stack (BACKLOG §A15 / Phase 2).
  agentRunPolishStage(input: RunPolishStageInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_polish_stage", { input });
  },
  agentApplyPolish(input: ApplyPolishInput): Promise<ApplyPolishResultDto> {
    return invoke("agent_apply_polish", { input });
  },
  agentRunSceneCritic(input: RunSceneCriticInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_scene_critic", { input });
  },
  // Quality stack (BACKLOG §A16 / Phase 3).
  voiceFingerprint(input: VoiceFingerprintInput): Promise<VoiceFingerprintResult> {
    return invoke("voice_fingerprint", { input });
  },
  voiceAnchorSet(input: VoiceAnchorSetInput): Promise<VoiceAnchorSetResult> {
    return invoke("voice_anchor_set", { input });
  },
  voiceAnchorGet(): Promise<VoiceAnchorGetResult> {
    return invoke("voice_anchor_get");
  },
  stylometricDistanceCompute(
    input: StylometricDistanceInput,
  ): Promise<StylometricDistanceResult> {
    return invoke("stylometric_distance_compute", { input });
  },
  tellsScan(input: TellsScanInput): Promise<TellsScanResult> {
    return invoke("tells_scan", { input });
  },
  genrePackGet(input: GenrePackInput): Promise<GenrePack> {
    return invoke("genre_pack_get", { input });
  },
  // Project classification (Phase 4 / 5B).
  projectKindSet(input: ProjectKindSetInput): Promise<ProjectKindSetResult> {
    return invoke("project_kind_set", { input });
  },
  // Round 5 — manually-edit ProjectBrief (powers the creative_profile
  // injection when the writer adds comp authors / themes / forbidden
  // tropes / era / cultural context / creative seed after intake).
  projectBriefLoad(): Promise<ProjectBriefDto> {
    return invoke("project_brief_load");
  },
  projectBriefSave(input: ProjectBriefSaveInput): Promise<ProjectBriefDto> {
    return invoke("project_brief_save", { input });
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
  /**
   * Phase C — Stage 1 quality gate. Scores the saved brief on five
   * axes (0-10 each) + composite, with up to 5 targeted revision
   * suggestions. Light tier auto-resolves when `input.model` is
   * `null`. Wall-clock ~30-60 s on qwen3.5:9b.
   */
  agentRunConceptScorer(input: RunConceptScorerInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_concept_scorer", { input });
  },
  /**
   * Phase C — Stage 2. Generates a structured Reader Expectation Map
   * from the saved brief; persists it to `book:audience_map` memory.
   * Auto-resolves to Light tier.
   */
  agentRunAudienceMapper(input: RunAudienceMapperInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_audience_mapper", { input });
  },
  /**
   * Phase C — Stage 3 quality gate. Reads the saved CharacterCards
   * (entity scope `character:*`) and scores each on 5 axes
   * (depth, consistency, uniqueness, narrative_usefulness,
   * emotional_impact) + composite, surfaces cross-card findings
   * (duplicate-voice, missing-arc, etc.), and returns targeted
   * field-level edits. Auto-resolves to Medium tier.
   */
  agentRunCharacterCritic(input: RunCharacterCriticInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_character_critic", { input });
  },
  /**
   * Phase C — Stage 4 quality gate. Reads the brief from
   * `book:project_brief` memory and scores the outline JSON passed
   * in by the caller (Stage 7 holds the proposal in component state
   * after `agent_run_outline` returns). Returns a
   * `StructureCriticProposal` with 4-axis scores, structural
   * findings, and per-location edits. Auto-resolves to Medium tier.
   */
  agentRunStructureCritic(input: RunStructureCriticInput): Promise<AgentRunResultDto> {
    return invoke("agent_run_structure_critic", { input });
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
  publishingTargetsList(): Promise<PublishingTargetRow[]> {
    return invoke("publishing_targets_list");
  },
  /**
   * Phase 7 / UX R4 — single action that bundles per-platform packages
   * (KDP / Google Play / Apple Books) under `<bundle>/exports/<platform>/`
   * with a `READY_TO_UPLOAD.md` and a per-item readiness checklist.
   */
  prepareForPublishing(
    input: PrepareForPublishingInput,
  ): Promise<PrepareForPublishingResult> {
    return invoke("prepare_for_publishing", { input });
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

  // ── Stage 6 — cover & boilerplate flow ───────────────────────────────────
  /**
   * Load the project's current cover set. Returns a fully-populated
   * CoverSetDto with all three slots; absent slots are `null`.
   */
  coverLoad(): Promise<CoverSetDto> {
    return invoke("cover_load");
  },
  /**
   * Copy an image file at `source_path` into the bundle's
   * `assets/cover-<slot>.<ext>` and persist its metadata. Returns
   * the full updated CoverSet so the UI can re-render in one
   * round-trip.
   */
  coverImport(input: CoverImportInput): Promise<CoverSetDto> {
    return invoke("cover_import", { input });
  },
  /**
   * Clear a single cover slot from memory. The file on disk is
   * preserved (no destructive ops without explicit confirmation).
   */
  coverRemove(input: CoverRemoveInput): Promise<CoverSetDto> {
    return invoke("cover_remove", { input });
  },
  /**
   * Load the boilerplate pages list, sorted by `order`.
   */
  boilerplateLoad(): Promise<BoilerplatePageDto[]> {
    return invoke("boilerplate_load");
  },
  /**
   * Whole-list upsert. Replaces what is in `book:boilerplate_pages`
   * memory with the new list.
   */
  boilerplateSave(input: BoilerplateSaveInput): Promise<BoilerplateSaveResult> {
    return invoke("boilerplate_save", { input });
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
