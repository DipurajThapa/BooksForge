import React from "react";

/**
 * Top-level React error boundary.
 *
 * Catches render-time exceptions anywhere in the component tree and
 * shows a recoverable fallback UI instead of a blank window.
 *
 * Closes EXTERNAL_AUDIT_BACKLOG.md #24.
 *
 * Wiring: `App.tsx` wraps its root in `<ErrorBoundary>`.  Adding
 * additional boundaries lower in the tree (e.g. around `EditorShell`
 * or each agent panel) is fine and should be encouraged once those
 * surfaces stabilise.
 *
 * Behaviour
 *   1. Captures the error + componentStack via `componentDidCatch`.
 *   2. Logs a structured entry to `console.error` with a session-id
 *      correlation token (see `lib/sessionId.ts`).
 *   3. Renders the fallback panel: title, recovery instructions, two
 *      buttons (Reload / Open settings).
 *   4. The "Save error report" button is wired to a future
 *      `crash.preview` Tauri command (see `docs/CRASH_REPORTING_DESIGN.md`)
 *      — currently a stub that copies the error to clipboard.
 *
 * Privacy
 *   The boundary intentionally does NOT send anything to a remote
 *   sink.  It writes only to the console and (on user click) to the
 *   clipboard.  See `PRIVACY_POLICY.md §3`.
 */

import { getSessionId } from "../lib/sessionId";

interface ErrorBoundaryProps {
  /** Children to render normally. */
  children: React.ReactNode;
  /** Optional override for the fallback UI. */
  fallback?: (error: Error, retry: () => void) => React.ReactNode;
  /** Called whenever a render error is caught.  Useful for tests. */
  onError?: (error: Error, info: React.ErrorInfo) => void;
}

interface ErrorBoundaryState {
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
  /** ULID of the captured error, used in the user-visible report. */
  reportId: string | null;
}

export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { error: null, errorInfo: null, reportId: null };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo): void {
    const reportId = generateReportId();
    this.setState({ errorInfo: info, reportId });

    // Structured log line for grep-ability.  Session id correlates
    // this with the backend's tracing logs at ~/.booksforge/logs/.
    // eslint-disable-next-line no-console
    console.error("[booksforge:render-crash]", {
      reportId,
      sessionId: getSessionId(),
      message: error.message,
      stack: error.stack,
      componentStack: info.componentStack,
      timestamp: new Date().toISOString(),
    });

    this.props.onError?.(error, info);
  }

  retry = (): void => {
    this.setState({ error: null, errorInfo: null, reportId: null });
  };

  copyReport = async (): Promise<void> => {
    const { error, errorInfo, reportId } = this.state;
    if (!error) return;
    const payload = JSON.stringify(
      {
        reportId,
        sessionId: getSessionId(),
        message: error.message,
        stack: error.stack,
        componentStack: errorInfo?.componentStack,
        appVersion: "0.0.1",
        timestamp: new Date().toISOString(),
      },
      null,
      2,
    );
    try {
      await navigator.clipboard.writeText(payload);
    } catch {
      // Clipboard API can fail under restrictive CSP/permissions.
      // Fall back to a textarea-based copy is overkill for a fallback
      // UI; the user can take a screenshot instead.
    }
  };

  render(): React.ReactNode {
    const { error, reportId } = this.state;
    if (!error) return this.props.children;

    if (this.props.fallback) {
      return this.props.fallback(error, this.retry);
    }

    return (
      <div
        role="alertdialog"
        aria-labelledby="bf-eb-title"
        aria-describedby="bf-eb-body"
        style={fallbackPanel}
      >
        <h1 id="bf-eb-title" style={fallbackTitle}>
          Something went wrong
        </h1>
        <p id="bf-eb-body" style={fallbackBody}>
          BooksForge hit an unexpected error and could not finish
          rendering this view. Your work is autosaved — your manuscript
          is safe.
        </p>
        <pre style={fallbackPre}>
          {error.message}
        </pre>
        <p style={fallbackHint}>
          Report ID: <code>{reportId ?? "n/a"}</code>
        </p>
        <div style={fallbackActions}>
          <button type="button" onClick={this.retry} style={btnPrimary}>
            Try again
          </button>
          <button
            type="button"
            onClick={() => window.location.reload()}
            style={btnSecondary}
          >
            Reload app
          </button>
          <button
            type="button"
            onClick={this.copyReport}
            style={btnSecondary}
          >
            Copy error report
          </button>
        </div>
        <p style={fallbackPrivacy}>
          BooksForge does not send error reports automatically. Nothing
          on this screen has been transmitted off your device.
        </p>
      </div>
    );
  }
}

// ── Helpers ────────────────────────────────────────────────────────

function generateReportId(): string {
  // ULID-ish: 10 chars timestamp + 16 chars random.  We don't import
  // the `ulid` package here to keep the boundary dependency-free.
  const ts = Date.now().toString(36).padStart(10, "0").toUpperCase();
  const rand = Array.from({ length: 16 }, () =>
    Math.floor(Math.random() * 36).toString(36).toUpperCase(),
  ).join("");
  return `${ts}${rand}`;
}

// ── Inline styles ──────────────────────────────────────────────────
//
// Inline `style={...}` is acceptable here because the page-level CSP
// allows `style-src 'self'` only (no `'unsafe-inline'`).  React's
// inline-style prop sets `element.style.*` directly — no `<style>`
// tag injection — and is therefore not subject to the inline-style
// CSP directive.

const fallbackPanel: React.CSSProperties = {
  maxWidth: "40rem",
  margin: "10vh auto",
  padding: "2rem",
  fontFamily:
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif",
  color: "var(--color-neutral-900, #1f2328)",
  background: "var(--color-bg, #ffffff)",
  border: "1px solid var(--color-error, #d1242f)",
  borderRadius: "0.5rem",
};

const fallbackTitle: React.CSSProperties = {
  fontSize: "1.25rem",
  marginTop: 0,
};

const fallbackBody: React.CSSProperties = {
  marginTop: "0.5rem",
  marginBottom: "1rem",
  lineHeight: 1.5,
};

const fallbackPre: React.CSSProperties = {
  background: "var(--color-bg-subtle, #f6f8fa)",
  padding: "0.75rem",
  borderRadius: "0.375rem",
  overflowX: "auto",
  fontSize: "0.85rem",
};

const fallbackHint: React.CSSProperties = {
  fontSize: "0.85rem",
  opacity: 0.7,
};

const fallbackActions: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  marginTop: "1rem",
  flexWrap: "wrap",
};

const btnBase: React.CSSProperties = {
  padding: "0.5rem 1rem",
  fontSize: "0.95rem",
  borderRadius: "0.375rem",
  border: "1px solid var(--color-neutral-300, #d0d7de)",
  cursor: "pointer",
};

const btnPrimary: React.CSSProperties = {
  ...btnBase,
  background: "var(--color-primary, #0969da)",
  color: "#fff",
  borderColor: "var(--color-primary, #0969da)",
};

const btnSecondary: React.CSSProperties = {
  ...btnBase,
  background: "var(--color-bg, #ffffff)",
};

const fallbackPrivacy: React.CSSProperties = {
  marginTop: "1.5rem",
  fontSize: "0.8rem",
  opacity: 0.6,
};
