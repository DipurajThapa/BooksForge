/**
 * Internationalisation scaffolding.
 *
 * MVP ships English-only — the audit's #36 calls for the *structure*
 * to be in place now so that adding a second language later is a
 * translation task, not a refactor.
 *
 * This module is a deliberately minimal stand-in for `react-i18next`
 * / `formatjs`: the same `t(key)` API surface, an in-memory dictionary
 * loaded at startup, no plural / interpolation logic until a real
 * translator drives the requirements.  When the team picks the final
 * library, this module's surface is the migration boundary.
 *
 * Closes EXTERNAL_AUDIT_BACKLOG.md #36 (in-MVP scaffolding form).
 *
 * Privacy
 *   Locale loading is filesystem-only (bundled JSON).  No remote
 *   translation service is contacted.  Any future translation sync
 *   that runs against a remote service must be opt-in per the privacy
 *   invariants in `outputs/SECURITY_PRIVACY.md`.
 */

import en from "../../locales/en.json";

/** Locale id (BCP-47 language subtag, e.g. "en", "fr", "ja-JP"). */
export type Locale = "en";

/** Translation dictionary — flat, dot-namespaced keys. */
type Dictionary = Record<string, string>;

const DICTIONARIES: Record<Locale, Dictionary> = { en };

let currentLocale: Locale = "en";

/**
 * Switch the active locale.  No-op until additional locales are
 * added to `locales/`.
 */
export function setLocale(locale: Locale): void {
  if (DICTIONARIES[locale]) {
    currentLocale = locale;
  }
}

/** Read the active locale. */
export function getLocale(): Locale {
  return currentLocale;
}

/**
 * Translate a key into the active locale.
 *
 *   t("app.title")  →  "BooksForge"
 *   t("missing")    →  "missing"   // returns key as fallback so a
 *                                    // missing translation is loud,
 *                                    // not silent.
 *
 * `params` is interpolated using `{name}` placeholders if present:
 *
 *   "Created {n} chapters" + { n: 3 }  →  "Created 3 chapters"
 *
 * For pluralisation / gender / RTL handling, switch to a real i18n
 * library — the API of this stub is intentionally a subset of the
 * eventual signature.
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const dict = DICTIONARIES[currentLocale];
  let value = dict?.[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      value = value.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
    }
  }
  return value;
}

/**
 * React hook for use inside components.  Today this is a thin wrapper
 * around `t()`; once we adopt `react-i18next` it will be the
 * `useTranslation()` shim.
 */
export function useT(): { t: typeof t; locale: Locale } {
  return { t, locale: currentLocale };
}
