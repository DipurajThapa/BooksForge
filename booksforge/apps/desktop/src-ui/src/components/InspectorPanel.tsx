/**
 * Right-pane inspector for the currently-selected node.
 *
 * Lets the writer edit the metadata that drives the Outline view, the
 * status colour in the binder, and the per-scene word target shown in
 * the status bar:
 *
 *   • Title
 *   • Status (planned / drafting / revised / final)
 *   • POV character
 *   • Story beat
 *   • Target word count
 *
 * All edits are debounced (500ms) and persisted via `nodeUpdate`. The
 * parent re-fetches the node list after each save so the binder + status
 * bar reflect the change.
 */
import React, { useEffect, useRef, useState } from "react";
import type { NodeInfo, NodeUpdateInput } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";

interface Props {
  node:    NodeInfo | null;
  onSaved: () => void;
}

const STATUSES: Array<{ value: string; label: string }> = [
  { value: "planned",  label: "Planned" },
  { value: "drafting", label: "Drafting" },
  { value: "revised",  label: "Revised" },
  { value: "final",    label: "Final" },
];

export default function InspectorPanel({ node, onSaved }: Props) {
  // Local mirrors so typing is responsive even before the IPC round-trip.
  const [title,        setTitle]        = useState("");
  const [status,       setStatus]       = useState("planned");
  const [pov,          setPov]          = useState("");
  const [beat,         setBeat]         = useState("");
  const [targetWords,  setTargetWords]  = useState<string>("");
  const [savingError,  setSavingError]  = useState<string | null>(null);
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Sync local state when the selected node changes.
  useEffect(() => {
    if (debounce.current) {
      clearTimeout(debounce.current);
      debounce.current = null;
    }
    if (!node) {
      setTitle(""); setStatus("planned"); setPov(""); setBeat(""); setTargetWords("");
      return;
    }
    setTitle(node.title);
    setStatus(node.status);
    setPov(node.pov ?? "");
    setBeat(node.beat ?? "");
    setTargetWords(node.target_words ? String(node.target_words) : "");
    setSavingError(null);
  }, [node?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  function scheduleSave(next: Partial<NodeUpdateInput>) {
    if (!node) return;
    if (debounce.current) clearTimeout(debounce.current);
    debounce.current = setTimeout(async () => {
      try {
        await ipc.nodeUpdate({
          id:           node.id,
          title:        next.title        ?? title,
          status:       next.status       ?? status,
          pov:          next.pov          ?? (pov || null),
          beat:         next.beat         ?? (beat || null),
          target_words: next.target_words ?? parseTarget(targetWords),
          position:     null,
        });
        onSaved();
        setSavingError(null);
      } catch (e) {
        setSavingError(String(e));
      }
    }, 500);
  }

  if (!node) {
    return (
      <aside style={s.panel}>
        <div style={s.empty}>Select a node to edit its outline metadata.</div>
      </aside>
    );
  }

  const isScene   = node.kind === "scene";
  const showPov   = isScene;
  const showBeat  = isScene;
  const showWords = isScene;

  return (
    <aside style={s.panel}>
      <div style={s.header}>
        <span style={s.kindBadge}>{node.kind.replace("_", " ")}</span>
      </div>

      <Field label="Title">
        <input
          style={s.input}
          value={title}
          onChange={(e) => { setTitle(e.target.value); scheduleSave({ title: e.target.value }); }}
          placeholder="Untitled"
        />
      </Field>

      <Field label="Status">
        <select
          style={s.input}
          value={status}
          onChange={(e) => { setStatus(e.target.value); scheduleSave({ status: e.target.value }); }}
        >
          {STATUSES.map((opt) => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
        </select>
      </Field>

      {showPov && (
        <Field label="POV">
          <input
            style={s.input}
            value={pov}
            onChange={(e) => { setPov(e.target.value); scheduleSave({ pov: e.target.value || null }); }}
            placeholder="Whose eyes is this scene through?"
          />
        </Field>
      )}

      {showBeat && (
        <Field label="Beat">
          <input
            style={s.input}
            value={beat}
            onChange={(e) => { setBeat(e.target.value); scheduleSave({ beat: e.target.value || null }); }}
            placeholder="e.g. inciting incident, pinch point"
          />
        </Field>
      )}

      {showWords && (
        <Field label="Target words">
          <input
            style={s.input}
            type="number"
            inputMode="numeric"
            min={0}
            max={100000}
            value={targetWords}
            onChange={(e) => { setTargetWords(e.target.value); scheduleSave({ target_words: parseTarget(e.target.value) }); }}
            placeholder="e.g. 1500"
          />
        </Field>
      )}

      <div style={s.meta}>
        <div><b>{node.word_count.toLocaleString()}</b> words written</div>
        {showWords && parseTarget(targetWords) !== null && (
          <div style={s.metaProgress}>
            <Progress current={node.word_count} target={parseTarget(targetWords) ?? 0} />
          </div>
        )}
      </div>

      {savingError && <div style={s.error}>{savingError}</div>}
    </aside>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>{label}</span>
      {children}
    </label>
  );
}

function Progress({ current, target }: { current: number; target: number }) {
  const pct = target > 0 ? Math.min(100, Math.round((current / target) * 100)) : 0;
  return (
    <div style={s.progressTrack}>
      <div style={{ ...s.progressFill, width: `${pct}%` }} />
      <span style={s.progressLabel}>{pct}%</span>
    </div>
  );
}

function parseTarget(raw: string): number | null {
  const n = Number(raw.trim());
  if (!Number.isFinite(n) || n <= 0) return null;
  return Math.floor(n);
}

const s: Record<string, React.CSSProperties> = {
  panel: {
    width: 280,
    minWidth: 280,
    borderLeft: "1px solid var(--color-border)",
    display: "flex",
    flexDirection: "column",
    background: "var(--color-neutral-50, #fafafa)",
    padding: "12px 14px",
    gap: 12,
    overflowY: "auto",
  },
  header: {
    display: "flex", alignItems: "center", gap: 8,
    paddingBottom: 4, borderBottom: "1px solid var(--color-border)",
  },
  kindBadge: {
    fontSize: 10, fontWeight: 700, letterSpacing: "0.06em",
    textTransform: "uppercase", color: "var(--color-text-tertiary)",
  },
  empty: {
    color: "var(--color-text-tertiary)", fontSize: 13, padding: "20px 0",
    textAlign: "center", lineHeight: 1.5,
  },
  field: {
    display: "flex", flexDirection: "column", gap: 4,
    fontSize: 12, color: "var(--color-text-secondary)",
  },
  fieldLabel: {
    fontSize: 11, fontWeight: 600, textTransform: "uppercase",
    letterSpacing: "0.04em", color: "var(--color-text-tertiary)",
  },
  input: {
    fontFamily: "var(--font-ui)", fontSize: 13,
    padding: "5px 8px", border: "1px solid var(--color-border)",
    borderRadius: 4, background: "var(--color-surface)",
    color: "var(--color-text-primary)",
    width: "100%", boxSizing: "border-box",
  },
  meta: {
    display: "flex", flexDirection: "column", gap: 6,
    paddingTop: 10, borderTop: "1px solid var(--color-border)",
    fontSize: 12, color: "var(--color-text-secondary)",
    fontVariantNumeric: "tabular-nums",
  },
  metaProgress: { fontSize: 11 },
  progressTrack: {
    position: "relative", height: 14, background: "var(--color-surface)",
    border: "1px solid var(--color-border)", borderRadius: 4, overflow: "hidden",
  },
  progressFill: {
    height: "100%", background: "var(--color-amber-500, #f59e0b)",
    transition: "width 200ms ease",
  },
  progressLabel: {
    position: "absolute", inset: 0, display: "flex",
    alignItems: "center", justifyContent: "center", fontSize: 10,
    color: "var(--color-text-secondary)", fontWeight: 600,
  },
  error: {
    fontSize: 11, color: "var(--color-error, #ef4444)",
    padding: "6px 8px", border: "1px solid var(--color-error, #ef4444)",
    borderRadius: 4, background: "var(--color-surface)",
  },
};
