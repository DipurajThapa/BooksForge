/**
 * Brief → Outline panel — chained intake + outline-architect workflow
 * (BACKLOG §E1).  One-form, two-call dispatch: free-text idea →
 * `ProjectBrief` → `OutlineProposal`.
 *
 * Surfaces both halves so the writer can see the brief the model
 * extracted before the outline-architect ran.  If either half fails
 * the UI shows the raw output so the user can copy it out and try
 * again rather than losing work to a transient model hallucination.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  RunIntakeAndOutlineInput, RunIntakeAndOutlineResult,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  projectId: string;
  model:     string;
  onClose:   () => void;
}

export default function IntakeAndOutlinePanel({ projectId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [idea,           setIdea]            = useState("");
  const [chapterCount,   setChapterCount]    = useState(12);
  const [genreOverlay,   setGenreOverlay]    = useState("");
  const [preferredMode,  setPreferredMode]   = useState("");
  const [running,        setRunning]         = useState(false);
  const [result,         setResult]          = useState<RunIntakeAndOutlineResult | null>(null);
  const [error,          setError]           = useState<string | null>(null);

  async function handleRun() {
    if (!idea.trim()) {
      setError("Describe your book idea before running.");
      return;
    }
    setError(null);
    setRunning(true);
    setResult(null);
    try {
      const input: RunIntakeAndOutlineInput = {
        project_id:           projectId,
        idea_text:            idea,
        preferred_mode:       preferredMode || null,
        target_chapter_count: chapterCount,
        genre_overlay:        genreOverlay || null,
        model,
      };
      const r = await ipc.agentRunIntakeAndOutline(input);
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Brief → Outline (chained)</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Describe your book in your own words — the intake agent will
            extract a typed brief, then the outline architect will turn
            that brief into a chapter-by-chapter outline.  Two model
            calls, one form.
          </p>

          <textarea
            style={s.textarea}
            value={idea}
            onChange={e => setIdea(e.target.value)}
            placeholder="e.g.  A reluctant pastry chef discovers her grandmother's
notebook is the key to a centuries-old guild war.  Cosy mystery,
small-town setting, ensemble cast."
            rows={5}
          />

          <div style={s.row}>
            <label style={s.label}>Target chapters</label>
            <input
              type="number"
              min={1}
              max={60}
              style={s.numInput}
              value={chapterCount}
              onChange={e => setChapterCount(parseInt(e.target.value || "12", 10))}
            />
            <label style={s.label}>Genre overlay (optional)</label>
            <input
              style={s.input}
              value={genreOverlay}
              onChange={e => setGenreOverlay(e.target.value)}
              placeholder="e.g. cosy mystery, hard sci-fi"
            />
          </div>

          <div style={s.row}>
            <label style={s.label}>Preferred mode (optional)</label>
            <select
              style={s.select}
              value={preferredMode}
              onChange={e => setPreferredMode(e.target.value)}
            >
              <option value="">— let intake decide —</option>
              <option value="fiction">fiction</option>
              <option value="non_fiction">non-fiction</option>
              <option value="academic">academic</option>
            </select>
          </div>

          <button style={s.runBtn} onClick={handleRun} disabled={running}>
            {running ? "Running…" : "Run intake → outline"}
          </button>

          {error && <div style={s.error}>{error}</div>}

          {result && (
            <div style={s.results}>
              <section>
                <h4 style={s.sectionTitle}>1.  Brief (intake)</h4>
                <div style={s.statusLine}>
                  <span style={{ opacity: 0.5 }}>run id <code>{result.intake_task_id}</code></span>
                </div>
                {result.brief_json ? (
                  <details style={s.details}>
                    <summary style={s.summary}>Show brief JSON</summary>
                    <pre style={s.pre}>{prettyJson(result.brief_json)}</pre>
                  </details>
                ) : (
                  <div style={s.error}>{result.intake_error ?? "intake failed"}</div>
                )}
              </section>

              <section>
                <h4 style={s.sectionTitle}>2.  Outline</h4>
                <div style={s.statusLine}>
                  Status: <strong>{result.outline_status}</strong>
                  {result.outline_task_id && (
                    <span style={{ opacity: 0.5 }}> · run id <code>{result.outline_task_id}</code></span>
                  )}
                </div>
                {result.outline_json ? (
                  <details style={s.details} open>
                    <summary style={s.summary}>Show outline JSON</summary>
                    <pre style={s.pre}>{prettyJson(result.outline_json)}</pre>
                  </details>
                ) : (
                  <div style={s.error}>{result.outline_error ?? "outline did not run"}</div>
                )}
              </section>

              {result.outline_status === "completed" && result.outline_task_id && (
                <div style={s.hint}>
                  Outline ready.  To materialise the chapters and scenes
                  into the project tree, dispatch the existing
                  <strong> Apply Outline</strong> command with task id
                  <code> {result.outline_task_id}</code>.
                </div>
              )}
            </div>
          )}
        </div>
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
  dialog:   { width: "min(820px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "12px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 12 },
  blurb:    { fontSize: 13, opacity: 0.85, margin: 0 },
  textarea: { padding: "8px 10px", border: "1px solid var(--color-border)", borderRadius: 4, fontSize: 13, fontFamily: "inherit", resize: "vertical", background: "var(--color-bg)", color: "inherit" },
  row:      { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" },
  label:    { fontSize: 12 },
  input:    { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", flex: 1 },
  numInput: { width: 70, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  select:   { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", flex: 1 },
  runBtn:   { alignSelf: "flex-start", padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit", fontWeight: 600 },
  error:    { color: "var(--color-error, #c62828)", padding: 8, fontSize: 13 },
  results:  { display: "flex", flexDirection: "column", gap: 14, marginTop: 8 },
  sectionTitle: { margin: 0, fontSize: 13, fontWeight: 600 },
  statusLine: { fontSize: 12, opacity: 0.75, margin: "4px 0" },
  details:  { border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  summary:  { cursor: "pointer", fontSize: 12, fontWeight: 600 },
  pre:      { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  hint:     { fontSize: 12, padding: 8, background: "var(--color-success-bg, rgba(46,125,50,0.10))", borderRadius: 4 },
};
