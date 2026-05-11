/**
 * Book-kind overlay (Phase 5B + 5C of PRODUCT_ROADMAP_E2E.md).
 *
 * Used in two modes:
 *   1. **Onboarding** (`mode="onboarding"`) — pops up when an opened
 *      project's `book_kind` is `null` (legacy projects from before
 *      Phase 4). Shown as a non-dismissible modal: the user MUST pick
 *      before continuing because the workflow router needs the kind.
 *   2. **Settings edit** (`mode="settings"`) — opened from the
 *      SettingsPanel "Change book kind" action. Dismissible.
 *
 * Either way the chosen kind is persisted via `project_kind_set`,
 * which writes the manifest atomically.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import type { BookKind } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  mode:           "onboarding" | "settings";
  /** Current book_kind, if any. Used to highlight the active card. */
  currentKind:    BookKind | null;
  /** Called after a successful save. Parent updates its open-project state. */
  onSaved:        (newKind: BookKind) => void;
  /** Onboarding mode ignores this; settings mode uses it for Cancel. */
  onClose:        () => void;
}

interface KindOption {
  key:        BookKind;
  name:       string;
  blurb:      string;
  supported:  boolean;
}

const OPTIONS: KindOption[] = [
  {
    key: "literary-fiction",
    name: "Literary Fiction",
    blurb: "Voice-driven prose. Sentence-craft over plot. Polish: voice → metaphor → dialogue → tension.",
    supported: true,
  },
  {
    key: "genre-fiction",
    name: "Genre Fiction",
    blurb: "Cozy fantasy / thriller / romance / mystery / YA. Pacing first. Polish: tension → dialogue → metaphor → voice.",
    supported: true,
  },
  {
    key: "non-fiction",
    name: "Non-Fiction",
    blurb: "Strategy / popular science / long-form essay / business. Argument + evidence weighted highest. Never fabricates stats.",
    supported: true,
  },
  {
    key: "memoir",
    name: "Memoir",
    blurb: "Prose-craft + interiority weighted like literary, with non-fiction's no-fabrication discipline.",
    supported: true,
  },
  {
    key: "childrens-book",
    name: "Children's Book (coming soon)",
    blurb: "Different word counts, layout, reading-level constraints. Out of MVP scope.",
    supported: false,
  },
];

export default function BookKindOverlay({ mode, currentKind, onSaved, onClose }: Props) {
  // Onboarding mode: dismissal is disabled.
  const { dialogProps, titleId } = useDialogA11y(
    mode === "settings" ? onClose : () => undefined,
  );
  const [selected, setSelected] = useState<BookKind | null>(currentKind);
  const [busy,     setBusy]     = useState(false);
  const [error,    setError]    = useState<string | null>(null);

  const valid = selected !== null
    && OPTIONS.find(o => o.key === selected)?.supported === true;

  async function handleSave() {
    if (!valid || !selected) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.projectKindSet({ book_kind: selected });
      onSaved(selected);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  const isOnboarding = mode === "onboarding";

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>
            {isOnboarding
              ? "Pick a book kind to continue"
              : "Change book kind"}
          </strong>
          {!isOnboarding && (
            <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
          )}
        </header>

        <div style={s.body}>
          {isOnboarding ? (
            <p style={s.blurb}>
              This project was created before BooksForge tracked book kind. Pick
              one now so the drafter, polish stack, and rubric weights tune
              correctly. You can change this later in Settings.
            </p>
          ) : (
            <p style={s.blurb}>
              Changing book kind re-routes the drafter prompts, polish-stack
              ordering, and rubric weights. Existing scenes are unchanged
              until you run agents on them next.
            </p>
          )}

          <div style={s.grid}>
            {OPTIONS.map(opt => {
              const sel = selected === opt.key;
              return (
                <button
                  key={opt.key}
                  style={{
                    ...s.card,
                    ...(sel ? s.cardSelected : {}),
                    ...(opt.supported ? {} : s.cardDisabled),
                  }}
                  onClick={() => opt.supported && setSelected(opt.key)}
                  disabled={!opt.supported}
                  title={opt.supported ? opt.blurb : "Coming soon."}
                >
                  <strong style={s.cardName}>{opt.name}</strong>
                  <div style={s.cardBlurb}>{opt.blurb}</div>
                </button>
              );
            })}
          </div>

          {error && <div style={s.error}>{error}</div>}

          <div style={s.footer}>
            {!isOnboarding && (
              <button style={s.ghostBtn} onClick={onClose} disabled={busy}>
                Cancel
              </button>
            )}
            <button
              style={s.primaryBtn}
              onClick={handleSave}
              disabled={!valid || busy}
            >
              {busy ? "Saving…" : isOnboarding ? "Save and continue" : "Save"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay: { position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", zIndex: 200, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:  { width: "min(820px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden", boxShadow: "0 20px 60px rgba(0,0,0,0.4)" },
  header:  { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 16px", borderBottom: "1px solid var(--color-border)" },
  close:   { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:    { padding: 14, overflowY: "auto", display: "flex", flexDirection: "column", gap: 12 },
  blurb:   { margin: 0, fontSize: 13, opacity: 0.85 },
  grid:    { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 },
  card:    { textAlign: "left", padding: 12, background: "var(--color-bg)", color: "inherit", border: "2px solid var(--color-border)", borderRadius: 6, cursor: "pointer", display: "flex", flexDirection: "column", gap: 4 },
  cardSelected: { borderColor: "var(--color-success, #2e7d32)", background: "var(--color-success-bg, rgba(46,125,50,0.08))" },
  cardDisabled: { opacity: 0.5, cursor: "not-allowed" },
  cardName:  { fontSize: 14 },
  cardBlurb: { fontSize: 12, opacity: 0.85, lineHeight: 1.4 },
  footer:  { display: "flex", justifyContent: "flex-end", gap: 8 },
  ghostBtn: { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "transparent", color: "inherit" },
  primaryBtn: { padding: "6px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontWeight: 600 },
  error:   { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12 },
};
