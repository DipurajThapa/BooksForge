/**
 * App-mode state — the single biggest UX axis in BooksForge.
 *
 * Modes:
 *   - "manual"    — writer types prose themselves; AI assistance is
 *                   passive (validators, vocab, autocomplete). No
 *                   agent runs unless the writer explicitly opens a
 *                   panel from the ⋯ menu.
 *   - "ai_writer" — writer is the editor; agents draft, writer
 *                   reviews/refines. Empty scenes show a "Generate
 *                   this scene" CTA. The toolbar's primary action is
 *                   "Generate Book" / "Generate scene" / "Refine".
 *
 * Stored in localStorage today; migrating to `~/.booksforge/settings.toml`
 * when the Rust UiSettings struct grows the field. localStorage is
 * acceptable transitionally because (a) it's per-machine same as the
 * settings file, and (b) the default is sensible (`null` → first-open
 * picker prompts the writer).
 *
 * The `null` state is meaningful: it means "user hasn't picked yet,
 * show the first-time picker overlay." Distinct from "manual."
 */

export type AppMode = "manual" | "ai_writer";

const KEY = "bf:app-mode";

/** Read the current mode. Returns `null` when the writer hasn't picked
 *  yet — the App should surface the mode picker overlay in that case. */
export function loadAppMode(): AppMode | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(KEY);
  if (raw === "manual" || raw === "ai_writer") return raw;
  return null;
}

/** Persist the chosen mode. Idempotent. */
export function setAppMode(mode: AppMode): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(KEY, mode);
}

/** Clear the recorded choice (re-prompts the writer on next project open). */
export function clearAppMode(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(KEY);
}

/** Human-readable label for the mode pill. */
export function modeLabel(mode: AppMode): string {
  return mode === "ai_writer" ? "AI Writer" : "Manual";
}

/** Emoji prefix used in the pill so the mode is recognisable at a glance. */
export function modeEmoji(mode: AppMode): string {
  return mode === "ai_writer" ? "🤖" : "📝";
}

/** One-sentence explanation surfaced in the picker overlay. */
export function modeBlurb(mode: AppMode): string {
  return mode === "ai_writer"
    ? "BooksForge drafts every chapter and scene with local LLMs (qwen3.5/3.6 via Ollama). You review, accept, refine. Right for: a fast first draft you'll edit."
    : "You write the prose. AI assistance is passive — vocabulary, validators, formatting. No agent runs unless you open a panel. Right for: writers with strong voice who want help with mechanics, not generation.";
}
