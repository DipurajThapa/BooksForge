import React, { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { OpenProjectResult, RecentProjectEntry } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";

interface Props {
  onProjectOpened: (result: OpenProjectResult) => void;
  onNewProject: () => void;
}

export default function ProjectPicker({ onProjectOpened, onNewProject }: Props) {
  const [recents, setRecents] = useState<RecentProjectEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .projectRecent()
      .then(setRecents)
      .catch((e: unknown) => setError(String(e)))
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
      setError(String(e));
    }
  }

  async function handleOpenRecent(entry: RecentProjectEntry) {
    if (entry.missing) return;
    try {
      const result = await ipc.projectOpen({ bundle_path: entry.path });
      onProjectOpened(result);
    } catch (e) {
      setError(String(e));
    }
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

        {!loading && recents.length > 0 && (
          <section style={s.recentSection}>
            <h2 style={s.recentHeading}>Recent</h2>
            <ul style={s.recentList}>
              {recents.map((entry) => (
                <li
                  key={entry.id}
                  style={{
                    ...s.recentItem,
                    opacity: entry.missing ? 0.4 : 1,
                    cursor: entry.missing ? "default" : "pointer",
                  }}
                  onClick={() => handleOpenRecent(entry)}
                  title={entry.missing ? "Project folder not found" : entry.path}
                >
                  <span style={s.recentName}>{entry.name}</span>
                  <span style={s.recentPath}>{entry.path}</span>
                  {entry.missing && <span style={s.missingBadge}>missing</span>}
                </li>
              ))}
            </ul>
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
    width: 480,
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
    flexDirection: "column",
    padding: "var(--space-2) var(--space-3)",
    borderRadius: 4,
    gap: 2,
    position: "relative",
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
  missingBadge: {
    position: "absolute",
    right: "var(--space-3)",
    top: "var(--space-2)",
    fontSize: 10,
    color: "var(--color-error)",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.06em",
  },
};
