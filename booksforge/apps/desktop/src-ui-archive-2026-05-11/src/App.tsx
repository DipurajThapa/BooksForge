import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppVersion, OpenProjectResult, OllamaStatusResponse } from "@booksforge/shared-types";
import ProjectPicker from "./components/ProjectPicker";
import NewProjectWizard from "./components/NewProjectWizard";
import EditorShell from "./components/EditorShell";
import OllamaWizard from "./components/OllamaWizard";
import BookKindOverlay from "./components/BookKindOverlay";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastProvider } from "./components/ToastProvider";
import { ShortcutHelp } from "./components/ShortcutHelp";
import { useShortcut } from "./lib/keymap";
import { t } from "./lib/i18n";

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
  const [showShortcuts, setShowShortcuts] = useState(false);

  // `?` (or `mod+?` on macOS-style binding) opens the shortcut help.
  // The `app.show-shortcuts` binding is registered in lib/keymap.ts.
  useShortcut(
    "app.show-shortcuts",
    useCallback(() => setShowShortcuts(true), []),
  );

  useEffect(() => {
    invoke<AppVersion>("app_version").then(setVersion).catch(() => null);
    invoke<OllamaStatusResponse>("ollama_status")
      .then((status) => {
        setOllama(status);
        // Auto-open the OllamaWizard the first time we observe Ollama
        // is not running. This is the core onboarding gap the audit
        // surfaced: a non-developer who downloads BooksForge.app would
        // otherwise see a red status dot with no explanation. We only
        // fire once per session (sessionStorage flag) to avoid
        // re-prompting if the user dismisses it.
        if (!status.running && !sessionStorage.getItem("bf:ollama-wizard-shown")) {
          sessionStorage.setItem("bf:ollama-wizard-shown", "1");
          setShowOllamaWizard(true);
        }
      })
      .catch(() => {
        setOllama({ running: false, version: null });
        if (!sessionStorage.getItem("bf:ollama-wizard-shown")) {
          sessionStorage.setItem("bf:ollama-wizard-shown", "1");
          setShowOllamaWizard(true);
        }
      });
  }, []);

  function handleProjectOpened(result: OpenProjectResult) {
    setView({ tag: "editor", project: result });
  }

  // Phase 5C — onboarding overlay for projects opened with `book_kind = null`.
  // The user MUST pick before continuing (overlay is non-dismissible in
  // onboarding mode); the chosen kind is persisted via project_kind_set
  // and the open-project state is updated in-place so the editor sees
  // the new kind on its next render.
  if (view.tag === "editor") {
    const proj = view.project;
    const needsKind = proj.book_kind === null || proj.book_kind === undefined;
    return (
      <>
        <EditorShell
          project={proj}
          onClose={() => setView({ tag: "picker" })}
        />
        {needsKind && (
          <BookKindOverlay
            mode="onboarding"
            currentKind={null}
            onSaved={(newKind) => {
              setView({
                tag: "editor",
                project: { ...proj, book_kind: newKind },
              });
            }}
            onClose={() => undefined}
          />
        )}
      </>
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
      {showShortcuts && (
        <ShortcutHelp onClose={() => setShowShortcuts(false)} />
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
    ? t("ollama.status.checking")
    : ollama.running
    ? `${t("ollama.status.online")} ${ollama.version ?? ""}`.trim()
    : t("ollama.status.offline");
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
