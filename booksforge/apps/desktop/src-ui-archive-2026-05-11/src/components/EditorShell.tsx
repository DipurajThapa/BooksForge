/**
 * Three-pane editor shell: Binder (left) | Editor (center) | [right panel stub]
 *
 * Manages:
 * - Node list refresh
 * - Scene selection and content loading
 * - Crash recovery check on mount
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { NodeInfo, OpenProjectResult, RecoveryStatus, SceneLoadResult } from "@booksforge/shared-types";
import {
  EditorToolbar,
  SceneEditor,
  type SceneEditorHandle,
  type JSONContent,
  type Editor as TiptapEditor,
} from "@booksforge/editor";
import AgentDebugForm from "./AgentDebugForm";
import AgentsPanel from "./agents/AgentsPanel";
import BiblesPanel from "./BiblesPanel";
import BookGenerationPanel from "./BookGenerationPanel";
import ExportPanel from "./ExportPanel";
import ModePicker from "./ModePicker";
import MoreMenu, { type MoreMenuItem } from "./MoreMenu";
import {
  type AppMode,
  loadAppMode,
  modeEmoji,
  modeLabel,
  setAppMode,
} from "../lib/appMode";
import PrepareForPublishingPanel from "./PrepareForPublishingPanel";
import WorkflowGuide from "./WorkflowGuide";
import BriefEditorPanel from "./BriefEditorPanel";
import HelpDrawer from "./HelpDrawer";
import LiveRunOverlay from "./LiveRunOverlay";
import OnboardingTour, { shouldShowOnboarding } from "./OnboardingTour";
import SettingsPanel from "./SettingsPanel";
import Binder from "./Binder";
import FindReplaceBar from "./FindReplaceBar";
import InspectorPanel from "./InspectorPanel";
import OllamaWizard from "./OllamaWizard";
import QuickActionBar from "./QuickActionBar";
import KnowledgePanel from "./KnowledgePanel";
import RecoveryDialog from "./RecoveryDialog";
import SnapshotsPanel from "./SnapshotsPanel";
import ValidatorPanel from "./ValidatorPanel";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
interface Props {
  project: OpenProjectResult;
  onClose: () => void;
}

export default function EditorShell({ project, onClose }: Props) {
  const [nodes, setNodes] = useState<NodeInfo[]>([]);
  // Node-list fetch states drive the Binder's empty / loading / error UI
  // (audit #60).  We start in `loading` so a slow IPC round-trip doesn't
  // briefly flash the "No scenes yet" empty state.
  const [nodesLoading, setNodesLoading] = useState(true);
  const [nodesError,   setNodesError]   = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<NodeInfo | null>(null);
  const [sceneContent, setSceneContent] = useState<SceneLoadResult | null>(null);
  const [recovery, setRecovery] = useState<RecoveryStatus | null>(null);
  const [saving, setSaving] = useState(false);
  const [showOllamaWizard, setShowOllamaWizard] = useState(false);
  const [showAgentDebug,   setShowAgentDebug]   = useState(false);
  const [showSnapshots,    setShowSnapshots]    = useState(false);
  const [showQuickAction,  setShowQuickAction]  = useState(false);
  const [showValidators,   setShowValidators]   = useState(false);
  const [showKnowledge,    setShowKnowledge]    = useState(false);
  const [showAgents,       setShowAgents]       = useState(false);
  const [showBibles,       setShowBibles]       = useState(false);
  const [showBookGen,      setShowBookGen]      = useState(false);
  const [showExport,       setShowExport]       = useState(false);
  const [showPublishing,   setShowPublishing]   = useState(false);
  const [showWorkflow,     setShowWorkflow]     = useState(false);
  const [showBrief,        setShowBrief]        = useState(false);
  const [showSettings,     setShowSettings]     = useState(false);
  const [showHelp,         setShowHelp]         = useState(false);
  const [showOnboarding,   setShowOnboarding]   = useState(() => shouldShowOnboarding());
  const [showFindReplace,  setShowFindReplace]  = useState(false);
  const [exporting,        setExporting]        = useState(false);
  const [exportToast,      setExportToast]      = useState<string | null>(null);
  // D5 — focus / distraction-free mode.  Hides the binder + inspector +
  // status bar so only the prose remains.  Toggle via the Focus button or
  // ⌘. (Mac) / Ctrl+. (Win/Linux).
  const [focusMode,        setFocusMode]        = useState(false);
  // App-mode (manual vs ai_writer) reshapes the toolbar's primary CTA
  // and the empty-scene editor state. `null` triggers the first-time
  // ModePicker overlay so the writer commits to a path.
  const [appMode,          setAppModeState]     = useState<AppMode | null>(() => loadAppMode());
  // Position state for the ⋯ overflow menu, anchored to the trigger.
  const [moreOpen,         setMoreOpen]         = useState(false);
  const [moreAnchor,       setMoreAnchor]       = useState<{ top: number; right: number }>({ top: 0, right: 0 });
  const moreBtnRef = useRef<HTMLButtonElement>(null);
  const editorHandleRef = useRef<SceneEditorHandle>(null);
  // Held in state (not just a ref) so the toolbar re-renders when the
  // editor is mounted/unmounted as the user switches scenes.
  const [editorInstance, setEditorInstance] = useState<TiptapEditor | null>(null);
  // Live word count of the active scene; updated on every keystroke via
  // the SceneEditor `onSave` callback.
  const [liveSceneWords, setLiveSceneWords] = useState<number>(0);
  // Project total at session start, used to compute "today" delta.
  const sessionBaselineRef = useRef<number | null>(null);

  // Quick Markdown export via the OS save-file dialog (Cmd/Ctrl+E).
  // Gates on the validator: errors block, warnings prompt, info silent.
  // The full multi-format export lives in ExportPanel; this is the fast path.
  const handleExport = useCallback(async () => {
    if (exporting) return;
    setExporting(true);
    setExportToast(null);

    // Pre-export gate (Phase 4).
    try {
      const gate = await ipc.validatorsGate();
      if (gate.outcome === "block") {
        setExportToast(
          `Export blocked — ${gate.errors.length} error(s) must be fixed first.`
        );
        window.setTimeout(() => setExportToast(null), 6000);
        setShowValidators(true);
        setExporting(false);
        return;
      }
      if (gate.outcome === "warn") {
        const ok = window.confirm(
          `${gate.warnings.length} warning(s) detected. Export anyway?`,
        );
        if (!ok) {
          setExporting(false);
          setShowValidators(true);
          return;
        }
      }
    } catch (e) {
      // If the gate itself fails, surface and bail — don't ship a possibly-
      // broken manuscript silently.
      setExportToast(`Pre-export check failed: ${errorMessage(e)}`);
      window.setTimeout(() => setExportToast(null), 6000);
      setExporting(false);
      return;
    }

    const safeTitle = project.title.trim().replace(/[^a-zA-Z0-9_\- ]/g, "") || "manuscript";
    const target = await saveDialog({
      title: "Export manuscript as Markdown",
      defaultPath: `${safeTitle}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    }).catch(() => null);
    if (!target) { setExporting(false); return; }

    try {
      const result = await ipc.exportMarkdown({ output_path: target });
      setExportToast(
        `Exported · ${result.scene_count} scenes · ${result.word_count.toLocaleString()} words`
      );
      window.setTimeout(() => setExportToast(null), 5000);
    } catch (e) {
      setExportToast(`Export failed: ${errorMessage(e)}`);
      window.setTimeout(() => setExportToast(null), 6000);
    } finally {
      setExporting(false);
    }
  }, [exporting, project.title]);

  // Load node list.  Errors are surfaced to the Binder so the user can
  // see what failed and retry — the previous behaviour silently swallowed
  // them and rendered the empty state, which masked outages.
  const refreshNodes = useCallback(async () => {
    setNodesLoading(true);
    try {
      const list = await ipc.nodeList();
      setNodes(list);
      setNodesError(null);
      // Capture the project word count once on first successful load — that
      // becomes the "today" baseline so the status bar can show session delta.
      if (sessionBaselineRef.current === null) {
        const total = list
          .filter((n) => !n.parent_id || n.kind === "project")
          .reduce((sum, n) => sum + n.word_count, 0);
        sessionBaselineRef.current = total;
      }
    } catch (e) {
      setNodesError(String(e));
    } finally {
      setNodesLoading(false);
    }
  }, []);

  // Cmd/Ctrl+K opens the quick-action bar; Cmd/Ctrl+. toggles focus mode.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey)) {
        if ((e.key === "k" || e.key === "K") && selectedNode?.kind === "scene") {
          e.preventDefault();
          setShowQuickAction(true);
          return;
        }
        if (e.key === ".") {
          e.preventDefault();
          setFocusMode((on) => !on);
          return;
        }
        if ((e.key === "f" || e.key === "F") && selectedNode?.kind === "scene") {
          e.preventDefault();
          setShowFindReplace(true);
          return;
        }
        if (e.key === "e" || e.key === "E") {
          e.preventDefault();
          void handleExport();
          return;
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selectedNode, handleExport]);

  // On mount: load nodes first, then check for crash recovery (so the node is
  // already in the list when the dialog's onRestore tries to select it).
  useEffect(() => {
    (async () => {
      await refreshNodes();
      const status = await ipc.recoveryCheck().catch(() => null);
      if (status?.has_pending) setRecovery(status);
    })();
  }, [refreshNodes]);

  // Load scene content when selection changes.
  useEffect(() => {
    if (!selectedNode || selectedNode.kind !== "scene") {
      setSceneContent(null);
      return;
    }
    ipc.sceneLoad(selectedNode.id).then((content) => {
      setSceneContent(content);
    }).catch(() => null);
  }, [selectedNode]);

  const handleSave = useCallback(
    async (doc: JSONContent, wordCount: number, charCount: number) => {
      if (!selectedNode) return;
      setLiveSceneWords(wordCount);
      setSaving(true);
      try {
        await ipc.sceneSave({
          node_id: selectedNode.id,
          pm_doc: doc,
          word_count: wordCount,
          char_count: charCount,
        });
        await refreshNodes(); // pulls fresh rollups for the binder + status bar
      } catch {
        // Best-effort — don't crash the UI on autosave failure.
      } finally {
        setSaving(false);
      }
    },
    [selectedNode, refreshNodes]
  );

  async function handleRecoveryRestore() {
    // The pending node is already in the recovery log — just clear the flag.
    // The next load will pick up the SQLite state (which may or may not have
    // the pending content — see recovery.rs).  In a future MZ we'll wire
    // the JSONL content directly into the editor here.
    await ipc.recoveryClear().catch(() => null);
    setRecovery(null);
    if (recovery?.node_id) {
      const node = nodes.find((n) => n.id === recovery.node_id) ?? null;
      if (node) setSelectedNode(node);
    }
  }

  async function handleRecoveryDiscard() {
    await ipc.recoveryClear().catch(() => null);
    setRecovery(null);
  }

  return (
    <div style={s.shell}>
      {/* ── Header — 2026-05 redesign ──
          Reduced from 14 equal-weight buttons to:
            [BooksForge] [project title]   [Mode pill] [Primary CTA] [⋯] [Close]
          See book-output/design/UX_REDESIGN_2026-05.md for the rationale. */}
      <header style={s.header}>
        <span style={s.wordmark}>BooksForge</span>
        <span style={s.projectTitle}>{project.title}</span>
        <div style={s.headerRight}>
          {saving && <span style={s.savingDot} title="Saving…" />}

          {/* Mode pill — always visible. Click to swap mode. The mode
              shapes the primary CTA + the empty-scene state below. */}
          {appMode && (
            <button
              style={s.modePill}
              onClick={() => {
                const next: AppMode = appMode === "ai_writer" ? "manual" : "ai_writer";
                setAppMode(next);
                setAppModeState(next);
              }}
              title={`Currently ${modeLabel(appMode)} mode — click to switch`}
              aria-label={`Switch from ${modeLabel(appMode)} to ${modeLabel(appMode === "ai_writer" ? "manual" : "ai_writer")} mode`}
            >
              <span style={s.modePillEmoji}>{modeEmoji(appMode)}</span>
              <span>{modeLabel(appMode)}</span>
            </button>
          )}

          {/* Primary CTA — context-bound. AI mode lights up; manual
              mode shows nothing here (writer just types). */}
          {appMode === "ai_writer" && renderPrimaryCta(
            selectedNode,
            sceneContent,
            () => setShowBookGen(true),
            () => selectedNode && void runSingleSceneFromCta(selectedNode, project.project_id),
          )}

          {/* ⋯ overflow — every other action lives here. */}
          <button
            ref={moreBtnRef}
            style={s.moreBtn}
            onClick={() => {
              const r = moreBtnRef.current?.getBoundingClientRect();
              if (r) {
                setMoreAnchor({
                  top: r.bottom + 4,
                  right: window.innerWidth - r.right,
                });
              }
              setMoreOpen((o) => !o);
            }}
            title="More options"
            aria-label="More options"
            aria-expanded={moreOpen}
          >
            ⋯
          </button>

          <button
            style={s.closeBtn}
            onClick={() => {
              ipc.projectClose().catch(() => null);
              onClose();
            }}
          >
            Close
          </button>
        </div>
      </header>

      {/* ⋯ menu portal — anchored under the trigger. Items are grouped
          by purpose; the writer can find any panel from here. */}
      <MoreMenu
        open={moreOpen}
        onClose={() => setMoreOpen(false)}
        anchor={moreAnchor}
        items={buildMoreMenuItems({
          focusMode,
          hasScene: selectedNode?.kind === "scene",
          onShowBrief:      () => setShowBrief(true),
          onShowBibles:     () => setShowBibles(true),
          onShowKnowledge:  () => setShowKnowledge(true),
          onShowWorkflow:   () => setShowWorkflow(true),
          onShowAgents:     () => setShowAgents(true),
          onShowOllama:     () => setShowOllamaWizard(true),
          onShowAgentDebug: () => setShowAgentDebug(true),
          onShowValidators: () => setShowValidators(true),
          onShowSnapshots:  () => setShowSnapshots(true),
          onShowExport:     () => setShowExport(true),
          onShowPublishing: () => setShowPublishing(true),
          onToggleFocus:    () => setFocusMode((on) => !on),
          onShowSettings:   () => setShowSettings(true),
          onShowHelp:       () => setShowHelp(true),
        })}
      />

      {/* ── Body ── */}
      <div style={s.body}>
        {!focusMode && (
          <Binder
            nodes={nodes}
            selectedId={selectedNode?.id ?? null}
            onSelect={setSelectedNode}
            onNodesChanged={refreshNodes}
            loading={nodesLoading}
            error={nodesError}
            onRetry={refreshNodes}
          />
        )}

        <main style={s.center}>
          {selectedNode?.kind === "scene" ? (
            <>
              <EditorToolbar editor={editorInstance} />
              <SceneEditor
                key={selectedNode.id}
                ref={editorHandleRef}
                initialDoc={
                  sceneContent ? (sceneContent.pm_doc as JSONContent) : null
                }
                onSave={handleSave}
                saveDelay={5000}
                onEditorReady={setEditorInstance}
              />
            </>
          ) : (
            <div style={s.noScene}>
              {renderEmptyState({
                hasScenes: nodes.length > 1,
                appMode,
                onGenerateBook: () => setShowBookGen(true),
              })}
            </div>
          )}
        </main>

        {/* Right panel — outline / metadata inspector (Phase 2). */}
        {!focusMode && (
          <InspectorPanel
            node={selectedNode}
            onSaved={refreshNodes}
          />
        )}
      </div>

      {/* ── Status bar (hidden in focus mode) ── */}
      {!focusMode && (
        <StatusBar
          nodes={nodes}
          selectedNode={selectedNode}
          liveSceneWords={liveSceneWords}
          sessionBaseline={sessionBaselineRef.current ?? 0}
          saving={saving}
        />
      )}

      {/* ── Crash recovery dialog ── */}
      {recovery && (
        <RecoveryDialog
          status={recovery}
          onRestore={handleRecoveryRestore}
          onDiscard={handleRecoveryDiscard}
        />
      )}

      {/* ── Ollama Setup Wizard ── */}
      {showOllamaWizard && (
        <OllamaWizard
          onClose={() => setShowOllamaWizard(false)}
          onComplete={() => setShowOllamaWizard(false)}
        />
      )}

      {/* ── MZ-05 Agent debug form ── */}
      {showAgentDebug && (
        <AgentDebugForm
          projectId={project.project_id}
          onClose={() => setShowAgentDebug(false)}
        />
      )}

      {/* ── MZ-06 Snapshots panel ── */}
      {showSnapshots && (
        <SnapshotsPanel onClose={() => setShowSnapshots(false)} />
      )}

      {/* ── Phase 4 Validator panel ── */}
      {showValidators && (
        <ValidatorPanel
          onClose={() => setShowValidators(false)}
          onSelectNode={(nodeId) => {
            const target = nodes.find((n) => n.id === nodeId);
            if (target) {
              setSelectedNode(target);
              setShowValidators(false);
            }
          }}
        />
      )}

      {/* ── Turn B Knowledge panel ── */}
      {showKnowledge && (
        <KnowledgePanel onClose={() => setShowKnowledge(false)} />
      )}

      {/* ── §E0d.9 Agents panel ── */}
      {showAgents && (
        <AgentsPanel
          projectId={project.project_id}
          sceneId={selectedNode?.kind === "scene" ? selectedNode.id : null}
          onClose={() => setShowAgents(false)}
          onApplied={() => {
            // After an agent's Apply writes new pm_doc into the scene,
            // reload it so the editor shows the freshly drafted prose.
            if (selectedNode?.kind === "scene") {
              ipc.sceneLoad(selectedNode.id).then(setSceneContent).catch(() => null);
            }
          }}
        />
      )}

      {/* ── Bibles editor (writer-supplied character + world bibles) ── */}
      {showBibles && (
        <BiblesPanel onClose={() => setShowBibles(false)} />
      )}

      {/* ── Book pipeline (bibles → per-scene drafter) ── */}
      {showBookGen && (
        <BookGenerationPanel
          projectId={project.project_id}
          sceneCount={nodes.filter((n) => n.kind === "scene").length}
          onClose={() => setShowBookGen(false)}
          onSceneApplied={() => {
            // Refresh the binder + reload current scene's prose so the
            // user sees freshly-drafted scenes the moment they land.
            void refreshNodes();
            if (selectedNode?.kind === "scene") {
              ipc.sceneLoad(selectedNode.id).then(setSceneContent).catch(() => null);
            }
          }}
        />
      )}

      {/* ── Phase 6 Export panel ── */}
      {showExport && (
        <ExportPanel onClose={() => setShowExport(false)} />
      )}

      {/* ── Phase 7 Prepare-for-Publishing panel (UX R4) ── */}
      {showPublishing && (
        <PrepareForPublishingPanel onClose={() => setShowPublishing(false)} />
      )}

      {/* ── Phase 9 Workflow approval gates (UX R6) ── */}
      {showWorkflow && (
        <WorkflowGuide
          projectId={project.project_id}
          onClose={() => setShowWorkflow(false)}
        />
      )}

      {/* ── Round 5 Brief editor (drives creative_profile injection) ── */}
      {showBrief && (
        <BriefEditorPanel onClose={() => setShowBrief(false)} />
      )}

      {/* ── §B4 Settings panel ── */}
      {showSettings && (
        <SettingsPanel onClose={() => setShowSettings(false)} />
      )}

      {/* ── §I4 Help drawer ── */}
      {showHelp && (
        <HelpDrawer
          onClose={() => setShowHelp(false)}
          onReplayWelcome={() => setShowOnboarding(true)}
        />
      )}

      {/* ── §I5 Onboarding tour ── */}
      {showOnboarding && (
        <OnboardingTour onClose={() => setShowOnboarding(false)} />
      )}

      {/* ── §E4 Live agent-run overlay ── */}
      <LiveRunOverlay />

      {/* ── First-time mode picker. Non-dismissible until the writer
            commits to manual or AI Writer. See UX_REDESIGN_2026-05.md. */}
      {appMode === null && (
        <ModePicker onChosen={(m) => setAppModeState(m)} />
      )}

      {/* Export toast */}
      {exportToast && (
        <div style={s.toast} role="status">{exportToast}</div>
      )}

      {/* ── D3 Find / Replace bar (Cmd/Ctrl+F) ── */}
      <FindReplaceBar
        open={showFindReplace}
        editor={editorInstance}
        onClose={() => setShowFindReplace(false)}
      />

      {/* ── MZ-08 Quick-action bar (Cmd/Ctrl+K) ── */}
      {showQuickAction && selectedNode?.kind === "scene" && (
        <QuickActionBar
          open={showQuickAction}
          nodeId={selectedNode.id}
          getScopeText={() => editorHandleRef.current?.getSelectionText() ?? ""}
          onClose={() => setShowQuickAction(false)}
          onApplied={() => {
            // Reload scene content from storage so the editor shows the new prose.
            ipc.sceneLoad(selectedNode.id).then(setSceneContent).catch(() => null);
          }}
        />
      )}
    </div>
  );
}

// ── StatusBar ─────────────────────────────────────────────────────────────────

// ── Toolbar helpers (2026-05 redesign) ─────────────────────────────────────

/**
 * Render the AI-mode primary CTA. Resolves to one of:
 *   - "Generate Book"    when no scene is selected (or project root)
 *   - "Generate scene"   when an empty scene is open
 *   - "Refine scene"     when a drafted scene is open
 * Each button is amber + filled to signal it's the headline action.
 */
function renderPrimaryCta(
  selectedNode: NodeInfo | null,
  sceneContent: SceneLoadResult | null,
  onGenerateBook: () => void,
  onGenerateScene: () => void,
): React.ReactNode {
  if (!selectedNode || selectedNode.kind !== "scene") {
    return (
      <button style={primaryCtaStyles.btn} onClick={onGenerateBook}>
        ✨ Generate Book
      </button>
    );
  }
  const wordCount = sceneContent?.word_count ?? selectedNode.word_count ?? 0;
  if (wordCount < 50) {
    return (
      <button style={primaryCtaStyles.btn} onClick={onGenerateScene}>
        ✨ Generate scene
      </button>
    );
  }
  return (
    <button style={primaryCtaStyles.btnSecondary} onClick={onGenerateScene}>
      ✨ Refine scene
    </button>
  );
}

/** Stub — wires up to the existing per-scene full-pipeline command on
 *  the next iteration. For now, it points the user at the Agents panel
 *  so they can pick critic/polish/drafter for the open scene. */
async function runSingleSceneFromCta(_node: NodeInfo, _projectId: string): Promise<void> {
  // TODO(next iteration): call ipc.agentRunFullScenePipeline directly
  // with sensible defaults (use scene title as the goal, etc.). For
  // now, opening the Agents panel preserves the current proven flow
  // — the AgentsPanel is already wired with the right per-agent
  // dispatch and can run scene-drafter-fic on the open scene.
  // The button remains useful because it tells the writer "yes,
  // there's a one-click way; here's the panel" — the scope shrinks
  // to writing the wiring without changing the user-visible CTA.
  alert(
    "Single-scene generation: open the Agents panel via ⋯ → Agents and run \"scene-drafter-fic\" on this scene.\n\n(Next commit will wire this CTA directly to that pipeline so you don't have to navigate the menu.)",
  );
}

interface BuildMenuArgs {
  focusMode:        boolean;
  hasScene:         boolean;
  onShowBrief:      () => void;
  onShowBibles:     () => void;
  onShowKnowledge:  () => void;
  onShowWorkflow:   () => void;
  onShowAgents:     () => void;
  onShowOllama:     () => void;
  onShowAgentDebug: () => void;
  onShowValidators: () => void;
  onShowSnapshots:  () => void;
  onShowExport:     () => void;
  onShowPublishing: () => void;
  onToggleFocus:    () => void;
  onShowSettings:   () => void;
  onShowHelp:       () => void;
}

/**
 * Items for the ⋯ overflow menu, grouped by purpose. Dividers separate
 * groups. Order matches `book-output/design/UX_REDESIGN_2026-05.md`.
 */
function buildMoreMenuItems(args: BuildMenuArgs): MoreMenuItem[] {
  return [
    { label: "Brief",            hint: "Comp authors, themes, era — drives every agent.", onSelect: args.onShowBrief },
    { label: "Bibles",           hint: "Add character + world bibles yourself — skips the AI bible stages and saves 5–10 min per run.", onSelect: args.onShowBibles },
    { label: "Knowledge",        hint: "Project memory + vocabulary lists.",                onSelect: args.onShowKnowledge },

    { label: "Approval gates",   hint: "Track topic / plan / bibles / polish gates.", onSelect: args.onShowWorkflow,   divider: true },
    { label: "Agents",           hint: "Run any single agent on the open scene.",      onSelect: args.onShowAgents,    disabled: !args.hasScene },
    { label: "AI Setup",         hint: "Pull / probe local Ollama models.",           onSelect: args.onShowOllama },
    { label: "Debug AI",         hint: "Manually invoke the outline agent.",          onSelect: args.onShowAgentDebug },

    { label: "Manuscript checks", hint: "Run all 16 validators (HRC, KDP, AI-tells…).", onSelect: args.onShowValidators, divider: true },
    { label: "Snapshots",        hint: "Manual save points + restore.",                onSelect: args.onShowSnapshots },

    { label: "Export…",          hint: "Markdown / EPUB / DOCX / PDF.",                onSelect: args.onShowExport,    divider: true },
    { label: "Prepare for publishing", hint: "KDP / Apple Books / Google Play packages.", onSelect: args.onShowPublishing },

    { label: args.focusMode ? "Exit Focus mode" : "Focus mode (⌘.)", hint: "Hide binder + inspector.", onSelect: args.onToggleFocus, divider: true },
    { label: "Settings",         hint: "Telemetry, originality, dependencies.",        onSelect: args.onShowSettings },
    { label: "Help",             hint: "Keyboard shortcuts, agent guide.",             onSelect: args.onShowHelp },
  ];
}

/**
 * Render the editor's empty-pane state. Mode-aware:
 *   - AI Writer + no scenes        → hero CTA "Generate Book"
 *   - AI Writer + scenes, none open → "Select a scene, or generate one"
 *   - Manual + no scenes            → instructions for adding manually
 *   - Manual + scenes, none open    → simple "Select a scene" prompt
 *   - Mode unset                    → conservative pre-mode hint
 */
function renderEmptyState(args: {
  hasScenes: boolean;
  appMode:   AppMode | null;
  onGenerateBook: () => void;
}): React.ReactNode {
  const { hasScenes, appMode, onGenerateBook } = args;
  if (appMode === "ai_writer" && !hasScenes) {
    return (
      <div style={emptyStateStyles.hero}>
        <p style={emptyStateStyles.heroTitle}>Your book is ready to be drafted.</p>
        <p style={emptyStateStyles.heroBody}>
          BooksForge will run the full pipeline locally: character bible,
          world bible, then prose for every scene in your outline. You
          review and refine.
        </p>
        <button style={emptyStateStyles.heroBtn} onClick={onGenerateBook}>
          ✨ Generate Book
        </button>
        <p style={emptyStateStyles.heroFootnote}>
          Or open <b>⋯ → Brief</b> first to set comp authors and themes,
          then come back here.
        </p>
      </div>
    );
  }
  if (appMode === "ai_writer" && hasScenes) {
    return (
      <div style={emptyStateStyles.hero}>
        <p style={emptyStateStyles.heroTitle}>Select a scene from the left.</p>
        <p style={emptyStateStyles.heroBody}>
          Empty scenes can be drafted with one click; drafted scenes can
          be refined.
        </p>
        <button style={emptyStateStyles.heroBtn} onClick={onGenerateBook}>
          ✨ Generate every scene at once
        </button>
      </div>
    );
  }
  if (appMode === "manual" && !hasScenes) {
    return (
      <div style={emptyStateStyles.hero}>
        <p style={emptyStateStyles.heroTitle}>This project has no scenes yet.</p>
        <p style={emptyStateStyles.heroBody}>
          Add a chapter and scene from the binder on the left
          (<b>+ Scene</b> under any chapter), or open
          <b> ⋯ → Brief</b> to record your premise first.
        </p>
        <p style={emptyStateStyles.heroFootnote}>
          Want AI to draft for you? Switch to <b>🤖 AI Writer</b> using the
          mode pill in the toolbar.
        </p>
      </div>
    );
  }
  if (appMode === "manual" && hasScenes) {
    return (
      <p style={emptyStateStyles.simple}>
        Select a scene in the binder on the left to start writing.
      </p>
    );
  }
  // Mode not picked yet — the ModePicker overlay is showing on top of
  // this state, so this copy is rarely visible. Be conservative.
  return (
    <p style={emptyStateStyles.simple}>
      Pick a writing mode to get started.
    </p>
  );
}

const emptyStateStyles: Record<string, React.CSSProperties> = {
  hero: {
    maxWidth: 540,
    padding: "var(--space-6)",
    color: "var(--color-text-primary)",
    fontFamily: "var(--font-ui)",
    textAlign: "center",
    display: "flex", flexDirection: "column", alignItems: "center",
    gap: "var(--space-3)",
  },
  heroTitle: {
    fontSize: 22, fontWeight: 700,
    margin: 0,
    fontFamily: "var(--font-prose)",
    color: "var(--color-text-primary)",
  },
  heroBody: {
    fontSize: 14, color: "var(--color-text-secondary)",
    lineHeight: 1.6, margin: 0,
  },
  heroBtn: {
    background: "var(--color-amber-600)",
    color: "#fff",
    border: "1px solid var(--color-amber-600)",
    borderRadius: 6,
    fontSize: 15, fontWeight: 600,
    padding: "10px var(--space-5)",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
    marginTop: "var(--space-2)",
  },
  heroFootnote: {
    fontSize: 12, color: "var(--color-text-tertiary)",
    margin: "var(--space-2) 0 0",
  },
  simple: {
    color: "var(--color-text-tertiary)", fontSize: 14, margin: 0, textAlign: "center",
  },
};

const primaryCtaStyles: Record<string, React.CSSProperties> = {
  btn: {
    background: "var(--color-amber-600)",
    color: "#fff",
    border: "1px solid var(--color-amber-600)",
    borderRadius: 5,
    fontSize: 12, fontWeight: 700,
    padding: "4px var(--space-3)",
    cursor: "pointer", fontFamily: "var(--font-ui)",
  },
  btnSecondary: {
    background: "transparent",
    color: "var(--color-amber-600)",
    border: "1px solid var(--color-amber-600)",
    borderRadius: 5,
    fontSize: 12, fontWeight: 600,
    padding: "4px var(--space-3)",
    cursor: "pointer", fontFamily: "var(--font-ui)",
  },
};

interface StatusBarProps {
  nodes:           NodeInfo[];
  selectedNode:    NodeInfo | null;
  liveSceneWords:  number;
  sessionBaseline: number;
  saving:          boolean;
}

function StatusBar({ nodes, selectedNode, liveSceneWords, sessionBaseline, saving }: StatusBarProps) {
  // Project total = sum of every leaf scene's count. Do this from the
  // node_list response so we don't need a second IPC.
  const projectWords = nodes
    .filter((n) => n.kind === "scene" || n.kind === "front_matter" || n.kind === "back_matter")
    .reduce((sum, n) => sum + n.word_count, 0);

  // Selected scene word count: prefer live (matches what the user sees as
  // they type) over the persisted count (one autosave-cycle stale).
  const sceneWords = selectedNode?.kind === "scene"
    ? Math.max(liveSceneWords, selectedNode.word_count)
    : 0;
  const sceneTarget = selectedNode?.target_words ?? null;

  // Chapter rollup — walk up to nearest chapter ancestor.
  let chapterWords: number | null = null;
  if (selectedNode) {
    let cur: NodeInfo | undefined = selectedNode;
    while (cur && cur.kind !== "chapter") {
      cur = cur.parent_id ? nodes.find((n) => n.id === cur!.parent_id) : undefined;
    }
    if (cur) chapterWords = cur.word_count;
  }

  const today = projectWords - sessionBaseline;

  return (
    <footer style={statusBarStyles.bar} role="status">
      <span style={statusBarStyles.cell}>
        <b>{projectWords.toLocaleString()}</b> total
      </span>
      {chapterWords !== null && (
        <span style={statusBarStyles.cell}>
          <b>{chapterWords.toLocaleString()}</b> chapter
        </span>
      )}
      {selectedNode?.kind === "scene" && (
        <span style={statusBarStyles.cell}>
          <b>{sceneWords.toLocaleString()}</b>
          {sceneTarget ? ` / ${sceneTarget.toLocaleString()}` : ""} scene
        </span>
      )}
      {today !== 0 && (
        <span
          style={{
            ...statusBarStyles.cell,
            color: today > 0 ? "var(--color-success, #22c55e)" : "var(--color-error, #ef4444)",
          }}
        >
          {today > 0 ? "+" : ""}{today.toLocaleString()} today
        </span>
      )}
      <span style={statusBarStyles.spacer} />
      {saving && <span style={statusBarStyles.cellMuted}>Saving…</span>}
    </footer>
  );
}

const statusBarStyles: Record<string, React.CSSProperties> = {
  bar: {
    height: 28,
    flexShrink: 0,
    display: "flex",
    alignItems: "center",
    gap: 16,
    padding: "0 var(--space-4)",
    borderTop: "1px solid var(--color-border)",
    background: "var(--color-surface-raised, #fafafa)",
    fontSize: 11,
    color: "var(--color-text-secondary)",
    fontVariantNumeric: "tabular-nums",
  },
  cell:      { whiteSpace: "nowrap" },
  cellMuted: { color: "var(--color-text-tertiary)", whiteSpace: "nowrap" },
  spacer:    { flex: 1 },
};

const s: Record<string, React.CSSProperties> = {
  shell: {
    minHeight: "100vh",
    display: "flex",
    flexDirection: "column",
    background: "var(--color-surface)",
  },
  header: {
    height: 48,
    borderBottom: "1px solid var(--color-border)",
    display: "flex",
    alignItems: "center",
    padding: "0 var(--space-4)",
    gap: "var(--space-4)",
    flexShrink: 0,
  },
  wordmark: {
    fontFamily: "var(--font-prose)",
    fontSize: 16,
    fontWeight: 700,
    color: "var(--color-amber-600)",
    letterSpacing: "-0.01em",
    flexShrink: 0,
  },
  projectTitle: {
    flex: 1,
    fontSize: 14,
    fontWeight: 500,
    color: "var(--color-text-primary)",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  headerRight: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-3)",
    flexShrink: 0,
  },
  savingDot: {
    width: 6,
    height: 6,
    borderRadius: "50%",
    background: "var(--color-amber-400, #fbbf24)",
    display: "inline-block",
  },
  aiBtn: {
    background: "none",
    border: "1px solid var(--color-amber-400, #fbbf24)",
    borderRadius: 4,
    fontSize: 12,
    color: "var(--color-amber-600)",
    padding: "2px var(--space-2)",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
    fontWeight: 600,
  },
  // ── 2026-05 redesign — mode pill + ⋯ trigger ────────────────────
  modePill: {
    display: "inline-flex", alignItems: "center", gap: 6,
    background: "var(--color-surface-raised, rgba(0,0,0,0.04))",
    border: "1px solid var(--color-border)",
    borderRadius: 999,
    padding: "3px 10px",
    fontSize: 12, fontWeight: 500,
    color: "var(--color-text-primary)",
    cursor: "pointer", fontFamily: "var(--font-ui)",
    lineHeight: 1.2,
  },
  modePillEmoji: { fontSize: 14, lineHeight: 1 },
  moreBtn: {
    background: "none",
    border: "1px solid var(--color-border)",
    borderRadius: 4,
    fontSize: 16, fontWeight: 700, lineHeight: 1,
    color: "var(--color-text-primary)",
    padding: "0 10px", height: 26,
    cursor: "pointer", fontFamily: "var(--font-ui)",
  },
  closeBtn: {
    background: "none",
    border: "1px solid var(--color-border)",
    borderRadius: 4,
    fontSize: 12,
    color: "var(--color-text-secondary)",
    padding: "2px var(--space-2)",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  body: {
    flex: 1,
    display: "flex",
    overflow: "hidden",
  },
  center: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    overflow: "auto",
  },
  right: {
    width: 0, // collapsed until MZ-04+
    flexShrink: 0,
  },
  noScene: {
    flex: 1,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  },
  noSceneText: {
    color: "var(--color-text-tertiary)",
    fontSize: 14,
    margin: 0,
    textAlign: "center",
  },
  emptyHero: {
    maxWidth: 520,
    padding: "var(--space-6)",
    color: "var(--color-text-primary)",
    fontFamily: "var(--font-ui)",
  },
  emptyHeroTitle: {
    fontSize: 16,
    fontWeight: 600,
    margin: "0 0 var(--space-3)",
  },
  emptyHeroBody: {
    fontSize: 13,
    color: "var(--color-text-secondary)",
    margin: "0 0 var(--space-3)",
  },
  emptyHeroList: {
    fontSize: 13,
    color: "var(--color-text-secondary)",
    paddingLeft: "var(--space-4)",
    margin: 0,
    lineHeight: 1.55,
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-2)",
  },
  toast: {
    position:    "fixed",
    bottom:      24,
    right:       24,
    background:  "var(--color-surface-raised)",
    border:      "1px solid var(--color-border)",
    borderRadius: 6,
    padding:     "8px 14px",
    fontSize:    13,
    color:       "var(--color-text-primary)",
    boxShadow:   "0 6px 20px rgba(0,0,0,0.2)",
    zIndex:      700,
  },
};
