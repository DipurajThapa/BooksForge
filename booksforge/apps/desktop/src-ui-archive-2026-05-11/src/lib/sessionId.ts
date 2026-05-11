/**
 * Per-session correlation ID for log entries.
 *
 * Generated once at module load and exposed via `getSessionId()`.
 * The same value flows into:
 *   - Console error logs from `ErrorBoundary`.
 *   - Future Tauri command logs (the `invoke` wrapper can attach this
 *     as a header so the Rust `tracing` span carries it through to
 *     `~/.booksforge/logs/`).
 *   - The "Help → About" dialog so users can paste it into a bug
 *     report.
 *
 * Privacy note: the session id is generated in-process and is **not**
 * persisted across launches.  It is therefore not a tracker — a fresh
 * id is produced on every app start.  See `PRIVACY_POLICY.md §1.1`
 * which lists this behaviour as part of the "stays on the device"
 * data set.
 *
 * Closes EXTERNAL_AUDIT_BACKLOG.md #57 (frontend session-id logging).
 */

/**
 * The id is computed once and frozen.  Tests that need a specific
 * value can call `__resetSessionIdForTests` (only available when
 * `import.meta.env.MODE !== "production"`).
 */
let sessionId: string = generateSessionId();

/**
 * ULID-ish: 10 chars Crockford-base32-ish timestamp + 16 chars
 * random.  We don't pull in the `ulid` npm package because this is
 * the only consumer in MVP — a simple monotonic-by-time string is
 * enough.
 */
function generateSessionId(): string {
  const ts = Date.now().toString(36).padStart(10, "0").toUpperCase();
  const rand = Array.from({ length: 16 }, () =>
    Math.floor(Math.random() * 36)
      .toString(36)
      .toUpperCase(),
  ).join("");
  return `${ts}${rand}`;
}

/** Return the current session id. */
export function getSessionId(): string {
  return sessionId;
}

/** **Test-only.**  Replace the session id; throws in production. */
export function __resetSessionIdForTests(value?: string): void {
  if (import.meta.env.MODE === "production") {
    throw new Error("__resetSessionIdForTests is not available in production");
  }
  sessionId = value ?? generateSessionId();
}
