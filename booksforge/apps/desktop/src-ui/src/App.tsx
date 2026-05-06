import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppVersion, OpenProjectResult, OllamaStatusResponse } from "@booksforge/shared-types";
import ProjectPicker from "./components/ProjectPicker";
import NewProjectWizard from "./components/NewProjectWizard";
import EditorShell from "./components/EditorShell";

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

  if (view.tag === "editor") {
    return (
      <EditorShell
        project={view.project}
        onClose={() => setView({ tag: "picker" })}
      />
    );
  }

  return (
    <>
      <OllamaStatusBar ollama={ollama} version={version} />
      <ProjectPicker
        onProjectOpened={handleProjectOpened}
        onNewProject={() => setView({ tag: "new-project" })}
      />
      {view.tag === "new-project" && (
        <NewProjectWizard
          onCreated={handleProjectOpened}
          onCancel={() => setView({ tag: "picker" })}
        />
      )}
    </>
  );
}

function OllamaStatusBar({
  ollama,
  version,
}: {
  ollama: OllamaStatusResponse | null;
  version: AppVersion | null;
}) {
  if (!ollama && !version) return null;
  const dotColor = !ollama
    ? "var(--color-neutral-400)"
    : ollama.running
    ? "var(--color-success)"
    : "var(--color-error)";
  const title = !ollama
    ? "Checking…"
    : ollama.running
    ? `Ollama ${ollama.version ?? "running"}`
    : "Ollama offline";
  return (
    <div style={s.bar}>
      <span style={{ ...s.dot, background: dotColor }} title={title} />
      {version && (
        <span style={s.ver}>
          v{version.major}.{version.minor}.{version.patch}
        </span>
      )}
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  bar: {
    position: "fixed",
    top: 12,
    right: 16,
    display: "flex",
    alignItems: "center",
    gap: "var(--space-2)",
    zIndex: 200,
    pointerEvents: "none",
  },
  dot: {
    width: 8,
    height: 8,
    borderRadius: "50%",
    display: "inline-block",
  },
  ver: {
    fontSize: 11,
    color: "var(--color-text-tertiary)",
    fontFamily: "var(--font-mono)",
  },
};
