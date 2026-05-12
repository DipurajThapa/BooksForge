import React, { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { OpenProjectResult, RecentProjectEntry } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
interface Props {
  onProjectOpened: (result: OpenProjectResult) => void;
  onNewProject: () => void;
}

export default function ProjectPicker({ onProjectOpened, onNewProject }: Props) {
  const [recents, setRecents] = useState<RecentProjectEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  // Per-row busy lock so a slow projectOpen doesn't let the user
  // double-click into two open requests on the same row.
  const [busyId, setBusyId] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .projectRecent()
      .then(setRecents)
      .catch((e: unknown) => setError(errorMessage(e)))
      .finally(() => setLoading(false));
  }, []);

  async function handleOpen() {
    try {
      const selected = await openDialog({
        title: "Open BooksForge Project",
        directory: true,
        multiple: false,
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : selected[0];
      if (!path) return;
      const result = await ipc.projectOpen({ bundle_path: path });
      onProjectOpened(result);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  async function handleOpenRecent(entry: RecentProjectEntry) {
    if (entry.missing || busyId) return;
    setBusyId(entry.id);
    setError(null);
    try {
      const result = await ipc.projectOpen({ bundle_path: entry.path });
      onProjectOpened(result);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  // Remove the entry from the picker only — never deletes the bundle on disk.
  // We confirm via window.confirm so an accidental click doesn't wipe a row
  // the user actually wants to keep.
  async function handleRemoveRecent(entry: RecentProjectEntry, e: React.MouseEvent) {
    e.stopPropagation();
    if (busyId) return;
    const ok = window.confirm(
      `Remove "${entry.name}" from Recent?\n\nThis only clears the entry in the picker — your project files are not deleted.`,
    );
    if (!ok) return;
    setBusyId(entry.id);
    setError(null);
    try {
      const updated = await ipc.projectRecentRemove({ path: entry.path });
      setRecents(updated);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  function formatLastOpened(iso: string): string {
    const t = Date.parse(iso);
    if (Number.isNaN(t)) return "";
    const elapsedMs = Date.now() - t;
    const minutes = Math.floor(elapsedMs / 60_000);
    if (minutes < 1) return "just now";
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months}mo ago`;
    const years = Math.floor(days / 365);
    return `${years}y ago`;
  }

  return (
    <div style={s.root}>
      <div style={s.card}>
        <h1 style={s.wordmark}>BooksForge</h1>
        <p style={s.tagline}>Local-first writing, AI-assisted.</p>

        <div style={s.actions}>
          <button style={s.primaryBtn} onClick={onNewProject}>
            New Project…
          </button>
          <button style={s.secondaryBtn} onClick={handleOpen}>
            Open…
          </button>
        </div>

        {error && <p style={s.error}>{error}</p>}

        {!loading && recents.length === 0 && (
          <section style={s.recentSection}>
            <h2 style={s.recentHeading}>Recent</h2>
            <p style={s.emptyHint}>
              No recent projects yet — start a new one or open an existing
              <code style={s.codeInline}>.booksforge/</code> bundle.
            </p>
          </section>
        )}

        {!loading && recents.length > 0 && (
          <section style={s.recentSection}>
            <h2 style={s.recentHeading}>Recent</h2>
            <ul style={s.recentList}>
              {recents.map((entry) => {
                const isHovered = hoveredId === entry.id;
                const isBusy = busyId === entry.id;
                const itemBg = entry.missing
                  ? "transparent"
                  : isHovered
                  ? "var(--color-surface-raised, rgba(0,0,0,0.04))"
                  : "transparent";
                const itemBorder = isHovered && !entry.missing
                  ? "1px solid var(--color-amber-400, #fbbf24)"
                  : "1px solid transparent";
                return (
                  <li
                    key={entry.id}
                    style={{
                      ...s.recentItem,
                      opacity: entry.missing ? 0.55 : 1,
                      cursor: entry.missing || isBusy ? "default" : "pointer",
                      background: itemBg,
                      border: itemBorder,
                    }}
                    onClick={() => handleOpenRecent(entry)}
                    onMouseEnter={() => setHoveredId(entry.id)}
                    onMouseLeave={() => setHoveredId((id) => (id === entry.id ? null : id))}
                    title={entry.missing ? "Project folder not found on disk" : `Open ${entry.path}`}
                    aria-disabled={entry.missing || isBusy}
                  >
                    <div style={s.recentLeft}>
                      <span style={s.recentName}>{entry.name}</span>
                      <span style={s.recentPath}>{entry.path}</span>
                      <span style={s.recentMeta}>
                        {formatLastOpened(entry.last_opened)}
                      </span>
                    </div>
                    <div style={s.recentRight}>
                      {entry.missing && <span style={s.missingBadge}>missing</span>}
                      {!entry.missing && (
                        <button
                          style={{
                            ...s.openBtn,
                            visibility: isHovered ? "visible" : "hidden",
                          }}
                          onClick={(e) => {
                            e.stopPropagation();
                            handleOpenRecent(entry);
                          }}
                          disabled={isBusy}
                          aria-label={`Open ${entry.name}`}
                        >
                          {isBusy ? "Opening…" : "Open"}
                        </button>
                      )}
                      <button
                        style={{
                          ...s.removeBtn,
                          visibility: isHovered || entry.missing ? "visible" : "hidden",
                        }}
                        onClick={(e) => handleRemoveRecent(entry, e)}
                        disabled={isBusy}
                        title="Remove from Recent (does not delete files)"
                        aria-label={`Remove ${entry.name} from Recent`}
                      >
                        Remove
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
            <p style={s.recentFootnote}>
              Click a row to open. “Remove” only clears the picker entry — your
              files stay on disk.
            </p>
          </section>
        )}
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  root: {
    minHeight: "100vh",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: "var(--color-surface)",
  },
  card: {
    width: 540,
    padding: "var(--space-8)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-4)",
  },
  wordmark: {
    fontFamily: "var(--font-prose)",
    fontSize: 32,
    fontWeight: 700,
    color: "var(--color-amber-600)",
    margin: 0,
    letterSpacing: "-0.02em",
  },
  tagline: {
    color: "var(--color-text-secondary)",
    fontSize: 14,
    margin: 0,
  },
  actions: {
    display: "flex",
    gap: "var(--space-2)",
    marginTop: "var(--space-2)",
  },
  primaryBtn: {
    flex: 1,
    padding: "var(--space-3) var(--space-4)",
    background: "var(--color-amber-600)",
    color: "#fff",
    border: "none",
    borderRadius: 6,
    fontSize: 14,
    fontWeight: 600,
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  secondaryBtn: {
    flex: 1,
    padding: "var(--space-3) var(--space-4)",
    background: "transparent",
    color: "var(--color-text-primary)",
    border: "1px solid var(--color-border)",
    borderRadius: 6,
    fontSize: 14,
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  error: {
    color: "var(--color-error)",
    fontFamily: "var(--font-mono)",
    fontSize: 12,
    margin: 0,
  },
  recentSection: {
    marginTop: "var(--space-4)",
    borderTop: "1px solid var(--color-border)",
    paddingTop: "var(--space-4)",
  },
  recentHeading: {
    fontSize: 11,
    fontWeight: 600,
    letterSpacing: "0.08em",
    textTransform: "uppercase",
    color: "var(--color-text-tertiary)",
    margin: "0 0 var(--space-2)",
  },
  emptyHint: {
    fontSize: 12,
    color: "var(--color-text-tertiary)",
    margin: 0,
  },
  codeInline: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-surface-raised, rgba(0,0,0,0.05))",
    borderRadius: 3,
    margin: "0 2px",
  },
  recentList: {
    listStyle: "none",
    margin: 0,
    padding: 0,
    display: "flex",
    flexDirection: "column",
    gap: 2,
  },
  recentItem: {
    display: "flex",
    alignItems: "flex-start",
    justifyContent: "space-between",
    padding: "var(--space-2) var(--space-3)",
    borderRadius: 4,
    gap: "var(--space-3)",
    transition: "background 80ms ease, border-color 80ms ease",
  },
  recentLeft: {
    display: "flex",
    flexDirection: "column",
    gap: 2,
    minWidth: 0,
    flex: 1,
  },
  recentRight: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-2)",
    flexShrink: 0,
  },
  recentName: {
    fontSize: 14,
    fontWeight: 500,
    color: "var(--color-text-primary)",
  },
  recentPath: {
    fontSize: 11,
    color: "var(--color-text-tertiary)",
    fontFamily: "var(--font-mono)",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  recentMeta: {
    fontSize: 10,
    color: "var(--color-text-tertiary)",
    fontVariantNumeric: "tabular-nums",
    marginTop: 2,
  },
  missingBadge: {
    fontSize: 10,
    color: "var(--color-error)",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    padding: "2px 6px",
    border: "1px solid var(--color-error)",
    borderRadius: 3,
  },
  openBtn: {
    background: "var(--color-amber-600)",
    color: "#fff",
    border: "none",
    borderRadius: 4,
    fontSize: 11,
    fontWeight: 600,
    padding: "4px 10px",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  removeBtn: {
    background: "transparent",
    color: "var(--color-text-tertiary)",
    border: "1px solid var(--color-border)",
    borderRadius: 4,
    fontSize: 11,
    fontWeight: 500,
    padding: "4px 10px",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  recentFootnote: {
    fontSize: 10,
    color: "var(--color-text-tertiary)",
    margin: "var(--space-2) 0 0",
    lineHeight: 1.4,
  },
};
