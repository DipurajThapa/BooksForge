/**
 * Read-only inspector for the project's memory + vocabulary stores.
 *
 * Two tabs:
 *   - Memory  : book / chapter / entity / style — typed entries written
 *               by agents during the workflow.
 *   - Vocab   : layered dictionaries — shows the active layer set
 *               (project + ai_tells today; future: per-genre / audience).
 *
 * MVP scope is read-only.  Editing land in a later milestone alongside the
 * Vocabulary Dictionary Agent (Phase 5).
 */
import React, { useCallback, useEffect, useState } from "react";
import type { MemoryEntryDto, VocabEntryDto } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";

interface Props {
  onClose: () => void;
}

type Tab = "memory" | "vocab";
type MemoryScope = "book" | "chapter" | "entity" | "style";

const MEMORY_SCOPES: { value: MemoryScope; label: string }[] = [
  { value: "book",    label: "Book" },
  { value: "chapter", label: "Chapter" },
  { value: "entity",  label: "Entity" },
  { value: "style",   label: "Style" },
];

const ACTIVE_VOCAB_LAYERS = ["project", "ai_tells", "genre:fantasy", "genre:romance", "mode:non_fiction"];

export default function KnowledgePanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [tab, setTab] = useState<Tab>("memory");
  const [scope, setScope] = useState<MemoryScope>("book");
  const [memory, setMemory] = useState<MemoryEntryDto[]>([]);
  const [vocab, setVocab]   = useState<VocabEntryDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadMemory = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await ipc.memoryList({ scope });
      setMemory(rows);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [scope]);

  const loadVocab = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await ipc.vocabList({ layers: ACTIVE_VOCAB_LAYERS });
      setVocab(rows);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (tab === "memory") void loadMemory();
    if (tab === "vocab")  void loadVocab();
  }, [tab, scope, loadMemory, loadVocab]);

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <span id={titleId} style={s.title}>Knowledge</span>
          <div style={s.tabs}>
            <button
              style={{ ...s.tab, ...(tab === "memory" ? s.tabActive : null) }}
              onClick={() => setTab("memory")}
            >Memory</button>
            <button
              style={{ ...s.tab, ...(tab === "vocab"  ? s.tabActive : null) }}
              onClick={() => setTab("vocab")}
            >Vocabulary</button>
          </div>
          <button style={s.closeBtn} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {error && <div style={s.error}>{error}</div>}

          {tab === "memory" && (
            <>
              <div style={s.scopeRow}>
                {MEMORY_SCOPES.map((sc) => (
                  <button
                    key={sc.value}
                    style={{ ...s.scopeBtn, ...(scope === sc.value ? s.scopeBtnActive : null) }}
                    onClick={() => setScope(sc.value)}
                  >{sc.label}</button>
                ))}
              </div>
              {loading && <p style={s.empty}>Loading…</p>}
              {!loading && memory.length === 0 && (
                <p style={s.empty}>No <b>{scope}</b> memory entries yet.</p>
              )}
              {!loading && memory.length > 0 && (
                <ul style={s.list}>
                  {memory.map((m) => (
                    <li key={m.id} style={s.row}>
                      <div style={s.rowHead}>
                        <span style={s.key}>{m.key}</span>
                        <span style={s.agentBadge}>{m.agent_id}</span>
                      </div>
                      <pre style={s.value}>{prettyJson(m.value_json)}</pre>
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}

          {tab === "vocab" && (
            <>
              {loading && <p style={s.empty}>Loading…</p>}
              {!loading && vocab.length === 0 && (
                <p style={s.empty}>No vocabulary entries on the active layers.</p>
              )}
              {!loading && vocab.length > 0 && (
                <>
                  <p style={s.note}>
                    Active layers: {ACTIVE_VOCAB_LAYERS.map((l) => (
                      <code key={l} style={s.layerChip}>{l}</code>
                    ))}
                  </p>
                  <ul style={s.list}>
                    {vocab.map((v) => (
                      <li key={v.id} style={s.row}>
                        <div style={s.rowHead}>
                          <span style={s.term}>{v.display_term}</span>
                          <span style={{ ...s.kindBadge, ...kindColor(v.kind) }}>{v.kind}</span>
                          <span style={s.layerChip}>{v.layer}</span>
                        </div>
                        {v.replacement && (
                          <div style={s.replacement}>→ <b>{v.replacement}</b></div>
                        )}
                        {v.rationale && (
                          <div style={s.rationale}>{v.rationale}</div>
                        )}
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </>
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

function kindColor(kind: string): React.CSSProperties {
  switch (kind) {
    case "prefer":  return { background: "var(--color-success, #22c55e)" };
    case "avoid":   return { background: "var(--color-error, #ef4444)" };
    case "replace": return { background: "var(--color-amber-500, #f59e0b)" };
    default:        return { background: "var(--color-neutral-400)" };
  }
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)",
    display: "flex", alignItems: "flex-start", justifyContent: "center",
    zIndex: 500, paddingTop: 48,
  },
  dialog: {
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)",
    borderRadius: 8,
    width: "min(96vw, 880px)",
    maxHeight: "calc(100vh - 72px)",
    display: "flex", flexDirection: "column", overflow: "hidden",
    boxShadow: "0 8px 32px rgba(0,0,0,0.25)",
  },
  header: {
    display: "flex", alignItems: "center", gap: 12,
    padding: "12px 16px", borderBottom: "1px solid var(--color-border)",
    flexShrink: 0,
  },
  title: { fontWeight: 600, fontSize: 14, color: "var(--color-text-primary)" },
  tabs:  { display: "flex", gap: 4, flex: 1, justifyContent: "center" },
  tab: {
    background: "transparent", border: "1px solid var(--color-border)",
    borderRadius: 4, fontSize: 12, padding: "4px 12px",
    cursor: "pointer", color: "var(--color-text-secondary)",
  },
  tabActive: {
    background: "var(--color-amber-600)", borderColor: "var(--color-amber-600)",
    color: "#fff", fontWeight: 600,
  },
  closeBtn: {
    background: "none", border: "none", cursor: "pointer",
    fontSize: 16, color: "var(--color-text-secondary)", padding: "0 4px",
  },
  body: { padding: 16, overflowY: "auto", display: "flex", flexDirection: "column", gap: 12, flex: 1 },
  scopeRow: { display: "flex", gap: 6 },
  scopeBtn: {
    background: "var(--color-surface-raised)", border: "1px solid var(--color-border)",
    borderRadius: 4, fontSize: 12, padding: "4px 12px", cursor: "pointer",
    color: "var(--color-text-secondary)",
  },
  scopeBtnActive: {
    background: "var(--color-amber-600)", borderColor: "var(--color-amber-600)",
    color: "#fff", fontWeight: 600,
  },
  empty: { fontSize: 13, color: "var(--color-text-tertiary)", textAlign: "center", padding: "20px 0" },
  note:  { fontSize: 11, color: "var(--color-text-tertiary)", margin: 0, display: "flex", alignItems: "center", gap: 6 },
  list:  { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 6 },
  row: {
    padding: "8px 12px",
    border: "1px solid var(--color-border)", borderRadius: 4,
    background: "var(--color-surface-raised)",
    display: "flex", flexDirection: "column", gap: 4,
  },
  rowHead: { display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" },
  key:     { fontSize: 13, fontWeight: 600, color: "var(--color-text-primary)" },
  term:    { fontSize: 13, fontWeight: 600, color: "var(--color-text-primary)" },
  agentBadge: {
    fontSize: 10, padding: "1px 6px", borderRadius: 99,
    background: "var(--color-surface)", border: "1px solid var(--color-border)",
    color: "var(--color-text-tertiary)", fontFamily: "var(--font-mono)",
  },
  kindBadge: {
    fontSize: 10, padding: "1px 6px", borderRadius: 99,
    color: "#fff", fontWeight: 700,
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  layerChip: {
    fontSize: 10, padding: "1px 6px", borderRadius: 4,
    background: "var(--color-surface)", border: "1px solid var(--color-border)",
    color: "var(--color-text-tertiary)", fontFamily: "var(--font-mono)",
    margin: "0 2px",
  },
  value: {
    margin: 0, padding: "6px 8px", borderRadius: 4,
    background: "var(--color-surface)", border: "1px solid var(--color-border)",
    fontSize: 11, fontFamily: "var(--font-mono)",
    whiteSpace: "pre-wrap", color: "var(--color-text-primary)",
    overflowX: "auto",
  },
  replacement: { fontSize: 12, color: "var(--color-text-primary)" },
  rationale:   { fontSize: 11, color: "var(--color-text-tertiary)", fontStyle: "italic" },
  error: {
    padding: "8px 12px", borderRadius: 4,
    border: "1px solid var(--color-error, #ef4444)",
    color:  "var(--color-error, #ef4444)",
    fontSize: 12, fontFamily: "var(--font-mono)",
  },
};
