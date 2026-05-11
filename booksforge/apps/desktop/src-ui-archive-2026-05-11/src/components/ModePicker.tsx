/**
 * ModePicker — one-time prompt asking the writer how they want to use
 * BooksForge. Shown when `loadAppMode()` returns `null` (i.e. the user
 * hasn't picked yet). Re-shown if they Reset from the Settings panel.
 *
 * Two cards, equal weight, no preselection. The writer commits with a
 * single click; the choice persists via `setAppMode`.
 */
import React from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { type AppMode, modeBlurb, modeEmoji, modeLabel, setAppMode } from "../lib/appMode";

interface Props {
  onChosen: (mode: AppMode) => void;
}

export default function ModePicker({ onChosen }: Props) {
  // Non-dismissible — the user MUST pick. Closing without choosing
  // would leave the toolbar in an undefined state.
  const { dialogProps, titleId } = useDialogA11y(() => undefined);

  function pick(mode: AppMode) {
    setAppMode(mode);
    onChosen(mode);
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <h2 id={titleId} style={s.title}>How do you want to write this book?</h2>
          <p style={s.subtitle}>
            You can switch any time from the toolbar. This sets the default
            shape of the editor.
          </p>
        </header>

        <div style={s.cards}>
          <Card mode="manual"     onClick={() => pick("manual")} />
          <Card mode="ai_writer"  onClick={() => pick("ai_writer")} />
        </div>
      </div>
    </div>
  );
}

function Card({ mode, onClick }: { mode: AppMode; onClick: () => void }) {
  return (
    <button style={s.card} onClick={onClick} aria-label={`Pick ${modeLabel(mode)} mode`}>
      <div style={s.cardEmoji}>{modeEmoji(mode)}</div>
      <div style={s.cardTitle}>{modeLabel(mode)}</div>
      <div style={s.cardBody}>{modeBlurb(mode)}</div>
    </button>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", inset: 0,
    background: "rgba(0,0,0,0.55)",
    display: "flex", alignItems: "center", justifyContent: "center",
    zIndex: 1100, backdropFilter: "blur(2px)",
  },
  dialog: {
    width: 720, maxWidth: "92vw",
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)", borderRadius: 10,
    boxShadow: "0 24px 80px rgba(0,0,0,0.4)",
    padding: "var(--space-6)",
    fontFamily: "var(--font-ui)",
  },
  header: {
    textAlign: "center",
    marginBottom: "var(--space-5)",
  },
  title: {
    margin: 0, fontSize: 22, fontWeight: 700,
    color: "var(--color-text-primary)", fontFamily: "var(--font-prose)",
  },
  subtitle: {
    margin: "var(--space-2) 0 0",
    fontSize: 13, color: "var(--color-text-secondary)",
  },
  cards: {
    display: "grid", gridTemplateColumns: "1fr 1fr",
    gap: "var(--space-4)",
  },
  card: {
    display: "flex", flexDirection: "column", alignItems: "flex-start",
    gap: "var(--space-2)",
    padding: "var(--space-5)",
    background: "var(--color-surface-raised, rgba(0,0,0,0.02))",
    border: "1px solid var(--color-border)",
    borderRadius: 8, textAlign: "left", cursor: "pointer",
    fontFamily: "inherit",
    transition: "border-color 100ms ease, transform 100ms ease",
  },
  cardEmoji: { fontSize: 32, lineHeight: 1 },
  cardTitle: {
    fontSize: 17, fontWeight: 600,
    color: "var(--color-text-primary)", fontFamily: "var(--font-prose)",
  },
  cardBody: {
    fontSize: 13, color: "var(--color-text-secondary)",
    lineHeight: 1.55,
  },
};
