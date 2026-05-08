/**
 * Vocabulary Dictionary panel — runs the agent against recent edit
 * history and lets the user pick which proposed avoid / prefer / replace
 * rules to promote into the project layer (BACKLOG §E0d.10).
 *
 * Proposals are reviewed individually with checkboxes; the Apply button
 * dispatches `vocab_apply_proposals` with the accepted indices.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  RunVocabDictionaryInput,
  VocabApplyResult,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import VerificationReportView from "./VerificationReportView";

interface VocabAddition {
  term:        string;
  kind:        "prefer" | "avoid" | "replace";
  layer:       string;
  replacement: string | null;
  rationale:   string;
}
interface VocabModification {
  term:      string;
  layer:     string;
  field:     string;
  new_value: unknown;
  rationale: string;
}
interface VocabUpdateProposals {
  additions:    VocabAddition[];
  modifications: VocabModification[];
}

interface Props {
  projectId: string;
  model:     string;
  onClose:   () => void;
}

export default function VocabDictionaryPanel({ projectId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [running,    setRunning]    = useState(false);
  const [result,     setResult]     = useState<AgentRunResultDto | null>(null);
  const [error,      setError]      = useState<string | null>(null);
  const [lookback,   setLookback]   = useState(200);
  const [accAdds,    setAccAdds]    = useState<Set<number>>(new Set());
  const [accMods,    setAccMods]    = useState<Set<number>>(new Set());
  const [applying,   setApplying]   = useState(false);
  const [applyOutcome, setApplyOutcome] = useState<VocabApplyResult | string | null>(null);

  const proposals: VocabUpdateProposals | null = (() => {
    if (!result?.proposal_json) return null;
    try { return JSON.parse(result.proposal_json) as VocabUpdateProposals; }
    catch { return null; }
  })();

  async function handleRun() {
    setError(null);
    setApplyOutcome(null);
    setAccAdds(new Set());
    setAccMods(new Set());
    setRunning(true);
    setResult(null);
    try {
      const input: RunVocabDictionaryInput = {
        project_id: projectId, model, lookback,
      };
      const r = await ipc.agentRunVocabDictionary(input);
      setResult(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  function toggleAdd(i: number) {
    setAccAdds(prev => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i); else next.add(i);
      return next;
    });
  }
  function toggleMod(i: number) {
    setAccMods(prev => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i); else next.add(i);
      return next;
    });
  }
  function selectAll() {
    if (!proposals) return;
    setAccAdds(new Set(proposals.additions.map((_, i) => i)));
    setAccMods(new Set(proposals.modifications.map((_, i) => i)));
  }
  function selectNone() {
    setAccAdds(new Set());
    setAccMods(new Set());
  }

  async function handleApply() {
    if (!result) return;
    setApplying(true);
    setApplyOutcome(null);
    try {
      const r = await ipc.vocabApplyProposals({
        task_id: result.task_id,
        accepted_addition_indices: Array.from(accAdds).sort((a, b) => a - b),
        accepted_modification_indices: Array.from(accMods).sort((a, b) => a - b),
      });
      setApplyOutcome(r);
    } catch (e) {
      setApplyOutcome(String(e));
    } finally {
      setApplying(false);
    }
  }

  const totalAccepted = accAdds.size + accMods.size;

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Vocabulary Dictionary</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.controls}>
          <label style={s.label}>Lookback (recent edits to feed):</label>
          <input
            type="number"
            min={1}
            max={1000}
            value={lookback}
            onChange={e => setLookback(parseInt(e.target.value || "200", 10))}
            style={s.numInput}
          />
          <button style={s.runBtn} onClick={handleRun} disabled={running}>
            {running ? "Running…" : "Generate proposals"}
          </button>
        </div>

        {error && <div style={s.error}>{error}</div>}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              status: <strong>{result.status}</strong> · task: <code>{result.task_id}</code>
            </div>

            {proposals && (proposals.additions.length === 0 && proposals.modifications.length === 0) && (
              <div style={s.empty}>The dictionary agent had no new rules to suggest.</div>
            )}

            {proposals && (proposals.additions.length > 0 || proposals.modifications.length > 0) && (
              <>
                <div style={s.toolbar}>
                  <button style={s.linkBtn} onClick={selectAll}>Select all</button>
                  <button style={s.linkBtn} onClick={selectNone}>Select none</button>
                  <span style={s.counter}>
                    {totalAccepted} of {proposals.additions.length + proposals.modifications.length} selected
                  </span>
                  <button
                    style={{ ...s.applyBtn, opacity: totalAccepted === 0 ? 0.5 : 1 }}
                    onClick={handleApply}
                    disabled={applying || totalAccepted === 0}
                  >
                    {applying ? "Applying…" : "Promote selected to project layer"}
                  </button>
                </div>

                {applyOutcome && typeof applyOutcome !== "string" && (
                  <div style={s.applyOk}>
                    Applied — {applyOutcome.additions_applied} addition(s),{" "}
                    {applyOutcome.modifications_applied} modification(s).
                    {(applyOutcome.additions_skipped > 0 || applyOutcome.modifications_skipped > 0) && (
                      <> Skipped: {applyOutcome.additions_skipped + applyOutcome.modifications_skipped} (already exist or invalid).</>
                    )}
                  </div>
                )}
                {typeof applyOutcome === "string" && (
                  <div style={s.error}>{applyOutcome}</div>
                )}

                {proposals.additions.length > 0 && (
                  <section>
                    <h4 style={s.sectionTitle}>New rules ({proposals.additions.length})</h4>
                    <ul style={s.list}>
                      {proposals.additions.map((a, i) => (
                        <li key={i} style={s.row}>
                          <input
                            type="checkbox"
                            checked={accAdds.has(i)}
                            onChange={() => toggleAdd(i)}
                            style={s.checkbox}
                          />
                          <span style={s.kindTag}>{a.kind}</span>
                          <span style={s.term}>{a.term}</span>
                          {a.replacement && (
                            <>
                              <span style={s.arrow}>→</span>
                              <span style={s.repl}>{a.replacement}</span>
                            </>
                          )}
                          <span style={s.rationale}>{a.rationale}</span>
                        </li>
                      ))}
                    </ul>
                  </section>
                )}

                {proposals.modifications.length > 0 && (
                  <section>
                    <h4 style={s.sectionTitle}>Modifications to existing rules ({proposals.modifications.length})</h4>
                    <ul style={s.list}>
                      {proposals.modifications.map((m, i) => (
                        <li key={i} style={s.row}>
                          <input
                            type="checkbox"
                            checked={accMods.has(i)}
                            onChange={() => toggleMod(i)}
                            style={s.checkbox}
                          />
                          <span style={s.term}>{m.term}</span>
                          <span style={s.kindTag}>{m.field}</span>
                          <span style={s.arrow}>→</span>
                          <code style={s.code}>{JSON.stringify(m.new_value)}</code>
                          <span style={s.rationale}>{m.rationale}</span>
                        </li>
                      ))}
                    </ul>
                  </section>
                )}
              </>
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
  dialog:   { width: "min(840px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  controls: { display: "flex", alignItems: "center", gap: 10, padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  label:    { fontSize: 12 },
  numInput: { width: 90, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  runBtn:   { marginLeft: "auto", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  body:     { padding: "10px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 14 },
  statusLine: { fontSize: 12, opacity: 0.85 },
  empty:    { fontSize: 13, fontStyle: "italic", opacity: 0.7 },
  toolbar:  { display: "flex", alignItems: "center", gap: 12, padding: 8, border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)" },
  linkBtn:  { background: "transparent", border: "none", color: "inherit", cursor: "pointer", textDecoration: "underline", fontSize: 12, padding: 0 },
  counter:  { fontSize: 12, opacity: 0.7 },
  applyBtn: { marginLeft: "auto", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  applyOk:  { padding: 8, borderRadius: 4, background: "var(--color-success-bg, rgba(46,125,50,0.12))", color: "var(--color-success, #2e7d32)", fontSize: 13 },
  sectionTitle: { fontSize: 13, fontWeight: 600, margin: "0 0 6px 0" },
  list:     { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 },
  row:      { display: "flex", alignItems: "baseline", gap: 8, padding: 6, borderBottom: "1px dashed var(--color-border)", fontSize: 13 },
  checkbox: { marginTop: 2 },
  kindTag:  { fontSize: 10, padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3, textTransform: "uppercase" },
  term:     { fontWeight: 600 },
  arrow:    { opacity: 0.5 },
  repl:     { fontStyle: "italic" },
  rationale:{ flex: 1, fontSize: 12, opacity: 0.75 },
  code:     { fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 12 },
  error:    { color: "var(--color-error, #c62828)", padding: "8px 14px" },
};
