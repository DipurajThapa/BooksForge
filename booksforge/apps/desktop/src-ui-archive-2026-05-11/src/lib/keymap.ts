/**
 * Centralised keyboard-shortcut registry.
 *
 * Replaces the ad-hoc `useEffect`-with-`window.addEventListener`
 * sprawl that EXTERNAL_AUDIT_BACKLOG.md #33 called out, and gives
 * the help-overlay (`?`) a single source of truth.
 *
 * Wiring: components import `useShortcut(commandId, handler)` and the
 * keymap drives the shortcut.  EditorShell / Binder are in flight, so
 * they will adopt this incrementally — this module is the foundation,
 * not the migration.
 *
 * Conventions
 *   - Command ids are dot-separated kebab-case (`editor.find`).
 *   - Bindings use `mod` for the platform-aware ⌘/Ctrl modifier.
 *   - Mac and Windows/Linux can have different defaults; the resolver
 *     picks based on `navigator.platform`.
 *
 * Privacy
 *   No remote sink, no analytics — the keymap is purely local.
 */

import { useEffect } from "react";

/** Logical command identifier. */
export type CommandId =
  | "app.help"
  | "app.toggle-focus-mode"
  | "app.show-shortcuts"
  | "editor.save"
  | "editor.find"
  | "editor.find-replace"
  | "editor.toggle-bold"
  | "editor.toggle-italic"
  | "editor.toggle-underline"
  | "editor.heading-1"
  | "editor.heading-2"
  | "editor.heading-3"
  | "binder.new-scene"
  | "binder.new-chapter"
  | "binder.focus"
  | "snapshot.create"
  | "snapshot.list"
  | "agents.quick-action"
  | "agents.dispatch-copyedit"
  | "agents.dispatch-continuity"
  | "export.run";

/** A platform-aware key binding. */
export interface KeyBinding {
  /** macOS shortcut, in normalised form (see normaliseEvent). */
  mac: string;
  /** Windows + Linux shortcut, in normalised form. */
  pc: string;
  /** Short human-readable description for the help overlay. */
  description: string;
  /** Group label used by the help overlay to organise commands. */
  group: "App" | "Editor" | "Binder" | "Snapshots" | "Agents" | "Export";
}

/**
 * The keymap.  Edit here to change defaults; do NOT scatter
 * `event.key === "..."` checks across components.
 */
export const KEYMAP: Record<CommandId, KeyBinding> = {
  "app.help":              { mac: "mod+?",   pc: "mod+?",   description: "Open the in-app help drawer", group: "App" },
  "app.toggle-focus-mode": { mac: "mod+.",   pc: "mod+.",   description: "Toggle distraction-free mode", group: "App" },
  "app.show-shortcuts":    { mac: "?",       pc: "?",       description: "Show the keyboard-shortcuts overlay", group: "App" },
  "editor.save":           { mac: "mod+s",   pc: "mod+s",   description: "Force save the current scene", group: "Editor" },
  "editor.find":           { mac: "mod+f",   pc: "mod+f",   description: "Find in current scene", group: "Editor" },
  "editor.find-replace":   { mac: "mod+shift+f", pc: "mod+shift+f", description: "Find and replace", group: "Editor" },
  "editor.toggle-bold":    { mac: "mod+b",   pc: "mod+b",   description: "Toggle bold", group: "Editor" },
  "editor.toggle-italic":  { mac: "mod+i",   pc: "mod+i",   description: "Toggle italic", group: "Editor" },
  "editor.toggle-underline": { mac: "mod+u", pc: "mod+u",   description: "Toggle underline", group: "Editor" },
  "editor.heading-1":      { mac: "mod+alt+1", pc: "mod+alt+1", description: "Heading 1", group: "Editor" },
  "editor.heading-2":      { mac: "mod+alt+2", pc: "mod+alt+2", description: "Heading 2", group: "Editor" },
  "editor.heading-3":      { mac: "mod+alt+3", pc: "mod+alt+3", description: "Heading 3", group: "Editor" },
  "binder.new-scene":      { mac: "mod+n",   pc: "mod+n",   description: "Create a new scene", group: "Binder" },
  "binder.new-chapter":    { mac: "mod+shift+n", pc: "mod+shift+n", description: "Create a new chapter", group: "Binder" },
  "binder.focus":          { mac: "mod+1",   pc: "mod+1",   description: "Focus the binder", group: "Binder" },
  "snapshot.create":       { mac: "mod+shift+s", pc: "mod+shift+s", description: "Take a manual snapshot", group: "Snapshots" },
  "snapshot.list":         { mac: "mod+shift+h", pc: "mod+shift+h", description: "Open the snapshot timeline", group: "Snapshots" },
  "agents.quick-action":   { mac: "mod+k",   pc: "mod+k",   description: "Open the quick-action bar", group: "Agents" },
  "agents.dispatch-copyedit":   { mac: "mod+shift+c", pc: "mod+shift+c", description: "Dispatch the Copyedit agent", group: "Agents" },
  "agents.dispatch-continuity": { mac: "mod+shift+y", pc: "mod+shift+y", description: "Dispatch the Continuity agent", group: "Agents" },
  "export.run":            { mac: "mod+shift+e", pc: "mod+shift+e", description: "Open the export dialog", group: "Export" },
};

/**
 * React hook: invoke `handler` whenever the user presses the binding
 * registered to `commandId`.
 *
 * Caveats
 *   - The handler is keyed by the binding stored in `KEYMAP[commandId]`
 *     at call time — if the keymap is mutated at runtime (which it
 *     should not be), unmount + remount the consumer.
 *   - Bindings do NOT fire when the user is typing into a text input
 *     UNLESS the binding includes `mod`.  This matches macOS / Windows
 *     conventions and prevents accidental triggers while typing.
 */
export function useShortcut(commandId: CommandId, handler: (e: KeyboardEvent) => void): void {
  useEffect(() => {
    const binding = KEYMAP[commandId];
    if (!binding) return;
    const target = isMac() ? binding.mac : binding.pc;
    const isModBinding = target.includes("mod+");

    function onKeyDown(e: KeyboardEvent) {
      if (!isModBinding && isInTextInput(e.target)) return;
      if (matches(e, target)) {
        e.preventDefault();
        handler(e);
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [commandId, handler]);
}

/** Normalised binding string → {key, mod, alt, shift}. */
interface ParsedBinding {
  key: string;
  mod: boolean;
  alt: boolean;
  shift: boolean;
}

function parse(binding: string): ParsedBinding {
  const parts = binding.toLowerCase().split("+").map((p) => p.trim());
  const flags: ParsedBinding = { key: "", mod: false, alt: false, shift: false };
  for (const p of parts) {
    if (p === "mod") flags.mod = true;
    else if (p === "alt") flags.alt = true;
    else if (p === "shift") flags.shift = true;
    else flags.key = p;
  }
  return flags;
}

function matches(e: KeyboardEvent, binding: string): boolean {
  const want = parse(binding);
  const modPressed = isMac() ? e.metaKey : e.ctrlKey;
  if (want.mod !== modPressed) return false;
  if (want.alt !== e.altKey) return false;
  if (want.shift !== e.shiftKey) return false;
  return e.key.toLowerCase() === want.key.toLowerCase();
}

function isMac(): boolean {
  if (typeof navigator === "undefined") return false;
  return /Mac|iPhone|iPad/.test(navigator.platform);
}

function isInTextInput(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toUpperCase();
  if (tag === "INPUT" || tag === "TEXTAREA") return true;
  if (target.isContentEditable) return true;
  return false;
}

/** Render-friendly representation of a binding for the help overlay. */
export function formatBinding(commandId: CommandId): string {
  const b = KEYMAP[commandId];
  if (!b) return "";
  const raw = isMac() ? b.mac : b.pc;
  return raw
    .split("+")
    .map((part) => {
      const p = part.toLowerCase();
      if (p === "mod") return isMac() ? "⌘" : "Ctrl";
      if (p === "alt") return isMac() ? "⌥" : "Alt";
      if (p === "shift") return isMac() ? "⇧" : "Shift";
      return p.toUpperCase();
    })
    .join(isMac() ? "" : "+");
}
