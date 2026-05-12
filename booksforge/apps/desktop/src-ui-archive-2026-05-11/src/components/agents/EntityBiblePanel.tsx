/**
 * Entity Bible auto-extraction panel (BACKLOG §F4).
 *
 * Runs the memory-curator with `scope=entity` over the active scene
 * (or empty if no scene is open) and surfaces the proposed
 * `EntityStub`s as a checkbox review-list.  Accepted stubs get
 * promoted to real `Entity` rows in the project's bible via
 * `entity_bible_apply_proposals`.
 *
 * UI parity with VocabDictionaryPanel — same select-all / select-none
 * toolbar, same Apply button, same skip/insert counts on success.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  EntityBibleApplyResult,
  RunMemoryCuratorInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import VerificationReportView from "./VerificationReportView";
import { errorMessage } from "../../lib/errorMessage";

interface EntityStub {
  kind:    string;
  name:    string;
  aliases: string[];
  fields:  unknown;
}
interface MemoryRefreshProposals {
  upserts:      unknown[];
  new_entities: EntityStub[];
}

interface Props {
  projectId: string;
  sceneId:   string | null;
  model:     string;
  onClose:   () => void;
}

export default function EntityBiblePanel({ projectId, sceneId, model, onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [running,    setRunning]    = useState(false);
  const [result,     setResult]     = useState<AgentRunResultDto | null>(null);
  const [error,      setError]      = useState<string | null>(null);
  const [accepted,   setAccepted]   = useState<Set<number>>(new Set());
  const [applying,   setApplying]   = useState(false);
  const [applyOutcome, setApplyOutcome] = useState<EntityBibleApplyResult | string | null>(null);

  const proposals: MemoryRefreshProposals | null = (() => {
    if (!result?.proposal_json) return null;
    try { return JSON.parse(result.proposal_json) as MemoryRefreshProposals; }
    catch { return null; }
  })();
  const stubs = proposals?.new_entities ?? [];

  async function handleRun() {
    setError(null);
    setApplyOutcome(null);
    setAccepted(new Set());
    setRunning(true);
    setResult(null);
    try {
      const input: RunMemoryCuratorInput = {
        project_id: projectId,
        scope:      "entity",
        node_id:    sceneId,
        model,
      };
      const r = await ipc.agentRunMemoryCurator(input);
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  function toggle(i: number) {
    setAccepted(prev => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i); else next.add(i);
      return next;
    });
  }
  function selectAll() {
    setAccepted(new Set(stubs.map((_, i) => i)));
  }
  function selectNone() {
    setAccepted(new Set());
  }

  async function handleApply() {
    if (!result) return;
    setApplying(true);
    setApplyOutcome(null);
    try {
      const r = await ipc.entityBibleApplyProposals({
        task_id: result.task_id,
        accepted_indices: Array.from(accepted).sort((a, b) => a - b),
      });
      setApplyOutcome(r);
    } catch (e) {
      setApplyOutcome(errorMessage(e));
    } finally {
      setApplying(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Entity bible (auto-extract)</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Run the Memory Curator in entity-extraction mode to find
            characters, places, items, and themes the model recognises
            in the current scene (or the entire project).  Pick which to
            promote to the project's entity bible.
          </p>

          <div style={s.controls}>
            <span style={s.muted}>
              Scope: {sceneId ? "current scene" : "project (no scene open)"}
            </span>
            <button style={s.runBtn} onClick={handleRun} disabled={running}>
              {running ? "Running…" : "Extract entities"}
            </button>
          </div>

          {error && <div style={s.error}>{error}</div>}

          {result && (
            <div style={s.results}>
              <div style={s.statusLine}>
                Status: <strong>{result.status}</strong> <span style={{ opacity: 0.5 }}>· run id <code>{result.task_id}</code></span>
              </div>

              {stubs.length === 0 && (
                <div style={s.empty}>
                  Memory Curator suggested no new entities.
                </div>
              )}

              {stubs.length > 0 && (
                <>
                  <div style={s.toolbar}>
                    <button style={s.linkBtn} onClick={selectAll}>Select all</button>
                    <button style={s.linkBtn} onClick={selectNone}>Select none</button>
                    <span style={s.counter}>
                      {accepted.size} of {stubs.length} selected
                    </span>
                    <button
                      style={{ ...s.applyBtn, opacity: accepted.size === 0 ? 0.5 : 1 }}
                      onClick={handleApply}
                      disabled={applying || accepted.size === 0}
                    >
                      {applying ? "Applying…" : "Promote to bible"}
                    </button>
                  </div>

                  {applyOutcome && typeof applyOutcome !== "string" && (
                    <div style={s.ok}>
                      Inserted {applyOutcome.inserted} entit{applyOutcome.inserted === 1 ? "y" : "ies"}
                      {applyOutcome.skipped > 0 && <>, skipped {applyOutcome.skipped}</>}.
                    </div>
                  )}
                  {typeof applyOutcome === "string" && (
                    <div style={s.error}>{applyOutcome}</div>
                  )}

                  <ul style={s.list}>
                    {stubs.map((stub, i) => (
                      <li key={i} style={s.row}>
                        <input
                          type="checkbox"
                          checked={accepted.has(i)}
                          onChange={() => toggle(i)}
                          style={s.checkbox}
                        />
                        <span style={s.kindTag}>{stub.kind}</span>
                        <span style={s.name}>{stub.name}</span>
                        {stub.aliases.length > 0 && (
                          <span style={s.aliases}>
                            (aka {stub.aliases.join(", ")})
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>
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
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(820px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "12px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 12 },
  blurb:    { fontSize: 13, opacity: 0.85, margin: 0 },
  controls: { display: "flex", alignItems: "center", gap: 12 },
  muted:    { fontSize: 12, opacity: 0.7, flex: 1 },
  runBtn:   { padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit" },
  error:    { color: "var(--color-error, #c62828)", padding: 8, fontSize: 13 },
  results:  { display: "flex", flexDirection: "column", gap: 12, marginTop: 4 },
  statusLine: { fontSize: 12, opacity: 0.85 },
  empty:    { fontSize: 13, fontStyle: "italic", opacity: 0.7 },
  toolbar:  { display: "flex", alignItems: "center", gap: 12, padding: 8, border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)" },
  linkBtn:  { background: "transparent", border: "none", color: "inherit", cursor: "pointer", textDecoration: "underline", fontSize: 12, padding: 0 },
  counter:  { fontSize: 12, opacity: 0.7, flex: 1 },
  applyBtn: { padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit" },
  ok:       { padding: 8, borderRadius: 4, background: "var(--color-success-bg, rgba(46,125,50,0.12))", color: "var(--color-success, #2e7d32)", fontSize: 13 },
  list:     { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 },
  row:      { display: "flex", alignItems: "baseline", gap: 8, padding: 6, borderBottom: "1px dashed var(--color-border)", fontSize: 13 },
  checkbox: { marginTop: 2 },
  kindTag:  { fontSize: 10, padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3, textTransform: "uppercase" },
  name:     { fontWeight: 600 },
  aliases:  { fontSize: 12, opacity: 0.75 },
};
