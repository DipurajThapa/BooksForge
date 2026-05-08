/**
 * Developmental Review panel — chained chapter-level review (BACKLOG §F2).
 *
 * One LLM call (dev_editor on the chapter) + per-scene deterministic
 * continuity-linter passes (free).  Surfaces both halves together so
 * the writer sees structural issues alongside per-scene name/POV/tense
 * drift in a single dialog.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  RunDevelopmentalReviewInput,
  RunDevelopmentalReviewResult,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";

interface Props {
  projectId: string;
  model:     string;
  onClose:   () => void;
}

interface DevelopmentalNote {
  axis:       string;
  severity:   string;
  message:    string;
  suggestion: string | null;
}

interface DevelopmentalNotes {
  chapter_id: string;
  notes:      DevelopmentalNote[];
  summary:    string;
}

interface ContinuityFinding {
  kind:       string;
  severity:   string;
  ambiguous:  boolean;
  evidence:   Array<{ excerpt: string }>;
}

export default function DevelopmentalReviewPanel({ projectId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [chapterId,   setChapterId]   = useState("");
  const [projectPov,  setProjectPov]  = useState("");
  const [running,     setRunning]     = useState(false);
  const [result,      setResult]      = useState<RunDevelopmentalReviewResult | null>(null);
  const [error,       setError]       = useState<string | null>(null);

  const devNotes: DevelopmentalNotes | null = (() => {
    if (!result?.dev_notes_json) return null;
    try { return JSON.parse(result.dev_notes_json) as DevelopmentalNotes; }
    catch { return null; }
  })();

  async function handleRun() {
    if (!chapterId.trim()) {
      setError("Enter a chapter id (the ULID of the chapter node).");
      return;
    }
    setError(null);
    setRunning(true);
    setResult(null);
    try {
      const input: RunDevelopmentalReviewInput = {
        project_id:  projectId,
        chapter_id:  chapterId,
        model,
        project_pov: projectPov || null,
        high_confidence_mode: null,
      };
      const r = await ipc.agentRunDevelopmentalReview(input);
      setResult(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Developmental review</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            One LLM call evaluates the chapter on six structural axes
            (pacing, stakes, character, POV tension, theme, structural
            balance) while a deterministic linter scans every scene for
            name / POV / tense / timeline drift in parallel.
          </p>

          <div style={s.row}>
            <label style={s.label}>Chapter id</label>
            <input
              style={s.input}
              value={chapterId}
              onChange={e => setChapterId(e.target.value)}
              placeholder="ULID of the chapter node"
            />
          </div>
          <div style={s.row}>
            <label style={s.label}>POV (optional)</label>
            <input
              style={s.input}
              value={projectPov}
              onChange={e => setProjectPov(e.target.value)}
              placeholder="e.g. third-limited"
            />
          </div>

          <button style={s.runBtn} onClick={handleRun} disabled={running}>
            {running ? "Running…" : "Run developmental review"}
          </button>

          {error && <div style={s.error}>{error}</div>}

          {result && (
            <div style={s.results}>
              <section>
                <h4 style={s.sectionTitle}>Dev editor</h4>
                <div style={s.statusLine}>
                  status: <strong>{result.dev_status}</strong>
                  {" · task: "}<code>{result.dev_task_id}</code>
                </div>
                {devNotes && (
                  <>
                    <div style={s.summary}>{devNotes.summary}</div>
                    {devNotes.notes.length === 0 ? (
                      <div style={s.empty}>No structural issues raised.</div>
                    ) : (
                      <ul style={s.list}>
                        {devNotes.notes.map((n, i) => (
                          <li key={i} style={s.row2}>
                            <span style={s.tag}>{n.axis}</span>
                            <span style={s.sev}>{n.severity}</span>
                            <span style={s.msg}>{n.message}</span>
                            {n.suggestion && (
                              <span style={s.suggestion}>↳ {n.suggestion}</span>
                            )}
                          </li>
                        ))}
                      </ul>
                    )}
                  </>
                )}
                {!devNotes && result.dev_error && (
                  <div style={s.error}>{result.dev_error}</div>
                )}
              </section>

              <section>
                <h4 style={s.sectionTitle}>
                  Continuity linter ({result.scenes_scanned} scene(s) scanned)
                </h4>
                {result.continuity_passes.length === 0 ? (
                  <div style={s.empty}>No continuity drift detected.</div>
                ) : (
                  result.continuity_passes.map(p => (
                    <div key={p.scene_id} style={s.scenePass}>
                      <div style={s.scenePassHead}>
                        <strong>{p.scene_title}</strong>
                        <span style={s.muted}>
                          ({p.finding_count} finding{p.finding_count === 1 ? "" : "s"})
                        </span>
                      </div>
                      <ContinuityFindings findingsJson={p.findings_json} />
                    </div>
                  ))
                )}
              </section>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ContinuityFindings({ findingsJson }: { findingsJson: string }) {
  let findings: ContinuityFinding[] = [];
  try { findings = JSON.parse(findingsJson) as ContinuityFinding[]; }
  catch { return null; }
  return (
    <ul style={s.list}>
      {findings.map((f, i) => (
        <li key={i} style={s.row2}>
          <span style={s.tag}>{f.kind}</span>
          <span style={s.sev}>{f.severity}</span>
          <span style={s.msg}>
            {f.evidence[0]?.excerpt ?? "(no excerpt)"}
          </span>
          {f.ambiguous && <span style={s.ambig}>ambiguous</span>}
        </li>
      ))}
    </ul>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(820px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "12px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 12 },
  blurb:    { fontSize: 13, opacity: 0.85, margin: 0 },
  row:      { display: "flex", alignItems: "center", gap: 8 },
  label:    { fontSize: 12, minWidth: 110 },
  input:    { flex: 1, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  runBtn:   { alignSelf: "flex-start", padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit", fontWeight: 600 },
  error:    { color: "var(--color-error, #c62828)", padding: 8, fontSize: 13 },
  results:  { display: "flex", flexDirection: "column", gap: 14, marginTop: 4 },
  sectionTitle: { margin: 0, fontSize: 13, fontWeight: 600 },
  statusLine: { fontSize: 12, opacity: 0.75, margin: "4px 0" },
  summary:  { padding: 10, background: "var(--color-bg)", borderRadius: 4, fontSize: 13, marginBottom: 6 },
  empty:    { fontSize: 13, fontStyle: "italic", opacity: 0.7, padding: "6px 0" },
  list:     { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 },
  row2:     { display: "flex", flexWrap: "wrap", alignItems: "baseline", gap: 8, padding: 6, borderBottom: "1px dashed var(--color-border)", fontSize: 12 },
  tag:      { fontSize: 10, padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3, textTransform: "uppercase" },
  sev:      { fontSize: 10, padding: "1px 6px", borderRadius: 3, fontWeight: 600, opacity: 0.8 },
  msg:      { flex: 1 },
  suggestion: { width: "100%", paddingLeft: 24, fontStyle: "italic", opacity: 0.75 },
  ambig:    { fontSize: 10, padding: "1px 5px", border: "1px dashed var(--color-warn, #f9a825)", color: "var(--color-warn, #f9a825)", borderRadius: 3 },
  scenePass: { padding: "6px 8px", border: "1px solid var(--color-border)", borderRadius: 4, marginBottom: 6 },
  scenePassHead: { display: "flex", alignItems: "baseline", gap: 8, marginBottom: 4 },
  muted:    { opacity: 0.7, fontSize: 11 },
};
