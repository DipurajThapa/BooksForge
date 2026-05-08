import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppVersion, OpenProjectResult, OllamaStatusResponse } from "@booksforge/shared-types";
import ProjectPicker from "./components/ProjectPicker";
import NewProjectWizard from "./components/NewProjectWizard";
import EditorShell from "./components/EditorShell";
import OllamaWizard from "./components/OllamaWizard";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastProvider } from "./components/ToastProvider";

type AppView =
  | { tag: "picker" }
  | { tag: "new-project" }
  | { tag: "editor"; project: OpenProjectResult };

export default function App() {
  // Top-level providers wrap the whole tree:
  //   - ErrorBoundary catches any render-time exception below.  See
  //     EXTERNAL_AUDIT_BACKLOG.md #24.
  //   - ToastProvider exposes a global queue via `useToast()` so
  //     components no longer need `.catch(() => null)` to swallow
  //     errors silently.  See EXTERNAL_AUDIT_BACKLOG.md #25.
  return (
    <ErrorBoundary>
      <ToastProvider>
        <AppContent />
      </ToastProvider>
    </ErrorBoundary>
  );
}

function AppContent() {
  const [view, setView] = useState<AppView>({ tag: "picker" });
  const [ollama, setOllama] = useState<OllamaStatusResponse | null>(null);
  const [version, setVersion] = useState<AppVersion | null>(null);
  const [showOllamaWizard, setShowOllamaWizard] = useState(false);

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
      <OllamaStatusBar
        ollama={ollama}
        version={version}
        onOpenWizard={() => setShowOllamaWizard(true)}
      />
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
      {showOllamaWizard && (
        <OllamaWizard
          onClose={() => setShowOllamaWizard(false)}
          onComplete={(modelId) => {
            setShowOllamaWizard(false);
            // Refresh Ollama status after wizard completes.
            invoke<OllamaStatusResponse>("ollama_status")
              .then(setOllama)
              .catch(() => null);
            void modelId; // model stored in project settings in MZ-05+
          }}
        />
      )}
    </>
  );
}

function OllamaStatusBar({
  ollama,
  version,
  onOpenWizard,
}: {
  ollama: OllamaStatusResponse | null;
  version: AppVersion | null;
  onOpenWizard: () => void;
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
    ? `Ollama ${ollama.version ?? "running"} — click to manage`
    : "Ollama offline — click to set up";
  return (
    <div style={s.bar}>
      <button
        style={{ ...s.dotBtn, background: "none", border: "none", cursor: "pointer", padding: 0 }}
        onClick={onOpenWizard}
        title={title}
        aria-label={title}
      >
        <span style={{ ...s.dot, background: dotColor }} />
      </button>
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
  dotBtn: {
    display: "flex",
    alignItems: "center",
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
