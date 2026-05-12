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
import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";
import type { NodeInfo } from "@booksforge/shared-types";

/** F8 — Imperative handle exposed to the parent route (Manuscript)
 *  so a keyboard shortcut can focus the first row in the binder
 *  without the parent having to manage DOM refs itself. */
export interface BinderHandle {
  /** Move keyboard focus to the first visible row. No-op if the
   *  tree is empty. */
  focusFirstRow(): void;
}

interface Props {
  nodes:            NodeInfo[];
  selectedSceneId:  string | null;
  onSelectScene:    (id: string) => void;
  /** F8 — Optional rename handler. Called when the writer commits
   *  an inline rename (Enter key or input blur). Returns the
   *  IPC promise so the binder can show a transient state. If
   *  omitted, rename UI is hidden. */
  onRenameNode?:    (id: string, newTitle: string) => Promise<void>;
  /** Right-click handler. Receives the targeted node + viewport
   *  cursor coords so the parent can mount a context menu near
   *  the cursor. Omit to disable right-click affordances. */
  onContextMenu?:   (info: { node: NodeInfo; x: number; y: number }) => void;
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

const Binder = forwardRef<BinderHandle, Props>(function Binder(
  { nodes, selectedSceneId, onSelectScene, onRenameNode, onContextMenu },
  ref,
) {
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

  // F8 — Rename state lifted into the parent so any descendant row
  // can enter rename mode and the others stay read-only.
  const [renamingId, setRenamingId] = useState<string | null>(null);

  // F8 — DOM ref to the binder nav so the imperative handle can find
  // the first focusable row.
  const navRef = useRef<HTMLElement>(null);
  useImperativeHandle(ref, () => ({
    focusFirstRow() {
      const first = navRef.current?.querySelector<HTMLButtonElement>(
        "button[data-binder-row]"
      );
      first?.focus();
    },
  }), []);

  // Find the project root (kind === "project") to anchor the heading.
  const projectRoot = tree.find((t) => t.info.kind === "project");
  const topLevel = projectRoot ? projectRoot.children : tree;

  return (
    <nav style={s.binder} aria-label="Manuscript binder" ref={navRef}>
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
              renamingId={renamingId}
              onStartRename={onRenameNode ? setRenamingId : undefined}
              onCommitRename={onRenameNode}
              onCancelRename={() => setRenamingId(null)}
              onContextMenu={onContextMenu}
            />
          ))
        )}
      </ol>
    </nav>
  );
});

export default Binder;

interface BinderRowProps {
  node:             TreeNode;
  depth:            number;
  selectedSceneId:  string | null;
  collapsed:        Set<string>;
  onToggle:         (id: string) => void;
  onSelect:         (id: string) => void;
  // F8 — rename plumbing (all optional; when omitted, rows are not renamable)
  renamingId?:      string | null;
  onStartRename?:   (id: string) => void;
  onCommitRename?:  (id: string, newTitle: string) => Promise<void>;
  onCancelRename?:  () => void;
  // Context-menu plumbing (parent owns the menu; we just forward the event).
  onContextMenu?:   (info: { node: NodeInfo; x: number; y: number }) => void;
}

function BinderRow(props: BinderRowProps) {
  const {
    node, depth, selectedSceneId, collapsed, onToggle, onSelect,
    renamingId, onStartRename, onCommitRename, onCancelRename,
    onContextMenu,
  } = props;
  const { info, children } = node;
  const isScene    = info.kind === "scene";
  const isExpanded = !collapsed.has(info.id);
  const isSelected = isScene && info.id === selectedSceneId;
  const hasChildren = children.length > 0;
  const wordRollup = isScene ? info.word_count : sumWords(node);
  const isRenaming = renamingId === info.id;

  function handleClick() {
    if (isRenaming) return; // ignore row clicks while editing the title
    if (isScene) {
      onSelect(info.id);
    } else if (hasChildren) {
      onToggle(info.id);
    }
  }

  function handleDoubleClick(e: ReactMouseEvent) {
    // F8 — Double-click any row to rename. The handler is wired only
    // when the parent passes `onStartRename`; otherwise nothing happens.
    if (!onStartRename) return;
    e.preventDefault();
    e.stopPropagation();
    onStartRename(info.id);
  }

  function handleContextMenu(e: ReactMouseEvent) {
    if (!onContextMenu) return;
    e.preventDefault();
    e.stopPropagation();
    onContextMenu({ node: info, x: e.clientX, y: e.clientY });
  }

  return (
    <li>
      {isRenaming && onCommitRename && onCancelRename ? (
        <RenameInput
          initial={info.title || untitledLabel(info.kind)}
          depth={depth}
          onCommit={(next) => onCommitRename(info.id, next).finally(onCancelRename)}
          onCancel={onCancelRename}
        />
      ) : (
        <button
          data-binder-row=""
          style={{
            ...s.row,
            ...(isSelected ? s.rowSelected : {}),
            paddingLeft: 8 + depth * 14,
          }}
          onClick={handleClick}
          onDoubleClick={handleDoubleClick}
          onContextMenu={handleContextMenu}
          aria-expanded={hasChildren ? isExpanded : undefined}
          aria-current={isSelected ? "page" : undefined}
          title={
            isScene
              ? `${info.title}${onStartRename ? " · double-click to rename" : ""}`
              : `${info.title} — ${wordRollup.toLocaleString()} words${onStartRename ? " · double-click to rename" : ""}`
          }
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
      )}
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
              renamingId={renamingId}
              onStartRename={onStartRename}
              onCommitRename={onCommitRename}
              onCancelRename={onCancelRename}
              onContextMenu={onContextMenu}
            />
          ))}
        </ol>
      )}
    </li>
  );
}

/**
 * F8 — Inline rename input rendered in place of a binder row. Auto-
 * focuses on mount, selects all text so the writer can type-replace,
 * commits on Enter / blur, cancels on Escape.
 *
 * Empty / whitespace-only titles are NOT committed (the row keeps
 * its old title); use Cancel for "I didn't mean to start renaming".
 */
function RenameInput({
  initial, depth, onCommit, onCancel,
}: {
  initial:   string;
  depth:     number;
  onCommit:  (next: string) => void;
  onCancel:  () => void;
}) {
  const [value, setValue] = useState(initial);
  const inputRef = useRef<HTMLInputElement>(null);
  // Track whether we already committed so a stray blur after Enter
  // doesn't re-fire the IPC.
  const committedRef = useRef(false);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  function commit() {
    if (committedRef.current) return;
    const trimmed = value.trim();
    if (trimmed.length === 0 || trimmed === initial) {
      onCancel();
      return;
    }
    committedRef.current = true;
    onCommit(trimmed);
  }

  return (
    <input
      ref={inputRef}
      style={{ ...s.renameInput, marginLeft: 8 + depth * 14 }}
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter")  { e.preventDefault(); commit(); }
        if (e.key === "Escape") { e.preventDefault(); onCancel(); }
      }}
      onBlur={commit}
    />
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
  // F8 — Inline rename input style: matches the row's text size and
  // sits in the same horizontal track so it doesn't visually jump.
  renameInput: {
    display: "block",
    boxSizing: "border-box",
    width: "calc(100% - 24px)",
    margin: "4px 12px 4px 0",
    padding: "4px 8px",
    border: "1px solid var(--color-amber-500, #f59e0b)",
    borderRadius: 3,
    background: "#fff",
    color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)",
    fontSize: 13,
    outline: "none",
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
