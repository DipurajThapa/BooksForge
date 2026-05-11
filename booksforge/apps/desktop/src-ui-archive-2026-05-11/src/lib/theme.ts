/**
 * Theme management — light, dark, system.
 *
 * Listens to `prefers-color-scheme` and sets the
 * `data-theme="light"|"dark"` attribute on `<html>` so the CSS tokens
 * in `packages/ui/src/tokens.css` resolve correctly.
 *
 * Closes the system-preference half of EXTERNAL_AUDIT_BACKLOG.md #35.
 * The user-facing toggle in Settings → Appearance is a separate
 * follow-up that touches `SettingsPanel.tsx` (currently in-flight) —
 * once that lands, the toggle just calls `setThemePreference(...)`.
 *
 * Persistence
 *   The user's preference is persisted in `localStorage` under the
 *   key `bf-theme-preference`.  Values: `"system" | "light" | "dark"`.
 *   Default on first launch: `"system"`.
 *
 * Privacy
 *   No remote sink.  `prefers-color-scheme` is read locally from the
 *   browser; localStorage is local to the app data directory.
 */

const STORAGE_KEY = "bf-theme-preference";
const DATA_ATTR = "data-theme";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

/**
 * Read the user's preference from localStorage.  Returns "system" if
 * unset or unparseable.
 */
export function getThemePreference(): ThemePreference {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw === "light" || raw === "dark" || raw === "system") return raw;
    return "system";
  } catch {
    return "system";
  }
}

/**
 * Persist the user's preference and immediately apply it.
 */
export function setThemePreference(pref: ThemePreference): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, pref);
  } catch {
    // localStorage can be unavailable in some sandboxed contexts; the
    // theme still applies for the current session.
  }
  applyTheme(resolveTheme(pref));
}

/**
 * Resolve a preference (which may be "system") to a concrete theme.
 */
export function resolveTheme(pref: ThemePreference): ResolvedTheme {
  if (pref === "light" || pref === "dark") return pref;
  if (typeof window === "undefined" || !window.matchMedia) return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/**
 * Set `<html data-theme="...">`.  Idempotent.
 */
export function applyTheme(theme: ResolvedTheme): void {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute(DATA_ATTR, theme);
}

/**
 * Initialise the theme system at app startup.  Reads the saved
 * preference, applies it, and (when preference is "system") attaches
 * a `matchMedia` change listener that re-applies on OS change.
 *
 * Returns a teardown function — call it on cleanup (e.g. test
 * lifecycle).
 */
export function initThemeSystem(): () => void {
  if (typeof window === "undefined") return () => undefined;

  const pref = getThemePreference();
  applyTheme(resolveTheme(pref));

  if (pref !== "system" || !window.matchMedia) return () => undefined;

  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const onChange = (e: MediaQueryListEvent): void => {
    // Only react if the user is still on "system".  If they flipped
    // to an explicit preference between init and now, don't override.
    if (getThemePreference() === "system") {
      applyTheme(e.matches ? "dark" : "light");
    }
  };

  // Older Safari uses `addListener`/`removeListener`.  TypeScript's
  // lib.dom only types `addEventListener` though, so cast at the
  // boundary.
  if (mq.addEventListener) {
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (mq as any).addListener?.(onChange);
  return () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mq as any).removeListener?.(onChange);
  };
}
