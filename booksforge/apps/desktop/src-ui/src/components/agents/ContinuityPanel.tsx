/**
 * Continuity agent panel — runs the deterministic linter + LLM
 * adjudicator and lists each finding with its proposed fix (rename /
 * annotate / none).  Accept dispatches `agent_apply_continuity`
 * (BACKLOG §E0d.7).
 */
import React, { useState } from "react";
import type {
  AgentRunResultDto,
  ApplyContinuityResultDto,
  RunContinuityInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { useDialogA11y } from "../../lib/useDialogA11y";
import VerificationReportView from "./VerificationReportView";

interface ContinuityFix {
  kind:  "rename" | "annotate" | "none";
  from?: string | null;
  to?:   string | null;
  scope: "scene" | "chapter" | "project";
}
interface ContinuityFinding {
  kind:         string;
  severity:     string;
  diagnosis:    string;
  proposed_fix: ContinuityFix;
}
interface ContinuityReport { findings: ContinuityFinding[]; }

interface Props {
  projectId: string;
  sceneId:   string | null;
  model:     string;
  onClose:   () => void;
}

export default function ContinuityPanel({ projectId, sceneId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [running,  setRunning]  = useState(false);
  const [result,   setResult]   = useState<AgentRunResultDto | null>(null);
  const [accepted, setAccepted] = useState<Record<number, ApplyContinuityResultDto | string>>({});
  const [highConf, setHighConf] = useState(false);
  const [error,    setError]    = useState<string | null>(null);
  const [kindFilter, setKindFilter] = useState<string | null>(null);

  const report: ContinuityReport | null = (() => {
    if (!result?.proposal_json) return null;
    try { return JSON.parse(result.proposal_json) as ContinuityReport; }
    catch { return null; }
  })();

  async function handleRun() {
    if (!sceneId) { setError("Open a scene first."); return; }
    setError(null);
    setAccepted({});
    setRunning(true);
    setResult(null);
    try {
      const input: RunContinuityInput = {
        project_id:    projectId,
        node_id:       sceneId,
        model,
        project_pov:   null,
        prior_summary: null,
        high_confidence_mode: highConf,
      };
      const r = await ipc.agentRunContinuity(input);
      setResult(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  async function handleApply(idx: number, kind: string) {
    if (!result) return;
    if (kind === "none") return;
    setAccepted(prev => ({ ...prev, [idx]: "applying" as unknown as ApplyContinuityResultDto }));
    try {
      const r = await ipc.agentApplyContinuity({
        project_id:    projectId,
        task_id:       result.task_id,
        finding_index: idx,
      });
      setAccepted(prev => ({ ...prev, [idx]: r }));
    } catch (e) {
      setAccepted(prev => ({ ...prev, [idx]: `error: ${e}` }));
    }
  }

  function kindCounts(): { name: string; total: number; pending: number }[] {
    if (!report) return [];
    const m = new Map<string, { total: number; pending: number }>();
    report.findings.forEach((f, i) => {
      const cur = m.get(f.kind) ?? { total: 0, pending: 0 };
      cur.total += 1;
      if (!accepted[i] && f.proposed_fix.kind !== "none") cur.pending += 1;
      m.set(f.kind, cur);
    });
    return Array.from(m.entries())
      .map(([name, v]) => ({ name, ...v }))
      .sort((a, b) => a.name.localeCompare(b.name));
  }

  // Apply every actionable finding of `kind` that hasn't been applied.
  // Sequential because each rename mutates many scenes.
  async function handleApplyKind(kind: string) {
    if (!report) return;
    const indices = report.findings
      .map((f, i) => ({ f, i }))
      .filter(({ f, i }) => f.kind === kind && !accepted[i] && f.proposed_fix.kind !== "none")
      .map(({ i }) => i);
    for (const i of indices) {
      const fix = report.findings[i]?.proposed_fix;
      if (!fix) continue;
      // eslint-disable-next-line no-await-in-loop
      await handleApply(i, fix.kind);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Continuity</strong>
          <button style={s.close} onClick={onClose} aria-label="Close continuity panel">✕</button>
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

        {error && <div style={s.error} role="alert">{error}</div>}
        {!sceneId && <div style={s.hint} role="status">Open a scene in the editor first.</div>}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              status: <strong>{result.status}</strong> · task: <code>{result.task_id}</code>
            </div>

            {report && report.findings.length === 0 && (
              <div style={s.empty}>No continuity issues detected.</div>
            )}

            {report && report.findings.length > 0 && kindCounts().length > 0 && (
              <div style={s.kindBar}>
                <button
                  style={{ ...s.kindBtn, fontWeight: kindFilter === null ? 700 : 400 }}
                  onClick={() => setKindFilter(null)}
                >
                  all ({report.findings.length})
                </button>
                {kindCounts().map(k => (
                  <span key={k.name} style={s.kindCell}>
                    <button
                      style={{ ...s.kindBtn, fontWeight: kindFilter === k.name ? 700 : 400 }}
                      onClick={() => setKindFilter(k.name)}
                      title={`${k.pending} actionable / ${k.total} total`}
                    >
                      {k.name} ({k.total})
                    </button>
                    {k.pending > 0 && (
                      <button
                        style={s.applyKindBtn}
                        onClick={() => handleApplyKind(k.name)}
                        title={`Apply all ${k.pending} actionable ${k.name} findings`}
                      >
                        apply {k.pending}
                      </button>
                    )}
                  </span>
                ))}
              </div>
            )}

            {report && report.findings.length > 0 && (
              <ul style={s.list}>
                {report.findings.map((f, i) => {
                  if (kindFilter !== null && f.kind !== kindFilter) return null;
                  const acc     = accepted[i];
                  const applied = acc && typeof acc !== "string";
                  const fix     = f.proposed_fix;
                  return (
                    <li key={i} style={s.row}>
                      <div style={s.rowHead}>
                        <span style={s.kind}>{f.kind}</span>
                        <span style={s.sev}>{f.severity}</span>
                        <span style={s.fixKind}>{fix.kind} · {fix.scope}</span>
                        <button
                          style={{ ...s.acceptBtn, opacity: applied ? 0.6 : 1 }}
                          disabled={!!acc || fix.kind === "none"}
                          onClick={() => handleApply(i, fix.kind)}
                          title={fix.kind === "none" ? "Acknowledge-only finding — nothing to apply" : ""}
                        >
                          {acc === "applying" ? "Applying…" :
                           applied ? "Applied" :
                           typeof acc === "string" ? "Failed" :
                           fix.kind === "none" ? "—" : "Apply"}
                        </button>
                      </div>
                      <div style={s.diag}>{f.diagnosis}</div>
                      {fix.kind === "rename" && fix.from && fix.to && (
                        <div style={s.diffRow}>
                          <span style={s.before}>{fix.from}</span>
                          <span style={s.arrow}>→</span>
                          <span style={s.after}>{fix.to}</span>
                        </div>
                      )}
                      {applied && typeof acc !== "string" && acc.kind === "rename" && (
                        <div style={s.applyMeta}>
                          rewrote {acc.scenes_touched} scene(s); pre-snapshot <code>{acc.pre_snapshot_id}</code>
                        </div>
                      )}
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
  empty:    { fontSize: 13, fontStyle: "italic", opacity: 0.7 },
  list:     { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 8 },
  row:      { padding: 10, border: "1px solid var(--color-border)", borderRadius: 4, display: "flex", flexDirection: "column", gap: 4 },
  rowHead:  { display: "flex", alignItems: "center", gap: 8 },
  kind:     { padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 11 },
  sev:      { fontSize: 11, opacity: 0.7 },
  fixKind:  { fontSize: 11, padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3 },
  acceptBtn:{ marginLeft: "auto", padding: "4px 10px", border: "1px solid var(--color-border)", borderRadius: 3, cursor: "pointer", background: "var(--color-bg)" },
  diag:     { fontSize: 13 },
  diffRow:  { display: "flex", gap: 8, alignItems: "baseline", fontSize: 13 },
  before:   { textDecoration: "line-through", opacity: 0.7 },
  arrow:    { opacity: 0.5 },
  after:    { fontWeight: 600 },
  applyMeta:{ fontSize: 12, opacity: 0.7 },
  acceptError: { color: "var(--color-error, #c62828)", fontSize: 12 },
  error:    { color: "var(--color-error, #c62828)", padding: "8px 14px" },
  hint:     { fontSize: 12, opacity: 0.7, padding: "8px 14px" },
  kindBar:  { display: "flex", flexWrap: "wrap", gap: 8, alignItems: "baseline" },
  kindCell: { display: "inline-flex", alignItems: "baseline", gap: 4 },
  kindBtn:  { padding: "3px 8px", border: "1px solid var(--color-border)", borderRadius: 3, cursor: "pointer", background: "var(--color-bg)", fontSize: 11, textTransform: "uppercase" },
  applyKindBtn: { padding: "3px 6px", border: "1px solid var(--color-border)", borderRadius: 3, cursor: "pointer", background: "var(--color-bg)", fontSize: 10 },
};
