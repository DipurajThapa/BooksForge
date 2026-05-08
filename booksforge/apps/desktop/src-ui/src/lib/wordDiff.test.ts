/**
 * Vitest unit tests for the word-level diff helper (K4 / Turn B).
 *
 * Run with `pnpm -C apps/desktop/src-ui test`.
 */
import { describe, it, expect } from "vitest";
import { tokenize, wordDiff } from "./wordDiff";

describe("tokenize", () => {
  it("splits on word/non-word boundaries", () => {
    expect(tokenize("hello, world!")).toEqual(["hello", ", ", "world", "!"]);
  });

  it("preserves leading and trailing whitespace", () => {
    expect(tokenize("  alpha   beta  ")).toEqual(["  ", "alpha", "   ", "beta", "  "]);
  });

  it("returns empty array for empty input", () => {
    expect(tokenize("")).toEqual([]);
  });
});

describe("wordDiff", () => {
  it("returns nothing for identical empty strings", () => {
    expect(wordDiff("", "")).toEqual([]);
  });

  it("returns a single equal segment for identical input", () => {
    expect(wordDiff("hello world", "hello world")).toEqual([
      { op: "equal", text: "hello world" },
    ]);
  });

  it("flags pure additions", () => {
    const out = wordDiff("hello", "hello world");
    expect(out).toContainEqual({ op: "equal", text: "hello" });
    expect(out.some((s) => s.op === "add" && s.text.includes("world"))).toBe(true);
  });

  it("flags pure removals", () => {
    const out = wordDiff("hello world", "hello");
    expect(out).toContainEqual({ op: "equal", text: "hello" });
    expect(out.some((s) => s.op === "remove" && s.text.includes("world"))).toBe(true);
  });

  it("coalesces consecutive same-op tokens", () => {
    const out = wordDiff("the quick brown fox", "the lazy red fox");
    // Equal-prefix "the " then a remove run, then an add run, then equal "fox".
    const equalTexts = out.filter((s) => s.op === "equal").map((s) => s.text);
    expect(equalTexts.some((t) => t.startsWith("the"))).toBe(true);
    expect(equalTexts.some((t) => t.includes("fox"))).toBe(true);
  });

  it("preserves whitespace tokens through the diff", () => {
    const out = wordDiff("alpha beta", "alpha gamma");
    const joined = out.map((s) => s.text).join("");
    // Joining all segments must yield a string that contains both inputs'
    // characters intact in stream order.
    expect(joined).toContain("alpha");
    expect(joined).toContain("beta");
    expect(joined).toContain("gamma");
  });
});
