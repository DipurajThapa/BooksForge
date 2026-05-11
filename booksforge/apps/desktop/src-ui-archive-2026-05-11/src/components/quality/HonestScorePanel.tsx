/**
 * Honest Score panel (BACKLOG §A16 / Phase 3).
 *
 * Shows the per-genre rubric weights so the user understands what
 * matters in their genre. Surfaces:
 *   - Stylometric distance vs. the project's Voice Anchor
 *   - AI-tells density verdict
 *   - Per-axis rubric weights (so a low pacing score weighed 1× in
 *     literary doesn't look as bad as a low pacing score weighed 3× in
 *     genre)
 *
 * No fake 9/10 — this surface exists so the user always sees the
 * honest measurements alongside any rubric scores from the agents.
 */
import React, { useEffect, useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  BookKind,
  GenrePack,
  StylometricDistance,
  TellsReport,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  /** Optional manuscript text — when supplied, the panel auto-runs the
   *  full measurement (anchor distance + tells density) on mount. */
  manuscriptText?: string;
  onClose: () => void;
}

const KINDS: BookKind[] = ["literary-fiction", "genre-fiction", "non-fiction"];

export default function HonestScorePanel({ manuscriptText, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [kind,        setKind]        = useState<BookKind>("literary-fiction");
  const [pack,        setPack]        = useState<GenrePack | null>(null);
  const [tellsReport, setTellsReport] = useState<TellsReport | null>(null);
  const [distance,    setDistance]    = useState<StylometricDistance | null>(null);
  const [text,        setText]        = useState(manuscriptText ?? "");
  const [busy,        setBusy]        = useState(false);
  const [error,       setError]       = useState<string | null>(null);
  const [anchorMissing, setAnchorMissing] = useState<boolean>(false);

  // Load pack on kind change.
  useEffect(() => {
    ipc.genrePackGet({ kind })
      .then(setPack)
      .catch(e => setError(errorMessage(e)));
  }, [kind]);

  async function handleScore() {
    if (!text.trim()) {
      setError("Paste manuscript prose to score.");
      return;
    }
    setError(null);
    setBusy(true);
    setAnchorMissing(false);
    try {
      // 1. AI-tells density (always available).
      const tells = await ipc.tellsScan({ text });
      setTellsReport(tells.report);

      // 2. Stylometric distance vs. project anchor — requires anchor set.
      const anchor = await ipc.voiceAnchorGet();
      if (anchor.profile && anchor.constraints_block) {
        const d = await ipc.stylometricDistanceCompute({
          anchor_text: anchor.constraints_block, // close enough for distance
          measured_text: text,
        });
        setDistance(d.distance);
      } else {
        setAnchorMissing(true);
        setDistance(null);
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Honest Score — measured-not-vibes quality view</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Two measurements that don&apos;t lie: stylometric distance from
            your Voice Anchor (0–10, 10 = identical) and AI-tells density
            (PUBLISHABLE / NEEDS_REVISION / AI_SMELL_HIGH). Per-genre
            rubric weights show what the system actually optimises for in
            your book&apos;s vertical.
          </p>

          <div style={s.row}>
            <label style={s.label}>Genre lens</label>
            <select
              style={s.select}
              value={kind}
              onChange={e => setKind(e.target.value as BookKind)}
            >
              {KINDS.map(k => (
                <option key={k} value={k}>{k.replace(/-/g, " ")}</option>
              ))}
            </select>
          </div>

          <textarea
            style={s.textarea}
            value={text}
            onChange={e => setText(e.target.value)}
            placeholder="Paste manuscript prose to score…"
            rows={6}
          />

          <button style={s.scoreBtn} onClick={handleScore} disabled={busy}>
            {busy ? "Scoring…" : "Score honestly"}
          </button>

          {error && <div style={s.error}>{error}</div>}

          {(tellsReport || distance) && (
            <div style={s.metricsRow}>
              {distance && (
                <Metric
                  label="Stylometric distance"
                  value={`${distance.distance_score_out_of_10.toFixed(1)} / 10`}
                  hint="vs. your Voice Anchor (10 = identical)"
                />
              )}
              {tellsReport && (
                <Metric
                  label="AI-tells verdict"
                  value={tellsReport.verdict}
                  hint={`${tellsReport.weighted_density_per_1000.toFixed(1)} weighted/1000`}
                />
              )}
            </div>
          )}

          {anchorMissing && (
            <div style={s.warningBanner}>
              No voice anchor set yet. Stylometric distance is unavailable.
              Open the Voice Anchor panel and paste comp samples to enable
              this measurement.
            </div>
          )}

          {pack && (
            <div style={s.card}>
              <h4 style={s.cardTitle}>Per-axis rubric weights — {pack.kind.replace(/-/g, " ")}</h4>
              <div style={s.weightGrid}>
                {Object.entries(pack.rubric_weights)
                  .map(([axis, w]) => [axis, w ?? 0] as [string, number])
                  .sort(([, a], [, b]) => b - a)
                  .map(([axis, w]) => (
                    <div key={axis} style={s.weightRow}>
                      <span style={s.axisName}>{axis}</span>
                      <div style={s.weightBarContainer}>
                        <div style={{ ...s.weightBar, width: `${(w / 3.0) * 100}%` }} />
                      </div>
                      <span style={s.weightValue}>{w.toFixed(1)}×</span>
                    </div>
                  ))}
              </div>
            </div>
          )}

          {pack && (
            <details style={s.details}>
              <summary style={s.detailsHead}>Hard rules + critic axes for this genre</summary>
              <div style={s.detailsBody}>
                <p style={s.subhead}>Hard rules (non-negotiable):</p>
                <ul style={s.list}>
                  {pack.hard_rules.map((r, i) => <li key={i}>{r}</li>)}
                </ul>
                <p style={s.subhead}>Critic axes (the per-scene critic uses these):</p>
                <ul style={s.list}>
                  {pack.critic_axes.map((a, i) => <li key={i}>{a}</li>)}
                </ul>
                <p style={s.subhead}>Polish stack order:</p>
                <ol style={s.list}>
                  {pack.polish_stack_order.map((s, i) => <li key={i}>{s}</li>)}
                </ol>
              </div>
            </details>
          )}
        </div>
      </div>
    </div>
  );
}

function Metric({ label, value, hint }: { label: string; value: string; hint: string }) {
  return (
    <div style={s.metric}>
      <div style={s.metricValue}>{value}</div>
      <div style={s.metricLabel}>{label}</div>
      <div style={s.metricHint}>{hint}</div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(820px, 94vw)", maxHeight: "92vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "10px 14px", overflowY: "auto", display: "flex", flexDirection: "column", gap: 10 },
  blurb:    { margin: 0, fontSize: 13, opacity: 0.85 },
  row:      { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" },
  label:    { fontSize: 12, fontWeight: 500 },
  select:   { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  textarea: { padding: "8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", fontFamily: "Georgia, serif", resize: "vertical" },
  scoreBtn: { alignSelf: "flex-start", padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", fontWeight: 600 },
  error:    { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12 },
  warningBanner: { padding: 10, background: "#fff3cd", color: "#664d03", borderRadius: 4, fontSize: 12 },
  metricsRow:{ display: "flex", gap: 10, flexWrap: "wrap" },
  metric:   { padding: "10px 14px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", textAlign: "center", minWidth: 180 },
  metricValue: { fontSize: 22, fontWeight: 700 },
  metricLabel: { fontSize: 11, opacity: 0.85, marginTop: 2 },
  metricHint:  { fontSize: 10, opacity: 0.55, marginTop: 2 },
  card:     { border: "1px solid var(--color-border)", borderRadius: 4, padding: 10, background: "var(--color-bg)" },
  cardTitle:{ margin: "0 0 8px", fontSize: 13 },
  weightGrid:{ display: "flex", flexDirection: "column", gap: 4 },
  weightRow:{ display: "grid", gridTemplateColumns: "180px 1fr 50px", alignItems: "center", gap: 8, fontSize: 11 },
  axisName: { opacity: 0.85 },
  weightBarContainer: { background: "var(--color-surface)", borderRadius: 2, height: 10, overflow: "hidden" },
  weightBar:{ background: "var(--color-success, #2e7d32)", height: "100%" },
  weightValue: { textAlign: "right", fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 11 },
  details:  { fontSize: 12, border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  detailsHead: { cursor: "pointer", fontWeight: 600 },
  detailsBody: { padding: "8px 0" },
  subhead:  { margin: "8px 0 4px", fontSize: 12, fontWeight: 600 },
  list:     { margin: "0 0 0 20px", padding: 0, fontSize: 12 },
};
