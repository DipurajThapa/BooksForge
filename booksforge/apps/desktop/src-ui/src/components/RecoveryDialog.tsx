import React from "react";
import type { RecoveryStatus } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";

interface Props {
  status: RecoveryStatus;
  onRestore: () => void;
  onDiscard: () => void;
}

export default function RecoveryDialog({ status, onRestore, onDiscard }: Props) {
  // Discard is the safe default for ESC on an alertdialog: it
  // dismisses the recovery prompt and leaves the user with the
  // last-saved state (no destructive action without explicit
  // Restore-then-Save).
  const { dialogProps, titleId } = useDialogA11y(onDiscard);
  const date = status.pending_at
    ? new Date(status.pending_at).toLocaleString()
    : "unknown time";

  return (
    <div style={s.overlay} role="presentation">
      {/* `useDialogA11y` provides the aria-modal/labelledby + ESC + focus
          plumbing; we override role to `alertdialog` because this is a
          destructive-choice prompt (per WAI-ARIA 1.2 guidance). */}
      <div {...dialogProps} role="alertdialog" style={s.dialog}>
        <div style={s.icon}>⚠️</div>
        <h2 id={titleId} style={s.title}>Unsaved changes found</h2>
        <p style={s.body}>
          BooksForge found an unsaved scene from a previous session (
          {date}). This may have been caused by an unexpected quit.
        </p>
        <p style={s.sub}>
          Would you like to restore the unsaved version, or discard it and
          use the last saved version?
        </p>
        <div style={s.actions}>
          <button style={s.discardBtn} onClick={onDiscard}>
            Discard
          </button>
          <button style={s.restoreBtn} onClick={onRestore}>
            Restore unsaved
          </button>
        </div>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.5)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 500,
  },
  dialog: {
    width: 440,
    background: "var(--color-surface)",
    borderRadius: 10,
    padding: "var(--space-8)",
    boxShadow: "0 24px 72px rgba(0,0,0,0.35)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-3)",
  },
  icon: { fontSize: 32, lineHeight: 1 },
  title: {
    margin: 0,
    fontSize: 18,
    fontWeight: 600,
    color: "var(--color-text-primary)",
  },
  body: {
    margin: 0,
    fontSize: 14,
    color: "var(--color-text-primary)",
    lineHeight: 1.6,
  },
  sub: {
    margin: 0,
    fontSize: 13,
    color: "var(--color-text-secondary)",
    lineHeight: 1.5,
  },
  actions: {
    display: "flex",
    justifyContent: "flex-end",
    gap: "var(--space-2)",
    marginTop: "var(--space-2)",
  },
  discardBtn: {
    padding: "var(--space-2) var(--space-4)",
    background: "transparent",
    border: "1px solid var(--color-border)",
    borderRadius: 5,
    fontSize: 14,
    color: "var(--color-text-secondary)",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  restoreBtn: {
    padding: "var(--space-2) var(--space-5)",
    background: "var(--color-amber-600)",
    border: "none",
    borderRadius: 5,
    fontSize: 14,
    fontWeight: 600,
    color: "#fff",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
};
