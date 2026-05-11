/**
 * MZ-05 — Debug form for running the Outline Architect Agent.
 *
 * Takes a ProjectBrief JSON, a chapter count, and a model tag.
 * Calls agent_run_outline and shows the result inline.
 *
 * Visible only while the EditorShell "Debug AI" button is active.
 */
import React, { useState } from "react";
import type { OutlineRunResult, RunOutlineInput } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
interface Props {
  projectId: string;
  onClose: () => void;
}

const DEFAULT_BRIEF = JSON.stringify(
  {
    title_suggestions: ["Untitled"],
    mode: "fiction",
    genre: "fantasy",
    audience: "adult",
    tone: "adventurous",
    target_word_count: 80000,
    premise: "A hero embarks on an unexpected journey.",
    key_promises: ["action", "heart"],
    questions_for_user: [],
  },
  null,
  2
);

export default function AgentDebugForm({ projectId, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [briefJson, setBriefJson]       = useState(DEFAULT_BRIEF);
  const [chapterCount, setChapterCount] = useState(12);
  const [genreOverlay, setGenreOverlay] = useState("");
  const [model, setModel]               = useState("qwen2.5:7b-instruct-q4_K_M");
  const [running, setRunning]           = useState(false);
  const [result, setResult]             = useState<OutlineRunResult | null>(null);
  const [parseError, setParseError]     = useState<string | null>(null);

  function validateBriefJson(): boolean {
    try {
      JSON.parse(briefJson);
      setParseError(null);
      return true;
    } catch (e) {
      setParseError(`Invalid JSON: ${e}`);
      return false;
    }
  }

  async function handleRun() {
    if (!validateBriefJson()) return;
    setRunning(true);
    setResult(null);
    try {
      const input: RunOutlineInput = {
        project_id:          projectId,
        brief_json:          briefJson,
        target_chapter_count: chapterCount,
        genre_overlay:        genreOverlay || null,
        model,
      };
      const r = await ipc.agentRunOutline(input);
      setResult(r);
    } catch (e) {
      setResult({
        run_id:       "error",
        task_id:      "error",
        status:       "error",
        proposal_json: null,
        error:        errorMessage(e),
        raw_output:   null,
      });
    } finally {
      setRunning(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <span id={titleId} style={s.title}>AI Debug — Outline Architect</span>
          <button style={s.closeBtn} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {/* Left pane — inputs */}
          <div style={s.inputs}>
            <label style={s.label}>Model</label>
            <input
              style={s.input}
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="qwen2.5:7b-instruct-q4_K_M"
              disabled={running}
            />

            <label style={s.label}>Target chapter count</label>
            <input
              style={s.input}
              type="number"
              min={6}
              max={60}
              value={chapterCount}
              onChange={(e) => setChapterCount(Number(e.target.value))}
              disabled={running}
            />

            <label style={s.label}>Genre overlay (optional)</label>
            <input
              style={s.input}
              value={genreOverlay}
              onChange={(e) => setGenreOverlay(e.target.value)}
              placeholder="e.g. grimdark, cozy mystery…"
              disabled={running}
            />

            <label style={s.label}>
              ProjectBrief JSON
              {parseError && (
                <span style={s.parseError}> — {parseError}</span>
              )}
            </label>
            <textarea
              style={s.textarea}
              value={briefJson}
              onChange={(e) => setBriefJson(e.target.value)}
              onBlur={validateBriefJson}
              disabled={running}
              spellCheck={false}
            />

            <button
              style={{ ...s.runBtn, opacity: running ? 0.6 : 1 }}
              onClick={handleRun}
              disabled={running}
            >
              {running ? "Running…" : "Run outline agent"}
            </button>
          </div>

          {/* Right pane — result */}
          <div style={s.result}>
            {result === null && !running && (
              <p style={s.hint}>Results appear here after you run the agent.</p>
            )}
            {running && (
              <p style={s.hint}>Agent is running… this may take 30–120 seconds.</p>
            )}
            {result !== null && (
              <>
                <StatusBadge status={result.status} />
                {result.error && (
                  <pre style={s.errorBox}>{result.error}</pre>
                )}
                {result.proposal_json && (
                  <pre style={s.codeBox}>
                    {JSON.stringify(JSON.parse(result.proposal_json), null, 2)}
                  </pre>
                )}
                {!result.proposal_json && result.raw_output && (
                  <>
                    <p style={s.hint}>Raw model output:</p>
                    <pre style={s.codeBox}>{result.raw_output}</pre>
                  </>
                )}
                <p style={s.meta}>run: {result.run_id} · task: {result.task_id}</p>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    completed: "var(--color-success, #22c55e)",
    invalid:   "var(--color-amber-400, #fbbf24)",
    error:     "var(--color-error, #ef4444)",
    cancelled: "var(--color-neutral-400, #9ca3af)",
  };
  const bg = colors[status] ?? "var(--color-neutral-400)";
  return (
    <span style={{ ...s.badge, background: bg }}>
      {status}
    </span>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position:       "fixed",
    inset:          0,
    background:     "rgba(0,0,0,0.55)",
    display:        "flex",
    alignItems:     "flex-start",
    justifyContent: "center",
    zIndex:         500,
    paddingTop:     48,
  },
  dialog: {
    background:   "var(--color-surface)",
    border:       "1px solid var(--color-border)",
    borderRadius: 8,
    width:        "min(96vw, 1100px)",
    maxHeight:    "calc(100vh - 72px)",
    display:      "flex",
    flexDirection:"column",
    overflow:     "hidden",
    boxShadow:    "0 8px 32px rgba(0,0,0,0.25)",
  },
  header: {
    display:        "flex",
    alignItems:     "center",
    justifyContent: "space-between",
    padding:        "12px 16px",
    borderBottom:   "1px solid var(--color-border)",
    flexShrink:     0,
  },
  title: {
    fontWeight: 600,
    fontSize:   14,
    color:      "var(--color-text-primary)",
  },
  closeBtn: {
    background: "none",
    border:     "none",
    cursor:     "pointer",
    fontSize:   16,
    color:      "var(--color-text-secondary)",
    padding:    "0 4px",
  },
  body: {
    display:  "flex",
    flex:     1,
    overflow: "hidden",
    gap:      0,
  },
  inputs: {
    display:        "flex",
    flexDirection:  "column",
    gap:            8,
    padding:        16,
    width:          360,
    flexShrink:     0,
    borderRight:    "1px solid var(--color-border)",
    overflowY:      "auto",
  },
  result: {
    flex:      1,
    padding:   16,
    overflowY: "auto",
  },
  label: {
    fontSize:   12,
    fontWeight: 600,
    color:      "var(--color-text-secondary)",
  },
  parseError: {
    color:      "var(--color-error, #ef4444)",
    fontWeight: 400,
  },
  input: {
    fontFamily:   "var(--font-ui)",
    fontSize:     13,
    padding:      "5px 8px",
    border:       "1px solid var(--color-border)",
    borderRadius: 4,
    background:   "var(--color-surface-raised)",
    color:        "var(--color-text-primary)",
    width:        "100%",
    boxSizing:    "border-box",
  },
  textarea: {
    fontFamily:  "var(--font-mono)",
    fontSize:    11,
    padding:     "5px 8px",
    border:      "1px solid var(--color-border)",
    borderRadius: 4,
    background:  "var(--color-surface-raised)",
    color:       "var(--color-text-primary)",
    resize:      "vertical",
    minHeight:   200,
    flex:        1,
    boxSizing:   "border-box",
  },
  runBtn: {
    background:   "var(--color-amber-600, #d97706)",
    border:       "none",
    borderRadius: 4,
    color:        "#fff",
    fontWeight:   600,
    fontSize:     13,
    padding:      "8px 16px",
    cursor:       "pointer",
    marginTop:    4,
  },
  hint: {
    fontSize: 13,
    color:    "var(--color-text-tertiary)",
    margin:   0,
  },
  badge: {
    display:      "inline-block",
    padding:      "2px 10px",
    borderRadius: 99,
    fontSize:     12,
    fontWeight:   700,
    color:        "#fff",
    marginBottom: 12,
  },
  errorBox: {
    background:   "var(--color-surface-raised)",
    border:       "1px solid var(--color-error, #ef4444)",
    borderRadius: 4,
    padding:      "8px 10px",
    fontSize:     11,
    fontFamily:   "var(--font-mono)",
    overflowX:    "auto",
    whiteSpace:   "pre-wrap",
    color:        "var(--color-error, #ef4444)",
    marginBottom: 8,
  },
  codeBox: {
    background:   "var(--color-surface-raised)",
    border:       "1px solid var(--color-border)",
    borderRadius: 4,
    padding:      "8px 10px",
    fontSize:     11,
    fontFamily:   "var(--font-mono)",
    overflowX:    "auto",
    whiteSpace:   "pre-wrap",
    color:        "var(--color-text-primary)",
  },
  meta: {
    fontSize: 10,
    color:    "var(--color-text-tertiary)",
    margin:   "8px 0 0",
    fontFamily: "var(--font-mono)",
  },
};
