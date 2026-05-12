/**
 * Binder — left-pane node tree for the Manuscript view (F1).
 *
 * Per `outputs/UI_UX_SPEC.md §5.1`, the binder is a collapsible tree
 * of the project's nodes: Project → Parts → Chapters → Scenes. The
 * writer clicks a scene to load it into the centre-pane TipTap
 * editor.
 *
 * MVP scope (what this file does):
 *   - Render the tree from a flat `NodeInfo[]` list (`ipc.nodeList()`)
 *   - Click-to-select a scene; selected scene is visually highlighted
 *   - Status dot per scene (planned / drafting / revised / final)
 *   - Word-count rollup per chapter (sum of child scenes)
 *   - Expand / collapse parts and chapters (local state — not persisted)
 *
 * Deferred (later F-fix PRs):
 *   - Drag-reorder (spec §5.1)
 *   - Right-click context menu (New scene, Rename, Duplicate, Delete)
 *   - Outline-view tab (flat scene list with synopsis editing)
 *   - Soft-delete with undo
 *
 * Mutation policy: this component is **read-only on the tree**. All
 * mutations (rename, reorder, create, delete) go through the parent
 * Manuscript route via callbacks — Binder never invokes IPC itself.
 * That keeps the "Orchestrator is the only mutator" rule alive on
 * the UI side and lets a single owner refresh the tree after IPC.
 */
import { useMemo, useState } from "react";
import type { NodeInfo } from "@booksforge/shared-types";

interface Props {
  nodes:            NodeInfo[];
  selectedSceneId:  string | null;
  onSelectScene:    (id: string) => void;
}

interface TreeNode {
  info:     NodeInfo;
  children: TreeNode[];
}

/**
 * Build a parent → children tree from the flat NodeInfo list, sorted
 * by the `position` field (an LSEQ-style string the backend uses so
 * scene order survives drag-and-drop without rewriting every row).
 */
function buildTree(nodes: NodeInfo[]): TreeNode[] {
  const byParent = new Map<string | null, NodeInfo[]>();
  for (const n of nodes) {
    const key = n.parent_id;
    if (!byParent.has(key)) byParent.set(key, []);
    byParent.get(key)!.push(n);
  }
  for (const list of byParent.values()) {
    list.sort((a, b) => a.position.localeCompare(b.position));
  }
  function attach(parentId: string | null): TreeNode[] {
    return (byParent.get(parentId) ?? []).map((info) => ({
      info,
      children: attach(info.id),
    }));
  }
  return attach(null);
}

/** Sum word counts of every scene under this subtree. */
function sumWords(node: TreeNode): number {
  if (node.info.kind === "scene") return node.info.word_count;
  return node.children.reduce((acc, c) => acc + sumWords(c), 0);
}

export default function Binder({ nodes, selectedSceneId, onSelectScene }: Props) {
  const tree = useMemo(() => buildTree(nodes), [nodes]);

  // Local expand state per node id. Default: parts open, chapters open
  // for the chapter containing the selected scene, others closed. Once
  // the writer collapses, we respect their choice.
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  function toggle(id: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }

  // Find the project root (kind === "project") to anchor the heading.
  const projectRoot = tree.find((t) => t.info.kind === "project");
  const topLevel = projectRoot ? projectRoot.children : tree;

  return (
    <nav style={s.binder} aria-label="Manuscript binder">
      <h2 style={s.heading}>Binder</h2>
      <ol style={s.list}>
        {topLevel.length === 0 ? (
          <li style={s.empty}>
            No chapters or scenes yet. Generate an outline in Stage 4
            and accept it — the structure shows up here.
          </li>
        ) : (
          topLevel.map((n) => (
            <BinderRow
              key={n.info.id}
              node={n}
              depth={0}
              selectedSceneId={selectedSceneId}
              collapsed={collapsed}
              onToggle={toggle}
              onSelect={onSelectScene}
            />
          ))
        )}
      </ol>
    </nav>
  );
}

function BinderRow({
  node, depth, selectedSceneId, collapsed, onToggle, onSelect,
}: {
  node:             TreeNode;
  depth:            number;
  selectedSceneId:  string | null;
  collapsed:        Set<string>;
  onToggle:         (id: string) => void;
  onSelect:         (id: string) => void;
}) {
  const { info, children } = node;
  const isScene    = info.kind === "scene";
  const isExpanded = !collapsed.has(info.id);
  const isSelected = isScene && info.id === selectedSceneId;
  const hasChildren = children.length > 0;
  const wordRollup = isScene ? info.word_count : sumWords(node);

  function handleClick() {
    if (isScene) {
      onSelect(info.id);
    } else if (hasChildren) {
      onToggle(info.id);
    }
  }

  return (
    <li>
      <button
        style={{
          ...s.row,
          ...(isSelected ? s.rowSelected : {}),
          paddingLeft: 8 + depth * 14,
        }}
        onClick={handleClick}
        aria-expanded={hasChildren ? isExpanded : undefined}
        aria-current={isSelected ? "page" : undefined}
        title={isScene ? info.title : `${info.title} — ${wordRollup.toLocaleString()} words`}
      >
        <span style={s.disclosure} aria-hidden="true">
          {hasChildren ? (isExpanded ? "▾" : "▸") : ""}
        </span>
        {isScene && (
          <span style={statusDotStyle(info.status)} aria-hidden="true" />
        )}
        <span style={s.label}>{info.title || untitledLabel(info.kind)}</span>
        {wordRollup > 0 && (
          <span style={s.wordCount}>{wordRollup.toLocaleString()}</span>
        )}
      </button>
      {hasChildren && isExpanded && (
        <ol style={s.list}>
          {children.map((c) => (
            <BinderRow
              key={c.info.id}
              node={c}
              depth={depth + 1}
              selectedSceneId={selectedSceneId}
              collapsed={collapsed}
              onToggle={onToggle}
              onSelect={onSelect}
            />
          ))}
        </ol>
      )}
    </li>
  );
}

function untitledLabel(kind: string): string {
  switch (kind) {
    case "part":    return "Untitled part";
    case "chapter": return "Untitled chapter";
    case "scene":   return "Untitled scene";
    default:        return "Untitled";
  }
}

function statusDotStyle(status: string): React.CSSProperties {
  // Status colours mirror StageRail's traffic-light convention so
  // the writer's mental model stays consistent.
  const colour =
    status === "final"    ? "var(--color-green-500, #22c55e)" :
    status === "revised"  ? "var(--color-amber-500, #f59e0b)" :
    status === "drafting" ? "var(--color-amber-400, #fbbf24)" :
    /* planned */           "var(--color-neutral-300)";
  return {
    width: 6, height: 6, borderRadius: "50%",
    background: colour, flexShrink: 0,
  };
}

const s: Record<string, React.CSSProperties> = {
  binder: {
    width: 240,
    flexShrink: 0,
    padding: "16px 0",
    borderRight: "1px solid var(--color-neutral-200)",
    background: "var(--color-neutral-50)",
    display: "flex", flexDirection: "column", gap: 8,
    fontFamily: "var(--font-ui)",
    overflowY: "auto",
  },
  heading: {
    fontSize: 11, fontWeight: 600,
    letterSpacing: "0.08em", textTransform: "uppercase",
    color: "var(--color-neutral-500)",
    margin: "0 0 4px 16px",
  },
  list: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column",
  },
  empty: {
    margin: "4px 16px 0",
    fontSize: 12, color: "var(--color-neutral-500)",
    lineHeight: 1.55,
  },
  row: {
    display: "flex", alignItems: "center", gap: 8,
    width: "100%",
    padding: "6px 12px 6px 8px",
    background: "transparent",
    border: "none",
    cursor: "pointer",
    textAlign: "left",
    fontFamily: "inherit",
    fontSize: 13,
    color: "var(--color-neutral-900)",
    minHeight: 28,
  },
  rowSelected: {
    background: "var(--color-amber-50, #fffbeb)",
    color: "var(--color-amber-700, #b45309)",
    fontWeight: 600,
  },
  disclosure: {
    width: 12,
    color: "var(--color-neutral-400)",
    fontSize: 10,
    flexShrink: 0,
  },
  label: {
    flex: 1,
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
  wordCount: {
    fontFamily: "var(--font-mono)",
    fontSize: 10,
    color: "var(--color-neutral-500)",
    fontVariantNumeric: "tabular-nums",
    flexShrink: 0,
  },
};
