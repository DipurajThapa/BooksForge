/**
 * Onboarding tour (BACKLOG §I5) — three-step welcome overlay shown
 * once per browser/local-storage session.  Dismissed by clicking
 * "Got it" or the X.  Storage key: `booksforge.onboarding.v1.shown`.
 *
 * Intentionally lightweight — no DOM-anchored tooltips, no fancy
 * highlights.  Just a clear, three-card walkthrough that points at the
 * key concepts a first-time user needs to know.  When the writer is
 * ready for more, they can open the Help drawer.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";

interface Props { onClose: () => void; }

const STORAGE_KEY = "booksforge.onboarding.v1.shown";

export function shouldShowOnboarding(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) !== "1";
  } catch {
    return false;
  }
}

export function markOnboardingShown(): void {
  try { localStorage.setItem(STORAGE_KEY, "1"); } catch { /* ignore */ }
}

const STEPS: Array<{ title: string; body: React.ReactNode }> = [
  {
    title: "Welcome to BooksForge",
    body: (
      <>
        <p>
          BooksForge is a local-first writing app.  Your manuscript stays
          on your device — no cloud, no telemetry, no surprises.
        </p>
        <p>
          The <strong>Binder</strong> on the left holds your project tree.
          Add Parts, Chapters, and Scenes; click any Scene to start writing
          in the centre pane.
        </p>
      </>
    ),
  },
  {
    title: "Snapshots have your back",
    body: (
      <>
        <p>
          Every five minutes during active writing, BooksForge takes an
          automatic snapshot of your manuscript.  Click <strong>Snapshots</strong>{" "}
          in the toolbar to browse them, diff against the current state,
          or restore.
        </p>
        <p>
          Before any agent edits your prose, a <em>pre-edit snapshot</em>{" "}
          is taken automatically — so you can always revert.
        </p>
      </>
    ),
  },
  {
    title: "Agents are optional",
    body: (
      <>
        <p>
          With <a href="https://ollama.com" target="_blank" rel="noreferrer">
            Ollama
          </a>{" "}
          running locally, the <strong>Agents</strong> button opens the swarm
          — copyeditor, humanizer, continuity checker, chapter drafter, and
          more.  Each runs entirely on your hardware.
        </p>
        <p>
          Don't have Ollama yet?  That's fine.  Everything else (the
          editor, snapshots, exports) works without it.  Open the
          <strong> Help</strong> drawer any time for a tour of features.
        </p>
      </>
    ),
  },
];

export default function OnboardingTour({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [step, setStep] = useState(0);

  function dismiss() {
    markOnboardingShown();
    onClose();
  }

  function next() {
    if (step < STEPS.length - 1) {
      setStep(step + 1);
    } else {
      dismiss();
    }
  }

  const current = STEPS[step];

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <span style={s.dotRow}>
            {STEPS.map((_, i) => (
              <span
                key={i}
                style={{
                  ...s.dot,
                  background: i === step
                    ? "var(--color-accent, #2e7d32)"
                    : "var(--color-border)",
                }}
              />
            ))}
          </span>
          <button style={s.close} onClick={dismiss} aria-label="Skip onboarding">✕</button>
        </header>
        <div style={s.body}>
          <h2 id={titleId} style={s.title}>{current.title}</h2>
          <div style={s.copy}>{current.body}</div>
        </div>
        <footer style={s.footer}>
          <button style={s.linkBtn} onClick={dismiss}>Skip</button>
          <button style={s.primary} onClick={next}>
            {step < STEPS.length - 1 ? "Next" : "Got it"}
          </button>
        </footer>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.45)", zIndex: 60, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(540px, 92vw)", maxHeight: "85vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 16px" },
  dotRow:   { display: "flex", gap: 6 },
  dot:      { width: 8, height: 8, borderRadius: 4, transition: "background 0.2s" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "8px 24px 20px", flex: 1 },
  title:    { fontSize: 20, fontWeight: 600, margin: "8px 0 12px 0" },
  copy:     { fontSize: 14, lineHeight: 1.55 },
  footer:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 16px", borderTop: "1px solid var(--color-border)" },
  linkBtn:  { background: "transparent", border: "none", color: "inherit", cursor: "pointer", textDecoration: "underline", fontSize: 13, padding: 0 },
  primary:  { padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-accent, #2e7d32)", color: "white", fontWeight: 600 },
};
