/**
 * MZ-08 — Quick-action bar (Cmd/Ctrl+K).
 *
 * Floating panel anchored at the top of the editor.  Shows three preset
 * buttons (Sharpen, Continue, Rephrase), live-streamed model output, and
 * Accept / Reject / Regenerate controls.  Accepting calls `aiApply`, which
 * takes a `pre_ai` snapshot before mutating the scene.
 *
 * The component is "uncontrolled" with respect to which scene is active —
 * the parent passes `nodeId` and `getScopeText()` (a callback that reads
 * the editor's current selection or paragraph).
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AiSuggestDoneEvent, AiSuggestTokenEvent } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { wordDiff } from "../lib/wordDiff";

export type QuickActionPreset =
  | "sharpen" | "continue" | "rephrase" | "final_polish"
  | "shorten" | "expand";

interface Props {
  open:    boolean;
  nodeId:  string;
  /** Pulled fresh on every preset click so it reflects the live selection. */
  getScopeText: () => string;
  onClose: () => void;
  /** Called after a successful Accept so the parent can refresh the editor. */
  onApplied?: (newText: string, op: "replace" | "append") => void;
}

type Phase = "idle" | "streaming" | "done" | "applying";

export default function QuickActionBar({
  open, nodeId, getScopeText, onClose, onApplied,
}: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [phase,     setPhase]     = useState<Phase>("idle");
  const [preset,    setPreset]    = useState<QuickActionPreset | null>(null);
  const [scope,     setScope]     = useState<string>("");
  const [output,    setOutput]    = useState<string>("");
  const [error,     setError]     = useState<string | null>(null);
  const [aiCallId,  setAiCallId]  = useState<string | null>(null);
  const [jobId,     setJobId]     = useState<string | null>(null);
  const [status,    setStatus]    = useState<string | null>(null);
  const [duration,  setDuration]  = useState<number | null>(null);

  // Keep token-listener cleanup callbacks so we can detach on close / new job.
  const unlistenRef = useRef<(() => void)[]>([]);

  const detachListeners = useCallback(() => {
    unlistenRef.current.forEach((fn) => { try { fn(); } catch {} });
    unlistenRef.current = [];
  }, []);

  // Reset state when bar closes; cancel any in-flight job.
  useEffect(() => {
    if (!open) {
      if (jobId && phase === "streaming") {
        void ipc.aiCancel({ job_id: jobId });
      }
      detachListeners();
      setPhase("idle");
      setPreset(null);
      setScope("");
      setOutput("");
      setError(null);
      setAiCallId(null);
      setJobId(null);
      setStatus(null);
      setDuration(null);
    }
  }, [open, jobId, phase, detachListeners]);

  const startSuggest = useCallback(async (p: QuickActionPreset) => {
    detachListeners();
    setError(null);
    setOutput("");
    setStatus(null);
    setDuration(null);
    setAiCallId(null);
    setPreset(p);
    setPhase("streaming");

    const liveScope = getScopeText().trim();
    if (!liveScope) {
      setError("Place the caret in a paragraph or select some text first.");
      setPhase("idle");
      return;
    }
    setScope(liveScope);

    try {
      const { job_id } = await ipc.aiSuggest({
        node_id:      nodeId,
        preset:       p,
        scope_text:   liveScope,
        model:        null,
        options_json: null,
      });
      setJobId(job_id);

      const unToken = await ipc.onAiSuggestToken(job_id, (e: AiSuggestTokenEvent) => {
        setOutput((prev) => prev + e.delta);
      });
      const unDone = await ipc.onAiSuggestDone(job_id, (e: AiSuggestDoneEvent) => {
        setStatus(e.status);
        setDuration(e.duration_ms);
        setAiCallId(e.ai_call_id || null);
        if (e.full_text) setOutput(e.full_text);
        if (e.error) setError(e.error);
        setPhase("done");
      });
      unlistenRef.current = [unToken, unDone];
    } catch (e) {
      setError(String(e));
      setPhase("idle");
    }
  }, [nodeId, getScopeText, detachListeners]);

  const handleCancel = useCallback(async () => {
    if (jobId) await ipc.aiCancel({ job_id: jobId });
  }, [jobId]);

  const handleAccept = useCallback(async () => {
    if (!aiCallId || !preset) return;
    setPhase("applying");
    setError(null);
    try {
      // Continue is the only preset that appends; everything else replaces
      // the original passage with the polished version.
      const op: "replace" | "append" = preset === "continue" ? "append" : "replace";
      await ipc.aiApply({
        ai_call_id:    aiCallId,
        accepted_text: output,
        op,
      });
      onApplied?.(output, op);
      onClose();
    } catch (e) {
      setError(String(e));
      setPhase("done");
    }
  }, [aiCallId, preset, output, onApplied, onClose]);

  const handleRegenerate = useCallback(() => {
    if (preset) void startSuggest(preset);
  }, [preset, startSuggest]);

  // Esc closes the bar.
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;

  const showOutput = phase === "streaming" || phase === "done" || phase === "applying";

  // K4 — word-level diff between scope (original) and output (suggestion).
  // Only computed when the user toggles into "Diff" view; cheap enough
  // (LCS over ≤ a few thousand tokens) but no need to do it while
  // streaming since output is still growing.
  const [viewMode, setViewMode] = useState<"split" | "diff">("split");
  const diffSegments = useMemo(
    () => (viewMode === "diff" && phase === "done")
        ? wordDiff(scope, output)
        : null,
    [viewMode, phase, scope, output],
  );

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.panel}>
        <header style={s.header}>
          <span id={titleId} style={s.title}>Quick AI</span>
          <span style={s.shortcut}>Esc to close</span>
          <button style={s.closeBtn} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.presetRow}>
          <PresetButton label="Sharpen"   active={preset === "sharpen"}   busy={phase === "streaming"} onClick={() => startSuggest("sharpen")} />
          <PresetButton label="Continue"  active={preset === "continue"}  busy={phase === "streaming"} onClick={() => startSuggest("continue")} />
          <PresetButton label="Rephrase"  active={preset === "rephrase"}  busy={phase === "streaming"} onClick={() => startSuggest("rephrase")} />
          <PresetButton
            label="Shorten"
            active={preset === "shorten"}
            busy={phase === "streaming"}
            onClick={() => startSuggest("shorten")}
            tooltip="Tighten the passage to ≈ half its length"
          />
          <PresetButton
            label="Expand"
            active={preset === "expand"}
            busy={phase === "streaming"}
            onClick={() => startSuggest("expand")}
            tooltip="Flesh out the passage with grounded sensory + interior detail"
          />
          <PresetButton
            label="Final Polish"
            active={preset === "final_polish"}
            busy={phase === "streaming"}
            onClick={() => startSuggest("final_polish")}
            tooltip="World-class editorial pass — runs on Qwen 3.6 (slow, high quality)"
          />
        </div>

        {error && <div style={s.error}>{error}</div>}

        {showOutput && phase === "done" && status === "ok" && (
          <div style={s.viewToggle}>
            <button
              style={{ ...s.toggleBtn, ...(viewMode === "split" ? s.toggleBtnActive : null) }}
              onClick={() => setViewMode("split")}
            >Split</button>
            <button
              style={{ ...s.toggleBtn, ...(viewMode === "diff" ? s.toggleBtnActive : null) }}
              onClick={() => setViewMode("diff")}
            >Diff</button>
          </div>
        )}

        {showOutput && viewMode === "split" && (
          <div style={s.diffWrap}>
            <div style={s.col}>
              <div style={s.colLabel}>Original</div>
              <pre style={s.colBody}>{scope}</pre>
            </div>
            <div style={s.col}>
              <div style={s.colLabel}>
                {phase === "streaming" ? "Streaming…" : status === "ok" ? "Suggestion" : status ?? "Output"}
                {duration !== null && status === "ok" && (
                  <span style={s.colMeta}> · {(duration / 1000).toFixed(1)}s</span>
                )}
              </div>
              <pre style={s.colBody}>{output || (phase === "streaming" ? "…" : "")}</pre>
            </div>
          </div>
        )}

        {showOutput && viewMode === "diff" && diffSegments && (
          <div style={s.diffSingle}>
            <div style={s.colLabel}>Word-level diff</div>
            <pre style={s.diffBody}>
              {diffSegments.map((seg, i) => (
                <span
                  key={i}
                  style={
                    seg.op === "remove" ? s.diffRemove :
                    seg.op === "add"    ? s.diffAdd :
                    s.diffEqual
                  }
                >
                  {seg.text}
                </span>
              ))}
            </pre>
          </div>
        )}

        <footer style={s.footer}>
          {phase === "streaming" && (
            <button style={s.ghostBtn} onClick={handleCancel}>Cancel</button>
          )}
          {phase === "done" && (
            <>
              <button style={s.ghostBtn} onClick={onClose}>Reject</button>
              <button style={s.ghostBtn} onClick={handleRegenerate}>Regenerate</button>
              <button
                style={s.primaryBtn}
                onClick={handleAccept}
                disabled={status !== "ok" || !output.trim() || !aiCallId}
              >
                Accept
              </button>
            </>
          )}
          {phase === "applying" && (
            <button style={s.primaryBtn} disabled>Applying…</button>
          )}
        </footer>
      </div>
    </div>
  );
}

function PresetButton({
  label, active, busy, onClick, tooltip,
}: { label: string; active: boolean; busy: boolean; onClick: () => void; tooltip?: string }) {
  return (
    <button
      style={{
        ...s.presetBtn,
        ...(active ? s.presetBtnActive : null),
        opacity: busy && !active ? 0.5 : 1,
      }}
      onClick={onClick}
      disabled={busy}
      title={tooltip}
    >
      {label}
    </button>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", top: 56, left: 0, right: 0,
    display: "flex", justifyContent: "center", zIndex: 400, pointerEvents: "none",
  },
  panel: {
    pointerEvents: "auto",
    width: "min(92vw, 880px)",
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)",
    borderRadius: 8,
    boxShadow: "0 14px 40px rgba(0,0,0,0.25)",
    display: "flex", flexDirection: "column", overflow: "hidden",
    maxHeight: "calc(100vh - 120px)",
  },
  header: {
    display: "flex", alignItems: "center", gap: 12,
    padding: "10px 14px", borderBottom: "1px solid var(--color-border)",
  },
  title:    { fontWeight: 600, fontSize: 13, color: "var(--color-text-primary)" },
  shortcut: { flex: 1, fontSize: 11, color: "var(--color-text-tertiary)" },
  closeBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-tertiary)" },
  presetRow:{ display: "flex", gap: 8, padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  presetBtn:{
    background: "var(--color-surface-raised)",
    border: "1px solid var(--color-border)", borderRadius: 5,
    fontSize: 13, padding: "6px 12px", cursor: "pointer",
    color: "var(--color-text-primary)", fontFamily: "var(--font-ui)",
  },
  presetBtnActive: {
    background: "var(--color-amber-600)", borderColor: "var(--color-amber-600)", color: "#fff",
  },
  diffWrap: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, padding: 12, overflow: "hidden", flex: 1, minHeight: 0 },
  col:      { display: "flex", flexDirection: "column", border: "1px solid var(--color-border)", borderRadius: 5, overflow: "hidden", minHeight: 0 },
  colLabel: { fontSize: 11, padding: "4px 8px", color: "var(--color-text-tertiary)", borderBottom: "1px solid var(--color-border)", background: "var(--color-surface-raised)" },
  colMeta:  { color: "var(--color-text-tertiary)" },
  colBody:  {
    flex: 1, margin: 0, padding: 10, overflow: "auto",
    fontFamily: "var(--font-prose)", fontSize: 13, lineHeight: 1.55,
    whiteSpace: "pre-wrap", color: "var(--color-text-primary)",
  },
  viewToggle: {
    display: "flex", gap: 4, padding: "4px 12px",
  },
  toggleBtn: {
    background: "transparent", border: "1px solid var(--color-border)",
    borderRadius: 4, fontSize: 11, padding: "2px 8px",
    cursor: "pointer", color: "var(--color-text-secondary)",
  },
  toggleBtnActive: {
    background: "var(--color-amber-600)", borderColor: "var(--color-amber-600)",
    color: "#fff", fontWeight: 600,
  },
  diffSingle: {
    padding: 12, overflow: "hidden", flex: 1, minHeight: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  diffBody: {
    flex: 1, margin: 0, padding: 10, overflow: "auto",
    fontFamily: "var(--font-prose)", fontSize: 13, lineHeight: 1.55,
    whiteSpace: "pre-wrap", color: "var(--color-text-primary)",
    border: "1px solid var(--color-border)", borderRadius: 5,
    background: "var(--color-surface-raised)",
  },
  diffEqual:  { },
  diffRemove: {
    background: "rgba(239, 68, 68, 0.12)",
    color:      "var(--color-error, #ef4444)",
    textDecoration: "line-through",
  },
  diffAdd:    {
    background: "rgba(34, 197, 94, 0.16)",
    color:      "var(--color-success, #16a34a)",
  },
  footer:   { display: "flex", justifyContent: "flex-end", gap: 8, padding: "10px 14px", borderTop: "1px solid var(--color-border)" },
  primaryBtn:{
    padding: "6px 14px", background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 5, fontSize: 13, fontWeight: 600, cursor: "pointer",
  },
  ghostBtn: {
    padding: "6px 14px", background: "transparent", color: "var(--color-text-secondary)",
    border: "1px solid var(--color-border)", borderRadius: 5, fontSize: 13, cursor: "pointer",
  },
  error: {
    margin: "0 14px 8px", padding: "6px 10px",
    fontSize: 12, color: "var(--color-error, #ef4444)",
    background: "var(--color-surface-raised)",
    border: "1px solid var(--color-error, #ef4444)",
    borderRadius: 4,
  },
};
