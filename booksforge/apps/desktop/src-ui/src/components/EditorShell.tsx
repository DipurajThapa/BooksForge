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
import type { JSONContent } from "@tiptap/core";
import type { NodeInfo, OpenProjectResult, RecoveryStatus, SceneLoadResult } from "@booksforge/shared-types";
import { EditorToolbar, SceneEditor, type SceneEditorHandle } from "@booksforge/editor";
import AgentDebugForm from "./AgentDebugForm";
import AgentsPanel from "./agents/AgentsPanel";
import ExportPanel from "./ExportPanel";
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
  const [showExport,       setShowExport]       = useState(false);
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
  const editorHandleRef = useRef<SceneEditorHandle>(null);
  // Held in state (not just a ref) so the toolbar re-renders when the
  // editor is mounted/unmounted as the user switches scenes.
  const [editorInstance, setEditorInstance] = useState<import("@tiptap/react").Editor | null>(null);
  // Live word count of the active scene; updated on every keystroke via
  // the SceneEditor `onSave` callback.
  const [liveSceneWords, setLiveSceneWords] = useState<number>(0);
  // Project total at session start, used to compute "today" delta.
  const sessionBaselineRef = useRef<number | null>(null);

  // Export the manuscript to Markdown via the OS save-file dialog.
  // Gates on the validator: errors block, warnings prompt, info silent.
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
      setExportToast(`Pre-export check failed: ${String(e)}`);
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
      setExportToast(`Export failed: ${String(e)}`);
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
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selectedNode]);

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
      {/* ── Header ── */}
      <header style={s.header}>
        <span style={s.wordmark}>BooksForge</span>
        <span style={s.projectTitle}>{project.title}</span>
        <div style={s.headerRight}>
          {saving && <span style={s.savingDot} title="Saving…" />}
          <button
            style={s.aiBtn}
            onClick={() => setShowOllamaWizard(true)}
            title="AI Setup"
          >
            AI Setup
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowAgentDebug(true)}
            title="Debug: run outline agent"
          >
            Debug AI
          </button>
          <button
            style={{
              ...s.aiBtn,
              borderColor: focusMode ? "var(--color-amber-600)" : "var(--color-border)",
              background:  focusMode ? "var(--color-amber-600)" : "none",
              color:       focusMode ? "#fff" : "var(--color-amber-600)",
            }}
            onClick={() => setFocusMode((on) => !on)}
            title="Focus mode — hides binder and inspector (⌘. / Ctrl+.)"
          >
            {focusMode ? "Exit Focus" : "Focus"}
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowSnapshots(true)}
            title="Snapshots — manual save points and restore"
          >
            Snapshots
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowValidators(true)}
            title="Run all 16 manuscript validators"
          >
            Check
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowKnowledge(true)}
            title="Project memory + vocabulary"
          >
            Knowledge
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowAgents(true)}
            title="Run any of the 11 agents (copyedit, humanize, continuity, draft, …)"
          >
            Agents
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowExport(true)}
            title="Export manuscript — Markdown / EPUB / DOCX / PDF"
          >
            Export
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowHelp(true)}
            title="In-app help — keyboard shortcuts, agents, exports"
          >
            Help
          </button>
          <button
            style={{ ...s.aiBtn, borderColor: "var(--color-border)" }}
            onClick={() => setShowSettings(true)}
            title="Settings — telemetry, diagnostics, originality, dependencies"
          >
            Settings
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
              <p style={s.noSceneText}>
                {nodes.length === 0
                  ? "Create your first scene in the binder."
                  : "Select a scene to start writing."}
              </p>
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
        />
      )}

      {/* ── Phase 6 Export panel ── */}
      {showExport && (
        <ExportPanel onClose={() => setShowExport(false)} />
      )}

      {/* ── §B4 Settings panel ── */}
      {showSettings && (
        <SettingsPanel onClose={() => setShowSettings(false)} />
      )}

      {/* ── §I4 Help drawer ── */}
      {showHelp && (
        <HelpDrawer onClose={() => setShowHelp(false)} />
      )}

      {/* ── §I5 Onboarding tour ── */}
      {showOnboarding && (
        <OnboardingTour onClose={() => setShowOnboarding(false)} />
      )}

      {/* ── §E4 Live agent-run overlay ── */}
      <LiveRunOverlay />

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
