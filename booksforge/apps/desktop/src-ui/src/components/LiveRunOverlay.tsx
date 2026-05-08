/**
 * Live agent-run overlay (BACKLOG §E4).
 *
 * Subscribes once at mount to `agent-run-started` / `agent-run-completed`
 * Tauri events.  Maintains a small in-memory map of in-flight runs and
 * shows a floating bottom-right card listing each one with an elapsed
 * time and a Cancel button.
 *
 * Local LLM calls take 10-60+ seconds; without this overlay every agent
 * dispatch looks like a UI freeze.  With it, the user knows the work is
 * happening and can abort if they hit the wrong button or want to try
 * again with different inputs.
 *
 * No persistence — restarting the app loses the in-flight visualisation
 * (and the runs themselves are torn down by the orchestrator dropping).
 */
import React, { useEffect, useState } from "react";
import type {
  AgentRunStartedEvent,
  AgentRunCompletedEvent,
  AgentRunProgressEvent,
} from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";

interface ActiveRun {
  run_id:     string;
  agent_id:   string;
  started_at: string;
  cancelling: boolean;
  tokens?:    number;
  /// Wall-clock ms since dispatch, sourced from the latest progress
  /// event.  Distinct from frontend `Date.now() - started_at` because
  /// the backend timestamp is monotonic on the orchestrator's clock,
  /// which avoids clock-skew drift on long runs.
  elapsed_ms?: number;
}

const AGENT_LABELS: Record<string, string> = {
  "outline-architect":      "Outline Architect",
  "intake":                 "Intake",
  "intake-and-outline":     "Brief → Outline",
  "memory-curator":         "Memory Curator",
  "vocab-dictionary":       "Vocab Dictionary",
  "chapter-drafter":        "Chapter Drafter",
  "dev-editor":             "Developmental Editor",
  "humanization":           "Humanization",
  "continuity":             "Continuity",
  "copyeditor":             "Copyeditor",
  "proposal-validator":     "Proposal Validator (Tier 2)",
  "peer-review":            "Peer Review",
};

export default function LiveRunOverlay() {
  const [runs, setRuns] = useState<Record<string, ActiveRun>>({});
  // Tick every 500ms so the elapsed time updates while a run is alive.
  const [, setTick] = useState(0);

  useEffect(() => {
    let unlistenStarted:   undefined | (() => void);
    let unlistenCompleted: undefined | (() => void);
    let unlistenProgress:  undefined | (() => void);
    let cancelled = false;

    (async () => {
      unlistenStarted = await ipc.onAgentRunStarted((e: AgentRunStartedEvent) => {
        if (cancelled) return;
        setRuns(prev => ({
          ...prev,
          [e.run_id]: {
            run_id:     e.run_id,
            agent_id:   e.agent_id,
            started_at: e.started_at,
            cancelling: false,
          },
        }));
      });
      unlistenCompleted = await ipc.onAgentRunCompleted((e: AgentRunCompletedEvent) => {
        if (cancelled) return;
        setRuns(prev => {
          const next = { ...prev };
          delete next[e.run_id];
          return next;
        });
      });
      unlistenProgress = await ipc.onAgentRunProgress((e: AgentRunProgressEvent) => {
        if (cancelled) return;
        setRuns(prev => {
          const cur = prev[e.run_id];
          if (!cur) return prev;
          return {
            ...prev,
            [e.run_id]: { ...cur, tokens: e.tokens, elapsed_ms: e.elapsed_ms },
          };
        });
      });
    })();

    return () => {
      cancelled = true;
      unlistenStarted?.();
      unlistenCompleted?.();
      unlistenProgress?.();
    };
  }, []);

  // Tick the elapsed-time clock while at least one run is active.
  useEffect(() => {
    if (Object.keys(runs).length === 0) return;
    const id = window.setInterval(() => setTick(t => t + 1), 500);
    return () => window.clearInterval(id);
  }, [runs]);

  async function handleCancel(run_id: string) {
    setRuns(prev => prev[run_id]
      ? { ...prev, [run_id]: { ...prev[run_id], cancelling: true } }
      : prev);
    try {
      await ipc.agentCancel({ run_id });
    } catch {
      /* idempotent — overlay clears on the completed event regardless */
    }
  }

  const list = Object.values(runs);
  if (list.length === 0) return null;

  return (
    <div style={s.root}>
      {list.map(r => (
        <div key={r.run_id} style={s.card}>
          <div style={s.head}>
            <span style={s.dot} />
            <span style={s.title}>{AGENT_LABELS[r.agent_id] ?? r.agent_id}</span>
            <span style={s.elapsed}>{formatElapsed(r.started_at)}</span>
          </div>
          {(r.tokens !== undefined && r.elapsed_ms !== undefined) && (
            <div style={s.tokensLine}>
              {r.tokens.toLocaleString()} tokens
              {r.elapsed_ms > 500 && (
                <> · {((r.tokens * 1000) / r.elapsed_ms).toFixed(1)} t/s</>
              )}
            </div>
          )}
          <div style={s.idLine}>
            <code style={s.code}>{r.run_id.slice(0, 8)}</code>
          </div>
          <button
            style={{ ...s.cancelBtn, opacity: r.cancelling ? 0.6 : 1 }}
            onClick={() => handleCancel(r.run_id)}
            disabled={r.cancelling}
          >
            {r.cancelling ? "Cancelling…" : "Cancel"}
          </button>
        </div>
      ))}
    </div>
  );
}

function formatElapsed(startedIso: string): string {
  try {
    const startedMs = new Date(startedIso).getTime();
    const elapsed = Math.max(0, (Date.now() - startedMs) / 1000);
    if (elapsed < 60) return `${elapsed.toFixed(0)}s`;
    const m = Math.floor(elapsed / 60);
    const s = Math.floor(elapsed % 60);
    return `${m}m ${s.toString().padStart(2, "0")}s`;
  } catch {
    return "";
  }
}

const s: Record<string, React.CSSProperties> = {
  root: {
    position: "fixed",
    bottom: 12,
    right:  12,
    zIndex: 90,
    display: "flex",
    flexDirection: "column",
    gap: 8,
    pointerEvents: "auto",
  },
  card: {
    minWidth: 220,
    padding: "8px 12px",
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)",
    borderRadius: 6,
    boxShadow: "0 4px 12px rgba(0,0,0,0.18)",
    fontSize: 13,
    display: "flex",
    flexDirection: "column",
    gap: 4,
  },
  head:    { display: "flex", alignItems: "center", gap: 8 },
  dot: {
    width: 8, height: 8, borderRadius: 4,
    background: "var(--color-accent, #2e7d32)",
    animation: "bf-pulse 1.4s ease-in-out infinite",
  },
  title:   { fontWeight: 600 },
  elapsed: { marginLeft: "auto", fontSize: 11, opacity: 0.7, fontVariantNumeric: "tabular-nums" },
  idLine:  { fontSize: 11, opacity: 0.6 },
  tokensLine: { fontSize: 12, opacity: 0.85, fontVariantNumeric: "tabular-nums" },
  code:    { fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  cancelBtn: {
    alignSelf: "flex-end",
    padding: "3px 10px",
    fontSize: 12,
    border: "1px solid var(--color-border)",
    borderRadius: 3,
    cursor: "pointer",
    background: "var(--color-bg)",
    color: "inherit",
  },
};
