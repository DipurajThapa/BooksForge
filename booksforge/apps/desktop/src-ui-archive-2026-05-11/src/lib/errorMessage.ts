/**
 * Format any thrown value (Error, Tauri IPC error object, string, primitive)
 * into a human-readable single-line string suitable for inline UI display.
 *
 * Why this exists: `String(e)` returns "[object Object]" for any plain object
 * that lacks a `toString` override, which is exactly the shape Tauri IPC
 * surfaces for `BooksForgeError` and similar structured errors. Using
 * `String(e)` directly means users see "[object Object]" instead of the
 * actual message — the bug seen on 2026-05-08 in NewProjectWizard.
 *
 * The function preserves backward-compatible behaviour for Error / string /
 * primitive inputs and never throws.
 */
export function errorMessage(e: unknown): string {
  if (e == null) return "Unknown error.";
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message || e.name || String(e);
  if (typeof e === "object") {
    const obj = e as Record<string, unknown>;
    // Tauri IPC errors typically have { kind, message } or { code, message }.
    if (typeof obj.message === "string" && obj.message.length > 0) {
      return obj.message;
    }
    // Some commands return { kind: "ValidationError", details: "..." }.
    if (typeof obj.details === "string" && obj.details.length > 0) {
      return obj.details;
    }
    if (typeof obj.error === "string" && obj.error.length > 0) {
      return obj.error;
    }
    // Last resort: JSON-stringify so users see something meaningful instead
    // of "[object Object]". Truncated to keep the inline error readable.
    try {
      const json = JSON.stringify(obj);
      return json.length > 240 ? `${json.slice(0, 240)}…` : json;
    } catch {
      return "Unknown error (non-serializable).";
    }
  }
  return String(e);
}
