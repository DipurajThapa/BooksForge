import React, { createContext, useCallback, useContext, useMemo, useState } from "react";

/**
 * Application-wide toast queue.
 *
 * Replaces the per-component toast state and the `.catch(() => null)`
 * silent-error swallowing pattern that EXTERNAL_AUDIT_BACKLOG.md #25
 * called out.  Now any component (or IPC error handler) can call
 * `useToast().push(...)` and surface a real, dismissable message.
 *
 * Wiring: `App.tsx` wraps its root in `<ToastProvider>`; existing
 * components that previously did `.catch(() => null)` should migrate
 * incrementally (e.g. EditorShell, NewProjectWizard) — that migration
 * is in-flight team work and is NOT done in this commit.
 *
 * Accessibility
 *   - The toast region uses `role="status"` for transient
 *     informational toasts and `role="alert"` for errors.
 *   - Toasts auto-dismiss after `durationMs` (default 6 s for info,
 *     0 = persistent for errors).
 *   - The dismiss button is keyboard-focusable and announces "Dismiss
 *     <severity> toast".
 *
 * Privacy
 *   Toasts render only the message text the caller provides; nothing
 *   is logged or transmitted off-device.
 */

export type ToastSeverity = "info" | "success" | "warning" | "error";

export interface ToastDescriptor {
  /** Stable id; use the same id to update an existing toast. */
  id?: string;
  severity?: ToastSeverity;
  title?: string;
  body: string;
  /**
   * Auto-dismiss timeout in ms.  Default 6 s for info/success/warning;
   * `0` (persistent) for error.  Pass `0` explicitly to opt out of
   * auto-dismiss for any severity.
   */
  durationMs?: number;
  /** Optional action button. */
  action?: { label: string; onClick: () => void };
}

interface InternalToast extends Required<Omit<ToastDescriptor, "action" | "title">> {
  title: string | undefined;
  action: ToastDescriptor["action"];
  createdAt: number;
}

interface ToastContextValue {
  push: (toast: ToastDescriptor) => string;
  dismiss: (id: string) => void;
  clearAll: () => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    // Defensive: if a component calls useToast() outside the
    // provider, fall back to console so we don't crash the render.
    return {
      push: (t) => {
        // eslint-disable-next-line no-console
        console.warn("[booksforge:toast outside-provider]", t);
        return "no-op";
      },
      dismiss: () => undefined,
      clearAll: () => undefined,
    };
  }
  return ctx;
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<InternalToast[]>([]);

  const dismiss = useCallback((id: string) => {
    setToasts((current) => current.filter((t) => t.id !== id));
  }, []);

  const push = useCallback(
    (descriptor: ToastDescriptor): string => {
      const id =
        descriptor.id ??
        `toast-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
      const severity: ToastSeverity = descriptor.severity ?? "info";
      const durationMs =
        descriptor.durationMs ??
        (severity === "error" ? 0 : 6000); // errors persist by default
      const next: InternalToast = {
        id,
        severity,
        title: descriptor.title,
        body: descriptor.body,
        durationMs,
        action: descriptor.action,
        createdAt: Date.now(),
      };
      setToasts((current) => {
        const filtered = current.filter((t) => t.id !== id);
        return [...filtered, next];
      });
      if (durationMs > 0) {
        window.setTimeout(() => dismiss(id), durationMs);
      }
      return id;
    },
    [dismiss],
  );

  const clearAll = useCallback(() => setToasts([]), []);

  const value = useMemo(
    () => ({ push, dismiss, clearAll }),
    [push, dismiss, clearAll],
  );

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div
        aria-live="polite"
        aria-atomic="false"
        style={containerStyle}
      >
        {toasts.map((t) => (
          <ToastItem key={t.id} toast={t} onDismiss={dismiss} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: InternalToast;
  onDismiss: (id: string) => void;
}) {
  const role = toast.severity === "error" ? "alert" : "status";
  return (
    <div role={role} style={toastStyle(toast.severity)}>
      <div style={toastBody}>
        {toast.title && <div style={toastTitle}>{toast.title}</div>}
        <div>{toast.body}</div>
        {toast.action && (
          <button
            type="button"
            onClick={toast.action.onClick}
            style={toastActionBtn}
          >
            {toast.action.label}
          </button>
        )}
      </div>
      <button
        type="button"
        aria-label={`Dismiss ${toast.severity} toast`}
        onClick={() => onDismiss(toast.id)}
        style={toastDismissBtn}
      >
        ×
      </button>
    </div>
  );
}

// ── Inline styles (CSP-friendly: React inline styles set
//    element.style.* directly and are NOT subject to style-src) ──

const containerStyle: React.CSSProperties = {
  position: "fixed",
  bottom: "1rem",
  right: "1rem",
  display: "flex",
  flexDirection: "column",
  gap: "0.5rem",
  maxWidth: "24rem",
  zIndex: 9999,
  pointerEvents: "none",
};

function toastStyle(severity: ToastSeverity): React.CSSProperties {
  const palette: Record<ToastSeverity, { bg: string; fg: string; border: string }> = {
    info: {
      bg: "var(--color-bg, #ffffff)",
      fg: "var(--color-neutral-900, #1f2328)",
      border: "var(--color-neutral-300, #d0d7de)",
    },
    success: {
      bg: "var(--color-success-bg, #dcfce7)",
      fg: "var(--color-success-fg, #166534)",
      border: "var(--color-success, #22c55e)",
    },
    warning: {
      bg: "var(--color-warning-bg, #fef3c7)",
      fg: "var(--color-warning-fg, #854d0e)",
      border: "var(--color-warning, #ca8a04)",
    },
    error: {
      bg: "var(--color-error-bg, #fee2e2)",
      fg: "var(--color-error-fg, #991b1b)",
      border: "var(--color-error, #d1242f)",
    },
  };
  const c = palette[severity];
  return {
    background: c.bg,
    color: c.fg,
    border: `1px solid ${c.border}`,
    borderRadius: "0.5rem",
    padding: "0.75rem 1rem",
    boxShadow: "0 4px 12px rgba(0,0,0,0.08)",
    display: "flex",
    alignItems: "flex-start",
    gap: "0.5rem",
    pointerEvents: "auto",
  };
}

const toastBody: React.CSSProperties = { flex: 1, fontSize: "0.9rem", lineHeight: 1.4 };
const toastTitle: React.CSSProperties = { fontWeight: 600, marginBottom: "0.25rem" };
const toastActionBtn: React.CSSProperties = {
  marginTop: "0.5rem",
  padding: "0.25rem 0.5rem",
  background: "transparent",
  color: "inherit",
  border: "1px solid currentColor",
  borderRadius: "0.25rem",
  fontSize: "0.85rem",
  cursor: "pointer",
};
const toastDismissBtn: React.CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: 0,
  fontSize: "1.25rem",
  cursor: "pointer",
  padding: "0 0.25rem",
  lineHeight: 1,
};
