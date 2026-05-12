/**
 * Stage 7 — Outline & Structure (Phase B Step 2).
 *
 * State machine:
 *   idle       → no proposal in hand; show "Generate outline" CTA
 *                (or "Re-run" if scenes already exist)
 *   no_brief   → Stage 1 hasn't been completed; show pointer to Stage 1
 *   generating → outline-architect is running; live spinner + token counter
 *   preview    → proposal returned; show OutlinePreview with Accept / Re-run
 *   applying   → calling agentApplyOutline; brief spinner
 *   applied    → tree exists in DB; show structural summary
 *   error      → display the agent's error + raw output if any
 *
 * Reuses existing IPC commands:
 *   - ipc.projectBriefLoad()
 *   - ipc.nodeList()
 *   - ipc.agentRunOutline()        ← persists brief, emits agent-run-progress
 *   - ipc.agentApplyOutline()      ← merges into existing project root (2026-05-11 fix)
 *   - ipc.onAgentRunStarted / onAgentRunProgress / onAgentRunCompleted
 */
import { useEffect, useRef, useState } from "react";
import type {
  AgentRunCompletedEvent,
  AgentRunProgressEvent,
  AgentRunStartedEvent,
  NodeInfo,
  OpenProjectResult,
  OutlineRunResult,
} from "@booksforge/shared-types";
import OutlinePreview, { type OutlineProposal } from "../components/OutlinePreview";
import {
  AxisBar,
  AXIS_FLOOR,
  COMPOSITE_THRESHOLD,
  type AxisLike,
} from "../components/AxisBar";
import { ScoreSummary, FindingsList, EditsList } from "../components/ScorePanel";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  project:    OpenProjectResult;
  onChanged?: () => void;
}

type StageState =
  | { kind: "idle" }
  | { kind: "no_brief" }
  | { kind: "generating" }
  | { kind: "preview"; proposal: OutlineProposal; taskId: string }
  | { kind: "applying" }
  | { kind: "applied" }
  | { kind: "error"; message: string; rawOutput?: string };

// ── Structure-critic types (Phase C — Stage 4 quality gate) ────────────────
// Mirrors `booksforge_domain::structure_score::*`. Hand-typed because
// the domain crate doesn't derive ts-rs; the payload arrives as JSON
// inside `AgentRunResultDto.proposal_json` and the UI parses it.
interface StructureFindingDto {
  kind:     string;
  message:  string;
  severity: "error" | "warning" | string;
}
interface StructureEditDto {
  target:       string;
  locator?:     string;
  suggestion:   string;
  replacement?: string;
}
interface StructureCriticProposal {
  promise_payoff:      AxisLike;
  flow:                AxisLike;
  reader_satisfaction: AxisLike;
  length_realism:      AxisLike;
  overall_summary?:    string;
  findings:            StructureFindingDto[];
  edits:               StructureEditDto[];
}

type CritState =
  | { kind: "idle" }
  | { kind: "running"; startedAt: number }
  | { kind: "ready";   proposal: StructureCriticProposal }
  | { kind: "error";   message: string };

function scComposite(p: StructureCriticProposal): number {
  return (
    p.promise_payoff.score
    + p.flow.score
    + p.reader_satisfaction.score
    + p.length_realism.score
  ) / 4;
}
function scPasses(p: StructureCriticProposal): boolean {
  const axesPass =
    p.promise_payoff.score      >= AXIS_FLOOR
    && p.flow.score                >= AXIS_FLOOR
    && p.reader_satisfaction.score >= AXIS_FLOOR
    && p.length_realism.score      >= AXIS_FLOOR;
  const compositePass = scComposite(p) >= COMPOSITE_THRESHOLD;
  const noErrors = p.findings.every((f) => f.severity !== "error");
  return axesPass && compositePass && noErrors;
}

export default function Stage7_Outline({ project, onChanged }: Props) {
  const [state,          setState]          = useState<StageState>({ kind: "idle" });
  const [briefLoaded,    setBriefLoaded]    = useState<boolean>(false);
  const [briefJson,      setBriefJson]      = useState<unknown>(null);
  const [nodes,          setNodes]          = useState<NodeInfo[]>([]);
  const [loading,        setLoading]        = useState(true);
  const [targetChapters, setTargetChapters] = useState<number>(12);

  // Live progress while generating.
  const [tokens,    setTokens]    = useState<number>(0);
  const [elapsedMs, setElapsedMs] = useState<number>(0);
  const [cancelling, setCancelling] = useState<boolean>(false);
  const [runId,     setRunId]     = useState<string | null>(null);
  // Phase C — Stage 4 quality gate (scoring the current preview).
  const [critState, setCritState] = useState<CritState>({ kind: "idle" });
  const startedAtRef = useRef<number | null>(null);
  const [softTick,  setSoftTick]  = useState(0);
  void softTick; // tick state forces wall-clock re-renders during generation

  // Initial load: brief + nodes.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [brief, ns] = await Promise.all([
          ipc.projectBriefLoad(),
          ipc.nodeList(),
        ]);
        if (cancelled) return;
        setBriefLoaded(brief.loaded);
        setBriefJson(brief.brief_json);
        setNodes(ns);
        // Default target chapter count: from brief target_word_count
        // (rough heuristic ~3500 words/chapter) or 12.
        if (brief.loaded && brief.brief_json && typeof brief.brief_json === "object") {
          const b = brief.brief_json as { target_word_count?: number };
          if (b.target_word_count && b.target_word_count > 0) {
            setTargetChapters(Math.max(6, Math.min(60, Math.round(b.target_word_count / 3500))));
          }
        }
        // If scenes already exist, the outline has been applied.
        const sceneCount = ns.filter((n) => n.kind === "scene").length;
        const chapterCount = ns.filter((n) => n.kind === "chapter").length;
        if (sceneCount > 0 || chapterCount > 0) {
          setState({ kind: "applied" });
        } else if (!brief.loaded) {
          setState({ kind: "no_brief" });
        }
      } catch (e) {
        if (!cancelled) setState({ kind: "error", message: errorMessage(e) });
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  // Soft wall-clock + agent-run-progress subscription, only while generating.
  useEffect(() => {
    if (state.kind !== "generating") return;
    let cancelled = false;
    let unStarted:   (() => void) | undefined;
    let unProgress:  (() => void) | undefined;
    let unCompleted: (() => void) | undefined;

    const tick = window.setInterval(() => setSoftTick((t) => t + 1), 500);

    (async () => {
      unStarted = await ipc.onAgentRunStarted((e: AgentRunStartedEvent) => {
        if (cancelled || e.agent_id !== "outline-architect") return;
        setRunId(e.run_id);
        startedAtRef.current = Date.now();
      });
      unProgress = await ipc.onAgentRunProgress((e: AgentRunProgressEvent) => {
        if (cancelled) return;
        setTokens(e.tokens);
        setElapsedMs(e.elapsed_ms);
      });
      unCompleted = await ipc.onAgentRunCompleted((e: AgentRunCompletedEvent) => {
        if (cancelled || e.agent_id !== "outline-architect") return;
        setRunId(null);
        setCancelling(false);
      });
    })();

    return () => {
      cancelled = true;
      window.clearInterval(tick);
      unStarted?.();
      unProgress?.();
      unCompleted?.();
    };
  }, [state.kind]);

  const displayElapsedMs = elapsedMs > 0
    ? elapsedMs
    : startedAtRef.current
    ? Date.now() - startedAtRef.current
    : 0;

  async function generateOutline() {
    if (!briefJson) {
      setState({ kind: "no_brief" });
      return;
    }
    setState({ kind: "generating" });
    setTokens(0); setElapsedMs(0); setRunId(null); setCancelling(false);
    startedAtRef.current = Date.now();
    try {
      const result: OutlineRunResult = await ipc.agentRunOutline({
        project_id:           project.project_id,
        brief_json:           JSON.stringify(briefJson),
        target_chapter_count: targetChapters,
        genre_overlay:        null,
        model:                null,  // auto-resolve (Light tier for outline)
      });
      if (result.status === "completed" && result.proposal_json) {
        const proposal = JSON.parse(result.proposal_json) as OutlineProposal;
        setState({ kind: "preview", proposal, taskId: result.task_id });
      } else {
        setState({
          kind: "error",
          message: result.error ?? `Agent returned status: ${result.status}`,
          rawOutput: result.raw_output ?? undefined,
        });
      }
    } catch (e) {
      setState({ kind: "error", message: errorMessage(e) });
    }
  }

  async function cancelGeneration() {
    if (!runId) {
      // No run id yet — just bounce back to idle.
      setState({ kind: "idle" });
      return;
    }
    setCancelling(true);
    try {
      await ipc.agentCancel({ run_id: runId });
    } catch {
      /* idempotent — fall through */
    }
  }

  async function acceptOutline() {
    if (state.kind !== "preview") return;
    const taskId = state.taskId;
    setState({ kind: "applying" });
    try {
      await ipc.agentApplyOutline({
        project_id:    project.project_id,
        task_id:       taskId,
        project_title: project.title,
      });
      // Re-fetch the node tree.
      const ns = await ipc.nodeList();
      setNodes(ns);
      setState({ kind: "applied" });
      onChanged?.();
    } catch (e) {
      setState({ kind: "error", message: errorMessage(e) });
    }
  }

  async function reRun() {
    // Refresh nodes first so the "scenes already exist" warning is accurate.
    const ns = await ipc.nodeList().catch(() => nodes);
    setNodes(ns);
    setCritState({ kind: "idle" });
    setState({ kind: "idle" });
  }

  /**
   * Phase C — Stage 4 quality gate. Scores the current preview's
   * outline against the saved brief. Runs the `structure-critic`
   * agent (Medium tier, ~60-120 s). Only available in `preview`
   * state where the in-memory proposal exists.
   */
  async function scoreOutline() {
    if (state.kind !== "preview") return;
    setCritState({ kind: "running", startedAt: Date.now() });
    try {
      const r = await ipc.agentRunStructureCritic({
        project_id:   project.project_id,
        outline_json: JSON.stringify(state.proposal),
        model:        null,  // auto-resolve to Medium
      });
      if (r.status !== "completed" || !r.proposal_json) {
        setCritState({
          kind: "error",
          message: r.error ?? `Agent returned status: ${r.status}`,
        });
        return;
      }
      const parsed = JSON.parse(r.proposal_json) as Partial<StructureCriticProposal>;
      const proposal: StructureCriticProposal = {
        promise_payoff:      parsed.promise_payoff      ?? { score: 7.0 },
        flow:                parsed.flow                ?? { score: 7.0 },
        reader_satisfaction: parsed.reader_satisfaction ?? { score: 7.0 },
        length_realism:      parsed.length_realism      ?? { score: 7.0 },
        overall_summary:     parsed.overall_summary,
        findings:            parsed.findings ?? [],
        edits:               parsed.edits    ?? [],
      };
      setCritState({ kind: "ready", proposal });
    } catch (e) {
      setCritState({ kind: "error", message: errorMessage(e) });
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────

  if (loading) {
    return (
      <div style={s.root}>
        <div style={s.col}>
          <Header />
          <p style={s.muted}>Loading project…</p>
        </div>
      </div>
    );
  }

  const sceneCount   = nodes.filter((n) => n.kind === "scene").length;
  const chapterCount = nodes.filter((n) => n.kind === "chapter").length;

  return (
    <div style={s.root}>
      <div style={s.col}>
        <Header />

        {state.kind === "no_brief" && (
          <div style={s.bannerWarn}>
            <b>Brief required.</b> The outline architect reads the brief
            (premise, key promises, audience) to know what to outline.
            Open <b>Stage 1 — Book Setup</b> from the rail on the left
            and save the brief first.
          </div>
        )}

        {state.kind === "idle" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>Generate an outline</h2>
              <p style={s.sectionHint}>
                Calls the <code style={s.code}>outline-architect</code> agent
                with your brief. Runs on the <b>Light tier</b>
                {" "}(<code style={s.code}>qwen3.5:9b</code> when installed) — typical
                wall-clock 30–120 s. Auto-rescales per-scene word totals to fit
                your brief's target. After accept, scenes mount under the
                existing project root (no duplicate trees).
              </p>
            </header>
            <div style={s.sectionBody}>
              <label style={s.field}>
                <span style={s.fieldLabel}>Target chapter count</span>
                <input
                  type="number"
                  min={6} max={60}
                  value={targetChapters}
                  onChange={(e) => setTargetChapters(Number(e.target.value))}
                  style={s.input}
                />
                <span style={s.fieldHint}>
                  6 – 60. Default derived from your brief's target word count
                  (~3 500 words per chapter).
                </span>
              </label>

              {sceneCount > 0 && (
                <div style={s.bannerNote}>
                  This project already has <b>{chapterCount} chapters</b> and
                  {" "}<b>{sceneCount} scenes</b>. Generating again adds new
                  chapters alongside the existing ones — they share one
                  project root. Manual cleanup of placeholder chapters is
                  on you for now (Stage 7 won't auto-delete).
                </div>
              )}

              <div style={s.actionsRow}>
                <button
                  style={s.primaryBtn}
                  onClick={generateOutline}
                  disabled={!briefLoaded}
                  title={!briefLoaded ? "Save a brief in Stage 1 first" : undefined}
                >
                  ✨ {sceneCount > 0 ? "Generate again" : "Generate outline"}
                </button>
              </div>
            </div>
          </section>
        )}

        {state.kind === "generating" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>
                Generating outline · <b>{formatElapsed(displayElapsedMs)}</b> elapsed
              </h2>
              <p style={s.sectionHint}>
                {tokens > 0 && elapsedMs > 500 ? (
                  <>
                    <b>{tokens.toLocaleString()}</b> tokens · {" "}
                    <b>{((tokens / (elapsedMs / 1000)) || 0).toFixed(1)}</b> tok/s
                  </>
                ) : (
                  <>Waiting for first token from Ollama…</>
                )}
              </p>
            </header>
            <div style={s.sectionBody}>
              <div style={s.runStatus} role="status" aria-live="polite">
                <span style={s.spinner} aria-hidden="true" />
                <div>
                  <p style={s.muted}>
                    The outline-architect runs as a single LLM call against your
                    brief. Parse failures auto-retry up to 3× with schema
                    relaxation (purpose / rationale / scenes / chapters are all
                    tolerant of missing fields).
                  </p>
                </div>
              </div>
              <div style={s.actionsRow}>
                <button
                  style={s.ghostBtn}
                  onClick={cancelGeneration}
                  disabled={cancelling}
                >
                  {cancelling ? "Cancelling…" : "Cancel"}
                </button>
              </div>
            </div>
          </section>
        )}

        {state.kind === "preview" && (
          <>
            <section style={s.section}>
              <header style={s.sectionHeader}>
                <h2 style={s.sectionTitle}>Review the proposed outline</h2>
                <p style={s.sectionHint}>
                  A pre-edit snapshot is taken automatically before any nodes
                  land, so accepting is fully reversible from the Snapshots
                  panel.
                </p>
              </header>
              <div style={s.sectionBody}>
                <OutlinePreview proposal={state.proposal} />
                <div style={s.actionsRow}>
                  <button style={s.ghostBtn} onClick={reRun}>Re-run</button>
                  <button
                    style={s.ghostBtn}
                    onClick={scoreOutline}
                    disabled={critState.kind === "running"}
                    title="Run the structure-critic agent (~60-120 s on Medium tier)"
                  >
                    {critState.kind === "running" ? "Scoring…" : "✨ Score with AI"}
                  </button>
                  <button style={s.primaryBtn} onClick={acceptOutline}>
                    Accept &amp; apply
                  </button>
                </div>
                <StructureCriticPanel
                  state={critState}
                  onClear={() => setCritState({ kind: "idle" })}
                />
              </div>
            </section>
          </>
        )}

        {state.kind === "applying" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>Applying outline…</h2>
            </header>
            <div style={s.sectionBody}>
              <div style={s.runStatus}>
                <span style={s.spinner} aria-hidden="true" />
                <p style={s.muted}>
                  Snapshotting current state, then inserting the
                  parts → chapters → scenes tree under the existing
                  project root.
                </p>
              </div>
            </div>
          </section>
        )}

        {state.kind === "applied" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>Outline applied</h2>
              <p style={s.sectionHint}>
                <b>{chapterCount} chapters</b> · <b>{sceneCount} scenes</b> in
                the binder. Next: <b>Stage 5 — Characters</b> (build the
                character bible) and <b>Stage 8 — Drafting</b>.
              </p>
            </header>
            <div style={s.sectionBody}>
              <div style={s.bannerOk}>
                ✓ The outline is now the project's structural tree. Snapshots
                preserve the pre-apply state if you change your mind.
              </div>
              <div style={s.actionsRow}>
                <button style={s.ghostBtn} onClick={() => setState({ kind: "idle" })}>
                  Generate another outline
                </button>
              </div>
            </div>
          </section>
        )}

        {state.kind === "error" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={{ ...s.sectionTitle, color: "var(--color-red-700, #b91c1c)" }}>
                Outline generation failed
              </h2>
            </header>
            <div style={s.sectionBody}>
              <div style={s.errorBox}>{state.message}</div>
              {state.rawOutput && (
                <details style={s.details}>
                  <summary style={s.detailsSummary}>Raw model output</summary>
                  <pre style={s.rawOutput}>{state.rawOutput}</pre>
                </details>
              )}
              <div style={s.actionsRow}>
                <button style={s.primaryBtn} onClick={() => setState({ kind: "idle" })}>
                  Try again
                </button>
              </div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

function Header() {
  return (
    <header style={s.header}>
      <p style={s.stageNum}>Stage 4 of 6</p>
      <h1 style={s.title}>Outline &amp; Structure</h1>
      <p style={s.lede}>
        Parts → chapters → scenes. The outline-architect agent reads your
        brief and proposes a structural tree; you review and accept. After
        apply, the tree drives drafting (Stage 5) and every downstream pass.
      </p>
    </header>
  );
}

function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  if (total < 60) return `${total}s`;
  const m = Math.floor(total / 60);
  const s_ = total % 60;
  return `${m}m ${s_.toString().padStart(2, "0")}s`;
}

// ── StructureCriticPanel (Phase C — Stage 4 quality gate) ────────────────

function StructureCriticPanel({
  state, onClear,
}: {
  state:   CritState;
  onClear: () => void;
}) {
  if (state.kind === "idle") {
    return null;
  }
  if (state.kind === "running") {
    return (
      <section style={s.critPanel}>
        <header style={s.critPanelHeader}>
          <span style={s.critPanelTitle}>Scoring outline…</span>
        </header>
        <div style={s.critRunning}>
          <span style={s.critSpinner} aria-hidden="true" />
          <span>
            structure-critic is reading the outline against your brief. ~60-120 s
            on the Medium tier.
          </span>
        </div>
      </section>
    );
  }
  if (state.kind === "error") {
    return (
      <section style={s.critPanel}>
        <header style={s.critPanelHeader}>
          <span style={s.critPanelTitle}>Score failed</span>
        </header>
        <div style={s.critErr}>{state.message}</div>
        <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 8 }}>
          <button style={s.smallBtn} onClick={onClear}>Dismiss</button>
        </div>
      </section>
    );
  }
  // Ready
  const p = state.proposal;
  const composite = scComposite(p);
  const passes = scPasses(p);
  const errors = p.findings.filter((f) => f.severity === "error");
  const axes: Array<[string, AxisLike]> = [
    ["Promise / payoff",     p.promise_payoff],
    ["Flow",                 p.flow],
    ["Reader satisfaction",  p.reader_satisfaction],
    ["Length realism",       p.length_realism],
  ];
  return (
    <section style={s.critPanel}>
      <header style={s.critPanelHeader}>
        <span style={s.critPanelTitle}>
          {passes ? "✓ Outline passes gate" : "Outline needs revision"}
        </span>
        <button style={s.smallBtn} onClick={onClear}>Clear</button>
      </header>
      <ScoreSummary
        composite={composite}
        passing={passes}
        stats={(
          <>
            <div>
              <b>{axes.filter(([_, a]) => a.score >= AXIS_FLOOR).length}</b>{" "}
              / 4 axes ≥ {AXIS_FLOOR}
            </div>
            <div>
              <b>{errors.length}</b> blocking finding{errors.length === 1 ? "" : "s"}
            </div>
            <div>
              <b>{p.edits.length}</b> suggested edit{p.edits.length === 1 ? "" : "s"}
            </div>
          </>
        )}
      />

      <div style={s.axisGrid}>
        {axes.map(([label, axis]) => (
          <AxisBar key={label} label={label} axis={axis} />
        ))}
      </div>

      {p.overall_summary && (
        <div style={s.overallSummary}>{p.overall_summary}</div>
      )}

      <FindingsList
        title="Structural findings"
        findings={p.findings.map((f) => ({
          kind:     f.kind,
          message:  f.message,
          severity: f.severity,
        }))}
      />

      <EditsList
        title="Suggested edits"
        edits={p.edits.map((edit) => ({
          field:       edit.target + (edit.locator ? ` · ${edit.locator}` : ""),
          suggestion:  edit.suggestion,
          replacement: edit.replacement,
        }))}
        footerHint="Edits are advisory — apply them by re-running the outline with a tightened brief, or by hand-editing scenes after Accept & apply."
      />
    </section>
  );
}

// Inject spinner keyframes once (HMR-safe).
if (typeof document !== "undefined" && !document.getElementById("bf-stage7-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-stage7-anim";
  styleEl.textContent = `@keyframes bf-stage7-spin {
    from { transform: rotate(0deg); } to { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}

const s: Record<string, React.CSSProperties> = {
  root: {
    height: "100%", overflow: "auto",
    padding: "32px 24px 48px",
    display: "flex", justifyContent: "center",
    fontFamily: "var(--font-ui)",
  },
  col: { width: "min(820px, 100%)", display: "flex", flexDirection: "column", gap: 16 },
  header: { display: "flex", flexDirection: "column", gap: 4, marginBottom: 8 },
  stageNum: {
    margin: 0, fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.1em",
    color: "var(--color-amber-600)",
  },
  title: {
    margin: 0,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 32, fontWeight: 700, lineHeight: 1.2,
    color: "var(--color-neutral-900)",
  },
  lede: {
    margin: "4px 0 0",
    fontSize: 14, color: "var(--color-neutral-700)", lineHeight: 1.6,
  },
  muted: { color: "var(--color-neutral-500)", margin: 0, fontSize: 13 },
  section: {
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6, overflow: "hidden",
  },
  sectionHeader: {
    padding: "12px 16px",
    background: "var(--color-neutral-50)",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  sectionTitle: {
    margin: 0,
    fontSize: 15, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  sectionHint: { margin: "4px 0 0", fontSize: 12, color: "var(--color-neutral-600)", lineHeight: 1.5 },
  sectionBody: { padding: 16, display: "flex", flexDirection: "column", gap: 12 },
  bannerWarn: {
    padding: "10px 14px",
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    borderRadius: 6,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
  },
  bannerNote: {
    padding: "10px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 12, color: "var(--color-neutral-700)", lineHeight: 1.6,
  },
  bannerOk: {
    padding: "10px 14px",
    background: "rgba(34,197,94,0.06)",
    border: "1px solid rgba(34,197,94,0.25)",
    borderRadius: 6,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
  },
  field: { display: "flex", flexDirection: "column", gap: 4 },
  fieldLabel: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  fieldHint: { fontSize: 11, color: "var(--color-neutral-500)" },
  input: {
    width: 120, boxSizing: "border-box",
    padding: "8px 12px",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 14, outline: "none",
  },
  actionsRow: {
    display: "flex", justifyContent: "flex-end", gap: 12, marginTop: 4,
  },
  primaryBtn: {
    padding: "10px 20px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 5,
    fontSize: 14, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  ghostBtn: {
    padding: "10px 16px",
    background: "transparent", color: "var(--color-neutral-700)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 5,
    fontSize: 13, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  runStatus: {
    display: "flex", alignItems: "flex-start", gap: 12,
    padding: 12,
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
  },
  spinner: {
    width: 18, height: 18, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-neutral-300)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-stage7-spin 0.9s linear infinite",
    marginTop: 2,
  },
  errorBox: {
    padding: "10px 14px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4,
    fontFamily: "var(--font-mono)", fontSize: 12, lineHeight: 1.5,
    whiteSpace: "pre-wrap", wordBreak: "break-word",
  },
  details: {
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4, padding: "4px 12px",
    fontSize: 12,
  },
  detailsSummary: {
    cursor: "pointer", padding: "6px 0",
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-600)",
    textTransform: "uppercase", letterSpacing: "0.06em",
  },
  rawOutput: {
    margin: 0, padding: "8px 0",
    fontSize: 11, fontFamily: "var(--font-mono)",
    whiteSpace: "pre-wrap", wordBreak: "break-word",
    color: "var(--color-neutral-700)",
    maxHeight: 300, overflow: "auto",
  },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
  // ── Structure-critic panel (Phase C — Stage 4) ──────────────────────────
  critPanel: {
    marginTop: 12,
    padding: 12,
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    display: "flex", flexDirection: "column", gap: 10,
  },
  critPanelHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "center",
  },
  critPanelTitle: {
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-700)",
  },
  critRunning: {
    display: "flex", alignItems: "center", gap: 10,
    padding: "10px 12px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-700)",
  },
  critSpinner: {
    width: 14, height: 14, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-neutral-300)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-stage7-spin 0.9s linear infinite",
  },
  critErr: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 12,
  },
  critSummaryRow: {
    display: "flex", gap: 24, alignItems: "center",
    flexWrap: "wrap",
  },
  scoreSummary: {
    display: "flex", alignItems: "baseline", gap: 8,
  },
  scoreBig: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 40, fontWeight: 700, lineHeight: 1,
    fontVariantNumeric: "tabular-nums",
  },
  scoreBigDenom: {
    fontSize: 16, fontWeight: 500,
    color: "var(--color-neutral-500)",
    marginLeft: 4,
  },
  scoreBigLabel: {
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-500)",
  },
  critSummaryStats: {
    display: "flex", flexDirection: "column", gap: 2,
    fontSize: 12, color: "var(--color-neutral-700)",
  },
  axisGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: "8px 16px",
  },
  overallSummary: {
    padding: "10px 14px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
    fontFamily: "var(--font-prose, serif)",
  },
  findingsBlock: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  findingsH: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  findingsList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  findingRow: {
    display: "flex", gap: 10, alignItems: "flex-start",
    padding: "6px 10px",
    borderRadius: 4,
    fontSize: 12, lineHeight: 1.5,
  },
  findingErr: {
    background: "rgba(220,38,38,0.06)",
    border: "1px solid rgba(220,38,38,0.25)",
    color: "var(--color-red-700, #b91c1c)",
  },
  findingWarn: {
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    color: "var(--color-amber-700, #b45309)",
  },
  findingKind: {
    fontFamily: "var(--font-mono)", fontSize: 10,
    fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.04em",
    flexShrink: 0,
  },
  editsBlock: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  editsH: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  editsList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  editRow: {
    display: "flex", justifyContent: "space-between", alignItems: "flex-start",
    gap: 12,
    padding: "8px 12px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  editLeft: {
    display: "flex", flexDirection: "column", gap: 2,
    flex: 1, minWidth: 0,
  },
  editField: {
    fontSize: 10, fontWeight: 700, letterSpacing: "0.06em",
    textTransform: "uppercase",
    color: "var(--color-amber-600)",
  },
  editSuggestion: {
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.5,
  },
  editReplacement: {
    fontSize: 12, color: "var(--color-neutral-600)",
    fontFamily: "var(--font-prose, serif)",
    lineHeight: 1.5,
  },
  editsHint: {
    margin: 0,
    fontSize: 11, color: "var(--color-neutral-500)", lineHeight: 1.5,
    fontStyle: "italic",
  },
  smallBtn: {
    padding: "4px 10px",
    background: "var(--color-amber-50, #fffbeb)",
    color: "var(--color-amber-700, #b45309)",
    border: "1px solid var(--color-amber-300, #fcd34d)",
    borderRadius: 4,
    fontSize: 12, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
    flexShrink: 0,
  },
};
