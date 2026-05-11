/**
 * errorMessage unit tests — extract a human-readable string from any
 * thrown value (Error, Tauri IPC error object, string, primitive,
 * null).
 */
import { describe, it, expect } from "vitest";
import { errorMessage } from "./errorMessage";

describe("errorMessage", () => {
  it("returns the string itself for a string input", () => {
    expect(errorMessage("boom")).toBe("boom");
  });

  it("extracts message from an Error", () => {
    expect(errorMessage(new Error("kaboom"))).toBe("kaboom");
  });

  it("returns 'Unknown error.' for null/undefined", () => {
    expect(errorMessage(null)).toBe("Unknown error.");
    expect(errorMessage(undefined)).toBe("Unknown error.");
  });

  it("extracts `message` from a Tauri-shaped error object", () => {
    expect(errorMessage({ kind: "ValidationError", message: "field X invalid" }))
      .toBe("field X invalid");
  });

  it("falls back to `details` when `message` is absent", () => {
    expect(errorMessage({ kind: "ValidationError", details: "constraint failed" }))
      .toBe("constraint failed");
  });

  it("falls back to `error` when neither `message` nor `details` exist", () => {
    expect(errorMessage({ error: "something bad" })).toBe("something bad");
  });

  it("JSON-stringifies an unknown object shape rather than '[object Object]'", () => {
    const out = errorMessage({ foo: "bar", baz: 1 });
    expect(out).toContain("foo");
    expect(out).toContain("bar");
    expect(out).not.toBe("[object Object]");
  });

  it("stringifies primitives via String() as a last resort", () => {
    expect(errorMessage(42)).toBe("42");
    expect(errorMessage(true)).toBe("true");
  });
});
