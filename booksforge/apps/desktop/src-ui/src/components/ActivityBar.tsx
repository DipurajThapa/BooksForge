/**
 * ActivityBar — persistent footer status strip (F4).
 *
 * Spec: `outputs/UI_UX_SPEC.md §5.4` — "Always visible. Items: save
 * state, project word count, today's word count, last snapshot age,
 * Ollama status (connected / disconnected / pulling), current
 * default model." The 2026-05 redesign collapsed the editor into a
 * rail-of-stages with no status surface; F4 restores a minimal
 * version of the spec's footer.
 *
 * MVP scope (this PR):
 *   - **AI run state** — listens to `book-pipeline:progress` and
 *     `agent-run-{started,progress,completed}` broadcast events. When
 *     anything is running, surfaces what + elapsed + scene counter
 *     (for the pipeline) or token count (for standalone agents).
 *   - **Idle state** — bar is hidden (no visual noise when nothing
 *     is happening). Could become a permanent "Ready" indicator in
 *     a later iteration if writers want it.
 *
 * Deferred (later F-fix PRs):
 *   - Pipeline-run cancellation: the Rust `agent_cancel` command
 *     accepts a `run_id`, but `agent_run_book_pipeline` doesn't emit
 *     `agent-run-started` (only `book-pipeline:progress`), so the
 *     frontend has no run_id to cancel with. Solving cleanly needs
 *     a Rust-side change (either emit a started event with run_id,
 *     or accept a frontend-issued "abort token"). Out of scope.
 *   - Save state (would need Manuscript view to broadcast to the bar)
 *   - Word counts (today / total)
 *   - Snapshot age + Ollama status
 *
 * Why this is safe: the ActivityBar is a *parallel observer* — it
 * never invokes a mutating IPC and never blocks the in-stage panel's
 * own progress UI. Event subscribers in Tauri are multi-cast; the
 * existing Stage8 listener and this bar's listener both receive each
 * event independently.
 */
import { useEffect, useState } from "react";
import {
  ipc,
  type BookPipelineProgressEvent,
} from "../lib/ipc";
import type {
  AgentRunCompletedEvent,
  AgentRunProgressEvent,
  AgentRunStartedEvent,
} from "@booksforge/shared-types";

interface PipelineState {
  /** "character-bible" | "world-bible" | "scene-drafter-fic" */
  stage:     string;
  status:    string;
  summary:   string;
  current:   number;
  total:     number;
  /** Latest `elapsed_s` reported by the backend for this stage. */
  elapsedS:  number;
  /** Wall-clock the bar locked when the first event for this run
   *  arrived — used for a smooth 1-Hz tick between progress events. */
  firstSeen: number;
}

interface AgentRunState {
  runId:     string;
  agentId:   string;
  startedAt: number;
  tokens:    number;
  elapsedMs: number;
}

export default function ActivityBar() {
  const [pipeline, setPipeline] = useState<PipelineState | null>(null);
  const [agentRun, setAgentRun] = useState<AgentRunState | null>(null);
  // 1 Hz tick so "elapsed" keeps advancing between sparse progress events.
  const [, setTick] = useState(0);

  // Book-pipeline progress.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await ipc.onBookPipelineProgress((e: BookPipelineProgressEvent) => {
        if (cancelled) return;
        if (e.status === "completed" && e.stage === "scene-drafter-fic" && e.current >= e.total) {
          // Pipeline finished — clear the bar after a short delay so
          // the writer notices the final state, then idle.
          setPipeline({
            stage:     e.stage,
            status:    e.status,
            summary:   e.summary,
            current:   e.current,
            total:     e.total,
            elapsedS:  e.elapsed_s,
            firstSeen: Date.now(),
          });
          window.setTimeout(() => {
            if (!cancelled) setPipeline(null);
          }, 4000);
          return;
        }
        setPipeline((prev) => ({
          stage:     e.stage,
          status:    e.status,
          summary:   e.summary,
          current:   e.current,
          total:     e.total,
          elapsedS:  e.elapsed_s,
          firstSeen: prev?.firstSeen ?? Date.now(),
        }));
      });
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Standalone-agent runs (outline-architect, concept-scorer, etc.).
  useEffect(() => {
    let cancelled = false;
    let unStart:    (() => void) | undefined;
    let unProgress: (() => void) | undefined;
    let unComplete: (() => void) | undefined;
    (async () => {
      unStart = await ipc.onAgentRunStarted((e: AgentRunStartedEvent) => {
        if (cancelled) return;
        setAgentRun({
          runId:     e.run_id,
          agentId:   e.agent_id,
          startedAt: Date.now(),
          tokens:    0,
          elapsedMs: 0,
        });
      });
      unProgress = await ipc.onAgentRunProgress((e: AgentRunProgressEvent) => {
        if (cancelled) return;
        setAgentRun((prev) => prev && prev.runId === e.run_id ? {
          ...prev,
          tokens:    e.tokens,
          elapsedMs: e.elapsed_ms,
        } : prev);
      });
      unComplete = await ipc.onAgentRunCompleted((e: AgentRunCompletedEvent) => {
        if (cancelled) return;
        setAgentRun((prev) => prev && prev.runId === e.run_id ? null : prev);
      });
    })();
    return () => {
      cancelled = true;
      unStart?.();
      unProgress?.();
      unComplete?.();
    };
  }, []);

  // 1 Hz tick — only while something is active, so we don't churn on idle.
  useEffect(() => {
    if (!pipeline && !agentRun) return;
    const id = window.setInterval(() => setTick((t) => t + 1), 1000);
    return () => window.clearInterval(id);
  }, [pipeline, agentRun]);

  // Render priority: pipeline takes precedence over standalone agent if
  // both happen to be live (shouldn't normally, but possible if the
  // writer triggers a concept-scorer while drafting is queued).
  if (pipeline) {
    return <PipelineRow state={pipeline} />;
  }
  if (agentRun) {
    return <AgentRunRow state={agentRun} />;
  }
  return null;
}

function PipelineRow({ state }: { state: PipelineState }) {
  const wallElapsed = Date.now() - state.firstSeen;
  const label       = stageLabel(state.stage);
  const sceneCtr    = state.stage === "scene-drafter-fic" && state.total > 0
    ? ` · scene ${state.current} / ${state.total}`
    : "";
  return (
    <footer
      style={{ ...s.bar, ...statusBg(state.status) }}
      role="status"
      aria-live="polite"
    >
      <span style={s.dot} aria-hidden="true" />
      <span style={s.label}>
        <b>{label}</b>{sceneCtr} — {state.summary}
      </span>
      <span style={s.elapsed}>{formatElapsed(wallElapsed)}</span>
    </footer>
  );
}

function AgentRunRow({ state }: { state: AgentRunState }) {
  const wallElapsed = Date.now() - state.startedAt;
  const elapsed = state.elapsedMs > 0 ? state.elapsedMs : wallElapsed;
  const tps = elapsed > 500 && state.tokens > 0
    ? ((state.tokens / (elapsed / 1000)) || 0).toFixed(1)
    : null;
  return (
    <footer
      style={{ ...s.bar, ...s.barRunning }}
      role="status"
      aria-live="polite"
    >
      <span style={s.dot} aria-hidden="true" />
      <span style={s.label}>
        Running <b>{state.agentId}</b>
        {state.tokens > 0 && (
          <> · {state.tokens.toLocaleString()} tokens{tps && ` · ${tps} tok/s`}</>
        )}
      </span>
      <span style={s.elapsed}>{formatElapsed(elapsed)}</span>
    </footer>
  );
}

function stageLabel(stage: string): string {
  switch (stage) {
    case "character-bible":   return "Character bible";
    case "world-bible":       return "World bible";
    case "scene-drafter-fic": return "Drafting";
    default:                  return stage;
  }
}

function statusBg(status: string): React.CSSProperties {
  if (status === "failed")    return s.barFailed;
  if (status === "completed") return s.barCompleted;
  if (status === "skipped")   return s.barSkipped;
  return s.barRunning;
}

function formatElapsed(ms: number): string {
  const t = Math.floor(ms / 1000);
  if (t < 60) return `${t}s`;
  const m  = Math.floor(t / 60);
  const sc = t % 60;
  return `${m}m ${sc.toString().padStart(2, "0")}s`;
}

const s: Record<string, React.CSSProperties> = {
  bar: {
    height: 28,
    flexShrink: 0,
    padding: "0 14px",
    display: "flex", alignItems: "center", gap: 10,
    fontFamily: "var(--font-ui)",
    fontSize: 12,
    color: "var(--color-neutral-800)",
    borderTop: "1px solid var(--color-neutral-200)",
    transition: "background 200ms ease",
  },
  barRunning:   { background: "rgba(245,158,11,0.10)" },
  barCompleted: { background: "rgba(34,197,94,0.10)"  },
  barFailed:    { background: "rgba(220,38,38,0.10)"  },
  barSkipped:   { background: "var(--color-neutral-50)" },
  dot: {
    width: 8, height: 8, borderRadius: "50%",
    background: "currentColor",
    opacity: 0.7,
    flexShrink: 0,
  },
  label: {
    flex: 1,
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
  elapsed: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-neutral-600)",
    fontVariantNumeric: "tabular-nums",
    minWidth: 70,
    textAlign: "right",
  },
};
