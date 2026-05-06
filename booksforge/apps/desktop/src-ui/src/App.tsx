import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppVersion, OpenProjectResult, OllamaStatusResponse } from "@booksforge/shared-types";
import ProjectPicker from "./components/ProjectPicker";
import NewProjectWizard from "./components/NewProjectWizard";

type AppView =
  | { tag: "picker" }
  | { tag: "new-project" }
  | { tag: "editor"; project: OpenProjectResult };

export default function App() {
  const [view, setView] = useState<AppView>({ tag: "picker" });
  const [ollama, setOllama] = useState<OllamaStatusResponse | null>(null);
  const [version, setVersion] = useState<AppVersion | null>(null);

  useEffect(() => {
    invoke<AppVersion>("app_version").then(setVersion).catch(() => null);
    invoke<OllamaStatusResponse>("ollama_status")
      .then(setOllama)
      .catch(() => setOllama({ running: false, version: null }));
  }, []);

  function handleProjectOpened(result: OpenProjectResult) {
    setView({ tag: "editor", project: result });
  }

  if (view.tag === "picker") {
    return (
      <>
        {ollama && <OllamaBar status={ollama} version={version} />}
        <ProjectPicker
          onProjectOpened={handleProjectOpened}
          onNewProject={() => setView({ tag: "new-project" })}
        />
        {view.tag === "picker" && false /* wizard is modal, rendered separately */}
      </>
    );
  }

  if (view.tag === "new-project") {
    return (
      <>
        {ollama && <OllamaBar status={ollama} version={version} />}
        <ProjectPicker
          onProjectOpened={handleProjectOpened}
          onNewProject={() => setView({ tag: "new-project" })}
        />
        <NewProjectWizard
          onCreated={handleProjectOpened}
          onCancel={() => setView({ tag: "picker" })}
        />
      </>
    );
  }

  // editor view
  const project = view.project;
  return (
    <div style={s.editorRoot}>
      <header style={s.editorHeader}>
        <span style={s.wordmark}>BooksForge</span>
        <span style={s.projectTitle}>{project.title}</span>
        <div style={s.headerRight}>
          <OllamaIndicator status={ollama} />
          <button
            style={s.closeProjectBtn}
            onClick={() => {
              invoke("project_close").catch(() => null);
              setView({ tag: "picker" });
            }}
          >
            Close
          </button>
        </div>
      </header>
      <main style={s.editorMain}>
        <p style={s.placeholderText}>
          Editor coming in MZ-03. Project: <strong>{project.title}</strong> by{" "}
          {project.author}
        </p>
        {version && (
          <p style={s.versionTag}>
            v{version.major}.{version.minor}.{version.patch}
          </p>
        )}
      </main>
    </div>
  );
}

function OllamaBar({
  status,
  version,
}: {
  status: OllamaStatusResponse | null;
  version: AppVersion | null;
}) {
  return (
    <div style={s.ollamaBar}>
      <OllamaIndicator status={status} />
      {version && (
        <span style={s.versionInline}>
          v{version.major}.{version.minor}.{version.patch}
        </span>
      )}
    </div>
  );
}

function OllamaIndicator({ status }: { status: OllamaStatusResponse | null }) {
  const color = !status
    ? "var(--color-neutral-400)"
    : status.running
    ? "var(--color-success)"
    : "var(--color-error)";
  const title = !status
    ? "Checking Ollama…"
    : status.running
    ? `Ollama ${status.version ?? "running"}`
    : "Ollama offline — run `ollama serve`";
  return <span title={title} style={{ ...s.dot, background: color }} />;
}

const s: Record<string, React.CSSProperties> = {
  ollamaBar: {
    position: "fixed",
    top: 12,
    right: 16,
    display: "flex",
    alignItems: "center",
    gap: "var(--space-2)",
    zIndex: 200,
  },
  versionInline: {
    fontSize: 11,
    color: "var(--color-text-tertiary)",
    fontFamily: "var(--font-mono)",
  },
  dot: {
    width: 8,
    height: 8,
    borderRadius: "50%",
    display: "inline-block",
    flexShrink: 0,
  },
  editorRoot: {
    minHeight: "100vh",
    display: "flex",
    flexDirection: "column",
    background: "var(--color-surface)",
  },
  editorHeader: {
    height: 48,
    borderBottom: "1px solid var(--color-border)",
    display: "flex",
    alignItems: "center",
    padding: "0 var(--space-4)",
    gap: "var(--space-4)",
  },
  wordmark: {
    fontFamily: "var(--font-prose)",
    fontSize: 16,
    fontWeight: 700,
    color: "var(--color-amber-600)",
    letterSpacing: "-0.01em",
    flexShrink: 0,
  },
  projectTitle: {
    flex: 1,
    fontSize: 14,
    fontWeight: 500,
    color: "var(--color-text-primary)",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  headerRight: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-3)",
    flexShrink: 0,
  },
  closeProjectBtn: {
    background: "none",
    border: "1px solid var(--color-border)",
    borderRadius: 4,
    fontSize: 12,
    color: "var(--color-text-secondary)",
    padding: "2px var(--space-2)",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  editorMain: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: "var(--space-2)",
  },
  placeholderText: {
    color: "var(--color-text-secondary)",
    fontSize: 14,
    textAlign: "center",
  },
  versionTag: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-text-tertiary)",
  },
};
