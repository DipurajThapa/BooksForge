/**
 * AI-tells inspector (BACKLOG §A16 / Phase 3).
 *
 * Pastes prose, scans for AI-prose fingerprints, shows the per-span hits
 * with severity + category, and surfaces the density verdict
 * (PUBLISHABLE / NEEDS_REVISION / AI_SMELL_HIGH). The revision-prompt
 * fragment is exposed for hand-off to a polish run.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type { TellHit, TellsReport } from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  onClose: () => void;
  /** Optional initial text — useful for "scan this scene" entry points. */
  initialText?: string;
}

export default function TellsInspectorPanel({ onClose, initialText }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [text,           setText]           = useState(initialText ?? "");
  const [report,         setReport]         = useState<TellsReport | null>(null);
  const [hits,           setHits]           = useState<TellHit[]>([]);
  const [revisionPrompt, setRevisionPrompt] = useState<string>("");
  const [busy,           setBusy]           = useState(false);
  const [error,          setError]          = useState<string | null>(null);

  async function handleScan() {
    if (!text.trim()) {
      setError("Paste some prose to scan.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      const r = await ipc.tellsScan({ text });
      setReport(r.report);
      setHits(r.hits);
      setRevisionPrompt(r.revision_prompt);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  const verdictColor = (v: string): React.CSSProperties => {
    if (v === "PUBLISHABLE") return { background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)" };
    if (v === "NEEDS_REVISION") return { background: "#fff3cd", color: "#664d03" };
    return { background: "#f8d7da", color: "#842029" };
  };

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>AI-Tells Inspector — measure AI-prose fingerprint density</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Paste prose; the inspector flags spans that match the AI-prose
            dictionary (delve, tapestry, hedge openers, cliché body
            phrases, em-dash overuse, told-not-shown labels). Verdict
            grades by weighted density: PUBLISHABLE (&lt;6/1000),
            NEEDS_REVISION (6–12/1000), AI_SMELL_HIGH (&gt;12/1000).
          </p>

          <textarea
            style={s.textarea}
            value={text}
            onChange={e => setText(e.target.value)}
            placeholder="Paste prose to scan…"
            rows={8}
          />

          <div style={s.row}>
            <button style={s.scanBtn} onClick={handleScan} disabled={busy}>
              {busy ? "Scanning…" : "Scan for AI-tells"}
            </button>
            {report && (
              <span style={{ ...s.verdictBadge, ...verdictColor(report.verdict) }}>
                {report.verdict} · {report.weighted_density_per_1000.toFixed(1)} weighted/1000
              </span>
            )}
          </div>

          {error && <div style={s.error}>{error}</div>}

          {report && (
            <div style={s.metricsRow}>
              <Metric label="Words" value={report.word_count.toLocaleString()} />
              <Metric label="Tells" value={report.tell_count.toLocaleString()} />
              <Metric label="Sev 3 (glaring)" value={report.by_severity_3.toLocaleString()} />
              <Metric label="Sev 2 (routine)" value={report.by_severity_2.toLocaleString()} />
              <Metric label="Sev 1 (cosmetic)" value={report.by_severity_1.toLocaleString()} />
            </div>
          )}

          {hits.length > 0 && (
            <div style={s.hitsList}>
              <h4 style={s.hitsTitle}>Flagged spans ({hits.length})</h4>
              {hits.slice(0, 50).map((h, i) => (
                <div key={i} style={s.hitRow}>
                  <span style={{ ...s.severityBadge, ...severityStyle(h.severity) }}>
                    sev {h.severity}
                  </span>
                  <span style={s.hitCategory}>{h.category}</span>
                  <code style={s.hitMatched}>{h.matched.length > 60 ? `${h.matched.slice(0, 60)}…` : h.matched}</code>
                  <span style={s.hitWhy}>{h.why}</span>
                  {h.suggested_replacement !== null && (
                    <span style={s.hitSuggestion}>→ {h.suggested_replacement || "(cut)"}</span>
                  )}
                </div>
              ))}
              {hits.length > 50 && (
                <div style={s.moreHits}>+{hits.length - 50} more (truncated for display)</div>
              )}
            </div>
          )}

          {revisionPrompt && (
            <details style={s.details}>
              <summary style={s.detailsHead}>Revision-prompt fragment (paste into a polish run)</summary>
              <pre style={s.pre}>{revisionPrompt}</pre>
            </details>
          )}
        </div>
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div style={s.metric}>
      <div style={s.metricValue}>{value}</div>
      <div style={s.metricLabel}>{label}</div>
    </div>
  );
}

function severityStyle(sev: number): React.CSSProperties {
  if (sev === 3) return { background: "#f8d7da", color: "#842029" };
  if (sev === 2) return { background: "#fff3cd", color: "#664d03" };
  return { background: "#e2e3e5", color: "#41464b" };
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(900px, 96vw)", maxHeight: "94vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "10px 14px", overflowY: "auto", display: "flex", flexDirection: "column", gap: 10 },
  blurb:    { margin: 0, fontSize: 13, opacity: 0.85 },
  textarea: { padding: "8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", fontFamily: "Georgia, serif", resize: "vertical" },
  row:      { display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" },
  scanBtn:  { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", fontWeight: 600 },
  verdictBadge: { padding: "4px 10px", borderRadius: 4, fontSize: 11, fontWeight: 700 },
  error:    { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12 },
  metricsRow:{ display: "flex", gap: 10, flexWrap: "wrap" },
  metric:   { padding: "8px 12px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", textAlign: "center" },
  metricValue: { fontSize: 18, fontWeight: 700 },
  metricLabel: { fontSize: 10, opacity: 0.65, textTransform: "uppercase" },
  hitsList: { display: "flex", flexDirection: "column", gap: 4 },
  hitsTitle: { margin: "4px 0", fontSize: 13 },
  hitRow:   { display: "flex", alignItems: "center", gap: 8, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 11, flexWrap: "wrap" },
  severityBadge: { padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 700 },
  hitCategory: { fontSize: 10, opacity: 0.65, textTransform: "uppercase" },
  hitMatched: { fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 11, padding: "1px 4px", background: "var(--color-bg)", borderRadius: 2 },
  hitWhy:   { opacity: 0.75, flex: 1, minWidth: 200 },
  hitSuggestion: { color: "var(--color-success, #2e7d32)", fontSize: 11 },
  moreHits: { fontSize: 11, opacity: 0.6, fontStyle: "italic", padding: "4px 8px" },
  details:  { fontSize: 12, border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  detailsHead: { cursor: "pointer", fontWeight: 600 },
  pre:      { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
};
