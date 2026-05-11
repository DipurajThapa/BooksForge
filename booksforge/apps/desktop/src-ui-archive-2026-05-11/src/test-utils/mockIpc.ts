/**
 * Shared mock for the `ipc` module. Tests `vi.mock("../lib/ipc", () => ...)`
 * and stub the methods they exercise. The defaults below return safe
 * empty/successful responses for every method so a smoke `render(...)`
 * call doesn't crash on the `useEffect` IPC fetches that most panels
 * fire on mount.
 *
 * Pattern in test files:
 *
 * ```ts
 * import { vi } from "vitest";
 * import { mockIpcModule } from "../test-utils/mockIpc";
 * vi.mock("../lib/ipc", () => mockIpcModule());
 * ```
 *
 * Override individual methods inside an `it(...)` by re-mocking:
 *
 * ```ts
 * import { ipc } from "../lib/ipc";
 * (ipc.someMethod as any).mockResolvedValue({ ... });
 * ```
 */
import { vi } from "vitest";

/**
 * Returns a vi.fn() that resolves to the given value. Single argument
 * makes the test setup terser than spelling out `vi.fn().mockResolvedValue(...)`.
 */
function ok<T>(value: T) {
  return vi.fn().mockResolvedValue(value);
}

/** A no-op unlisten function for event subscriptions. */
const unlisten = vi.fn().mockResolvedValue(() => {});

export interface MockIpcOverrides {
  [key: string]: unknown;
}

export function buildMockIpc(overrides: MockIpcOverrides = {}) {
  const base = {
    // ── Project lifecycle ─────────────────────────────────────────────────
    projectCreate: ok({ project_id: "01TESTPROJECT00000000000000", title: "Test", author: "T", bundle_path: "/tmp/test.booksforge", recovered: false }),
    projectOpen:   ok({ project_id: "01TESTPROJECT00000000000000", title: "Test", author: "T", bundle_path: "/tmp/test.booksforge", recovered: false }),
    projectClose:  ok(undefined),
    projectRecent: ok([]),
    projectKindSet: ok({ project_id: "01TESTPROJECT00000000000000", book_kind: "literary-fiction" }),
    projectBriefLoad: ok({ loaded: false, brief_json: {} }),
    projectBriefSave: ok({ loaded: true, brief_json: {} }),

    // ── Document tree ─────────────────────────────────────────────────────
    nodeList: ok([]),
    nodeCreate: ok({ id: "01NODE00000000000000000000", parent_id: null, kind: "scene", title: "T", position: 0, status: "drafting", pov: null, beat: null, target_words: null, created_at: "", updated_at: "", deleted_at: null }),
    nodeUpdate: ok({ id: "01NODE00000000000000000000", parent_id: null, kind: "scene", title: "T", position: 0, status: "drafting", pov: null, beat: null, target_words: null, created_at: "", updated_at: "", deleted_at: null }),
    nodeDelete: ok(undefined),

    // ── Scene content ─────────────────────────────────────────────────────
    sceneSave: ok(undefined),
    sceneLoad: ok(null),

    // ── Recovery ──────────────────────────────────────────────────────────
    recoveryCheck: ok({ has_pending: false, last_seen_at: null }),
    recoveryClear: ok(undefined),

    // ── Ollama ────────────────────────────────────────────────────────────
    ollamaProbe: ok({ running: true, version: "test-0.0.0", base_url: "http://127.0.0.1:11434" }),
    ollamaLaunch: ok(undefined),
    ollamaListModels: ok([]),
    ollamaPull: ok(undefined),
    ollamaSmokeTest: ok({ ok: true, model: "test", elapsed_ms: 0, output_preview: "" }),

    // ── Agent workflows (typed-ish; tests override as needed) ────────────
    agentRunOutline: ok({ run_id: "01R", task_id: "01T", status: "completed", output: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentApplyOutline: ok({ created_node_count: 0, snapshot_id: "01S" }),
    agentApplyCopyedit: ok({ accepted_count: 0, snapshot_id: "01S" }),
    agentApplyChapterDrafter: ok({ word_count: 0, snapshot_id: "01S" }),
    agentRunCharacterBible: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentApplyCharacterBible: ok({ accepted_count: 0, snapshot_id: "01S" }),
    agentRunWorldBible: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentApplyWorldBible: ok({ accepted_count: 0, snapshot_id: "01S" }),
    agentRunSceneDrafterFic: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentApplySceneDrafterFic: ok({ snapshot_id: "01S", word_count: 0 }),
    agentRunPolishStage: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentApplyPolish: ok({ snapshot_id: "01S", word_count: 0 }),
    agentRunSceneCritic: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunFullScenePipeline: ok({ project_id: "01P", node_id: "01N", book_kind: "literary-fiction", stages: [], final_tells_verdict: "PUBLISHABLE", total_elapsed_s: 0 }),

    // Quality
    voiceFingerprint: ok({ profile: { corpus_tokens: 0, sentence_words_mean: 0, sentence_words_stddev: 0, em_dash_per_1000: 0, ly_adverb_per_1000: 0, ai_tell_triad_per_1000: 0, discourse_marker_per_1000: 0, type_token_ratio: 0 } }),
    voiceAnchorSet: ok({ ok: true }),
    voiceAnchorGet: ok({ profile: null }),
    stylometricDistanceCompute: ok({ distance: 0, components: [] }),
    tellsScan: ok({ verdict: "PUBLISHABLE", density_per_1000: 0, hits: [] }),
    genrePackGet: ok({ kind: "literary-fiction", genre_label: "literary_fiction", system_prompt: "", draft_lens: "", critic_axes: [], polish_stack_order: [], rubric_weights: {}, hard_rules: [] }),

    agentApplyHumanization: ok({ accepted_count: 0, snapshot_id: "01S" }),
    agentApplyContinuity: ok({ accepted_count: 0, snapshot_id: "01S" }),
    vocabApplyProposals: ok({ accepted_count: 0 }),
    originalityConsentLoad: ok(null),
    originalityConsentSet: ok(undefined),
    originalityConsentClear: ok(undefined),
    agentRunCopyedit: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunContinuity: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunIntake: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunIntakeAndOutline: ok({ intake_status: "completed", outline_status: "completed", intake_task_id: "01T", outline_task_id: "01T", brief_json: null, outline_json: null, intake_error: null, outline_error: null }),
    agentCancel: ok(undefined),
    agentRunDevelopmentalReview: ok({ dev_status: "completed", dev_task_id: "01T", peer_status: "completed", peer_task_id: "01T", dev_notes_json: null, peer_review_json: null, dev_error: null, peer_error: null }),
    entityBibleApplyProposals: ok({ accepted_count: 0 }),
    onAgentRunStarted: unlisten,
    onAgentRunCompleted: unlisten,
    onAgentRunProgress: unlisten,
    agentRunMemoryCurator: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunVocabDictionary: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunChapterDrafter: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunDevEditor: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunHumanization: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    agentRunProposalValidator: ok({ run_id: "01R", task_id: "01T", status: "completed", proposal_json: null, raw_output: null, verification: { proposal: { ok: true, checks: [] }, peer_reviews: [] }, error: null }),
    voiceFingerprintRefresh: ok(null),
    voiceFingerprintLoad: ok(null),
    originalityScanChapter: ok({ verdict: "PUBLISHABLE", overlaps: [] }),

    // Quick-action presets
    aiSuggest: ok({ job_id: "01J" }),
    aiCancel: ok(undefined),
    aiApply: ok({ snapshot_id: "01S" }),
    onAiSuggestToken: unlisten,
    onAiSuggestDone: unlisten,

    // Snapshots
    snapshotCreate: ok({ id: "01S", trigger: "manual", description: null, scope_id: null, created_at: "" }),
    snapshotList: ok([]),
    snapshotDiff: ok([]),
    snapshotRestore: ok({ restored_count: 0, snapshot_id: "01S" }),

    // Export pipeline
    exportMarkdown: ok({ output_path: "" }),
    exportRun: ok({ output_path: "", validation_ok: true, validation_message: null, validation_errors: 0, validation_warnings: 0 }),
    exportHistory: ok([]),
    exportCheckDependencies: ok({ items: [] }),
    publishingTargetsList: ok([]),
    prepareForPublishing: ok({ project_id: "01P", platforms: [], elapsed_s: 0 }),
    saveDiagnosticBundle: ok({ output_path: "" }),
    appVersion: ok({ major: 0, minor: 0, patch: 1, pre: null }),

    // Validators
    validatorsRun: ok({ issues: [], scope_hash: "", elapsed_ms: 0 }),
    validatorsGate: ok({ outcome: "pass", errors: [], warnings: [], info: [] }),
    validatorsApplyFix: ok({ ok: true, message: null }),

    // Memory + vocab
    memoryList: ok([]),
    vocabList: ok([]),
  };

  return { ipc: { ...base, ...overrides } };
}

/**
 * Vitest mock factory — pass to `vi.mock("../lib/ipc", () => mockIpcModule())`.
 * Re-exports the same shape `lib/ipc.ts` does.
 */
export function mockIpcModule(overrides: MockIpcOverrides = {}) {
  return buildMockIpc(overrides);
}
