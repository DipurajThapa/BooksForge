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

function formatWordCount(n: number): string {
  if (n >= 10_000) return `${(n / 1_000).toFixed(0)}k`;
  if (n >= 1_000)  return `${(n / 1_000).toFixed(1)}k`;
  return `${n}`;
}

const STATUS_COLOR: Record<string, string> = {
  planned:  "var(--color-neutral-400)",
  drafting: "var(--color-amber-500)",
  revised:  "var(--color-blue-500, #3b82f6)",
  final:    "var(--color-success)",
};

// LexoRank helpers: positions are `<bucket>|<rank>:` (e.g. `0|i00000:`).
// Compute a fresh rank string strictly between `prev` and `next` using
// base-36 arithmetic on the rank portion.  Falls back to bucket midpoint
// if either end is omitted.
const LEXORANK_LO_INT  = parseInt("100000", 36); // 60466176
const LEXORANK_HI_INT  = parseInt("yzzzzz", 36); // 2176782335
const LEXORANK_PREFIX  = "0|";
function rankBetween(prev: string | null, next: string | null): string {
  const lo = prev ? extractRankInt(prev) : LEXORANK_LO_INT;
  const hi = next ? extractRankInt(next) : LEXORANK_HI_INT;
  if (hi <= lo + 1) {
    // Tight gap — fall back to a fresh slot just below `hi`.
    return formatRank(Math.max(lo, hi - 1));
  }
  return formatRank(Math.floor((lo + hi) / 2));
}
function extractRankInt(p: string): number {
  // `0|<rank>:` → rank as base-36 integer; if it's not the canonical
  // shape, fall back to a midpoint so we never throw.
  const m = /^0\|([0-9a-z]+):/.exec(p);
  if (!m) return Math.floor((LEXORANK_LO_INT + LEXORANK_HI_INT) / 2);
  return parseInt(m[1], 36);
}
function formatRank(value: number): string {
  let v = Math.max(0, Math.min(value, LEXORANK_HI_INT));
  let rank = v.toString(36);
  // Pad to 6 chars so lexicographic compare matches numeric.
  while (rank.length < 6) rank = "0" + rank;
  return `${LEXORANK_PREFIX}${rank}:`;
}

export default function Binder({ nodes, selectedId, onSelect, onNodesChanged }: Props) {
  const [creating, setCreating] = useState<string | null>(null); // parent_id being created under
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

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

  /**
   * Handle a drop: compute a new LexoRank position for the dragged
   * scene that places it directly above `target`, and persist via
   * `nodeUpdate(position)`.  Reordering only works among scene siblings
   * (same chapter parent) for MVP — cross-parent moves are deferred.
   */
  async function handleDrop(target: NodeInfo) {
    if (!draggingId) return;
    const dragged = nodes.find((n) => n.id === draggingId);
    setDraggingId(null);
    setDropTargetId(null);
    if (!dragged || dragged.id === target.id) return;
    if (dragged.kind !== "scene" || target.kind !== "scene") return;
    if (dragged.parent_id !== target.parent_id) return;

    // Find the previous sibling (the one currently above `target`).
    const siblings = nodes
      .filter((n) => n.parent_id === target.parent_id && n.kind === "scene")
      .sort((a, b) => a.position.localeCompare(b.position));
    const targetIdx = siblings.findIndex((n) => n.id === target.id);
    if (targetIdx < 0) return;
    const prev = targetIdx > 0 ? siblings[targetIdx - 1] : null;
    // If user dragged the row immediately below the previous neighbour,
    // there's nothing to do.
    if (prev?.id === dragged.id) return;

    const newPosition = rankBetween(prev?.position ?? null, target.position);
    try {
      await ipc.nodeUpdate({
        id:           dragged.id,
        title:        null,
        position:     newPosition,
        status:       null,
        pov:          null,
        beat:         null,
        target_words: null,
      });
      onNodesChanged();
    } catch {
      // silently ignore for now — rebalancing failure shouldn't crash UI
    }
  }

  if (nodes.length === 0) {
    return (
      <nav style={s.root} aria-label="Manuscript binder">
        <div style={s.header}>
          <span style={s.headerLabel}>Binder</span>
        </div>
        <div style={s.empty}>
          <p style={s.emptyText}>No scenes yet.</p>
          <button style={s.addBtn} onClick={() => handleAddScene(null)} aria-label="Create first scene">
            + New Scene
          </button>
        </div>
      </nav>
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

    const isDraggable = node.kind === "scene";
    const isDropTarget = dropTargetId === node.id && draggingId !== node.id;
    return (
      <React.Fragment key={node.id}>
        <div
          // ARIA tree-widget roles (WAI-ARIA Authoring Practices §3.27).
          // Chapters are non-selectable group nodes that always present
          // their scenes expanded; scenes are the selectable leaves.
          role="treeitem"
          aria-level={depth + 1}
          aria-expanded={node.kind === "chapter" ? true : undefined}
          aria-selected={isClickable ? isSelected : undefined}
          // Roving-tabindex pattern: only the selected scene is in the
          // tab order; everything else gets focus via arrow-key
          // navigation (TODO — minimal keyboard nav).
          tabIndex={isClickable && isSelected ? 0 : -1}
          onKeyDown={(e) => {
            if (!isClickable) return;
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              onSelect(node);
            }
          }}
          draggable={isDraggable}
          onDragStart={(e) => {
            if (!isDraggable) return;
            setDraggingId(node.id);
            e.dataTransfer.effectAllowed = "move";
          }}
          onDragOver={(e) => {
            if (draggingId && node.kind === "scene") {
              e.preventDefault();
              setDropTargetId(node.id);
            }
          }}
          onDragLeave={() => {
            if (dropTargetId === node.id) setDropTargetId(null);
          }}
          onDrop={(e) => {
            e.preventDefault();
            void handleDrop(node);
          }}
          onDragEnd={() => {
            setDraggingId(null);
            setDropTargetId(null);
          }}
          style={{
            ...s.row,
            paddingLeft: 12 + depth * 16,
            background: isSelected ? "var(--color-amber-50, #fffbeb)" : undefined,
            cursor: isClickable ? "pointer" : "default",
            borderTop: isDropTarget ? "2px solid var(--color-amber-600)" : "2px solid transparent",
            opacity: draggingId === node.id ? 0.45 : 1,
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
          {node.word_count > 0 && (
            <span style={s.wordCount} title={`${node.word_count.toLocaleString()} words`}>
              {formatWordCount(node.word_count)}
            </span>
          )}
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
              aria-label={`Delete scene: ${node.title || "Untitled"}`}
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
    <nav style={s.root} aria-label="Manuscript binder">
      <div style={s.header}>
        <span style={s.headerLabel}>Binder</span>
        <button
          style={s.addBtn}
          onClick={() => handleAddScene(null)}
          title="Add scene"
          aria-label="Add scene"
        >
          +
        </button>
      </div>
      <div style={s.tree} role="tree" aria-label="Manuscript tree">
        {chapters.length > 0
          ? chapters.map((c) => renderNode(c))
          : orphanScenes.map((sc) => renderNode(sc))}
        {orphanScenes.length > 0 && chapters.length > 0 &&
          orphanScenes.map((sc) => renderNode(sc))}
      </div>
    </nav>
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
  wordCount: {
    fontSize: 10,
    color: "var(--color-text-tertiary)",
    fontFamily: "var(--font-mono)",
    fontVariantNumeric: "tabular-nums",
    flexShrink: 0,
    marginRight: 4,
  },
};
