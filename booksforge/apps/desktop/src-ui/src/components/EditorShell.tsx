/**
 * Three-pane editor shell: Binder (left) | Editor (center) | [right panel stub]
 *
 * Manages:
 * - Node list refresh
 * - Scene selection and content loading
 * - Crash recovery check on mount
 */
import React, { useCallback, useEffect, useState } from "react";
import type { JSONContent } from "@tiptap/core";
import type { NodeInfo, OpenProjectResult, RecoveryStatus, SceneLoadResult } from "@booksforge/shared-types";
import { SceneEditor } from "@booksforge/editor";
import Binder from "./Binder";
import RecoveryDialog from "./RecoveryDialog";
import { ipc } from "../lib/ipc";

interface Props {
  project: OpenProjectResult;
  onClose: () => void;
}

export default function EditorShell({ project, onClose }: Props) {
  const [nodes, setNodes] = useState<NodeInfo[]>([]);
  const [selectedNode, setSelectedNode] = useState<NodeInfo | null>(null);
  const [sceneContent, setSceneContent] = useState<SceneLoadResult | null>(null);
  const [recovery, setRecovery] = useState<RecoveryStatus | null>(null);
  const [saving, setSaving] = useState(false);

  // Load node list.
  const refreshNodes = useCallback(async () => {
    const list = await ipc.nodeList().catch(() => [] as NodeInfo[]);
    setNodes(list);
  }, []);

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
      setSaving(true);
      try {
        await ipc.sceneSave({
          node_id: selectedNode.id,
          pm_doc: doc,
          word_count: wordCount,
          char_count: charCount,
        });
      } catch {
        // Best-effort — don't crash the UI on autosave failure.
      } finally {
        setSaving(false);
      }
    },
    [selectedNode]
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
        <Binder
          nodes={nodes}
          selectedId={selectedNode?.id ?? null}
          onSelect={setSelectedNode}
          onNodesChanged={refreshNodes}
        />

        <main style={s.center}>
          {selectedNode?.kind === "scene" ? (
            <SceneEditor
              key={selectedNode.id}
              initialDoc={
                sceneContent ? (sceneContent.pm_doc as JSONContent) : null
              }
              onSave={handleSave}
              saveDelay={5000}
            />
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

        {/* Right panel stub — will host inspector / entity panel in MZ-04+ */}
        <aside style={s.right} />
      </div>

      {/* ── Crash recovery dialog ── */}
      {recovery && (
        <RecoveryDialog
          status={recovery}
          onRestore={handleRecoveryRestore}
          onDiscard={handleRecoveryDiscard}
        />
      )}
    </div>
  );
}

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
};
