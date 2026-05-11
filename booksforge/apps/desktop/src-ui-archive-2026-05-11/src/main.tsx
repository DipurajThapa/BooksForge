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

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
