/**
 * World Bible panel (BACKLOG §A13 / Phase 1).
 *
 * Runs the world-bible agent (locations + social rules + history +
 * sensory palette + motifs + continuity constraints) and lets the user
 * review before accepting. On accept: locations → entity-scope memory,
 * everything else → book-scope memory.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  RunWorldBibleInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  projectId: string;
  model:     string;
  onClose:   () => void;
  onApplied?: () => void;
}

interface WorldLocation {
  name: string;
  purpose_in_story: string;
  sensory_signature: string;
  key_constraints: string;
}

interface SensoryPalette {
  sight: string;
  sound: string;
  smell: string;
  touch: string;
  taste: string;
}

interface WorldBibleProposal {
  main_locations: WorldLocation[];
  social_rules: string[];
  history: string;
  sensory_palette: SensoryPalette;
  conflict_sources: string[];
  symbolic_motifs: string[];
  continuity_constraints: string[];
}

function tryParseProposal(json: string | null): WorldBibleProposal | null {
  if (!json) return null;
  try {
    return JSON.parse(json);
  } catch {
    return null;
  }
}

export default function WorldBiblePanel({ projectId, model, onClose, onApplied }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [running,  setRunning]  = useState(false);
  const [applying, setApplying] = useState(false);
  const [applied,  setApplied]  = useState(false);
  const [result,   setResult]   = useState<AgentRunResultDto | null>(null);
  const [error,    setError]    = useState<string | null>(null);

  async function handleRun() {
    setError(null);
    setRunning(true);
    setResult(null);
    setApplied(false);
    try {
      const input: RunWorldBibleInput = { project_id: projectId, model };
      const r = await ipc.agentRunWorldBible(input);
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
      const r = await ipc.agentApplyWorldBible({ task_id: result.task_id });
      setApplied(true);
      setError(`✓ Applied ${r.location_names.length} location(s) + ${r.book_scope_keys.length} book-scope field(s) to project memory.`);
      onApplied?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setApplying(false);
    }
  }

  const proposal = tryParseProposal(result?.proposal_json ?? null);

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>World Bible</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Build a world / setting bible — locations with sensory signatures,
            social rules, history, motifs, and continuity constraints — from
            your project brief. Reads any prior accepted bible from memory
            and extends it (re-run to refine).
          </p>
          <button style={s.runBtn} onClick={handleRun} disabled={running}>
            {running ? "Running…" : "Generate world bible"}
          </button>
        </div>

        {error && <div style={s.error}>{error}</div>}

        {result && (
          <div style={s.body}>
            <div style={s.statusLine}>
              Status: <strong>{result.status}</strong> <span style={{ opacity: 0.5 }}>· run id <code>{result.task_id}</code></span>
            </div>

            {proposal && (
              <>
                {proposal.main_locations.length > 0 && (
                  <Section title={`Locations (${proposal.main_locations.length})`}>
                    {proposal.main_locations.map((l, i) => (
                      <div key={i} style={s.locCard}>
                        <strong>{l.name}</strong>
                        <div style={s.locField}><em>Purpose:</em> {l.purpose_in_story}</div>
                        <div style={s.locField}><em>Sensory:</em> {l.sensory_signature}</div>
                        {l.key_constraints && (
                          <div style={s.locField}><em>Constraints:</em> {l.key_constraints}</div>
                        )}
                      </div>
                    ))}
                  </Section>
                )}

                {proposal.social_rules.length > 0 && (
                  <Section title={`Social rules (${proposal.social_rules.length})`}>
                    <ul style={s.list}>
                      {proposal.social_rules.map((r, i) => <li key={i}>{r}</li>)}
                    </ul>
                  </Section>
                )}

                {proposal.history && (
                  <Section title="History">
                    <p style={s.paragraph}>{proposal.history}</p>
                  </Section>
                )}

                <Section title="Sensory palette">
                  <ul style={s.list}>
                    {(["sight","sound","smell","touch","taste"] as const).map(k => (
                      proposal.sensory_palette[k] ? (
                        <li key={k}><strong>{k}:</strong> {proposal.sensory_palette[k]}</li>
                      ) : null
                    ))}
                  </ul>
                </Section>

                {proposal.continuity_constraints.length > 0 && (
                  <Section title={`Continuity constraints (${proposal.continuity_constraints.length})`}>
                    <ul style={s.list}>
                      {proposal.continuity_constraints.map((c, i) => <li key={i}>{c}</li>)}
                    </ul>
                  </Section>
                )}

                {proposal.symbolic_motifs.length > 0 && (
                  <Section title="Symbolic motifs">
                    <ul style={s.list}>
                      {proposal.symbolic_motifs.map((m, i) => <li key={i}>{m}</li>)}
                    </ul>
                  </Section>
                )}
              </>
            )}

            {result.proposal_json && (
              <div style={s.applyRow}>
                <button
                  style={s.applyBtn}
                  onClick={handleApply}
                  disabled={applying || applied || !proposal}
                  title="Save the world bible to project memory. Snapshots project first; one audit-ledger row per memory write."
                >
                  {applying ? "Applying…" :
                   applied ? "✓ Applied — saved to project memory" :
                   "Apply world bible to project memory"}
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

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={s.section}>
      <h4 style={s.sectionTitle}>{title}</h4>
      {children}
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
  runBtn:   { alignSelf: "flex-start", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  statusLine: { fontSize: 12, opacity: 0.85 },
  section:  { borderTop: "1px solid var(--color-border)", paddingTop: 8 },
  sectionTitle: { margin: "0 0 6px", fontSize: 13 },
  locCard:  { border: "1px solid var(--color-border)", borderRadius: 4, padding: 8, marginBottom: 6, fontSize: 12, background: "var(--color-bg)" },
  locField: { marginTop: 4 },
  list:     { margin: "0 0 0 18px", padding: 0, fontSize: 12 },
  paragraph:{ margin: 0, fontSize: 12, lineHeight: 1.5 },
  applyRow: { display: "flex", justifyContent: "flex-end", padding: "8px 0" },
  applyBtn: { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontSize: 13, fontWeight: 600 },
  proposal: { border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  proposalHead: { cursor: "pointer", fontSize: 12, fontWeight: 600 },
  pre:      { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  error:    { color: "var(--color-error, #c62828)", padding: "8px 14px", fontSize: 12 },
};
