/**
 * Manuscript — three-pane writing surface (F1).
 *
 *   ┌──────────┬──────────────────────────────┬─────────────────┐
 *   │ Binder   │ TipTap SceneEditor           │ Scene meta      │
 *   │ (tree)   │ (current scene's pm_doc)     │ (status, beat,  │
 *   │ 240px    │ centred 720px column         │  pov, words)    │
 *   └──────────┴──────────────────────────────┴─────────────────┘
 *
 * Owns the scene-selection lifecycle:
 *   - On mount: `ipc.nodeList()` → tree; pick first scene (or honour
 *     a caller-supplied `initialSceneId`)
 *   - On selection change: force-remount the SceneEditor by passing
 *     `selectedSceneId` as React `key`. The unmount cleanup in
 *     SceneEditor (packages/editor/src/SceneEditor.tsx:124-137) flushes
 *     any pending autosave so we can't lose the previous scene's
 *     edits when the writer clicks a new one in the binder.
 *   - On save (autosave debounced inside SceneEditor + blur):
 *     `ipc.sceneSave({ node_id, pm_doc, word_count, char_count })`.
 *     Errors surface via toast.
 *
 * Backend boundary: this route uses only existing IPC commands —
 * `nodeList`, `sceneLoad`, `sceneSave`. No new commands; no changes
 * to the locked drafter / orchestrator. The scene's `pm_doc` is the
 * same SQLite blob the AI pipeline writes during Drafting.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { SceneEditor, type Editor, type JSONContent } from "@booksforge/editor";
import type {
  NodeInfo,
  OpenProjectResult,
  SceneLoadResult,
} from "@booksforge/shared-types";
import Binder, { type BinderHandle } from "../components/Binder";
import QuickActionBar from "../components/QuickActionBar";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import { useToast } from "../components/ToastProvider";
import { useShortcut } from "../lib/keymap";

interface Props {
  project:          OpenProjectResult;
  /** Pre-selected scene (e.g. from a "jump to scene" CTA elsewhere). */
  initialSceneId?:  string | null;
  /** Called when the writer wants to go back to the stage rail. */
  onSwitchToJourney: () => void;
}

type SceneLoadState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "loaded";  doc: JSONContent | null; loadedFromBackend: SceneLoadResult | null }
  | { kind: "error";   message: string };

type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved";  at: number; words: number }
  | { kind: "error"; message: string };

export default function Manuscript({ project, initialSceneId, onSwitchToJourney }: Props) {
  const [nodes,           setNodes]           = useState<NodeInfo[]>([]);
  const [selectedSceneId, setSelectedSceneId] = useState<string | null>(initialSceneId ?? null);
  const [sceneLoad,       setSceneLoad]       = useState<SceneLoadState>({ kind: "idle" });
  const [saveState,       setSaveState]       = useState<SaveState>({ kind: "idle" });
  const [refreshKey,      setRefreshKey]      = useState(0);
  const toast = useToast();

  // Initial node-tree load + default-scene pick.
  useEffect(() => {
    let cancelled = false;
    ipc.nodeList()
      .then((list) => {
        if (cancelled) return;
        setNodes(list);
        if (selectedSceneId === null) {
          // Pick the first scene (smallest position) as the default.
          const scenes = list
            .filter((n) => n.kind === "scene")
            .sort((a, b) => a.position.localeCompare(b.position));
          const first = scenes[0];
          if (first) setSelectedSceneId(first.id);
        }
      })
      .catch((e) => {
        toast.push({
          severity: "error",
          title: "Could not load manuscript",
          body: errorMessage(e),
        });
      });
    return () => { cancelled = true; };
    // We deliberately do NOT depend on selectedSceneId here — this
    // effect is only the initial tree load. Subsequent re-fetches
    // happen via `refreshKey` so writers can refresh after editing
    // scene titles elsewhere.
  }, [refreshKey, toast, selectedSceneId]);

  // Load the scene's pm_doc whenever the selection changes.
  useEffect(() => {
    if (!selectedSceneId) {
      setSceneLoad({ kind: "idle" });
      return;
    }
    let cancelled = false;
    setSceneLoad({ kind: "loading" });
    ipc.sceneLoad(selectedSceneId)
      .then((result) => {
        if (cancelled) return;
        if (!result) {
          // Scene has never been drafted — start with an empty doc.
          setSceneLoad({ kind: "loaded", doc: null, loadedFromBackend: null });
        } else {
          setSceneLoad({
            kind:              "loaded",
            doc:               result.pm_doc as JSONContent,
            loadedFromBackend: result,
          });
        }
      })
      .catch((e) => {
        if (cancelled) return;
        const msg = errorMessage(e);
        setSceneLoad({ kind: "error", message: msg });
        toast.push({
          severity: "error",
          title: "Could not load scene",
          body: msg,
        });
      });
    return () => { cancelled = true; };
  }, [selectedSceneId, toast]);

  // Persist a scene save. Called by SceneEditor's debounced autosave
  // and blur handler. We capture `selectedSceneId` in a ref because
  // by the time autosave fires the writer might have switched scenes
  // (though we force-remount on switch, this is a defensive guard).
  const selectedIdRef = useRef(selectedSceneId);
  selectedIdRef.current = selectedSceneId;

  const handleSave = useCallback(async (doc: JSONContent, words: number, chars: number) => {
    const id = selectedIdRef.current;
    if (!id) return;
    setSaveState({ kind: "saving" });
    try {
      await ipc.sceneSave({
        node_id:     id,
        pm_doc:      doc,
        word_count:  words,
        char_count:  chars,
      });
      setSaveState({ kind: "saved", at: Date.now(), words });
      // Update the local node's word_count so the binder's running
      // total stays accurate without a full refetch.
      setNodes((prev) => prev.map((n) => (
        n.id === id ? { ...n, word_count: words } : n
      )));
    } catch (e) {
      const msg = errorMessage(e);
      setSaveState({ kind: "error", message: msg });
      toast.push({
        severity: "error",
        title: "Save failed",
        body: msg,
      });
    }
  }, [toast]);

  const selectedScene: NodeInfo | undefined = useMemo(
    () => nodes.find((n) => n.id === selectedSceneId),
    [nodes, selectedSceneId],
  );

  // F8 — Inline rename handler. Backend uses `nodeUpdate`; we update
  // local state optimistically so the binder reflects the new title
  // even before the IPC settles. Errors surface via toast.
  const handleRename = useCallback(async (id: string, newTitle: string) => {
    setNodes((prev) => prev.map((n) => (
      n.id === id ? { ...n, title: newTitle } : n
    )));
    try {
      await ipc.nodeUpdate({
        id,
        title:        newTitle,
        position:     null,
        status:       null,
        pov:          null,
        beat:         null,
        target_words: null,
      });
      toast.push({ severity: "success", body: `Renamed to "${newTitle}".` });
    } catch (e) {
      // Roll back local state — refetch from source of truth.
      try {
        const fresh = await ipc.nodeList();
        setNodes(fresh);
      } catch { /* ignore — keep the optimistic value */ }
      toast.push({
        severity: "error",
        title: "Rename failed",
        body: errorMessage(e),
      });
    }
  }, [toast]);

  // F8 — Imperative handle for the binder so the focus shortcut
  // (mod+1) can ask it to focus the first row.
  const binderRef = useRef<BinderHandle>(null);
  useShortcut("binder.focus", () => binderRef.current?.focusFirstRow());

  // Cmd+K quick-action bar state. Captures the editor's current
  // selection (or surrounding paragraph) at open time so the bar's
  // scope doesn't shift while the writer reviews the suggestion.
  const editorRef = useRef<Editor | null>(null);
  const [quickAction, setQuickAction] = useState<null | { scope: string }>(null);
  useShortcut("agents.quick-action", () => {
    if (!editorRef.current) return;
    const ed = editorRef.current;
    const { from, to, $from } = ed.state.selection;
    let scope: string;
    if (from !== to) {
      scope = ed.state.doc.textBetween(from, to, "\n");
    } else {
      // No selection — take the surrounding paragraph so the writer
      // doesn't have to highlight before invoking the bar.
      const start = $from.before($from.depth);
      const end   = $from.after($from.depth);
      scope = ed.state.doc.textBetween(start, end, "\n").trim();
    }
    setQuickAction({ scope });
  });

  return (
    <div style={s.shell}>
      <Binder
        ref={binderRef}
        nodes={nodes}
        selectedSceneId={selectedSceneId}
        onSelectScene={setSelectedSceneId}
        onRenameNode={handleRename}
      />

      <main style={s.editorPane}>
        <header style={s.editorHeader}>
          <button
            style={s.journeyBtn}
            onClick={onSwitchToJourney}
            title="Back to the stage rail"
          >
            ← Journey
          </button>
          <span style={s.editorTitle}>
            {selectedScene
              ? selectedScene.title || "Untitled scene"
              : "No scene selected"}
          </span>
          <span style={s.saveIndicator}>{renderSaveStatus(saveState)}</span>
        </header>

        <div style={s.editorBody}>
          {sceneLoad.kind === "idle" && (
            <p style={s.muted}>Select a scene from the binder to start writing.</p>
          )}
          {sceneLoad.kind === "loading" && (
            <p style={s.muted}>Loading scene…</p>
          )}
          {sceneLoad.kind === "error" && (
            <p style={s.error}>Could not load scene: {sceneLoad.message}</p>
          )}
          {sceneLoad.kind === "loaded" && selectedSceneId && (
            <SceneEditor
              // Force remount on scene change. Triggers the SceneEditor
              // cleanup hook which flushes pending autosave — so the
              // previous scene's edits land before we wipe state.
              key={selectedSceneId}
              initialDoc={sceneLoad.doc}
              onSave={handleSave}
              onEditorReady={(ed) => { editorRef.current = ed; }}
            />
          )}
        </div>
      </main>

      <SceneMetaPane
        scene={selectedScene}
        project={project}
        onRefreshTree={() => setRefreshKey((k) => k + 1)}
      />

      {quickAction && (
        <QuickActionBar
          sceneNodeId={selectedSceneId}
          initialScope={quickAction.scope}
          editor={editorRef.current}
          onClose={() => setQuickAction(null)}
        />
      )}
    </div>
  );
}

function renderSaveStatus(state: SaveState): string {
  switch (state.kind) {
    case "idle":   return "";
    case "saving": return "Saving…";
    case "saved":  return `Saved · ${state.words.toLocaleString()} words`;
    case "error":  return `Save failed`;
  }
}

/**
 * Right-pane scene metadata. Read-only in F1 — edit-in-place lives
 * in a follow-up PR (writer needs node_update with title/status/pov/
 * beat/target_words; the IPC already supports it, just not surfaced).
 */
function SceneMetaPane({
  scene, project, onRefreshTree,
}: {
  scene:           NodeInfo | undefined;
  project:         OpenProjectResult;
  onRefreshTree:   () => void;
}) {
  return (
    <aside style={s.metaPane}>
      <h3 style={s.metaHeading}>Scene</h3>
      {scene ? (
        <dl style={s.metaList}>
          <MetaRow label="Title"  value={scene.title || "Untitled"} />
          <MetaRow label="Status" value={scene.status} />
          <MetaRow label="Words"  value={scene.word_count.toLocaleString()} />
          {scene.pov          && <MetaRow label="POV"    value={scene.pov} />}
          {scene.beat         && <MetaRow label="Beat"   value={scene.beat} />}
          {scene.target_words && <MetaRow label="Target" value={scene.target_words.toLocaleString()} />}
          <MetaRow label="Updated" value={formatUpdated(scene.updated_at)} />
        </dl>
      ) : (
        <p style={s.muted}>No scene selected.</p>
      )}
      <h3 style={{ ...s.metaHeading, marginTop: 16 }}>Project</h3>
      <dl style={s.metaList}>
        <MetaRow label="Title"  value={project.title} />
        <MetaRow label="Author" value={project.author} />
      </dl>
      <button
        style={s.refreshBtn}
        onClick={onRefreshTree}
        title="Re-fetch the node tree (e.g. after an AI run added scenes)"
      >
        ↻ Refresh binder
      </button>
    </aside>
  );
}

function MetaRow({ label, value }: { label: string; value: string }) {
  return (
    <>
      <dt style={s.metaDt}>{label}</dt>
      <dd style={s.metaDd}>{value}</dd>
    </>
  );
}

function formatUpdated(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  const elapsed = Date.now() - t;
  const min = Math.floor(elapsed / 60000);
  if (min < 1)   return "just now";
  if (min < 60)  return `${min} min ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24)   return `${hr} h ago`;
  const day = Math.floor(hr / 24);
  if (day < 30)  return `${day} d ago`;
  return new Date(t).toLocaleDateString();
}

const s: Record<string, React.CSSProperties> = {
  shell: {
    flex: 1,
    display: "flex",
    minHeight: 0,
    overflow: "hidden",
    fontFamily: "var(--font-ui)",
  },
  editorPane: {
    flex: 1,
    display: "flex", flexDirection: "column",
    background: "#fff",
    minWidth: 0,
  },
  editorHeader: {
    display: "flex", alignItems: "center", gap: 12,
    padding: "8px 16px",
    borderBottom: "1px solid var(--color-neutral-200)",
    background: "var(--color-neutral-50)",
    minHeight: 40,
  },
  journeyBtn: {
    background: "transparent",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    padding: "4px 10px",
    fontSize: 12,
    fontFamily: "var(--font-ui)",
    color: "var(--color-neutral-700)",
    cursor: "pointer",
  },
  editorTitle: {
    flex: 1,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 15,
    fontWeight: 600,
    color: "var(--color-neutral-900)",
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
  saveIndicator: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-neutral-500)",
    fontVariantNumeric: "tabular-nums",
    minWidth: 120,
    textAlign: "right",
  },
  editorBody: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    overflow: "auto",
  },
  metaPane: {
    width: 220,
    flexShrink: 0,
    padding: "16px",
    borderLeft: "1px solid var(--color-neutral-200)",
    background: "var(--color-neutral-50)",
    overflowY: "auto",
    display: "flex", flexDirection: "column", gap: 8,
  },
  metaHeading: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    letterSpacing: "0.08em", textTransform: "uppercase",
    color: "var(--color-neutral-500)",
  },
  metaList: {
    display: "grid",
    gridTemplateColumns: "max-content 1fr",
    columnGap: 12, rowGap: 6,
    margin: 0, padding: 0,
  },
  metaDt: {
    fontSize: 10,
    fontWeight: 600,
    letterSpacing: "0.06em",
    textTransform: "uppercase",
    color: "var(--color-neutral-500)",
  },
  metaDd: {
    margin: 0,
    fontSize: 12,
    color: "var(--color-neutral-900)",
    overflow: "hidden",
    wordBreak: "break-word",
  },
  refreshBtn: {
    marginTop: 8,
    background: "transparent",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    padding: "6px 10px",
    fontSize: 11,
    fontFamily: "var(--font-ui)",
    color: "var(--color-neutral-700)",
    cursor: "pointer",
    textAlign: "left",
  },
  muted: {
    margin: "32px",
    fontSize: 13,
    color: "var(--color-neutral-500)",
  },
  error: {
    margin: "32px",
    fontSize: 12,
    fontFamily: "var(--font-mono)",
    color: "var(--color-red-700, #b91c1c)",
  },
};
