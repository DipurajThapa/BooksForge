import React, { useState } from "react";
import type { NodeInfo } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";

interface Props {
  nodes: NodeInfo[];
  selectedId: string | null;
  onSelect: (node: NodeInfo) => void;
  onNodesChanged: () => void;
}

const KIND_LABEL: Record<string, string> = {
  project:      "📚",
  part:         "📂",
  chapter:      "📄",
  scene:        "📝",
  front_matter: "🔖",
  back_matter:  "🔖",
};

const STATUS_COLOR: Record<string, string> = {
  planned:  "var(--color-neutral-400)",
  drafting: "var(--color-amber-500)",
  revised:  "var(--color-blue-500, #3b82f6)",
  final:    "var(--color-success)",
};

export default function Binder({ nodes, selectedId, onSelect, onNodesChanged }: Props) {
  const [creating, setCreating] = useState<string | null>(null); // parent_id being created under

  // Build a flat tree ordered by position, parent_id = null first.
  const roots = nodes.filter((n) => !n.parent_id && n.kind !== "project");
  const scenes = nodes.filter((n) => n.kind === "scene");

  async function handleAddScene(parentId: string | null) {
    setCreating(parentId);
    try {
      await ipc.nodeCreate({
        parent_id: parentId,
        kind: "scene",
        title: "Untitled Scene",
        position: `0|${Date.now().toString(36)}:`,
        status: "planned",
        target_words: null,
      });
      onNodesChanged();
    } catch {
      // silently ignore for now
    } finally {
      setCreating(null);
    }
  }

  async function handleDelete(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    await ipc.nodeDelete(id).catch(() => null);
    onNodesChanged();
  }

  if (nodes.length === 0) {
    return (
      <div style={s.root}>
        <div style={s.header}>
          <span style={s.headerLabel}>Binder</span>
        </div>
        <div style={s.empty}>
          <p style={s.emptyText}>No scenes yet.</p>
          <button style={s.addBtn} onClick={() => handleAddScene(null)}>
            + New Scene
          </button>
        </div>
      </div>
    );
  }

  // Group: show chapters containing scenes, and orphan scenes.
  const chapters = nodes.filter((n) => n.kind === "chapter");
  const orphanScenes = scenes.filter((s) => !s.parent_id || !chapters.find((c) => c.id === s.parent_id));

  function renderNode(node: NodeInfo, depth = 0) {
    const children = nodes.filter(
      (n) => n.parent_id === node.id && n.kind === "scene"
    );
    const isSelected = node.id === selectedId;
    const isClickable = node.kind === "scene";

    return (
      <React.Fragment key={node.id}>
        <div
          style={{
            ...s.row,
            paddingLeft: 12 + depth * 16,
            background: isSelected ? "var(--color-amber-50, #fffbeb)" : undefined,
            cursor: isClickable ? "pointer" : "default",
          }}
          onClick={() => isClickable && onSelect(node)}
        >
          <span style={s.rowIcon}>{KIND_LABEL[node.kind] ?? "📄"}</span>
          <span
            style={{
              ...s.rowTitle,
              fontWeight: node.kind === "chapter" ? 600 : 400,
            }}
          >
            {node.title || "Untitled"}
          </span>
          <span
            style={{
              ...s.statusDot,
              background: STATUS_COLOR[node.status] ?? "var(--color-neutral-400)",
            }}
          />
          {node.kind === "scene" && (
            <button
              style={s.deleteBtn}
              onClick={(e) => handleDelete(node.id, e)}
              title="Delete scene"
            >
              ×
            </button>
          )}
        </div>
        {node.kind === "chapter" &&
          children.map((child) => renderNode(child, depth + 1))}
        {node.kind === "chapter" && (
          <button
            style={{ ...s.addSceneInline, paddingLeft: 12 + (depth + 1) * 16 }}
            onClick={() => handleAddScene(node.id)}
            disabled={creating === node.id}
          >
            + Scene
          </button>
        )}
      </React.Fragment>
    );
  }

  return (
    <div style={s.root}>
      <div style={s.header}>
        <span style={s.headerLabel}>Binder</span>
        <button style={s.addBtn} onClick={() => handleAddScene(null)} title="Add scene">
          +
        </button>
      </div>
      <div style={s.tree}>
        {chapters.length > 0
          ? chapters.map((c) => renderNode(c))
          : orphanScenes.map((sc) => renderNode(sc))}
        {orphanScenes.length > 0 && chapters.length > 0 &&
          orphanScenes.map((sc) => renderNode(sc))}
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  root: {
    width: 220,
    minWidth: 220,
    borderRight: "1px solid var(--color-border)",
    display: "flex",
    flexDirection: "column",
    background: "var(--color-neutral-50, #fafafa)",
    userSelect: "none",
  },
  header: {
    height: 40,
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "0 var(--space-3)",
    borderBottom: "1px solid var(--color-border)",
    flexShrink: 0,
  },
  headerLabel: {
    fontSize: 11,
    fontWeight: 600,
    letterSpacing: "0.08em",
    textTransform: "uppercase",
    color: "var(--color-text-tertiary)",
  },
  addBtn: {
    background: "none",
    border: "none",
    color: "var(--color-text-secondary)",
    fontSize: 18,
    cursor: "pointer",
    lineHeight: 1,
    padding: "2px 4px",
  },
  tree: {
    flex: 1,
    overflow: "auto",
    padding: "var(--space-2) 0",
  },
  empty: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: "var(--space-3)",
    padding: "var(--space-4)",
  },
  emptyText: {
    color: "var(--color-text-tertiary)",
    fontSize: 13,
    margin: 0,
    textAlign: "center",
  },
  row: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-1)",
    padding: "5px 8px",
    fontSize: 13,
    color: "var(--color-text-primary)",
    position: "relative",
    minHeight: 30,
  },
  rowIcon: { fontSize: 14, flexShrink: 0 },
  rowTitle: {
    flex: 1,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    fontSize: 13,
  },
  statusDot: {
    width: 6,
    height: 6,
    borderRadius: "50%",
    flexShrink: 0,
  },
  deleteBtn: {
    background: "none",
    border: "none",
    color: "var(--color-text-tertiary)",
    fontSize: 16,
    cursor: "pointer",
    lineHeight: 1,
    padding: "0 2px",
    opacity: 0,
  },
  addSceneInline: {
    display: "block",
    background: "none",
    border: "none",
    color: "var(--color-text-tertiary)",
    fontSize: 12,
    cursor: "pointer",
    padding: "2px 8px",
    textAlign: "left",
    width: "100%",
  },
};
