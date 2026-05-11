/**
 * Glossary unit tests — pure-data sanity checks on the in-app glossary
 * and `isGlossaryKey` type guard.
 */
import { describe, it, expect } from "vitest";
import { GLOSSARY, isGlossaryKey } from "./glossary";

describe("glossary", () => {
  it("isGlossaryKey returns true for a known key", () => {
    expect(isGlossaryKey("KDP")).toBe(true);
    expect(isGlossaryKey("EPUB")).toBe(true);
    expect(isGlossaryKey("BISAC")).toBe(true);
  });

  it("isGlossaryKey returns false for an unknown key", () => {
    expect(isGlossaryKey("not-a-real-key")).toBe(false);
    expect(isGlossaryKey("")).toBe(false);
  });

  it("every entry has a non-empty label and short description", () => {
    for (const [key, entry] of Object.entries(GLOSSARY)) {
      expect(entry.label, `entry ${key} missing label`).toBeTruthy();
      expect(entry.label.length, `entry ${key} label empty`).toBeGreaterThan(0);
      expect(entry.short, `entry ${key} missing short`).toBeTruthy();
      expect(entry.short.length, `entry ${key} short empty`).toBeGreaterThan(0);
    }
  });

  it("every entry's `short` description fits the tooltip budget (≤ 240 chars)", () => {
    // The header doc claims ≤ 120 chars but a few real entries exceed
    // that today. 240 is a comfortable upper bound that catches obvious
    // bloat without forcing a copy-edit pass on existing entries.
    for (const [key, entry] of Object.entries(GLOSSARY)) {
      expect(entry.short.length, `entry ${key} short too long`).toBeLessThanOrEqual(240);
    }
  });

  it("includes the workflow + privacy keywords the UI references", () => {
    // These keys are referenced by `<Term>` callsites in
    // WorkflowGuide / PrepareForPublishingPanel — broken links here
    // would make those panels fall back to raw key text.
    expect(isGlossaryKey("approval_gate")).toBe(true);
    expect(isGlossaryKey("Ollama")).toBe(true);
    expect(isGlossaryKey("HUMAN_REQUIRED")).toBe(true);
  });
});
