/**
 * BooksForge — application root (2026-05 redesign).
 *
 * Three top-level views, switched by a single `view` state. No Tauri
 * IPC happens at this layer — each route owns its own data fetching.
 *
 *   picker → user clicks "New" → wizard → on create → editor
 *   picker → user opens a recent → editor
 *   editor → Close → picker
 *
 * Stage state (which of the 6 MVP stages a project is on) lives in
 * project-scope memory (`book:stage_state`), not in this component.
 * The editor's StageRail reads it; the wizard writes it on completion.
 */
import { useState } from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastProvider } from "./components/ToastProvider";
import ProjectPicker from "./routes/ProjectPicker";
import BookSetupWizard from "./routes/BookSetupWizard";
import EditorShell from "./routes/EditorShell";

type View =
  | { tag: "picker" }
  | { tag: "wizard" }
  | { tag: "editor"; project: OpenProjectResult };

export default function App() {
  // ErrorBoundary catches any render-time exception in the routes;
  // ToastProvider exposes a global queue via `useToast()` so route
  // components don't need `.catch(() => null)` to swallow IPC errors.
  return (
    <ErrorBoundary>
      <ToastProvider>
        <AppContent />
      </ToastProvider>
    </ErrorBoundary>
  );
}

function AppContent() {
  const [view, setView] = useState<View>({ tag: "picker" });

  if (view.tag === "wizard") {
    return (
      <BookSetupWizard
        onCreated={(project) => setView({ tag: "editor", project })}
        onCancel={() => setView({ tag: "picker" })}
      />
    );
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
    <ProjectPicker
      onProjectOpened={(project) => setView({ tag: "editor", project })}
      onNewProject={() => setView({ tag: "wizard" })}
    />
  );
}
