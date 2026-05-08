/**
 * Humanization agent panel — runs the humanizer against the active scene
 * and lists proposed AI-tell rewrites.  Each Accept dispatches
 * `agent_apply_humanization` (BACKLOG §E0d.6).
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  ApplyCopyeditResult,
  RunHumanizationInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import VerificationReportView from "./VerificationReportView";

interface HumanizationEdit {
  range_from:    number;
  range_to:      number;
  before:        string;
  after:         string;
  triggered_rule: string;
  rationale:     string;
}
interface HumanizationProposals {
  edits: HumanizationEdit[];
}

interface Props {
  projectId: string;
  sceneId:   string | null;
  model:     string;
  onClose:   () => void;
}

export default function HumanizationPanel({ projectId, sceneId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [running,  setRunning]  = useState(false);
  const [result,   setResult]   = useState<AgentRunResultDto | null>(null);
  const [accepted, setAccepted] = useState<Record<number, ApplyCopyeditResult | string>>({});
  const [highConf, setHighConf] = useState(false);
  const [error,    setError]    = useState<string | null>(null);

  const proposals: HumanizationProposals | null = (() => {
    if (!result?.proposal_json) return null;
    try { return JSON.parse(result.proposal_json) as HumanizationProposals; }
    catch { return null; }
  })();

  async function handleRun() {
    if (!sceneId) { setError("Open a scene first."); return; }
    setError(null);
    setAccepted({});
    setRunning(true);
    setResult(null);
    try {
      const input: RunHumanizationInput = {
        project_id: projectId,
        node_id:    sceneId,
        model,
        high_confidence_mode: highConf,
      };
      const r = await ipc.agentRunHumanization(input);
      setResult(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  async function handleAccept(idx: number) {
    if (!result || !sceneId) return;
    setAccepted(prev => ({ ...prev, [idx]: "applying" as unknown as ApplyCopyeditResult }));
    try {
      const r = await ipc.agentApplyHumanization({
        task_id:    result.task_id,
        scene_id:   sceneId,
        edit_index: idx,
      });
      setAccepted(prev => ({ ...prev, [idx]: r }));
    } catch (e) {
      setAccepted(prev => ({ ...prev, [idx]: `error: ${e}` }));
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Humanization</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>
        <div style={s.controls}>
          <label style={s.checkbox}>
            <input type="checkbox" checked={highConf} onChange={e => setHighConf(e.target.checked)} />
            High-confidence mode
          </label>
          <button style={s.runBtn} onClick={handleRun} disabled={running || !sceneId}>
            {running ? "Running…" : "Run on current scene"}
          </button>
        </div>

        {error && <div style={s.error}>{error}</div>}
        {!sceneId && <div style={s.hint}>Open a scene in the editor first.</div>}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              status: <strong>{result.status}</strong> · task: <code>{result.task_id}</code>
            </div>

            {proposals && (
              <ul style={s.editList}>
                {proposals.edits.map((e, i) => {
                  const acc = accepted[i];
                  const applied = acc && typeof acc !== "string";
                  return (
                    <li key={i} style={s.editRow}>
                      <div style={s.editHead}>
                        <span style={s.cat}>{e.triggered_rule}</span>
                        <span style={s.range}>chars {e.range_from}–{e.range_to}</span>
                        <button
                          style={{ ...s.acceptBtn, opacity: applied ? 0.6 : 1 }}
                          disabled={!!acc}
                          onClick={() => handleAccept(i)}
                        >
                          {acc === "applying" ? "Applying…" :
                           applied ? "Accepted" :
                           typeof acc === "string" ? "Failed" : "Accept"}
                        </button>
                      </div>
                      <div style={s.diffRow}>
                        <span style={s.before}>{e.before}</span>
                        <span style={s.arrow}>→</span>
                        <span style={s.after}>{e.after}</span>
                      </div>
                      <div style={s.rationale}>{e.rationale}</div>
                      {typeof acc === "string" && acc !== "applying" && (
                        <div style={s.acceptError}>{acc}</div>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}

            {result.verification && (
              <VerificationReportView report={result.verification} />
            )}

            {result.error && <div style={s.error}>{result.error}</div>}
          </div>
        )}
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(800px, 92vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  controls: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  checkbox: { display: "flex", alignItems: "center", gap: 6, fontSize: 13 },
  runBtn:   { padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  body:     { padding: "10px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 14 },
  statusLine: { fontSize: 12, opacity: 0.85 },
  editList: { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 8 },
  editRow:  { padding: 10, border: "1px solid var(--color-border)", borderRadius: 4, display: "flex", flexDirection: "column", gap: 4 },
  editHead: { display: "flex", alignItems: "center", gap: 8 },
  cat:      { padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 11 },
  range:    { fontSize: 11, opacity: 0.7 },
  acceptBtn:{ marginLeft: "auto", padding: "4px 10px", border: "1px solid var(--color-border)", borderRadius: 3, cursor: "pointer", background: "var(--color-bg)" },
  diffRow:  { display: "flex", gap: 8, alignItems: "baseline", fontSize: 13 },
  before:   { textDecoration: "line-through", opacity: 0.7 },
  arrow:    { opacity: 0.5 },
  after:    { fontWeight: 600 },
  rationale:{ fontSize: 12, fontStyle: "italic", opacity: 0.8 },
  acceptError: { color: "var(--color-error, #c62828)", fontSize: 12 },
  error:    { color: "var(--color-error, #c62828)", padding: "8px 14px" },
  hint:     { fontSize: 12, opacity: 0.7, padding: "8px 14px" },
};
