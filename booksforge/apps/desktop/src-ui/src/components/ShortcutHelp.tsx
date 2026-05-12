/**
 * ShortcutHelp — modal overlay listing every shortcut in `KEYMAP`,
 * grouped by section.
 *
 * Trigger: `?` key (registered by App.tsx via `useShortcut("app.show-shortcuts")`).
 * Close: ESC, click on the backdrop, or the explicit close button.
 *
 * Background: the `lib/keymap.ts` module has carried a centralised
 * shortcut registry for some time, but writers had no way to
 * *discover* what existed — the audit called this out as "keyboard
 * shortcuts: defined, never wired" (it's now mostly wired but the
 * help surface was the missing piece).
 *
 * Privacy: the overlay reads only the local `KEYMAP` constant; no
 * IPC, no remote sink. Pure-presentational component.
 */
import type { CSSProperties } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { KEYMAP, formatBinding, type CommandId, type KeyBinding } from "../lib/keymap";

interface Props {
  onClose: () => void;
}

const GROUP_ORDER: Array<KeyBinding["group"]> = [
  "App",
  "Editor",
  "Binder",
  "Agents",
  "Snapshots",
  "Export",
];

/**
 * Group every `KEYMAP` entry by its `group` field, preserving the
 * insertion order within each group so the rendered list matches
 * what `keymap.ts` declares (not alphabetical — declaration order
 * is the intentional reading order).
 */
function groupedEntries(): Record<KeyBinding["group"], Array<[CommandId, KeyBinding]>> {
  const out = Object.fromEntries(
    GROUP_ORDER.map((g) => [g, [] as Array<[CommandId, KeyBinding]>])
  ) as Record<KeyBinding["group"], Array<[CommandId, KeyBinding]>>;
  for (const [id, binding] of Object.entries(KEYMAP) as Array<[CommandId, KeyBinding]>) {
    out[binding.group].push([id, binding]);
  }
  return out;
}

export default function ShortcutHelp({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const groups = groupedEntries();

  return (
    <div
      style={s.backdrop}
      role="presentation"
      onMouseDown={(e) => {
        // Close only when the click started on the backdrop itself,
        // not on a descendant — prevents accidental close while
        // dragging text inside the dialog.
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <h2 id={titleId} style={s.title}>Keyboard shortcuts</h2>
          <button
            type="button"
            onClick={onClose}
            style={s.closeBtn}
            aria-label="Close shortcuts overlay"
          >
            ×
          </button>
        </header>
        <div style={s.body}>
          {GROUP_ORDER.map((group) => {
            const entries = groups[group];
            if (entries.length === 0) return null;
            return (
              <section key={group} style={s.section}>
                <h3 style={s.sectionTitle}>{group}</h3>
                <dl style={s.list}>
                  {entries.map(([id, binding]) => (
                    <ShortcutRow key={id} commandId={id} description={binding.description} />
                  ))}
                </dl>
              </section>
            );
          })}
        </div>
        <footer style={s.footer}>
          Press <Kbd>Esc</Kbd> or click outside to close.
        </footer>
      </div>
    </div>
  );
}

function ShortcutRow({
  commandId,
  description,
}: {
  commandId:   CommandId;
  description: string;
}) {
  return (
    <>
      <dt style={s.dt}>{description}</dt>
      <dd style={s.dd}>
        <Kbd>{formatBinding(commandId)}</Kbd>
      </dd>
    </>
  );
}

function Kbd({ children }: { children: React.ReactNode }) {
  return <kbd style={s.kbd}>{children}</kbd>;
}

const s: Record<string, CSSProperties> = {
  backdrop: {
    position: "fixed",
    inset: 0,
    background: "rgba(15,15,15,0.55)",
    display: "flex", alignItems: "center", justifyContent: "center",
    zIndex: 9998,
    padding: 24,
  },
  dialog: {
    width: "min(640px, 100%)",
    maxHeight: "min(85vh, 720px)",
    display: "flex", flexDirection: "column",
    background: "#fff",
    borderRadius: 8,
    boxShadow: "0 24px 60px rgba(0,0,0,0.35)",
    fontFamily: "var(--font-ui)",
    outline: "none",
  },
  header: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "14px 18px",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  title: {
    margin: 0,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 18, fontWeight: 700,
    color: "var(--color-neutral-900)",
  },
  closeBtn: {
    background: "transparent",
    border: "none",
    fontSize: 24, lineHeight: 1,
    color: "var(--color-neutral-500)",
    cursor: "pointer",
    padding: "0 4px",
  },
  body: {
    flex: 1,
    overflowY: "auto",
    padding: "16px 18px 4px",
    display: "flex", flexDirection: "column", gap: 18,
  },
  section: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  sectionTitle: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    letterSpacing: "0.08em", textTransform: "uppercase",
    color: "var(--color-neutral-500)",
  },
  list: {
    display: "grid",
    gridTemplateColumns: "1fr auto",
    rowGap: 4, columnGap: 16,
    margin: 0, padding: 0,
  },
  dt: {
    margin: 0,
    fontSize: 13,
    color: "var(--color-neutral-800)",
  },
  dd: {
    margin: 0,
    textAlign: "right",
  },
  kbd: {
    display: "inline-block",
    padding: "2px 8px",
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-neutral-800)",
    background: "var(--color-neutral-100)",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    boxShadow: "0 1px 0 var(--color-neutral-200)",
    minWidth: 22,
    textAlign: "center",
    whiteSpace: "nowrap",
  },
  footer: {
    padding: "10px 18px 14px",
    borderTop: "1px solid var(--color-neutral-200)",
    fontSize: 11,
    color: "var(--color-neutral-500)",
  },
};
