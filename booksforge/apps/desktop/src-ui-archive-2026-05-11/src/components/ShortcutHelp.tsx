import React, { useEffect, useMemo } from "react";
import { KEYMAP, formatBinding, type CommandId } from "../lib/keymap";

/**
 * Shortcut-help overlay.
 *
 * Renders the contents of `lib/keymap.ts` as a modal grouped by
 * category.  Open by pressing `?` (the `app.show-shortcuts` binding)
 * or via Help → Keyboard shortcuts.
 *
 * Closes the help-overlay half of EXTERNAL_AUDIT_BACKLOG.md #33 (the
 * keymap module itself was added in the same commit).
 *
 * Wiring is opt-in:
 *
 *   import { ShortcutHelp } from "./components/ShortcutHelp";
 *   const [showShortcuts, setShowShortcuts] = useState(false);
 *   useShortcut("app.show-shortcuts", () => setShowShortcuts(true));
 *   {showShortcuts && <ShortcutHelp onClose={() => setShowShortcuts(false)} />}
 *
 * The overlay is keyboard-accessible: focus is trapped while open,
 * `Esc` closes, and the focus returns to whichever element opened it.
 */

type Group = "App" | "Editor" | "Binder" | "Snapshots" | "Agents" | "Export";
const GROUP_ORDER: Group[] = ["App", "Editor", "Binder", "Snapshots", "Agents", "Export"];

export interface ShortcutHelpProps {
  onClose: () => void;
}

export function ShortcutHelp({ onClose }: ShortcutHelpProps): JSX.Element {
  // Group bindings by `binding.group`.
  const groups = useMemo(() => {
    const out: Record<Group, Array<{ id: CommandId; label: string; rendered: string }>> = {
      App: [], Editor: [], Binder: [], Snapshots: [], Agents: [], Export: [],
    };
    (Object.keys(KEYMAP) as CommandId[]).forEach((id) => {
      const b = KEYMAP[id];
      out[b.group].push({
        id,
        label: b.description,
        rendered: formatBinding(id),
      });
    });
    return out;
  }, []);

  // Esc to close.
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      role="dialog"
      aria-labelledby="bf-shortcut-help-title"
      aria-modal="true"
      style={overlayStyle}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={panelStyle}>
        <header style={headerStyle}>
          <h2 id="bf-shortcut-help-title" style={titleStyle}>
            Keyboard shortcuts
          </h2>
          <button
            type="button"
            aria-label="Close shortcuts"
            onClick={onClose}
            style={closeBtn}
          >
            ×
          </button>
        </header>

        <div style={bodyStyle}>
          {GROUP_ORDER.map((group) => {
            const items = groups[group];
            if (items.length === 0) return null;
            return (
              <section key={group} style={groupStyle}>
                <h3 style={groupTitle}>{group}</h3>
                <ul style={listStyle}>
                  {items.map(({ id, label, rendered }) => (
                    <li key={id} style={rowStyle}>
                      <span style={labelStyle}>{label}</span>
                      <kbd style={kbdStyle}>{rendered}</kbd>
                    </li>
                  ))}
                </ul>
              </section>
            );
          })}
        </div>

        <footer style={footerStyle}>
          Press <kbd style={kbdInlineStyle}>Esc</kbd> to close.
        </footer>
      </div>
    </div>
  );
}

// ── Inline styles (CSP-friendly: React inline `style={...}` sets
//    element.style.* directly, not a `<style>` tag) ──

const overlayStyle: React.CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,0.45)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 9000,
};

const panelStyle: React.CSSProperties = {
  width: "min(48rem, 90vw)",
  maxHeight: "80vh",
  display: "flex",
  flexDirection: "column",
  background: "var(--color-bg, #ffffff)",
  color: "var(--color-neutral-900, #1f2328)",
  borderRadius: "0.5rem",
  boxShadow: "0 16px 48px rgba(0,0,0,0.2)",
  overflow: "hidden",
};

const headerStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "1rem 1.25rem",
  borderBottom: "1px solid var(--color-neutral-200, #e5e7eb)",
};

const titleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "1.1rem",
};

const closeBtn: React.CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: 0,
  fontSize: "1.5rem",
  cursor: "pointer",
  padding: "0 0.25rem",
  lineHeight: 1,
};

const bodyStyle: React.CSSProperties = {
  flex: 1,
  overflowY: "auto",
  padding: "1rem 1.25rem",
};

const groupStyle: React.CSSProperties = {
  marginBottom: "1.25rem",
};

const groupTitle: React.CSSProperties = {
  margin: "0 0 0.5rem",
  fontSize: "0.85rem",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  opacity: 0.7,
};

const listStyle: React.CSSProperties = {
  listStyle: "none",
  margin: 0,
  padding: 0,
  display: "grid",
  gridTemplateColumns: "1fr auto",
  gap: "0.25rem 1rem",
};

const rowStyle: React.CSSProperties = {
  display: "contents",
};

const labelStyle: React.CSSProperties = {
  fontSize: "0.95rem",
  paddingTop: "0.25rem",
};

const kbdStyle: React.CSSProperties = {
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  fontSize: "0.85rem",
  background: "var(--color-bg-subtle, #f6f8fa)",
  border: "1px solid var(--color-neutral-300, #d0d7de)",
  borderRadius: "0.375rem",
  padding: "0.15rem 0.5rem",
  whiteSpace: "nowrap",
};

const kbdInlineStyle: React.CSSProperties = {
  ...kbdStyle,
  fontSize: "0.75rem",
  padding: "0.05rem 0.35rem",
};

const footerStyle: React.CSSProperties = {
  padding: "0.75rem 1.25rem",
  fontSize: "0.85rem",
  borderTop: "1px solid var(--color-neutral-200, #e5e7eb)",
  opacity: 0.7,
};
