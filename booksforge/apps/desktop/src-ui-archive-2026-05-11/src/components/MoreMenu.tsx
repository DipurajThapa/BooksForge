/**
 * MoreMenu — the `⋯` overflow that absorbs the 12 secondary toolbar
 * buttons that used to compete for attention with the writing actions.
 *
 * Items are grouped:
 *   1. Project content      — Brief, Knowledge
 *   2. AI workflow          — Workflow gates, Agents, AI Setup, Debug AI
 *   3. Quality & versioning — Check, Snapshots
 *   4. Output               — Export, Publish
 *   5. App                  — Focus mode, Settings, Help
 *
 * Closes on Escape, click outside, or item selection. Anchored to the
 * trigger button via a fixed-position fallback (no floating-ui dep).
 */
import React, { useEffect, useRef } from "react";

export interface MoreMenuItem {
  label:    string;
  onSelect: () => void;
  /** Optional one-line hint shown under the label in 11pt grey. */
  hint?:    string;
  /** Set true to render a top-divider above this item. */
  divider?: boolean;
  /** When true, the item is rendered greyed-out and click is a no-op.
   *  Used when the action requires a scene to be selected etc. */
  disabled?: boolean;
}

interface Props {
  open:    boolean;
  onClose: () => void;
  items:   MoreMenuItem[];
  /** Anchor point — fixed-position px coords from window top-left.
   *  Caller computes from the trigger button's bounding rect. */
  anchor:  { top: number; right: number };
}

export default function MoreMenu({ open, onClose, items, anchor }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener("keydown", onKey);
    // Defer the click listener so the same click that opened the menu
    // doesn't immediately close it.
    const id = window.setTimeout(
      () => window.addEventListener("mousedown", onClick),
      0,
    );
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onClick);
      window.clearTimeout(id);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      ref={ref}
      role="menu"
      style={{ ...s.menu, top: anchor.top, right: anchor.right }}
    >
      {items.map((it, idx) => (
        <React.Fragment key={`${idx}-${it.label}`}>
          {it.divider && <div style={s.divider} aria-hidden="true" />}
          <button
            style={{
              ...s.item,
              ...(it.disabled ? s.itemDisabled : {}),
            }}
            role="menuitem"
            disabled={it.disabled}
            onClick={() => {
              if (it.disabled) return;
              it.onSelect();
              onClose();
            }}
          >
            <span style={s.itemLabel}>{it.label}</span>
            {it.hint && <span style={s.itemHint}>{it.hint}</span>}
          </button>
        </React.Fragment>
      ))}
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  menu: {
    position: "fixed",
    minWidth: 260,
    background: "var(--color-surface, #fff)",
    border: "1px solid var(--color-border)",
    borderRadius: 6,
    boxShadow: "0 12px 36px rgba(0,0,0,0.18)",
    padding: "4px",
    fontFamily: "var(--font-ui)",
    fontSize: 13,
    zIndex: 900,
  },
  divider: {
    height: 1,
    background: "var(--color-border)",
    margin: "4px 0",
  },
  item: {
    display: "flex", flexDirection: "column", alignItems: "flex-start",
    width: "100%",
    padding: "6px 10px",
    background: "transparent", border: "none", cursor: "pointer",
    textAlign: "left", borderRadius: 4,
    color: "var(--color-text-primary)",
    fontFamily: "inherit", fontSize: 13,
  },
  itemDisabled: {
    cursor: "not-allowed",
    color: "var(--color-text-tertiary)",
  },
  itemLabel: { fontWeight: 500 },
  itemHint:  { fontSize: 11, color: "var(--color-text-tertiary)", marginTop: 2 },
};
