/**
 * Workflow guide panel — Phase 9 of `PRODUCT_ROADMAP_E2E.md`
 * (closes UX recommendation R6 from the audit).
 *
 * Walks the writer through the four explicit approval gates in the
 * manuscript factory: Topic → Plan → Bibles → Pre-final-polish.
 *
 * Each row shows:
 *   - Stage label + plain-English blurb.
 *   - Which agent feeds into it.
 *   - Current status: unset (greyed) / pending (highlighted) / approved.
 *   - "Mark pending" / "Approve" / "Reset" controls.
 *
 * The state is stored per project via `lib/workflowGates.ts`.  When
 * approval gates are disabled in Settings (advanced mode), this panel
 * still renders but every gate auto-approves.
 */
import React, { useEffect, useMemo, useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import {
  GATE_BLURBS,
  GATE_LABELS,
  GATE_PRECEDED_BY,
  type GateId,
  type GateStatus,
  type WorkflowState,
  loadWorkflowState,
  resetWorkflowState,
  setGate,
  gatesEnabled,
  setGatesEnabled,
  nextPendingGate,
} from "../lib/workflowGates";
import Term from "./Term";

interface Props {
  projectId: string;
  onClose:   () => void;
}

const ORDER: GateId[] = ["topic", "plan", "bibles", "pre_final_polish"];

export default function WorkflowGuide({ projectId, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [state,    setState]    = useState<WorkflowState>(() => loadWorkflowState(projectId));
  const [enabled,  setEnabled]  = useState<boolean>(() => gatesEnabled());

  // Re-load if the user switches projects while the panel is open.
  useEffect(() => {
    setState(loadWorkflowState(projectId));
  }, [projectId]);

  function transition(gate: GateId, status: GateStatus, note?: string) {
    const next = setGate(projectId, gate, status, note);
    setState({ ...next });
  }

  function handleReset() {
    if (!window.confirm("Reset all four approval gates back to 'unset'? This won't touch any of your prose.")) return;
    setState(resetWorkflowState(projectId));
  }

  function handleToggleEnabled(checked: boolean) {
    setGatesEnabled(checked);
    setEnabled(checked);
  }

  const blocking = useMemo(() => nextPendingGate(state), [state]);

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Workflow guide</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            BooksForge ships with four <Term k="approval_gate">approval gates</Term> between
            the major agent stages so you can course-correct early — before any drift
            propagates into chapters of prose. Approve each gate as the prior agent
            finishes; reject and re-run if the output is off.
          </p>

          <label style={s.toggleRow}>
            <input
              type="checkbox"
              checked={enabled}
              onChange={e => handleToggleEnabled(e.target.checked)}
            />
            <span>
              <strong>Use approval gates</strong>
              <span style={s.toggleHint}>
                {" "}— turn off for "advanced mode" (every gate auto-approves; no pauses).
              </span>
            </span>
          </label>

          {blocking && enabled && (
            <div style={s.blockingBanner}>
              <strong>Pending:</strong> {GATE_LABELS[blocking]} is waiting on your
              review before the workflow can advance.
            </div>
          )}

          <ol style={s.list}>
            {ORDER.map((gate, idx) => (
              <GateRow
                key={gate}
                index={idx + 1}
                gate={gate}
                state={state[gate]}
                disabled={!enabled}
                onMarkPending={() => transition(gate, "pending")}
                onApprove={(note) => transition(gate, "approved", note)}
                onReset={() => transition(gate, "unset")}
              />
            ))}
          </ol>

          <div style={s.footer}>
            <button style={s.ghostBtn} onClick={handleReset}>Reset all gates</button>
            <button style={s.primaryBtn} onClick={onClose}>Done</button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Sub-component ───────────────────────────────────────────────────────────

function GateRow({
  index, gate, state, disabled, onMarkPending, onApprove, onReset,
}: {
  index:         number;
  gate:          GateId;
  state:         { status: GateStatus; changed_at: string; note?: string };
  disabled:      boolean;
  onMarkPending: () => void;
  onApprove:     (note?: string) => void;
  onReset:       () => void;
}) {
  const [note, setNote] = useState<string>(state.note ?? "");
  const statusColor: Record<GateStatus, string> = {
    unset:    "var(--color-border)",
    pending:  "var(--color-amber-500, #f59e0b)",
    approved: "var(--color-success, #2e7d32)",
  };
  const statusLabel: Record<GateStatus, string> = {
    unset:    "Not started",
    pending:  "Awaiting your review",
    approved: "Approved",
  };
  const dim = disabled ? { opacity: 0.55 } : {};

  return (
    <li style={{ ...s.gateRow, borderColor: statusColor[state.status], ...dim }}>
      <div style={s.gateHead}>
        <span style={s.gateIndex}>Gate {index}</span>
        <strong style={s.gateLabel}>{GATE_LABELS[gate]}</strong>
        <span style={{ ...s.statusPill, background: statusColor[state.status] }}>
          {statusLabel[state.status]}
        </span>
      </div>
      <div style={s.gateBody}>
        <div style={s.gateBlurb}>{GATE_BLURBS[gate]}</div>
        <div style={s.gateMeta}>
          Comes after: <em>{GATE_PRECEDED_BY[gate]}</em>
          {state.changed_at && (
            <span style={{ marginLeft: 8, opacity: 0.7 }}>
              · last update {new Date(state.changed_at).toLocaleString()}
            </span>
          )}
        </div>
        {state.status === "pending" && (
          <div style={s.noteRow}>
            <input
              style={s.noteInput}
              placeholder="Optional note: what did you change before approving?"
              value={note}
              onChange={e => setNote(e.target.value)}
            />
          </div>
        )}
        {state.status === "approved" && state.note && (
          <div style={s.savedNote}>Note: {state.note}</div>
        )}
        <div style={s.gateActions}>
          {state.status !== "pending" && (
            <button style={s.smallBtn} onClick={onMarkPending} disabled={disabled}>
              Mark pending
            </button>
          )}
          {state.status === "pending" && (
            <button
              style={{ ...s.smallBtn, ...s.approveBtn }}
              onClick={() => onApprove(note.trim() || undefined)}
              disabled={disabled}
            >
              Approve &amp; continue
            </button>
          )}
          {state.status !== "unset" && (
            <button style={s.smallBtn} onClick={onReset} disabled={disabled}>
              Reset
            </button>
          )}
        </div>
      </div>
    </li>
  );
}

// ── Styles ──────────────────────────────────────────────────────────────────

const s: Record<string, React.CSSProperties> = {
  overlay:   { position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", zIndex: 200, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:    { width: "min(820px, 96vw)", maxHeight: "92vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden", boxShadow: "0 20px 60px rgba(0,0,0,0.4)" },
  header:    { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 16px", borderBottom: "1px solid var(--color-border)" },
  close:     { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:      { padding: 14, overflowY: "auto", display: "flex", flexDirection: "column", gap: 14 },
  blurb:     { margin: 0, fontSize: 13, opacity: 0.85, lineHeight: 1.5 },
  toggleRow: { display: "flex", alignItems: "flex-start", gap: 8, fontSize: 13, padding: "8px 10px", border: "1px solid var(--color-border)", borderRadius: 6, background: "var(--color-bg)" },
  toggleHint:{ opacity: 0.7 },
  blockingBanner:{ background: "var(--color-amber-bg, rgba(245,158,11,0.08))", border: "1px solid var(--color-amber-500, #f59e0b)", borderRadius: 6, padding: "8px 10px", fontSize: 13 },
  list:      { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 10 },
  gateRow:   { border: "2px solid", borderRadius: 6, padding: 12, display: "flex", flexDirection: "column", gap: 6 },
  gateHead:  { display: "flex", alignItems: "center", gap: 8 },
  gateIndex: { opacity: 0.6, fontSize: 11, fontWeight: 600 },
  gateLabel: { fontSize: 14, flex: 1 },
  statusPill:{ fontSize: 11, padding: "2px 8px", borderRadius: 999, color: "white", fontWeight: 600 },
  gateBody:  { display: "flex", flexDirection: "column", gap: 8 },
  gateBlurb: { fontSize: 12, opacity: 0.85, lineHeight: 1.5 },
  gateMeta:  { fontSize: 11, opacity: 0.75 },
  noteRow:   { },
  noteInput: { width: "100%", padding: "6px 8px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", color: "inherit", fontSize: 12, fontFamily: "inherit" },
  savedNote: { fontSize: 12, opacity: 0.85, fontStyle: "italic" },
  gateActions:{ display: "flex", gap: 6, marginTop: 4 },
  smallBtn:  { padding: "4px 10px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "transparent", color: "inherit", fontSize: 12 },
  approveBtn:{ background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", borderColor: "var(--color-success, #2e7d32)", fontWeight: 600 },
  footer:    { display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 4 },
  ghostBtn:  { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "transparent", color: "inherit" },
  primaryBtn:{ padding: "6px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontWeight: 600 },
};
