/**
 * Scene Drafter (Fiction) panel (BACKLOG §A13 / Phase 1).
 *
 * Fiction-shaped scene drafter. Loads the project's character + world
 * bibles automatically (read from memory at run time on the backend).
 * The user supplies the scene card: goal / conflict / reveal / target
 * words / POV / genre lens.
 *
 * On Apply, routes through the orchestrator's `apply_scene_drafter_fic`
 * — mandatory `pre_agent_edit` snapshot + audit-ledger row.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  RunSceneDrafterFicInput,
} from "@booksforge/shared-types";
import { ipc, type RunFullScenePipelineResult } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";
import {
  firstUnapprovedGate,
  loadWorkflowState,
  GATE_LABELS,
} from "../../lib/workflowGates";

interface Props {
  projectId: string;
  sceneId:   string | null;
  model:     string;
  onClose:   () => void;
  onApplied?: () => void;
}

interface SceneDraftProposal {
  pm_doc: { type: string; content?: unknown[] };
  word_count: number;
  notes: string;
}

function tryParseProposal(json: string | null): SceneDraftProposal | null {
  if (!json) return null;
  try {
    return JSON.parse(json);
  } catch {
    return null;
  }
}

/** Walk the pm_doc and return readable plain text for the preview pane. */
function pmDocToPlainText(doc: unknown): string {
  if (!doc || typeof doc !== "object") return "";
  const out: string[] = [];
  function walk(n: unknown): void {
    if (!n || typeof n !== "object") return;
    const node = n as { type?: string; text?: string; content?: unknown[] };
    if (node.type === "text" && typeof node.text === "string") {
      out.push(node.text);
      return;
    }
    if (Array.isArray(node.content)) {
      node.content.forEach(walk);
    }
    if (node.type === "paragraph" || node.type === "heading") {
      out.push("\n\n");
    }
  }
  walk(doc);
  return out.join("").replace(/\n{3,}/g, "\n\n").trim();
}

export default function SceneDrafterFicPanel({ projectId, sceneId, model, onClose, onApplied }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [sceneGoal,     setSceneGoal]     = useState("");
  const [sceneConflict, setSceneConflict] = useState("");
  const [sceneReveal,   setSceneReveal]   = useState("");
  const [targetWords,   setTargetWords]   = useState(1500);
  const [chapterPov,    setChapterPov]    = useState("third-limited");
  const [genreLens,     setGenreLens]     = useState<"literary_fiction" | "genre_fiction">("literary_fiction");
  const [running,       setRunning]       = useState(false);
  const [applying,      setApplying]      = useState(false);
  const [applied,       setApplied]       = useState(false);
  const [result,        setResult]        = useState<AgentRunResultDto | null>(null);
  const [error,         setError]         = useState<string | null>(null);
  const [pipelineRunning, setPipelineRunning] = useState(false);
  const [pipelineResult,  setPipelineResult]  = useState<RunFullScenePipelineResult | null>(null);

  async function handleRun() {
    if (!sceneId) {
      setError("Open a scene in the editor before drafting.");
      return;
    }
    if (!sceneGoal.trim()) {
      setError("Scene goal is required.");
      return;
    }
    if (!sceneConflict.trim()) {
      setError("Scene conflict is required.");
      return;
    }
    setError(null);
    setRunning(true);
    setResult(null);
    setApplied(false);
    try {
      const input: RunSceneDrafterFicInput = {
        project_id:     projectId,
        node_id:        sceneId,
        scene_goal:     sceneGoal,
        scene_conflict: sceneConflict,
        scene_reveal:   sceneReveal || "(no reveal — quiet scene; carry tension via subtext)",
        target_words:   targetWords,
        chapter_pov:    chapterPov,
        genre_lens:     genreLens,
        model,
      };
      const r = await ipc.agentRunSceneDrafterFic(input);
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  async function handleRunPipeline() {
    if (!sceneId) {
      setError("Open a scene in the editor before running the pipeline.");
      return;
    }
    if (!sceneGoal.trim() || !sceneConflict.trim()) {
      setError("Scene goal and scene conflict are required.");
      return;
    }
    // Phase 9 enforcement: a chained workflow walks past every approval
    // gate, so refuse to launch unless every gate is approved (or gates
    // are disabled in Settings → "advanced mode").
    const blocking = firstUnapprovedGate(loadWorkflowState(projectId));
    if (blocking) {
      setError(
        `Approval gate "${GATE_LABELS[blocking]}" is not yet approved. ` +
        `Open the Workflow guide and approve it (or disable gates in Settings → ` +
        `"advanced mode") before running the full pipeline.`
      );
      return;
    }
    setError(null);
    setPipelineRunning(true);
    setResult(null);
    setPipelineResult(null);
    setApplied(false);
    try {
      const r = await ipc.agentRunFullScenePipeline({
        project_id:     projectId,
        node_id:        sceneId,
        scene_goal:     sceneGoal,
        scene_conflict: sceneConflict,
        scene_reveal:   sceneReveal || "(no reveal — quiet scene; carry tension via subtext)",
        target_words:   targetWords,
        chapter_pov:    chapterPov,
        model,
      });
      setPipelineResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setPipelineRunning(false);
    }
  }

  async function handleApply() {
    if (!result || !sceneId) return;
    setApplying(true);
    setError(null);
    try {
      await ipc.agentApplySceneDrafterFic({
        task_id:  result.task_id,
        scene_id: sceneId,
      });
      setApplied(true);
      onApplied?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setApplying(false);
    }
  }

  const proposal = tryParseProposal(result?.proposal_json ?? null);
  const previewText = proposal ? pmDocToPlainText(proposal.pm_doc) : "";
  const previewWords = previewText.split(/\s+/).filter(Boolean).length;

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Scene Drafter (Fiction)</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Drafts a fiction scene from a scene card, with your character +
            world bibles loaded into context automatically. Run a Character
            Bible and World Bible first so the drafter has them to draw from.
          </p>

          <label style={s.label}>Scene goal</label>
          <textarea
            style={s.textarea}
            value={sceneGoal}
            onChange={e => setSceneGoal(e.target.value)}
            placeholder="What is the protagonist trying to do in this scene?"
            rows={2}
          />

          <label style={s.label}>Scene conflict</label>
          <textarea
            style={s.textarea}
            value={sceneConflict}
            onChange={e => setSceneConflict(e.target.value)}
            placeholder="What stops them or makes it hard?"
            rows={2}
          />

          <label style={s.label}>Scene reveal / turn (optional)</label>
          <textarea
            style={s.textarea}
            value={sceneReveal}
            onChange={e => setSceneReveal(e.target.value)}
            placeholder="What does the reader learn, or how does the situation flip?"
            rows={2}
          />

          <div style={s.row}>
            <label style={s.label}>POV</label>
            <input
              style={s.input}
              value={chapterPov}
              onChange={e => setChapterPov(e.target.value)}
              placeholder="e.g. third-limited"
            />
            <label style={s.label}>Target words</label>
            <input
              type="number"
              style={s.numInput}
              min={200}
              max={6000}
              value={targetWords}
              onChange={e => setTargetWords(parseInt(e.target.value || "1500", 10))}
            />
            <label style={s.label}>Genre lens</label>
            <select
              style={s.select}
              value={genreLens}
              onChange={e => setGenreLens(e.target.value as "literary_fiction" | "genre_fiction")}
            >
              <option value="literary_fiction">literary</option>
              <option value="genre_fiction">genre</option>
            </select>
          </div>

          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <button style={s.runBtn} onClick={handleRun} disabled={running || pipelineRunning}>
              {running ? "Drafting…" : "Draft scene"}
            </button>
            <button
              style={{ ...s.runBtn, background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)" }}
              onClick={handleRunPipeline}
              disabled={running || pipelineRunning}
              title="Draft → critic → 4-stage genre-ordered polish → AI-tells scan, all in one shot. Requires every approval gate approved (Workflow guide)."
            >
              {pipelineRunning ? "Running pipeline…" : "Run full pipeline"}
            </button>
          </div>
        </div>

        {error && <div style={s.error}>{error}</div>}

        {pipelineResult && (
          <div style={s.body}>
            <div style={s.statusLine}>
              <strong>Pipeline finished</strong>
              <span style={{ opacity: 0.7 }}>
                {" "}· {pipelineResult.book_kind} ·
                final tells verdict <strong>{pipelineResult.final_tells_verdict}</strong> ·
                {pipelineResult.total_elapsed_s.toFixed(1)} s total
              </span>
            </div>
            <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 4 }}>
              {pipelineResult.stages.map((stg, i) => {
                const icon = stg.status === "completed" ? "✓"
                          : stg.status === "skipped"   ? "·"
                          : stg.status === "failed"    ? "✗" : "?";
                const color = stg.status === "completed" ? "var(--color-success, #2e7d32)"
                            : stg.status === "failed"    ? "var(--color-error, #c62828)"
                            : "currentColor";
                return (
                  <li key={i} style={{ fontSize: 12, display: "grid", gridTemplateColumns: "20px 1fr 60px", gap: 6, alignItems: "baseline" }}>
                    <span style={{ color, textAlign: "center" }}>{icon}</span>
                    <span><strong>{stg.stage}</strong> — {stg.summary || "(no detail)"}</span>
                    <span style={{ opacity: 0.7, textAlign: "right" }}>{stg.elapsed_s.toFixed(1)}s</span>
                  </li>
                );
              })}
            </ul>
          </div>
        )}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              Status: <strong>{result.status}</strong> <span style={{ opacity: 0.5 }}>· run id <code>{result.task_id}</code></span>
            </div>

            {previewText && (
              <div style={s.previewWrap}>
                <div style={s.previewHead}>
                  <strong>Generated prose ({previewWords.toLocaleString()} words)</strong>
                  {sceneId ? (
                    <button
                      style={s.applyBtn}
                      onClick={handleApply}
                      disabled={applying || applied}
                      title="Snapshot the scene, then write this draft into the editor through the orchestrator with an audit-ledger row."
                    >
                      {applying ? "Applying…" :
                       applied ? "✓ Applied — editor refreshed" :
                       "Apply to scene"}
                    </button>
                  ) : (
                    <span style={s.hint}>Open a scene in the editor to apply.</span>
                  )}
                </div>
                <pre style={s.previewBody}>{previewText}</pre>
                {proposal?.notes && (
                  <details style={s.notes}>
                    <summary>Drafter notes</summary>
                    <p style={s.notesP}>{proposal.notes}</p>
                  </details>
                )}
              </div>
            )}

            {result.proposal_json && (
              <details style={s.proposal}>
                <summary style={s.proposalHead}>Raw proposal JSON</summary>
                <pre style={s.pre}>{prettyJson(result.proposal_json)}</pre>
              </details>
            )}

            {result.error && <div style={s.error}>{result.error}</div>}
          </div>
        )}
      </div>
    </div>
  );
}

function prettyJson(s: string): string {
  try { return JSON.stringify(JSON.parse(s), null, 2); }
  catch { return s; }
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(820px, 94vw)", maxHeight: "92vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "10px 14px", overflowY: "auto", display: "flex", flexDirection: "column", gap: 8 },
  blurb:    { margin: 0, fontSize: 13, opacity: 0.85 },
  row:      { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" },
  label:    { fontSize: 12, fontWeight: 500 },
  hint:     { fontSize: 11, opacity: 0.65 },
  input:    { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  numInput: { width: 90, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  select:   { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  textarea: { padding: "8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", fontFamily: "inherit", resize: "vertical" },
  runBtn:   { alignSelf: "flex-start", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  statusLine: { fontSize: 12, opacity: 0.85 },
  previewWrap:{ border: "1px solid var(--color-border)", borderRadius: 4, padding: 10, background: "var(--color-bg)" },
  previewHead:{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, marginBottom: 8 },
  previewBody:{ margin: 0, whiteSpace: "pre-wrap", fontSize: 13, lineHeight: 1.55, maxHeight: 360, overflowY: "auto", fontFamily: "Georgia, serif" },
  notes:    { marginTop: 8, fontSize: 11 },
  notesP:   { margin: "4px 0 0", fontStyle: "italic" },
  applyBtn: { padding: "5px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontSize: 12, fontWeight: 600 },
  proposal: { border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  proposalHead: { cursor: "pointer", fontSize: 12, fontWeight: 600 },
  pre:      { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  error:    { color: "var(--color-error, #c62828)", padding: "8px 14px", fontSize: 12 },
};
