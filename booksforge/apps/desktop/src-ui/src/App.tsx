import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppVersion, OllamaStatusResponse } from "@booksforge/shared-types";

export default function App() {
  const [version, setVersion] = useState<AppVersion | null>(null);
  const [ollama, setOllama] = useState<OllamaStatusResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppVersion>("app_version")
      .then(setVersion)
      .catch((e: unknown) => setError(String(e)));

    invoke<OllamaStatusResponse>("ollama_status")
      .then(setOllama)
      .catch(() => setOllama({ running: false, version: null }));
  }, []);

  return (
    <div style={styles.root}>
      <header style={styles.header}>
        <span style={styles.wordmark}>BooksForge</span>
        <div style={styles.statusRow}>
          <OllamaIndicator status={ollama} />
        </div>
      </header>

      <main style={styles.main}>
        {error ? (
          <p style={styles.error}>IPC error: {error}</p>
        ) : version ? (
          <p style={styles.versionTag}>
            App version: {version.major}.{version.minor}.{version.patch}
            {version.pre ? `-${version.pre}` : ""}
          </p>
        ) : (
          <p style={styles.loading}>Loading…</p>
        )}
      </main>
    </div>
  );
}

function OllamaIndicator({ status }: { status: OllamaStatusResponse | null }) {
  if (!status) return <span style={{ ...styles.dot, background: "var(--color-neutral-400)" }} />;
  return (
    <span
      title={
        status.running
          ? `Ollama ${status.version ?? "running"}`
          : "Ollama offline — run `ollama serve`"
      }
      style={{
        ...styles.dot,
        background: status.running ? "var(--color-success)" : "var(--color-error)",
      }}
    />
  );
}

const styles: Record<string, React.CSSProperties> = {
  root: {
    fontFamily: "var(--font-ui)",
    background: "var(--color-surface)",
    color: "var(--color-text-primary)",
    minHeight: "100vh",
    display: "flex",
    flexDirection: "column",
  },
  header: {
    height: 48,
    borderBottom: "1px solid var(--color-border)",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "0 var(--space-4)",
  },
  wordmark: {
    fontFamily: "var(--font-prose)",
    fontSize: 18,
    fontWeight: 600,
    color: "var(--color-amber-600)",
    letterSpacing: "-0.01em",
  },
  statusRow: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-2)",
  },
  dot: {
    width: 8,
    height: 8,
    borderRadius: "50%",
    display: "inline-block",
  },
  main: {
    flex: 1,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  },
  versionTag: {
    fontFamily: "var(--font-mono)",
    fontSize: 13,
    color: "var(--color-text-secondary)",
  },
  loading: {
    color: "var(--color-text-tertiary)",
    fontSize: 14,
  },
  error: {
    color: "var(--color-error)",
    fontFamily: "var(--font-mono)",
    fontSize: 13,
  },
};
