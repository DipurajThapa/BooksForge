/**
 * MZ-06 — Snapshots panel skeleton.
 *
 * Reverse-chronological timeline of snapshots for the open project.
 * Lets the user:
 *   - Create a manual snapshot (with optional label)
 *   - Restore a snapshot (always after taking a pre-restore safety snapshot)
 *
 * Diff and selective restore are placeholders — the IPC plumbing exists,
 * but the visual diff is left for a follow-up MZ.
 */
import React, { useCallback, useEffect, useState } from "react";
import type { NodeDiffDto, SnapshotDto } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";

interface Props {
  onClose: () => void;
}

export default function SnapshotsPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [snapshots, setSnapshots] = useState<SnapshotDto[]>([]);
  const [loading, setLoading]     = useState(false);
  const [error, setError]         = useState<string | null>(null);
  const [label, setLabel]         = useState("");
  const [busy, setBusy]           = useState(false);
  const [confirmRestore, setConfirmRestore] = useState<SnapshotDto | null>(null);
  // Compare flow: pick first snapshot, then a second; show node-level diff.
  const [compareA, setCompareA]   = useState<SnapshotDto | null>(null);
  const [compareB, setCompareB]   = useState<SnapshotDto | null>(null);
  const [diffRows, setDiffRows]   = useState<NodeDiffDto[] | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await ipc.snapshotList();
      setSnapshots(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  async function handleCreate() {
    setBusy(true);
    setError(null);
    try {
      await ipc.snapshotCreate({
        scope:    "project",
        scope_id: null,
        label:    label.trim() || null,
        trigger:  "manual",
      });
      setLabel("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  // Compare picker: click once → A, click another → B + auto-run diff,
  // click an already-picked one → un-pick.
  function handleCompareToggle(snap: SnapshotDto) {
    setDiffRows(null);
    if (compareA?.id === snap.id) { setCompareA(null); return; }
    if (compareB?.id === snap.id) { setCompareB(null); return; }
    if (!compareA) { setCompareA(snap); return; }
    setCompareB(snap);
    void runDiff(compareA, snap);
  }

  async function runDiff(a: SnapshotDto, b: SnapshotDto) {
    setDiffLoading(true);
    setError(null);
    try {
      const rows = await ipc.snapshotDiff({ a: a.id, b: b.id });
      setDiffRows(rows);
    } catch (e) {
      setError(String(e));
      setDiffRows([]);
    } finally {
      setDiffLoading(false);
    }
  }

  function handleClearCompare() {
    setCompareA(null);
    setCompareB(null);
    setDiffRows(null);
  }

  async function handleRestore(snap: SnapshotDto) {
    setBusy(true);
    setError(null);
    try {
      await ipc.snapshotRestore({
        snapshot_id: snap.id,
        selective:   null,
      });
      setConfirmRestore(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <span id={titleId} style={s.title}>Snapshots</span>
          <button style={s.closeBtn} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {/* Create row */}
          <div style={s.createRow}>
            <input
              style={s.input}
              placeholder="Optional label"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              disabled={busy}
            />
            <button
              style={{ ...s.primaryBtn, opacity: busy ? 0.6 : 1 }}
              onClick={handleCreate}
              disabled={busy}
            >
              {busy ? "Working…" : "Snapshot now"}
            </button>
          </div>

          {error && <div style={s.error}>{error}</div>}

          {/* Timeline */}
          {loading && <p style={s.hint}>Loading snapshots…</p>}
          {!loading && snapshots.length === 0 && (
            <p style={s.hint}>
              No snapshots yet. Create one above, or one will be taken automatically
              before any agent applies edits.
            </p>
          )}
          {!loading && snapshots.length > 0 && (
            <ul style={s.list}>
              {snapshots.map((snap) => (
                <li key={snap.id} style={s.row}>
                  <div style={s.rowMain}>
                    <TriggerBadge trigger={snap.trigger} />
                    <span style={s.rowTitle}>{snap.label ?? "(unlabeled)"}</span>
                    <span style={s.rowMeta}>
                      {new Date(snap.created_at).toLocaleString()} · {formatBytes(snap.size_bytes)}
                    </span>
                  </div>
                  <div style={s.rowActions}>
                    <button
                      style={{
                        ...s.smallBtn,
                        ...(compareA?.id === snap.id || compareB?.id === snap.id
                            ? { background: "var(--color-amber-50, #fffbeb)",
                                borderColor: "var(--color-amber-600, #d97706)",
                                color:       "var(--color-amber-600, #d97706)" }
                            : null),
                      }}
                      onClick={() => handleCompareToggle(snap)}
                      disabled={busy}
                      title="Pick two snapshots, then click Compare"
                    >
                      {compareA?.id === snap.id ? "A" :
                       compareB?.id === snap.id ? "B" :
                       "Compare"}
                    </button>
                    <button
                      style={s.smallBtn}
                      onClick={() => setConfirmRestore(snap)}
                      disabled={busy}
                    >
                      Restore
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}

          {/* Compare-flow status + diff result */}
          {(compareA || compareB) && (
            <div style={s.compareBox}>
              <div style={s.compareHeader}>
                <span><b>A:</b> {compareA?.label ?? "—"}</span>
                <span><b>B:</b> {compareB?.label ?? "(pick one)"}</span>
                <span style={s.spacer} />
                <button style={s.smallBtn} onClick={handleClearCompare}>Clear</button>
              </div>
              {diffLoading && <div style={s.compareEmpty}>Computing diff…</div>}
              {diffRows && diffRows.length === 0 && (
                <div style={s.compareEmpty}>No node-level differences between A and B.</div>
              )}
              {diffRows && diffRows.length > 0 && (
                <ul style={s.diffList}>
                  {diffRows.map((d) => (
                    <li key={d.node_id} style={s.diffRow}>
                      <span style={{ ...s.diffKind, ...diffKindColor(d.kind) }}>{d.kind}</span>
                      <span style={s.diffTitle}>{d.title || "(untitled)"}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Restore confirmation */}
      {confirmRestore && (
        <div style={s.confirmOverlay}>
          <div style={s.confirmBox}>
            <p style={s.confirmText}>
              Restore <b>{confirmRestore.label ?? "(unlabeled)"}</b> from{" "}
              {new Date(confirmRestore.created_at).toLocaleString()}?
            </p>
            <p style={s.confirmHint}>
              A pre-restore snapshot will be created automatically so this is
              reversible.
            </p>
            <div style={s.confirmActions}>
              <button style={s.cancelBtn} onClick={() => setConfirmRestore(null)} disabled={busy}>
                Cancel
              </button>
              <button
                style={{ ...s.primaryBtn, opacity: busy ? 0.6 : 1 }}
                onClick={() => handleRestore(confirmRestore)}
                disabled={busy}
              >
                {busy ? "Restoring…" : "Restore"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function TriggerBadge({ trigger }: { trigger: string }) {
  const colors: Record<string, string> = {
    manual:          "var(--color-amber-600, #d97706)",
    auto:            "var(--color-neutral-500, #6b7280)",
    pre_ai:          "var(--color-blue-500, #3b82f6)",
    pre_export:      "var(--color-purple-500, #a855f7)",
    pre_migration:   "var(--color-neutral-500, #6b7280)",
    pre_agent_edit:  "var(--color-blue-500, #3b82f6)",
    crash_recovery:  "var(--color-error, #ef4444)",
  };
  const bg = colors[trigger] ?? "var(--color-neutral-400)";
  return <span style={{ ...s.badge, background: bg }}>{trigger.replace(/_/g, " ")}</span>;
}

function diffKindColor(kind: string): React.CSSProperties {
  switch (kind) {
    case "added":   return { background: "var(--color-success, #22c55e)" };
    case "removed": return { background: "var(--color-error, #ef4444)" };
    case "changed": return { background: "var(--color-amber-500, #f59e0b)" };
    default:        return { background: "var(--color-neutral-400)" };
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
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
    background:    "var(--color-surface)",
    border:        "1px solid var(--color-border)",
    borderRadius:  8,
    width:         "min(90vw, 720px)",
    maxHeight:     "calc(100vh - 72px)",
    display:       "flex",
    flexDirection: "column",
    overflow:      "hidden",
    boxShadow:     "0 8px 32px rgba(0,0,0,0.25)",
  },
  header: {
    display:        "flex",
    alignItems:     "center",
    justifyContent: "space-between",
    padding:        "12px 16px",
    borderBottom:   "1px solid var(--color-border)",
    flexShrink:     0,
  },
  title: { fontWeight: 600, fontSize: 14, color: "var(--color-text-primary)" },
  closeBtn: {
    background: "none", border: "none", cursor: "pointer",
    fontSize: 16, color: "var(--color-text-secondary)", padding: "0 4px",
  },
  body: { padding: 16, overflowY: "auto", display: "flex", flexDirection: "column", gap: 12 },
  createRow: { display: "flex", gap: 8, alignItems: "center" },
  input: {
    flex: 1,
    fontFamily:   "var(--font-ui)",
    fontSize:     13,
    padding:      "5px 8px",
    border:       "1px solid var(--color-border)",
    borderRadius: 4,
    background:   "var(--color-surface-raised)",
    color:        "var(--color-text-primary)",
  },
  primaryBtn: {
    background:   "var(--color-amber-600, #d97706)",
    border:       "none",
    borderRadius: 4,
    color:        "#fff",
    fontWeight:   600,
    fontSize:     13,
    padding:      "6px 14px",
    cursor:       "pointer",
  },
  smallBtn: {
    background:   "none",
    border:       "1px solid var(--color-border)",
    borderRadius: 4,
    fontSize:     12,
    color:        "var(--color-text-primary)",
    padding:      "2px 8px",
    cursor:       "pointer",
  },
  cancelBtn: {
    background:   "none",
    border:       "1px solid var(--color-border)",
    borderRadius: 4,
    fontSize:     13,
    color:        "var(--color-text-secondary)",
    padding:      "6px 14px",
    cursor:       "pointer",
  },
  list: { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 6 },
  row: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "8px 10px",
    border: "1px solid var(--color-border)", borderRadius: 4,
    background: "var(--color-surface-raised)",
  },
  rowMain:   { display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 },
  rowTitle:  { fontSize: 13, color: "var(--color-text-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  rowMeta:   { fontSize: 11, color: "var(--color-text-tertiary)", whiteSpace: "nowrap" },
  rowActions:{ display: "flex", gap: 4 },
  badge: {
    display:      "inline-block",
    padding:      "1px 6px",
    borderRadius: 99,
    fontSize:     10,
    fontWeight:   700,
    color:        "#fff",
    textTransform:"uppercase",
    letterSpacing:"0.03em",
    flexShrink:   0,
  },
  hint:  { fontSize: 13, color: "var(--color-text-tertiary)", margin: 0 },
  error: {
    fontSize:     12,
    color:        "var(--color-error, #ef4444)",
    background:   "var(--color-surface-raised)",
    border:       "1px solid var(--color-error, #ef4444)",
    borderRadius: 4,
    padding:      "6px 10px",
  },
  confirmOverlay: {
    position:   "fixed",
    inset:      0,
    background: "rgba(0,0,0,0.55)",
    display:    "flex",
    alignItems: "center",
    justifyContent: "center",
    zIndex:     600,
  },
  confirmBox: {
    background:   "var(--color-surface)",
    border:       "1px solid var(--color-border)",
    borderRadius: 8,
    padding:      20,
    maxWidth:     420,
    boxShadow:    "0 8px 32px rgba(0,0,0,0.4)",
  },
  confirmText: { fontSize: 14, color: "var(--color-text-primary)", margin: "0 0 8px" },
  confirmHint: { fontSize: 12, color: "var(--color-text-tertiary)", margin: "0 0 16px" },
  confirmActions: { display: "flex", justifyContent: "flex-end", gap: 8 },
  compareBox: {
    marginTop: 8, padding: "8px 10px",
    border: "1px solid var(--color-amber-400, #fbbf24)",
    borderRadius: 6, background: "var(--color-amber-50, #fffbeb)",
    display: "flex", flexDirection: "column", gap: 8,
  },
  compareHeader: {
    display: "flex", gap: 12, alignItems: "center",
    fontSize: 12, color: "var(--color-text-secondary)",
  },
  compareEmpty: { fontSize: 12, color: "var(--color-text-tertiary)", padding: "4px 0" },
  spacer: { flex: 1 },
  diffList: {
    listStyle: "none", padding: 0, margin: 0,
    display: "flex", flexDirection: "column", gap: 4,
    maxHeight: 220, overflowY: "auto",
  },
  diffRow: {
    display: "flex", alignItems: "center", gap: 8,
    padding: "4px 8px", borderRadius: 4,
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)",
    fontSize: 12,
  },
  diffKind: {
    display: "inline-block", padding: "1px 6px",
    borderRadius: 99, fontSize: 10, fontWeight: 700,
    color: "#fff", textTransform: "uppercase", letterSpacing: "0.04em",
    flexShrink: 0,
  },
  diffTitle: { color: "var(--color-text-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
};
