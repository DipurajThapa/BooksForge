/**
 * Character Bible panel (BACKLOG §A13 / Phase 1).
 *
 * Runs the character-bible agent and lets the user review the proposed
 * character cards before accepting. On accept, the orchestrator persists
 * one entity-scope memory entry per character.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  RunCharacterBibleInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  projectId: string;
  model:     string;
  onClose:   () => void;
  onApplied?: () => void;
}

interface CharacterCard {
  name: string;
  role: string;
  external_objective: string;
  internal_need: string;
  fear_or_wound: string;
  secret_or_contradiction: string;
  voice_traits: string[];
  relationships: { to: string; nature: string }[];
  chapter_arc: string[];
  emotional_turning_points: string[];
}

function tryParseProposal(json: string | null): { characters: CharacterCard[] } | null {
  if (!json) return null;
  try {
    return JSON.parse(json);
  } catch {
    return null;
  }
}

export default function CharacterBiblePanel({ projectId, model, onClose, onApplied }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [chapterCount,   setChapterCount]    = useState(12);
  const [acceptedProseRaw, setAcceptedProseRaw] = useState("");
  const [running,        setRunning]         = useState(false);
  const [applying,       setApplying]        = useState(false);
  const [applied,        setApplied]         = useState(false);
  const [result,         setResult]          = useState<AgentRunResultDto | null>(null);
  const [error,          setError]           = useState<string | null>(null);

  async function handleRun() {
    if (chapterCount < 1) {
      setError("Chapter count must be at least 1.");
      return;
    }
    setError(null);
    setRunning(true);
    setResult(null);
    setApplied(false);
    try {
      // One sample per blank-line-separated paragraph (intuitive UX).
      const samples = acceptedProseRaw
        .split(/\n\s*\n/)
        .map(s => s.trim())
        .filter(s => s.length > 0);
      const input: RunCharacterBibleInput = {
        project_id:             projectId,
        chapter_count:          chapterCount,
        accepted_prose_samples: samples,
        model,
      };
      const r = await ipc.agentRunCharacterBible(input);
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  async function handleApply() {
    if (!result) return;
    setApplying(true);
    setError(null);
    try {
      const r = await ipc.agentApplyCharacterBible({ task_id: result.task_id });
      setApplied(true);
      // Surface a friendly summary in the status line.
      if (r.character_names.length > 0) {
        setError(`✓ Applied ${r.character_names.length} character(s) to project memory.`);
      }
      onApplied?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setApplying(false);
    }
  }

  const proposal = tryParseProposal(result?.proposal_json ?? null);
  const characters = proposal?.characters ?? [];

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Character Bible</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Build a per-character bible — objective, internal need, wound, secret,
            voice traits, per-chapter arc — from your project brief. Optionally
            paste 1-3 paragraphs of accepted prose so the bible can derive
            measurable voice traits from them.
          </p>

          <div style={s.row}>
            <label style={s.label}>Chapter count</label>
            <input
              type="number"
              min={1}
              max={120}
              style={s.numInput}
              value={chapterCount}
              onChange={e => setChapterCount(parseInt(e.target.value || "12", 10))}
            />
            <span style={s.hint}>
              Each character&apos;s arc must have one entry per chapter.
            </span>
          </div>

          <label style={s.label}>Accepted prose samples (optional)</label>
          <textarea
            style={s.textarea}
            value={acceptedProseRaw}
            onChange={e => setAcceptedProseRaw(e.target.value)}
            placeholder="Paste 1-3 paragraphs of accepted prose, separated by blank lines.&#10;&#10;The bible will derive measurable voice traits (sentence length, lexicon, evasion patterns) from them."
            rows={6}
          />

          <button style={s.runBtn} onClick={handleRun} disabled={running}>
            {running ? "Running…" : "Generate character bible"}
          </button>
        </div>

        {error && <div style={s.error}>{error}</div>}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              Status: <strong>{result.status}</strong> <span style={{ opacity: 0.5 }}>· run id <code>{result.task_id}</code></span>
            </div>

            {characters.length > 0 && (
              <div style={s.charList}>
                {characters.map((c, i) => (
                  <CharacterCardView key={`${c.name}-${i}`} card={c} />
                ))}
              </div>
            )}

            {result.proposal_json && (
              <div style={s.applyRow}>
                <button
                  style={s.applyBtn}
                  onClick={handleApply}
                  disabled={applying || applied || characters.length === 0}
                  title="Save each character to project memory (entity scope). Snapshots project first; writes one audit-ledger row per character."
                >
                  {applying ? "Applying…" :
                   applied ? "✓ Applied — characters saved to project memory" :
                   `Apply ${characters.length} character${characters.length === 1 ? "" : "s"} to project memory`}
                </button>
              </div>
            )}

            {result.proposal_json && (
              <details style={s.proposal}>
                <summary style={s.proposalHead}>Raw proposal JSON</summary>
                <pre style={s.pre}>{prettyJson(result.proposal_json)}</pre>
              </details>
            )}

            {result.error && <div style={s.error}>{result.error}</div>}
          </div>
        )}
      </div>
    </div>
  );
}

function CharacterCardView({ card }: { card: CharacterCard }) {
  return (
    <div style={s.charCard}>
      <div style={s.charHead}>
        <strong style={s.charName}>{card.name}</strong>
        <span style={s.charRole}>{card.role}</span>
      </div>
      <div style={s.charField}>
        <span style={s.charLabel}>Wants:</span> {card.external_objective}
      </div>
      <div style={s.charField}>
        <span style={s.charLabel}>Needs:</span> {card.internal_need}
      </div>
      <div style={s.charField}>
        <span style={s.charLabel}>Wound:</span> {card.fear_or_wound}
      </div>
      <div style={s.charField}>
        <span style={s.charLabel}>Secret:</span> {card.secret_or_contradiction}
      </div>
      {card.voice_traits.length > 0 && (
        <div style={s.charField}>
          <span style={s.charLabel}>Voice:</span>
          <ul style={s.voiceList}>
            {card.voice_traits.map((t, i) => <li key={i}>{t}</li>)}
          </ul>
        </div>
      )}
      {card.relationships.length > 0 && (
        <div style={s.charField}>
          <span style={s.charLabel}>Relationships:</span>
          <ul style={s.voiceList}>
            {card.relationships.map((r, i) => (
              <li key={i}><em>{r.to}</em> — {r.nature}</li>
            ))}
          </ul>
        </div>
      )}
      {card.chapter_arc.length > 0 && (
        <details style={s.arcDetails}>
          <summary>Chapter arc ({card.chapter_arc.length} chapters)</summary>
          <ol style={s.arcList}>
            {card.chapter_arc.map((a, i) => <li key={i}>{a}</li>)}
          </ol>
        </details>
      )}
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
  body:     { padding: "10px 14px", overflowY: "auto", flex: "0 1 auto", display: "flex", flexDirection: "column", gap: 8 },
  blurb:    { margin: 0, fontSize: 13, opacity: 0.85 },
  row:      { display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" },
  label:    { fontSize: 12, fontWeight: 500 },
  hint:     { fontSize: 11, opacity: 0.65 },
  numInput: { width: 90, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  textarea: { padding: "8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", fontFamily: "inherit", resize: "vertical" },
  runBtn:   { alignSelf: "flex-start", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  statusLine: { fontSize: 12, opacity: 0.85 },
  charList:  { display: "flex", flexDirection: "column", gap: 8 },
  charCard:  { border: "1px solid var(--color-border)", borderRadius: 4, padding: 10, background: "var(--color-bg)" },
  charHead:  { display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 6 },
  charName:  { fontSize: 14 },
  charRole:  { fontSize: 11, opacity: 0.7, textTransform: "uppercase" },
  charField: { fontSize: 12, marginBottom: 4 },
  charLabel: { fontWeight: 600, marginRight: 4 },
  voiceList: { margin: "4px 0 0 18px", padding: 0, fontSize: 12 },
  arcDetails:{ marginTop: 6, fontSize: 11 },
  arcList:   { margin: "6px 0 0 18px", padding: 0, fontSize: 12 },
  applyRow:  { display: "flex", justifyContent: "flex-end", padding: "8px 0" },
  applyBtn:  { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontSize: 13, fontWeight: 600 },
  proposal:  { border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  proposalHead: { cursor: "pointer", fontSize: 12, fontWeight: 600 },
  pre:       { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  error:     { color: "var(--color-error, #c62828)", padding: "8px 14px", fontSize: 12 },
};
