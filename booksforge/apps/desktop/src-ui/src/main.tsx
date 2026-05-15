import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initThemeSystem } from "./lib/theme";
import "../../../../packages/ui/src/tokens.css";

// Apply the user's saved theme preference (or system default) before
// the first paint so there's no light → dark flash on launch.  The
// returned teardown is intentionally not stored: the listener lives
// for the app's lifetime.
initThemeSystem();

// Tauri-environment guard.
//
// The frontend uses `invoke()` from `@tauri-apps/api/core`, which reads
// `window.__TAURI_INTERNALS__.invoke` — only injected by the Tauri
// WebView preload. If the page is opened in a plain browser tab
// (e.g., http://localhost:5173), the first IPC call throws
// "Cannot read properties of undefined (reading 'invoke')" with no
// indication that the cause is "wrong window."  Detect that up front
// and render a clear message instead of letting the app crash on the
// first stage transition.
const isInTauri = typeof window !== "undefined"
  && Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const rootEl = document.getElementById("root")!;

if (!isInTauri) {
  // We're in a plain browser. Render a static fallback — no IPC, no
  // React routes (they all assume Tauri). Use raw HTML to avoid any
  // dependency on the rest of the UI bundle.
  rootEl.innerHTML = `
    <div style="
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 48px 24px;
      font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
      color: #1f2937;
      background: #fafaf9;
      text-align: center;
    ">
      <div style="max-width: 520px;">
        <h1 style="
          font-family: Georgia, serif;
          font-size: 28px;
          font-weight: 700;
          margin: 0 0 12px;
          color: #111827;
        ">BooksForge runs in the desktop shell</h1>
        <p style="margin: 0 0 16px; font-size: 15px; line-height: 1.6; color: #4b5563;">
          This page is the Vite dev URL. The app's IPC layer
          (<code style="background:#f3f4f6;padding:2px 6px;border-radius:4px;font-size:13px;">invoke</code>)
          is only available inside the Tauri WebView window.
        </p>
        <p style="margin: 0 0 24px; font-size: 14px; line-height: 1.6; color: #4b5563;">
          Look for a separate window titled
          <strong style="color:#111827;">"BooksForge"</strong>
          launched by <code style="background:#f3f4f6;padding:2px 6px;border-radius:4px;font-size:13px;">cargo tauri dev</code>.
          If you don't see one, the Rust build may still be running — check the terminal that started the dev command.
        </p>
        <p style="margin: 0; font-size: 12px; color: #9ca3af;">
          Closing this tab is safe; the desktop window is independent.
        </p>
      </div>
    </div>
  `;
} else {
  ReactDOM.createRoot(rootEl).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
