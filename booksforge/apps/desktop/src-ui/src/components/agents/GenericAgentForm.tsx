/**
 * Generic agent form — drives the 7 non-mutating agents from a single
 * component.  Asks the user for the agent-specific input (free text /
 * scope / id), dispatches the right `agentRun*` IPC, and shows the
 * proposal JSON + verification report.  No Accept buttons — these
 * agents either auto-apply (memory-curator) or are advisory.
 *
 * Per-apply paths for the advising agents are tracked under the
 * follow-up §E0d entries.  This form is the bridge until those land.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  RunChapterDrafterInput,
  RunDevEditorInput,
  RunIntakeInput,
  RunMemoryCuratorInput,
  RunVocabDictionaryInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import VerificationReportView from "./VerificationReportView";

type AgentKey =
  | "outline" | "chapter-drafter" | "dev-editor"
  | "memory-curator" | "vocab-dictionary"
  | "intake" | "intake-and-outline"
  | "developmental-review" | "entity-bible"
  | "proposal-validator" | "peer-review"
  | "copyeditor" | "humanization" | "continuity";

interface Props {
  agentKey:  AgentKey;
  projectId: string;
  sceneId:   string | null;
  model:     string;
  onClose:   () => void;
}

export default function GenericAgentForm({ agentKey, projectId, sceneId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [running, setRunning] = useState(false);
  const [result,  setResult]  = useState<AgentRunResultDto | null>(null);
  const [error,   setError]   = useState<string | null>(null);

  // Per-agent extra inputs.
  const [ideaText,        setIdeaText]        = useState("");
  const [memoryScope,     setMemoryScope]     = useState<"book" | "chapter" | "entity">("chapter");
  const [chapterId,       setChapterId]       = useState("");
  const [scenePov,        setScenePov]        = useState("");
  const [synopsis,        setSynopsis]        = useState("");
  const [chapterPurpose,  setChapterPurpose]  = useState("");
  const [targetWords,     setTargetWords]     = useState(2000);
  const [lookback,        setLookback]        = useState(200);

  async function handleRun() {
    setError(null);
    setRunning(true);
    setResult(null);
    try {
      let r: AgentRunResultDto | { run_id: string; task_id: string; status: string; proposal_json: string | null; error: string | null; raw_output: string | null };
      switch (agentKey) {
        case "intake": {
          const input: RunIntakeInput = {
            project_id: projectId, idea_text: ideaText, preferred_mode: null, model,
          };
          r = await ipc.agentRunIntake(input);
          break;
        }
        case "memory-curator": {
          const input: RunMemoryCuratorInput = {
            project_id: projectId, scope: memoryScope, node_id: sceneId, model,
          };
          r = await ipc.agentRunMemoryCurator(input);
          break;
        }
        case "vocab-dictionary": {
          const input: RunVocabDictionaryInput = {
            project_id: projectId, model, lookback,
          };
          r = await ipc.agentRunVocabDictionary(input);
          break;
        }
        case "chapter-drafter": {
          if (!sceneId) throw new Error("Open a scene first.");
          const input: RunChapterDrafterInput = {
            project_id: projectId, node_id: sceneId,
            scene_synopsis: synopsis, chapter_purpose: chapterPurpose,
            project_pov: scenePov || "third-limited",
            target_words: targetWords, model,
            genre: null, tone: null, high_confidence_mode: null,
          };
          r = await ipc.agentRunChapterDrafter(input);
          break;
        }
        case "dev-editor": {
          if (!chapterId) throw new Error("Enter a chapter id.");
          const input: RunDevEditorInput = {
            project_id: projectId, chapter_id: chapterId, model,
            high_confidence_mode: null,
          };
          r = await ipc.agentRunDevEditor(input);
          break;
        }
        case "outline":
          throw new Error("Use the Outline Architect debug form on the editor toolbar.");
        case "proposal-validator":
        case "peer-review":
          throw new Error(
            "This agent is invoked automatically by the orchestrator alongside another agent's run. " +
            "Look at any other agent's output — its verification report shows this agent's verdict."
          );
        default:
          throw new Error("This agent has its own panel — use the switchboard cards.");
      }
      setResult(r as AgentRunResultDto);
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
          <strong id={titleId}>{titleFor(agentKey)}</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.controls}>
          {agentKey === "intake" && (
            <textarea
              style={s.textArea}
              value={ideaText}
              onChange={e => setIdeaText(e.target.value)}
              placeholder="Describe your book idea in your own words…"
              rows={4}
            />
          )}
          {agentKey === "memory-curator" && (
            <div style={s.row}>
              <label style={s.label}>Scope:</label>
              <select
                style={s.select}
                value={memoryScope}
                onChange={e => setMemoryScope(e.target.value as "book" | "chapter" | "entity")}
              >
                <option value="book">book</option>
                <option value="chapter">chapter</option>
                <option value="entity">entity</option>
              </select>
              <span style={s.hint}>
                {memoryScope === "chapter" && sceneId ? "uses current scene id as node_id" : ""}
              </span>
            </div>
          )}
          {agentKey === "vocab-dictionary" && (
            <div style={s.row}>
              <label style={s.label}>Lookback (recent edits to feed):</label>
              <input
                type="number"
                min={1}
                max={1000}
                value={lookback}
                onChange={e => setLookback(parseInt(e.target.value || "200", 10))}
                style={s.numInput}
              />
            </div>
          )}
          {agentKey === "chapter-drafter" && (
            <>
              <textarea
                style={s.textArea}
                value={synopsis}
                onChange={e => setSynopsis(e.target.value)}
                placeholder="Scene synopsis — 1–3 sentences of what the scene needs to do"
                rows={3}
              />
              <input
                style={s.input}
                value={chapterPurpose}
                onChange={e => setChapterPurpose(e.target.value)}
                placeholder="Chapter purpose (one sentence)"
              />
              <div style={s.row}>
                <label style={s.label}>POV:</label>
                <input
                  style={s.input}
                  value={scenePov}
                  onChange={e => setScenePov(e.target.value)}
                  placeholder="e.g. third-limited"
                />
                <label style={s.label}>Target words:</label>
                <input
                  type="number"
                  style={s.numInput}
                  value={targetWords}
                  onChange={e => setTargetWords(parseInt(e.target.value || "2000", 10))}
                />
              </div>
            </>
          )}
          {agentKey === "dev-editor" && (
            <input
              style={s.input}
              value={chapterId}
              onChange={e => setChapterId(e.target.value)}
              placeholder="Chapter ULID (the chapter node id)"
            />
          )}
          {(agentKey === "proposal-validator" || agentKey === "peer-review") && (
            <div style={s.note}>
              This agent is invoked automatically by the orchestrator alongside another agent's run.
              Look at any other agent's output — its verification report shows this agent's verdict.
            </div>
          )}
          {agentKey === "outline" && (
            <div style={s.note}>
              Use the existing "AI" → Outline Architect debug form on the editor toolbar.
            </div>
          )}

          <button
            style={s.runBtn}
            onClick={handleRun}
            disabled={running || agentKey === "proposal-validator" || agentKey === "peer-review" || agentKey === "outline"}
          >
            {running ? "Running…" : "Run agent"}
          </button>
        </div>

        {error && <div style={s.error}>{error}</div>}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              status: <strong>{result.status}</strong> · task: <code>{result.task_id}</code>
            </div>
            {result.proposal_json && (
              <details style={s.proposal}>
                <summary style={s.proposalHead}>Proposal JSON</summary>
                <pre style={s.pre}>{prettyJson(result.proposal_json)}</pre>
              </details>
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

function titleFor(k: AgentKey): string {
  return ({
    "outline":           "Outline Architect",
    "chapter-drafter":   "Chapter Drafter",
    "dev-editor":        "Developmental Editor",
    "memory-curator":    "Memory Curator",
    "vocab-dictionary":  "Vocabulary Dictionary",
    "intake":            "Intake",
    "proposal-validator":"Proposal Validator (Tier 2)",
    "peer-review":       "Peer Review",
  } as Record<string, string>)[k] ?? k;
}

function prettyJson(s: string): string {
  try { return JSON.stringify(JSON.parse(s), null, 2); }
  catch { return s; }
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(720px, 92vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  controls: { display: "flex", flexDirection: "column", gap: 8, padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  row:      { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" },
  label:    { fontSize: 12 },
  input:    { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  numInput: { width: 90, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  select:   { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  textArea: { padding: "6px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", fontFamily: "inherit", resize: "vertical" },
  hint:     { fontSize: 12, opacity: 0.7 },
  note:     { fontSize: 12, fontStyle: "italic", opacity: 0.85, padding: 8, background: "var(--color-bg)", borderRadius: 4 },
  runBtn:   { alignSelf: "flex-start", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  body:     { padding: "10px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 12 },
  statusLine: { fontSize: 12, opacity: 0.85 },
  proposal: { border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  proposalHead: { cursor: "pointer", fontSize: 13, fontWeight: 600 },
  pre:      { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  error:    { color: "var(--color-error, #c62828)", padding: "8px 14px" },
};
