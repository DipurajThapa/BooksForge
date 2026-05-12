/**
 * BinderContextMenu — right-click affordance for Binder rows.
 *
 * Positions itself at the cursor anchor passed in by the parent
 * (Manuscript). Items are filtered by node kind so the writer only
 * sees what's valid for the row they targeted.
 *
 * The component is purely presentational — every action is passed
 * back to the parent as a callback. The parent owns the IPC + the
 * snapshot-undo flow for delete.
 *
 * Closes on ESC, outside-click, or after the writer picks an item.
 */
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import type { NodeInfo } from "@booksforge/shared-types";

export interface BinderContextMenuAnchor {
  /** Viewport x in CSS pixels. */
  x: number;
  /** Viewport y in CSS pixels. */
  y: number;
  /** The node the writer right-clicked. */
  node: NodeInfo;
}

interface Props {
  anchor:    BinderContextMenuAnchor;
  onClose:   () => void;
  onNewScene:    (parentNode: NodeInfo) => void;
  onNewChapter:  () => void;
  onRename:      (node: NodeInfo) => void;
  onDelete:      (node: NodeInfo) => void;
}

export default function BinderContextMenu({
  anchor, onClose, onNewScene, onNewChapter, onRename, onDelete,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside-click or ESC.
  useEffect(() => {
    function onDocDown(e: MouseEvent) {
      if (!ref.current) return;
      if (e.target instanceof Node && ref.current.contains(e.target)) return;
      onClose();
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    // Mousedown rather than click so the menu closes before any
    // accidental focus shift to a clicked element under it.
    document.addEventListener("mousedown", onDocDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  // Clamp menu to viewport so right-clicking near the bottom edge
  // doesn't push items off-screen.
  const left = clamp(anchor.x, 8, window.innerWidth  - 220);
  const top  = clamp(anchor.y, 8, window.innerHeight - 220);

  const { node } = anchor;
  const isScene   = node.kind === "scene";
  const isChapter = node.kind === "chapter";
  const isPart    = node.kind === "part";
  const isProject = node.kind === "project";

  // "New scene" runs under a chapter — when right-clicking a scene
  // itself, we hoist to the chapter (the scene's parent).
  // "New chapter" always runs at the project root.
  function pick(fn: () => void) {
    return () => { onClose(); fn(); };
  }

  return (
    <div
      ref={ref}
      style={{ ...s.menu, left, top }}
      role="menu"
      aria-label={`Binder actions for ${node.title || node.kind}`}
    >
      <div style={s.headingRow}>
        <span style={s.heading}>{node.title || node.kind}</span>
        <span style={s.kindBadge}>{node.kind}</span>
      </div>

      {isChapter && (
        <button
          style={s.item}
          role="menuitem"
          onClick={pick(() => onNewScene(node))}
        >
          + New scene in this chapter
        </button>
      )}
      {isScene && (
        <button
          style={s.item}
          role="menuitem"
          onClick={pick(() => onNewScene(node))}
          title="Adds a new scene under the same chapter as this scene."
        >
          + New scene in this chapter
        </button>
      )}
      {(isProject || isPart || isChapter || isScene) && (
        <button
          style={s.item}
          role="menuitem"
          onClick={pick(onNewChapter)}
        >
          + New chapter at end
        </button>
      )}

      <div style={s.divider} aria-hidden="true" />

      {!isProject && (
        <button
          style={s.item}
          role="menuitem"
          onClick={pick(() => onRename(node))}
        >
          Rename…
        </button>
      )}
      {(isScene || isChapter) && (
        <button
          style={s.itemDanger}
          role="menuitem"
          onClick={pick(() => onDelete(node))}
        >
          Delete {node.kind}…
        </button>
      )}
    </div>
  );
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, v));
}

const s: Record<string, CSSProperties> = {
  menu: {
    position: "fixed",
    minWidth: 212,
    background: "#fff",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 6,
    boxShadow: "0 8px 24px rgba(0,0,0,0.18)",
    padding: 4,
    fontFamily: "var(--font-ui)",
    zIndex: 9999,
  },
  headingRow: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "6px 8px 4px",
    gap: 8,
  },
  heading: {
    fontSize: 12, fontWeight: 600,
    color: "var(--color-neutral-900)",
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
    flex: 1,
  },
  kindBadge: {
    fontFamily: "var(--font-mono)",
    fontSize: 10, color: "var(--color-neutral-500)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  divider: {
    height: 1,
    background: "var(--color-neutral-200)",
    margin: "4px 0",
  },
  item: {
    display: "block",
    width: "100%",
    padding: "6px 10px",
    background: "transparent",
    border: "none",
    borderRadius: 4,
    fontFamily: "inherit",
    fontSize: 13,
    color: "var(--color-neutral-900)",
    textAlign: "left",
    cursor: "pointer",
  },
  itemDanger: {
    display: "block",
    width: "100%",
    padding: "6px 10px",
    background: "transparent",
    border: "none",
    borderRadius: 4,
    fontFamily: "inherit",
    fontSize: 13,
    color: "var(--color-red-700, #b91c1c)",
    textAlign: "left",
    cursor: "pointer",
  },
};
