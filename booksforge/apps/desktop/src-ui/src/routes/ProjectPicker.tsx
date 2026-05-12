/**
 * ProjectPicker — landing route. Lists recent projects, opens an
 * existing bundle, or starts a new project via the wizard.
 *
 * Single source of truth: `ipc.projectRecent()`. Nothing is stored
 * here; recents come from `~/.booksforge/settings.toml` round-trips.
 */
import React, { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { OpenProjectResult, RecentProjectEntry } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import { useToast } from "../components/ToastProvider";

interface Props {
  onProjectOpened: (result: OpenProjectResult) => void;
  onNewProject:    () => void;
}

export default function ProjectPicker({ onProjectOpened, onNewProject }: Props) {
  const [recents, setRecents] = useState<RecentProjectEntry[]>([]);
  const [error,   setError]   = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [hover,   setHover]   = useState<string | null>(null);
  const [busy,    setBusy]    = useState<string | null>(null);
  const toast = useToast();

  useEffect(() => {
    ipc
      .projectRecent()
      .then(setRecents)
      .catch((e) => {
        const msg = errorMessage(e);
        setError(msg);
        toast.push({
          severity: "error",
          title: "Could not load recent projects",
          body: msg,
        });
      })
      .finally(() => setLoading(false));
    // toast is stable across renders (memoised in the provider), so
    // depending on it here is safe and silences lint without re-running.
  }, [toast]);

  async function openExisting() {
    setError(null);
    try {
      const selected = await openDialog({
        title: "Open BooksForge project",
        directory: true,
        multiple: false,
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : selected[0];
      if (!path) return;
      const result = await ipc.projectOpen({ bundle_path: path });
      onProjectOpened(result);
    } catch (e) {
      const msg = errorMessage(e);
      setError(msg);
      toast.push({
        severity: "error",
        title: "Could not open project",
        body: msg,
      });
    }
  }

  async function openRecent(entry: RecentProjectEntry) {
    if (entry.missing || busy) return;
    setBusy(entry.id);
    setError(null);
    try {
      const result = await ipc.projectOpen({ bundle_path: entry.path });
      onProjectOpened(result);
    } catch (e) {
      const msg = errorMessage(e);
      setError(msg);
      toast.push({
        severity: "error",
        title: `Could not open "${entry.name}"`,
        body: msg,
      });
    } finally {
      setBusy(null);
    }
  }

  async function removeRecent(entry: RecentProjectEntry, ev: React.MouseEvent) {
    ev.stopPropagation();
    if (busy) return;
    const ok = window.confirm(
      `Remove "${entry.name}" from Recent?\n\nThe project files on disk are not touched.`,
    );
    if (!ok) return;
    setBusy(entry.id);
    try {
      const updated = await ipc.projectRecentRemove({ path: entry.path });
      setRecents(updated);
      toast.push({
        severity: "success",
        body: `Removed "${entry.name}" from Recent. Files on disk are untouched.`,
      });
    } catch (e) {
      const msg = errorMessage(e);
      setError(msg);
      toast.push({
        severity: "error",
        title: "Could not remove from Recent",
        body: msg,
      });
    } finally {
      setBusy(null);
    }
  }

  return (
    <div style={s.root}>
      <div style={s.card}>
        <h1 style={s.wordmark}>BooksForge</h1>
        <p style={s.tagline}>Local-first book creation factory.</p>

        <div style={s.actions}>
          <button style={s.primaryBtn} onClick={onNewProject}>New Project…</button>
          <button style={s.secondaryBtn} onClick={openExisting}>Open…</button>
        </div>

        {error && <p style={s.error}>{error}</p>}

        {!loading && (
          <section style={s.recentSection}>
            <h2 style={s.recentHeading}>Recent</h2>
            {recents.length === 0 ? (
              <p style={s.recentEmpty}>
                No recent projects yet. Start a new one or open an existing
                <code style={s.code}>.booksforge/</code> bundle.
              </p>
            ) : (
              <ul style={s.recentList}>
                {recents.map((entry) => {
                  const isHover = hover === entry.id;
                  const isBusy  = busy  === entry.id;
                  return (
                    <li
                      key={entry.id}
                      style={{
                        ...s.recentItem,
                        opacity: entry.missing ? 0.55 : 1,
                        cursor: entry.missing || isBusy ? "default" : "pointer",
                        background: isHover && !entry.missing
                          ? "var(--color-neutral-100)"
                          : "transparent",
                        borderColor: isHover && !entry.missing
                          ? "var(--color-amber-400)"
                          : "transparent",
                      }}
                      onClick={() => openRecent(entry)}
                      onMouseEnter={() => setHover(entry.id)}
                      onMouseLeave={() => setHover((h) => (h === entry.id ? null : h))}
                      title={entry.missing ? "Folder not found on disk" : entry.path}
                    >
                      <div style={s.recentLeft}>
                        <span style={s.recentName}>{entry.name}</span>
                        <span style={s.recentPath}>{entry.path}</span>
                      </div>
                      <div style={s.recentRight}>
                        {entry.missing && <span style={s.missingBadge}>missing</span>}
                        <button
                          style={{
                            ...s.removeBtn,
                            visibility: isHover || entry.missing ? "visible" : "hidden",
                          }}
                          onClick={(e) => removeRecent(entry, e)}
                          disabled={isBusy}
                        >
                          Remove
                        </button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        )}
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  root: {
    minHeight: "100vh",
    display: "flex", alignItems: "center", justifyContent: "center",
    background: "var(--color-neutral-50)",
  },
  card: {
    width: 540, padding: "var(--space-8, 32px)",
    display: "flex", flexDirection: "column", gap: "var(--space-4, 16px)",
  },
  wordmark: {
    fontFamily: "var(--font-prose)", fontSize: 32, fontWeight: 700,
    color: "var(--color-amber-600)", margin: 0, letterSpacing: "-0.02em",
  },
  tagline: { color: "var(--color-neutral-600)", fontSize: 14, margin: 0 },
  actions: {
    display: "flex", gap: 8, marginTop: 8,
  },
  primaryBtn: {
    flex: 1, padding: "12px 16px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 6,
    fontSize: 14, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  secondaryBtn: {
    flex: 1, padding: "12px 16px",
    background: "transparent", color: "var(--color-neutral-800)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 6,
    fontSize: 14, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  error: {
    color: "var(--color-red-600, #dc2626)", fontFamily: "var(--font-mono)",
    fontSize: 12, margin: 0,
  },
  recentSection: {
    marginTop: 16,
    borderTop: "1px solid var(--color-neutral-200)",
    paddingTop: 16,
  },
  recentHeading: {
    fontSize: 11, fontWeight: 600,
    letterSpacing: "0.08em", textTransform: "uppercase",
    color: "var(--color-neutral-500)", margin: "0 0 8px",
  },
  recentEmpty: { fontSize: 12, color: "var(--color-neutral-500)", margin: 0 },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px", background: "var(--color-neutral-100)",
    borderRadius: 3, margin: "0 2px",
  },
  recentList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 2,
  },
  recentItem: {
    display: "flex", justifyContent: "space-between", alignItems: "flex-start",
    padding: "8px 12px", borderRadius: 4, gap: 12,
    border: "1px solid transparent",
    transition: "background 80ms ease, border-color 80ms ease",
  },
  recentLeft: { display: "flex", flexDirection: "column", gap: 2, minWidth: 0, flex: 1 },
  recentRight: { display: "flex", alignItems: "center", gap: 8, flexShrink: 0 },
  recentName: { fontSize: 14, fontWeight: 500, color: "var(--color-neutral-900)" },
  recentPath: {
    fontSize: 11, color: "var(--color-neutral-500)",
    fontFamily: "var(--font-mono)",
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
  missingBadge: {
    fontSize: 10, color: "var(--color-red-600, #dc2626)",
    fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em",
    padding: "2px 6px",
    border: "1px solid var(--color-red-600, #dc2626)",
    borderRadius: 3,
  },
  removeBtn: {
    background: "transparent", color: "var(--color-neutral-500)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    fontSize: 11, fontWeight: 500, padding: "4px 10px",
    cursor: "pointer", fontFamily: "var(--font-ui)",
  },
};
