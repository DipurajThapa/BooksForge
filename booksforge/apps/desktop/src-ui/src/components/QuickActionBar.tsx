/**
 * QuickActionBar — Cmd+K inline AI assist over the selected prose.
 *
 * Per `outputs/UI_UX_SPEC.md §5.2`: "Quick-action bar (Cmd/Ctrl+K):
 * Sharpen, Continue, Rephrase, Shorten, Expand. Each is a
 * single-shot call to Ollama using a versioned prompt template
 * — not an agent. Suggestions appear in a side panel with a diff
 * view; user accepts/rejects/regenerates."
 *
 * Scope of this PR:
 *   - Three presets the Rust backend already supports natively:
 *     `sharpen` | `continue` | `rephrase` (see
 *     `crates/booksforge-ipc/bindings/AiSuggestInput.ts`). Shorten
 *     and Expand are documented in the spec but would require new
 *     prompt templates on the Rust side — deferred until the
 *     backend exposes them.
 *   - Streaming preview via `ai-suggest:<job_id>:token` events.
 *   - Accept (insert into the TipTap editor at the selection) /
 *     Reject (close) / Regenerate (re-run the same preset).
 *
 * Diff view (per spec) is deferred to a follow-up PR — accepting
 * replaces the selection directly. The audit trail is preserved
 * server-side regardless (`ai_calls.id` returned in the done event).
 *
 * Backend boundary: this component uses only the existing
 * `ai_suggest` + `ai_cancel` IPC. No new commands. The locked
 * drafter / orchestrator / agents are untouched.
 */
import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { Editor } from "@booksforge/editor";
import { ipc } from "../lib/ipc";
import { useDialogA11y } from "../lib/useDialogA11y";
import { useToast } from "./ToastProvider";
import { errorMessage } from "../lib/errorMessage";

type Preset = "sharpen" | "continue" | "rephrase";

const PRESET_LABEL: Record<Preset, string> = {
  sharpen:  "Sharpen",
  continue: "Continue",
  rephrase: "Rephrase",
};

const PRESET_HINT: Record<Preset, string> = {
  sharpen:  "Tighten the selected prose without changing meaning.",
  continue: "Continue from the cursor in the same voice.",
  rephrase: "Restate the selection in a different way.",
};

type RunState =
  | { kind: "idle" }
  | { kind: "running"; preset: Preset; jobId: string; output: string }
  | { kind: "done";    preset: Preset; jobId: string; output: string; aiCallId: string; durationMs: number }
  | { kind: "error";   message: string };

interface Props {
  /** The currently-active scene node so the backend can attach the
   *  call to it (and read surrounding context for `continue`). */
  sceneNodeId: string | null;
  /** Plain-text snapshot of the writer's current selection (or
   *  surrounding paragraph if nothing is highlighted). Captured at
   *  open time so the bar's content doesn't shift if the writer
   *  clicks elsewhere while reviewing the suggestion. */
  initialScope: string;
  /** TipTap editor handle so Accept can write the suggestion back
   *  in place of the selection. Always wrapped in `editor.chain()`. */
  editor: Editor | null;
  /** Called when the writer dismisses or finishes the action. */
  onClose: () => void;
}

export default function QuickActionBar({ sceneNodeId, initialScope, editor, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [state, setState] = useState<RunState>({ kind: "idle" });
  const toast = useToast();
  // Track the job id in a ref so the event listeners get a stable
  // value even when React batches state updates around it.
  const jobIdRef = useRef<string | null>(null);
  const cancelRequestedRef = useRef<boolean>(false);

  // Subscribe to the live token / done streams whenever a job is
  // running. Cleaned up on state change so we never multi-subscribe.
  useEffect(() => {
    if (state.kind !== "running") return;
    let cancelled = false;
    let unToken: (() => void) | undefined;
    let unDone:  (() => void) | undefined;

    (async () => {
      unToken = await ipc.onAiSuggestToken(state.jobId, (e) => {
        if (cancelled) return;
        setState((cur) => cur.kind === "running" && cur.jobId === e.job_id
          ? { ...cur, output: cur.output + e.delta }
          : cur,
        );
      });
      unDone = await ipc.onAiSuggestDone(state.jobId, (e) => {
        if (cancelled) return;
        if (e.status === "ok") {
          setState({
            kind:       "done",
            preset:     state.preset,
            jobId:      e.job_id,
            output:     e.full_text,
            aiCallId:   e.ai_call_id,
            durationMs: Number(e.duration_ms),
          });
        } else if (e.status === "cancelled") {
          setState({ kind: "idle" });
        } else {
          setState({
            kind:    "error",
            message: e.error ?? `AI suggest returned status: ${e.status}`,
          });
        }
        jobIdRef.current = null;
      });
    })();

    return () => {
      cancelled = true;
      unToken?.();
      unDone?.();
    };
  }, [state.kind === "running" ? state.jobId : null]); // eslint-disable-line react-hooks/exhaustive-deps

  async function startPreset(preset: Preset) {
    if (!sceneNodeId) {
      toast.push({
        severity: "warning",
        body:     "Select a scene in the binder first — quick actions write into the active scene.",
      });
      return;
    }
    if (initialScope.trim().length === 0) {
      toast.push({
        severity: "warning",
        body:     "Nothing to act on. Place the cursor in a paragraph or highlight text, then try again.",
      });
      return;
    }
    cancelRequestedRef.current = false;
    try {
      const r = await ipc.aiSuggest({
        node_id:       sceneNodeId,
        preset,
        scope_text:    initialScope,
        model:         null,
        options_json:  null,
      });
      jobIdRef.current = r.job_id;
      setState({ kind: "running", preset, jobId: r.job_id, output: "" });
    } catch (e) {
      const msg = errorMessage(e);
      setState({ kind: "error", message: msg });
      toast.push({ severity: "error", title: "Quick action failed", body: msg });
    }
  }

  async function cancel() {
    if (jobIdRef.current && !cancelRequestedRef.current) {
      cancelRequestedRef.current = true;
      try { await ipc.aiCancel({ job_id: jobIdRef.current }); } catch { /* best-effort */ }
    }
    setState({ kind: "idle" });
  }

  function regenerate() {
    if (state.kind === "done") {
      void startPreset(state.preset);
    }
  }

  function accept() {
    if (state.kind !== "done" || !editor) return;
    // The TipTap chain replaces the current selection with the new
    // text. If there was no selection, the text inserts at the
    // cursor. The editor was bound to the selection that fed
    // `initialScope`, so the writer sees the new prose land where
    // they expected.
    editor.chain().focus().insertContent(state.output).run();
    toast.push({
      severity: "success",
      body:     `Inserted ${PRESET_LABEL[state.preset]} suggestion.`,
    });
    onClose();
  }

  const wordCount = initialScope.trim().split(/\s+/).filter(Boolean).length;

  return (
    <div
      style={s.backdrop}
      role="presentation"
      onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <h2 id={titleId} style={s.title}>Quick action</h2>
          <span style={s.scopeNote}>
            {wordCount > 0
              ? `${wordCount.toLocaleString()} word${wordCount === 1 ? "" : "s"} in scope`
              : "(no selection)"}
          </span>
          <button
            type="button"
            onClick={onClose}
            style={s.closeBtn}
            aria-label="Close quick action bar"
          >
            ×
          </button>
        </header>

        <div style={s.body}>
          {state.kind === "idle" && (
            <div style={s.presetGrid}>
              {(Object.keys(PRESET_LABEL) as Preset[]).map((p) => (
                <button
                  key={p}
                  style={s.presetBtn}
                  onClick={() => startPreset(p)}
                  title={PRESET_HINT[p]}
                >
                  <span style={s.presetLabel}>{PRESET_LABEL[p]}</span>
                  <span style={s.presetHint}>{PRESET_HINT[p]}</span>
                </button>
              ))}
            </div>
          )}

          {state.kind === "running" && (
            <div style={s.runningWrap}>
              <header style={s.runningHeader}>
                <span style={s.runningLabel}>
                  Running <b>{PRESET_LABEL[state.preset]}</b>…
                </span>
                <button style={s.ghostBtn} onClick={cancel}>Cancel</button>
              </header>
              <pre style={s.output}>{state.output || "Waiting for the first token…"}</pre>
            </div>
          )}

          {state.kind === "done" && (
            <div style={s.runningWrap}>
              <header style={s.runningHeader}>
                <span style={s.runningLabel}>
                  <b>{PRESET_LABEL[state.preset]}</b> · ~{Math.round(state.durationMs / 100) / 10}s
                </span>
              </header>
              <pre style={s.output}>{state.output}</pre>
              <div style={s.actions}>
                <button style={s.ghostBtn} onClick={onClose}>Reject</button>
                <button style={s.ghostBtn} onClick={regenerate}>Regenerate</button>
                <button style={s.primaryBtn} onClick={accept}>Accept &amp; insert</button>
              </div>
            </div>
          )}

          {state.kind === "error" && (
            <div style={s.runningWrap}>
              <div style={s.errorBox}>{state.message}</div>
              <div style={s.actions}>
                <button style={s.ghostBtn} onClick={() => setState({ kind: "idle" })}>
                  Try again
                </button>
              </div>
            </div>
          )}
        </div>

        <footer style={s.footer}>
          <span>Heavy-tier model. Streamed locally; the call lands in <code style={s.code}>ai_calls</code>.</span>
        </footer>
      </div>
    </div>
  );
}

const s: Record<string, CSSProperties> = {
  backdrop: {
    position: "fixed",
    inset: 0,
    background: "rgba(15,15,15,0.45)",
    display: "flex", alignItems: "flex-start", justifyContent: "center",
    paddingTop: "10vh",
    zIndex: 9997,
  },
  dialog: {
    width: "min(680px, 100%)",
    maxHeight: "78vh",
    display: "flex", flexDirection: "column",
    background: "#fff",
    borderRadius: 10,
    boxShadow: "0 24px 60px rgba(0,0,0,0.35)",
    fontFamily: "var(--font-ui)",
    outline: "none",
  },
  header: {
    display: "flex", alignItems: "center", gap: 12,
    padding: "12px 18px",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  title: {
    margin: 0,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 16, fontWeight: 700,
    color: "var(--color-neutral-900)",
    flexShrink: 0,
  },
  scopeNote: {
    flex: 1,
    fontSize: 12,
    color: "var(--color-neutral-500)",
    fontFamily: "var(--font-mono)",
  },
  closeBtn: {
    background: "transparent",
    border: "none",
    fontSize: 22, lineHeight: 1,
    color: "var(--color-neutral-500)",
    cursor: "pointer",
    padding: "0 4px",
  },
  body: {
    flex: 1,
    overflowY: "auto",
    padding: "16px 18px",
    display: "flex", flexDirection: "column", gap: 12,
  },
  presetGrid: {
    display: "grid",
    gridTemplateColumns: "1fr",
    gap: 8,
  },
  presetBtn: {
    display: "flex", flexDirection: "column", gap: 4,
    padding: "12px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    cursor: "pointer",
    textAlign: "left",
    fontFamily: "inherit",
  },
  presetLabel: {
    fontSize: 14, fontWeight: 600,
    color: "var(--color-amber-700, #b45309)",
  },
  presetHint: {
    fontSize: 12,
    color: "var(--color-neutral-600)",
    lineHeight: 1.45,
  },
  runningWrap: {
    display: "flex", flexDirection: "column", gap: 10,
  },
  runningHeader: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    fontSize: 12, color: "var(--color-neutral-600)",
  },
  runningLabel: {
    fontFamily: "var(--font-mono)",
    fontSize: 12,
  },
  output: {
    margin: 0,
    padding: "12px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 14,
    lineHeight: 1.6,
    color: "var(--color-neutral-900)",
    whiteSpace: "pre-wrap",
    wordBreak: "break-word",
    minHeight: 80,
    maxHeight: 360,
    overflow: "auto",
  },
  actions: {
    display: "flex", gap: 8, justifyContent: "flex-end",
  },
  ghostBtn: {
    padding: "8px 14px",
    background: "transparent",
    color: "var(--color-neutral-700)",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 5,
    fontSize: 13, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  primaryBtn: {
    padding: "8px 16px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 5,
    fontSize: 13, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  errorBox: {
    padding: "10px 14px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4,
    fontFamily: "var(--font-mono)", fontSize: 12, lineHeight: 1.5,
  },
  footer: {
    padding: "10px 18px",
    borderTop: "1px solid var(--color-neutral-200)",
    fontSize: 11,
    color: "var(--color-neutral-500)",
  },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
};
