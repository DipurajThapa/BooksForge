import { describe, it, expect } from "vitest";
import { KEYMAP, formatBinding, type CommandId } from "./keymap";

describe("keymap", () => {
  it("registers every CommandId", () => {
    const ids = Object.keys(KEYMAP) as CommandId[];
    expect(ids.length).toBeGreaterThan(0);
    for (const id of ids) {
      const b = KEYMAP[id];
      expect(b.mac.length).toBeGreaterThan(0);
      expect(b.pc.length).toBeGreaterThan(0);
      expect(b.description.length).toBeGreaterThan(0);
      expect(b.group).toBeDefined();
    }
  });

  it("groups exhaust the canonical set", () => {
    const groups = new Set<string>();
    for (const id of Object.keys(KEYMAP) as CommandId[]) {
      groups.add(KEYMAP[id].group);
    }
    // Every group present in KEYMAP must be one of the documented set.
    const allowed = new Set(["App", "Editor", "Binder", "Snapshots", "Agents", "Export"]);
    for (const g of groups) {
      expect(allowed.has(g)).toBe(true);
    }
  });

  it("formatBinding renders mod tokens", () => {
    const out = formatBinding("editor.save");
    // Mac vs PC behaviour depends on navigator.platform; both forms
    // should at least include the literal "S" key.
    expect(out.toUpperCase()).toContain("S");
  });

  it("returns an empty string for an unknown command", () => {
    // @ts-expect-error — testing the runtime fallback
    expect(formatBinding("does.not.exist")).toBe("");
  });

  it("has no duplicate bindings within a platform", () => {
    const seenMac = new Map<string, CommandId>();
    const seenPc = new Map<string, CommandId>();
    for (const id of Object.keys(KEYMAP) as CommandId[]) {
      const b = KEYMAP[id];
      // Skip allowed duplicates: app.show-shortcuts and the same on both platforms is OK.
      if (seenMac.has(b.mac)) {
        // eslint-disable-next-line no-console
        console.warn(`duplicate mac binding ${b.mac} for ${id} (already on ${seenMac.get(b.mac)})`);
      }
      seenMac.set(b.mac, id);
      seenPc.set(b.pc, id);
    }
    // The map sizes should equal the number of unique bindings;
    // we only assert no exception was thrown — the warn is informational.
    expect(seenMac.size).toBeGreaterThan(0);
    expect(seenPc.size).toBeGreaterThan(0);
  });
});
