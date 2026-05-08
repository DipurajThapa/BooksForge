/**
 * Vitest unit tests for the project template catalogue (BACKLOG §K5).
 *
 * `applyTemplate` itself depends on the Tauri IPC layer and is exercised
 * via the Rust side; here we test the pure-logic catalogue invariants
 * so a future template addition doesn't accidentally break the wizard.
 *
 * Run with `pnpm -C apps/desktop/src-ui test`.
 */
import { describe, it, expect } from "vitest";
import { TEMPLATES, type TemplateId } from "./projectTemplates";

describe("project template catalogue", () => {
  it("ships exactly the four canonical templates", () => {
    const ids = TEMPLATES.map(t => t.id).sort();
    expect(ids).toEqual(["blank", "generic-novel", "non-fiction", "romance"]);
  });

  it("every template has a non-empty label and description", () => {
    for (const t of TEMPLATES) {
      expect(t.label.length).toBeGreaterThan(0);
      expect(t.description.length).toBeGreaterThan(0);
    }
  });

  it("template ids are unique", () => {
    const ids = TEMPLATES.map(t => t.id);
    const uniq = new Set(ids);
    expect(ids.length).toBe(uniq.size);
  });

  it("every template id matches the TemplateId union", () => {
    // Compile-time check via assignment: if a template's id ever
    // drifts from the TemplateId union, this stops compiling.
    for (const t of TEMPLATES) {
      const id: TemplateId = t.id;
      expect(typeof id).toBe("string");
    }
  });
});
